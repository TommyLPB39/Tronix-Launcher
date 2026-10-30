mod covers;
mod details;
mod i18n;
mod icons;
mod imports;
mod launchers;
mod playtime;
mod rank;
mod secret;
mod settings;
mod steam_account;
mod stats;
mod store;

use serde::Serialize;
use covers::Kind;
use settings::{ACCENTS, Collection, Look, PLATFORMS, Settings};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use store::{Program, Store};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Verrou qui reste utilisable si un autre fil a paniqué en le tenant : l'application continue au lieu de
/// planter à chaque commande suivante.
trait LockExt<T> {
    fn lock_safe(&self) -> std::sync::MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_safe(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Remet un indicateur « en cours » à faux quand le fil se termine, même par un panic ; sinon le travail
/// ne pourrait plus jamais être relancé.
struct ResetOnDrop(&'static AtomicBool);

impl Drop for ResetOnDrop {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

static START: OnceLock<Instant> = OnceLock::new();
/// Dernier décalage horaire reçu de l'interface, pour classer depuis l'icône et le raccourci.
static TZ: AtomicI32 = AtomicI32::new(0);
/// Faux si le système a refusé le raccourci enregistré (déjà pris par un autre programme).
static SHORTCUT_OK: AtomicBool = AtomicBool::new(true);
/// Vrai tant qu'un téléchargement de pochettes est en cours (un seul à la fois).
static FETCHING: AtomicBool = AtomicBool::new(false);

const TRAY_ID: &str = "main";
const TRAY_PROGRAMS: usize = 5;

type AppStore<'a> = State<'a, Mutex<Store>>;
type AppSettings<'a> = State<'a, Mutex<Settings>>;

/// Appelée par l'interface après son premier affichage.
/// Écrit le temps écoulé depuis le début du programme (hors lancement du processus).
#[tauri::command]
fn app_ready() {
    if let Some(start) = START.get() {
        eprintln!("[tronix] accueil affiché en {} ms", start.elapsed().as_millis());
    }
}

/// Programmes des launchers activés, plus ceux ajoutés à la main.
fn visible_programs(store: &Store, settings: &Settings) -> Vec<Program> {
    store
        .programs
        .iter()
        .filter(|p| !p.uninstalled && (p.source == "manual" || settings.is_enabled(&p.source)))
        .map(Program::with_display_name) // `name` est le titre choisi par le joueur, s'il y en a un
        .collect()
}

/// Jeux Steam possédés mais non installés, classés par nom ; vide tant que le joueur ne les affiche pas.
fn uninstalled_programs(store: &Store, settings: &Settings) -> Vec<Program> {
    if !settings.show_uninstalled() || !settings.is_enabled("steam") {
        return Vec::new();
    }
    let mut games: Vec<Program> = store.programs.iter().filter(|p| p.uninstalled).map(Program::with_display_name).collect();
    games.sort_by_cached_key(|p| p.name.to_lowercase());
    games
}

/// Un programme et sa pochette, tels que l'interface les reçoit.
#[derive(Serialize)]
struct ProgramView {
    #[serde(flatten)]
    program: Program,
    /// Chemin de l'image à afficher, s'il y en a une.
    cover: Option<String>,
    /// "custom" (choisie par le joueur), "art" (pochette) ou "icon" (icône du programme).
    cover_kind: Option<&'static str>,
    /// Change quand l'image est remplacée, pour que l'affichage se rafraîchisse.
    cover_ver: u64,
    /// Titre d'origine, que « Rétablir » redonne (`program.name` est le titre affiché).
    original_name: String,
}

/// Bibliothèque classée du plus au moins probable, sans les jeux des launchers
/// désactivés. `tz` : décalage horaire au format JavaScript
/// (`Date.getTimezoneOffset()`).
fn snapshot(store: &Store, settings: &Settings, tz: i32) -> Vec<ProgramView> {
    TZ.store(tz, Ordering::Relaxed);
    let visible = visible_programs(store, settings);
    let mut found = covers::scan(&store.covers_dir());
    let originals: HashMap<&str, &str> = store.programs.iter().map(|p| (p.id.as_str(), p.name.as_str())).collect();
    // Les jeux non installés viennent après les autres : ils ne sont jamais suggérés.
    rank::ranked(&visible, store::now_ms(), tz)
        .into_iter()
        .chain(uninstalled_programs(store, settings))
        .map(|program| {
            let cover = found.remove(&program.id);
            ProgramView {
                cover_kind: cover.as_ref().map(|c| c.kind.as_str()),
                cover_ver: cover.as_ref().map_or(0, |c| c.version),
                cover: cover.map(|c| c.path),
                original_name: originals.get(program.id.as_str()).map_or_else(|| program.name.clone(), |n| n.to_string()),
                program,
            }
        })
        .collect()
}

// ------------------------------------------------------ Icône de notification

fn tray_menu(app: &AppHandle, programs: &[ProgramView]) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    for view in programs.iter().filter(|v| !v.program.uninstalled).take(TRAY_PROGRAMS) {
        // Dans un menu Windows, « & » désigne un raccourci : on le double pour l'afficher.
        let label = view.program.name.replace('&', "&&");
        menu.append(&MenuItem::with_id(app, format!("launch:{}", view.program.id), label, true, None::<&str>)?)?;
    }
    if programs.iter().any(|v| !v.program.uninstalled) {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    menu.append(&MenuItem::with_id(app, "open", i18n::t("tray.open"), true, None::<&str>)?)?;
    menu.append(&MenuItem::with_id(app, "quit", i18n::t("tray.quit"), true, None::<&str>)?)?;
    Ok(menu)
}

/// Remet à jour les 5 programmes affichés par l'icône.
fn refresh_tray(app: &AppHandle, programs: &[ProgramView]) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), tray_menu(app, programs)) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

// ------------------------------------------------------ Lancement rapide

fn show_quick(window: &WebviewWindow) {
    let _ = window.center();
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit("quick-open", ());
}

/// Affiche ou cache la mini-fenêtre. Elle est créée au premier usage seulement,
/// pour ne rien coûter au démarrage.
fn toggle_quick(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("quick") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_quick(&window);
        }
        return;
    }
    // Créer une fenêtre depuis le thread des événements bloquerait sous Windows.
    let app = app.clone();
    std::thread::spawn(move || {
        let built = WebviewWindowBuilder::new(&app, "quick", WebviewUrl::App("quick.html".into()))
            .title(i18n::t("quick.title"))
            .inner_size(640.0, 440.0)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .center()
            .background_color(Color(10, 11, 16, 255))
            .build();
        if let Err(error) = built {
            eprintln!("[tronix] mini-fenêtre impossible : {error}");
        }
    });
}

#[tauri::command]
fn hide_quick(app: AppHandle) {
    if let Some(window) = app.get_webview_window("quick") {
        let _ = window.hide();
    }
}

fn register_shortcut(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_quick(app);
            }
        })
        .map_err(|e| e.to_string())
}

/// Change le raccourci global. L'ancien reste actif si le nouveau est refusé.
#[tauri::command]
fn set_shortcut(app: AppHandle, settings: AppSettings, shortcut: String) -> Result<String, String> {
    let mut settings = settings.lock_safe();
    let old = settings.shortcut().to_string();
    let _ = app.global_shortcut().unregister(old.as_str());
    if register_shortcut(&app, &shortcut).is_err() {
        // On remet l'ancien ; s'il était déjà refusé, il le reste.
        SHORTCUT_OK.store(register_shortcut(&app, &old).is_ok(), Ordering::Relaxed);
        return Err(i18n::t("err.invalid_shortcut"));
    }
    SHORTCUT_OK.store(true, Ordering::Relaxed);
    settings.set_shortcut(&shortcut).map_err(|e| e.to_string())?;
    Ok(shortcut)
}

#[tauri::command]
fn set_close_to_tray(settings: AppSettings, enabled: bool) -> Result<(), String> {
    settings.lock_safe().set_close_to_tray(enabled).map_err(|e| e.to_string())
}

// ------------------------------------------------------------ Apparence et pochettes

#[derive(Serialize)]
struct AccentView {
    id: &'static str,
    label: &'static str,
    hex: &'static str,
}

fn accent_hex(id: &str) -> &'static str {
    ACCENTS.iter().find(|(known, _, _)| *known == id).map_or(ACCENTS[0].2, |(_, _, hex)| *hex)
}

#[tauri::command]
fn set_accent(settings: AppSettings, id: String) -> Result<String, String> {
    let mut settings = settings.lock_safe();
    settings.set_accent(&id)?;
    Ok(accent_hex(settings.accent()).to_string())
}

/// Couleur d'accent à afficher : la couleur libre si le joueur en a choisi une.
fn accent_value(settings: &Settings) -> String {
    match settings.look().custom_accent.as_str() {
        "" => accent_hex(settings.accent()).to_string(),
        custom => custom.to_string(),
    }
}

// ------------------------------------------------------------ Collections

/// Crée une collection ; renvoie l'état à jour.
#[tauri::command]
fn create_collection(app: AppHandle, store: AppStore, settings: AppSettings, name: String, tz: i32) -> Result<AppState, String> {
    let mut settings = settings.lock_safe();
    settings.add_collection(&name)?;
    let store = store.lock_safe();
    Ok(state_and_tray(&app, &store, &settings, tz))
}

#[tauri::command]
fn rename_collection(app: AppHandle, store: AppStore, settings: AppSettings, id: String, name: String, tz: i32) -> Result<AppState, String> {
    let mut settings = settings.lock_safe();
    settings.rename_collection(&id, &name)?;
    let store = store.lock_safe();
    Ok(state_and_tray(&app, &store, &settings, tz))
}

/// Supprime une collection (les jeux ne sont pas touchés, seulement retirés de la collection).
#[tauri::command]
fn delete_collection(app: AppHandle, store: AppStore, settings: AppSettings, id: String, tz: i32) -> Result<AppState, String> {
    let mut settings = settings.lock_safe();
    settings.delete_collection(&id)?;
    let mut store = store.lock_safe();
    if store.forget_collection(&id) {
        store.save().map_err(|e| e.to_string())?;
    }
    Ok(state_and_tray(&app, &store, &settings, tz))
}

/// Range un programme dans une collection, ou l'en retire.
#[tauri::command]
fn set_program_collection(
    app: AppHandle,
    store: AppStore,
    settings: AppSettings,
    id: String,
    collection: String,
    member: bool,
    tz: i32,
) -> Result<AppState, String> {
    let settings = settings.lock_safe();
    if !settings.collections().iter().any(|c| c.id == collection) {
        return Err(i18n::t("err.unknown_collection"));
    }
    let mut store = store.lock_safe();
    store.set_collection(&id, &collection, member)?;
    store.save().map_err(|e| e.to_string())?;
    Ok(state_and_tray(&app, &store, &settings, tz))
}

fn state_and_tray(app: &AppHandle, store: &Store, settings: &Settings, tz: i32) -> AppState {
    let state = app_state(store, settings, tz);
    refresh_tray(app, &state.programs);
    state
}

// ---------------------------------------------------------------- Langue

#[derive(Serialize)]
struct Strings {
    /// Langue utilisée : "fr" ou "en".
    lang: &'static str,
    strings: HashMap<String, String>,
}

/// Tous les textes de la langue en cours, pour l'interface.
#[tauri::command]
fn get_strings() -> Strings {
    let lang = i18n::lang();
    Strings { lang, strings: i18n::strings(lang) }
}

/// Choisit la langue ("auto", "fr" ou "en") ; elle s'applique tout de suite, menu de l'icône compris.
#[tauri::command]
fn set_language(app: AppHandle, store: AppStore, settings: AppSettings, language: String, tz: i32) -> Result<AppState, String> {
    let mut settings = settings.lock_safe();
    settings.set_language(&language)?;
    i18n::set_lang(i18n::resolve(settings.language()));
    let store = store.lock_safe();
    Ok(state_and_tray(&app, &store, &settings, tz))
}

/// Choisit le tri de la bibliothèque.
#[tauri::command]
fn set_sort(settings: AppSettings, sort: String) -> Result<(), String> {
    settings.lock_safe().set_sort(&sort)
}

/// Épingle ou désépingle un programme ; renvoie la bibliothèque à jour.
#[tauri::command]
fn set_favorite(app: AppHandle, store: AppStore, settings: AppSettings, id: String, favorite: bool, tz: i32) -> Result<Vec<ProgramView>, String> {
    let settings = settings.lock_safe();
    let mut store = store.lock_safe();
    store.set_favorite(&id, favorite)?;
    store.save().map_err(|e| e.to_string())?;
    let programs = snapshot(&store, &settings, tz);
    refresh_tray(&app, &programs);
    Ok(programs)
}

/// Enregistre l'apparence et renvoie la version nettoyée (valeurs inconnues remplacées).
#[tauri::command]
fn set_look(settings: AppSettings, look: Look) -> Result<Look, String> {
    let mut settings = settings.lock_safe();
    settings.set_look(look).map_err(|e| e.to_string())?;
    Ok(settings.look().clone())
}

/// Rétablit l'apparence par défaut (thème, tailles, fond, couleur d'accentuation).
#[tauri::command]
fn reset_look(settings: AppSettings) -> Result<Look, String> {
    let mut settings = settings.lock_safe();
    settings.reset_look().map_err(|e| e.to_string())?;
    Ok(settings.look().clone())
}

/// Identifiant Steam d'un lien de lancement `steam://rungameid/<appid>`.
fn steam_appid(target: &str) -> Option<&str> {
    target.strip_prefix("steam://rungameid/")
}

/// Copie dans le dossier des pochettes celles que Steam garde déjà sur le PC.
fn copy_steam_covers(store: &Store) {
    #[cfg(windows)]
    if let Some(root) = imports::steam_root() {
        let dir = store.covers_dir();
        let have = covers::scan(&dir);
        for program in store.programs.iter().filter(|p| p.source == "steam") {
            if have.get(&program.id).is_some_and(|c| c.kind != Kind::Icon) {
                continue;
            }
            if let Some(bytes) = steam_appid(&program.target).and_then(|appid| covers::steam_local(&root, appid)) {
                let _ = covers::save(&dir, &program.id, Kind::Art, &bytes);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = store;
}

/// Un programme dont il manque la pochette.
struct Wanted {
    id: String,
    name: String,
    appid: Option<String>,
}

/// Télécharge en arrière-plan les pochettes manquantes : Steam d'abord, puis SteamGridDB
/// si une clé est renseignée. Un seul téléchargement à la fois.
fn start_cover_download(app: &AppHandle) {
    if FETCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let _reset = ResetOnDrop(&FETCHING);
        download_missing_covers(&app);
    });
}

fn download_missing_covers(app: &AppHandle) {
    let (wanted, key, dir) = {
        let settings = app.state::<Mutex<Settings>>();
        let store = app.state::<Mutex<Store>>();
        let settings = settings.lock_safe();
        let store = store.lock_safe();
        if !settings.download_covers() {
            return;
        }
        let dir = store.covers_dir();
        let have = covers::scan(&dir);
        let wanted: Vec<Wanted> = store
            .programs
            .iter()
            .filter(|p| p.source == "manual" || settings.is_enabled(&p.source))
            .filter(|p| !p.uninstalled || settings.show_uninstalled())
            .filter(|p| have.get(&p.id).is_none_or(|c| c.kind == Kind::Icon))
            .map(|p| Wanted {
                id: p.id.clone(),
                name: p.display_name().to_string(),
                appid: steam_appid(&p.target).map(str::to_string),
            })
            .collect();
        (wanted, settings.steamgriddb_key().to_string(), dir)
    };
    // Aucun verrou n'est tenu pendant les échanges réseau.
    let mut saved = 0;
    for game in wanted {
        let bytes = game
            .appid
            .as_deref()
            .and_then(covers::steam_download)
            .or_else(|| (!key.is_empty()).then(|| covers::steamgriddb_download(&game.name, &key)).flatten());
        if bytes.is_some_and(|b| covers::save(&dir, &game.id, Kind::Art, &b).is_ok()) {
            saved += 1;
            if saved % 5 == 0 {
                let _ = app.emit("library-changed", "covers");
            }
        }
    }
    if saved > 0 {
        let _ = app.emit("library-changed", "covers");
    }
}

/// Active ou non le téléchargement des pochettes, et enregistre la clé SteamGridDB si elle est donnée.
#[tauri::command]
fn set_download_covers(
    app: AppHandle,
    settings: AppSettings,
    enabled: bool,
    key: Option<String>,
) -> Result<(), String> {
    {
        let mut settings = settings.lock_safe();
        settings.set_download_covers(enabled).map_err(|e| e.to_string())?;
        if let Some(key) = key {
            settings.set_steamgriddb_key(&key).map_err(|e| e.to_string())?;
        }
    }
    if enabled {
        start_cover_download(&app);
    }
    Ok(())
}

/// Relance le téléchargement des pochettes manquantes (bouton des paramètres).
#[tauri::command]
fn fetch_covers(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    if !settings.lock_safe().download_covers() {
        return Err(i18n::t("err.covers_off"));
    }
    start_cover_download(&app);
    Ok(())
}

/// Choisit une image du PC comme pochette d'un programme.
#[tauri::command]
fn set_custom_cover(
    app: AppHandle,
    store: AppStore,
    settings: AppSettings,
    id: String,
    path: String,
    tz: i32,
) -> Result<Vec<ProgramView>, String> {
    let settings = settings.lock_safe();
    let store = store.lock_safe();
    if !store.programs.iter().any(|p| p.id == id) {
        return Err(i18n::t("err.unknown_program"));
    }
    covers::set_custom(&store.covers_dir(), &id, Path::new(&path))?;
    let programs = snapshot(&store, &settings, tz);
    refresh_tray(&app, &programs);
    Ok(programs)
}

/// Retire l'image choisie par le joueur : la pochette d'origine revient.
#[tauri::command]
fn clear_custom_cover(store: AppStore, settings: AppSettings, id: String, tz: i32) -> Vec<ProgramView> {
    let settings = settings.lock_safe();
    let store = store.lock_safe();
    covers::remove(&store.covers_dir(), &id, Kind::Custom);
    snapshot(&store, &settings, tz)
}

// ------------------------------------------------------------ Pages des jeux et logiciels

/// Pages dont le rafraichissement est en cours, et dernieres erreurs reseau par programme.
static REFRESHING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static ERRORS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
/// Vrai pendant la preparation du texte de toutes les pages (une seule a la fois).
static PREPARING: AtomicBool = AtomicBool::new(false);

fn refreshing() -> &'static Mutex<HashSet<String>> {
    REFRESHING.get_or_init(Default::default)
}

fn errors() -> &'static Mutex<HashMap<String, String>> {
    ERRORS.get_or_init(Default::default)
}

fn subject_of(program: &Program) -> details::Subject {
    details::Subject {
        name: program.display_name().to_string(),
        source: program.source.clone(),
        target: program.target.clone(),
    }
}

#[derive(Serialize)]
struct ShotView {
    path: String,
    ver: u64,
}

#[derive(Serialize)]
struct LocalInfo {
    added: i64,
    launches: usize,
    last_launch: Option<i64>,
    play_ms: u64,
    play_est_ms: u64,
    target: String,
}

#[derive(Serialize)]
struct DetailsView {
    id: String,
    /// Titre affiché (celui du joueur s'il en a mis un).
    name: String,
    original_name: String,
    /// Description écrite par le joueur : elle remplace celle des sources en ligne.
    custom_description: Option<String>,
    source: String,
    cover: Option<String>,
    cover_kind: Option<&'static str>,
    cover_ver: u64,
    /// "ready", "none" (aucune page), "off" (informations en ligne desactivees),
    /// "needs_key" (cle RAWG manquante) ou "pending" (rien encore, chargement).
    state: &'static str,
    loading: bool,
    error: Option<String>,
    details: Option<details::Details>,
    shots: Vec<ShotView>,
    /// Vrai si une autre page peut etre choisie (hors jeux Steam).
    can_correct: bool,
    online: bool,
    rawg_key_set: bool,
    local: LocalInfo,
}

fn details_view(app: &AppHandle, id: &str) -> Result<DetailsView, String> {
    let settings = app.state::<Mutex<Settings>>();
    let store = app.state::<Mutex<Store>>();
    let settings = settings.lock_safe();
    let store = store.lock_safe();
    let program = store.programs.iter().find(|p| p.id == id).ok_or_else(|| i18n::t("err.unknown_program"))?;
    let dir = store.details_dir();
    let subject = subject_of(program);
    let choice = details::load_choice(&dir, id);
    let cached = details::load(&dir, id);
    let online = settings.online_info();
    let key = settings.rawg_key();

    let (state, details) = match cached {
        Some(d) if d.provider == "none" => ("none", None),
        Some(d) => ("ready", Some(d)),
        None if details::needs_rawg_key(&subject, key, choice.as_ref()) => ("needs_key", None),
        None if online => ("pending", None),
        None => ("off", None),
    };
    let cover = covers::scan(&store.covers_dir()).remove(id);
    Ok(DetailsView {
        id: id.to_string(),
        name: program.display_name().to_string(),
        original_name: program.name.clone(),
        custom_description: program.custom_description.clone(),
        source: program.source.clone(),
        cover_kind: cover.as_ref().map(|c| c.kind.as_str()),
        cover_ver: cover.as_ref().map_or(0, |c| c.version),
        cover: cover.map(|c| c.path),
        state,
        loading: online && refreshing().lock_safe().contains(id),
        error: errors().lock_safe().get(id).cloned(),
        shots: details::local_shots(&dir, id).into_iter().map(|(path, ver)| ShotView { path, ver }).collect(),
        can_correct: online && !program.target.starts_with("steam://"),
        online,
        rawg_key_set: !key.is_empty(),
        local: LocalInfo {
            added: program.added,
            launches: program.launches.len(),
            last_launch: program.launches.iter().copied().max(),
            play_ms: program.play_ms,
            play_est_ms: program.play_est_ms,
            target: program.target.clone(),
        },
        details,
    })
}

/// Met a jour la page d'un programme : texte, avis, captures et actualites, selon leur age.
fn refresh_details(app: &AppHandle, id: &str) {
    let (subject, choice, key, dir) = {
        let settings = app.state::<Mutex<Settings>>();
        let store = app.state::<Mutex<Store>>();
        let settings = settings.lock_safe();
        let store = store.lock_safe();
        let Some(program) = store.programs.iter().find(|p| p.id == id) else {
            return;
        };
        if !settings.online_info() {
            return;
        }
        (
            subject_of(program),
            details::load_choice(&store.details_dir(), id),
            settings.rawg_key().to_string(),
            store.details_dir(),
        )
    };
    // Aucun verrou n'est tenu pendant les echanges reseau.
    let now = store::now_ms();
    let mut current = details::load(&dir, id);
    if current.as_ref().is_none_or(|d| !d.is_fresh(now)) {
        match details::fetch_text(&subject, choice.as_ref(), &key, now) {
            Ok(Some(found)) => current = Some(found),
            Ok(None) => {
                current = Some(details::Details { provider: "none".into(), fetched_at: now, ui_lang: i18n::lang().to_string(), ..Default::default() });
            }
            Err(error) => {
                errors().lock_safe().insert(id.to_string(), error);
                let _ = app.emit("details-updated", id);
                return;
            }
        }
        errors().lock_safe().remove(id);
    }
    let Some(mut page) = current else {
        return;
    };
    if page.provider != "none" {
        if page.provider == "steam" && now - page.reviews_at > details::REVIEWS_TTL_MS {
            if let Some(rating) = details::steam_reviews(&page.match_id) {
                page.rating = Some(rating);
            }
            page.reviews_at = now;
        }
        if !page.shots_ok {
            let urls = match page.provider.as_str() {
                "steam" => page.screenshot_urls.clone(),
                "rawg" if !key.is_empty() => details::rawg_shots(&page.match_id, &key),
                _ => Vec::new(),
            };
            // Hors connexion, on reessaiera plus tard : on ne marque « fait » que si tout s'est passe.
            page.shots_ok = urls.is_empty() || details::download_shots(&dir, id, &urls) > 0;
        }
        if page.provider == "steam" && now - page.news_at > details::NEWS_TTL_MS {
            page.news = details::steam_news(&page.match_id);
            page.news_at = now;
        }
    }
    let _ = details::save(&dir, id, &page);
    let _ = app.emit("details-updated", id);
}

fn start_refresh(app: &AppHandle, id: &str) {
    if !refreshing().lock_safe().insert(id.to_string()) {
        return;
    }
    let (app, id) = (app.clone(), id.to_string());
    std::thread::spawn(move || {
        refresh_details(&app, &id);
        refreshing().lock_safe().remove(&id);
        let _ = app.emit("details-updated", &id);
    });
}

/// Ouvre la page d'un programme : renvoie ce qui est en cache et rafraichit en arriere-plan.
#[tauri::command]
fn open_details(app: AppHandle, id: String) -> Result<DetailsView, String> {
    start_refresh(&app, &id);
    details_view(&app, &id)
}

/// Relit la page sans lancer de mise a jour (apres l'evenement `details-updated`).
#[tauri::command]
fn get_details(app: AppHandle, id: String) -> Result<DetailsView, String> {
    details_view(&app, &id)
}

/// Prepare en arriere-plan le texte des pages qui n'en ont pas encore, une a la fois.
fn prepare_details(app: &AppHandle) {
    if PREPARING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let _reset = ResetOnDrop(&PREPARING);
        let wanted: Vec<String> = {
            let settings = app.state::<Mutex<Settings>>();
            let store = app.state::<Mutex<Store>>();
            let settings = settings.lock_safe();
            let store = store.lock_safe();
            if !settings.online_info() {
                PREPARING.store(false, Ordering::SeqCst);
                return;
            }
            let dir = store.details_dir();
            let now = store::now_ms();
            visible_programs(&store, &settings)
                .iter()
                .filter(|p| {
                    let choice = details::load_choice(&dir, &p.id);
                    let needs_key = details::needs_rawg_key(&subject_of(p), settings.rawg_key(), choice.as_ref());
                    let fresh = details::load(&dir, &p.id).is_some_and(|d| d.is_fresh(now));
                    !needs_key && !fresh
                })
                .map(|p| p.id.clone())
                .collect()
        };
        for id in wanted {
            if !app.state::<Mutex<Settings>>().lock_safe().online_info() {
                break;
            }
            // Si la page est deja en cours de mise a jour par son ouverture, on la laisse faire.
            if refreshing().lock_safe().insert(id.clone()) {
                refresh_text_only(&app, &id);
                refreshing().lock_safe().remove(&id);
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        PREPARING.store(false, Ordering::SeqCst);
    });
}

/// Preparation d'une page : texte et fiche seulement ; captures et actualites attendent l'ouverture.
fn refresh_text_only(app: &AppHandle, id: &str) {
    let (subject, choice, key, dir) = {
        let settings = app.state::<Mutex<Settings>>();
        let store = app.state::<Mutex<Store>>();
        let settings = settings.lock_safe();
        let store = store.lock_safe();
        let Some(program) = store.programs.iter().find(|p| p.id == id) else {
            return;
        };
        (
            subject_of(program),
            details::load_choice(&store.details_dir(), id),
            settings.rawg_key().to_string(),
            store.details_dir(),
        )
    };
    let now = store::now_ms();
    let page = match details::fetch_text(&subject, choice.as_ref(), &key, now) {
        Ok(Some(found)) => found,
        Ok(None) => details::Details { provider: "none".into(), fetched_at: now, ui_lang: i18n::lang().to_string(), ..Default::default() },
        Err(_) => return, // reseau indisponible : on reessaiera
    };
    let _ = details::save(&dir, id, &page);
}

/// Active ou non les informations en ligne, et enregistre la cle RAWG si elle est donnee.
#[tauri::command]
fn set_online_info(app: AppHandle, settings: AppSettings, enabled: bool, key: Option<String>) -> Result<(), String> {
    {
        let mut settings = settings.lock_safe();
        settings.set_online_info(enabled).map_err(|e| e.to_string())?;
        if let Some(key) = key {
            settings.set_rawg_key(&key).map_err(|e| e.to_string())?;
        }
    }
    if enabled {
        prepare_details(&app);
    }
    Ok(())
}

/// Pages possibles pour « Ce n'est pas le bon ? ». Appel reseau : fait hors du fil principal.
#[tauri::command]
async fn details_candidates(app: AppHandle, id: String) -> Result<Vec<details::Candidate>, String> {
    let (subject, key) = {
        let settings = app.state::<Mutex<Settings>>();
        let store = app.state::<Mutex<Store>>();
        let settings = settings.lock_safe();
        let store = store.lock_safe();
        let program = store.programs.iter().find(|p| p.id == id).ok_or_else(|| i18n::t("err.unknown_program"))?;
        (subject_of(program), settings.rawg_key().to_string())
    };
    tauri::async_runtime::spawn_blocking(move || details::candidates(&subject, &key))
        .await
        .map_err(|e| e.to_string())?
}

/// Memorise la page choisie (`provider` = "none" : aucune page ; "auto" : retour au choix automatique).
#[tauri::command]
fn choose_details(app: AppHandle, store: AppStore, id: String, provider: String, match_id: String) -> Result<(), String> {
    let dir = store.lock_safe().details_dir();
    let choice = match provider.as_str() {
        "auto" => None,
        _ => Some(details::Choice { provider, id: match_id }),
    };
    details::save_choice(&dir, &id, choice.as_ref()).map_err(|e| e.to_string())?;
    details::forget(&dir, &id);
    errors().lock_safe().remove(&id);
    start_refresh(&app, &id);
    Ok(())
}

/// Ouvre un lien dans le navigateur. Seuls les liens https sont acceptes.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if url.len() > 2048 || !covers::remote_url_ok(&url) || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(i18n::t("err.link_refused"));
    }
    store::explorer().arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
}


// ------------------------------------------------ Titre, description et pochette choisis

/// Enregistre le titre et la description du joueur ; un texte vide rétablit l'original.
#[tauri::command]
fn set_program_info(
    app: AppHandle,
    store: AppStore,
    settings: AppSettings,
    id: String,
    name: String,
    description: String,
    tz: i32,
) -> Result<Vec<ProgramView>, String> {
    let (programs, refresh) = {
        let settings = settings.lock_safe();
        let mut store = store.lock_safe();
        let renamed = store.set_info(&id, &name, &description)?;
        store.save().map_err(|e| e.to_string())?;
        // Une page trouvée avec l'ancien titre est refaite avec le nouveau, sauf si le joueur a choisi la page lui-même.
        let dir = store.details_dir();
        let refresh = renamed && settings.online_info() && details::load_choice(&dir, &id).is_none();
        if refresh {
            details::forget(&dir, &id);
        }
        let programs = snapshot(&store, &settings, tz);
        refresh_tray(&app, &programs);
        (programs, refresh)
    };
    if refresh {
        errors().lock_safe().remove(&id);
        start_refresh(&app, &id);
    }
    Ok(programs)
}

fn steamgriddb_key(app: &AppHandle) -> String {
    app.state::<Mutex<Settings>>().lock_safe().steamgriddb_key().to_string()
}

/// Jeux de SteamGridDB correspondant à une recherche. Appel réseau : fait hors du fil principal.
#[tauri::command]
async fn sgdb_search(app: AppHandle, query: String) -> Result<Vec<covers::SgdbGame>, String> {
    let key = steamgriddb_key(&app);
    tauri::async_runtime::spawn_blocking(move || covers::sgdb_search_games(&query, &key))
        .await
        .map_err(|e| e.to_string())?
}

/// Pochettes verticales d'un jeu de SteamGridDB.
#[tauri::command]
async fn sgdb_grids(app: AppHandle, game_id: i64) -> Result<Vec<covers::SgdbGrid>, String> {
    let key = steamgriddb_key(&app);
    tauri::async_runtime::spawn_blocking(move || covers::sgdb_grids(game_id, &key))
        .await
        .map_err(|e| e.to_string())?
}

/// Télécharge la pochette choisie et en fait l'image du programme (comme une image personnelle).
#[tauri::command]
async fn sgdb_pick(app: AppHandle, id: String, url: String, tz: i32) -> Result<Vec<ProgramView>, String> {
    let bytes = tauri::async_runtime::spawn_blocking(move || covers::sgdb_download(&url))
        .await
        .map_err(|e| e.to_string())??;
    let settings = app.state::<Mutex<Settings>>();
    let store = app.state::<Mutex<Store>>();
    let settings = settings.lock_safe();
    let store = store.lock_safe();
    if !store.programs.iter().any(|p| p.id == id) {
        return Err(i18n::t("err.unknown_program"));
    }
    covers::save(&store.covers_dir(), &id, Kind::Custom, &bytes)?;
    let programs = snapshot(&store, &settings, tz);
    refresh_tray(&app, &programs);
    Ok(programs)
}

/// Statistiques d'usage des programmes visibles.
#[tauri::command]
fn get_stats(store: AppStore, settings: AppSettings, tz: i32) -> stats::Stats {
    let settings = settings.lock_safe();
    let store = store.lock_safe();
    stats::compute(&visible_programs(&store, &settings), store::now_ms(), tz)
}

// ------------------------------------------------------------ État de l'écran

#[derive(Serialize)]
struct PlatformView {
    id: &'static str,
    label: &'static str,
    enabled: bool,
    /// Présent sur ce PC (calculé seulement avant le premier choix).
    detected: bool,
}

#[derive(Serialize)]
struct AppState {
    onboarded: bool,
    platforms: Vec<PlatformView>,
    programs: Vec<ProgramView>,
    close_to_tray: bool,
    shortcut: String,
    shortcut_ok: bool,
    accent: String,
    accent_id: String,
    look: Look,
    sort: String,
    /// Réglage de langue : "auto", "fr" ou "en".
    language: String,
    collections: Vec<Collection>,
    accents: Vec<AccentView>,
    download_covers: bool,
    steamgriddb_key_set: bool,
    online_info: bool,
    rawg_key_set: bool,
    show_uninstalled: bool,
    steam_key_set: bool,
}

fn app_state(store: &Store, settings: &Settings, tz: i32) -> AppState {
    let onboarded = settings.onboarded();
    let detected = if onboarded { Vec::new() } else { imports::detected() };
    AppState {
        onboarded,
        platforms: PLATFORMS
            .iter()
            .map(|&(id, label)| PlatformView {
                id,
                label,
                enabled: if onboarded { settings.is_enabled(id) } else { detected.contains(&id) },
                detected: detected.contains(&id),
            })
            .collect(),
        programs: snapshot(store, settings, tz),
        close_to_tray: settings.close_to_tray(),
        shortcut: settings.shortcut().to_string(),
        shortcut_ok: SHORTCUT_OK.load(Ordering::Relaxed),
        accent: accent_value(settings),
        accent_id: if settings.look().custom_accent.is_empty() { settings.accent().to_string() } else { "custom".to_string() },
        look: settings.look().clone(),
        sort: settings.sort().to_string(),
        language: settings.language().to_string(),
        collections: settings.collections().to_vec(),
        accents: ACCENTS.iter().map(|&(id, label, hex)| AccentView { id, label, hex }).collect(),
        download_covers: settings.download_covers(),
        steamgriddb_key_set: !settings.steamgriddb_key().is_empty(),
        online_info: settings.online_info(),
        rawg_key_set: !settings.rawg_key().is_empty(),
        show_uninstalled: settings.show_uninstalled(),
        steam_key_set: !settings.steam_key().is_empty(),
    }
}

/// Tout ce qu'il faut pour afficher l'écran, en un seul appel au démarrage.
#[tauri::command]
fn get_state(app: AppHandle, store: AppStore, settings: AppSettings, tz: i32) -> AppState {
    // Ordre des verrous, partout : réglages puis bibliothèque.
    let settings = settings.lock_safe();
    let store = store.lock_safe();
    let state = app_state(&store, &settings, tz);
    refresh_tray(&app, &state.programs);
    state
}

/// Enregistre les launchers choisis (premier lancement ou paramètres).
#[tauri::command]
fn set_platforms(
    app: AppHandle,
    store: AppStore,
    settings: AppSettings,
    enabled: Vec<String>,
    tz: i32,
) -> Result<AppState, String> {
    let mut settings = settings.lock_safe();
    settings.set_enabled(&enabled).map_err(|e| e.to_string())?;
    let store = store.lock_safe();
    let state = app_state(&store, &settings, tz);
    refresh_tray(&app, &state.programs);
    Ok(state)
}

#[tauri::command]
fn add_program(
    app: AppHandle,
    store: AppStore,
    settings: AppSettings,
    path: String,
    tz: i32,
) -> Result<Vec<ProgramView>, String> {
    let settings = settings.lock_safe();
    let mut store = store.lock_safe();
    let id = store.add_manual(&path)?;
    store.save().map_err(|e| e.to_string())?;
    // L'icône du programme sert de pochette tant qu'il n'y en a pas de meilleure.
    if path.to_lowercase().ends_with(".exe") {
        if let Some(png) = icons::extract_png(Path::new(&path)) {
            let _ = covers::save(&store.covers_dir(), &id, Kind::Icon, &png);
        }
    }
    let programs = snapshot(&store, &settings, tz);
    refresh_tray(&app, &programs);
    if settings.download_covers() {
        start_cover_download(&app);
    }
    if settings.online_info() {
        prepare_details(&app);
    }
    Ok(programs)
}

/// Ajoute une durée de jeu à un programme et l'enregistre.
fn add_play_time(app: &AppHandle, id: &str, ms: u64, estimated: bool) {
    let store = app.state::<Mutex<Store>>();
    let mut store = store.lock_safe();
    store.add_play_time(id, ms, estimated);
    let _ = store.save();
}

/// Suit le temps de jeu d'un lancement : mesuré si Tronix a lancé l'exécutable lui-même,
/// estimé (par le dossier d'installation) si un autre launcher s'en est chargé.
fn track_play_time(app: &AppHandle, id: &str, launched: store::Launched) {
    let (app, id) = (app.clone(), id.to_string());
    if let Some(child) = launched.child {
        std::thread::spawn(move || {
            let ms = playtime::measure_child(child);
            add_play_time(&app, &id, ms, false);
        });
    } else if let Some(dir) = launched.install_dir {
        std::thread::spawn(move || {
            let ms = playtime::watch_dir(
                Path::new(&dir),
                Duration::from_secs(5),
                Duration::from_secs(120),
                Duration::from_secs(20),
            );
            if ms > 0 {
                add_play_time(&app, &id, ms, true);
            }
        });
    }
}

/// Lance un programme, l'enregistre et prévient les autres fenêtres.
/// `origin` : fenêtre d'où vient la demande, qui n'a pas besoin d'être prévenue.
fn launch_and_publish(app: &AppHandle, id: &str, tz: i32, origin: &str) -> Result<Vec<ProgramView>, String> {
    let settings = app.state::<Mutex<Settings>>();
    let store = app.state::<Mutex<Store>>();
    let (programs, launched) = {
        let settings = settings.lock_safe();
        let mut store = store.lock_safe();
        let launched = store.record_launch(id)?;
        store.save().map_err(|e| e.to_string())?;
        (snapshot(&store, &settings, tz), launched)
    };
    track_play_time(app, id, launched);
    refresh_tray(app, &programs);
    let _ = app.emit("library-changed", origin);
    Ok(programs)
}

#[tauri::command]
fn launch_program(app: AppHandle, window: tauri::Window, id: String, tz: i32) -> Result<Vec<ProgramView>, String> {
    launch_and_publish(&app, &id, tz, window.label())
}

#[derive(Serialize)]
struct ImportReport {
    programs: Vec<ProgramView>,
    /// Nombre de launchers activés, donc cherchés.
    searched: usize,
    /// Nombre de jeux ajoutés, par launcher d'origine.
    added: Vec<(String, usize)>,
}

/// Cherche les jeux des launchers activés et ajoute ceux qui manquent.
#[tauri::command]
fn import_all(app: AppHandle, store: AppStore, settings: AppSettings, tz: i32) -> Result<ImportReport, String> {
    let settings = settings.lock_safe();
    let found = imports::scan_all(|source| settings.is_enabled(source));
    let mut store = store.lock_safe();
    let mut added: Vec<(String, usize)> = Vec::new();
    let mut changed = false;
    for game in found {
        if let Some(dir) = &game.install_dir {
            store.update_install_dir(game.source, &game.key, dir);
        }
        changed |= store.mark_installed(game.source, &game.key);
        if store.add(game.source, &game.key, &game.name, &game.target) {
            match added.iter_mut().find(|(s, _)| s == game.source) {
                Some((_, n)) => *n += 1,
                None => added.push((game.source.to_string(), 1)),
            }
        }
    }
    if !added.is_empty() || changed {
        store.save().map_err(|e| e.to_string())?;
    }
    copy_steam_covers(&store);
    let programs = snapshot(&store, &settings, tz);
    refresh_tray(&app, &programs);
    if settings.download_covers() {
        start_cover_download(&app);
    }
    if settings.online_info() {
        prepare_details(&app);
    }
    Ok(ImportReport {
        programs,
        searched: settings.enabled_count(),
        added,
    })
}

// ------------------------------------------------- Jeux Steam non installés

/// Active l'affichage des jeux Steam non installés et enregistre la clé d'API si elle est donnée.
#[tauri::command]
fn set_steam_account(settings: AppSettings, enabled: bool, key: Option<String>) -> Result<(), String> {
    let mut settings = settings.lock_safe();
    settings.set_show_uninstalled(enabled).map_err(|e| e.to_string())?;
    if let Some(key) = key {
        settings.set_steam_key(&key).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize)]
struct SyncReport {
    programs: Vec<ProgramView>,
    /// Nombre de jeux possédés et non installés.
    uninstalled: usize,
}

/// Relit les jeux possédés sur le compte Steam (réseau) et met la bibliothèque à jour.
#[tauri::command]
async fn sync_owned_games(app: AppHandle, tz: i32) -> Result<SyncReport, String> {
    let key = {
        let settings = app.state::<Mutex<Settings>>();
        let settings = settings.lock_safe();
        if !settings.show_uninstalled() || !settings.is_enabled("steam") {
            return Err(i18n::t("err.uninstalled_off"));
        }
        settings.steam_key().to_string()
    };
    if key.is_empty() {
        return Err(i18n::t(steam_account::NO_KEY));
    }
    // Lectures de fichiers et réseau hors de tout verrou.
    let (installed, owned) = tauri::async_runtime::spawn_blocking(move || {
        let root = imports::steam_root().ok_or_else(|| i18n::t(steam_account::NO_STEAM))?;
        let id = steam_account::steam_id(&root).ok_or_else(|| i18n::t(steam_account::NO_STEAM))?;
        let owned = steam_account::fetch_owned(&key, &id)?;
        let installed: HashSet<String> = imports::scan_steam(&root).into_iter().map(|g| g.key).collect();
        Ok::<_, String>((installed, owned))
    })
    .await
    .map_err(|e| e.to_string())??;
    let owned: Vec<(String, String)> = owned.into_iter().map(|g| (g.appid, g.name)).collect();

    let settings = app.state::<Mutex<Settings>>();
    let store = app.state::<Mutex<Store>>();
    let settings = settings.lock_safe();
    let mut store = store.lock_safe();
    let uninstalled = store.sync_steam(&installed, &owned);
    store.save().map_err(|e| e.to_string())?;
    copy_steam_covers(&store);
    let programs = snapshot(&store, &settings, tz);
    refresh_tray(&app, &programs);
    if settings.download_covers() {
        start_cover_download(&app);
    }
    Ok(SyncReport { programs, uninstalled })
}

/// Ouvre la fenêtre d'installation de Steam pour un jeu possédé.
#[tauri::command]
fn install_program(store: AppStore, id: String) -> Result<(), String> {
    let store = store.lock_safe();
    let program = store.programs.iter().find(|p| p.id == id && p.uninstalled).ok_or_else(|| i18n::t("err.not_to_install"))?;
    let appid = steam_appid(&program.target)
        .filter(|a| !a.is_empty() && a.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| i18n::t("err.unknown_steam_game"))?;
    store::explorer()
        .arg(format!("steam://install/{appid}"))
        .spawn()
        .map(|_| ())
        .map_err(|e| i18n::tf("err.open_steam", &[("e", &e.to_string())]))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    START.get_or_init(Instant::now);
    tauri::Builder::default()
        // Une seconde instance ne démarre pas : elle ramène la fenêtre de la première.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let settings = Settings::load(dir.join("settings.json"));
            i18n::set_lang(i18n::resolve(settings.language()));
            let shortcut = settings.shortcut().to_string();
            app.manage(Mutex::new(Store::load(dir.join("programs.json"))));
            app.manage(Mutex::new(settings));

            let handle = app.handle().clone();
            if let Err(error) = register_shortcut(&handle, &shortcut) {
                SHORTCUT_OK.store(false, Ordering::Relaxed);
                eprintln!("[tronix] raccourci {shortcut} indisponible : {error}");
            }

            let menu = tray_menu(&handle, &[])?;
            let mut tray = TrayIconBuilder::with_id(TRAY_ID)
                .tooltip("Tronix Launcher")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => show_main(app),
                    "quit" => app.exit(0),
                    id => {
                        if let Some(program) = id.strip_prefix("launch:") {
                            let tz = TZ.load(Ordering::Relaxed);
                            if let Err(error) = launch_and_publish(app, program, tz, "tray") {
                                eprintln!("[tronix] lancement depuis l'icône impossible : {error}");
                            }
                        }
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if app.state::<Mutex<Settings>>().lock_safe().close_to_tray() {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    // La mini-fenêtre cachée garderait l'application en vie : on quitte explicitement.
                    app.exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_ready,
            get_state,
            get_stats,
            set_platforms,
            set_close_to_tray,
            set_shortcut,
            set_accent,
            set_look,
            reset_look,
            set_sort,
            set_favorite,
            get_strings,
            set_language,
            create_collection,
            rename_collection,
            delete_collection,
            set_program_collection,
            set_download_covers,
            set_online_info,
            set_steam_account,
            sync_owned_games,
            install_program,
            set_program_info,
            sgdb_search,
            sgdb_grids,
            sgdb_pick,
            open_details,
            get_details,
            details_candidates,
            choose_details,
            open_url,
            fetch_covers,
            set_custom_cover,
            clear_custom_cover,
            hide_quick,
            add_program,
            launch_program,
            import_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::settings::DEFAULT_SHORTCUT;
    use tauri_plugin_global_shortcut::Shortcut;

    #[test]
    fn raccourcis_acceptes_et_refuses() {
        assert!(DEFAULT_SHORTCUT.parse::<Shortcut>().is_ok());
        assert!("Ctrl+Shift+J".parse::<Shortcut>().is_ok());
        assert!("Ctrl+Alt+F5".parse::<Shortcut>().is_ok());
        // L'interface envoie « Space » ; le mot français n'existe pas pour le système.
        assert!("Ctrl+Alt+Espace".parse::<Shortcut>().is_err());
        assert!("n'importe quoi".parse::<Shortcut>().is_err());
    }
}

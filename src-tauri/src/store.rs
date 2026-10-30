use serde::{Deserialize, Serialize};
use crate::i18n;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Program {
    pub id: String,
    pub name: String,
    /// Chemin d'un exécutable ou lien de lancement (ex. steam://...).
    pub target: String,
    /// "manual", "steam", "epic", ...
    pub source: String,
    #[serde(default)]
    pub added: i64,
    /// Horodatages (ms) de chaque lancement depuis Tronix.
    #[serde(default)]
    pub launches: Vec<i64>,
    /// Temps de jeu mesuré (ms) : programmes lancés directement par Tronix.
    #[serde(default)]
    pub play_ms: u64,
    /// Temps de jeu estimé (ms) : jeux lancés par un autre launcher.
    #[serde(default)]
    pub play_est_ms: u64,
    /// Dossier d'installation, pour estimer le temps de jeu.
    #[serde(default)]
    pub install_dir: Option<String>,
    /// Titre choisi par le joueur, affiché à la place de `name`.
    #[serde(default)]
    pub custom_name: Option<String>,
    /// Description écrite par le joueur, affichée à la place de celle des sources en ligne.
    #[serde(default)]
    pub custom_description: Option<String>,
    /// Jeu Steam possédé mais pas installé : il ne se lance pas, on propose de l'installer.
    #[serde(default)]
    pub uninstalled: bool,
    /// Épinglé par le joueur : affiché en premier, dans la section « Favoris ».
    #[serde(default)]
    pub favorite: bool,
    /// Identifiants des collections du joueur où se trouve ce programme.
    #[serde(default)]
    pub collections: Vec<String>,
}

/// Identifiant utilisable dans un nom de fichier : jamais de séparateur ni de « .. » venus de l'interface.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Lit un fichier JSON de l'utilisateur ; un BOM UTF-8 (ajouté par certains éditeurs) est ignoré.
pub fn read_text(path: &Path) -> io::Result<String> {
    let text = fs::read_to_string(path)?;
    Ok(text.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(text))
}

/// Explorateur Windows, appelé par son chemin complet : jamais un `explorer.exe` trouvé ailleurs.
pub fn explorer() -> Command {
    let system = std::env::var_os("SystemRoot").map(|root| PathBuf::from(root).join("explorer.exe"));
    Command::new(system.filter(|p| p.is_file()).unwrap_or_else(|| PathBuf::from("explorer.exe")))
}

/// Liens de lancement que Tronix produit lui-même (imports) ; tout autre type de lien est refusé.
fn uri_allowed(target: &str) -> bool {
    const SCHEMES: [&str; 5] = ["steam://", "com.epicgames.launcher://", "uplay://", "link2ea://", "battlenet://"];
    let lower = target.to_ascii_lowercase();
    SCHEMES.iter().any(|s| lower.starts_with(s)) || lower.starts_with("shell:appsfolder\\")
}

pub const MAX_NAME_CHARS: usize = 80;
pub const MAX_DESCRIPTION_CHARS: usize = 5000;

impl Program {
    /// Titre à afficher : celui du joueur s'il en a mis un, sinon le titre d'origine.
    pub fn display_name(&self) -> &str {
        self.custom_name.as_deref().unwrap_or(&self.name)
    }

    /// Copie dont `name` est le titre affiché.
    pub fn with_display_name(&self) -> Program {
        Program { name: self.display_name().to_string(), ..self.clone() }
    }
}

pub struct Store {
    path: PathBuf,
    pub programs: Vec<Program>,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Identifiant stable : le même programme donne toujours le même id.
pub fn make_id(source: &str, key: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in source.bytes().chain([b'|']).chain(key.to_lowercase().bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let programs = match read_text(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
                // Fichier illisible : on le garde de côté plutôt que de l'écraser à la prochaine sauvegarde.
                let _ = fs::rename(&path, path.with_extension("json.corrupt"));
                Vec::new()
            }),
            Err(_) => Vec::new(),
        };
        Store { path, programs }
    }

    pub fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec(&self.programs)?)?;
        fs::rename(tmp, &self.path)
    }

    /// Ajoute un programme s'il n'existe pas déjà. Renvoie true si ajouté.
    pub fn add(&mut self, source: &str, key: &str, name: &str, target: &str) -> bool {
        let id = make_id(source, key);
        if self.programs.iter().any(|p| p.id == id) {
            return false;
        }
        self.programs.push(Program {
            id,
            name: name.to_string(),
            target: target.to_string(),
            source: source.to_string(),
            added: now_ms(),
            ..Default::default()
        });
        true
    }

    /// Retient le dossier d'installation d'un jeu importé (y compris ceux déjà connus).
    pub fn update_install_dir(&mut self, source: &str, key: &str, dir: &str) {
        let id = make_id(source, key);
        if let Some(p) = self.programs.iter_mut().find(|p| p.id == id) {
            if p.install_dir.as_deref() != Some(dir) {
                p.install_dir = Some(dir.to_string());
            }
        }
    }

    /// Un jeu trouvé installé redevient un jeu normal. Renvoie vrai si son état a changé.
    pub fn mark_installed(&mut self, source: &str, key: &str) -> bool {
        let id = make_id(source, key);
        match self.programs.iter_mut().find(|p| p.id == id && p.uninstalled) {
            Some(p) => {
                p.uninstalled = false;
                true
            }
            None => false,
        }
    }

    /// Range un programme dans une collection, ou l'en retire.
    pub fn set_collection(&mut self, id: &str, collection: &str, member: bool) -> Result<(), String> {
        let program = self.programs.iter_mut().find(|p| p.id == id).ok_or_else(|| i18n::t("err.unknown_program"))?;
        program.collections.retain(|c| c != collection);
        if member {
            program.collections.push(collection.to_string());
        }
        Ok(())
    }

    /// Retire une collection supprimée de tous les programmes. Renvoie vrai si un programme a changé.
    pub fn forget_collection(&mut self, collection: &str) -> bool {
        let mut changed = false;
        for p in &mut self.programs {
            let before = p.collections.len();
            p.collections.retain(|c| c != collection);
            changed |= p.collections.len() != before;
        }
        changed
    }

    /// Épingle ou désépingle un programme.
    pub fn set_favorite(&mut self, id: &str, favorite: bool) -> Result<(), String> {
        let program = self.programs.iter_mut().find(|p| p.id == id).ok_or_else(|| i18n::t("err.unknown_program"))?;
        program.favorite = favorite;
        Ok(())
    }

    /// Met à jour les jeux Steam d'après le compte : `installed` = identifiants installés sur ce PC,
    /// `owned` = (identifiant, nom) des jeux possédés. Un jeu possédé et absent du PC devient « non installé » ;
    /// un jeu non installé qui n'est plus possédé et n'a jamais été utilisé disparaît.
    /// Renvoie le nombre de jeux non installés.
    pub fn sync_steam(&mut self, installed: &HashSet<String>, owned: &[(String, String)]) -> usize {
        let owned_ids: HashSet<&str> = owned.iter().map(|(appid, _)| appid.as_str()).collect();
        for p in self.programs.iter_mut().filter(|p| p.source == "steam") {
            if let Some(appid) = p.target.strip_prefix("steam://rungameid/") {
                p.uninstalled = !installed.contains(appid);
            }
        }
        self.programs.retain(|p| {
            let unused = p.launches.is_empty() && p.play_ms == 0 && p.play_est_ms == 0;
            let owned = p.target.strip_prefix("steam://rungameid/").is_some_and(|a| owned_ids.contains(a));
            !(p.source == "steam" && p.uninstalled && unused && !owned)
        });
        for (appid, name) in owned.iter().filter(|(appid, _)| !installed.contains(appid)) {
            let id = make_id("steam", appid);
            if !self.programs.iter().any(|p| p.id == id) {
                self.programs.push(Program {
                    id,
                    name: name.clone(),
                    target: format!("steam://rungameid/{appid}"),
                    source: "steam".into(),
                    added: now_ms(),
                    uninstalled: true,
                    ..Default::default()
                });
            }
        }
        self.programs.iter().filter(|p| p.uninstalled).count()
    }

    /// Ajoute une durée de jeu (ms), mesurée ou estimée.
    pub fn add_play_time(&mut self, id: &str, ms: u64, estimated: bool) {
        if let Some(p) = self.programs.iter_mut().find(|p| p.id == id) {
            if estimated {
                p.play_est_ms += ms;
            } else {
                p.play_ms += ms;
            }
        }
    }

    /// Ajoute un programme choisi à la main et renvoie son identifiant.
    pub fn add_manual(&mut self, path: &str) -> Result<String, String> {
        let p = Path::new(path);
        if !p.is_file() {
            return Err(i18n::tf("err.file_not_found", &[("path", path)]));
        }
        // Seuls les types que « Ajouter » propose : un chemin venu de l'interface ne peut pas viser un autre fichier.
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if !["exe", "lnk", "bat", "cmd"].contains(&ext.as_str()) {
            return Err(i18n::t("err.program_types"));
        }
        let name = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        self.add("manual", path, &name, path);
        Ok(make_id("manual", path))
    }

    /// Enregistre le titre et la description choisis par le joueur. Un texte vide rétablit l'original.
    /// Renvoie vrai si le titre affiché a changé.
    pub fn set_info(&mut self, id: &str, name: &str, description: &str) -> Result<bool, String> {
        let name = name.trim();
        let description = description.trim();
        if name.chars().count() > MAX_NAME_CHARS {
            return Err(i18n::tf("err.title_too_long", &[("max", &MAX_NAME_CHARS.to_string())]));
        }
        if description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(i18n::tf("err.description_too_long", &[("max", &MAX_DESCRIPTION_CHARS.to_string())]));
        }
        let program = self.programs.iter_mut().find(|p| p.id == id).ok_or_else(|| i18n::t("err.unknown_program"))?;
        let before = program.display_name().to_string();
        // Un titre identique à l'original n'est pas une modification.
        program.custom_name = (!name.is_empty() && name != program.name).then(|| name.to_string());
        program.custom_description = (!description.is_empty()).then(|| description.to_string());
        Ok(program.display_name() != before)
    }

    /// Dossier des pages (informations en ligne), à côté du fichier de la bibliothèque.
    pub fn details_dir(&self) -> PathBuf {
        crate::details::dir_for(&self.path)
    }

    /// Dossier des pochettes, à côté du fichier de la bibliothèque.
    pub fn covers_dir(&self) -> PathBuf {
        crate::covers::dir_for(&self.path)
    }

    /// Lance le programme et enregistre le lancement.
    pub fn record_launch(&mut self, id: &str) -> Result<Launched, String> {
        let p = self
            .programs
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| i18n::t("err.unknown_program"))?;
        if p.uninstalled {
            return Err(i18n::t("err.not_installed"));
        }
        let child = spawn(&p.target)?;
        p.launches.push(now_ms());
        Ok(Launched {
            child,
            install_dir: p.install_dir.clone(),
        })
    }
}

/// Ce qu'il faut pour suivre le temps de jeu d'un lancement.
pub struct Launched {
    /// Le processus, quand Tronix a lancé directement un exécutable (durée mesurable).
    pub child: Option<Child>,
    /// Dossier d'installation, pour estimer la durée quand un autre launcher lance le jeu.
    pub install_dir: Option<String>,
}

/// Lance la cible. Renvoie le processus quand c'est un exécutable lancé directement.
fn spawn(target: &str) -> Result<Option<Child>, String> {
    let is_uri = target.contains("://") || target.starts_with("shell:");
    if is_uri && !uri_allowed(target) {
        return Err(i18n::t("err.link_type"));
    }
    let path = Path::new(target);
    if !is_uri && !path.exists() {
        return Err(i18n::tf("err.file_not_found", &[("path", target)]));
    }
    let mut cmd;
    let direct = !is_uri && target.to_lowercase().ends_with(".exe");
    if direct {
        cmd = Command::new(target);
        if let Some(dir) = path.parent() {
            cmd.current_dir(dir);
        }
    } else {
        // Le shell de Windows ouvre liens et fichiers. On évite `cmd /C start`, qui coupe la cible au premier `&`.
        cmd = explorer();
        cmd.arg(target);
    }
    cmd.spawn()
        .map(|child| direct.then_some(child))
        .map_err(|e| i18n::tf("err.launch_failed", &[("e", &e.to_string())]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ajout_manuel_sans_doublon() {
        let tmp = std::env::temp_dir().join("tronix-test-store.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Store::load(tmp.clone());
        let exe = r"C:\Windows\System32\notepad.exe";
        s.add_manual(exe).unwrap();
        s.add_manual(exe).unwrap();
        assert_eq!(s.programs.len(), 1);
        assert_eq!(s.programs[0].name, "notepad");
        s.save().unwrap();
        assert_eq!(Store::load(tmp.clone()).programs.len(), 1);
        assert!(s.add_manual(r"C:\nope\absent.exe").is_err());
        assert!(s.add_manual(r"C:\Windows\System32\drivers\etc\hosts").is_err()); // existe, mais n'est pas un programme
        assert_eq!(s.programs.len(), 1);
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn fichier_illisible_mis_de_cote() {
        let dir = std::env::temp_dir().join("tronix-test-corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("programs.json");
        let broken = r#"[{"id":"a1","name":"important","target":"C:\\x.exe","source":"manual","added":1,"launches":[]},{"id":"a2","na"#;
        fs::write(&file, broken).unwrap();
        let mut s = Store::load(file.clone());
        assert!(s.programs.is_empty());
        // La sauvegarde suivante ne détruit plus l'ancien contenu.
        s.add("manual", "k", "nouveau", "C:\\y.exe");
        s.save().unwrap();
        assert_eq!(fs::read_to_string(dir.join("programs.json.corrupt")).unwrap(), broken);
        assert_eq!(Store::load(file).programs.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn lancement_avec_esperluette() {
        let dir = std::env::temp_dir().join("tronix-test-esperluette");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let proof = dir.join("preuve.txt");
        let bat = dir.join("a&b.bat");
        fs::write(&bat, format!("@echo ok> \"{}\"\r\n", proof.display())).unwrap();
        spawn(&bat.to_string_lossy()).unwrap();
        for _ in 0..50 {
            if proof.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        assert!(proof.exists(), "le fichier à & n'a pas été lancé");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn titre_et_description_du_joueur() {
        let mut s = Store::load(std::env::temp_dir().join("tronix-test-info.json"));
        s.add("manual", "k", "blender-launcher", "C:\\a.exe");
        let id = s.programs[0].id.clone();
        assert_eq!(s.programs[0].display_name(), "blender-launcher");

        assert!(s.set_info(&id, "  Blender  ", " Modeleur 3D. ").unwrap());
        assert_eq!(s.programs[0].display_name(), "Blender");
        assert_eq!(s.programs[0].name, "blender-launcher"); // l'original reste
        assert_eq!(s.programs[0].custom_description.as_deref(), Some("Modeleur 3D."));
        assert_eq!(s.programs[0].with_display_name().name, "Blender");

        // Mêmes valeurs : pas de changement de titre. Titre identique à l'original : aucune modification.
        assert!(!s.set_info(&id, "Blender", "Autre description").unwrap());
        assert!(s.set_info(&id, "blender-launcher", "Autre description").unwrap());
        assert_eq!(s.programs[0].custom_name, None);

        // Texte vide : retour à l'original.
        s.set_info(&id, "Titre", "Texte").unwrap();
        assert!(s.set_info(&id, "", "").unwrap());
        assert_eq!((s.programs[0].custom_name.clone(), s.programs[0].custom_description.clone()), (None, None));

        // Limites et programme inconnu.
        assert!(s.set_info(&id, &"x".repeat(MAX_NAME_CHARS + 1), "").is_err());
        assert!(s.set_info(&id, "", &"x".repeat(MAX_DESCRIPTION_CHARS + 1)).is_err());
        assert!(s.set_info("inconnu", "a", "b").is_err());
        assert_eq!(s.programs[0].custom_name, None); // un refus ne change rien
    }

    #[test]
    fn modifications_conservees_sur_disque() {
        let tmp = std::env::temp_dir().join("tronix-test-info2.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Store::load(tmp.clone());
        s.add("manual", "k", "orig", "C:\\a.exe");
        let id = s.programs[0].id.clone();
        s.set_info(&id, "Mon titre", "Ma description").unwrap();
        s.save().unwrap();
        let again = Store::load(tmp.clone());
        assert_eq!(again.programs[0].display_name(), "Mon titre");
        assert_eq!(again.programs[0].custom_description.as_deref(), Some("Ma description"));
        // Un ancien fichier sans ces champs reste lisible.
        fs::write(&tmp, r#"[{"id":"a","name":"Ancien","target":"x","source":"manual"}]"#).unwrap();
        assert_eq!(Store::load(tmp.clone()).programs[0].display_name(), "Ancien");
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn jeux_steam_non_installes() {
        let mut s = Store::load(std::env::temp_dir().join("tronix-test-uninstalled.json"));
        s.add("steam", "10", "Installé", "steam://rungameid/10");
        s.add("steam", "20", "Désinstallé et joué", "steam://rungameid/20");
        s.add("steam", "30", "Désinstallé jamais joué", "steam://rungameid/30");
        s.add("manual", "m", "Logiciel", "C:\\a.exe");
        s.programs[1].launches.push(1);
        let installed: HashSet<String> = ["10".to_string()].into();
        let owned = vec![
            ("10".to_string(), "Installé".to_string()),
            ("20".to_string(), "Joué".to_string()),
            ("40".to_string(), "Nouveau".to_string()),
        ];
        assert_eq!(s.sync_steam(&installed, &owned), 2);
        let by = |n: &str| s.programs.iter().find(|p| p.name == n).cloned();
        assert!(!by("Installé").unwrap().uninstalled);
        assert!(by("Désinstallé et joué").unwrap().uninstalled);
        assert!(by("Désinstallé jamais joué").is_none()); // plus possédé, jamais utilisé : retiré
        let new = by("Nouveau").unwrap();
        assert!(new.uninstalled && new.target == "steam://rungameid/40" && new.source == "steam");
        assert_eq!(new.id, make_id("steam", "40")); // même id que s'il était installé
        assert!(!by("Logiciel").unwrap().uninstalled);

        // Pas de doublon à la synchronisation suivante ; lancer un jeu non installé est refusé.
        s.sync_steam(&installed, &owned);
        assert_eq!(s.programs.iter().filter(|p| p.name == "Nouveau").count(), 1);
        assert!(s.record_launch(&make_id("steam", "40")).is_err());

        // Une fois installé, il redevient un jeu normal (même entrée, même id).
        assert!(s.mark_installed("steam", "40"));
        assert!(!s.mark_installed("steam", "40"));
        assert!(!s.programs.iter().find(|p| p.name == "Nouveau").unwrap().uninstalled);
    }

    #[test]
    fn favoris() {
        let tmp = std::env::temp_dir().join("tronix-test-fav.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Store::load(tmp.clone());
        s.add("manual", "k", "Jeu", "C:\\a.exe");
        let id = s.programs[0].id.clone();
        assert!(!s.programs[0].favorite);
        s.set_favorite(&id, true).unwrap();
        s.save().unwrap();
        assert!(Store::load(tmp.clone()).programs[0].favorite);
        s.set_favorite(&id, false).unwrap();
        assert!(!s.programs[0].favorite);
        assert!(s.set_favorite("inconnu", true).is_err());
        // Un favori survit à la synchronisation Steam et à l'import.
        s.add("steam", "10", "Steam", "steam://rungameid/10");
        let steam = s.programs[1].id.clone();
        s.set_favorite(&steam, true).unwrap();
        s.sync_steam(&["10".to_string()].into(), &[("10".into(), "Steam".into())]);
        assert!(s.programs[1].favorite);
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn collections_des_programmes() {
        let mut s = Store::load(std::env::temp_dir().join("tronix-test-coll.json"));
        s.add("manual", "a", "A", "C:\\a.exe");
        s.add("manual", "b", "B", "C:\\b.exe");
        let (a, b) = (s.programs[0].id.clone(), s.programs[1].id.clone());
        s.set_collection(&a, "c1", true).unwrap();
        s.set_collection(&a, "c1", true).unwrap(); // pas de doublon
        s.set_collection(&a, "c2", true).unwrap();
        s.set_collection(&b, "c1", true).unwrap();
        assert_eq!(s.programs[0].collections, ["c1", "c2"]);
        s.set_collection(&a, "c2", false).unwrap();
        assert_eq!(s.programs[0].collections, ["c1"]);
        assert!(s.set_collection("inconnu", "c1", true).is_err());
        assert!(s.forget_collection("c1"));
        assert!(!s.forget_collection("c1"));
        assert!(s.programs.iter().all(|p| p.collections.is_empty()));
        // Un ancien fichier sans collections reste lisible.
        let old: Program = serde_json::from_str(r#"{"id":"x","name":"n","target":"t","source":"manual"}"#).unwrap();
        assert!(old.collections.is_empty());
    }

    #[test]
    fn identifiants_surs_liens_et_bom() {
        assert!(is_safe_id("a1") && is_safe_id("0123456789abcdef") && is_safe_id("p_1-b"));
        for bad in ["", "..", ".", "a/b", "a\\b", "../x", "x.y", "a b", "C:", &"x".repeat(65)] {
            assert!(!is_safe_id(bad), "{bad}");
        }
        assert!(uri_allowed("steam://rungameid/10") && uri_allowed("STEAM://run/1"));
        assert!(uri_allowed("com.epicgames.launcher://apps/a?action=launch&silent=true"));
        assert!(uri_allowed("uplay://launch/1/0") && uri_allowed("link2ea://launchgame/1") && uri_allowed("battlenet://WoW"));
        assert!(uri_allowed(r"shell:AppsFolder\Microsoft.X_8wekyb3d8bbwe!Game"));
        for bad in ["file:///C:/Windows/System32/calc.exe", "ms-msdt://x", "javascript://x", "http://example.com", "shell:startup", "ftp://x", "steam:rungameid/1"] {
            assert!(!uri_allowed(bad), "{bad}");
            assert!(spawn(bad).is_err(), "{bad}");
        }
        let tmp = std::env::temp_dir().join("tronix-test-bom.json");
        fs::write(&tmp, "\u{feff}[{\"id\":\"a\",\"name\":\"n\",\"target\":\"t\",\"source\":\"manual\"}]").unwrap();
        assert_eq!(Store::load(tmp.clone()).programs.len(), 1);
        assert!(!tmp.with_extension("json.corrupt").exists());
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn lancement_introuvable() {
        assert!(spawn(r"C:\nope\absent.exe").is_err());
    }
}

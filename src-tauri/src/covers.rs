//! Pochettes : images déjà présentes sur le PC, image choisie par le joueur, téléchargement.
//!
//! Chaque programme a au plus un fichier par rang dans le dossier `covers` :
//!   `<id>.custom.<ext>`  image choisie par le joueur (la plus prioritaire)
//!   `<id>.<ext>`         pochette trouvée sur le PC ou téléchargée
//!   `<id>.icon.png`      icône du programme

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use std::{fs, io};

use crate::i18n;
use crate::store::is_safe_id;

const EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];
/// Taille maximale d'une image acceptée, pour ne pas remplir le disque ni la mémoire.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Custom,
    Art,
    Icon,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Custom => "custom",
            Kind::Art => "art",
            Kind::Icon => "icon",
        }
    }
}

pub struct Cover {
    pub path: String,
    /// Date de modification (secondes) : change quand l'image est remplacée.
    pub version: u64,
    pub kind: Kind,
}

/// Type d'image reconnu à ses premiers octets (on ne se fie pas à l'extension).
pub fn image_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// Analyse un nom de fichier du dossier : (identifiant du programme, rang).
fn parse_name(name: &str) -> Option<(String, Kind)> {
    let (stem, ext) = name.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    if !EXTENSIONS.contains(&ext.as_str()) {
        return None;
    }
    if let Some(id) = stem.strip_suffix(".custom") {
        Some((id.to_string(), Kind::Custom))
    } else if let Some(id) = stem.strip_suffix(".icon") {
        (ext == "png").then(|| (id.to_string(), Kind::Icon))
    } else {
        Some((stem.to_string(), Kind::Art))
    }
}

fn rank(kind: Kind) -> u8 {
    match kind {
        Kind::Custom => 3,
        Kind::Art => 2,
        Kind::Icon => 1,
    }
}

/// Meilleure image de chaque programme, en une seule lecture du dossier.
pub fn scan(dir: &Path) -> HashMap<String, Cover> {
    let mut best: HashMap<String, Cover> = HashMap::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return best;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((id, kind)) = parse_name(&name) else {
            continue;
        };
        if best.get(&id).is_some_and(|c| rank(c.kind) >= rank(kind)) {
            continue;
        }
        let version = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        best.insert(
            id,
            Cover {
                path: entry.path().to_string_lossy().into_owned(),
                version,
                kind,
            },
        );
    }
    best
}

fn base_name(id: &str, kind: Kind, ext: &str) -> String {
    match kind {
        Kind::Custom => format!("{id}.custom.{ext}"),
        Kind::Art => format!("{id}.{ext}"),
        Kind::Icon => format!("{id}.icon.png"),
    }
}

/// Supprime les fichiers d'un programme pour un rang donné (toutes extensions).
pub fn remove(dir: &Path, id: &str, kind: Kind) {
    if !is_safe_id(id) {
        return;
    }
    for ext in EXTENSIONS {
        let _ = fs::remove_file(dir.join(base_name(id, kind, ext)));
    }
}

/// Enregistre une image pour un programme, en remplaçant l'ancienne de même rang.
pub fn save(dir: &Path, id: &str, kind: Kind, bytes: &[u8]) -> Result<(), String> {
    if !is_safe_id(id) {
        return Err(i18n::t("err.bad_id"));
    }
    let ext = if kind == Kind::Icon {
        "png"
    } else {
        image_ext(bytes).ok_or_else(|| i18n::t("err.not_an_image"))?
    };
    if bytes.len() as u64 > MAX_BYTES {
        return Err(i18n::t("err.image_too_big"));
    }
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    remove(dir, id, kind);
    let target = dir.join(base_name(id, kind, ext));
    let tmp = target.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    fs::rename(tmp, target).map_err(|e| e.to_string())
}

/// Image choisie par le joueur : lue depuis son PC, copiée dans le dossier des pochettes.
pub fn set_custom(dir: &Path, id: &str, source: &Path) -> Result<(), String> {
    // Un vrai fichier seulement : pas un dossier, ni un tube ou un périphérique qui bloquerait la lecture.
    if !source.is_file() {
        return Err(i18n::t("err.not_a_file"));
    }
    let len = fs::metadata(source).map_err(|e| e.to_string())?.len();
    if len > MAX_BYTES {
        return Err(i18n::t("err.image_too_big"));
    }
    let bytes = fs::read(source).map_err(|e| e.to_string())?;
    save(dir, id, Kind::Custom, &bytes)
}

// ------------------------------------------------------------ Steam, en local

/// Images portrait de Steam par ordre de préférence ; l'en-tête paysage sert de dernier recours.
const STEAM_FILES: [&str; 3] = ["library_600x900.jpg", "library_capsule.jpg", "library_header.jpg"];

/// Pochette que Steam garde déjà sur le PC, sans Internet.
pub fn steam_local(steam_root: &Path, appid: &str) -> Option<Vec<u8>> {
    let dir = steam_root.join("appcache").join("librarycache").join(appid);
    for wanted in STEAM_FILES {
        let mut folders = vec![dir.clone()];
        if let Ok(entries) = fs::read_dir(&dir) {
            folders.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
        }
        for folder in folders {
            let file = folder.join(wanted);
            if file.is_file() {
                if let Ok(bytes) = fs::read(&file) {
                    if image_ext(&bytes).is_some() {
                        return Some(bytes);
                    }
                }
            }
        }
    }
    None
}

// -------------------------------------------------------------- Téléchargement

/// Agent qui n'ouvre que des liens https et ne suit pas les redirections : celles-ci sont vérifiées à la main.
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .https_only(true)
        .redirects(0)
        .user_agent("TronixLauncher/0.1")
        .build()
}

/// Message d'erreur réseau sans l'adresse : elle peut contenir une clé d'API.
pub fn net_error(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Status(code, _) => i18n::tf("err.service_status", &[("code", &code.to_string())]),
        ureq::Error::Transport(_) => i18n::t("err.connection"),
    }
}

/// Adresse d'image acceptée : https, nom de domaine (ni adresse IP, ni nom local), sans identifiants.
pub fn remote_url_ok(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = match authority.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) => host,
        _ => authority,
    };
    let numeric = host.bytes().all(|b| b.is_ascii_digit() || b == b'.');
    !host.is_empty()
        && host.contains('.')
        && !numeric
        && !host.contains(['@', '[', ']', ' ', '\\'])
        && host != "localhost"
        && !host.ends_with(".local")
        && !host.ends_with(".internal")
        && !host.ends_with(".localhost")
}

/// Lit au plus `MAX_BYTES` octets d'une réponse.
fn read_limited(response: ureq::Response) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    response.into_reader().take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::other(i18n::t("err.image_too_big")));
    }
    Ok(bytes)
}

/// Adresse d'une redirection, absolue ; `None` si elle n'est pas lisible.
fn redirect_target(current: &str, location: &str) -> Option<String> {
    if location.starts_with("https://") {
        return Some(location.to_string());
    }
    let host = current.strip_prefix("https://")?.split(['/', '?', '#']).next()?;
    location.starts_with('/').then(|| format!("https://{host}{location}"))
}

/// Télécharge une image : trois redirections au plus, chaque adresse vérifiée, corps limité, type contrôlé.
pub fn download_image(url: &str, bearer: Option<&str>) -> Result<Vec<u8>, String> {
    let mut url = url.to_string();
    for _ in 0..4 {
        if !remote_url_ok(&url) {
            return Err(i18n::t("err.image_url_refused"));
        }
        let mut request = agent().get(&url);
        if let Some(key) = bearer {
            request = request.set("Authorization", &format!("Bearer {key}"));
        }
        let response = request.call().map_err(|e| net_error(&e))?;
        if (300..400).contains(&response.status()) {
            url = response
                .header("Location")
                .and_then(|location| redirect_target(&url, location))
                .ok_or_else(|| i18n::t("err.redirect_refused"))?;
            continue;
        }
        let bytes = read_limited(response).map_err(|e| e.to_string())?;
        image_ext(&bytes).ok_or_else(|| i18n::t("err.response_not_image"))?;
        return Ok(bytes);
    }
    Err(i18n::t("err.too_many_redirects"))
}

/// Adresses publiques des images d'un jeu Steam : pochette portrait, puis en-tête.
pub fn steam_cdn_urls(appid: &str) -> [String; 2] {
    let base = format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}");
    [format!("{base}/library_600x900.jpg"), format!("{base}/header.jpg")]
}

pub fn steam_download(appid: &str) -> Option<Vec<u8>> {
    steam_cdn_urls(appid).iter().find_map(|url| download_image(url, None).ok())
}

fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Premier résultat de SteamGridDB pour ce nom de jeu, au format portrait. Nécessite une clé d'API.
pub fn steamgriddb_download(name: &str, key: &str) -> Option<Vec<u8>> {
    let get_json = |url: String| -> Option<serde_json::Value> {
        let response = agent().get(&url).set("Authorization", &format!("Bearer {key}")).call().ok()?;
        let mut text = String::new();
        response.into_reader().take(4 * 1024 * 1024).read_to_string(&mut text).ok()?;
        serde_json::from_str(&text).ok()
    };
    let search = get_json(format!(
        "https://www.steamgriddb.com/api/v2/search/autocomplete/{}",
        percent_encode(name)
    ))?;
    let game_id = search.get("data")?.get(0)?.get("id")?.as_i64()?;
    let grids = get_json(format!(
        "https://www.steamgriddb.com/api/v2/grids/game/{game_id}?dimensions=600x900&types=static"
    ))?;
    let url = grids.get("data")?.get(0)?.get("url")?.as_str()?.to_string();
    // L'image elle-même est publique : on n'y envoie pas la clé.
    download_image(&url, None).ok()
}

// ------------------------------------------------------ Sélecteur SteamGridDB

#[derive(serde::Serialize, Debug, PartialEq)]
pub struct SgdbGame {
    pub id: i64,
    pub name: String,
    pub year: Option<i64>,
}

#[derive(serde::Serialize, Debug, PartialEq)]
pub struct SgdbGrid {
    pub id: i64,
    pub url: String,
    pub thumb: String,
    pub style: String,
    pub author: String,
}

const SGDB_GAMES: usize = 8;
const SGDB_GRIDS: usize = 24;

/// Seules les images hébergées par SteamGridDB sont acceptées (https uniquement).
pub fn sgdb_host_ok(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    !host.contains('@') && (host == "steamgriddb.com" || host.ends_with(".steamgriddb.com"))
}

pub fn parse_sgdb_games(v: &serde_json::Value) -> Vec<SgdbGame> {
    v.get("data")
        .and_then(|d| d.as_array())
        .map(|games| {
            games
                .iter()
                .filter_map(|g| {
                    Some(SgdbGame {
                        id: g.get("id")?.as_i64()?,
                        name: g.get("name")?.as_str()?.to_string(),
                        // Date de sortie en secondes depuis 1970 : l'année suffit à les distinguer.
                        year: g.get("release_date").and_then(|d| d.as_i64()).map(|secs| 1970 + secs / 31_556_952),
                    })
                })
                .take(SGDB_GAMES)
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_sgdb_grids(v: &serde_json::Value) -> Vec<SgdbGrid> {
    v.get("data")
        .and_then(|d| d.as_array())
        .map(|grids| {
            grids
                .iter()
                .filter(|g| !g.get("nsfw").and_then(|n| n.as_bool()).unwrap_or(false))
                .filter_map(|g| {
                    let url = g.get("url")?.as_str()?;
                    if !sgdb_host_ok(url) {
                        return None;
                    }
                    let thumb = g.get("thumb").and_then(|t| t.as_str()).filter(|t| sgdb_host_ok(t)).unwrap_or(url);
                    Some(SgdbGrid {
                        id: g.get("id")?.as_i64()?,
                        url: url.to_string(),
                        thumb: thumb.to_string(),
                        style: g.get("style").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        author: g.get("author").and_then(|a| a.get("name")).and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    })
                })
                .take(SGDB_GRIDS)
                .collect()
        })
        .unwrap_or_default()
}

fn sgdb_get(url: &str, key: &str) -> Result<serde_json::Value, String> {
    if key.is_empty() {
        return Err(i18n::t("err.sgdb_need_key"));
    }
    let response = agent()
        .get(url)
        .set("Authorization", &format!("Bearer {key}"))
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(401 | 403, _) => i18n::t("err.sgdb_key_refused"),
            other => i18n::tf("err.sgdb_unreachable", &[("e", &net_error(&other))]),
        })?;
    if !(200..300).contains(&response.status()) {
        return Err(i18n::t("err.sgdb_unexpected"));
    }
    // Corps limité : une réponse géante ne doit pas remplir la mémoire.
    let mut text = String::new();
    response.into_reader().take(4 * 1024 * 1024).read_to_string(&mut text).map_err(|_| i18n::t("err.sgdb_unexpected"))?;
    serde_json::from_str(&text).map_err(|_| i18n::t("err.sgdb_unexpected"))
}

pub fn sgdb_search_games(query: &str, key: &str) -> Result<Vec<SgdbGame>, String> {
    let url = format!("https://www.steamgriddb.com/api/v2/search/autocomplete/{}", percent_encode(query.trim()));
    Ok(parse_sgdb_games(&sgdb_get(&url, key)?))
}

/// Pochettes verticales d'un jeu, sans contenu pour adultes.
pub fn sgdb_grids(game_id: i64, key: &str) -> Result<Vec<SgdbGrid>, String> {
    let url = format!("https://www.steamgriddb.com/api/v2/grids/game/{game_id}?dimensions=600x900&types=static&nsfw=false");
    Ok(parse_sgdb_grids(&sgdb_get(&url, key)?))
}

/// Télécharge l'image choisie (l'adresse doit être une image de SteamGridDB).
pub fn sgdb_download(url: &str) -> Result<Vec<u8>, String> {
    if !sgdb_host_ok(url) {
        return Err(i18n::t("err.not_sgdb_image"));
    }
    download_image(url, None)
}

/// Dossier des pochettes, à côté de la bibliothèque.
pub fn dir_for(library_file: &Path) -> PathBuf {
    library_file.parent().map_or_else(|| PathBuf::from("covers"), |p| p.join("covers"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const JPG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0x10, b'J', b'F', b'I', b'F'];
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn adresses_distantes_et_identifiants() {
        for ok in ["https://cdn2.steamgriddb.com/grid/a.png", "https://media.rawg.io/x.jpg", "https://cdn.cloudflare.steamstatic.com:443/a.jpg"] {
            assert!(remote_url_ok(ok), "{ok}");
        }
        for bad in [
            "http://cdn.example.com/a.png", "https://127.0.0.1/a.png", "https://10.0.0.5/a", "https://[::1]/a", "https://localhost/a",
            "https://user@evil.example.com/a", "https://nas.local/a", "https://srv.internal/a", "https://exemple/a", "https:///a", "ftp://a.b/c",
            "file:///C:/x.png", "https://a.b\\c/d", "",
        ] {
            assert!(!remote_url_ok(bad), "{bad}");
        }
        assert_eq!(redirect_target("https://a.b/x/y", "/z.png").as_deref(), Some("https://a.b/z.png"));
        assert_eq!(redirect_target("https://a.b/x", "https://c.d/z.png").as_deref(), Some("https://c.d/z.png"));
        assert_eq!(redirect_target("https://a.b/x", "http://c.d/z.png"), None);
        assert_eq!(redirect_target("https://a.b/x", "z.png"), None);
        // Un identifiant venu de l'interface ne sort jamais du dossier.
        let dir = std::env::temp_dir().join("tronix-test-covers-id");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sous")).unwrap();
        let victim = dir.join("victime.custom.png");
        fs::write(&victim, b"x").unwrap();
        remove(&dir.join("sous"), "..\\victime", Kind::Custom);
        remove(&dir.join("sous"), "../victime", Kind::Custom);
        assert!(victim.exists());
        assert!(save(&dir.join("sous"), "../evil", Kind::Custom, PNG).is_err());
        assert!(!dir.join("evil.custom.png").exists());
        assert!(set_custom(&dir.join("sous"), "ok", &dir).is_err()); // un dossier n'est pas une image
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn types_d_image_reconnus_par_le_contenu() {
        assert_eq!(image_ext(JPG), Some("jpg"));
        assert_eq!(image_ext(PNG), Some("png"));
        assert_eq!(image_ext(b"RIFF\x00\x00\x00\x00WEBPVP8 "), Some("webp"));
        assert_eq!(image_ext(b"<html>pas une image</html>"), None);
        assert_eq!(image_ext(&[]), None);
    }

    #[test]
    fn noms_de_fichiers() {
        assert_eq!(parse_name("abc.jpg"), Some(("abc".into(), Kind::Art)));
        assert_eq!(parse_name("abc.custom.PNG"), Some(("abc".into(), Kind::Custom)));
        assert_eq!(parse_name("abc.icon.png"), Some(("abc".into(), Kind::Icon)));
        assert_eq!(parse_name("abc.icon.jpg"), None);
        assert_eq!(parse_name("abc.txt"), None);
        assert_eq!(parse_name("sans_extension"), None);
    }

    #[test]
    fn rang_des_images() {
        let dir = temp("tronix-test-covers-rank");
        save(&dir, "g1", Kind::Icon, PNG).unwrap();
        assert_eq!(scan(&dir)["g1"].kind, Kind::Icon);
        save(&dir, "g1", Kind::Art, JPG).unwrap();
        assert_eq!(scan(&dir)["g1"].kind, Kind::Art);
        save(&dir, "g1", Kind::Custom, PNG).unwrap();
        assert_eq!(scan(&dir)["g1"].kind, Kind::Custom);
        // Retirer l'image du joueur ramène la pochette ; une autre extension remplace l'ancienne.
        remove(&dir, "g1", Kind::Custom);
        assert_eq!(scan(&dir)["g1"].kind, Kind::Art);
        save(&dir, "g1", Kind::Art, PNG).unwrap();
        let arts: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| parse_name(&e.file_name().to_string_lossy()).is_some_and(|(_, k)| k == Kind::Art))
            .collect();
        assert_eq!(arts.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fichiers_refuses() {
        let dir = temp("tronix-test-covers-refus");
        assert!(save(&dir, "g1", Kind::Custom, b"pas une image").is_err());
        let source = dir.join("faux.png");
        fs::write(&source, b"texte").unwrap();
        assert!(set_custom(&dir, "g1", &source).is_err());
        assert!(scan(&dir).get("g1").is_none());
        let real = dir.join("vrai.png");
        fs::write(&real, PNG).unwrap();
        set_custom(&dir, "g1", &real).unwrap();
        assert_eq!(scan(&dir)["g1"].kind, Kind::Custom);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pochette_locale_de_steam() {
        let root = temp("tronix-test-steam-covers");
        let hash = root.join("appcache").join("librarycache").join("42").join("abcdef");
        fs::create_dir_all(&hash).unwrap();
        fs::write(hash.join("library_header.jpg"), JPG).unwrap();
        // Seul l'en-tête existe : on le prend.
        assert_eq!(steam_local(&root, "42").as_deref(), Some(JPG));
        // Une pochette portrait est préférée à l'en-tête.
        fs::write(hash.join("library_capsule.jpg"), [JPG, b"portrait"].concat()).unwrap();
        assert!(steam_local(&root, "42").unwrap().ends_with(b"portrait"));
        assert!(steam_local(&root, "43").is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn selecteur_steamgriddb() {
        assert!(sgdb_host_ok("https://cdn2.steamgriddb.com/grid/abc.png"));
        assert!(sgdb_host_ok("https://steamgriddb.com/x"));
        assert!(!sgdb_host_ok("http://cdn2.steamgriddb.com/grid/abc.png")); // pas de http
        assert!(!sgdb_host_ok("https://evil.com/steamgriddb.com/a.png"));
        assert!(!sgdb_host_ok("https://steamgriddb.com.evil.com/a.png"));
        assert!(!sgdb_host_ok("https://steamgriddb.com@evil.com/a.png"));
        assert!(!sgdb_host_ok("file:///C:/a.png"));
        assert!(sgdb_download("https://example.com/a.png").is_err());

        let games = serde_json::json!({"success": true, "data": [
            {"id": 5254127, "name": "Dota 2", "release_date": 1373587200, "verified": true},
            {"id": 2, "name": "Sans date"},
            {"name": "Sans identifiant"}
        ]});
        assert_eq!(
            parse_sgdb_games(&games),
            vec![
                SgdbGame { id: 5254127, name: "Dota 2".into(), year: Some(2013) },
                SgdbGame { id: 2, name: "Sans date".into(), year: None },
            ]
        );

        let grids = serde_json::json!({"success": true, "data": [
            {"id": 1, "nsfw": false, "style": "alternate", "url": "https://cdn2.steamgriddb.com/grid/a.png",
             "thumb": "https://cdn2.steamgriddb.com/thumb/a.jpg", "author": {"name": "Auteur"}},
            {"id": 2, "nsfw": true, "url": "https://cdn2.steamgriddb.com/grid/b.png"},
            {"id": 3, "url": "https://evil.com/c.png"},
            {"id": 4, "url": "https://cdn2.steamgriddb.com/grid/d.png", "thumb": "https://evil.com/t.jpg"}
        ]});
        let found = parse_sgdb_grids(&grids);
        assert_eq!(found.iter().map(|g| g.id).collect::<Vec<_>>(), [1, 4]);
        assert_eq!(found[0].thumb, "https://cdn2.steamgriddb.com/thumb/a.jpg");
        assert_eq!(found[0].author, "Auteur");
        // Une miniature d'un autre hébergeur est remplacée par l'image elle-même.
        assert_eq!(found[1].thumb, "https://cdn2.steamgriddb.com/grid/d.png");
        // Sans clé, aucun appel réseau : un message clair.
        assert!(sgdb_search_games("Dota", "").unwrap_err().contains("clé SteamGridDB"));
        assert!(sgdb_grids(1, "").is_err());
    }

    #[test]
    fn adresses_et_encodage() {
        let urls = steam_cdn_urls("3241660");
        assert!(urls[0].ends_with("/apps/3241660/library_600x900.jpg"));
        assert!(urls[1].ends_with("/apps/3241660/header.jpg"));
        assert_eq!(percent_encode("Far Cry 6 & co"), "Far%20Cry%206%20%26%20co");
        assert_eq!(percent_encode("a-b_c.d~e"), "a-b_c.d~e");
    }
}

//! Détection des jeux installés par les autres launchers.

use std::fs;
use std::path::{Path, PathBuf};

/// Un jeu trouvé chez un autre launcher.
pub struct Found {
    pub source: &'static str,
    /// Identifiant propre au launcher d'origine (évite les doublons).
    pub key: String,
    pub name: String,
    /// Chemin de l'exécutable ou lien de lancement.
    pub target: String,
    /// Dossier d'installation, pour estimer le temps de jeu.
    pub install_dir: Option<String>,
}

/// Valeur de la première ligne `"clé"  "valeur"` d'un fichier au format Valve (VDF/ACF).
fn vdf_value(text: &str, key: &str) -> Option<String> {
    vdf_values(text, key).into_iter().next()
}

pub fn vdf_values(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split('"');
            parts.next()?;
            if parts.next()? != key {
                return None;
            }
            parts.next()?;
            parts.next().map(|v| v.replace("\\\\", "\\"))
        })
        .collect()
}

fn epic_manifests() -> Option<PathBuf> {
    let data = std::env::var_os("ProgramData")?;
    Some(PathBuf::from(data).join(r"Epic\EpicGamesLauncher\Data\Manifests"))
}

#[cfg(windows)]
fn registry_key_exists(path: &str) -> bool {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path).is_ok()
}

/// Launchers présents sur ce PC, pour pré-cocher le choix du premier lancement.
pub fn detected() -> Vec<&'static str> {
    let mut found = Vec::new();
    #[cfg(windows)]
    {
        if steam_root().is_some() {
            found.push("steam");
        }
        if registry_key_exists(r"SOFTWARE\WOW6432Node\GOG.com\Games") {
            found.push("gog");
        }
        if registry_key_exists(r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs") {
            found.push("ubisoft");
        }
    }
    if epic_manifests().is_some_and(|p| p.is_dir()) {
        found.push("epic");
    }
    found.extend(crate::launchers::detected());
    found
}

/// Cherche les jeux, uniquement chez les launchers que `enabled` autorise.
pub fn scan_all(enabled: impl Fn(&str) -> bool) -> Vec<Found> {
    let mut found = Vec::new();
    #[cfg(windows)]
    if enabled("steam") {
        if let Some(root) = steam_root() {
            found.extend(scan_steam(&root));
        }
    }
    if enabled("epic") {
        if let Some(manifests) = epic_manifests() {
            found.extend(scan_epic(&manifests));
        }
    }
    #[cfg(windows)]
    if enabled("gog") {
        found.extend(scan_gog());
    }
    #[cfg(windows)]
    if enabled("ubisoft") {
        found.extend(scan_ubisoft());
    }
    if enabled("xbox") {
        found.extend(crate::launchers::scan_xbox());
    }
    if enabled("ea") {
        found.extend(crate::launchers::scan_ea());
    }
    if enabled("battlenet") {
        found.extend(crate::launchers::scan_battlenet());
    }
    found
}

// -------------------------------------------------------------- Ubisoft

/// Un jeu Ubisoft Connect : le nom est celui de son dossier d'installation.
fn ubisoft_game(id: &str, install_dir: &str) -> Option<Found> {
    let normalized = install_dir.replace('/', "\\");
    let name = normalized.trim_end_matches('\\').rsplit('\\').next()?.trim();
    if id.is_empty() || name.is_empty() {
        return None;
    }
    Some(Found {
        source: "ubisoft",
        key: id.to_string(),
        name: name.to_string(),
        target: format!("uplay://launch/{id}/0"),
        install_dir: Some(normalized.trim_end_matches('\\').to_string()),
    })
}

/// Ubisoft Connect inscrit chaque jeu installé dans le registre.
#[cfg(windows)]
fn scan_ubisoft() -> Vec<Found> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let Ok(installs) =
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs")
    else {
        return Vec::new();
    };
    installs
        .enum_keys()
        .flatten()
        .filter_map(|id| {
            let dir: String = installs.open_subkey(&id).ok()?.get_value("InstallDir").ok()?;
            ubisoft_game(&id, &dir)
        })
        .collect()
}

// ------------------------------------------------------------------- GOG

/// Un jeu GOG Galaxy à partir de ses valeurs du registre.
fn gog_game(id: &str, name: &str, exe: &str, dir: &str) -> Option<Found> {
    if id.is_empty() || name.is_empty() || exe.is_empty() {
        return None;
    }
    Some(Found {
        source: "gog",
        key: id.to_string(),
        name: name.to_string(),
        target: exe.to_string(),
        install_dir: (!dir.is_empty()).then(|| dir.to_string()),
    })
}

/// GOG Galaxy inscrit chaque jeu installé dans le registre.
#[cfg(windows)]
fn scan_gog() -> Vec<Found> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let Ok(games) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\WOW6432Node\GOG.com\Games")
    else {
        return Vec::new();
    };
    games
        .enum_keys()
        .flatten()
        .filter_map(|id| {
            let key = games.open_subkey(&id).ok()?;
            let value = |name: &str| key.get_value::<String, _>(name).unwrap_or_default();
            gog_game(&id, &value("gameName"), &value("exe"), &value("path"))
        })
        .collect()
}

// ------------------------------------------------------------------ Epic

/// Lit les fichiers `*.item` du dossier de manifestes d'Epic Games Store.
pub fn scan_epic(manifests: &Path) -> Vec<Found> {
    let Ok(entries) = fs::read_dir(manifests) else {
        return Vec::new();
    };
    let mut games = Vec::new();
    for entry in entries.flatten() {
        if entry.path().extension().is_none_or(|e| e != "item") {
            continue;
        }
        let Some(item) = fs::read_to_string(entry.path())
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        else {
            continue;
        };
        let text = |k: &str| item.get(k).and_then(|v| v.as_str()).unwrap_or("");
        let (name, app, exe) = (text("DisplayName"), text("AppName"), text("LaunchExecutable"));
        let incomplete = item
            .get("bIsIncompleteInstall")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        // Les extensions et outils n'ont pas d'exécutable à lancer.
        if name.is_empty() || app.is_empty() || exe.is_empty() || incomplete {
            continue;
        }
        // Lien officiel : passe par Epic, qui gère la connexion du joueur.
        let (ns, cat) = (text("CatalogNamespace"), text("CatalogItemId"));
        let target = if ns.is_empty() || cat.is_empty() {
            Path::new(text("InstallLocation")).join(exe).to_string_lossy().into_owned()
        } else {
            format!(
                "com.epicgames.launcher://apps/{ns}%3A{cat}%3A{app}?action=launch&silent=true"
            )
        };
        let location = text("InstallLocation");
        games.push(Found {
            source: "epic",
            key: app.to_string(),
            name: name.to_string(),
            target,
            install_dir: (!location.is_empty()).then(|| location.to_string()),
        });
    }
    games
}

// ---------------------------------------------------------------- Steam

/// Dossier d'installation de Steam, s'il est installé.
#[cfg(windows)]
pub fn steam_root() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam")
        .ok()?;
    key.get_value::<String, _>("SteamPath").ok().map(PathBuf::from)
}

/// Programmes techniques installés par Steam, qui ne sont pas des jeux.
pub fn steam_is_tool(appid: &str, name: &str) -> bool {
    appid == "228980"
        || name.starts_with("Steamworks Common")
        || name.starts_with("Proton")
        || name.starts_with("Steam Linux Runtime")
}

pub fn scan_steam(root: &Path) -> Vec<Found> {
    let mut libraries = vec![root.to_path_buf()];
    if let Ok(text) = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for path in vdf_values(&text, "path") {
            let p = PathBuf::from(path);
            if !libraries.contains(&p) {
                libraries.push(p);
            }
        }
    }

    let mut games = Vec::new();
    for lib in libraries {
        let Ok(entries) = fs::read_dir(lib.join("steamapps")) else {
            continue;
        };
        for entry in entries.flatten() {
            let file = entry.file_name().to_string_lossy().into_owned();
            if !(file.starts_with("appmanifest_") && file.ends_with(".acf")) {
                continue;
            }
            let Ok(text) = fs::read_to_string(entry.path()) else {
                continue;
            };
            let (Some(appid), Some(name)) = (vdf_value(&text, "appid"), vdf_value(&text, "name"))
            else {
                continue;
            };
            // StateFlags : le bit 4 signifie « entièrement installé ».
            let installed = vdf_value(&text, "StateFlags")
                .and_then(|v| v.parse::<u32>().ok())
                .is_some_and(|f| f & 4 != 0);
            if !installed || steam_is_tool(&appid, &name) {
                continue;
            }
            let install_dir = vdf_value(&text, "installdir")
                .map(|dir| lib.join("steamapps").join("common").join(dir).to_string_lossy().into_owned());
            games.push(Found {
                source: "steam",
                target: format!("steam://rungameid/{appid}"),
                key: appid,
                name,
                install_dir,
            });
        }
    }
    games
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lecture_vdf() {
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"3241660\"\n\t\"name\"\t\t\"R.E.P.O.\"\n\t\"StateFlags\"\t\t\"4\"\n}";
        assert_eq!(vdf_value(acf, "appid").as_deref(), Some("3241660"));
        assert_eq!(vdf_value(acf, "name").as_deref(), Some("R.E.P.O."));
        assert_eq!(vdf_value(acf, "absent"), None);
        let lib = "\t\t\"path\"\t\t\"D:\\\\Jeux\\\\Steam\"";
        assert_eq!(vdf_values(lib, "path"), [r"D:\Jeux\Steam"]);
    }

    #[test]
    fn epic_depuis_des_manifestes() {
        let dir = std::env::temp_dir().join("tronix-test-epic");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("A.item"),
            r#"{"DisplayName":"Jeu Epic","AppName":"Fortnite1","CatalogNamespace":"ns1","CatalogItemId":"cat1","InstallLocation":"D:\\Epic\\Jeu","LaunchExecutable":"bin\\jeu.exe","bIsIncompleteInstall":false}"#,
        )
        .unwrap();
        fs::write(
            dir.join("B.item"),
            r#"{"DisplayName":"Sans exe","AppName":"X","LaunchExecutable":""}"#,
        )
        .unwrap();
        fs::write(
            dir.join("C.item"),
            r#"{"DisplayName":"Incomplet","AppName":"Y","LaunchExecutable":"a.exe","bIsIncompleteInstall":true}"#,
        )
        .unwrap();
        fs::write(dir.join("D.item"), "pas du json").unwrap();
        let games = scan_epic(&dir);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "Jeu Epic");
        assert_eq!(
            games[0].target,
            "com.epicgames.launcher://apps/ns1%3Acat1%3AFortnite1?action=launch&silent=true"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn gog_depuis_le_registre() {
        let g = gog_game("1207658924", "Jeu GOG", r"D:\GOG\Jeu\jeu.exe", r"D:\GOG\Jeu").unwrap();
        assert_eq!((g.source, g.name.as_str()), ("gog", "Jeu GOG"));
        assert_eq!(g.target, r"D:\GOG\Jeu\jeu.exe");
        assert_eq!(g.install_dir.as_deref(), Some(r"D:\GOG\Jeu"));
        assert!(gog_game("1", "Sans exécutable", "", "").is_none());
    }

    #[test]
    fn ubisoft_depuis_le_registre() {
        let g = ubisoft_game("720", "C:/Program Files (x86)/Ubisoft/games/Far Cry 6/").unwrap();
        assert_eq!((g.source, g.name.as_str()), ("ubisoft", "Far Cry 6"));
        assert_eq!(g.target, "uplay://launch/720/0");
        assert_eq!(g.install_dir.as_deref(), Some(r"C:\Program Files (x86)\Ubisoft\games\Far Cry 6"));
        assert!(ubisoft_game("", "C:/x/").is_none());
    }

    #[test]
    fn steam_depuis_un_dossier() {
        let root = std::env::temp_dir().join("tronix-test-steam");
        let apps = root.join("steamapps");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&apps).unwrap();
        let acf = |id: &str, name: &str, flags: &str| {
            format!("\"AppState\"\n{{\n\t\"appid\"\t\t\"{id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"installdir\"\t\t\"SuperJeu\"\n\t\"StateFlags\"\t\t\"{flags}\"\n}}")
        };
        fs::write(apps.join("appmanifest_1.acf"), acf("1", "Super Jeu", "4")).unwrap();
        fs::write(apps.join("appmanifest_2.acf"), acf("2", "Pas installé", "1026")).unwrap();
        fs::write(
            apps.join("appmanifest_228980.acf"),
            acf("228980", "Steamworks Common Redistributables", "4"),
        )
        .unwrap();
        let games = scan_steam(&root);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "Super Jeu");
        assert_eq!(games[0].target, "steam://rungameid/1");
        assert!(games[0].install_dir.as_deref().unwrap().ends_with(r"steamapps\common\SuperJeu"));
        let _ = fs::remove_dir_all(&root);
    }
}

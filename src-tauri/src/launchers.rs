//! Xbox (Game Pass), EA app et Battle.net.
//!
//! Ces trois launchers ne sont pas installés sur la machine de développement : les lecteurs
//! ci-dessous sont testés sur des fichiers d'exemple, pas sur de vrais jeux. Seule la
//! construction des identifiants du Store et le lancement par `shell:AppsFolder` ont été
//! vérifiés sur de vraies applications Windows.

use crate::imports::Found;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------- Lecture XML simple

/// Début d'une balise `<tag` suivie d'un espace, de `/` ou de `>` (et non d'une balise plus longue).
fn find_tag(xml: &str, tag: &str) -> Option<usize> {
    let open = format!("<{tag}");
    xml.match_indices(&open).map(|(i, _)| i).find(|&i| {
        xml[i + open.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '/' || c == '>')
    })
}

/// Valeur de l'attribut `attr` de la première balise `<tag ...>`.
fn xml_attr(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let rest = &xml[find_tag(xml, tag)?..];
    let head = &rest[..rest.find('>')?];
    let needle = format!("{attr}=\"");
    let from = head.find(&needle)? + needle.len();
    let value = &head[from..];
    Some(value[..value.find('"')?].to_string())
}

/// Texte de la première balise `<tag ...>texte</tag>`.
fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let rest = &xml[find_tag(xml, tag)?..];
    let from = rest.find('>')? + 1;
    let body = &rest[from..];
    let text = body[..body.find(&format!("</{tag}>"))?].trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Fichier texte lu même avec un BOM ou un encodage imparfait.
fn read_text(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}').to_string())
}

// ------------------------------------------------------------------------- Xbox

/// Un jeu Xbox / Game Pass installé dans un dossier `XboxGames`.
///
/// - `folder_name` : nom du dossier du jeu ;
/// - `config` : contenu de `MicrosoftGame.Config` ;
/// - `packages` : noms des paquets installés (`Nom_1.0.0.0_x64__IdEditeur`), qui donnent l'identifiant de l'éditeur.
pub fn xbox_game(folder_name: &str, config: &str, packages: &[String], install_dir: &Path) -> Option<Found> {
    let identity = xml_attr(config, "Identity", "Name")?;
    // Famille du paquet = Nom + « _ » + identifiant de l'éditeur (dernier segment du nom complet).
    let prefix = format!("{}_", identity.to_lowercase());
    let publisher_id = packages
        .iter()
        .find(|p| p.to_lowercase().starts_with(&prefix))
        .and_then(|p| p.rsplit('_').next())
        .filter(|id| !id.is_empty())?;
    let app_id = xml_attr(config, "Executable", "Id").unwrap_or_else(|| "Game".to_string());
    let name = xml_attr(config, "ShellVisuals", "DefaultDisplayName")
        .filter(|n| !n.is_empty() && !n.starts_with("ms-resource"))
        .unwrap_or_else(|| folder_name.to_string());
    Some(Found {
        source: "xbox",
        key: identity.clone(),
        name,
        target: format!(r"shell:AppsFolder\{identity}_{publisher_id}!{app_id}"),
        install_dir: Some(install_dir.to_string_lossy().into_owned()),
    })
}

/// Jeux installés dans `<lecteur>:\XboxGames\<jeu>` sur tous les lecteurs.
pub fn scan_xbox_in(roots: &[PathBuf], packages: &[String]) -> Vec<Found> {
    let mut games = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let folder = entry.path();
            let content = folder.join("Content");
            let Some(config) = read_text(&content.join("MicrosoftGame.Config")) else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(game) = xbox_game(&name, &config, packages, &content) {
                games.push(game);
            }
        }
    }
    games
}

fn xbox_roots() -> Vec<PathBuf> {
    ('A'..='Z')
        .map(|letter| PathBuf::from(format!(r"{letter}:\XboxGames")))
        .filter(|p| p.is_dir())
        .collect()
}

// -------------------------------------------------------------------------- EA app

/// Un jeu de l'EA app à partir de son dossier d'installation et de `__Installer\installerdata.xml`.
///
/// On lance l'exécutable du jeu quand on le trouve, sinon le lien `link2ea` de son identifiant.
pub fn ea_game(key_name: &str, install_dir: &Path, installer_data: Option<&str>) -> Option<Found> {
    let data = installer_data.unwrap_or("");
    let name = xml_text(data, "gameTitle").unwrap_or_else(|| key_name.to_string());
    let offer_id = xml_text(data, "contentID");
    let launcher = data
        .find("<launcher")
        .and_then(|i| xml_text(&data[i..], "filePath"))
        .map(|raw| {
            // « [HKEY_LOCAL_MACHINE\...\Install Dir]jeu.exe » : la clé désigne le dossier d'installation.
            let file = raw.rsplit(']').next().unwrap_or(&raw).trim_start_matches(['\\', '/']);
            if Path::new(file).is_absolute() {
                PathBuf::from(file)
            } else {
                install_dir.join(file)
            }
        })
        .filter(|exe| exe.is_file());
    let target = match (&launcher, &offer_id) {
        (Some(exe), _) => exe.to_string_lossy().into_owned(),
        (None, Some(id)) => format!("link2ea://launchgame/{id}"),
        (None, None) => return None,
    };
    Some(Found {
        source: "ea",
        key: offer_id.unwrap_or_else(|| key_name.to_string()),
        name,
        target,
        install_dir: Some(install_dir.to_string_lossy().into_owned()),
    })
}

// ----------------------------------------------------------------------- Battle.net

/// Jeux Blizzard et leur code pour `battlenet://`. Les noms sont comparés sans accents ni symboles.
const BLIZZARD_CODES: [(&str, &str); 10] = [
    ("worldofwarcraft", "WoW"),
    ("diabloiv", "Fen"),
    ("diabloiiresurrected", "OSI"),
    ("diabloiii", "D3"),
    ("starcraftii", "S2"),
    ("starcraftremastered", "S1"),
    ("hearthstone", "WTCG"),
    ("overwatch", "Pro"),
    ("heroesofthestorm", "Hero"),
    ("warcraftiii", "W3"),
];

fn normalize(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
}

/// Un jeu Blizzard d'après son entrée de désinstallation Windows.
pub fn battlenet_game(display_name: &str, publisher: &str, install_location: &str, display_icon: &str) -> Option<Found> {
    if !publisher.to_lowercase().contains("blizzard") || display_name.is_empty() {
        return None;
    }
    let plain = normalize(display_name);
    if plain.starts_with("battlenet") {
        return None; // l'application Battle.net elle-même
    }
    let code = BLIZZARD_CODES.iter().find(|(known, _)| plain.contains(known)).map(|(_, code)| *code);
    // Sans code connu : l'exécutable donné par l'icône d'affichage, s'il existe.
    let icon = display_icon.trim().trim_matches('"');
    let exe = icon.rsplit_once(',').map_or(icon, |(path, _)| path).trim_matches('"');
    let exe = (exe.to_lowercase().ends_with(".exe") && Path::new(exe).is_file()).then(|| exe.to_string());
    let target = match (code, exe) {
        (Some(code), _) => format!("battlenet://{code}"),
        (None, Some(exe)) => exe,
        (None, None) => return None,
    };
    Some(Found {
        source: "battlenet",
        key: code.map_or_else(|| plain.clone(), str::to_string),
        name: display_name.to_string(),
        target,
        install_dir: (!install_location.is_empty()).then(|| install_location.trim_end_matches('\\').to_string()),
    })
}

// ------------------------------------------------------------------ Registre Windows

#[cfg(windows)]
mod windows {
    use super::*;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    fn value(key: &RegKey, name: &str) -> String {
        key.get_value::<String, _>(name).unwrap_or_default()
    }

    /// Noms complets des paquets installés pour l'utilisateur (applications du Store, jeux Xbox).
    fn package_names() -> Vec<String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages")
            .map(|k| k.enum_keys().flatten().collect())
            .unwrap_or_default()
    }

    pub fn xbox() -> Vec<Found> {
        let roots = xbox_roots();
        if roots.is_empty() {
            return Vec::new();
        }
        scan_xbox_in(&roots, &package_names())
    }

    pub fn ea() -> Vec<Found> {
        let mut games = Vec::new();
        for root in [r"SOFTWARE\WOW6432Node\EA Games", r"SOFTWARE\EA Games"] {
            let Ok(parent) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(root) else {
                continue;
            };
            for name in parent.enum_keys().flatten() {
                let Ok(key) = parent.open_subkey(&name) else {
                    continue;
                };
                let dir = value(&key, "Install Dir");
                if dir.is_empty() {
                    continue;
                }
                let dir = PathBuf::from(dir);
                let data = read_text(&dir.join("__Installer").join("installerdata.xml"));
                if let Some(game) = ea_game(&name, &dir, data.as_deref()) {
                    games.push(game);
                }
            }
        }
        games
    }

    pub fn battlenet() -> Vec<Found> {
        let mut games = Vec::new();
        for root in [
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        ] {
            let Ok(parent) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(root) else {
                continue;
            };
            for name in parent.enum_keys().flatten() {
                let Ok(key) = parent.open_subkey(&name) else {
                    continue;
                };
                if let Some(game) = battlenet_game(
                    &value(&key, "DisplayName"),
                    &value(&key, "Publisher"),
                    &value(&key, "InstallLocation"),
                    &value(&key, "DisplayIcon"),
                ) {
                    games.push(game);
                }
            }
        }
        games
    }

    pub fn has_key(root: &str) -> bool {
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(root).is_ok()
    }
}

#[cfg(windows)]
pub use windows::{battlenet as scan_battlenet, ea as scan_ea, xbox as scan_xbox};

#[cfg(not(windows))]
pub fn scan_xbox() -> Vec<Found> {
    Vec::new()
}
#[cfg(not(windows))]
pub fn scan_ea() -> Vec<Found> {
    Vec::new()
}
#[cfg(not(windows))]
pub fn scan_battlenet() -> Vec<Found> {
    Vec::new()
}

/// Launchers de ce groupe présents sur ce PC.
pub fn detected() -> Vec<&'static str> {
    let mut found = Vec::new();
    if !xbox_roots().is_empty() {
        found.push("xbox");
    }
    #[cfg(windows)]
    {
        if windows::has_key(r"SOFTWARE\WOW6432Node\EA Games") || windows::has_key(r"SOFTWARE\EA Games") {
            found.push("ea");
        }
        if windows::has_key(r"SOFTWARE\WOW6432Node\Blizzard Entertainment") || Path::new(r"C:\ProgramData\Battle.net").is_dir() {
            found.push("battlenet");
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const XBOX_CONFIG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="1">
  <Identity Name="Microsoft.624F8B84B80" Publisher="CN=Microsoft Corporation" Version="1.0.0.0" />
  <ExecutableList>
    <Executable Name="ForzaHorizon5.exe" Id="Game" TargetDeviceFamily="PC" />
  </ExecutableList>
  <ShellVisuals DefaultDisplayName="Forza Horizon 5" PublisherDisplayName="Xbox Game Studios" />
</Game>"#;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lecture_xml() {
        assert_eq!(xml_attr(XBOX_CONFIG, "Identity", "Name").as_deref(), Some("Microsoft.624F8B84B80"));
        assert_eq!(xml_attr(XBOX_CONFIG, "Executable", "Id").as_deref(), Some("Game"));
        // `<Executable` ne doit pas être confondu avec `<ExecutableList`.
        assert_eq!(xml_attr(XBOX_CONFIG, "Executable", "Name").as_deref(), Some("ForzaHorizon5.exe"));
        assert_eq!(xml_attr(XBOX_CONFIG, "Absente", "Name"), None);
        assert_eq!(xml_text("<a><gameTitle locale=\"fr\"> Jeu </gameTitle></a>", "gameTitle").as_deref(), Some("Jeu"));
        assert_eq!(xml_text("<a><gameTitle></gameTitle></a>", "gameTitle"), None);
    }

    #[test]
    fn jeu_xbox() {
        // Noms de paquets au format relevé dans le registre de ce PC.
        let packages = vec![
            "AppUp.IntelGraphicsExperience_1.100.5688.0_x64__8j3eq9eme6ctt".to_string(),
            "Microsoft.624F8B84B80_1.2.3.4_x64__8wekyb3d8bbwe".to_string(),
        ];
        let game = xbox_game("Forza Horizon 5", XBOX_CONFIG, &packages, Path::new(r"D:\XboxGames\Forza\Content")).unwrap();
        assert_eq!(game.source, "xbox");
        assert_eq!(game.name, "Forza Horizon 5");
        assert_eq!(game.target, r"shell:AppsFolder\Microsoft.624F8B84B80_8wekyb3d8bbwe!Game");
        assert_eq!(game.key, "Microsoft.624F8B84B80");
        // Paquet absent du registre : le jeu n'est pas installé pour cet utilisateur.
        assert!(xbox_game("X", XBOX_CONFIG, &packages[..1], Path::new("D:\\")).is_none());
        // Nom affiché sous forme de ressource : on prend le nom du dossier.
        let resource = XBOX_CONFIG.replace("Forza Horizon 5", "ms-resource:AppName");
        assert_eq!(xbox_game("Mon dossier", &resource, &packages, Path::new("D:\\")).unwrap().name, "Mon dossier");
    }

    #[test]
    fn dossier_xbox_games() {
        let root = temp("tronix-test-xboxgames");
        let content = root.join("Forza Horizon 5").join("Content");
        fs::create_dir_all(&content).unwrap();
        fs::write(content.join("MicrosoftGame.Config"), format!("\u{feff}{XBOX_CONFIG}")).unwrap();
        fs::create_dir_all(root.join("Dossier vide")).unwrap();
        let packages = vec!["Microsoft.624F8B84B80_1.0.0.0_x64__8wekyb3d8bbwe".to_string()];
        let games = scan_xbox_in(&[root.clone(), PathBuf::from(r"Z:\absent")], &packages);
        assert_eq!(games.len(), 1);
        assert!(games[0].install_dir.as_deref().unwrap().ends_with("Content"));
        let _ = fs::remove_dir_all(root);
    }

    const EA_DATA: &str = r#"<DiPManifest version="4.0">
  <gameTitles><gameTitle locale="en_US">Battlefield 4</gameTitle></gameTitles>
  <contentIDs><contentID>1015365</contentID></contentIDs>
  <runtime><launcher><filePath>[HKEY_LOCAL_MACHINE\SOFTWARE\EA Games\Battlefield 4\Install Dir]bf4.exe</filePath></launcher></runtime>
</DiPManifest>"#;

    #[test]
    fn jeu_ea() {
        let dir = temp("tronix-test-ea");
        // Sans l'exécutable sur le disque : on retombe sur le lien de l'identifiant.
        let by_link = ea_game("Battlefield 4", &dir, Some(EA_DATA)).unwrap();
        assert_eq!(by_link.name, "Battlefield 4");
        assert_eq!(by_link.target, "link2ea://launchgame/1015365");
        assert_eq!(by_link.key, "1015365");
        // Avec l'exécutable : on le lance directement.
        fs::write(dir.join("bf4.exe"), b"MZ").unwrap();
        let direct = ea_game("Battlefield 4", &dir, Some(EA_DATA)).unwrap();
        assert!(direct.target.ends_with("bf4.exe") && Path::new(&direct.target).is_file());
        // Sans fichier de données ni exécutable : rien à lancer.
        let _ = fs::remove_file(dir.join("bf4.exe"));
        assert!(ea_game("Inconnu", &dir, None).is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn jeu_battlenet() {
        let wow = battlenet_game("World of Warcraft", "Blizzard Entertainment", r"C:\Jeux\World of Warcraft\", "").unwrap();
        assert_eq!(wow.target, "battlenet://WoW");
        assert_eq!(wow.install_dir.as_deref(), Some(r"C:\Jeux\World of Warcraft"));
        // Symboles et chiffres romains.
        assert_eq!(battlenet_game("Diablo® IV", "Blizzard Entertainment", "", "").unwrap().target, "battlenet://Fen");
        assert_eq!(battlenet_game("Diablo III", "Blizzard Entertainment", "", "").unwrap().target, "battlenet://D3");
        assert_eq!(battlenet_game("Diablo II: Resurrected", "Blizzard Entertainment", "", "").unwrap().target, "battlenet://OSI");
        // L'application Battle.net, et les logiciels d'autres éditeurs, sont ignorés.
        assert!(battlenet_game("Battle.net", "Blizzard Entertainment", "", "").is_none());
        assert!(battlenet_game("World of Warcraft", "Un autre éditeur", "", "").is_none());
        // Jeu inconnu : seul un exécutable existant permet de le lancer.
        assert!(battlenet_game("Nouveau jeu Blizzard", "Blizzard Entertainment", "", "").is_none());
        let exe = std::env::temp_dir().join("tronix-test-blizzard.exe");
        fs::write(&exe, b"MZ").unwrap();
        let icon = format!("\"{}\",0", exe.display());
        let unknown = battlenet_game("Nouveau jeu Blizzard", "Blizzard Entertainment", "", &icon).unwrap();
        assert_eq!(Path::new(&unknown.target), exe);
        let _ = fs::remove_file(exe);
    }
}

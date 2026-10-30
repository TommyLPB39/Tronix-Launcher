use crate::i18n;
use crate::secret;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, io};

/// Launchers dont Tronix sait importer les jeux : (identifiant, nom affiché).
pub const PLATFORMS: [(&str, &str); 7] = [
    ("steam", "Steam"),
    ("epic", "Epic Games Store"),
    ("gog", "GOG Galaxy"),
    ("ubisoft", "Ubisoft Connect"),
    ("xbox", "Xbox / Game Pass"),
    ("ea", "EA app"),
    ("battlenet", "Battle.net"),
];

pub const MAX_COLLECTIONS: usize = 30;
pub const MAX_COLLECTION_NAME: usize = 40;

/// Une collection créée par le joueur ; les jeux y sont rangés par `id` (le nom peut changer).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
}

/// Tris proposés pour la bibliothèque : "rank" = probabilité (classement par usage).
pub const SORTS: [&str; 5] = ["rank", "name", "recent", "playtime", "added"];

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+Space";

/// Couleurs d'accentuation proposées : (identifiant, nom affiché, couleur).
pub const ACCENTS: [(&str, &str, &str); 5] = [
    ("tronix", "Tronix", "#3DF5C4"),
    ("braise", "Braise", "#FF7A45"),
    ("ciel", "Ciel", "#5AA9FF"),
    ("citron", "Citron", "#FFC145"),
    ("rose", "Rose", "#FF5470"),
];

/// Apparence du launcher. Toute valeur inconnue (fichier modifié à la main) retombe sur la valeur par défaut.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Look {
    /// "dark", "light" ou "auto" (suit Windows).
    pub theme: String,
    /// "small", "medium" ou "large".
    pub card_size: String,
    /// "square", "round" ou "soft".
    pub radius: String,
    /// "grid" ou "list".
    pub view: String,
    pub show_hero: bool,
    /// Pochette du jeu suggéré, floutée, en fond de l'accueil.
    pub hero_background: bool,
    /// Couleur d'accent libre « #RRGGBB » ; vide : on prend la couleur choisie dans `ACCENTS`.
    pub custom_accent: String,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            theme: "dark".into(),
            card_size: "medium".into(),
            radius: "round".into(),
            view: "grid".into(),
            show_hero: true,
            hero_background: false,
            custom_accent: String::new(),
        }
    }
}

impl Look {
    /// Remplace chaque valeur inconnue par celle par défaut.
    pub fn cleaned(mut self) -> Look {
        let default = Look::default();
        let pick = |value: &mut String, allowed: &[&str], fallback: &str| {
            if !allowed.contains(&value.as_str()) {
                *value = fallback.to_string();
            }
        };
        pick(&mut self.theme, &["dark", "light", "auto"], &default.theme);
        pick(&mut self.card_size, &["small", "medium", "large"], &default.card_size);
        pick(&mut self.radius, &["square", "round", "soft"], &default.radius);
        pick(&mut self.view, &["grid", "list"], &default.view);
        let hex = self.custom_accent.trim();
        let valid = hex.len() == 7 && hex.starts_with('#') && hex[1..].bytes().all(|b| b.is_ascii_hexdigit());
        self.custom_accent = if valid { hex.to_lowercase() } else { String::new() };
        self
    }
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_sort() -> String {
    SORTS[0].to_string()
}

fn default_accent() -> String {
    ACCENTS[0].0.to_string()
}

fn default_true() -> bool {
    true
}

fn default_shortcut() -> String {
    DEFAULT_SHORTCUT.to_string()
}

#[derive(Clone, Serialize, Deserialize)]
struct Data {
    /// Faux tant que le joueur n'a pas choisi ses launchers au premier lancement.
    #[serde(default)]
    onboarded: bool,
    /// On retient les launchers désactivés : un nouveau launcher est actif par défaut.
    #[serde(default)]
    disabled: Vec<String>,
    /// Vrai : la croix réduit Tronix près de l'horloge ; faux : elle le quitte.
    #[serde(default = "default_true")]
    close_to_tray: bool,
    /// Raccourci global du lancement rapide.
    #[serde(default = "default_shortcut")]
    shortcut: String,
    /// Identifiant de la couleur d'accentuation (voir `ACCENTS`).
    #[serde(default = "default_accent")]
    accent: String,
    /// Télécharger les pochettes manquantes (désactivé tant que le joueur ne l'a pas choisi).
    #[serde(default)]
    download_covers: bool,
    /// Clé d'API SteamGridDB, facultative.
    #[serde(default)]
    steamgriddb_key: String,
    /// Afficher des informations venues d'Internet sur les pages des jeux (désactivé d'office).
    #[serde(default)]
    online_info: bool,
    /// Clé d'API RAWG, facultative : sans elle, pas de page pour les jeux hors Steam.
    #[serde(default)]
    rawg_key: String,
    /// Clé d'API Steam, facultative : sans elle, pas de liste des jeux non installés.
    #[serde(default)]
    steam_key: String,
    /// Thème, tailles, fond, couleur libre.
    #[serde(default)]
    look: Look,
    /// Tri de la bibliothèque (voir `SORTS`).
    #[serde(default = "default_sort")]
    sort: String,
    /// Collections du joueur, dans l'ordre de création.
    #[serde(default)]
    collections: Vec<Collection>,
    /// Langue choisie : "auto" (celle de Windows), "fr" ou "en".
    #[serde(default = "default_language")]
    language: String,
    /// Afficher les jeux Steam possédés mais non installés (désactivé d'office).
    #[serde(default)]
    show_uninstalled: bool,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            onboarded: false,
            disabled: Vec::new(),
            close_to_tray: true,
            shortcut: default_shortcut(),
            accent: default_accent(),
            download_covers: false,
            steamgriddb_key: String::new(),
            online_info: false,
            rawg_key: String::new(),
            steam_key: String::new(),
            show_uninstalled: false,
            look: Look::default(),
            sort: default_sort(),
            collections: Vec::new(),
            language: default_language(),
        }
    }
}

impl Data {
    /// Les clés d'API, relues en clair ; une clé chiffrée par un autre compte ou un autre PC est perdue.
    /// Renvoie vrai si une clé était encore en clair sur le disque (à réécrire chiffrée).
    fn reveal_keys(&mut self) -> bool {
        let mut legacy = false;
        for key in [&mut self.steamgriddb_key, &mut self.rawg_key, &mut self.steam_key] {
            legacy |= !key.is_empty() && !secret::is_protected(key);
            *key = secret::unprotect(key).unwrap_or_default();
        }
        legacy
    }

    /// Copie destinée au disque, clés chiffrées. Échoue plutôt que d'écrire une clé en clair.
    fn sealed(&self) -> io::Result<Data> {
        let mut out = self.clone();
        for key in [&mut out.steamgriddb_key, &mut out.rawg_key, &mut out.steam_key] {
            *key = secret::protect(key).ok_or_else(|| io::Error::other("chiffrement de la clé impossible"))?;
        }
        Ok(out)
    }
}

pub struct Settings {
    path: PathBuf,
    data: Data,
}

impl Settings {
    pub fn load(path: PathBuf) -> Self {
        let mut data: Data = match crate::store::read_text(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| {
                // Fichier illisible : mis de côté plutôt qu'écrasé à la prochaine sauvegarde (clés, collections…).
                let _ = fs::rename(&path, path.with_extension("json.corrupt"));
                Data::default()
            }),
            Err(_) => Data::default(),
        };
        let legacy = data.reveal_keys();
        data.look = data.look.clone().cleaned();
        let settings = Settings { path, data };
        // Ancien fichier avec des clés en clair : on le réécrit chiffré dès la première lecture.
        if legacy {
            let _ = settings.save();
        }
        settings
    }

    /// Écrit dans un fichier temporaire puis renomme : un arrêt en pleine écriture
    /// ne laisse pas un fichier à moitié écrit.
    fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec(&self.data.sealed()?)?)?;
        fs::rename(tmp, &self.path)
    }

    pub fn close_to_tray(&self) -> bool {
        self.data.close_to_tray
    }

    pub fn set_close_to_tray(&mut self, enabled: bool) -> io::Result<()> {
        self.data.close_to_tray = enabled;
        self.save()
    }

    pub fn shortcut(&self) -> &str {
        &self.data.shortcut
    }

    pub fn set_shortcut(&mut self, shortcut: &str) -> io::Result<()> {
        self.data.shortcut = shortcut.to_string();
        self.save()
    }

    /// Identifiant de la couleur d'accentuation ; un identifiant inconnu retombe sur la couleur par défaut.
    pub fn accent(&self) -> &str {
        ACCENTS
            .iter()
            .find(|(id, _, _)| *id == self.data.accent)
            .map_or(ACCENTS[0].0, |(id, _, _)| *id)
    }

    pub fn set_accent(&mut self, id: &str) -> Result<(), String> {
        if !ACCENTS.iter().any(|(known, _, _)| *known == id) {
            return Err(i18n::t("err.unknown_color"));
        }
        self.data.accent = id.to_string();
        // Choisir une des couleurs proposées remplace la couleur libre.
        self.data.look.custom_accent.clear();
        self.save().map_err(|e| e.to_string())
    }

    pub fn collections(&self) -> &[Collection] {
        &self.data.collections
    }

    /// Nom valable : non vide, pas trop long, différent (sans tenir compte des majuscules) de celui des autres.
    fn check_collection_name(&self, name: &str, except: Option<&str>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(i18n::t("err.collection_name_empty"));
        }
        if name.chars().count() > MAX_COLLECTION_NAME {
            return Err(i18n::tf("err.collection_name_long", &[("max", &MAX_COLLECTION_NAME.to_string())]));
        }
        let taken = self.data.collections.iter().any(|c| Some(c.id.as_str()) != except && c.name.to_lowercase() == name.to_lowercase());
        if taken {
            return Err(i18n::t("err.collection_name_taken"));
        }
        Ok(name.to_string())
    }

    pub fn add_collection(&mut self, name: &str) -> Result<Collection, String> {
        let name = self.check_collection_name(name, None)?;
        if self.data.collections.len() >= MAX_COLLECTIONS {
            return Err(i18n::tf("err.collection_limit", &[("max", &MAX_COLLECTIONS.to_string())]));
        }
        let mut stamp = crate::store::now_ms();
        let id = loop {
            let id = format!("c{stamp:x}");
            if !self.data.collections.iter().any(|c| c.id == id) {
                break id;
            }
            stamp += 1;
        };
        let collection = Collection { id, name };
        self.data.collections.push(collection.clone());
        self.save().map_err(|e| e.to_string())?;
        Ok(collection)
    }

    pub fn rename_collection(&mut self, id: &str, name: &str) -> Result<(), String> {
        let name = self.check_collection_name(name, Some(id))?;
        let collection = self.data.collections.iter_mut().find(|c| c.id == id).ok_or_else(|| i18n::t("err.unknown_collection"))?;
        collection.name = name;
        self.save().map_err(|e| e.to_string())
    }

    pub fn delete_collection(&mut self, id: &str) -> Result<(), String> {
        let before = self.data.collections.len();
        self.data.collections.retain(|c| c.id != id);
        if self.data.collections.len() == before {
            return Err(i18n::t("err.unknown_collection"));
        }
        self.save().map_err(|e| e.to_string())
    }

    /// Langue choisie ; une valeur inconnue retombe sur « auto ».
    pub fn language(&self) -> &str {
        match self.data.language.as_str() {
            "fr" => "fr",
            "en" => "en",
            _ => "auto",
        }
    }

    pub fn set_language(&mut self, language: &str) -> Result<(), String> {
        if language != "auto" && !i18n::LANGS.contains(&language) {
            return Err(i18n::t("err.unknown_language"));
        }
        self.data.language = language.to_string();
        self.save().map_err(|e| e.to_string())
    }

    /// Tri choisi ; une valeur inconnue retombe sur la probabilité.
    pub fn sort(&self) -> &str {
        SORTS.iter().find(|s| **s == self.data.sort).copied().unwrap_or(SORTS[0])
    }

    pub fn set_sort(&mut self, sort: &str) -> Result<(), String> {
        if !SORTS.contains(&sort) {
            return Err(i18n::t("err.unknown_sort"));
        }
        self.data.sort = sort.to_string();
        self.save().map_err(|e| e.to_string())
    }

    pub fn look(&self) -> &Look {
        &self.data.look
    }

    pub fn set_look(&mut self, look: Look) -> io::Result<()> {
        self.data.look = look.cleaned();
        self.save()
    }

    /// Remet l'apparence d'origine, couleur d'accentuation comprise.
    pub fn reset_look(&mut self) -> io::Result<()> {
        self.data.look = Look::default();
        self.data.accent = default_accent();
        self.save()
    }

    pub fn download_covers(&self) -> bool {
        self.data.download_covers
    }

    pub fn set_download_covers(&mut self, enabled: bool) -> io::Result<()> {
        self.data.download_covers = enabled;
        self.save()
    }

    pub fn steamgriddb_key(&self) -> &str {
        &self.data.steamgriddb_key
    }

    pub fn set_steamgriddb_key(&mut self, key: &str) -> io::Result<()> {
        self.data.steamgriddb_key = key.trim().to_string();
        self.save()
    }

    pub fn online_info(&self) -> bool {
        self.data.online_info
    }

    pub fn set_online_info(&mut self, enabled: bool) -> io::Result<()> {
        self.data.online_info = enabled;
        self.save()
    }

    pub fn rawg_key(&self) -> &str {
        &self.data.rawg_key
    }

    pub fn set_rawg_key(&mut self, key: &str) -> io::Result<()> {
        self.data.rawg_key = key.trim().to_string();
        self.save()
    }

    pub fn steam_key(&self) -> &str {
        &self.data.steam_key
    }

    pub fn set_steam_key(&mut self, key: &str) -> io::Result<()> {
        self.data.steam_key = key.trim().to_string();
        self.save()
    }

    pub fn show_uninstalled(&self) -> bool {
        self.data.show_uninstalled
    }

    pub fn set_show_uninstalled(&mut self, enabled: bool) -> io::Result<()> {
        self.data.show_uninstalled = enabled;
        self.save()
    }

    pub fn enabled_count(&self) -> usize {
        PLATFORMS.iter().filter(|(id, _)| self.is_enabled(id)).count()
    }

    pub fn onboarded(&self) -> bool {
        self.data.onboarded
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        !self.data.disabled.iter().any(|d| d == id)
    }

    /// Active exactement les launchers de `enabled` et marque le premier choix comme fait.
    pub fn set_enabled(&mut self, enabled: &[String]) -> io::Result<()> {
        self.data.disabled = PLATFORMS
            .iter()
            .filter(|(id, _)| !enabled.iter().any(|e| e == id))
            .map(|(id, _)| id.to_string())
            .collect();
        self.data.onboarded = true;
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fermeture_et_raccourci() {
        let tmp = std::env::temp_dir().join("tronix-test-settings-quick.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Settings::load(tmp.clone());
        assert!(s.close_to_tray());
        assert_eq!(s.shortcut(), DEFAULT_SHORTCUT);

        s.set_close_to_tray(false).unwrap();
        s.set_shortcut("Ctrl+Shift+J").unwrap();
        let again = Settings::load(tmp.clone());
        assert!(!again.close_to_tray());
        assert_eq!(again.shortcut(), "Ctrl+Shift+J");

        // Un ancien fichier sans ces champs garde les valeurs par défaut.
        fs::write(&tmp, r#"{"onboarded":true,"disabled":[]}"#).unwrap();
        let old = Settings::load(tmp.clone());
        assert!(old.onboarded() && old.close_to_tray());
        assert_eq!(old.shortcut(), DEFAULT_SHORTCUT);
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn apparence_et_pochettes() {
        let tmp = std::env::temp_dir().join("tronix-test-settings-look.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Settings::load(tmp.clone());
        assert_eq!(s.accent(), "tronix");
        assert!(!s.download_covers());
        assert_eq!(s.steamgriddb_key(), "");
        assert!(!s.online_info());
        assert_eq!(s.rawg_key(), "");

        s.set_online_info(true).unwrap();
        s.set_rawg_key(" cle-rawg ").unwrap();
        s.set_accent("braise").unwrap();
        assert!(s.set_accent("fuchsia").is_err());
        s.set_download_covers(true).unwrap();
        s.set_steamgriddb_key("  abc123 \n").unwrap();

        let again = Settings::load(tmp.clone());
        assert_eq!(again.accent(), "braise");
        assert!(again.download_covers());
        assert_eq!(again.steamgriddb_key(), "abc123");
        assert!(again.online_info());
        assert_eq!(again.rawg_key(), "cle-rawg");

        // Une couleur inconnue dans le fichier retombe sur la couleur par défaut.
        fs::write(&tmp, r#"{"accent":"disparue"}"#).unwrap();
        assert_eq!(Settings::load(tmp.clone()).accent(), "tronix");
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn cles_chiffrees_sur_le_disque() {
        let tmp = std::env::temp_dir().join("tronix-test-settings-secret.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Settings::load(tmp.clone());
        s.set_steam_key(" cle-steam-XYZ ").unwrap();
        s.set_rawg_key("cle-rawg-XYZ").unwrap();
        s.set_show_uninstalled(true).unwrap();
        let raw = fs::read_to_string(&tmp).unwrap();
        assert!(!raw.contains("cle-steam-XYZ") && !raw.contains("cle-rawg-XYZ"));
        assert!(raw.contains("dpapi:"));
        let again = Settings::load(tmp.clone());
        assert_eq!(again.steam_key(), "cle-steam-XYZ");
        assert_eq!(again.rawg_key(), "cle-rawg-XYZ");
        assert!(again.show_uninstalled());

        // Migration : un ancien fichier en clair est relu puis réécrit chiffré.
        fs::write(&tmp, r#"{"steamgriddb_key":"ancienne-cle","rawg_key":""}"#).unwrap();
        let migrated = Settings::load(tmp.clone());
        assert_eq!(migrated.steamgriddb_key(), "ancienne-cle");
        let raw = fs::read_to_string(&tmp).unwrap();
        assert!(!raw.contains("ancienne-cle") && raw.contains("dpapi:"));
        assert_eq!(Settings::load(tmp.clone()).steamgriddb_key(), "ancienne-cle");

        // Valeur chiffrée illisible : clé vide, pas de plantage.
        fs::write(&tmp, r#"{"steam_key":"dpapi:AAAA"}"#).unwrap();
        assert_eq!(Settings::load(tmp.clone()).steam_key(), "");
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn apparence_personnalisee() {
        let tmp = std::env::temp_dir().join("tronix-test-settings-feel.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Settings::load(tmp.clone());
        assert_eq!(s.look(), &Look::default());

        let look = Look { theme: "light".into(), card_size: "large".into(), radius: "soft".into(), view: "list".into(), show_hero: false, hero_background: true, custom_accent: "#AA33FF".into() };
        s.set_look(look).unwrap();
        let again = Settings::load(tmp.clone());
        assert_eq!(again.look().theme, "light");
        assert_eq!((again.look().card_size.as_str(), again.look().view.as_str(), again.look().show_hero), ("large", "list", false));
        assert_eq!(again.look().custom_accent, "#aa33ff"); // normalisée en minuscules

        // Valeurs invalides : retour aux valeurs par défaut, sans refus ni plantage.
        let bad = Look { theme: "rose".into(), card_size: "".into(), radius: "<b>".into(), view: "x".into(), custom_accent: "rouge".into(), ..Look::default() };
        s.set_look(bad).unwrap();
        assert_eq!(s.look(), &Look::default());
        for hex in ["#12345", "#1234567", "123456", "#12345g", " #ABCDEF "] {
            s.set_look(Look { custom_accent: hex.into(), ..Look::default() }).unwrap();
            assert_eq!(s.look().custom_accent, if hex.trim() == "#ABCDEF" { "#abcdef" } else { "" }, "{hex}");
        }

        // Choisir une couleur proposée remplace la couleur libre ; rétablir remet tout à zéro.
        s.set_look(Look { custom_accent: "#aa33ff".into(), theme: "auto".into(), ..Look::default() }).unwrap();
        s.set_accent("ciel").unwrap();
        assert_eq!((s.look().custom_accent.as_str(), s.look().theme.as_str()), ("", "auto"));
        s.reset_look().unwrap();
        assert_eq!((s.look(), s.accent()), (&Look::default(), "tronix"));

        // Collections : création, doublons, renommage, suppression, rechargement.
        let one = s.add_collection("  Coop ").unwrap();
        assert_eq!(one.name, "Coop");
        let two = s.add_collection("Travail").unwrap();
        assert_ne!(one.id, two.id);
        assert!(s.add_collection("coop").is_err() && s.add_collection("   ").is_err());
        assert!(s.add_collection(&"x".repeat(MAX_COLLECTION_NAME + 1)).is_err());
        s.rename_collection(&two.id, "Boulot").unwrap();
        s.rename_collection(&one.id, "COOP").unwrap(); // son propre nom, autre casse : permis
        assert!(s.rename_collection(&two.id, "coop").is_err());
        assert!(s.rename_collection("inconnue", "Autre").is_err());
        let again = Settings::load(tmp.clone());
        assert_eq!(again.collections().iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["COOP", "Boulot"]);
        s.delete_collection(&one.id).unwrap();
        assert!(s.delete_collection(&one.id).is_err());
        assert_eq!(s.collections().len(), 1);
        for i in 0..MAX_COLLECTIONS {
            let _ = s.add_collection(&format!("c{i}"));
        }
        assert_eq!(s.collections().len(), MAX_COLLECTIONS);
        assert!(s.add_collection("de trop").is_err());
        s.delete_collection(&two.id).unwrap();
        assert!(s.add_collection("de trop").is_ok());

        // Tri de la bibliothèque : valeur par défaut, refus d'un tri inconnu, retenu au rechargement.
        assert_eq!(s.sort(), "rank");
        s.set_sort("playtime").unwrap();
        assert!(s.set_sort("au-hasard").is_err());
        assert_eq!(Settings::load(tmp.clone()).sort(), "playtime");
        fs::write(&tmp, r#"{"sort":"inconnu"}"#).unwrap();
        assert_eq!(Settings::load(tmp.clone()).sort(), "rank");

        // Langue : « auto » d'office, valeurs refusées, retenue au rechargement.
        assert_eq!(s.language(), "auto");
        s.set_language("en").unwrap();
        assert!(s.set_language("klingon").is_err() && s.set_language("").is_err());
        assert_eq!(Settings::load(tmp.clone()).language(), "en");
        fs::write(&tmp, r#"{"language":"zz"}"#).unwrap();
        assert_eq!(Settings::load(tmp.clone()).language(), "auto");

        // Fichier ancien ou abîmé.
        fs::write(&tmp, r#"{"onboarded":true}"#).unwrap();
        assert_eq!(Settings::load(tmp.clone()).look(), &Look::default());
        fs::write(&tmp, r#"{"look":{"theme":"auto","view":"bizarre"}}"#).unwrap();
        let partial = Settings::load(tmp.clone());
        assert_eq!((partial.look().theme.as_str(), partial.look().view.as_str()), ("auto", "grid"));
        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn bom_et_fichier_illisible() {
        let dir = std::env::temp_dir().join("tronix-test-settings-bom");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        // Un éditeur qui ajoute un BOM ne fait pas perdre les réglages.
        fs::write(&file, "\u{feff}{\"onboarded\":true,\"accent\":\"ciel\",\"collections\":[{\"id\":\"c1\",\"name\":\"Coop\"}]}").unwrap();
        let s = Settings::load(file.clone());
        assert!(s.onboarded() && s.accent() == "ciel" && s.collections().len() == 1);
        assert!(!dir.join("settings.json.corrupt").exists());
        // Un fichier abîmé est gardé de côté, pas écrasé par la sauvegarde suivante.
        let broken = r#"{"onboarded":true,"steam_key":"dpapi:AAAA","collec"#;
        fs::write(&file, broken).unwrap();
        let mut s = Settings::load(file.clone());
        assert!(!s.onboarded());
        s.set_accent("rose").unwrap();
        assert_eq!(fs::read_to_string(dir.join("settings.json.corrupt")).unwrap(), broken);
        assert_eq!(Settings::load(file).accent(), "rose");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn choix_des_launchers() {
        let tmp = std::env::temp_dir().join("tronix-test-settings.json");
        let _ = fs::remove_file(&tmp);
        let mut s = Settings::load(tmp.clone());
        assert!(!s.onboarded());
        assert!(s.is_enabled("steam") && s.is_enabled("gog"));
        assert_eq!(s.enabled_count(), PLATFORMS.len());

        s.set_enabled(&["steam".into(), "epic".into()]).unwrap();
        assert_eq!(s.enabled_count(), 2);
        assert!(!tmp.with_extension("json.tmp").exists());
        assert!(s.onboarded());
        assert!(s.is_enabled("steam") && s.is_enabled("epic"));
        assert!(!s.is_enabled("gog") && !s.is_enabled("ubisoft"));

        let again = Settings::load(tmp.clone());
        assert!(again.onboarded() && !again.is_enabled("gog"));
        let _ = fs::remove_file(tmp);
    }
}

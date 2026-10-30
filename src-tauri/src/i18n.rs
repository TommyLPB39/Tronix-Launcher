//! Langues de l'application. Les textes vivent dans `src/locales/<langue>.json` (clés à plat, pluriels `_one` / `_other`,
//! variables `{{nom}}`, format i18next), lus par l'interface et par ce module : une seule source pour les traducteurs.
//!
//! Pour ajouter une langue : copier `en.json`, la traduire, puis l'ajouter à `LANGS` et à `table()` ci-dessous.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// Langues proposées, la première est celle des textes d'origine.
pub const LANGS: [&str; 2] = ["fr", "en"];

const FR: &str = include_str!("../../src/locales/fr.json");
const EN: &str = include_str!("../../src/locales/en.json");

/// Langue de l'interface en cours (indice dans `LANGS`) ; les messages d'erreur du cœur la suivent.
static CURRENT: AtomicU8 = AtomicU8::new(0);

fn parse(json: &str) -> HashMap<String, String> {
    serde_json::from_str(json).unwrap_or_default()
}

fn table(lang: &str) -> &'static HashMap<String, String> {
    static FR_TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    static EN_TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    match lang {
        "fr" => FR_TABLE.get_or_init(|| parse(FR)),
        _ => EN_TABLE.get_or_init(|| parse(EN)),
    }
}

/// Langue en cours : "fr" ou "en".
pub fn lang() -> &'static str {
    LANGS[(CURRENT.load(Ordering::Relaxed) as usize).min(LANGS.len() - 1)]
}

/// Change la langue en cours. Une langue inconnue est refusée.
pub fn set_lang(lang: &str) -> bool {
    match LANGS.iter().position(|l| *l == lang) {
        Some(index) => {
            CURRENT.store(index as u8, Ordering::Relaxed);
            true
        }
        None => false,
    }
}

/// Réglage du joueur ("auto", "fr" ou "en") → langue à utiliser. « Auto » suit la langue de Windows.
pub fn resolve(setting: &str) -> &'static str {
    match LANGS.iter().find(|l| **l == setting) {
        Some(l) => l,
        None => system_lang(),
    }
}

/// Langue d'affichage de Windows, si l'application la gère ; l'anglais sinon.
#[cfg(windows)]
pub fn system_lang() -> &'static str {
    // SAFETY : simple lecture d'un identifiant de langue, sans pointeur.
    let id = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    // Les 10 bits de poids faible donnent la langue (0x0C = français), quelle que soit la région.
    if id & 0x3ff == 0x0c { "fr" } else { "en" }
}

#[cfg(not(windows))]
pub fn system_lang() -> &'static str {
    "en"
}

/// Texte d'une clé dans une langue ; à défaut l'anglais, à défaut la clé elle-même.
pub fn t_in(lang: &str, key: &str) -> String {
    table(lang)
        .get(key)
        .or_else(|| table("en").get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// Comme `t_in`, avec des variables `{{nom}}` remplacées.
pub fn tf_in(lang: &str, key: &str, vars: &[(&str, &str)]) -> String {
    let mut text = t_in(lang, key);
    for (name, value) in vars {
        text = text.replace(&format!("{{{{{name}}}}}"), value);
    }
    text
}

pub fn t(key: &str) -> String {
    t_in(lang(), key)
}

pub fn tf(key: &str, vars: &[(&str, &str)]) -> String {
    tf_in(lang(), key, vars)
}

/// Tous les textes d'une langue, complétés par l'anglais pour les clés manquantes (envoyés à l'interface).
pub fn strings(lang: &str) -> HashMap<String, String> {
    let mut all = table("en").clone();
    all.extend(table(lang).clone());
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn placeholders(text: &str) -> BTreeSet<String> {
        text.split("{{").skip(1).filter_map(|rest| rest.split_once("}}").map(|(name, _)| name.to_string())).collect()
    }

    #[test]
    fn langues_completes_et_coherentes() {
        let (fr, en) = (table("fr"), table("en"));
        assert!(fr.len() > 200, "fr.json illisible ou trop court");
        let only_fr: Vec<_> = fr.keys().filter(|k| !en.contains_key(*k)).collect();
        let only_en: Vec<_> = en.keys().filter(|k| !fr.contains_key(*k)).collect();
        assert!(only_fr.is_empty() && only_en.is_empty(), "clés absentes — seulement fr : {only_fr:?}, seulement en : {only_en:?}");
        for (key, text) in fr {
            assert_eq!(placeholders(text), placeholders(&en[key]), "variables différentes pour {key}");
            assert!(!text.trim().is_empty() && !en[key].trim().is_empty(), "texte vide : {key}");
        }
        // Chaque pluriel a ses deux formes.
        for key in fr.keys() {
            if let Some(base) = key.strip_suffix("_one") {
                assert!(fr.contains_key(&format!("{base}_other")), "pluriel incomplet : {base}");
            }
        }
    }

    #[test]
    fn traduction_variables_et_repli() {
        assert_eq!(t_in("fr", "err.unknown_program"), "Programme inconnu.");
        assert_eq!(t_in("en", "err.unknown_program"), "Unknown program.");
        assert_eq!(tf_in("en", "err.collection_limit", &[("max", "30")]), "You can't have more than 30 collections.");
        assert_eq!(tf_in("fr", "err.title_too_long", &[("max", "80")]), "Le titre est trop long (80 caractères maximum).");
        // Clé inconnue : la clé elle-même, jamais un plantage ni un texte vide.
        assert_eq!(t_in("fr", "n.existe.pas"), "n.existe.pas");
        // Langue inconnue : anglais.
        assert_eq!(t_in("xx", "err.unknown_program"), "Unknown program.");
        let all = strings("fr");
        assert_eq!(all["common.close"], "Fermer");
        assert_eq!(strings("en")["common.close"], "Close");
    }

    #[test]
    fn reglage_et_langue_du_systeme() {
        assert_eq!(resolve("fr"), "fr");
        assert_eq!(resolve("en"), "en");
        assert!(LANGS.contains(&resolve("auto")) && LANGS.contains(&resolve("n'importe quoi")));
        assert!(LANGS.contains(&system_lang()));
    }
}

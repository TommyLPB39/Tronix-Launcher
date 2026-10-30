//! Pages des jeux et logiciels : informations en ligne et cache local.
//!
//! Sources : l'API publique de Steam (jeux Steam), RAWG (autres jeux, clé d'API) et Wikipédia (logiciels).
//! Les lecteurs (`parse_*`) ne touchent ni au réseau ni au disque : ils sont testés sur des réponses d'exemple.

use crate::covers;
use crate::i18n;
use crate::store::is_safe_id;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, io};

pub const TEXT_TTL_MS: i64 = 30 * 86_400_000;
pub const REVIEWS_TTL_MS: i64 = 7 * 86_400_000;
pub const NEWS_TTL_MS: i64 = 10 * 60_000;
const MAX_SHOTS: usize = 5;
const MAX_JSON_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct Rating {
    pub label: String,
    /// Part d'avis positifs, quand la source la donne.
    pub percent: Option<u32>,
    pub count: u64,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub struct NewsItem {
    pub title: String,
    pub url: String,
    /// Secondes depuis 1970.
    pub date: i64,
    pub summary: String,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Details {
    /// "steam", "rawg", "wikipedia", ou "none" quand aucune page n'existe.
    pub provider: String,
    pub match_id: String,
    pub match_name: String,
    /// "fr" ou "en".
    pub lang: String,
    pub summary: String,
    pub description: String,
    pub genres: Vec<String>,
    pub developers: Vec<String>,
    pub publishers: Vec<String>,
    pub release_date: String,
    pub platforms: Vec<String>,
    pub website: String,
    pub page_url: String,
    pub metacritic: Option<u32>,
    pub rating: Option<Rating>,
    pub fetched_at: i64,
    /// Langue de l'interface au moment de la recherche ("" pour une page gardée avant les langues : le français).
    pub ui_lang: String,
    pub reviews_at: i64,
    pub screenshot_urls: Vec<String>,
    pub shots_ok: bool,
    pub news: Vec<NewsItem>,
    pub news_at: i64,
}

/// Page choisie par le joueur pour un programme (`provider == "none"` : aucune page).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Choice {
    pub provider: String,
    pub id: String,
}

/// Une page possible pour un programme, proposée dans « Ce n'est pas le bon ? ».
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Candidate {
    pub provider: String,
    pub id: String,
    pub name: String,
    pub hint: String,
}

/// Ce qu'il faut savoir d'un programme pour lui trouver une page.
#[derive(Clone)]
pub struct Subject {
    pub name: String,
    pub source: String,
    pub target: String,
}

// -------------------------------------------------------------------- Texte

fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let decoded = tail.find(';').filter(|&end| end <= 10).and_then(|end| {
            let name = &tail[1..end];
            let ch = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                "eacute" => Some('é'),
                "egrave" => Some('è'),
                "ecirc" => Some('ê'),
                "euml" => Some('ë'),
                "agrave" => Some('à'),
                "acirc" => Some('â'),
                "icirc" => Some('î'),
                "iuml" => Some('ï'),
                "ocirc" => Some('ô'),
                "ugrave" => Some('ù'),
                "ucirc" => Some('û'),
                "ccedil" => Some('ç'),
                "Eacute" => Some('É'),
                "Egrave" => Some('È'),
                "Agrave" => Some('À'),
                "Ccedil" => Some('Ç'),
                "oelig" => Some('œ'),
                "ndash" => Some('–'),
                "mdash" => Some('—'),
                "hellip" => Some('…'),
                "laquo" => Some('«'),
                "raquo" => Some('»'),
                "lsquo" | "rsquo" => Some('’'),
                "ldquo" | "rdquo" => Some('”'),
                "copy" => Some('©'),
                "reg" => Some('®'),
                "trade" => Some('™'),
                "euro" => Some('€'),
                _ => name
                    .strip_prefix("#x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            };
            ch.map(|c| (c, end + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &tail[len..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Texte brut d'un extrait HTML : paragraphes et listes deviennent des lignes, le reste est retiré.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c != '<' {
            out.push(c);
            continue;
        }
        let Some(end) = html[i..].find('>') else {
            out.push(c);
            continue;
        };
        let tag = html[i + 1..i + end].trim_start_matches('/').to_lowercase();
        let name: String = tag.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        match name.as_str() {
            "br" | "p" | "div" | "h1" | "h2" | "h3" | "h4" | "ul" | "ol" => out.push('\n'),
            "li" if !html[i + 1..].starts_with('/') => out.push_str("\n• "),
            "li" => {} // la fin d'un élément ne saute pas de ligne : le suivant commence déjà la sienne
            _ => {}
        }
        while chars.peek().is_some_and(|&(j, _)| j < i + end + 1) {
            chars.next();
        }
    }
    tidy(&decode_entities(&out))
}

/// Retire les balises `[b]`, `[url=…]` des articles Steam.
fn strip_bbcode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Lignes nettoyées, jamais plus d'une ligne vide d'affilée.
fn tidy(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() && out.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        out.push(line);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn names(v: &Value, key: &str) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.get(key).and_then(Value::as_str).map(str::to_string)).collect())
        .unwrap_or_default()
}

// -------------------------------------------------------------------- Steam

pub fn parse_steam_details(v: &Value, appid: &str) -> Option<Details> {
    let entry = v.get(appid)?;
    if !entry.get("success")?.as_bool()? {
        return None;
    }
    let d = entry.get("data")?;
    let text = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("");
    // La liste des langues est localisée : « Français » ou « French » selon la requête.
    // Les textes sont demandés dans la langue de l'interface ; si le jeu ne la gère pas, Steam répond en anglais.
    let languages = html_to_text(text("supported_languages")).to_lowercase();
    let ui = i18n::lang();
    let supported = ui != "fr" || languages.contains("français") || languages.contains("french");
    let lang = if supported { ui } else { "en" };
    let long = if text("about_the_game").is_empty() { text("detailed_description") } else { text("about_the_game") };
    let platforms = d.get("platforms");
    let platform = |key: &str, label: &str| {
        platforms
            .and_then(|p| p.get(key))
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .then(|| label.to_string())
    };
    Some(Details {
        provider: "steam".into(),
        match_id: appid.into(),
        match_name: text("name").into(),
        lang: lang.into(),
        summary: html_to_text(text("short_description")),
        description: html_to_text(long),
        genres: names(d.get("genres").unwrap_or(&Value::Null), "description"),
        developers: strings(d.get("developers").unwrap_or(&Value::Null)),
        publishers: strings(d.get("publishers").unwrap_or(&Value::Null)),
        release_date: d.get("release_date").and_then(|r| r.get("date")).and_then(Value::as_str).unwrap_or("").into(),
        platforms: [platform("windows", "Windows"), platform("mac", "macOS"), platform("linux", "Linux")]
            .into_iter()
            .flatten()
            .collect(),
        website: text("website").into(),
        page_url: format!("https://store.steampowered.com/app/{appid}"),
        metacritic: d.get("metacritic").and_then(|m| m.get("score")).and_then(Value::as_u64).map(|s| s as u32),
        rating: d
            .get("recommendations")
            .and_then(|r| r.get("total"))
            .and_then(Value::as_u64)
            .map(|count| Rating { label: i18n::t("rating.steam_recs"), percent: None, count }),
        screenshot_urls: d
            .get("screenshots")
            .and_then(Value::as_array)
            .map(|shots| {
                shots
                    .iter()
                    .filter_map(|s| s.get("path_full").and_then(Value::as_str))
                    .map(|u| u.split('?').next().unwrap_or(u).to_string())
                    .take(MAX_SHOTS)
                    .collect()
            })
            .unwrap_or_default(),
        ..Default::default()
    })
}

/// Libellé d'une note Steam (`review_score_desc`, en anglais) dans la langue de l'interface.
fn steam_label(desc: &str) -> String {
    let key = match desc {
        "Overwhelmingly Positive" => "rating.extremely_positive",
        "Very Positive" => "rating.very_positive",
        "Positive" => "rating.positive",
        "Mostly Positive" => "rating.mostly_positive",
        "Mixed" => "rating.mixed",
        "Mostly Negative" => "rating.mostly_negative",
        "Negative" => "rating.negative",
        "Very Negative" => "rating.very_negative",
        "Overwhelmingly Negative" => "rating.extremely_negative",
        "" => "rating.default_label",
        other => return other.to_string(),
    };
    i18n::t(key)
}

pub fn parse_steam_reviews(v: &Value) -> Option<Rating> {
    let summary = v.get("query_summary")?;
    let total = summary.get("total_reviews")?.as_u64()?;
    if total == 0 {
        return None;
    }
    let positive = summary.get("total_positive")?.as_u64()?;
    Some(Rating {
        label: steam_label(summary.get("review_score_desc").and_then(Value::as_str).unwrap_or("")),
        percent: Some((positive * 100 / total) as u32),
        count: total,
    })
}

pub fn parse_steam_news(v: &Value) -> Vec<NewsItem> {
    v.get("appnews")
        .and_then(|a| a.get("newsitems"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|i| {
                    let text = |k: &str| i.get(k).and_then(Value::as_str).unwrap_or("");
                    let url = text("url");
                    (!text("title").is_empty() && url.starts_with("https://")).then(|| NewsItem {
                        title: text("title").into(),
                        url: url.into(),
                        date: i.get("date").and_then(Value::as_i64).unwrap_or(0),
                        summary: truncate(&tidy(&strip_bbcode(&html_to_text(text("contents")))), 260),
                        label: text("feedlabel").into(),
                    })
                })
                .take(5)
                .collect()
        })
        .unwrap_or_default()
}

// --------------------------------------------------------------------- RAWG

pub fn parse_rawg_search(v: &Value) -> Vec<Candidate> {
    v.get("results")
        .and_then(Value::as_array)
        .map(|results| {
            results
                .iter()
                .filter_map(|r| {
                    let id = r.get("id")?.as_i64()?;
                    let name = r.get("name")?.as_str()?;
                    let year = r.get("released").and_then(Value::as_str).and_then(|d| d.get(..4)).unwrap_or("");
                    Some(Candidate { provider: "rawg".into(), id: id.to_string(), name: name.into(), hint: year.into() })
                })
                .take(5)
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_rawg_details(v: &Value) -> Option<Details> {
    let text = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("");
    let id = v.get("id")?.as_i64()?;
    let platforms: Vec<String> = v
        .get("platforms")
        .and_then(Value::as_array)
        .map(|p| p.iter().filter_map(|x| x.get("platform")?.get("name")?.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let rating = v.get("rating").and_then(Value::as_f64).filter(|r| *r > 0.0).map(|r| Rating {
        label: i18n::t("rating.rawg"),
        percent: Some((r / 5.0 * 100.0).round() as u32),
        count: v.get("ratings_count").and_then(Value::as_u64).unwrap_or(0),
    });
    Some(Details {
        provider: "rawg".into(),
        match_id: id.to_string(),
        match_name: text("name").into(),
        lang: "en".into(),
        summary: truncate(text("description_raw").lines().next().unwrap_or(""), 280),
        description: tidy(text("description_raw")),
        genres: names(v.get("genres").unwrap_or(&Value::Null), "name"),
        developers: names(v.get("developers").unwrap_or(&Value::Null), "name"),
        publishers: names(v.get("publishers").unwrap_or(&Value::Null), "name"),
        release_date: text("released").into(),
        platforms,
        website: text("website").into(),
        page_url: v.get("slug").and_then(Value::as_str).map(|s| format!("https://rawg.io/games/{s}")).unwrap_or_default(),
        metacritic: v.get("metacritic").and_then(Value::as_u64).map(|s| s as u32),
        rating,
        ..Default::default()
    })
}

pub fn parse_rawg_shots(v: &Value) -> Vec<String> {
    v.get("results")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(|s| s.get("image").and_then(Value::as_str)).map(str::to_string).take(MAX_SHOTS).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------- Wikipédia

pub fn parse_wiki_search(v: &Value, lang: &str) -> Vec<Candidate> {
    v.get("query")
        .and_then(|q| q.get("search"))
        .and_then(Value::as_array)
        .map(|results| {
            results
                .iter()
                .filter_map(|r| {
                    let title = r.get("title")?.as_str()?;
                    let hint = html_to_text(r.get("snippet").and_then(Value::as_str).unwrap_or(""));
                    Some(Candidate {
                        provider: "wikipedia".into(),
                        id: format!("{lang}:{title}"),
                        name: title.into(),
                        hint: truncate(&hint, 90),
                    })
                })
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_wiki_summary(v: &Value, lang: &str) -> Option<Details> {
    if v.get("type").and_then(Value::as_str) == Some("disambiguation") {
        return None;
    }
    let extract = v.get("extract")?.as_str()?.trim();
    let title = v.get("title")?.as_str()?;
    (!extract.is_empty()).then(|| Details {
        provider: "wikipedia".into(),
        match_id: format!("{lang}:{title}"),
        match_name: title.into(),
        lang: lang.into(),
        summary: v.get("description").and_then(Value::as_str).unwrap_or("").into(),
        description: extract.into(),
        page_url: v
            .get("content_urls")
            .and_then(|c| c.get("desktop"))
            .and_then(|d| d.get("page"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .into(),
        ..Default::default()
    })
}

// ------------------------------------------------------------------- Réseau

/// Seul https est ouvert ; `redirects` : nombre de redirections suivies (0 pour une adresse qui contient une clé).
fn agent(redirects: u32) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .https_only(true)
        .redirects(redirects)
        .user_agent("TronixLauncher/0.1 (launcher de jeux)")
        .build()
}

fn fetch_json(redirects: u32, url: &str) -> Result<Value, String> {
    // Les erreurs ne reprennent jamais l'adresse : elle peut contenir une clé d'API.
    let response = agent(redirects).get(url).set("Accept", "application/json").call().map_err(|e| covers::net_error(&e))?;
    let mut bytes = Vec::new();
    response.into_reader().take(MAX_JSON_BYTES).read_to_end(&mut bytes).map_err(|_| i18n::t("err.service_unreadable"))?;
    // Une page HTML (redirection, blocage) n'est pas une réponse valable.
    serde_json::from_slice(&bytes).map_err(|_| i18n::t("err.service_unexpected"))
}

fn get_json(url: &str) -> Result<Value, String> {
    fetch_json(3, url)
}

/// Pour les services qui reçoivent une clé dans l'adresse : aucune redirection suivie.
fn get_json_keyed(url: &str) -> Result<Value, String> {
    fetch_json(0, url)
}

/// Identifiant d'application Steam : des chiffres seulement.
fn appid_ok(appid: &str) -> bool {
    !appid.is_empty() && appid.len() <= 10 && appid.bytes().all(|b| b.is_ascii_digit())
}

fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn steam_fetch(appid: &str) -> Result<Details, String> {
    if !appid_ok(appid) {
        return Err(i18n::t("err.steam_bad_id"));
    }
    let steam_lang = if i18n::lang() == "fr" { "french" } else { "english" };
    let url = format!("https://store.steampowered.com/api/appdetails?appids={appid}&l={steam_lang}");
    parse_steam_details(&get_json(&url)?, appid).ok_or_else(|| i18n::t("err.steam_not_found"))
}

/// Note des joueurs : l'adresse peut ne pas répondre en JSON selon la région, sans que ce soit grave.
pub fn steam_reviews(appid: &str) -> Option<Rating> {
    if !appid_ok(appid) {
        return None;
    }
    let url = format!("https://store.steampowered.com/appreviews/{appid}?json=1&language=all&purchase_type=all&num_per_page=0");
    parse_steam_reviews(&get_json(&url).ok()?)
}

pub fn steam_news(appid: &str) -> Vec<NewsItem> {
    if !appid_ok(appid) {
        return Vec::new();
    }
    let url = format!("https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/?appid={appid}&count=5&maxlength=400&format=json");
    get_json(&url).map(|v| parse_steam_news(&v)).unwrap_or_default()
}

pub fn rawg_search(name: &str, key: &str) -> Result<Vec<Candidate>, String> {
    let url = format!("https://api.rawg.io/api/games?key={}&search={}&page_size=5", percent_encode(key), percent_encode(name));
    Ok(parse_rawg_search(&get_json_keyed(&url)?))
}

pub fn rawg_fetch(id: &str, key: &str) -> Result<Details, String> {
    let url = format!("https://api.rawg.io/api/games/{}?key={}", percent_encode(id), percent_encode(key));
    parse_rawg_details(&get_json_keyed(&url)?).ok_or_else(|| i18n::t("err.rawg_incomplete"))
}

pub fn rawg_shots(id: &str, key: &str) -> Vec<String> {
    let url = format!("https://api.rawg.io/api/games/{}/screenshots?key={}", percent_encode(id), percent_encode(key));
    get_json_keyed(&url).map(|v| parse_rawg_shots(&v)).unwrap_or_default()
}

/// Résultats Wikipédia en français, puis en anglais s'il n'y en a aucun.
pub fn wiki_search(name: &str) -> Result<Vec<Candidate>, String> {
    // La langue de l'interface d'abord, l'autre ensuite s'il n'y a aucun résultat.
    let order = if i18n::lang() == "fr" { ["fr", "en"] } else { ["en", "fr"] };
    for lang in order {
        let url = format!(
            "https://{lang}.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&srlimit=4&format=json",
            percent_encode(name)
        );
        let found = parse_wiki_search(&get_json(&url)?, lang);
        if !found.is_empty() {
            return Ok(found);
        }
    }
    Ok(Vec::new())
}

/// `id` : « fr:Blender » ou « en:Blender ».
pub fn wiki_fetch(id: &str) -> Result<Option<Details>, String> {
    let (lang, title) = id.split_once(':').unwrap_or(("fr", id));
    // La langue entre dans le nom du serveur : seules fr et en sont acceptées.
    if !matches!(lang, "fr" | "en") {
        return Err(i18n::t("err.wiki_language"));
    }
    let url = format!("https://{lang}.wikipedia.org/api/rest_v1/page/summary/{}", percent_encode(&title.replace(' ', "_")));
    Ok(parse_wiki_summary(&get_json(&url)?, lang))
}

// -------------------------------------------------------------------- Cache

impl Details {
    /// Page encore valable : récente, et cherchée dans la langue de l'interface actuelle.
    pub fn is_fresh(&self, now: i64) -> bool {
        let lang = if self.ui_lang.is_empty() { "fr" } else { self.ui_lang.as_str() };
        now - self.fetched_at <= TEXT_TTL_MS && lang == i18n::lang()
    }
}

pub fn dir_for(library_file: &Path) -> PathBuf {
    library_file.parent().map_or_else(|| PathBuf::from("details"), |p| p.join("details"))
}

/// Chemin d'un fichier de cache ; un identifiant douteux donne un chemin dans un dossier qui n'existe pas.
fn json_path(dir: &Path, id: &str, suffix: &str) -> PathBuf {
    if !is_safe_id(id) {
        return dir.join("invalide").join("invalide.json");
    }
    dir.join(format!("{id}{suffix}.json"))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)
}

pub fn load(dir: &Path, id: &str) -> Option<Details> {
    serde_json::from_slice(&fs::read(json_path(dir, id, "")).ok()?).ok()
}

pub fn save(dir: &Path, id: &str, details: &Details) -> io::Result<()> {
    if !is_safe_id(id) {
        return Err(io::Error::other("identifiant invalide"));
    }
    write_atomic(&json_path(dir, id, ""), &serde_json::to_vec(details)?)
}

pub fn load_choice(dir: &Path, id: &str) -> Option<Choice> {
    serde_json::from_slice(&fs::read(json_path(dir, id, ".choice")).ok()?).ok()
}

/// `None` oublie le choix du joueur.
pub fn save_choice(dir: &Path, id: &str, choice: Option<&Choice>) -> io::Result<()> {
    if !is_safe_id(id) {
        return Err(io::Error::other("identifiant invalide"));
    }
    let path = json_path(dir, id, ".choice");
    match choice {
        Some(c) => write_atomic(&path, &serde_json::to_vec(c)?),
        None => fs::remove_file(path).or_else(|e| if e.kind() == io::ErrorKind::NotFound { Ok(()) } else { Err(e) }),
    }
}

/// Oublie la page et ses captures (quand le joueur change de correspondance).
pub fn forget(dir: &Path, id: &str) {
    if !is_safe_id(id) {
        return;
    }
    let _ = fs::remove_file(json_path(dir, id, ""));
    let _ = fs::remove_dir_all(dir.join(id));
}

/// Captures déjà téléchargées : (chemin, version).
pub fn local_shots(dir: &Path, id: &str) -> Vec<(String, u64)> {
    if !is_safe_id(id) {
        return Vec::new();
    }
    let mut shots: Vec<(String, u64)> = fs::read_dir(dir.join(id))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("shot"))
                .map(|e| {
                    let version = e
                        .metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs());
                    (e.path().to_string_lossy().into_owned(), version)
                })
                .collect()
        })
        .unwrap_or_default();
    shots.sort();
    shots
}

/// Télécharge les captures ; renvoie le nombre d'images enregistrées.
pub fn download_shots(dir: &Path, id: &str, urls: &[String]) -> usize {
    if !is_safe_id(id) {
        return 0;
    }
    let folder = dir.join(id);
    let _ = fs::remove_dir_all(&folder);
    let mut saved = 0;
    for (index, url) in urls.iter().take(MAX_SHOTS).enumerate() {
        let Ok(bytes) = covers::download_image(url, None) else {
            continue;
        };
        let Some(ext) = covers::image_ext(&bytes) else {
            continue;
        };
        if fs::create_dir_all(&folder).is_ok() && fs::write(folder.join(format!("shot{}.{ext}", index + 1)), &bytes).is_ok() {
            saved += 1;
        }
    }
    saved
}

// ------------------------------------------------------------ Orchestration

/// Fournisseur et identifiant de la page d'un programme, sans appel réseau quand c'est possible.
pub fn known_provider(subject: &Subject, choice: Option<&Choice>) -> Option<(String, String)> {
    if let Some(c) = choice {
        return (c.provider != "none").then(|| (c.provider.clone(), c.id.clone()));
    }
    subject
        .target
        .strip_prefix("steam://rungameid/")
        .map(|appid| ("steam".to_string(), appid.to_string()))
}

/// Cherche la page d'un programme : texte, fiche et notes. `Ok(None)` : aucune page n'existe.
pub fn fetch_text(subject: &Subject, choice: Option<&Choice>, rawg_key: &str, now_ms: i64) -> Result<Option<Details>, String> {
    let (provider, id) = match known_provider(subject, choice) {
        Some(found) => found,
        None if choice.is_some() => return Ok(None), // « aucune page » choisi par le joueur
        None if subject.source == "manual" => match wiki_search(&subject.name)?.into_iter().next() {
            Some(first) => (first.provider, first.id),
            None => return Ok(None),
        },
        None if rawg_key.is_empty() => return Err(i18n::t("err.rawg_key_nonsteam")),
        None => match rawg_search(&subject.name, rawg_key)?.into_iter().next() {
            Some(first) => (first.provider, first.id),
            None => return Ok(None),
        },
    };
    let mut details = match provider.as_str() {
        "steam" => steam_fetch(&id)?,
        "rawg" if rawg_key.is_empty() => return Err(i18n::t("err.rawg_key_page")),
        "rawg" => rawg_fetch(&id, rawg_key)?,
        "wikipedia" => match wiki_fetch(&id)? {
            Some(d) => d,
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    details.fetched_at = now_ms;
    details.ui_lang = i18n::lang().to_string();
    if provider == "steam" {
        if let Some(rating) = steam_reviews(&id) {
            details.rating = Some(rating);
        }
        details.reviews_at = now_ms;
    }
    Ok(Some(details))
}

/// Pages possibles pour « Ce n'est pas le bon ? ». Un jeu Steam a une page exacte : rien à corriger.
pub fn candidates(subject: &Subject, rawg_key: &str) -> Result<Vec<Candidate>, String> {
    if subject.target.starts_with("steam://") {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    if subject.source == "manual" {
        found.extend(wiki_search(&subject.name)?);
    }
    if !rawg_key.is_empty() {
        found.extend(rawg_search(&subject.name, rawg_key)?);
    }
    Ok(found)
}

pub fn needs_rawg_key(subject: &Subject, rawg_key: &str, choice: Option<&Choice>) -> bool {
    rawg_key.is_empty()
        && subject.source != "manual"
        && !subject.target.starts_with("steam://")
        && choice.is_none_or(|c| c.provider == "rawg")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn steam_sample() -> Value {
        json!({"3241660": {"success": true, "data": {
            "name": "R.E.P.O.",
            "short_description": "Un jeu d&#39;horreur coop&eacute;ratif.",
            "about_the_game": "<p class=\"bb_paragraph\">Premier <b>paragraphe</b>.<br>Ligne deux &amp; fin.</p><ul><li>Un</li><li>Deux</li></ul>",
            "supported_languages": "Anglais<strong>*</strong><br><strong>*</strong>Langues avec support audio complet",
            "genres": [{"id": "1", "description": "Action"}, {"id": "70", "description": "Accès anticipé"}],
            "developers": ["semiwork"], "publishers": ["semiwork"],
            "release_date": {"coming_soon": false, "date": "26 févr. 2025"},
            "platforms": {"windows": true, "mac": false, "linux": false},
            "recommendations": {"total": 346424},
            "metacritic": {"score": 90},
            "website": "https://example.com",
            "screenshots": [{"path_full": "https://cdn/ss_1.1920x1080.jpg?t=1791817973"}, {"path_full": "https://cdn/ss_2.jpg"}]
        }}})
    }

    #[test]
    fn texte_html() {
        assert_eq!(html_to_text("<p>Un &amp; deux</p><p>Trois&nbsp;!</p>"), "Un & deux\n\nTrois !");
        assert_eq!(html_to_text("a<br>b<br><br><br>c"), "a\nb\n\nc");
        assert_eq!(html_to_text("<ul><li>x</li><li>y</li></ul>"), "• x\n• y");
        assert_eq!(html_to_text("<img src=\"a.png\"> sans balise < ouverte"), "sans balise < ouverte");
        assert_eq!(decode_entities("&#233;&#x41; &inconnu; &"), "éA &inconnu; &");
        assert_eq!(strip_bbcode("[h1]Titre[/h1] [url=https://x]lien[/url] fin"), "Titre lien fin");
        assert_eq!(truncate("abcdef", 4), "abcd…");
        assert_eq!(truncate("abc", 4), "abc");
    }

    #[test]
    fn page_steam() {
        let d = parse_steam_details(&steam_sample(), "3241660").unwrap();
        assert_eq!((d.provider.as_str(), d.match_name.as_str(), d.lang.as_str()), ("steam", "R.E.P.O.", "en"));
        assert_eq!(d.summary, "Un jeu d'horreur coopératif.");
        assert_eq!(d.description, "Premier paragraphe.\nLigne deux & fin.\n\n• Un\n• Deux");
        assert_eq!(d.genres, ["Action", "Accès anticipé"]);
        assert_eq!(d.platforms, ["Windows"]);
        assert_eq!(d.release_date, "26 févr. 2025");
        assert_eq!(d.metacritic, Some(90));
        assert_eq!(d.rating, Some(Rating { label: "Recommandations Steam".into(), percent: None, count: 346424 }));
        // Le paramètre de version est retiré des adresses d'images.
        assert_eq!(d.screenshot_urls, ["https://cdn/ss_1.1920x1080.jpg", "https://cdn/ss_2.jpg"]);
        assert_eq!(d.page_url, "https://store.steampowered.com/app/3241660");
        // Jeu absent, ou succès faux.
        assert!(parse_steam_details(&steam_sample(), "1").is_none());
        assert!(parse_steam_details(&json!({"9": {"success": false}}), "9").is_none());
        // Un jeu traduit en français est signalé comme tel.
        let mut fr = steam_sample();
        fr["3241660"]["data"]["supported_languages"] = json!("Anglais*, Français, Allemand");
        assert_eq!(parse_steam_details(&fr, "3241660").unwrap().lang, "fr");
    }

    #[test]
    fn avis_et_actualites_steam() {
        let reviews = json!({"success": 1, "query_summary": {"total_reviews": 200, "total_positive": 170, "review_score_desc": "Very Positive"}});
        assert_eq!(parse_steam_reviews(&reviews), Some(Rating { label: "Très positives".into(), percent: Some(85), count: 200 }));
        assert_eq!(parse_steam_reviews(&json!({"query_summary": {"total_reviews": 0, "total_positive": 0}})), None);
        assert_eq!(parse_steam_reviews(&json!({})), None);

        let news = json!({"appnews": {"newsitems": [
            {"title": "v0.4.4", "url": "https://x/1", "date": 1779701699, "feedlabel": "Community Announcements", "contents": "[h1]Notes[/h1] Correctifs &amp; ajouts"},
            {"title": "Lien dangereux", "url": "javascript:alert(1)", "date": 1, "contents": ""},
            {"title": "", "url": "https://x/3", "date": 1, "contents": ""}
        ]}});
        let items = parse_steam_news(&news);
        assert_eq!(items.len(), 1);
        assert_eq!((items[0].title.as_str(), items[0].date), ("v0.4.4", 1779701699));
        assert_eq!(items[0].summary, "Notes Correctifs & ajouts");
    }

    #[test]
    fn pages_rawg() {
        let search = json!({"results": [
            {"id": 3498, "name": "Grand Theft Auto V", "released": "2013-09-17"},
            {"id": 1, "name": "Sans date", "released": null},
            {"name": "Sans identifiant"}
        ]});
        let found = parse_rawg_search(&search);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], Candidate { provider: "rawg".into(), id: "3498".into(), name: "Grand Theft Auto V".into(), hint: "2013".into() });
        assert_eq!(found[1].hint, "");

        let detail = json!({"id": 3498, "slug": "gta-v", "name": "Grand Theft Auto V", "released": "2013-09-17", "rating": 4.47, "ratings_count": 6000,
            "metacritic": 92, "description_raw": "Première ligne.\n\nSuite.", "website": "https://rockstargames.com",
            "genres": [{"name": "Action"}], "developers": [{"name": "Rockstar North"}], "publishers": [{"name": "Rockstar Games"}],
            "platforms": [{"platform": {"name": "PC"}}, {"platform": {"name": "PlayStation 5"}}]});
        let d = parse_rawg_details(&detail).unwrap();
        assert_eq!((d.provider.as_str(), d.lang.as_str(), d.match_id.as_str()), ("rawg", "en", "3498"));
        assert_eq!(d.summary, "Première ligne.");
        assert_eq!(d.platforms, ["PC", "PlayStation 5"]);
        assert_eq!(d.rating, Some(Rating { label: "Note des joueurs RAWG".into(), percent: Some(89), count: 6000 }));
        assert_eq!(d.metacritic, Some(92));
        assert_eq!(d.page_url, "https://rawg.io/games/gta-v");
        assert_eq!(parse_rawg_shots(&json!({"results": [{"image": "https://a/1.jpg"}, {"id": 2}]})), ["https://a/1.jpg"]);
    }

    #[test]
    fn pages_wikipedia() {
        let search = json!({"query": {"search": [{"title": "Blender", "snippet": "<span class=\"searchmatch\">Blender</span> est un logiciel"}]}});
        let found = parse_wiki_search(&search, "fr");
        assert_eq!(found[0].id, "fr:Blender");
        assert_eq!(found[0].hint, "Blender est un logiciel");
        assert!(parse_wiki_search(&json!({"query": {"search": []}}), "fr").is_empty());

        let page = json!({"type": "standard", "title": "Blender", "description": "logiciel libre de graphisme 3D",
            "extract": "Blender est un logiciel libre.", "content_urls": {"desktop": {"page": "https://fr.wikipedia.org/wiki/Blender"}}});
        let d = parse_wiki_summary(&page, "fr").unwrap();
        assert_eq!((d.provider.as_str(), d.lang.as_str(), d.match_id.as_str()), ("wikipedia", "fr", "fr:Blender"));
        assert_eq!(d.description, "Blender est un logiciel libre.");
        assert_eq!(d.page_url, "https://fr.wikipedia.org/wiki/Blender");
        // Une page d'homonymie ne décrit rien.
        assert!(parse_wiki_summary(&json!({"type": "disambiguation", "title": "X", "extract": "Peut désigner…"}), "fr").is_none());
    }

    fn subject(source: &str, target: &str) -> Subject {
        Subject { name: "Jeu".into(), source: source.into(), target: target.into() }
    }

    #[test]
    fn choix_du_fournisseur() {
        let steam = subject("steam", "steam://rungameid/42");
        assert_eq!(known_provider(&steam, None), Some(("steam".into(), "42".into())));
        // Le choix du joueur l'emporte, y compris « aucune page ».
        let rawg = Choice { provider: "rawg".into(), id: "7".into() };
        assert_eq!(known_provider(&steam, Some(&rawg)), Some(("rawg".into(), "7".into())));
        assert_eq!(known_provider(&steam, Some(&Choice { provider: "none".into(), id: String::new() })), None);
        assert_eq!(known_provider(&subject("epic", "com.epicgames.launcher://x"), None), None);

        assert!(candidates(&steam, "cle").unwrap().is_empty());
        assert!(needs_rawg_key(&subject("epic", "x"), "", None));
        assert!(!needs_rawg_key(&subject("epic", "x"), "cle", None));
        assert!(!needs_rawg_key(&subject("manual", "C:\\a.exe"), "", None));
        assert!(!needs_rawg_key(&steam, "", None));
    }

    #[test]
    fn cache_sur_disque() {
        let dir = std::env::temp_dir().join("tronix-test-details");
        let _ = fs::remove_dir_all(&dir);
        assert!(load(&dir, "a").is_none());
        let d = Details { provider: "steam".into(), match_name: "Jeu".into(), fetched_at: 5, ..Default::default() };
        save(&dir, "a", &d).unwrap();
        assert_eq!(load(&dir, "a").unwrap().match_name, "Jeu");

        assert!(load_choice(&dir, "a").is_none());
        let choice = Choice { provider: "rawg".into(), id: "9".into() };
        save_choice(&dir, "a", Some(&choice)).unwrap();
        assert_eq!(load_choice(&dir, "a"), Some(choice));
        save_choice(&dir, "a", None).unwrap();
        save_choice(&dir, "a", None).unwrap(); // idempotent
        assert!(load_choice(&dir, "a").is_none());

        fs::create_dir_all(dir.join("a")).unwrap();
        fs::write(dir.join("a").join("shot2.jpg"), b"x").unwrap();
        fs::write(dir.join("a").join("shot1.png"), b"x").unwrap();
        fs::write(dir.join("a").join("autre.txt"), b"x").unwrap();
        let shots: Vec<_> = local_shots(&dir, "a").into_iter().map(|(p, _)| p.rsplit('\\').next().unwrap().to_string()).collect();
        assert_eq!(shots, ["shot1.png", "shot2.jpg"]);
        forget(&dir, "a");
        assert!(load(&dir, "a").is_none() && local_shots(&dir, "a").is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn encodage_des_requetes() {
        assert_eq!(percent_encode("Notepad++ & co"), "Notepad%2B%2B%20%26%20co");
    }
}

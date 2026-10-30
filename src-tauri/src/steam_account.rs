//! Jeux Steam possédés par le joueur, installés ou non, via l'API web de Steam.
//!
//! Le SteamID se lit dans les fichiers de Steam : Tronix ne demande jamais le mot de passe Steam.
//! La clé d'API ne figure dans aucun message d'erreur.

use serde_json::Value;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

const MAX_BYTES: u64 = 16 * 1024 * 1024;

// Clés de texte (voir `i18n`) : traduites au moment d'afficher l'erreur, pas avant.
pub const NO_STEAM: &str = "err.steam_not_found_pc";
pub const NO_KEY: &str = "err.steam_no_key";
pub const BAD_KEY: &str = "err.steam_bad_key";
pub const PRIVATE: &str = "err.steam_private";
pub const NETWORK: &str = "err.steam_network";
pub const UNEXPECTED: &str = "err.steam_unexpected";

/// Un jeu possédé sur le compte Steam.
#[derive(Debug, PartialEq)]
pub struct Owned {
    pub appid: String,
    pub name: String,
}

/// SteamID64 du compte : le dernier connecté, sinon le premier compte du fichier `loginusers.vdf`.
pub fn steam_id_from_vdf(text: &str) -> Option<String> {
    let (mut ids, mut current, mut recent) = (Vec::new(), None::<String>, None::<String>);
    for line in text.lines() {
        let mut parts = line.split('"');
        parts.next();
        let (Some(key), _, value) = (parts.next(), parts.next(), parts.next()) else { continue };
        let is_id = key.len() == 17 && key.bytes().all(|b| b.is_ascii_digit());
        if is_id && value.is_none() {
            current = Some(key.to_string());
            ids.push(key.to_string());
        } else if key == "MostRecent" && value == Some("1") {
            recent = current.clone();
        }
    }
    recent.or_else(|| ids.into_iter().next())
}

/// SteamID64 lu dans le dossier d'installation de Steam.
pub fn steam_id(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join("config").join("loginusers.vdf")).ok()?;
    steam_id_from_vdf(&text)
}

/// Décode la réponse de `GetOwnedGames`. Un profil privé répond sans liste de jeux.
pub fn parse_owned(json: &Value) -> Result<Vec<Owned>, &'static str> {
    let response = json.get("response").and_then(Value::as_object).ok_or(UNEXPECTED)?;
    let Some(games) = response.get("games").and_then(Value::as_array) else {
        // Un compte public sans aucun jeu annonce `game_count: 0` ; un compte privé n'annonce rien.
        return if response.get("game_count").and_then(Value::as_u64) == Some(0) { Ok(Vec::new()) } else { Err(PRIVATE) };
    };
    Ok(games
        .iter()
        .filter_map(|game| {
            let appid = game.get("appid")?.as_u64()?.to_string();
            let name = game.get("name")?.as_str()?.trim().to_string();
            (!name.is_empty() && !crate::imports::steam_is_tool(&appid, &name)).then_some(Owned { appid, name })
        })
        .collect())
}

/// Interroge l'API Steam. Appel réseau : à lancer hors de tout verrou.
pub fn fetch_owned(key: &str, steam_id: &str) -> Result<Vec<Owned>, String> {
    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key={key}&steamid={steam_id}&include_appinfo=1&include_played_free_games=1&format=json"
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .https_only(true)
        .redirects(0) // la clé est dans l'adresse : elle ne doit jamais suivre une redirection
        .user_agent("TronixLauncher/0.1 (launcher de jeux)")
        .build();
    let response = agent.get(&url).call().map_err(|e| match e {
        ureq::Error::Status(400 | 401 | 403, _) => crate::i18n::t(BAD_KEY),
        _ => crate::i18n::t(NETWORK),
    })?;
    let mut bytes = Vec::new();
    response.into_reader().take(MAX_BYTES).read_to_end(&mut bytes).map_err(|_| crate::i18n::t(NETWORK))?;
    let json: Value = serde_json::from_slice(&bytes).map_err(|_| crate::i18n::t(UNEXPECTED))?;
    parse_owned(&json).map_err(crate::i18n::t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const VDF: &str = "\"users\"\n{\n\t\"76561199122573107\"\n\t{\n\t\t\"AccountName\"\t\t\"x\"\n\t\t\"MostRecent\"\t\t\"0\"\n\t}\n\t\"76561198000000001\"\n\t{\n\t\t\"AccountName\"\t\t\"y\"\n\t\t\"MostRecent\"\t\t\"1\"\n\t}\n}\n";

    #[test]
    fn steamid_du_dernier_compte_connecte() {
        assert_eq!(steam_id_from_vdf(VDF).as_deref(), Some("76561198000000001"));
        // Sans « MostRecent » : le premier compte.
        let single = "\"users\"\n{\n\t\"76561199122573107\"\n\t{\n\t\t\"AccountName\"\t\t\"x\"\n\t}\n}";
        assert_eq!(steam_id_from_vdf(single).as_deref(), Some("76561199122573107"));
        assert_eq!(steam_id_from_vdf("\"users\"\n{\n}"), None);
        assert_eq!(steam_id_from_vdf(""), None);
    }

    #[test]
    fn jeux_possedes() {
        let ok = json!({"response": {"game_count": 3, "games": [
            {"appid": 620, "name": "Portal 2"},
            {"appid": 228980, "name": "Steamworks Common Redistributables"},
            {"appid": 5, "name": "  "}
        ]}});
        assert_eq!(parse_owned(&ok).unwrap(), vec![Owned { appid: "620".into(), name: "Portal 2".into() }]);
        // Profil privé : réponse vide.
        assert_eq!(parse_owned(&json!({"response": {}})), Err(PRIVATE));
        // Profil public sans aucun jeu.
        assert_eq!(parse_owned(&json!({"response": {"game_count": 0}})), Ok(vec![]));
        assert_eq!(parse_owned(&json!({"autre": 1})), Err(UNEXPECTED));
        assert_eq!(parse_owned(&json!([])), Err(UNEXPECTED));
    }

    /// Le vrai fichier de ce PC, s'il existe : un SteamID64 valide en sort.
    #[cfg(windows)]
    #[test]
    fn steamid_du_vrai_fichier() {
        let Some(root) = crate::imports::steam_root() else { return };
        if let Some(id) = steam_id(&root) {
            assert!(id.len() == 17 && id.starts_with("7656119"), "SteamID inattendu");
        }
    }

    /// Appel réseau réel avec une fausse clé (à lancer avec `--ignored`) : Steam doit la refuser.
    #[test]
    #[ignore]
    fn fausse_cle_refusee() {
        assert_eq!(fetch_owned("0123456789ABCDEF0123456789ABCDEF", "76561199122573107"), Err(crate::i18n::t(BAD_KEY)));
    }
}

//! Chiffrement des clés d'API avec le coffre de Windows (DPAPI).
//!
//! Une clé chiffrée n'est lisible que par le même compte Windows sur le même PC : un fichier de
//! réglages copié ailleurs ne la révèle pas.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// Préfixe des valeurs chiffrées dans `settings.json`.
const PREFIX: &str = "dpapi:";

/// Vrai si la valeur enregistrée est déjà chiffrée.
pub fn is_protected(stored: &str) -> bool {
    stored.starts_with(PREFIX)
}

/// Chiffre une clé pour l'écrire sur le disque. Une valeur vide reste vide.
pub fn protect(plain: &str) -> Option<String> {
    if plain.is_empty() {
        return Some(String::new());
    }
    let encrypted = dpapi(plain.as_bytes(), true)?;
    Some(format!("{PREFIX}{}", STANDARD.encode(encrypted)))
}

/// Relit une valeur enregistrée : déchiffrée si elle l'est, sinon renvoyée telle quelle
/// (ancien fichier en clair). `None` si une valeur chiffrée ne peut pas être déchiffrée.
pub fn unprotect(stored: &str) -> Option<String> {
    let Some(encoded) = stored.strip_prefix(PREFIX) else {
        return Some(stored.to_string());
    };
    let bytes = STANDARD.decode(encoded).ok()?;
    String::from_utf8(dpapi(&bytes, false)?).ok()
}

#[cfg(windows)]
fn dpapi(input: &[u8], encrypt: bool) -> Option<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData};

    let mut data = CRYPT_INTEGER_BLOB { cbData: input.len() as u32, pbData: input.as_ptr() as *mut u8 };
    let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    // SAFETY : `data` pointe vers `input`, valable pendant l'appel ; le tampon de sortie, alloué par
    // Windows, est copié puis libéré.
    unsafe {
        let ok = if encrypt {
            CryptProtectData(&mut data, std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut out)
        } else {
            CryptUnprotectData(&mut data, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut out)
        };
        if ok == 0 || out.pbData.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as *mut _);
        Some(bytes)
    }
}

#[cfg(not(windows))]
fn dpapi(input: &[u8], _encrypt: bool) -> Option<Vec<u8>> {
    Some(input.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_chiffre() {
        let stored = protect("ma-cle-secrete-123").unwrap();
        assert!(is_protected(&stored));
        assert!(!stored.contains("ma-cle-secrete"));
        assert_eq!(unprotect(&stored).as_deref(), Some("ma-cle-secrete-123"));
        // Deux chiffrements de la même clé ne donnent pas le même texte (sel aléatoire).
        assert_ne!(protect("ma-cle-secrete-123").unwrap(), stored);
    }

    #[test]
    fn valeurs_particulieres() {
        assert_eq!(protect("").as_deref(), Some(""));
        assert_eq!(unprotect("").as_deref(), Some(""));
        // Un ancien fichier en clair reste lisible.
        assert!(!is_protected("cle-en-clair"));
        assert_eq!(unprotect("cle-en-clair").as_deref(), Some("cle-en-clair"));
        // Accents et caractères longs.
        let odd = "clé-é€-".repeat(40);
        assert_eq!(unprotect(&protect(&odd).unwrap()).as_deref(), Some(odd.as_str()));
        // Valeur chiffrée abîmée : refus plutôt qu'une clé fausse.
        assert_eq!(unprotect("dpapi:pas-du-base64!"), None);
        assert_eq!(unprotect("dpapi:AAAA"), None);
    }
}

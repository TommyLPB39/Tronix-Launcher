use crate::store::Program;

const DAY_MS: f64 = 86_400_000.0;
/// Un lancement perd la moitié de son poids environ tous les 10 jours.
const DECAY_DAYS: f64 = 14.0;
/// Bonus quand le lancement a eu lieu à la même heure de la journée (±2 h).
const HOUR_BONUS: f64 = 0.75;

/// Heure locale (0-23) d'un horodatage. `tz_offset_min` suit la convention de
/// JavaScript : minutes à retrancher à l'heure UTC pour obtenir l'heure locale.
fn local_hour(ts_ms: i64, tz_offset_min: i32) -> i64 {
    let minutes = ts_ms / 60_000 - tz_offset_min as i64;
    minutes.rem_euclid(1440) / 60
}

fn hour_distance(a: i64, b: i64) -> i64 {
    let d = (a - b).abs();
    d.min(24 - d)
}

/// Plus le score est haut, plus le programme a de chances d'être lancé maintenant :
/// fréquence, récence et heure habituelle de lancement.
pub fn score(p: &Program, now_ms: i64, tz_offset_min: i32) -> f64 {
    let now_hour = local_hour(now_ms, tz_offset_min);
    p.launches
        .iter()
        .map(|&t| {
            let age_days = ((now_ms - t).max(0) as f64) / DAY_MS;
            let recency = (-age_days / DECAY_DAYS).exp();
            let same_hour = hour_distance(now_hour, local_hour(t, tz_offset_min)) <= 2;
            recency * if same_hour { 1.0 + HOUR_BONUS } else { 1.0 }
        })
        .sum()
}

/// Classement du plus au moins probable ; à égalité, le plus récemment ajouté d'abord.
pub fn ranked(programs: &[Program], now_ms: i64, tz_offset_min: i32) -> Vec<Program> {
    let mut list: Vec<(f64, Program)> = programs
        .iter()
        .map(|p| (score(p, now_ms, tz_offset_min), p.clone()))
        .collect();
    list.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.1.added.cmp(&a.1.added))
    });
    list.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: i64 = 3_600_000;
    const D: i64 = 24 * H;
    // 2026-01-15 21:00 UTC, fuseau UTC (offset 0)
    const NOW: i64 = 1_768_510_800_000;

    fn prog(id: &str, added: i64, launches: Vec<i64>) -> Program {
        Program {
            id: id.into(),
            name: id.into(),
            target: String::new(),
            source: "manual".into(),
            added,
            launches,
            ..Default::default()
        }
    }

    fn order(list: &[Program]) -> Vec<String> {
        list.iter().map(|p| p.id.clone()).collect()
    }

    #[test]
    fn heure_locale() {
        assert_eq!(local_hour(NOW, 0), 21);
        assert_eq!(local_hour(NOW, -60), 22); // UTC+1 (JS : -60)
        assert_eq!(local_hour(NOW, 300), 16); // UTC-5
    }

    #[test]
    fn jamais_lance_trie_par_ajout_recent() {
        let r = ranked(&[prog("vieux", 1, vec![]), prog("neuf", 2, vec![])], NOW, 0);
        assert_eq!(order(&r), ["neuf", "vieux"]);
    }

    #[test]
    fn frequence_et_recence() {
        let a = prog("souvent", 1, vec![NOW - 2 * D, NOW - 3 * D, NOW - 4 * D]);
        let b = prog("une_fois", 2, vec![NOW - 2 * D]);
        let c = prog("ancien", 3, vec![NOW - 90 * D, NOW - 91 * D]);
        let r = ranked(&[c, b, a], NOW, 0);
        assert_eq!(order(&r), ["souvent", "une_fois", "ancien"]);
    }

    #[test]
    fn heure_habituelle() {
        // `soir` a été lancé à 21 h comme maintenant ; `matin` à 9 h, et un peu plus récemment.
        let soir = prog("soir", 1, vec![NOW - 3 * D]);
        let matin = prog("matin", 2, vec![NOW - 3 * D + 12 * H]);
        let r = ranked(&[matin, soir], NOW, 0);
        assert_eq!(order(&r), ["soir", "matin"]);
    }
}

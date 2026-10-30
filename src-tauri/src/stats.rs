//! Statistiques d'usage, calculées à partir des lancements enregistrés.

use crate::store::Program;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const WEEKDAYS: [&str; 7] = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];
/// Nombre de programmes dans le classement.
const TOP: usize = 10;

#[derive(Serialize, Debug, PartialEq)]
pub struct Count {
    pub name: String,
    pub count: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct DayCount {
    /// Jour de la semaine, en toutes lettres.
    pub label: String,
    pub count: usize,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Week {
    pub launches: usize,
    /// Programme le plus lancé sur les 7 derniers jours.
    pub top: Option<Count>,
    /// Jour le plus actif sur les 7 derniers jours.
    pub busiest_day: Option<DayCount>,
    /// Programmes lancés pour la première fois cette semaine.
    pub new_played: Vec<String>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct TopProgram {
    pub id: String,
    pub name: String,
    pub source: String,
    pub launches: usize,
    pub play_ms: u64,
    pub play_est_ms: u64,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Stats {
    pub total_launches: usize,
    pub programs_played: usize,
    pub current_streak: u32,
    pub best_streak: u32,
    pub total_play_ms: u64,
    pub total_play_est_ms: u64,
    pub week: Week,
    pub top: Vec<TopProgram>,
}

/// Numéro du jour local (jours depuis le 1er janvier 1970). `tz_offset_min` suit la
/// convention de JavaScript : minutes à retrancher à l'heure UTC pour obtenir l'heure locale.
fn local_day(ts_ms: i64, tz_offset_min: i32) -> i64 {
    (ts_ms / 60_000 - tz_offset_min as i64).div_euclid(1440)
}

/// Jour de la semaine (0 = dimanche). Le 1er janvier 1970 était un jeudi.
fn weekday(day: i64) -> usize {
    (day + 4).rem_euclid(7) as usize
}

/// Jours consécutifs avec au moins un lancement : (série en cours, meilleure série).
/// La série en cours reste vivante tant qu'on a joué aujourd'hui ou hier.
fn streaks(days: &BTreeSet<i64>, today: i64) -> (u32, u32) {
    let mut best = 0;
    let mut run = 0;
    let mut previous: Option<i64> = None;
    for &day in days {
        run = if previous == Some(day - 1) { run + 1 } else { 1 };
        best = best.max(run);
        previous = Some(day);
    }
    let mut current = 0;
    let mut day = if days.contains(&today) { today } else { today - 1 };
    while days.contains(&day) {
        current += 1;
        day -= 1;
    }
    (current, best)
}

pub fn compute(programs: &[Program], now_ms: i64, tz_offset_min: i32) -> Stats {
    let today = local_day(now_ms, tz_offset_min);
    let week_start = today - 6;

    let mut days = BTreeSet::new();
    let mut week_per_day: BTreeMap<i64, usize> = BTreeMap::new();
    let mut week_per_program: Vec<(&Program, usize)> = Vec::new();
    let mut new_played = Vec::new();

    for program in programs {
        let mut in_week = 0;
        for &launch in &program.launches {
            let day = local_day(launch, tz_offset_min);
            days.insert(day);
            if (week_start..=today).contains(&day) {
                in_week += 1;
                *week_per_day.entry(day).or_default() += 1;
            }
        }
        if in_week > 0 {
            week_per_program.push((program, in_week));
        }
        let first = program.launches.iter().min().map(|&t| local_day(t, tz_offset_min));
        if first.is_some_and(|d| (week_start..=today).contains(&d)) {
            new_played.push(program.name.clone());
        }
    }

    // À égalité, on garde le jour le plus récent et le premier programme dans l'ordre reçu.
    let busiest_day = week_per_day
        .iter()
        .max_by_key(|&(day, count)| (*count, *day))
        .map(|(&day, &count)| DayCount {
            label: WEEKDAYS[weekday(day)].to_string(),
            count,
        });
    let top = week_per_program
        .iter()
        .rev()
        .max_by_key(|(_, count)| *count)
        .map(|(program, count)| Count {
            name: program.name.clone(),
            count: *count,
        });

    let (current_streak, best_streak) = streaks(&days, today);

    let mut ranking: Vec<TopProgram> = programs
        .iter()
        .filter(|p| !p.launches.is_empty() || p.play_ms + p.play_est_ms > 0)
        .map(|p| TopProgram {
            id: p.id.clone(),
            name: p.name.clone(),
            source: p.source.clone(),
            launches: p.launches.len(),
            play_ms: p.play_ms,
            play_est_ms: p.play_est_ms,
        })
        .collect();
    ranking.sort_by(|a, b| {
        b.launches
            .cmp(&a.launches)
            .then((b.play_ms + b.play_est_ms).cmp(&(a.play_ms + a.play_est_ms)))
            .then(a.name.cmp(&b.name))
    });
    ranking.truncate(TOP);

    Stats {
        total_launches: programs.iter().map(|p| p.launches.len()).sum(),
        programs_played: programs.iter().filter(|p| !p.launches.is_empty()).count(),
        current_streak,
        best_streak,
        total_play_ms: programs.iter().map(|p| p.play_ms).sum(),
        total_play_est_ms: programs.iter().map(|p| p.play_est_ms).sum(),
        week: Week {
            launches: week_per_day.values().sum(),
            top,
            busiest_day,
            new_played,
        },
        top: ranking,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: i64 = 3_600_000;
    const D: i64 = 24 * H;
    // jeudi 15 janvier 2026, 21 h UTC
    const NOW: i64 = 1_768_510_800_000;

    fn prog(name: &str, launches: Vec<i64>) -> Program {
        Program {
            id: name.to_lowercase(),
            name: name.into(),
            source: "manual".into(),
            launches,
            ..Default::default()
        }
    }

    #[test]
    fn jours_de_la_semaine() {
        assert_eq!(WEEKDAYS[weekday(local_day(NOW, 0))], "thursday");
        assert_eq!(WEEKDAYS[weekday(local_day(0, 0))], "thursday"); // 1er janvier 1970
        assert_eq!(WEEKDAYS[weekday(local_day(NOW + 4 * H, 0))], "friday"); // passé minuit
        // Même instant, vu depuis UTC-5 (JavaScript : +300) : encore jeudi à 16 h.
        assert_eq!(WEEKDAYS[weekday(local_day(NOW, 300))], "thursday");
    }

    #[test]
    fn series_de_jours() {
        let day = |n: i64| local_day(NOW - n * D, 0);
        let set = |ns: &[i64]| ns.iter().map(|&n| day(n)).collect::<BTreeSet<_>>();
        let today = day(0);
        assert_eq!(streaks(&set(&[]), today), (0, 0));
        assert_eq!(streaks(&set(&[0]), today), (1, 1));
        assert_eq!(streaks(&set(&[0, 1, 2]), today), (3, 3));
        // Pas encore joué aujourd'hui : la série d'hier reste vivante.
        assert_eq!(streaks(&set(&[1, 2]), today), (2, 2));
        // Un jour sans jouer avant hier : la série en cours est perdue, la meilleure reste.
        assert_eq!(streaks(&set(&[2, 3, 4, 5]), today), (0, 4));
        // Une série passée plus longue que la série en cours.
        assert_eq!(streaks(&set(&[0, 1, 5, 6, 7, 8]), today), (2, 4));
    }

    #[test]
    fn bilan_de_la_semaine() {
        let jeu = prog("Jeu", vec![NOW - 2 * D, NOW - 2 * D + H, NOW - D, NOW - 20 * D]);
        let outil = prog("Outil", vec![NOW - 2 * D + 2 * H]);
        let nouveau = prog("Nouveau", vec![NOW - D]);
        let vieux = prog("Vieux", vec![NOW - 30 * D]);
        let jamais = prog("Jamais", vec![]);
        let s = compute(&[jeu, outil, nouveau, vieux, jamais], NOW, 0);

        assert_eq!(s.total_launches, 4 + 1 + 1 + 1);
        assert_eq!(s.programs_played, 4);
        assert_eq!(s.week.launches, 5); // le lancement d'il y a 20 jours est exclu
        assert_eq!(s.week.top, Some(Count { name: "Jeu".into(), count: 3 }));
        // Avant-hier (mardi) : 3 lancements ; hier : 2.
        assert_eq!(s.week.busiest_day, Some(DayCount { label: "tuesday".into(), count: 3 }));
        // « Jeu » existait déjà avant cette semaine ; « Outil » et « Nouveau » sont des nouveautés.
        assert_eq!(s.week.new_played, ["Outil", "Nouveau"]);
        assert_eq!(s.current_streak, 2);
    }

    #[test]
    fn rien_de_joue() {
        let s = compute(&[prog("Seul", vec![])], NOW, 0);
        assert_eq!((s.total_launches, s.current_streak, s.best_streak), (0, 0, 0));
        assert!(s.week.top.is_none() && s.week.busiest_day.is_none() && s.week.new_played.is_empty());
        assert!(s.top.is_empty());
    }

    #[test]
    fn classement_et_temps_de_jeu() {
        let mut a = prog("A", vec![NOW; 3]);
        a.play_ms = 60_000;
        let mut b = prog("B", vec![NOW; 3]);
        b.play_est_ms = 120_000;
        let mut c = prog("C", vec![]);
        c.play_ms = 5_000; // du temps de jeu sans lancement enregistré compte quand même
        let s = compute(&[a, b, c], NOW, 0);
        let names: Vec<_> = s.top.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["B", "A", "C"]); // à lancements égaux, le plus long d'abord
        assert_eq!((s.total_play_ms, s.total_play_est_ms), (65_000, 120_000));
    }
}

//! Temps de jeu : mesuré quand Tronix lance lui-même le programme, estimé sinon.

use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

/// Durée pendant laquelle un processus lancé par Tronix a tourné (mesure exacte).
pub fn measure_child(mut child: Child) -> u64 {
    let start = Instant::now();
    let _ = child.wait();
    start.elapsed().as_millis() as u64
}

fn normalize(dir: &str) -> String {
    dir.to_lowercase().replace('/', "\\").trim_end_matches('\\').to_string()
}

/// Vrai si un processus en cours d'exécution a son exécutable dans ce dossier.
#[cfg(windows)]
pub fn running_in_dir(dir: &Path) -> bool {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};

    let wanted = format!("{}\\", normalize(&dir.to_string_lossy()));
    let mut found = false;
    // SAFETY : appels Win32 sur des tampons valides ; chaque poignée ouverte est refermée.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry);
        while more != 0 && !found {
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
            if !process.is_null() {
                let mut buffer = [0u16; 1024];
                let mut length = buffer.len() as u32;
                if QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) != 0 {
                    let path = String::from_utf16_lossy(&buffer[..length as usize]).to_lowercase();
                    found = path.starts_with(&wanted);
                }
                CloseHandle(process);
            }
            more = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    found
}

#[cfg(not(windows))]
pub fn running_in_dir(_dir: &Path) -> bool {
    false
}

/// Estime la durée de jeu d'un programme lancé par un autre launcher, en guettant
/// un processus de son dossier d'installation.
///
/// - attend jusqu'à `appear_timeout` que le jeu apparaisse (sinon : 0) ;
/// - le compte comme terminé quand plus aucun processus n'est vu pendant `gone_grace`.
pub fn watch_dir(dir: &Path, poll: Duration, appear_timeout: Duration, gone_grace: Duration) -> u64 {
    let waiting_since = Instant::now();
    while !running_in_dir(dir) {
        if waiting_since.elapsed() > appear_timeout {
            return 0;
        }
        std::thread::sleep(poll);
    }
    let start = Instant::now();
    let mut last_seen = Instant::now();
    loop {
        std::thread::sleep(poll);
        if running_in_dir(dir) {
            last_seen = Instant::now();
        } else if last_seen.elapsed() > gone_grace {
            break;
        }
    }
    // Le jeu tournait déjà un peu avant la première observation et un peu après la dernière :
    // on ajoute un intervalle de sondage, sans quoi un jeu vu une seule fois compterait zéro.
    (last_seen.duration_since(start) + poll).as_millis() as u64
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    /// Un dossier à nous contenant une copie de `ping.exe`, qui sert de « jeu » de quelques secondes.
    fn fake_game(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("faux-jeu.exe");
        fs::copy(r"C:\Windows\System32\PING.EXE", &exe).unwrap();
        (dir, exe)
    }

    #[test]
    fn duree_mesuree_d_un_processus_lance() {
        let (dir, exe) = fake_game("tronix-test-mesure");
        let child = Command::new(&exe).args(["-n", "3", "127.0.0.1"]).spawn().unwrap();
        let ms = measure_child(child);
        assert!((1000..8000).contains(&ms), "durée mesurée : {ms} ms");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn processus_repere_dans_un_dossier() {
        let (dir, exe) = fake_game("tronix-test-repere");
        assert!(!running_in_dir(&dir));
        let mut child = Command::new(&exe).args(["-n", "6", "127.0.0.1"]).spawn().unwrap();
        std::thread::sleep(Duration::from_millis(400));
        assert!(running_in_dir(&dir));
        assert!(!running_in_dir(Path::new(r"C:\dossier\qui\n'existe\pas")));
        let _ = child.kill();
        let _ = child.wait();
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn duree_estimee_d_un_jeu_lance_ailleurs() {
        let (dir, exe) = fake_game("tronix-test-estime");
        let launcher = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            Command::new(&exe).args(["-n", "4", "127.0.0.1"]).spawn().unwrap().wait().unwrap();
        });
        let ms = watch_dir(&dir, Duration::from_millis(150), Duration::from_secs(5), Duration::from_millis(600));
        launcher.join().unwrap();
        assert!((1500..9000).contains(&ms), "durée estimée : {ms} ms");
        // Un jeu qui n'apparaît jamais ne compte aucun temps.
        let none = watch_dir(Path::new(r"C:\rien\ici"), Duration::from_millis(50), Duration::from_millis(300), Duration::from_millis(100));
        assert_eq!(none, 0);

        // Un jeu plus court que l'intervalle de sondage compte quand même un temps non nul.
        let (dir2, exe2) = fake_game("tronix-test-court");
        let launcher = std::thread::spawn(move || {
            Command::new(&exe2).args(["-n", "2", "127.0.0.1"]).spawn().unwrap().wait().unwrap();
        });
        let short = watch_dir(&dir2, Duration::from_millis(900), Duration::from_secs(3), Duration::from_millis(1500));
        launcher.join().unwrap();
        assert!(short >= 900, "durée estimée : {short} ms");
        let _ = fs::remove_dir_all(dir2);
        let _ = fs::remove_dir_all(dir);
    }
}

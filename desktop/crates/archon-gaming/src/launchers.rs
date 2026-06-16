//! Detection of installed game launchers.
//!
//! Each launcher is described by the candidate paths (relative to `$HOME`) that
//! indicate it is installed. Detection is just "does any candidate path exist",
//! which keeps it cheap and, importantly, testable: [`detect_in`] takes the
//! root to probe so tests can point it at a fixture tree instead of the real
//! home directory.

use std::path::{Path, PathBuf};

/// A launcher ArchonSync knows how to detect and integrate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launcher {
    Steam,
    Heroic, // Epic / GOG
    Lutris,
    PrismLauncher, // Minecraft
    Bottles,
    Riot, // via Wine/Lutris prefix
}

impl Launcher {
    /// Every launcher we attempt to detect.
    pub const ALL: [Launcher; 6] = [
        Launcher::Steam,
        Launcher::Heroic,
        Launcher::Lutris,
        Launcher::PrismLauncher,
        Launcher::Bottles,
        Launcher::Riot,
    ];

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Launcher::Steam => "Steam",
            Launcher::Heroic => "Heroic (Epic/GOG)",
            Launcher::Lutris => "Lutris",
            Launcher::PrismLauncher => "Prism Launcher",
            Launcher::Bottles => "Bottles",
            Launcher::Riot => "Riot Client",
        }
    }

    /// Paths, relative to `$HOME`, whose existence implies the launcher is
    /// installed. Any one matching is enough.
    fn markers(self) -> &'static [&'static str] {
        match self {
            Launcher::Steam => &[".steam/steam", ".local/share/Steam", ".var/app/com.valvesoftware.Steam"],
            Launcher::Heroic => &[".config/heroic", ".var/app/com.heroicgameslauncher.hgl"],
            Launcher::Lutris => &[".local/share/lutris", ".config/lutris", ".var/app/net.lutris.Lutris"],
            Launcher::PrismLauncher => &[".local/share/PrismLauncher", ".var/app/org.prismlauncher.PrismLauncher"],
            Launcher::Bottles => &[".local/share/bottles", ".var/app/com.usebottles.bottles"],
            Launcher::Riot => &[
                ".local/share/lutris/runners/riotclient",
                "Games/riot-client",
            ],
        }
    }

    /// The command ArchonSync uses to launch it (best-effort; the installer may
    /// wire a flatpak alias instead).
    pub fn launch_command(self) -> &'static str {
        match self {
            Launcher::Steam => "steam",
            Launcher::Heroic => "heroic",
            Launcher::Lutris => "lutris",
            Launcher::PrismLauncher => "prismlauncher",
            Launcher::Bottles => "bottles",
            Launcher::Riot => "lutris lutris:rungame/riot-client",
        }
    }
}

/// A detected launcher and the path that proved it present.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedLauncher {
    pub launcher: Launcher,
    pub marker_path: PathBuf,
}

/// Detect launchers under an explicit home root. Used directly by tests.
pub fn detect_in(home: &Path) -> Vec<DetectedLauncher> {
    let mut found = Vec::new();
    for launcher in Launcher::ALL {
        for marker in launcher.markers() {
            let path = home.join(marker);
            if path.exists() {
                found.push(DetectedLauncher { launcher, marker_path: path });
                break;
            }
        }
    }
    found
}

/// Detect launchers under the real `$HOME`.
pub fn detect() -> Vec<DetectedLauncher> {
    match std::env::var_os("HOME") {
        Some(home) => detect_in(Path::new(&home)),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Unique per call so tests running in parallel never share a directory.
    fn tmp(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!(
            "archon-gaming-test-{}-{label}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn detects_steam_and_prism_from_markers() {
        let home = tmp("detect");
        fs::create_dir_all(home.join(".steam/steam")).unwrap();
        fs::create_dir_all(home.join(".local/share/PrismLauncher")).unwrap();

        let found = detect_in(&home);
        let names: Vec<_> = found.iter().map(|d| d.launcher).collect();
        assert!(names.contains(&Launcher::Steam));
        assert!(names.contains(&Launcher::PrismLauncher));
        assert!(!names.contains(&Launcher::Lutris));

        fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn empty_home_detects_nothing() {
        let home = tmp("empty");
        fs::create_dir_all(&home).unwrap();
        assert!(detect_in(&home).is_empty());
        fs::remove_dir_all(&home).unwrap();
    }
}

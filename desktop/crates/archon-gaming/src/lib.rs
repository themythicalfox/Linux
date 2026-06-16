//! # archon-gaming
//!
//! Gaming integration for the ArchonSync desktop: detect installed launchers,
//! attach per-game performance profiles, and translate a profile into the
//! environment a game should launch with. Game Mode orchestration (CPU
//! governor, feral gamemode handoff, compositor tearing path) is driven from
//! the [`GameMode`] toggle, which the compositor reads.
//!
//! The detection and profile logic is pure and unit-tested; the only
//! side-effecting part is [`GameMode::apply`], which is a thin, well-marked
//! shim the compositor calls.

mod launchers;
mod profile;

pub use launchers::{detect, detect_in, DetectedLauncher, Launcher};
pub use profile::{launch_env, GameProfile, PerfProfile};

/// Tracks whether Game Mode is currently engaged and applies the side effects.
#[derive(Clone, Copy, Debug, Default)]
pub struct GameMode {
    pub active: bool,
}

impl GameMode {
    pub fn new() -> Self {
        GameMode { active: false }
    }

    /// Toggle and return the new state.
    pub fn toggle(&mut self) -> bool {
        self.active = !self.active;
        self.active
    }

    /// Apply the current state to the system.
    ///
    /// This is intentionally a stub in the foundation: the wiring to feral
    /// `gamemoded` over D-Bus and to the CPU governor is environment-specific
    /// and cannot run in headless CI. The compositor calls this; the actual
    /// effects land once the gaming phase is fully built out. Returns the set
    /// of actions that *would* be taken, which keeps it observable and lets the
    /// caller log exactly what changed.
    pub fn apply(&self) -> Vec<&'static str> {
        if self.active {
            vec![
                "request gamemoded enter",
                "set cpu governor=performance",
                "disable compositor effects",
                "enable tearing for fullscreen game",
            ]
        } else {
            vec![
                "request gamemoded leave",
                "restore cpu governor",
                "re-enable compositor effects",
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_state() {
        let mut gm = GameMode::new();
        assert!(!gm.active);
        assert!(gm.toggle());
        assert!(!gm.toggle());
    }

    #[test]
    fn apply_reports_actions_per_state() {
        let mut gm = GameMode::new();
        assert!(gm.apply().iter().any(|a| a.contains("leave")));
        gm.toggle();
        assert!(gm.apply().iter().any(|a| a.contains("enter")));
    }
}

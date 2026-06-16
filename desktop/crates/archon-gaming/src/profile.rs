//! Per-game performance profiles and the environment they translate into.
//!
//! A [`PerfProfile`] is a high-level intent ("squeeze out maximum frames",
//! "balance", "quiet and cool"). [`launch_env`] turns a profile plus a few
//! toggles into the concrete environment variables ArchonSync exports before
//! spawning a game — the same knobs a power user would set by hand for
//! DXVK/VKD3D, MangoHud and the compositor's tearing path.

use serde::{Deserialize, Serialize};

/// The headline performance intent for a game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PerfProfile {
    /// Maximum FPS: tearing allowed, governor to performance, overlay optional.
    Performance,
    /// Sensible defaults for most games.
    #[default]
    Balanced,
    /// Cap frames and keep the GPU cool/quiet.
    Efficiency,
}

/// Tunables attached to a specific game, persisted by the shell.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameProfile {
    pub name: String,
    pub profile: PerfProfile,
    /// Show MangoHud's FPS/perf overlay for this game.
    pub mangohud: bool,
    /// Enable DXVK/VKD3D state-cache pre-warming.
    pub shader_cache: bool,
    /// Optional explicit FPS cap (0 = uncapped / follow display).
    pub fps_cap: u32,
}

impl GameProfile {
    /// A reasonable default profile for a freshly detected game.
    pub fn new(name: impl Into<String>) -> Self {
        GameProfile {
            name: name.into(),
            profile: PerfProfile::Balanced,
            mangohud: false,
            shader_cache: true,
            fps_cap: 0,
        }
    }
}

/// Compute the environment variables to export for a game launch.
///
/// Returned as sorted `(key, value)` pairs so the output is deterministic and
/// easy to assert on. The compositor reads `ARCHON_ALLOW_TEARING` to decide
/// whether to put a fullscreen game on a tearing-capable presentation path.
pub fn launch_env(p: &GameProfile) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = Vec::new();
    let mut push = |k: &str, v: &str| env.push((k.to_string(), v.to_string()));

    // DXVK/VKD3D state caches make shader stutter a one-time cost.
    if p.shader_cache {
        push("DXVK_STATE_CACHE", "1");
        push("VKD3D_SHADER_CACHE", "1");
    } else {
        push("DXVK_STATE_CACHE", "0");
    }

    if p.mangohud {
        push("MANGOHUD", "1");
    }

    match p.profile {
        PerfProfile::Performance => {
            push("ARCHON_ALLOW_TEARING", "1");
            push("ARCHON_GOVERNOR", "performance");
            push("__GL_MaxFramesAllowed", "1"); // low-latency on NVIDIA
        }
        PerfProfile::Balanced => {
            push("ARCHON_ALLOW_TEARING", "0");
            push("ARCHON_GOVERNOR", "schedutil");
        }
        PerfProfile::Efficiency => {
            push("ARCHON_ALLOW_TEARING", "0");
            push("ARCHON_GOVERNOR", "powersave");
        }
    }

    if p.fps_cap > 0 {
        push("ARCHON_FPS_CAP", &p.fps_cap.to_string());
        // DXVK honours its own frame limiter too.
        push("DXVK_FRAME_RATE", &p.fps_cap.to_string());
    }

    env.sort();
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(env: &'a [(String, String)], key: &str) -> Option<&'a str> {
        env.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    #[test]
    fn performance_profile_enables_tearing_and_low_latency() {
        let mut p = GameProfile::new("Valorant");
        p.profile = PerfProfile::Performance;
        let env = launch_env(&p);
        assert_eq!(get(&env, "ARCHON_ALLOW_TEARING"), Some("1"));
        assert_eq!(get(&env, "ARCHON_GOVERNOR"), Some("performance"));
        assert_eq!(get(&env, "__GL_MaxFramesAllowed"), Some("1"));
    }

    #[test]
    fn efficiency_caps_and_powersaves() {
        let mut p = GameProfile::new("Stardew");
        p.profile = PerfProfile::Efficiency;
        p.fps_cap = 60;
        let env = launch_env(&p);
        assert_eq!(get(&env, "ARCHON_ALLOW_TEARING"), Some("0"));
        assert_eq!(get(&env, "ARCHON_GOVERNOR"), Some("powersave"));
        assert_eq!(get(&env, "ARCHON_FPS_CAP"), Some("60"));
        assert_eq!(get(&env, "DXVK_FRAME_RATE"), Some("60"));
    }

    #[test]
    fn shader_cache_toggle_is_reflected() {
        let mut p = GameProfile::new("X");
        p.shader_cache = false;
        assert_eq!(get(&launch_env(&p), "DXVK_STATE_CACHE"), Some("0"));
    }

    #[test]
    fn output_is_sorted_and_deterministic() {
        let p = GameProfile::new("X");
        let a = launch_env(&p);
        let mut sorted = a.clone();
        sorted.sort();
        assert_eq!(a, sorted);
        assert_eq!(a, launch_env(&p));
    }
}

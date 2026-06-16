//! # archon-config
//!
//! Typed configuration for the ArchonSync desktop, persisted as TOML under
//! `~/.config/archonsync/config.toml`. Everything has a sensible default, so a
//! fresh install needs no config file at all; the compositor writes one out on
//! first run so users have something to edit.
//!
//! The types here are pure serde data — no I/O happens unless you call
//! [`Config::load`] / [`Config::save`], which keeps the bulk of the crate
//! testable without touching the filesystem.

mod keybind;

pub use keybind::{Keybind, KeybindParseError, Mods};

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Which screen edge a panel/dock lives on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Left,
    Right,
    Bottom,
    Top,
}

impl Default for Edge {
    fn default() -> Self {
        Edge::Left
    }
}

/// Top-level configuration document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub lock: LockConfig,
    pub dock: DockConfig,
    pub theme: ThemeConfig,
    pub gaming: GamingConfig,
    pub keybinds: Keybinds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    /// Absolute path to the wallpaper image.
    pub wallpaper: String,
    /// Animation speed multiplier (1.0 = designed speed). Lower = snappier.
    pub animation_scale: f32,
    /// Target compositor frame rate cap; 0 means "follow the display".
    pub max_fps: u32,
}

impl Default for General {
    fn default() -> Self {
        General {
            wallpaper: "/usr/share/wallpapers/ArchonSync/contents/images/3840x2160.png".into(),
            animation_scale: 1.0,
            max_fps: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LockConfig {
    /// If true, the edge-swipe gesture alone unlocks (no password required).
    pub swipe_to_unlock: bool,
    /// Fraction of screen width the dot must travel to unlock (`0..=1`).
    pub swipe_threshold: f32,
    /// Background blur radius on the lock screen, in logical pixels.
    pub blur_radius: f32,
    /// Parallax strength of the wallpaper relative to pointer motion.
    pub parallax: f32,
}

impl Default for LockConfig {
    fn default() -> Self {
        LockConfig {
            swipe_to_unlock: true,
            swipe_threshold: 0.75,
            blur_radius: 32.0,
            parallax: 12.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockConfig {
    pub edge: Edge,
    /// Pixels from the edge that trigger reveal.
    pub reveal_zone: u32,
    /// Auto-hide the dock when not hovered.
    pub auto_hide: bool,
    /// Thickness of the dock in logical pixels.
    pub thickness: u32,
}

impl Default for DockConfig {
    fn default() -> Self {
        DockConfig {
            edge: Edge::Left,
            reveal_zone: 6,
            auto_hide: true,
            thickness: 64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    /// One of `signature`, `harmonize`, `contrast`.
    pub accent_mode: String,
    /// Optional manual accent override as `#rrggbb`; empty means "auto".
    pub accent_override: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            accent_mode: "harmonize".into(),
            accent_override: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GamingConfig {
    /// Automatically engage Game Mode when a fullscreen game is focused.
    pub auto_game_mode: bool,
    /// Allow screen tearing for fullscreen games (lower latency).
    pub allow_tearing: bool,
    /// Show the FPS overlay by default.
    pub fps_overlay: bool,
}

impl Default for GamingConfig {
    fn default() -> Self {
        GamingConfig {
            auto_game_mode: true,
            allow_tearing: true,
            fps_overlay: false,
        }
    }
}

/// The default set of keybindings. Stored as a map of action name -> binding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keybinds {
    pub launch_terminal: Keybind,
    pub close_window: Keybind,
    pub toggle_dock: Keybind,
    pub toggle_wheel: Keybind,
    pub lock: Keybind,
    pub toggle_fps_overlay: Keybind,
    pub next_workspace: Keybind,
    pub prev_workspace: Keybind,
    pub toggle_tiling: Keybind,
    pub fullscreen: Keybind,
}

impl Default for Keybinds {
    fn default() -> Self {
        let kb = |s: &str| Keybind::parse(s).expect("builtin keybind is valid");
        Keybinds {
            launch_terminal: kb("Super+Return"),
            close_window: kb("Super+Q"),
            toggle_dock: kb("Super+D"),
            toggle_wheel: kb("Super+Space"),
            lock: kb("Super+L"),
            toggle_fps_overlay: kb("Super+F12"),
            next_workspace: kb("Super+Right"),
            prev_workspace: kb("Super+Left"),
            toggle_tiling: kb("Super+T"),
            fullscreen: kb("Super+F"),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: General::default(),
            lock: LockConfig::default(),
            dock: DockConfig::default(),
            theme: ThemeConfig::default(),
            gaming: GamingConfig::default(),
            keybinds: Keybinds::default(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("config serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

impl Config {
    /// The standard config path: `$XDG_CONFIG_HOME/archonsync/config.toml`,
    /// falling back to `~/.config/...`.
    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"));
        base.join("archonsync").join("config.toml")
    }

    /// Load config from `path`, or return defaults if the file does not exist.
    pub fn load(path: impl AsRef<Path>) -> Result<Config, ConfigError> {
        match std::fs::read_to_string(path.as_ref()) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    /// Serialize to TOML and write to `path`, creating parent dirs as needed.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_roundtrips_through_toml() {
        let cfg = Config::default();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn partial_config_fills_defaults() {
        // Only override one field; everything else should default.
        let text = r#"
            [general]
            max_fps = 144
        "#;
        let cfg: Config = toml::from_str(text).unwrap();
        assert_eq!(cfg.general.max_fps, 144);
        assert_eq!(cfg.dock.edge, Edge::Left);
        assert!(cfg.lock.swipe_to_unlock);
    }

    #[test]
    fn keybinds_serialize_as_strings() {
        let text = toml::to_string_pretty(&Config::default()).unwrap();
        assert!(text.contains("launch_terminal = \"Super+Return\""));
    }

    #[test]
    fn missing_file_yields_defaults() {
        let cfg = Config::load("/nonexistent/archonsync/config.toml").unwrap();
        assert_eq!(cfg, Config::default());
    }
}

//! # archon-comp (library)
//!
//! The window-management core of the ArchonSync compositor. It is intentionally
//! free of any Wayland/Smithay types so the *logic* — tiling, snapping,
//! workspaces, focus, keybinding dispatch, Game Mode triggering — can be unit
//! tested headlessly. The Smithay runtime (in `runtime`, behind the
//! `smithay-backend` feature) owns the surfaces and event loop and drives this
//! core in response to real input and client events.

pub mod keymap;
pub mod layout;
pub mod window;
pub mod workspace;

#[cfg(feature = "smithay-backend")]
pub mod runtime;

use archon_gaming::GameMode;
use archon_theme::Theme;

pub use keymap::{Action, Keymap};
pub use layout::{snap_from_pointer, tile, SnapTarget, TilingMode};
pub use window::{Geometry, Window, WindowId, WindowState};
pub use workspace::Workspaces;

/// A side effect the runtime should carry out after the core processes input.
/// Returning these instead of acting directly keeps the core pure and testable.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Spawn a process (e.g. the terminal).
    Spawn(String),
    /// Ask the focused client to close.
    CloseFocused,
    /// Show/hide the dock or wheel (handled by the shell over IPC).
    ToggleDock,
    ToggleWheel,
    /// Lock the session.
    Lock,
    /// Toggle the FPS overlay.
    ToggleFpsOverlay,
    /// Re-tile the active workspace because layout state changed.
    Relayout,
    /// Game Mode changed; the value is the new active state.
    GameModeChanged(bool),
}

/// The whole compositor's window-management state.
pub struct CompositorCore {
    pub windows: Vec<Window>,
    pub workspaces: Workspaces,
    pub keymap: Keymap,
    pub tiling: bool,
    pub tiling_mode: TilingMode,
    pub game_mode: GameMode,
    pub theme: Theme,
    /// The usable work area (output minus reserved panel space).
    pub work_area: Geometry,
    /// Gap between tiled windows, in logical pixels.
    pub gap: i32,
    /// Master column fraction for [`TilingMode::MasterStack`].
    pub master_ratio: f32,
    next_id: u64,
    auto_game_mode: bool,
}

impl CompositorCore {
    /// Build the core from user config.
    pub fn new(config: &archon_config::Config) -> Self {
        CompositorCore {
            windows: Vec::new(),
            workspaces: Workspaces::new(9),
            keymap: Keymap::from_config(&config.keybinds),
            tiling: false,
            tiling_mode: TilingMode::default(),
            game_mode: GameMode::new(),
            theme: Theme::default(),
            work_area: Geometry::new(0, 0, 1920, 1080),
            gap: 8,
            master_ratio: 0.6,
            next_id: 1,
            auto_game_mode: config.gaming.auto_game_mode,
        }
    }

    /// Register a newly mapped window and return its id. Adds it to the active
    /// workspace, focuses it, and re-tiles if tiling is on.
    pub fn add_window(&mut self, app_id: impl Into<String>, geometry: Geometry) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;
        let mut win = Window::new(id, geometry, app_id);
        if self.tiling {
            win.state = WindowState::Tiled;
        }
        self.windows.push(win);
        self.workspaces.add_window(id);
        if self.tiling {
            self.relayout();
        }
        id
    }

    /// Remove a window (it was unmapped/closed).
    pub fn remove_window(&mut self, id: WindowId) {
        self.windows.retain(|w| w.id != id);
        self.workspaces.remove_window(id);
        if self.tiling {
            self.relayout();
        }
    }

    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }

    pub fn window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    /// Whether the currently focused window is a fullscreen game — the signal
    /// the runtime uses to engage direct scanout and (optionally) Game Mode.
    pub fn focused_is_fullscreen_game(&self) -> bool {
        self.workspaces
            .focused()
            .and_then(|id| self.window(id))
            .map(|w| w.is_game() && w.state == WindowState::Fullscreen)
            .unwrap_or(false)
    }

    /// Recompute geometries for the tiled windows on the active workspace,
    /// writing the results back into each [`Window`].
    pub fn relayout(&mut self) {
        let ids: Vec<WindowId> = self
            .workspaces
            .active()
            .windows
            .iter()
            .copied()
            .filter(|id| {
                self.window(*id)
                    .map(|w| w.state == WindowState::Tiled)
                    .unwrap_or(false)
            })
            .collect();
        let geoms = tile(self.work_area, ids.len(), self.tiling_mode, self.gap, self.master_ratio);
        for (id, g) in ids.into_iter().zip(geoms) {
            if let Some(w) = self.window_mut(id) {
                w.geometry = g;
            }
        }
    }

    /// Snap the focused window to `target`.
    pub fn snap_focused(&mut self, target: SnapTarget) {
        let area = self.work_area;
        if let Some(id) = self.workspaces.focused() {
            if let Some(w) = self.window_mut(id) {
                w.save_floating();
                w.state = WindowState::Floating;
                w.geometry = target.resolve(area);
            }
        }
    }

    /// Toggle fullscreen on the focused window, possibly flipping Game Mode.
    pub fn toggle_fullscreen(&mut self) -> Vec<Effect> {
        let area = self.work_area;
        let Some(id) = self.workspaces.focused() else { return vec![] };
        let mut effects = Vec::new();
        if let Some(w) = self.window_mut(id) {
            if w.state == WindowState::Fullscreen {
                // Restore.
                w.state = WindowState::Floating;
                if let Some(g) = w.saved_floating.take() {
                    w.geometry = g;
                }
            } else {
                w.save_floating();
                w.state = WindowState::Fullscreen;
                w.geometry = area;
            }
        }
        // Auto Game Mode: engage when a fullscreen game takes focus.
        if self.auto_game_mode {
            let want = self.focused_is_fullscreen_game();
            if want != self.game_mode.active {
                self.game_mode.toggle();
                effects.push(Effect::GameModeChanged(self.game_mode.active));
            }
        }
        effects
    }

    /// Translate a pressed key combination into effects, mutating state for the
    /// purely-internal actions (workspace switch, tiling toggle) directly.
    pub fn handle_key(&mut self, mods: archon_config::Mods, key: &str) -> Vec<Effect> {
        let Some(action) = self.keymap.action_for(mods, key).cloned() else {
            return vec![];
        };
        match action {
            Action::LaunchTerminal => vec![Effect::Spawn("foot".into())],
            Action::CloseWindow => vec![Effect::CloseFocused],
            Action::ToggleDock => vec![Effect::ToggleDock],
            Action::ToggleWheel => vec![Effect::ToggleWheel],
            Action::Lock => vec![Effect::Lock],
            Action::ToggleFpsOverlay => vec![Effect::ToggleFpsOverlay],
            Action::NextWorkspace => {
                self.workspaces.cycle(true);
                vec![Effect::Relayout]
            }
            Action::PrevWorkspace => {
                self.workspaces.cycle(false);
                vec![Effect::Relayout]
            }
            Action::SwitchWorkspace(n) => {
                self.workspaces.switch_to(n as usize);
                vec![Effect::Relayout]
            }
            Action::ToggleTiling => {
                self.tiling = !self.tiling;
                let tiling = self.tiling;
                // Flip every window on the active workspace into/out of tiled.
                let ids: Vec<WindowId> = self.workspaces.active().windows.clone();
                for id in ids {
                    if let Some(w) = self.window_mut(id) {
                        w.state = if tiling { WindowState::Tiled } else { WindowState::Floating };
                    }
                }
                if tiling {
                    self.relayout();
                }
                vec![Effect::Relayout]
            }
            Action::Fullscreen => self.toggle_fullscreen(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archon_config::{Config, Mods};

    fn core() -> CompositorCore {
        let mut c = CompositorCore::new(&Config::default());
        c.work_area = Geometry::new(0, 0, 1000, 1000);
        c
    }

    #[test]
    fn adding_windows_focuses_latest() {
        let mut c = core();
        c.add_window("a", Geometry::new(0, 0, 100, 100));
        let b = c.add_window("b", Geometry::new(0, 0, 100, 100));
        assert_eq!(c.workspaces.focused(), Some(b));
    }

    #[test]
    fn toggling_tiling_lays_windows_out() {
        let mut c = core();
        c.add_window("a", Geometry::new(0, 0, 100, 100));
        c.add_window("b", Geometry::new(0, 0, 100, 100));
        let super_t = Mods { logo: true, ..Mods::NONE };
        c.handle_key(super_t, "t");
        assert!(c.tiling);
        // Both windows now tiled and occupying distinct, non-default rects.
        let rects: Vec<Geometry> = c.windows.iter().map(|w| w.geometry).collect();
        assert_ne!(rects[0], rects[1]);
        assert!(c.windows.iter().all(|w| w.state == WindowState::Tiled));
    }

    #[test]
    fn snap_sets_half_geometry() {
        let mut c = core();
        c.add_window("a", Geometry::new(10, 10, 100, 100));
        c.snap_focused(SnapTarget::LeftHalf);
        let g = c.windows[0].geometry;
        assert_eq!(g, Geometry::new(0, 0, 500, 1000));
        assert!(c.windows[0].saved_floating.is_some());
    }

    #[test]
    fn fullscreen_game_engages_game_mode() {
        let mut c = core();
        let id = c.add_window("steam_app_730", Geometry::new(0, 0, 100, 100));
        let effects = c.toggle_fullscreen();
        assert_eq!(c.window(id).unwrap().state, WindowState::Fullscreen);
        assert!(c.game_mode.active);
        assert!(effects.contains(&Effect::GameModeChanged(true)));
        // Toggling back off restores and disengages.
        let effects = c.toggle_fullscreen();
        assert!(!c.game_mode.active);
        assert!(effects.contains(&Effect::GameModeChanged(false)));
    }

    #[test]
    fn super_return_spawns_terminal() {
        let mut c = core();
        let just_super = Mods { logo: true, ..Mods::NONE };
        assert_eq!(c.handle_key(just_super, "return"), vec![Effect::Spawn("foot".into())]);
    }

    #[test]
    fn workspace_switch_changes_active() {
        let mut c = core();
        let just_super = Mods { logo: true, ..Mods::NONE };
        c.handle_key(just_super, "3");
        assert_eq!(c.workspaces.active_index(), 2);
    }
}

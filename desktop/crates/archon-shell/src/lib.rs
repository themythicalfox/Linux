//! # archon-shell
//!
//! The ArchonSync shell: the edge-activated [`Dock`] and the radial
//! [`ControlWheel`]. Both are pure state + geometry here (and unit tested); the
//! `archon-shell` binary hosts them on `wlr-layer-shell` surfaces, renders them
//! with `archon-ui`, and dispatches their actions to the compositor over the
//! `archon-ipc` socket.

mod dock;
mod wheel;

pub use dock::Dock;
pub use wheel::{ControlWheel, WheelAction, WheelItem};

use archon_config::Config;
use archon_theme::Theme;
use archon_ui::scene::{DrawList, Vec2};

/// Owns the shell's surfaces' state and the shared theme.
pub struct Shell {
    pub dock: Dock,
    pub wheel: ControlWheel,
    pub theme: Theme,
    /// Whether the wheel overlay is currently visible.
    pub wheel_visible: bool,
}

impl Shell {
    pub fn new(config: &Config, theme: Theme) -> Self {
        Shell {
            dock: Dock::new(config.dock.clone()),
            wheel: ControlWheel::new(ControlWheel::default_items(), 140.0),
            theme,
            wheel_visible: false,
        }
    }

    /// Advance all shell animations by `dt`.
    pub fn tick(&mut self, dt: f32) {
        self.dock.tick(dt);
        if self.wheel_visible {
            self.wheel.tick(dt);
        }
    }

    /// Toggle the radial wheel overlay.
    pub fn toggle_wheel(&mut self) {
        self.wheel_visible = !self.wheel_visible;
    }

    /// Build the wheel's draw list centered on `center` (only when visible).
    pub fn wheel_scene(&self, center: Vec2) -> Option<DrawList> {
        self.wheel_visible.then(|| self.wheel.build_scene(center, &self.theme))
    }
}

/// Translate a wheel action into the IPC request that realises it, when there is
/// a direct mapping. Actions handled entirely inside the shell (opening the
/// settings window, the notification center) return `None`.
pub fn wheel_action_to_request(action: &WheelAction) -> Option<archon_ipc::Request> {
    use archon_ipc::Request;
    match action {
        WheelAction::Lock => Some(Request::Lock),
        WheelAction::GameMode => Some(Request::SetGameMode(true)),
        WheelAction::Notifications => Some(Request::ToggleDock),
        WheelAction::Power
        | WheelAction::Settings
        | WheelAction::Workspaces
        | WheelAction::Widgets
        | WheelAction::Launch(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wheel_scene_only_when_visible() {
        let mut s = Shell::new(&Config::default(), Theme::default());
        assert!(s.wheel_scene(Vec2::new(0.0, 0.0)).is_none());
        s.toggle_wheel();
        assert!(s.wheel_scene(Vec2::new(0.0, 0.0)).is_some());
    }

    #[test]
    fn lock_action_maps_to_lock_request() {
        assert_eq!(
            wheel_action_to_request(&WheelAction::Lock),
            Some(archon_ipc::Request::Lock)
        );
        assert_eq!(wheel_action_to_request(&WheelAction::Settings), None);
    }
}

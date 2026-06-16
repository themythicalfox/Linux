//! The compositor's view of a window, independent of Wayland.
//!
//! The Smithay runtime maps each `xdg-toplevel` to one [`Window`] here; all the
//! window-management *logic* (focus, tiling, snapping, workspace assignment)
//! operates on these plain structs, which is what makes it unit-testable
//! without a display server.

/// A stable identifier for a managed window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

/// An integer rectangle in output-local logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Geometry {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Geometry { x, y, w, h }
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }

    /// Shrink by `gap` on every side (used to inset tiled windows).
    pub fn inset(self, gap: i32) -> Geometry {
        Geometry {
            x: self.x + gap,
            y: self.y + gap,
            w: (self.w - 2 * gap).max(1),
            h: (self.h - 2 * gap).max(1),
        }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// How a window is currently arranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowState {
    /// Free-floating; the user positions it.
    #[default]
    Floating,
    /// Managed by the active tiling layout.
    Tiled,
    /// Covers the whole output (no decorations, candidate for direct scanout).
    Fullscreen,
    /// Fills the work area but keeps the panel/dock reachable.
    Maximized,
}

/// A managed window.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub id: WindowId,
    pub geometry: Geometry,
    pub state: WindowState,
    /// Wayland `app_id` (e.g. `"steam"`, `"org.kde.konsole"`), used for game
    /// detection and per-app rules.
    pub app_id: String,
    pub title: String,
    /// Geometry to restore to when leaving fullscreen/maximized/tiled.
    pub saved_floating: Option<Geometry>,
}

impl Window {
    pub fn new(id: WindowId, geometry: Geometry, app_id: impl Into<String>) -> Self {
        Window {
            id,
            geometry,
            state: WindowState::Floating,
            app_id: app_id.into(),
            title: String::new(),
            saved_floating: None,
        }
    }

    /// Heuristic: should this window be treated as a game for Game Mode? Steam's
    /// app-id pattern (`steam_app_<id>`) and known launchers count.
    pub fn is_game(&self) -> bool {
        let a = self.app_id.to_ascii_lowercase();
        a.starts_with("steam_app_")
            || a.contains("valorant")
            || a.contains("riotclient")
            || a == "gamescope"
    }

    /// Remember the current geometry as the floating one, if not already saved.
    pub fn save_floating(&mut self) {
        if self.saved_floating.is_none() && self.state == WindowState::Floating {
            self.saved_floating = Some(self.geometry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inset_shrinks_symmetrically() {
        let g = Geometry::new(0, 0, 100, 100).inset(10);
        assert_eq!(g, Geometry::new(10, 10, 80, 80));
    }

    #[test]
    fn inset_never_collapses_below_one() {
        let g = Geometry::new(0, 0, 4, 4).inset(10);
        assert!(g.w >= 1 && g.h >= 1);
    }

    #[test]
    fn game_detection_matches_steam_pattern() {
        assert!(Window::new(WindowId(1), Geometry::new(0, 0, 1, 1), "steam_app_730").is_game());
        assert!(!Window::new(WindowId(2), Geometry::new(0, 0, 1, 1), "org.kde.konsole").is_game());
    }

    #[test]
    fn contains_is_half_open() {
        let g = Geometry::new(0, 0, 10, 10);
        assert!(g.contains(0, 0));
        assert!(!g.contains(10, 10));
    }
}

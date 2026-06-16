//! Workspaces: independent groups of windows the user switches between.
//!
//! A [`Workspaces`] owns a fixed set of [`Workspace`]s and tracks which one is
//! active and which window has focus. It is pure bookkeeping over [`WindowId`]s
//! — the Smithay runtime maps the focus/visibility decisions onto real
//! surfaces. Keeping it separate makes the (surprisingly fiddly) focus and
//! move-between-workspace rules testable.

use crate::window::WindowId;

/// A single workspace: an ordered list of windows plus the focused one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Workspace {
    pub windows: Vec<WindowId>,
    pub focused: Option<WindowId>,
}

impl Workspace {
    fn add(&mut self, id: WindowId) {
        if !self.windows.contains(&id) {
            self.windows.push(id);
        }
        self.focused = Some(id);
    }

    fn remove(&mut self, id: WindowId) {
        self.windows.retain(|w| *w != id);
        if self.focused == Some(id) {
            // Focus falls back to the most-recently-added remaining window.
            self.focused = self.windows.last().copied();
        }
    }
}

/// The full set of workspaces for an output (or the session).
#[derive(Clone, Debug, PartialEq)]
pub struct Workspaces {
    spaces: Vec<Workspace>,
    active: usize,
}

impl Workspaces {
    /// Create `count` empty workspaces (at least one), with the first active.
    pub fn new(count: usize) -> Self {
        Workspaces {
            spaces: vec![Workspace::default(); count.max(1)],
            active: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.spaces.len()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &Workspace {
        &self.spaces[self.active]
    }

    pub fn focused(&self) -> Option<WindowId> {
        self.spaces[self.active].focused
    }

    /// Add a window to the active workspace and focus it.
    pub fn add_window(&mut self, id: WindowId) {
        self.spaces[self.active].add(id);
    }

    /// Remove a window from whichever workspace holds it.
    pub fn remove_window(&mut self, id: WindowId) {
        for ws in &mut self.spaces {
            ws.remove(id);
        }
    }

    /// Which workspace currently holds `id`, if any.
    pub fn workspace_of(&self, id: WindowId) -> Option<usize> {
        self.spaces.iter().position(|ws| ws.windows.contains(&id))
    }

    /// Switch to workspace `index`. No-op if out of range.
    pub fn switch_to(&mut self, index: usize) -> bool {
        if index < self.spaces.len() {
            self.active = index;
            true
        } else {
            false
        }
    }

    /// Switch to the next/previous workspace, wrapping around.
    pub fn cycle(&mut self, forward: bool) {
        let n = self.spaces.len();
        self.active = if forward {
            (self.active + 1) % n
        } else {
            (self.active + n - 1) % n
        };
    }

    /// Move `id` to workspace `target`, keeping it focused there. The view
    /// follows the window is a UI decision the caller makes; this only moves the
    /// bookkeeping. Returns false if `target` is out of range or `id` is unknown.
    pub fn move_window_to(&mut self, id: WindowId, target: usize) -> bool {
        if target >= self.spaces.len() || self.workspace_of(id).is_none() {
            return false;
        }
        for ws in &mut self.spaces {
            ws.remove(id);
        }
        self.spaces[target].add(id);
        true
    }

    /// Cycle focus within the active workspace.
    pub fn focus_next(&mut self) {
        let ws = &mut self.spaces[self.active];
        if ws.windows.is_empty() {
            return;
        }
        let cur = ws.focused.and_then(|f| ws.windows.iter().position(|w| *w == f)).unwrap_or(0);
        let next = (cur + 1) % ws.windows.len();
        ws.focused = Some(ws.windows[next]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> WindowId {
        WindowId(n)
    }

    #[test]
    fn add_focuses_new_window() {
        let mut w = Workspaces::new(4);
        w.add_window(id(1));
        w.add_window(id(2));
        assert_eq!(w.focused(), Some(id(2)));
        assert_eq!(w.active().windows.len(), 2);
    }

    #[test]
    fn switch_changes_active_and_isolates_windows() {
        let mut w = Workspaces::new(3);
        w.add_window(id(1));
        w.switch_to(1);
        assert!(w.active().windows.is_empty());
        w.add_window(id(2));
        assert_eq!(w.active().windows, vec![id(2)]);
        w.switch_to(0);
        assert_eq!(w.active().windows, vec![id(1)]);
    }

    #[test]
    fn remove_refocuses_remaining() {
        let mut w = Workspaces::new(1);
        w.add_window(id(1));
        w.add_window(id(2));
        w.remove_window(id(2));
        assert_eq!(w.focused(), Some(id(1)));
    }

    #[test]
    fn move_window_relocates_and_focuses() {
        let mut w = Workspaces::new(3);
        w.add_window(id(1));
        assert!(w.move_window_to(id(1), 2));
        assert_eq!(w.workspace_of(id(1)), Some(2));
        assert!(w.active().windows.is_empty()); // gone from workspace 0
        assert_eq!(w.spaces[2].focused, Some(id(1)));
    }

    #[test]
    fn move_to_bad_target_fails() {
        let mut w = Workspaces::new(2);
        w.add_window(id(1));
        assert!(!w.move_window_to(id(1), 9));
        assert!(!w.move_window_to(id(42), 1)); // unknown window
    }

    #[test]
    fn cycle_wraps_both_ways() {
        let mut w = Workspaces::new(3);
        w.cycle(false); // 0 -> 2
        assert_eq!(w.active_index(), 2);
        w.cycle(true); // 2 -> 0
        assert_eq!(w.active_index(), 0);
    }

    #[test]
    fn focus_next_cycles_within_workspace() {
        let mut w = Workspaces::new(1);
        w.add_window(id(1));
        w.add_window(id(2));
        w.add_window(id(3)); // focused = 3
        w.focus_next(); // 3 -> wraps to 1
        assert_eq!(w.focused(), Some(id(1)));
        w.focus_next();
        assert_eq!(w.focused(), Some(id(2)));
    }
}

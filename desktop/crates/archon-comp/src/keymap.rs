//! Mapping key events to compositor actions.
//!
//! The compositor receives raw modifier state + a key name from libinput/xkb.
//! [`Keymap`] turns the user's [`archon_config::Keybinds`] into a lookup table
//! from a pressed combination to an [`Action`] the runtime executes. Building
//! the table once and matching against it keeps the input hot-path allocation
//! free.

use archon_config::{Keybind, Keybinds, Mods};

/// Everything the compositor can do in response to a keybinding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    LaunchTerminal,
    CloseWindow,
    ToggleDock,
    ToggleWheel,
    Lock,
    ToggleFpsOverlay,
    NextWorkspace,
    PrevWorkspace,
    ToggleTiling,
    Fullscreen,
    /// Switch directly to a workspace by index (0-based).
    SwitchWorkspace(u32),
}

/// A resolved key combination paired with the action it triggers.
pub struct Keymap {
    bindings: Vec<(Keybind, Action)>,
}

impl Keymap {
    /// Build the map from the user's configured bindings, plus the implicit
    /// `Super+1..=9` workspace-switch bindings that aren't worth spelling out in
    /// config.
    pub fn from_config(kb: &Keybinds) -> Self {
        let mut bindings = vec![
            (kb.launch_terminal.clone(), Action::LaunchTerminal),
            (kb.close_window.clone(), Action::CloseWindow),
            (kb.toggle_dock.clone(), Action::ToggleDock),
            (kb.toggle_wheel.clone(), Action::ToggleWheel),
            (kb.lock.clone(), Action::Lock),
            (kb.toggle_fps_overlay.clone(), Action::ToggleFpsOverlay),
            (kb.next_workspace.clone(), Action::NextWorkspace),
            (kb.prev_workspace.clone(), Action::PrevWorkspace),
            (kb.toggle_tiling.clone(), Action::ToggleTiling),
            (kb.fullscreen.clone(), Action::Fullscreen),
        ];
        // Super+1..Super+9 jump to workspaces 0..8.
        for n in 1..=9u32 {
            let key = n.to_string();
            bindings.push((
                Keybind { mods: Mods { logo: true, ..Mods::NONE }, key },
                Action::SwitchWorkspace(n - 1),
            ));
        }
        Keymap { bindings }
    }

    /// Look up the action for a pressed combination, if any. `key` is the
    /// xkb-style key name, lowercased by the caller.
    pub fn action_for(&self, mods: Mods, key: &str) -> Option<&Action> {
        self.bindings
            .iter()
            .find(|(kb, _)| kb.mods == mods && kb.key == key)
            .map(|(_, a)| a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_resolve() {
        let km = Keymap::from_config(&Keybinds::default());
        let supershift = Mods { logo: true, shift: true, ..Mods::NONE };
        let just_super = Mods { logo: true, ..Mods::NONE };

        assert_eq!(km.action_for(just_super, "return"), Some(&Action::LaunchTerminal));
        assert_eq!(km.action_for(just_super, "q"), Some(&Action::CloseWindow));
        assert_eq!(km.action_for(just_super, "l"), Some(&Action::Lock));
        // A binding that needs shift must not fire without it.
        assert_eq!(km.action_for(supershift, "return"), None);
    }

    #[test]
    fn numeric_workspace_bindings_exist() {
        let km = Keymap::from_config(&Keybinds::default());
        let just_super = Mods { logo: true, ..Mods::NONE };
        assert_eq!(km.action_for(just_super, "1"), Some(&Action::SwitchWorkspace(0)));
        assert_eq!(km.action_for(just_super, "9"), Some(&Action::SwitchWorkspace(8)));
    }

    #[test]
    fn unknown_combo_returns_none() {
        let km = Keymap::from_config(&Keybinds::default());
        assert_eq!(km.action_for(Mods::NONE, "z"), None);
    }
}

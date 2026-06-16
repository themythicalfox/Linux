//! Keybinding model and parser.
//!
//! Bindings are written in config as strings like `"Super+Return"` or
//! `"Super+Shift+Q"`. We parse them into a normalized [`Keybind`] so the
//! compositor can match them against key events without re-parsing strings on
//! the hot path. The grammar is deliberately tiny: a `+`-separated list of
//! modifiers followed by a single key name.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Modifier bitflags. Mirrors the modifiers the compositor cares about.
#[derive(Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Mods {
    pub logo: bool,  // "Super" / Windows key
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub const NONE: Mods = Mods { logo: false, ctrl: false, alt: false, shift: false };
}

/// A fully-parsed key combination.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Keybind {
    pub mods: Mods,
    /// Lowercased key name, e.g. `"return"`, `"q"`, `"f1"`, `"space"`.
    pub key: String,
}

impl Keybind {
    /// Parse a binding string such as `"Super+Shift+Q"`. Modifier and key names
    /// are case-insensitive; common aliases (`Win`, `Cmd`, `Meta`, `Control`)
    /// are accepted.
    pub fn parse(s: &str) -> Result<Keybind, KeybindParseError> {
        let mut mods = Mods::NONE;
        let mut key: Option<String> = None;
        for token in s.split('+').map(str::trim).filter(|t| !t.is_empty()) {
            match token.to_ascii_lowercase().as_str() {
                "super" | "win" | "logo" | "meta" | "cmd" => mods.logo = true,
                "ctrl" | "control" => mods.ctrl = true,
                "alt" | "mod1" | "option" => mods.alt = true,
                "shift" => mods.shift = true,
                other => {
                    if key.is_some() {
                        return Err(KeybindParseError::MultipleKeys(s.to_string()));
                    }
                    key = Some(other.to_string());
                }
            }
        }
        match key {
            Some(key) => Ok(Keybind { mods, key }),
            None => Err(KeybindParseError::NoKey(s.to_string())),
        }
    }

    /// Render back to the canonical `"Super+Shift+Q"` form.
    pub fn to_string_canonical(&self) -> String {
        let mut parts = Vec::new();
        if self.mods.logo {
            parts.push("Super");
        }
        if self.mods.ctrl {
            parts.push("Ctrl");
        }
        if self.mods.alt {
            parts.push("Alt");
        }
        if self.mods.shift {
            parts.push("Shift");
        }
        let key_title = title_case(&self.key);
        parts.push(&key_title);
        parts.join("+")
    }
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum KeybindParseError {
    #[error("keybinding `{0}` has no non-modifier key")]
    NoKey(String),
    #[error("keybinding `{0}` has more than one non-modifier key")]
    MultipleKeys(String),
}

// Serialize as the canonical string so config files stay human-friendly.
impl Serialize for Keybind {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_string_canonical())
    }
}

impl<'de> Deserialize<'de> for Keybind {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        Keybind::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Debug for Keybind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Keybind({})", self.to_string_canonical())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifiers_and_key() {
        let kb = Keybind::parse("Super+Shift+Q").unwrap();
        assert!(kb.mods.logo && kb.mods.shift);
        assert!(!kb.mods.ctrl && !kb.mods.alt);
        assert_eq!(kb.key, "q");
    }

    #[test]
    fn accepts_aliases() {
        let kb = Keybind::parse("win + control + return").unwrap();
        assert!(kb.mods.logo && kb.mods.ctrl);
        assert_eq!(kb.key, "return");
    }

    #[test]
    fn canonical_roundtrip() {
        let kb = Keybind::parse("alt+f4").unwrap();
        assert_eq!(kb.to_string_canonical(), "Alt+F4");
        assert_eq!(Keybind::parse(&kb.to_string_canonical()).unwrap(), kb);
    }

    #[test]
    fn rejects_missing_or_extra_keys() {
        assert_eq!(Keybind::parse("Super+Shift"), Err(KeybindParseError::NoKey("Super+Shift".into())));
        assert!(matches!(
            Keybind::parse("a+b"),
            Err(KeybindParseError::MultipleKeys(_))
        ));
    }
}

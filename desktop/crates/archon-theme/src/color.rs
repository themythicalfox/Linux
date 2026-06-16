//! A small, dependency-free color type with the operations the theming engine
//! needs: hex parsing, perceptual luminance, WCAG contrast, and blending.
//!
//! Colors are stored as straight (non-premultiplied) 8-bit sRGB plus alpha.
//! Luminance and contrast follow the WCAG 2.1 definitions so the derived
//! themes stay readable regardless of what wallpaper the user picks.

use serde::{Deserialize, Serialize};

/// An sRGB color with 8-bit channels and 8-bit alpha.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Parse `#rrggbb` or `#rrggbbaa` (the leading `#` is optional).
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        let byte = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
        match s.len() {
            6 => Some(Self::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Self::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    /// Format as `#rrggbb` (alpha dropped).
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Linear-light value of one channel (sRGB inverse companding).
    fn linearize(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// WCAG relative luminance in `0.0..=1.0`.
    pub fn luminance(self) -> f32 {
        0.2126 * Self::linearize(self.r)
            + 0.7152 * Self::linearize(self.g)
            + 0.0722 * Self::linearize(self.b)
    }

    /// True for colors a human reads as "dark" (luminance below mid-grey).
    pub fn is_dark(self) -> bool {
        self.luminance() < 0.18
    }

    /// WCAG contrast ratio between two colors, always `>= 1.0`.
    pub fn contrast(self, other: Color) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        let (hi, lo) = if a >= b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// Linear interpolation toward `other`. `t` is clamped to `0..=1`.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
        Color::rgba(
            lerp(self.r, other.r),
            lerp(self.g, other.g),
            lerp(self.b, other.b),
            lerp(self.a, other.a),
        )
    }

    /// Move a color toward white by `amount` (`0..=1`).
    pub fn lighten(self, amount: f32) -> Color {
        self.mix(Color::rgb(255, 255, 255), amount)
    }

    /// Move a color toward black by `amount` (`0..=1`).
    pub fn darken(self, amount: f32) -> Color {
        self.mix(Color::rgb(0, 0, 0), amount)
    }

    /// Same color at a new alpha.
    pub fn with_alpha(self, a: u8) -> Color {
        Color { a, ..self }
    }

    /// Straight sRGB as normalized `[r, g, b, a]` for GPU upload.
    pub fn to_array(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// HSL hue in degrees (`0..360`), saturation and lightness in `0..=1`.
    pub fn to_hsl(self) -> (f32, f32, f32) {
        let (r, g, b) = (
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        );
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        if d.abs() < f32::EPSILON {
            return (0.0, 0.0, l);
        }
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let h = if max == r {
            60.0 * (((g - b) / d) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        (if h < 0.0 { h + 360.0 } else { h }, s, l)
    }

    /// Build a color from HSL (hue degrees, sat/light `0..=1`).
    pub fn from_hsl(h: f32, s: f32, l: f32) -> Color {
        let h = h.rem_euclid(360.0);
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = match h as u32 / 60 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let to8 = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
        Color::rgb(to8(r), to8(g), to8(b))
    }

    /// Rotate the hue by `degrees`, keeping saturation and lightness.
    pub fn rotate_hue(self, degrees: f32) -> Color {
        let (h, s, l) = self.to_hsl();
        Color::from_hsl(h + degrees, s, l)
    }
}

impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.a == 255 {
            write!(f, "Color({})", self.to_hex())
        } else {
            write!(f, "Color({}@{})", self.to_hex(), self.a)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = Color::from_hex("#ff7a1a").unwrap();
        assert_eq!((c.r, c.g, c.b), (255, 122, 26));
        assert_eq!(c.to_hex(), "#ff7a1a");
        assert_eq!(Color::from_hex("ff7a1a"), Some(c));
    }

    #[test]
    fn hex_with_alpha() {
        let c = Color::from_hex("#0c0c0f80").unwrap();
        assert_eq!(c.a, 0x80);
    }

    #[test]
    fn bad_hex_rejected() {
        assert_eq!(Color::from_hex("nope"), None);
        assert_eq!(Color::from_hex("#fff"), None);
    }

    #[test]
    fn contrast_is_symmetric_and_bounded() {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);
        assert!((black.contrast(white) - 21.0).abs() < 0.01);
        assert!((white.contrast(black) - 21.0).abs() < 0.01);
        assert!((black.contrast(black) - 1.0).abs() < 0.001);
    }

    #[test]
    fn dark_surfaces_are_dark() {
        assert!(Color::from_hex("#0c0c0f").unwrap().is_dark());
        assert!(!Color::from_hex("#e8e8ec").unwrap().is_dark());
    }

    #[test]
    fn hsl_roundtrip_is_stable() {
        let c = Color::rgb(255, 122, 26);
        let (h, s, l) = c.to_hsl();
        let back = Color::from_hsl(h, s, l);
        // Allow a tiny rounding tolerance per channel.
        assert!((back.r as i32 - 255).abs() <= 2);
        assert!((back.g as i32 - 122).abs() <= 2);
        assert!((back.b as i32 - 26).abs() <= 2);
    }
}

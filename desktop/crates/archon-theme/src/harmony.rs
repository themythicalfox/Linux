//! Turning an extracted [`Palette`] into a usable accent color.
//!
//! The wallpaper gives us raw dominant colors; the theme needs a single accent
//! that (a) is vivid enough to read as an accent, (b) has enough contrast
//! against a near-black UI, and (c) can be chosen to either *harmonize* with the
//! wallpaper or *artistically contrast* it (the "AI Enhance" path). Everything
//! here is pure math so it stays deterministic and testable.

use crate::color::Color;
use crate::palette::Palette;

/// How the accent relates to the wallpaper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccentMode {
    /// Pull the accent straight from the wallpaper's most vivid color.
    Harmonize,
    /// Rotate to the complementary hue for a bold, contrasting accent.
    /// This is what the "AI Enhance" button uses.
    Contrast,
    /// Keep ArchonSync's signature orange regardless of wallpaper.
    Signature,
}

/// The brand orange, used as the signature accent and as a fallback whenever a
/// wallpaper is too washed-out to yield a usable accent of its own.
pub const SIGNATURE_ORANGE: Color = Color::rgb(0xff, 0x7a, 0x1a);

/// Nudge a candidate accent until it is vivid and bright enough to sit on a
/// near-black surface: raise saturation to a floor, and lightness into a band
/// that reads as a glowing accent rather than a muddy mid-tone.
fn make_accentable(c: Color) -> Color {
    let (h, s, l) = c.to_hsl();
    let s = s.max(0.55);
    let l = l.clamp(0.45, 0.62);
    Color::from_hsl(h, s, l)
}

/// Pick the accent color for a palette under the given mode.
pub fn accent_for(palette: &Palette, mode: AccentMode) -> Color {
    if palette.swatches.is_empty() {
        return SIGNATURE_ORANGE;
    }
    match mode {
        AccentMode::Signature => SIGNATURE_ORANGE,
        AccentMode::Harmonize => {
            let vivid = palette.most_vivid();
            // If the wallpaper is essentially greyscale, there is nothing to
            // harmonize with — keep the brand orange.
            if is_greyish(vivid) {
                SIGNATURE_ORANGE
            } else {
                make_accentable(vivid)
            }
        }
        AccentMode::Contrast => {
            let base = palette.most_vivid();
            if is_greyish(base) {
                SIGNATURE_ORANGE
            } else {
                make_accentable(base.rotate_hue(180.0))
            }
        }
    }
}

/// True for colors with so little chroma that hue is meaningless.
fn is_greyish(c: Color) -> bool {
    let max = c.r.max(c.g).max(c.b) as f32;
    let min = c.r.min(c.g).min(c.b) as f32;
    (max - min) / 255.0 < 0.10
}

/// Ensure `fg` has at least `ratio` WCAG contrast against `bg`, lightening or
/// darkening it as needed. Used to keep text and accents legible no matter what
/// the wallpaper extraction produced.
pub fn ensure_contrast(fg: Color, bg: Color, ratio: f32) -> Color {
    if fg.contrast(bg) >= ratio {
        return fg;
    }
    let toward_light = bg.is_dark();
    let mut out = fg;
    for step in 1..=20 {
        let amount = step as f32 / 20.0;
        out = if toward_light {
            fg.lighten(amount)
        } else {
            fg.darken(amount)
        };
        if out.contrast(bg) >= ratio {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::{Palette, Swatch};

    fn palette(colors: &[(Color, f32)]) -> Palette {
        Palette {
            swatches: colors
                .iter()
                .map(|&(color, weight)| Swatch { color, weight })
                .collect(),
        }
    }

    #[test]
    fn greyscale_wallpaper_keeps_signature() {
        let p = palette(&[(Color::rgb(20, 20, 20), 0.6), (Color::rgb(200, 200, 200), 0.4)]);
        assert_eq!(accent_for(&p, AccentMode::Harmonize), SIGNATURE_ORANGE);
    }

    #[test]
    fn contrast_mode_rotates_hue() {
        let p = palette(&[(Color::rgb(40, 120, 220), 1.0)]); // blue
        let accent = accent_for(&p, AccentMode::Contrast);
        let (h, _, _) = accent.to_hsl();
        // Complement of blue (~210) lands in the orange/yellow arc.
        assert!((20.0..90.0).contains(&h), "hue was {h}");
    }

    #[test]
    fn ensure_contrast_lifts_dim_text() {
        let bg = Color::rgb(12, 12, 15);
        let dim = Color::rgb(40, 40, 45);
        let fixed = ensure_contrast(dim, bg, 4.5);
        assert!(fixed.contrast(bg) >= 4.5);
    }

    #[test]
    fn accent_is_bright_enough_on_black() {
        let p = palette(&[(Color::rgb(80, 30, 30), 1.0)]);
        let accent = accent_for(&p, AccentMode::Harmonize);
        assert!(accent.contrast(Color::rgb(12, 12, 15)) >= 3.0);
    }
}

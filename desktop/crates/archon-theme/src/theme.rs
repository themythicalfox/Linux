//! The [`Theme`] every ArchonSync surface reads from.
//!
//! A theme is a flat set of resolved colors plus a few numeric knobs (blur,
//! corner radius, border opacity). The compositor, lock screen, dock, wheel and
//! widgets all consume the same struct, so a single wallpaper change re-themes
//! the entire desktop coherently.
//!
//! The default theme is seeded from the existing ArchonSync KDE color scheme
//! (`ArchonSyncDark.colors`) so the Wayland desktop matches the rest of the OS
//! out of the box.

use crate::color::Color;
use crate::harmony::{accent_for, ensure_contrast, AccentMode};
use crate::palette::Palette;
use serde::{Deserialize, Serialize};

/// How a widget blends with the wallpaper behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceMode {
    /// Solid, fully opaque panel.
    Opaque,
    /// Translucent frosted glass (background blur + low-alpha fill).
    #[default]
    Glass,
    /// Tinted to match the wallpaper so the widget reads as part of it.
    WallpaperIntegrated,
}

/// A fully resolved color theme.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    /// Primary accent — focus rings, the lock swipe glow, active highlights.
    pub accent: Color,
    /// Slightly brighter accent for hover/pressed states.
    pub accent_hover: Color,

    /// Four surface tiers from deepest background to raised controls.
    pub bg_deep: Color,
    pub bg: Color,
    pub bg_raised: Color,
    pub bg_control: Color,

    /// Primary and dimmed text.
    pub text: Color,
    pub text_dim: Color,

    /// Hairline border / divider color (use with `border_opacity`).
    pub border: Color,

    /// Status colors.
    pub positive: Color,
    pub negative: Color,
    pub warning: Color,

    /// Background blur radius in logical pixels for glass surfaces.
    pub blur_radius: f32,
    /// Default corner radius in logical pixels.
    pub corner_radius: f32,
    /// Border alpha (`0..=1`) applied on top of `border`.
    pub border_opacity: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::archonsync_dark()
    }
}

impl Theme {
    /// The signature ArchonSync dark theme, matching `ArchonSyncDark.colors`.
    pub fn archonsync_dark() -> Self {
        Theme {
            accent: Color::rgb(0xff, 0x7a, 0x1a),
            accent_hover: Color::rgb(0xff, 0x8f, 0x40),
            bg_deep: Color::rgb(0x0c, 0x0c, 0x0f),
            bg: Color::rgb(0x12, 0x12, 0x16),
            bg_raised: Color::rgb(0x1a, 0x1a, 0x1f),
            bg_control: Color::rgb(0x24, 0x24, 0x2b),
            text: Color::rgb(0xe8, 0xe8, 0xec),
            text_dim: Color::rgb(0x78, 0x78, 0x82),
            border: Color::rgb(0xff, 0x7a, 0x1a),
            positive: Color::rgb(0x27, 0xae, 0x60),
            negative: Color::rgb(0xda, 0x44, 0x53),
            warning: Color::rgb(0xf6, 0x74, 0x00),
            blur_radius: 24.0,
            corner_radius: 14.0,
            border_opacity: 0.18,
        }
    }

    /// Derive a theme from a wallpaper palette.
    ///
    /// The surface tiers stay near-black (this is a dark desktop by design) but
    /// pick up a faint tint from the wallpaper's dominant color so the whole UI
    /// feels grounded in the image. The accent is chosen per `mode`, then
    /// contrast-corrected so it always reads on the dark background.
    pub fn from_palette(palette: &Palette, mode: AccentMode) -> Self {
        let mut t = Theme::archonsync_dark();
        if palette.swatches.is_empty() {
            return t;
        }

        let accent = accent_for(palette, mode);
        t.accent = ensure_contrast(accent, t.bg, 3.5);
        t.accent_hover = t.accent.lighten(0.18);
        t.border = t.accent;

        // Tint the near-black surfaces toward the wallpaper's dominant hue, but
        // only a whisper of it (8%) so they stay dark and the accent still pops.
        let dom = palette.dominant();
        let tint = |base: Color| base.mix(dom, 0.08).darken_to_ceiling(0.16);
        t.bg_deep = tint(t.bg_deep);
        t.bg = tint(t.bg);
        t.bg_raised = tint(t.bg_raised);
        t.bg_control = tint(t.bg_control);

        // Keep text readable against the freshly tinted background.
        t.text = ensure_contrast(t.text, t.bg, 7.0);
        t.text_dim = ensure_contrast(t.text_dim, t.bg, 3.0);
        t
    }

    /// Resolve the fill color for a surface in the given mode. Glass and
    /// wallpaper-integrated surfaces return a translucent fill; the caller is
    /// responsible for compositing the blur behind it.
    pub fn surface_fill(&self, mode: SurfaceMode) -> Color {
        match mode {
            SurfaceMode::Opaque => self.bg_raised,
            SurfaceMode::Glass => self.bg.with_alpha(190),
            SurfaceMode::WallpaperIntegrated => self.bg_raised.with_alpha(120),
        }
    }
}

// Small helper used by the tinting above; lives here to keep `Color` generic.
trait DarkenToCeiling {
    /// Darken the color until its luminance is at most `ceiling`.
    fn darken_to_ceiling(self, ceiling: f32) -> Self;
}

impl DarkenToCeiling for Color {
    fn darken_to_ceiling(self, ceiling: f32) -> Self {
        let mut c = self;
        for step in 0..20 {
            if c.luminance() <= ceiling {
                break;
            }
            c = self.darken(step as f32 / 20.0);
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::{Palette, Swatch};

    #[test]
    fn default_matches_archonsync_scheme() {
        let t = Theme::default();
        assert_eq!(t.accent.to_hex(), "#ff7a1a");
        assert_eq!(t.bg_deep.to_hex(), "#0c0c0f");
        assert_eq!(t.text.to_hex(), "#e8e8ec");
    }

    #[test]
    fn derived_theme_keeps_dark_surfaces() {
        let p = Palette {
            swatches: vec![
                Swatch { color: Color::rgb(30, 60, 120), weight: 0.7 },
                Swatch { color: Color::rgb(220, 180, 60), weight: 0.3 },
            ],
        };
        let t = Theme::from_palette(&p, AccentMode::Harmonize);
        assert!(t.bg.is_dark(), "bg should stay dark, got {:?}", t.bg);
        assert!(t.text.contrast(t.bg) >= 7.0, "text must stay readable");
        assert!(t.accent.contrast(t.bg) >= 3.5, "accent must read on bg");
    }

    #[test]
    fn empty_palette_falls_back_to_default() {
        let t = Theme::from_palette(&Palette { swatches: vec![] }, AccentMode::Harmonize);
        assert_eq!(t, Theme::default());
    }

    #[test]
    fn surface_modes_differ_in_alpha() {
        let t = Theme::default();
        assert_eq!(t.surface_fill(SurfaceMode::Opaque).a, 255);
        assert!(t.surface_fill(SurfaceMode::Glass).a < 255);
    }
}

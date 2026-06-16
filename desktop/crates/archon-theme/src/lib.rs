//! # archon-theme
//!
//! The color-science engine behind "ArchonSync AI" adaptive theming. It has no
//! GPU or windowing dependencies — it takes a wallpaper (or raw pixels) and
//! produces a [`Theme`] of resolved colors that every other ArchonSync surface
//! reads from. Because it is pure data-in/data-out, it is fully unit-tested and
//! builds anywhere, including headless CI.
//!
//! ```
//! use archon_theme::{Theme, AccentMode, palette};
//!
//! // 2x1 image: near-black + brand orange.
//! let rgba = [12, 12, 15, 255,  255, 122, 26, 255];
//! let pal = palette::extract_from_rgba(&rgba, 2, 1, 4);
//! let theme = Theme::from_palette(&pal, AccentMode::Harmonize);
//! assert!(theme.bg.is_dark());
//! ```

mod color;
mod harmony;
pub mod palette;
mod theme;

pub use color::Color;
pub use harmony::{accent_for, ensure_contrast, AccentMode, SIGNATURE_ORANGE};
pub use palette::{Palette, Swatch};
pub use theme::{SurfaceMode, Theme};

use std::path::Path;

/// Errors loading a wallpaper for analysis.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("failed to read wallpaper: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to decode wallpaper image: {0}")]
    Decode(#[from] image::ImageError),
}

/// Load a wallpaper from disk and derive a theme from it in one step.
///
/// This is the call the compositor makes when the wallpaper changes. The image
/// is decoded, downsampled, clustered, and turned into a [`Theme`] under the
/// requested [`AccentMode`].
pub fn theme_from_wallpaper(path: impl AsRef<Path>, mode: AccentMode) -> Result<Theme, ThemeError> {
    let img = image::open(path)?.to_rgba8();
    let (w, h) = img.dimensions();
    let pal = palette::extract_from_rgba(img.as_raw(), w, h, 6);
    Ok(Theme::from_palette(&pal, mode))
}

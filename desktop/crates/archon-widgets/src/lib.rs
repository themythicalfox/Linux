//! # archon-widgets
//!
//! The model for ArchonSync's modular desktop widgets. A [`Widget`] is a kind
//! plus a [`Placement`] (where it sits on the wallpaper, how big, rotation,
//! z-order) plus a [`SurfaceMode`] (glass / opaque / wallpaper-integrated). The
//! shell owns a [`Desktop`] — the ordered collection of widgets — and renders
//! each one with `archon-ui`.
//!
//! This crate is pure data + geometry so it can be fully unit tested; the
//! actual data sources (system stats, clock, music) and GPU drawing live in the
//! shell. Layouts round-trip through serde for persistence.

mod graph;

pub use archon_theme::SurfaceMode;
pub use graph::History;

use serde::{Deserialize, Serialize};

/// The built-in widget kinds. Each carries only its *configuration*; live data
/// is fetched by the shell at render time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WidgetKind {
    /// Wall clock. `format` is a strftime-style string.
    Clock { format: String, show_seconds: bool },
    /// Date / month calendar.
    Calendar,
    /// CPU / GPU / memory monitor with a live graph.
    SystemMonitor { metric: Metric },
    /// Current-track music display driven by MPRIS.
    MusicPlayer,
    /// Current conditions for `location`.
    Weather { location: String },
    /// A grid of app shortcuts.
    QuickLauncher { apps: Vec<String> },
    /// Headlines from an RSS/Atom `url`.
    RssFeed { url: String },
    /// Live preview of a pinned, always-on-desktop app window.
    PinnedApp { app_id: String },
}

/// Which system metric a [`WidgetKind::SystemMonitor`] tracks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Metric {
    Cpu,
    Gpu,
    Memory,
    Network,
}

/// Where and how a widget sits on the desktop layer.
///
/// Coordinates are logical pixels with the origin at the top-left of the
/// primary output. `rotation` is in degrees, clockwise. `z` orders widgets back
/// to front.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub z: i32,
}

impl Placement {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Placement { x, y, width, height, rotation: 0.0, z: 0 }
    }

    /// Axis-aligned center of the widget, ignoring rotation.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Hit test a point against the (possibly rotated) widget rectangle. The
    /// point is rotated into the widget's local frame and compared to the
    /// unrotated bounds — this is what lets users grab a tilted widget.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        let (cx, cy) = self.center();
        let theta = -self.rotation.to_radians();
        let (s, c) = theta.sin_cos();
        let dx = px - cx;
        let dy = py - cy;
        let lx = dx * c - dy * s;
        let ly = dx * s + dy * c;
        lx.abs() <= self.width / 2.0 && ly.abs() <= self.height / 2.0
    }

    /// Clamp the widget so it stays within an output of `bounds_w` x `bounds_h`,
    /// keeping at least `margin` px on screen. Prevents widgets being dragged
    /// fully off the desktop.
    pub fn clamp_into(&mut self, bounds_w: f32, bounds_h: f32, margin: f32) {
        let min_x = margin - self.width;
        let min_y = margin - self.height;
        self.x = self.x.clamp(min_x, bounds_w - margin);
        self.y = self.y.clamp(min_y, bounds_h - margin);
    }
}

/// A widget instance: its identity, kind, placement and appearance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Widget {
    pub id: u64,
    pub kind: WidgetKind,
    pub placement: Placement,
    pub surface: SurfaceMode,
    /// Per-widget opacity multiplier (`0..=1`), applied on top of the surface
    /// mode's own alpha.
    pub opacity: f32,
    /// When true, the theming engine is allowed to restyle this widget when the
    /// wallpaper changes. Users who hand-tune a widget can switch this off.
    pub adaptive: bool,
}

impl Widget {
    pub fn new(id: u64, kind: WidgetKind, placement: Placement) -> Self {
        Widget {
            id,
            kind,
            placement,
            surface: SurfaceMode::default(),
            opacity: 1.0,
            adaptive: true,
        }
    }
}

/// The ordered collection of widgets on the desktop. Persisted as part of the
/// session layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Desktop {
    pub widgets: Vec<Widget>,
    next_id: u64,
}

impl Desktop {
    pub fn new() -> Self {
        Desktop::default()
    }

    /// Add a widget at `placement`, assigning it a fresh id and the top z-order.
    /// Returns the new widget's id.
    pub fn add(&mut self, kind: WidgetKind, placement: Placement) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let z = self.widgets.iter().map(|w| w.placement.z).max().unwrap_or(0) + 1;
        let mut w = Widget::new(id, kind, placement);
        w.placement.z = z;
        self.widgets.push(w);
        id
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let before = self.widgets.len();
        self.widgets.retain(|w| w.id != id);
        self.widgets.len() != before
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Widget> {
        self.widgets.iter_mut().find(|w| w.id == id)
    }

    /// Bring a widget to the front by giving it a z above all others.
    pub fn raise(&mut self, id: u64) {
        let top = self.widgets.iter().map(|w| w.placement.z).max().unwrap_or(0);
        if let Some(w) = self.get_mut(id) {
            w.placement.z = top + 1;
        }
    }

    /// The topmost widget containing `(px, py)`, for click/drag targeting.
    pub fn hit(&self, px: f32, py: f32) -> Option<u64> {
        self.widgets
            .iter()
            .filter(|w| w.placement.contains(px, py))
            .max_by_key(|w| w.placement.z)
            .map(|w| w.id)
    }

    /// Widgets in back-to-front draw order.
    pub fn draw_order(&self) -> Vec<&Widget> {
        let mut v: Vec<&Widget> = self.widgets.iter().collect();
        v.sort_by_key(|w| w.placement.z);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> WidgetKind {
        WidgetKind::Clock { format: "%H:%M".into(), show_seconds: false }
    }

    #[test]
    fn add_assigns_increasing_z_and_ids() {
        let mut d = Desktop::new();
        let a = d.add(clock(), Placement::new(0.0, 0.0, 100.0, 50.0));
        let b = d.add(WidgetKind::Calendar, Placement::new(0.0, 0.0, 100.0, 50.0));
        assert_ne!(a, b);
        assert!(d.widgets[1].placement.z > d.widgets[0].placement.z);
    }

    #[test]
    fn hit_returns_topmost() {
        let mut d = Desktop::new();
        let _bottom = d.add(clock(), Placement::new(0.0, 0.0, 100.0, 100.0));
        let top = d.add(WidgetKind::Calendar, Placement::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(d.hit(50.0, 50.0), Some(top));
    }

    #[test]
    fn rotated_contains_uses_local_frame() {
        let mut p = Placement::new(0.0, 0.0, 100.0, 20.0);
        p.rotation = 90.0;
        // After a 90-degree rotation the tall axis is vertical: a point well
        // above the center (but within the now-vertical extent) should hit.
        let (cx, cy) = p.center();
        assert!(p.contains(cx, cy - 40.0));
        // ...while a point far to the side should miss.
        assert!(!p.contains(cx + 40.0, cy));
    }

    #[test]
    fn clamp_keeps_widget_partly_on_screen() {
        let mut p = Placement::new(5000.0, 5000.0, 200.0, 100.0);
        p.clamp_into(1920.0, 1080.0, 32.0);
        assert!(p.x <= 1920.0 - 32.0);
        assert!(p.y <= 1080.0 - 32.0);
    }

    #[test]
    fn layout_roundtrips_through_json() {
        let mut d = Desktop::new();
        d.add(clock(), Placement::new(10.0, 10.0, 220.0, 90.0));
        d.add(WidgetKind::SystemMonitor { metric: Metric::Gpu }, Placement::new(40.0, 200.0, 260.0, 120.0));
        let json = serde_json::to_string(&d).unwrap();
        let back: Desktop = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}

//! The edge-activated, auto-hiding dock.
//!
//! The dock lives off-screen against a configurable edge and slides in when the
//! pointer reaches the reveal zone (or when pinned open). A spring drives the
//! reveal so the motion is smooth and interruptible. This module is just the
//! reveal logic + geometry; the layer-shell surface and rendering are wired in
//! the shell binary.

use archon_config::{DockConfig, Edge};
use archon_ui::scene::{Rect, Vec2};
use archon_ui::Spring;

/// The dock's slide-in state.
pub struct Dock {
    cfg: DockConfig,
    /// Reveal amount: 0 = fully hidden off-screen, 1 = fully shown.
    reveal: Spring,
    /// Pinned open (e.g. via `archonctl toggle-dock`) ignores auto-hide.
    pinned: bool,
    /// Whether the pointer is currently over the revealed dock.
    hovered: bool,
}

impl Dock {
    pub fn new(cfg: DockConfig) -> Self {
        Dock {
            cfg,
            reveal: Spring::new(0.0).with_params(200.0, 26.0),
            pinned: false,
            hovered: false,
        }
    }

    pub fn edge(&self) -> Edge {
        self.cfg.edge
    }

    /// Pin/unpin the dock open. Returns the new pinned state.
    pub fn toggle_pinned(&mut self) -> bool {
        self.pinned = !self.pinned;
        self.pinned
    }

    /// Feed the latest pointer position (in logical pixels) against an output of
    /// `w` x `h`. Updates whether the dock wants to be shown.
    pub fn update_pointer(&mut self, p: Vec2, w: f32, h: f32) {
        let zone = self.cfg.reveal_zone as f32;
        let thickness = self.cfg.thickness as f32;
        // "In the reveal zone" = near the configured edge.
        let near_edge = match self.cfg.edge {
            Edge::Left => p.x <= zone,
            Edge::Right => p.x >= w - zone,
            Edge::Top => p.y <= zone,
            Edge::Bottom => p.y >= h - zone,
        };
        // "Hovered" = within the dock's revealed thickness, so it stays open
        // while the pointer is on it.
        self.hovered = match self.cfg.edge {
            Edge::Left => p.x <= thickness,
            Edge::Right => p.x >= w - thickness,
            Edge::Top => p.y <= thickness,
            Edge::Bottom => p.y >= h - thickness,
        };
        let want_shown = self.pinned || near_edge || (self.hovered && self.is_open());
        self.reveal.set_target(if want_shown { 1.0 } else if self.cfg.auto_hide { 0.0 } else { 1.0 });
    }

    /// Advance the slide animation.
    pub fn tick(&mut self, dt: f32) {
        self.reveal.step(dt);
    }

    /// Reveal fraction `0..=1` for rendering.
    pub fn reveal(&self) -> f32 {
        self.reveal.value.clamp(0.0, 1.0)
    }

    /// Considered open once mostly revealed.
    pub fn is_open(&self) -> bool {
        self.reveal.value > 0.5
    }

    /// The dock's on-screen rectangle for the current reveal amount. When
    /// hidden it sits just off the edge; when shown it hugs the edge. `length`
    /// is how long the dock runs along its edge.
    pub fn rect(&self, w: f32, h: f32, length: f32) -> Rect {
        let t = self.cfg.thickness as f32;
        let r = self.reveal();
        let hidden_shift = t * (1.0 - r); // how far off-screen
        match self.cfg.edge {
            Edge::Left => Rect::new(-hidden_shift, (h - length) / 2.0, t, length),
            Edge::Right => Rect::new(w - t + hidden_shift, (h - length) / 2.0, t, length),
            Edge::Top => Rect::new((w - length) / 2.0, -hidden_shift, length, t),
            Edge::Bottom => Rect::new((w - length) / 2.0, h - t + hidden_shift, length, t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dock(edge: Edge) -> Dock {
        Dock::new(DockConfig { edge, reveal_zone: 6, auto_hide: true, thickness: 64 })
    }

    fn settle(d: &mut Dock) {
        for _ in 0..600 {
            d.tick(1.0 / 120.0);
        }
    }

    #[test]
    fn pointer_at_edge_reveals() {
        let mut d = dock(Edge::Left);
        d.update_pointer(Vec2::new(2.0, 500.0), 1920.0, 1080.0);
        settle(&mut d);
        assert!(d.is_open());
    }

    #[test]
    fn pointer_away_hides_when_auto_hide() {
        let mut d = dock(Edge::Left);
        d.update_pointer(Vec2::new(2.0, 500.0), 1920.0, 1080.0);
        settle(&mut d);
        d.update_pointer(Vec2::new(900.0, 500.0), 1920.0, 1080.0);
        settle(&mut d);
        assert!(!d.is_open());
    }

    #[test]
    fn pinned_dock_ignores_pointer() {
        let mut d = dock(Edge::Bottom);
        assert!(d.toggle_pinned());
        d.update_pointer(Vec2::new(900.0, 100.0), 1920.0, 1080.0); // far from edge
        settle(&mut d);
        assert!(d.is_open());
    }

    #[test]
    fn hidden_rect_sits_off_edge() {
        let d = dock(Edge::Left);
        // Freshly hidden: x is negative (off-screen to the left).
        let r = d.rect(1920.0, 1080.0, 600.0);
        assert!(r.x < 0.0);
    }

    #[test]
    fn right_edge_reveals_from_right() {
        let mut d = dock(Edge::Right);
        d.update_pointer(Vec2::new(1918.0, 500.0), 1920.0, 1080.0);
        settle(&mut d);
        assert!(d.is_open());
    }
}

//! A backend-agnostic draw list.
//!
//! Every ArchonSync surface (lock screen, dock, wheel, widgets) describes a
//! frame as a list of [`Primitive`]s in logical pixels. A renderer (the wgpu
//! backend, or a test harness) consumes the list. Keeping the description
//! separate from the GPU means the layout/visual logic is testable without a
//! graphics context.

use archon_theme::Color;

/// A 2D point / vector in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

/// An axis-aligned rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }
}

/// One drawable element.
#[derive(Clone, Debug)]
pub enum Primitive {
    /// A filled, rounded rectangle with an optional hairline border. The fill's
    /// alpha plus `blur` drives the glassmorphism look in the renderer.
    RoundedRect {
        rect: Rect,
        radius: f32,
        fill: Color,
        border: Color,
        border_width: f32,
        /// Background blur radius behind this rect (0 = no blur).
        blur: f32,
    },
    /// A soft radial glow — the lock-screen swipe dot and accent highlights.
    Glow {
        center: Vec2,
        radius: f32,
        color: Color,
        /// Brightness multiplier `0..` (1 = nominal).
        intensity: f32,
    },
    /// A ring/arc segment, used to draw the control wheel.
    Arc {
        center: Vec2,
        radius: f32,
        thickness: f32,
        start_deg: f32,
        sweep_deg: f32,
        color: Color,
    },
    /// A run of text. Layout/shaping happens in the renderer's text backend.
    Text {
        pos: Vec2,
        content: String,
        size: f32,
        color: Color,
    },
    /// A batch of particles (uploaded as point sprites).
    Particles(Vec<super::particles::Particle>),
}

/// An ordered list of primitives plus the logical size of the frame.
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    pub primitives: Vec<Primitive>,
}

impl DrawList {
    pub fn new() -> Self {
        DrawList::default()
    }

    pub fn push(&mut self, p: Primitive) {
        self.primitives.push(p);
    }

    /// Convenience: a glass panel using a theme surface fill.
    pub fn glass_panel(&mut self, rect: Rect, radius: f32, fill: Color, border: Color, blur: f32) {
        self.push(Primitive::RoundedRect {
            rect,
            radius,
            fill,
            border,
            border_width: 1.0,
            blur,
        });
    }

    /// Convenience: the signature glowing accent dot.
    pub fn glow_dot(&mut self, center: Vec2, radius: f32, color: Color, intensity: f32) {
        self.push(Primitive::Glow { center, radius, color, intensity });
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_contains_and_center() {
        let r = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert_eq!(r.center(), Vec2::new(60.0, 35.0));
        assert!(r.contains(Vec2::new(60.0, 35.0)));
        assert!(!r.contains(Vec2::new(0.0, 0.0)));
    }

    #[test]
    fn builders_append_primitives() {
        let mut dl = DrawList::new();
        dl.glass_panel(Rect::new(0.0, 0.0, 64.0, 400.0), 14.0, Color::rgb(18, 18, 22), Color::rgb(255, 122, 26), 24.0);
        dl.glow_dot(Vec2::new(8.0, 200.0), 6.0, Color::rgb(255, 122, 26), 1.5);
        assert_eq!(dl.len(), 2);
    }
}

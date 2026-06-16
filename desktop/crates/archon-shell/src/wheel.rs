//! The radial control wheel.
//!
//! Inspired by the ArchonSync "Wheel" launcher: a circle of items the user
//! scrolls to rotate, with the item at the top (12 o'clock) selected and shown
//! in the center hub. Clicking the hub (or an item) triggers its action. This
//! module computes the geometry and selection; the shell binary renders it via
//! `archon-ui` and dispatches actions over IPC.

use archon_ui::scene::{DrawList, Primitive, Vec2};
use archon_ui::Spring;
use archon_theme::Theme;
use std::f32::consts::{PI, TAU};

/// What a wheel item does when activated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WheelAction {
    Power,
    Settings,
    Notifications,
    Workspaces,
    Widgets,
    GameMode,
    Lock,
    /// Launch an arbitrary command.
    Launch(String),
}

/// One entry on the wheel.
#[derive(Clone, Debug)]
pub struct WheelItem {
    pub label: String,
    /// Icon name (freedesktop icon theme) the renderer resolves.
    pub icon: String,
    pub action: WheelAction,
}

impl WheelItem {
    pub fn new(label: &str, icon: &str, action: WheelAction) -> Self {
        WheelItem { label: label.into(), icon: icon.into(), action }
    }
}

/// The control wheel: a set of items plus a (spring-animated) rotation.
pub struct ControlWheel {
    items: Vec<WheelItem>,
    /// Rotation in "item steps" (continuous); the spring chases an integer step.
    rotation: Spring,
    radius: f32,
}

impl ControlWheel {
    /// The default ArchonSync control wheel.
    pub fn default_items() -> Vec<WheelItem> {
        vec![
            WheelItem::new("Power", "system-shutdown", WheelAction::Power),
            WheelItem::new("Settings", "configure", WheelAction::Settings),
            WheelItem::new("Notifications", "notifications", WheelAction::Notifications),
            WheelItem::new("Workspaces", "view-grid", WheelAction::Workspaces),
            WheelItem::new("Widgets", "widget", WheelAction::Widgets),
            WheelItem::new("Game Mode", "applications-games", WheelAction::GameMode),
            WheelItem::new("Lock", "system-lock-screen", WheelAction::Lock),
        ]
    }

    pub fn new(items: Vec<WheelItem>, radius: f32) -> Self {
        ControlWheel {
            items,
            rotation: Spring::new(0.0).with_params(160.0, 22.0),
            radius,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Scroll by `steps` notches (positive = next item to the top).
    pub fn scroll(&mut self, steps: i32) {
        let target = (self.rotation.target + steps as f32).rem_euclid(self.len() as f32);
        self.rotation.set_target(target);
    }

    pub fn tick(&mut self, dt: f32) {
        self.rotation.step(dt);
    }

    /// Index of the currently selected item (nearest the top), using the
    /// animated rotation so selection updates as the wheel spins.
    pub fn selected(&self) -> usize {
        let n = self.len();
        if n == 0 {
            return 0;
        }
        (self.rotation.value.round() as i64).rem_euclid(n as i64) as usize
    }

    pub fn selected_item(&self) -> Option<&WheelItem> {
        self.items.get(self.selected())
    }

    /// The angle (radians, 0 = +x axis, counter-clockwise) at which item `i`
    /// currently sits. The selected item is placed at the top (−π/2).
    pub fn item_angle(&self, i: usize) -> f32 {
        let n = self.len().max(1) as f32;
        let step = TAU / n;
        // Item i relative to the current rotation, anchored so the selected
        // item is at the top.
        -PI / 2.0 + (i as f32 - self.rotation.value) * step
    }

    /// Screen position of item `i` around `center`.
    pub fn item_position(&self, i: usize, center: Vec2) -> Vec2 {
        let a = self.item_angle(i);
        Vec2::new(center.x + a.cos() * self.radius, center.y + a.sin() * self.radius)
    }

    /// Build the wheel's frame around `center`: a ring, a glowing dot per item
    /// (brightest for the selected one), and the selected label in the hub.
    pub fn build_scene(&self, center: Vec2, theme: &Theme) -> DrawList {
        let mut dl = DrawList::new();
        // Backing ring.
        dl.push(Primitive::Arc {
            center,
            radius: self.radius,
            thickness: 3.0,
            start_deg: 0.0,
            sweep_deg: 360.0,
            color: theme.accent.with_alpha(60),
        });
        let selected = self.selected();
        for (i, item) in self.items.iter().enumerate() {
            let pos = self.item_position(i, center);
            let is_sel = i == selected;
            let intensity = if is_sel { 2.0 } else { 0.8 };
            let r = if is_sel { 16.0 } else { 10.0 };
            dl.glow_dot(pos, r, theme.accent, intensity);
            let _ = item; // icon resolved by the renderer from item.icon
        }
        // Center hub with the selected label.
        if let Some(item) = self.selected_item() {
            dl.push(Primitive::Text {
                pos: Vec2::new(center.x - 40.0, center.y - 10.0),
                content: item.label.clone(),
                size: 20.0,
                color: theme.text,
            });
        }
        dl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel() -> ControlWheel {
        ControlWheel::new(ControlWheel::default_items(), 120.0)
    }

    fn settle(w: &mut ControlWheel) {
        for _ in 0..600 {
            w.tick(1.0 / 120.0);
        }
    }

    #[test]
    fn default_wheel_is_populated() {
        assert_eq!(wheel().len(), 7);
    }

    #[test]
    fn scrolling_changes_selection() {
        let mut w = wheel();
        assert_eq!(w.selected(), 0);
        w.scroll(2);
        settle(&mut w);
        assert_eq!(w.selected(), 2);
    }

    #[test]
    fn scroll_wraps_around() {
        let mut w = wheel();
        w.scroll(-1); // before the first item wraps to the last
        settle(&mut w);
        assert_eq!(w.selected(), w.len() - 1);
    }

    #[test]
    fn selected_item_sits_at_the_top() {
        let w = wheel();
        let center = Vec2::new(500.0, 500.0);
        let pos = w.item_position(w.selected(), center);
        // Top of the circle: same x as center, smaller y.
        assert!((pos.x - center.x).abs() < 1.0);
        assert!(pos.y < center.y);
    }

    #[test]
    fn items_lie_on_the_radius() {
        let w = wheel();
        let center = Vec2::new(0.0, 0.0);
        for i in 0..w.len() {
            let p = w.item_position(i, center);
            let dist = (p.x * p.x + p.y * p.y).sqrt();
            assert!((dist - 120.0).abs() < 1e-2, "item {i} off-radius: {dist}");
        }
    }

    #[test]
    fn scene_contains_ring_and_items() {
        let w = wheel();
        let dl = w.build_scene(Vec2::new(500.0, 500.0), &Theme::default());
        // 1 ring + 7 dots + 1 label.
        assert!(dl.len() >= 9);
    }
}

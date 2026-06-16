//! Window arrangement math: edge/corner snapping and automatic tiling.
//!
//! All of this is pure geometry over [`Geometry`] values, so the behavior of
//! ArchonSync's window management — how a half-snap lands, how a master/stack
//! split divides the screen — is pinned down by unit tests rather than tested
//! by hand in a running session.

use crate::window::Geometry;

/// Where a window snaps when dragged to a screen region or triggered by a
/// keybinding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapTarget {
    LeftHalf,
    RightHalf,
    TopHalf,
    BottomHalf,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Maximize,
}

impl SnapTarget {
    /// Resolve this snap target against a work `area` (the output minus any
    /// reserved panel space).
    pub fn resolve(self, area: Geometry) -> Geometry {
        let (x, y, w, h) = (area.x, area.y, area.w, area.h);
        let (hw, hh) = (w / 2, h / 2);
        match self {
            SnapTarget::LeftHalf => Geometry::new(x, y, hw, h),
            SnapTarget::RightHalf => Geometry::new(x + hw, y, w - hw, h),
            SnapTarget::TopHalf => Geometry::new(x, y, w, hh),
            SnapTarget::BottomHalf => Geometry::new(x, y + hh, w, h - hh),
            SnapTarget::TopLeft => Geometry::new(x, y, hw, hh),
            SnapTarget::TopRight => Geometry::new(x + hw, y, w - hw, hh),
            SnapTarget::BottomLeft => Geometry::new(x, y + hh, hw, h - hh),
            SnapTarget::BottomRight => Geometry::new(x + hw, y + hh, w - hw, h - hh),
            SnapTarget::Maximize => area,
        }
    }
}

/// Detect a snap target from a pointer position near the work-area edges, used
/// while dragging a window. `margin` is how close to an edge counts as a hit.
/// Corners take priority over edges. Returns `None` when the pointer is not in
/// a snap zone.
pub fn snap_from_pointer(area: Geometry, px: i32, py: i32, margin: i32) -> Option<SnapTarget> {
    let near_left = px <= area.x + margin;
    let near_right = px >= area.x + area.w - margin;
    let near_top = py <= area.y + margin;
    let near_bottom = py >= area.y + area.h - margin;

    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(SnapTarget::TopLeft),
        (_, true, true, _) => Some(SnapTarget::TopRight),
        (true, _, _, true) => Some(SnapTarget::BottomLeft),
        (_, true, _, true) => Some(SnapTarget::BottomRight),
        (true, _, _, _) => Some(SnapTarget::LeftHalf),
        (_, true, _, _) => Some(SnapTarget::RightHalf),
        (_, _, true, _) => Some(SnapTarget::Maximize), // dragging to top maximizes
        _ => None,
    }
}

/// The automatic tiling arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TilingMode {
    /// One large master window on the left, the rest stacked on the right.
    #[default]
    MasterStack,
    /// Equal-width columns.
    Columns,
    /// A near-square grid.
    Grid,
}

/// Compute tiled geometries for `count` windows within `area`.
///
/// `gap` is applied as an inset on every window. `master_ratio` (`0..1`) is the
/// fraction of width the master column gets in [`TilingMode::MasterStack`].
/// Returns one [`Geometry`] per window, in input order.
pub fn tile(area: Geometry, count: usize, mode: TilingMode, gap: i32, master_ratio: f32) -> Vec<Geometry> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![area.inset(gap)];
    }

    match mode {
        TilingMode::MasterStack => {
            let ratio = master_ratio.clamp(0.1, 0.9);
            let master_w = (area.w as f32 * ratio).round() as i32;
            let stack_w = area.w - master_w;
            let stack_n = count - 1;
            let mut out = Vec::with_capacity(count);
            out.push(Geometry::new(area.x, area.y, master_w, area.h).inset(gap));
            let cell_h = area.h / stack_n as i32;
            for i in 0..stack_n {
                let y = area.y + cell_h * i as i32;
                // Last cell absorbs the rounding remainder.
                let h = if i == stack_n - 1 { area.h - cell_h * i as i32 } else { cell_h };
                out.push(Geometry::new(area.x + master_w, y, stack_w, h).inset(gap));
            }
            out
        }
        TilingMode::Columns => {
            let cell_w = area.w / count as i32;
            (0..count)
                .map(|i| {
                    let x = area.x + cell_w * i as i32;
                    let w = if i == count - 1 { area.w - cell_w * i as i32 } else { cell_w };
                    Geometry::new(x, area.y, w, area.h).inset(gap)
                })
                .collect()
        }
        TilingMode::Grid => {
            let cols = (count as f32).sqrt().ceil() as i32;
            let rows = ((count as i32) + cols - 1) / cols;
            let cell_w = area.w / cols;
            let cell_h = area.h / rows;
            (0..count)
                .map(|i| {
                    let c = i as i32 % cols;
                    let r = i as i32 / cols;
                    let x = area.x + cell_w * c;
                    let y = area.y + cell_h * r;
                    let w = if c == cols - 1 { area.w - cell_w * c } else { cell_w };
                    let h = if r == rows - 1 { area.h - cell_h * r } else { cell_h };
                    Geometry::new(x, y, w, h).inset(gap)
                })
                .collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Geometry = Geometry { x: 0, y: 0, w: 1920, h: 1080 };

    #[test]
    fn halves_partition_the_area_exactly() {
        let l = SnapTarget::LeftHalf.resolve(AREA);
        let r = SnapTarget::RightHalf.resolve(AREA);
        assert_eq!(l.w + r.w, AREA.w);
        assert_eq!(l.x, 0);
        assert_eq!(r.x, l.w);
    }

    #[test]
    fn maximize_is_the_whole_area() {
        assert_eq!(SnapTarget::Maximize.resolve(AREA), AREA);
    }

    #[test]
    fn pointer_corners_take_priority() {
        assert_eq!(snap_from_pointer(AREA, 2, 2, 10), Some(SnapTarget::TopLeft));
        assert_eq!(snap_from_pointer(AREA, 1918, 1078, 10), Some(SnapTarget::BottomRight));
        assert_eq!(snap_from_pointer(AREA, 5, 540, 10), Some(SnapTarget::LeftHalf));
        assert_eq!(snap_from_pointer(AREA, 960, 540, 10), None);
    }

    #[test]
    fn master_stack_covers_width_and_counts() {
        let g = tile(AREA, 3, TilingMode::MasterStack, 0, 0.6);
        assert_eq!(g.len(), 3);
        // Master width + stack width spans the full area.
        let master_right = g[0].x + g[0].w;
        assert_eq!(master_right + g[1].w, AREA.w);
        // The two stack windows partition the height exactly.
        assert_eq!(g[1].h + g[2].h, AREA.h);
    }

    #[test]
    fn columns_partition_width() {
        let g = tile(AREA, 4, TilingMode::Columns, 0, 0.5);
        let total: i32 = g.iter().map(|c| c.w).sum();
        assert_eq!(total, AREA.w);
    }

    #[test]
    fn grid_places_all_windows() {
        let g = tile(AREA, 5, TilingMode::Grid, 4, 0.5);
        assert_eq!(g.len(), 5);
        // Every tile is within the area.
        for c in &g {
            assert!(c.x >= AREA.x && c.y >= AREA.y);
            assert!(c.x + c.w <= AREA.x + AREA.w + 1);
        }
    }

    #[test]
    fn single_window_fills_area_minus_gap() {
        let g = tile(AREA, 1, TilingMode::MasterStack, 12, 0.5);
        assert_eq!(g[0], AREA.inset(12));
    }
}

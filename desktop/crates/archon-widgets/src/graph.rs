//! A fixed-capacity ring buffer for the live graphs (CPU/GPU/network).
//!
//! Widgets push a new sample each frame and read back the most recent `N`
//! values to draw a sparkline. Keeping this as its own tested type means the
//! GPU drawing code never has to worry about wrap-around or normalization.

/// A rolling history of `f32` samples, newest last.
#[derive(Clone, Debug)]
pub struct History {
    buf: Vec<f32>,
    cap: usize,
}

impl History {
    /// Create a history holding up to `cap` samples (at least 1).
    pub fn new(cap: usize) -> Self {
        History { buf: Vec::with_capacity(cap.max(1)), cap: cap.max(1) }
    }

    /// Append a sample, dropping the oldest once full.
    pub fn push(&mut self, v: f32) {
        if self.buf.len() == self.cap {
            self.buf.remove(0);
        }
        self.buf.push(v);
    }

    /// Samples oldest-to-newest.
    pub fn samples(&self) -> &[f32] {
        &self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Most recent sample, if any.
    pub fn latest(&self) -> Option<f32> {
        self.buf.last().copied()
    }

    /// Largest sample currently held (for auto-scaling a graph's Y axis).
    pub fn max(&self) -> f32 {
        self.buf.iter().copied().fold(0.0, f32::max)
    }

    /// Samples normalized into `0..=1` against either a fixed `ceiling` or, when
    /// `ceiling <= 0`, the current max. Useful for feeding straight to a shader.
    pub fn normalized(&self, ceiling: f32) -> Vec<f32> {
        let top = if ceiling > 0.0 { ceiling } else { self.max().max(f32::EPSILON) };
        self.buf.iter().map(|&v| (v / top).clamp(0.0, 1.0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_oldest_when_full() {
        let mut h = History::new(3);
        for v in [1.0, 2.0, 3.0, 4.0] {
            h.push(v);
        }
        assert_eq!(h.samples(), &[2.0, 3.0, 4.0]);
        assert_eq!(h.latest(), Some(4.0));
    }

    #[test]
    fn normalizes_against_ceiling() {
        let mut h = History::new(4);
        for v in [25.0, 50.0, 100.0] {
            h.push(v);
        }
        assert_eq!(h.normalized(100.0), vec![0.25, 0.5, 1.0]);
    }

    #[test]
    fn auto_scales_when_ceiling_unset() {
        let mut h = History::new(4);
        h.push(2.0);
        h.push(4.0);
        assert_eq!(h.normalized(0.0), vec![0.5, 1.0]);
    }
}

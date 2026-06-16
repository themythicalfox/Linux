//! Wallpaper palette extraction.
//!
//! We run a small, deterministic k-means over the wallpaper's pixels to find the
//! handful of colors that dominate the image. Determinism matters: the same
//! wallpaper must always produce the same theme, so we seed the centroids
//! evenly across sorted-by-luminance samples instead of randomly, and iterate a
//! fixed number of times. This is plenty for theme derivation and keeps the
//! whole thing unit-testable with no RNG.

use crate::color::Color;

/// One dominant color and the fraction of the image it represents (`0..=1`).
#[derive(Clone, Copy, Debug)]
pub struct Swatch {
    pub color: Color,
    pub weight: f32,
}

/// The extracted palette, ordered most-dominant first.
#[derive(Clone, Debug)]
pub struct Palette {
    pub swatches: Vec<Swatch>,
}

impl Palette {
    /// The single most dominant color.
    pub fn dominant(&self) -> Color {
        self.swatches
            .first()
            .map(|s| s.color)
            .unwrap_or(Color::rgb(18, 18, 22))
    }

    /// The most vivid (highest chroma) swatch — the one most useful as an accent.
    /// Falls back to the dominant color if everything is grey.
    pub fn most_vivid(&self) -> Color {
        self.swatches
            .iter()
            .max_by(|a, b| chroma(a.color).total_cmp(&chroma(b.color)))
            .map(|s| s.color)
            .unwrap_or_else(|| self.dominant())
    }

    /// Average luminance across the palette weighted by coverage — tells us
    /// whether the wallpaper is overall light or dark.
    pub fn mean_luminance(&self) -> f32 {
        let total: f32 = self.swatches.iter().map(|s| s.weight).sum();
        if total <= 0.0 {
            return 0.0;
        }
        self.swatches
            .iter()
            .map(|s| s.color.luminance() * s.weight)
            .sum::<f32>()
            / total
    }
}

/// Chroma proxy: max-min of RGB. Cheap and good enough to rank "colorfulness".
fn chroma(c: Color) -> f32 {
    let max = c.r.max(c.g).max(c.b) as f32;
    let min = c.r.min(c.g).min(c.b) as f32;
    (max - min) / 255.0
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    dr * dr + dg * dg + db * db
}

/// Extract up to `k` dominant colors from a set of RGB samples.
///
/// `samples` is a flat list of `[r, g, b]` in `0..=255`. The caller is expected
/// to have downsampled the image first (see [`extract_from_rgba`]).
pub fn extract(samples: &[[u8; 3]], k: usize) -> Palette {
    let k = k.max(1);
    if samples.is_empty() {
        return Palette { swatches: vec![] };
    }

    let pts: Vec<[f32; 3]> = samples
        .iter()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();

    // Deterministic seeding: sort by luminance and pick evenly spaced points so
    // the initial centroids span the dark-to-light range of the image.
    let mut order: Vec<usize> = (0..pts.len()).collect();
    order.sort_by(|&a, &b| {
        let la = 0.2126 * pts[a][0] + 0.7152 * pts[a][1] + 0.0722 * pts[a][2];
        let lb = 0.2126 * pts[b][0] + 0.7152 * pts[b][1] + 0.0722 * pts[b][2];
        la.total_cmp(&lb)
    });
    let k = k.min(pts.len());
    // Spread the initial centroids evenly across the sorted range so the first
    // and last seeds land on the darkest and lightest samples. Spacing by
    // `k - 1` (not `k`) is what makes the endpoints inclusive; with k == 1 we
    // just seed from the middle sample.
    let last = order.len() - 1;
    let mut centroids: Vec<[f32; 3]> = (0..k)
        .map(|i| {
            let idx = if k == 1 { last / 2 } else { i * last / (k - 1) };
            pts[order[idx]]
        })
        .collect();

    let mut assignment = vec![0usize; pts.len()];
    for _ in 0..16 {
        // Assign each point to the nearest centroid.
        let mut changed = false;
        for (i, p) in pts.iter().enumerate() {
            let mut best = 0;
            let mut best_d = f32::MAX;
            for (c, cen) in centroids.iter().enumerate() {
                let d = dist2(*p, *cen);
                if d < best_d {
                    best_d = d;
                    best = c;
                }
            }
            if assignment[i] != best {
                assignment[i] = best;
                changed = true;
            }
        }

        // Recompute centroids as the mean of their members.
        let mut sums = vec![[0f32; 3]; k];
        let mut counts = vec![0u32; k];
        for (i, p) in pts.iter().enumerate() {
            let c = assignment[i];
            sums[c][0] += p[0];
            sums[c][1] += p[1];
            sums[c][2] += p[2];
            counts[c] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                let n = counts[c] as f32;
                centroids[c] = [sums[c][0] / n, sums[c][1] / n, sums[c][2] / n];
            }
        }
        if !changed {
            break;
        }
    }

    // Build swatches with coverage weights, ordered most-dominant first.
    let mut counts = vec![0u32; k];
    for &a in &assignment {
        counts[a] += 1;
    }
    let total = pts.len() as f32;
    let mut swatches: Vec<Swatch> = centroids
        .iter()
        .zip(counts.iter())
        .filter(|(_, &n)| n > 0)
        .map(|(c, &n)| Swatch {
            color: Color::rgb(
                c[0].round().clamp(0.0, 255.0) as u8,
                c[1].round().clamp(0.0, 255.0) as u8,
                c[2].round().clamp(0.0, 255.0) as u8,
            ),
            weight: n as f32 / total,
        })
        .collect();
    swatches.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    Palette { swatches }
}

/// Extract a palette directly from an RGBA buffer, downsampling to at most
/// `max_samples` pixels first so extraction stays fast on 4K wallpapers.
pub fn extract_from_rgba(rgba: &[u8], width: u32, height: u32, k: usize) -> Palette {
    let px_count = (width as usize) * (height as usize);
    if px_count == 0 || rgba.len() < px_count * 4 {
        return Palette { swatches: vec![] };
    }
    let max_samples = 4096;
    let step = (px_count / max_samples).max(1);
    let mut samples = Vec::with_capacity(px_count / step + 1);
    let mut i = 0;
    while i < px_count {
        let o = i * 4;
        // Skip near-transparent pixels so cut-out wallpapers don't pull the
        // palette toward whatever shows through.
        if rgba[o + 3] >= 16 {
            samples.push([rgba[o], rgba[o + 1], rgba[o + 2]]);
        }
        i += step;
    }
    extract(&samples, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_safe() {
        let p = extract(&[], 4);
        assert!(p.swatches.is_empty());
        assert_eq!(p.dominant(), Color::rgb(18, 18, 22));
    }

    #[test]
    fn two_clusters_are_found() {
        // A 50/50 split of near-black and orange.
        let mut samples = vec![[10u8, 10, 12]; 100];
        samples.extend(vec![[255u8, 122, 26]; 100]);
        let p = extract(&samples, 2);
        assert_eq!(p.swatches.len(), 2);
        // Most vivid should be the orange cluster.
        let v = p.most_vivid();
        assert!(v.r > 180 && v.g > 60 && v.b < 90, "got {:?}", v);
    }

    #[test]
    fn deterministic() {
        let mut samples = vec![[10u8, 10, 12]; 50];
        samples.extend(vec![[200u8, 40, 60]; 30]);
        samples.extend(vec![[40u8, 120, 200]; 20]);
        let a = extract(&samples, 3);
        let b = extract(&samples, 3);
        let colors_a: Vec<_> = a.swatches.iter().map(|s| s.color).collect();
        let colors_b: Vec<_> = b.swatches.iter().map(|s| s.color).collect();
        assert_eq!(colors_a, colors_b);
    }

    #[test]
    fn weights_sum_to_one() {
        let mut samples = vec![[10u8, 10, 12]; 40];
        samples.extend(vec![[255u8, 122, 26]; 60]);
        let p = extract(&samples, 2);
        let sum: f32 = p.swatches.iter().map(|s| s.weight).sum();
        assert!((sum - 1.0).abs() < 0.001);
    }
}

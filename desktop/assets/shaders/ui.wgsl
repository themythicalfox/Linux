// ArchonSync UI shader.
//
// One unit quad is expanded per instance to cover the primitive's bounding box,
// and the fragment stage uses signed-distance fields to draw rounded
// rectangles, soft radial glows, and ring/arc segments — the building blocks of
// the dock, the control wheel, the lock-screen dot and the widget chrome.
//
// Coordinates arrive in logical pixels; the vertex stage maps them to clip
// space using the screen resolution uniform.

struct Screen {
    resolution: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;

struct Instance {
    @location(1) bounds: vec4<f32>,  // x, y, w, h (logical px)
    @location(2) color: vec4<f32>,
    @location(3) border: vec4<f32>,
    @location(4) params: vec4<f32>,  // radius, border_width, blur, intensity
    @location(5) arc: vec4<f32>,     // start, sweep (rad), thickness, unused
    @location(6) shape: vec4<u32>,   // shape tag in .x
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) local: vec2<f32>,   // pixel coord within the bounding box
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) arc: vec4<f32>,
    @location(6) @interpolate(flat) shape: u32,
};

@vertex
fn vs_main(@location(0) corner: vec2<f32>, inst: Instance) -> VsOut {
    var out: VsOut;
    let pos_px = inst.bounds.xy + corner * inst.bounds.zw;
    // Pixel space -> normalized device coordinates (flip Y).
    let ndc = vec2<f32>(
        pos_px.x / screen.resolution.x * 2.0 - 1.0,
        1.0 - pos_px.y / screen.resolution.y * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.local = corner * inst.bounds.zw;
    out.size = inst.bounds.zw;
    out.color = inst.color;
    out.border = inst.border;
    out.params = inst.params;
    out.arc = inst.arc;
    out.shape = inst.shape.x;
    return out;
}

// Signed distance to a rounded box centered at the origin with half-size `b`.
fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

const PI: f32 = 3.14159265;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let center = in.size * 0.5;
    let p = in.local - center;

    if (in.shape == 0u) {
        // Rounded rectangle with optional hairline border.
        let radius = in.params.x;
        let bw = in.params.y;
        let d = sd_rounded_box(p, center, radius);
        // Antialias over ~1px.
        let aa = 1.0;
        let fill_a = clamp(0.5 - d / aa, 0.0, 1.0);
        var col = in.color;
        if (bw > 0.0) {
            let edge = clamp(0.5 - (abs(d + bw * 0.5) - bw * 0.5) / aa, 0.0, 1.0);
            col = mix(col, in.border, edge * in.border.a);
        }
        return vec4<f32>(col.rgb, col.a * fill_a);
    } else if (in.shape == 1u) {
        // Soft radial glow.
        let radius = in.params.x;
        let intensity = in.params.w;
        let dist = length(p);
        let t = clamp(1.0 - dist / radius, 0.0, 1.0);
        let falloff = pow(t, 2.0) * intensity;
        return vec4<f32>(in.color.rgb, clamp(falloff, 0.0, 1.0) * in.color.a);
    } else {
        // Ring / arc segment for the control wheel.
        let radius = in.params.x;
        let start = in.arc.x;
        let sweep = in.arc.y;
        let thickness = in.arc.z;
        let dist = length(p);
        let ring = 1.0 - clamp(abs(dist - radius) / max(thickness, 0.5), 0.0, 1.0);
        // Angle of this fragment, normalized to 0..2pi from `start`.
        var ang = atan2(p.y, p.x) - start;
        ang = ang - floor(ang / (2.0 * PI)) * (2.0 * PI);
        let in_sweep = select(0.0, 1.0, ang <= sweep);
        return vec4<f32>(in.color.rgb, in.color.a * ring * in_sweep);
    }
}

// Rectangles colorés instanciés : fonds de cellule, curseur, bordures, barre
// d'onglets, et formes stylées du chrome (coins arrondis, bordure, dégradé
// vertical). Coordonnées en pixels, origine en haut à gauche.

struct Globals {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

struct Instance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) color_bottom: vec4<f32>,
    @location(4) border_color: vec4<f32>,
    @location(5) radius: f32,
    @location(6) border_width: f32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) color_bottom: vec4<f32>,
    @location(2) border_color: vec4<f32>,
    @location(3) rect_pos: vec2<f32>,
    @location(4) rect_size: vec2<f32>,
    @location(5) radius: f32,
    @location(6) border_width: f32,
};

fn corner(vi: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    return corners[vi];
}

fn to_clip(px: vec2<f32>) -> vec4<f32> {
    let ndc = vec2<f32>(px.x / globals.viewport.x * 2.0 - 1.0, 1.0 - px.y / globals.viewport.y * 2.0);
    return vec4<f32>(ndc, 0.0, 1.0);
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    var out: VsOut;
    out.clip = to_clip(inst.pos + corner(vi) * inst.size);
    out.color = inst.color;
    out.color_bottom = inst.color_bottom;
    out.border_color = inst.border_color;
    out.rect_pos = inst.pos;
    out.rect_size = inst.size;
    out.radius = inst.radius;
    out.border_width = inst.border_width;
    return out;
}

// Distance signée au rectangle arrondi centré en 0, de demi-taille `half`.
fn rounded_box(p: vec2<f32>, half: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let half = in.rect_size / 2.0;
    let p = in.clip.xy - (in.rect_pos + half);
    let d = rounded_box(p, half, in.radius);
    let coverage = 1.0 - smoothstep(-0.5, 0.5, d);
    let t = clamp((in.clip.y - in.rect_pos.y) / max(in.rect_size.y, 1.0), 0.0, 1.0);
    var color = mix(in.color, in.color_bottom, t);
    if in.border_width > 0.0 && d >= -in.border_width {
        color = in.border_color;
    }
    return vec4<f32>(color.rgb, color.a * coverage);
}

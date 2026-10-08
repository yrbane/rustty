// Quads texturés par l'atlas de glyphes. Monochrome : la texture donne la
// couverture (alpha), l'instance la couleur. Couleur (emoji) : la texture
// donne tout, l'instance ne module que l'alpha.

struct Globals {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_samp: sampler;

struct Instance {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) flags: u32,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) flags: u32,
};

fn corner(vi: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    return corners[vi];
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    let c = corner(vi);
    let px = inst.pos + c * inst.size;
    let ndc = vec2<f32>(px.x / globals.viewport.x * 2.0 - 1.0, 1.0 - px.y / globals.viewport.y * 2.0);
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = mix(inst.uv.xy, inst.uv.zw, c);
    out.color = inst.color;
    out.flags = inst.flags;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let t = textureSample(atlas_tex, atlas_samp, in.uv);
    if ((in.flags & 1u) != 0u) {
        return vec4<f32>(t.rgb, t.a * in.color.a);
    }
    return vec4<f32>(in.color.rgb, t.a * in.color.a);
}

struct EffectFrame {
    time: f32,
    delta: f32,
    frame: u32,
    pad: u32,
    viewport: vec4<f32>,
}

struct EffectQuad {
    bounds: vec4<f32>,
    uv: vec4<f32>,
    params: array<vec4<f32>, 16>,
    effect_meta: vec4<f32>,
    flags: vec4<u32>,
}

@group(1) @binding(0) var<uniform> effect_frame: EffectFrame;
@group(1) @binding(1) var<storage, read> effect_quads: array<EffectQuad>;
@group(1) @binding(2) var effect_backdrop: texture_2d<f32>;
@group(1) @binding(3) var effect_history: texture_2d<f32>;
@group(1) @binding(4) var effect_sampler: sampler;

struct EffectVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) index: u32,
}

struct EffectInput {
    uv: vec2<f32>,
    local: vec2<f32>,
    px: vec2<f32>,
    size: vec2<f32>,
    time: f32,
    scale: f32,
    opacity: f32,
    coverage: f32,
    backdrop: vec4<f32>,
    history: vec4<f32>,
    quad: EffectQuad,
}

const EFFECT_CORNERS = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
);

fn effect_param(input: EffectInput, slot: u32) -> vec4<f32> {
    return input.quad.params[min(slot, 15u)];
}

fn effect_backdrop_at(input: EffectInput, uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(effect_backdrop, effect_sampler, uv, 0.0);
}

fn effect_history_size() -> vec2<f32> {
    return vec2<f32>(textureDimensions(effect_history));
}

fn effect_history_at(input: EffectInput, px: vec2<f32>) -> vec4<f32> {
    let uv = (px + vec2<f32>(input.quad.effect_meta.w)) / effect_history_size();
    return textureSampleLevel(effect_history, effect_sampler, uv, 0.0);
}

fn effect_aa(distance: f32) -> f32 {
    return clamp(0.5 - distance, 0.0, 1.0);
}

fn effect_rotate(value: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec2<f32>(c * value.x - s * value.y, s * value.x + c * value.y);
}

fn effect_make_input(in: EffectVaryings, quad: EffectQuad) -> EffectInput {
    var input: EffectInput;
    input.uv = in.uv;
    input.local = in.uv * quad.bounds.zw;
    input.px = quad.bounds.xy + input.local;
    input.size = quad.bounds.zw;
    input.time = quad.effect_meta.x;
    input.scale = quad.effect_meta.y;
    input.opacity = quad.effect_meta.z;
    input.coverage = 1.0;
    input.quad = quad;
    input.backdrop = textureSampleLevel(effect_backdrop, effect_sampler, in.uv, 0.0);
    input.history = effect_history_at(input, input.px);
    return input;
}

@vertex
fn effect_vs(
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32,
) -> EffectVaryings {
    let quad = effect_quads[instance];
    let corner = EFFECT_CORNERS[vertex];
    let pixel = quad.bounds.xy + corner * quad.bounds.zw;
    let clip = vec2<f32>(
        pixel.x / effect_frame.viewport.x * 2.0 - 1.0,
        1.0 - pixel.y / effect_frame.viewport.y * 2.0,
    );
    var out: EffectVaryings;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.uv = mix(quad.uv.xy, quad.uv.zw, corner);
    out.index = instance;
    return out;
}

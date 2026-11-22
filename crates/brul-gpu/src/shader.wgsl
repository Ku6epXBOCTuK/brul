struct Viewport {
    size: vec2<f32>,
}

struct Instance {
    bounds: vec4<f32>,
    params: vec4<f32>,
    fill: vec4<f32>,
    border_widths: vec4<f32>,
    border_left: vec4<f32>,
    border_top: vec4<f32>,
    border_right: vec4<f32>,
    border_bottom: vec4<f32>,
    start: vec4<f32>,
    end: vec4<f32>,
    extra: vec4<f32>,
    padding: vec4<f32>,
}

struct QuadInstance {
    bounds: vec4<f32>,
    uv: vec4<f32>,
    tint: vec4<f32>,
    params: vec4<f32>,
}

struct ClipInstance {
    bounds: vec4<f32>,
    radius: vec4<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(0) @binding(1) var<storage, read> instances: array<Instance>;
@group(0) @binding(2) var<storage, read> clips: array<ClipInstance>;
@group(0) @binding(4) var<storage, read> quads: array<QuadInstance>;
@group(0) @binding(5) var quad_texture: texture_2d<f32>;
@group(0) @binding(6) var quad_sampler: sampler;

struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) @interpolate(flat) index: u32,
}

const CORNERS = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
);

@vertex
fn vs(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Varyings {
    let shape = instances[instance];
    let corner = CORNERS[vertex];
    let padding = shape.padding.x;
    let offset = select(vec2<f32>(0.0), shape.extra.yz, shape.extra.w >= 1.5);
    let local = corner * (shape.bounds.zw + vec2<f32>(2.0 * padding)) - vec2<f32>(padding);
    let pixel = shape.bounds.xy + local + offset;
    let clip = vec2<f32>(
        pixel.x / viewport.size.x * 2.0 - 1.0,
        1.0 - pixel.y / viewport.size.y * 2.0,
    );
    var out: Varyings;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.local = local;
    out.index = instance;
    return out;
}

fn rounded_distance(local: vec2<f32>, size: vec2<f32>, radius: f32, offset: vec2<f32>) -> f32 {
    let r = min(radius, min(size.x, size.y) * 0.5);
    let half = size * 0.5;
    let q = abs(local - offset - half) - (half - vec2<f32>(r));
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn clip_coverage(pixel: vec2<f32>, range: vec2<f32>) -> f32 {
    var coverage = 1.0;
    let start = u32(range.x);
    let count = u32(range.y);
    for (var i = 0u; i < count; i = i + 1u) {
        let clip = clips[start + i];
        let d = rounded_distance(
            pixel - clip.bounds.xy,
            clip.bounds.zw,
            clip.radius.x,
            vec2<f32>(0.0),
        );
        coverage = coverage * clamp(0.5 - d, 0.0, 1.0);
    }
    return coverage;
}

fn srgb_from_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, low, c <= vec3<f32>(0.0031308));
}

fn yuv_to_rgb(yuv: vec3<f32>) -> vec3<f32> {
    let luma = (yuv.r - 16.0 / 255.0) * (255.0 / 219.0);
    let blue = (yuv.g - 128.0 / 255.0) * (255.0 / 224.0);
    let red = (yuv.b - 128.0 / 255.0) * (255.0 / 224.0);
    return vec3<f32>(
        luma + 1.5748 * red,
        luma - 0.1873 * blue - 0.4681 * red,
        luma + 1.8556 * blue,
    );
}

fn shadow_coverage(d: f32, blur: f32) -> f32 {
    if blur <= 0.0 {
        return select(0.0, 1.0, d <= 0.0);
    }
    let t = clamp(0.5 - d / (2.0 * blur), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

@fragment
fn fs(in: Varyings) -> @location(0) vec4<f32> {
    let shape = instances[in.index];
    let size = shape.bounds.zw;
    let opacity = shape.params.w;
    var premultiplied = vec3<f32>(0.0);
    var alpha = 0.0;

    if shape.extra.w >= 1.5 {
        let d = rounded_distance(in.local, size, shape.params.x, shape.extra.yz);
        let coverage = shadow_coverage(d, shape.params.z) * shape.fill.a;
        premultiplied = shape.fill.rgb * coverage;
        alpha = coverage;
    } else {
        let widths = shape.border_widths;
        let d = rounded_distance(in.local, size, shape.params.x, vec2<f32>(0.0));
        let outer = clamp(0.5 - d, 0.0, 1.0);
        let inner_size = max(
            size - vec2<f32>(widths.x + widths.z, widths.y + widths.w),
            vec2<f32>(0.0),
        );
        let inner_radius = max(
            shape.params.x - max(max(widths.x, widths.y), max(widths.z, widths.w)),
            0.0,
        );
        var inner = 0.0;
        if inner_size.x > 0.0 && inner_size.y > 0.0 {
            let inner_d = rounded_distance(in.local, inner_size, inner_radius, widths.xy);
            inner = clamp(0.5 - inner_d, 0.0, 1.0);
        }
        var color_sum = vec4<f32>(0.0);
        var color_count = 0.0;
        if in.local.x < widths.x {
            color_sum = color_sum + shape.border_left;
            color_count = color_count + 1.0;
        }
        if in.local.y < widths.y {
            color_sum = color_sum + shape.border_top;
            color_count = color_count + 1.0;
        }
        if in.local.x > size.x - widths.z {
            color_sum = color_sum + shape.border_right;
            color_count = color_count + 1.0;
        }
        if in.local.y > size.y - widths.w {
            color_sum = color_sum + shape.border_bottom;
            color_count = color_count + 1.0;
        }
        var border_color = shape.border_top;
        if color_count > 0.0 {
            border_color = color_sum / color_count;
        }
        let border = (outer - inner) * border_color.a;
        premultiplied = border_color.rgb * border;
        alpha = border;

        var fill = shape.fill;
        if shape.extra.x > 0.5 {
            let angle = radians(shape.extra.x);
            let sine = sin(angle);
            let cosine = cos(angle);
            let span = max(abs(size.x * sine) + abs(size.y * cosine), 0.0001);
            let center = in.local - size * 0.5;
            let t = clamp((center.x * sine - center.y * cosine) / span + 0.5, 0.0, 1.0);
            fill = mix(shape.start, shape.end, t);
        }
        let coverage = inner * fill.a;
        premultiplied = fill.rgb * coverage + premultiplied * (1.0 - coverage);
        alpha = coverage + alpha * (1.0 - coverage);
    }

    let clip = clip_coverage(in.position.xy, shape.padding.yz);
    alpha *= opacity * clip;
    if alpha <= 0.0 {
        discard;
    }
    return vec4<f32>(premultiplied * opacity * clip, alpha);
}

struct QuadVaryings {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) tint: vec4<f32>,
    @location(2) @interpolate(flat) params: vec4<f32>,
}

@vertex
fn vs_quad(
    @builtin(vertex_index) vertex: u32,
    @builtin(instance_index) instance: u32,
) -> QuadVaryings {
    let quad = quads[instance];
    let corner = CORNERS[vertex];
    let pixel = quad.bounds.xy + corner * quad.bounds.zw;
    let clip = vec2<f32>(
        pixel.x / viewport.size.x * 2.0 - 1.0,
        1.0 - pixel.y / viewport.size.y * 2.0,
    );
    var out: QuadVaryings;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    out.uv = mix(quad.uv.xy, quad.uv.zw, corner);
    out.tint = quad.tint;
    out.params = quad.params;
    return out;
}

@fragment
fn fs_quad(in: QuadVaryings) -> @location(0) vec4<f32> {
    let sampled = textureSample(quad_texture, quad_sampler, in.uv);
    let raw = srgb_from_linear(sampled.rgb);
    var rgb: vec3<f32>;
    if in.params.x < 0.5 {
        rgb = in.tint.rgb;
    } else if in.params.x < 1.5 {
        rgb = raw * in.tint.rgb;
    } else {
        rgb = yuv_to_rgb(raw) * in.tint.rgb;
    }
    let alpha = sampled.a
        * in.tint.a
        * in.params.y
        * clip_coverage(in.position.xy, in.params.zw);
    if alpha <= 0.0 {
        discard;
    }
    return vec4<f32>(rgb * alpha, alpha);
}

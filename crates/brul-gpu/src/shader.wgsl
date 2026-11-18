struct Viewport {
    size: vec2<f32>,
}

struct Instance {
    bounds: vec4<f32>,
    params: vec4<f32>,
    fill: vec4<f32>,
    border: vec4<f32>,
    start: vec4<f32>,
    end: vec4<f32>,
    extra: vec4<f32>,
    padding: vec4<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(0) @binding(1) var<storage, read> instances: array<Instance>;

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
        let coverage = shadow_coverage(d, shape.params.z);
        premultiplied = shape.fill.rgb * coverage;
        alpha = coverage;
    } else {
        let d = rounded_distance(in.local, size, shape.params.x, vec2<f32>(0.0));
        let outer = clamp(0.5 - d, 0.0, 1.0);
        let inner = clamp(0.5 - (d + shape.params.y), 0.0, 1.0);
        let border = outer - inner;
        premultiplied = shape.border.rgb * border;
        alpha = border;

        var fill = shape.fill.rgb;
        if shape.extra.x > 0.5 {
            let angle = radians(shape.extra.x);
            let sine = sin(angle);
            let cosine = cos(angle);
            let span = max(abs(size.x * sine) + abs(size.y * cosine), 0.0001);
            let center = in.local - size * 0.5;
            let t = clamp((center.x * sine - center.y * cosine) / span + 0.5, 0.0, 1.0);
            fill = mix(shape.start.rgb, shape.end.rgb, t);
        }
        premultiplied = fill * inner + premultiplied * (1.0 - inner);
        alpha = inner + alpha * (1.0 - inner);
    }

    alpha *= opacity;
    if alpha <= 0.0 {
        discard;
    }
    return vec4<f32>(premultiplied * opacity, alpha);
}

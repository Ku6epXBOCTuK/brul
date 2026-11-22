fn shade(input: EffectInput) -> vec4<f32> {
    let phase = effect_param(input, 0u).x;
    let tint = effect_param(input, 1u);
    let glow = effect_param(input, 2u);

    let t = input.time + phase;
    let x = input.uv.x;
    let y = input.uv.y;
    let wave = sin(x * glow.x + t) * glow.y;
    let center = 0.5 + wave * 0.18;
    let curtain = 1.0 - smoothstep(0.0, glow.z, abs(y - center));
    let shimmer = 0.75 + 0.25 * sin(x * 24.0 + t * 2.0);
    let alpha = curtain * shimmer * tint.a * input.coverage * input.opacity;
    let color = mix(tint.rgb, vec3<f32>(1.0), curtain * glow.w * 0.5);
    return vec4<f32>(color, alpha);
}

use brul_gpu::effect::{
    EFFECT_FRAME_SIZE, EFFECT_INSTANCE_SIZE, EffectError, EffectFrame, EffectInstance, compile,
    effect_frame_bytes, effect_instance_bytes,
};

const SHADE: &str = "
fn shade(input: EffectInput) -> vec4<f32> {
    let tint = effect_param(input, 0u);
    let wave = sin(input.uv.x * 8.0 + input.time) * 0.5 + 0.5;
    return vec4<f32>(tint.rgb * wave, tint.a * input.opacity);
}
";

#[test]
fn a_shader_with_shade_compiles_into_a_full_module() {
    let shader = compile(SHADE, "cover.wgsl").expect("shader compiles");
    assert!(!shader.has_advance());
    assert!(shader.source().contains("fn effect_fs"));
    assert!(!shader.source().contains("fn effect_advance_fs"));
}

#[test]
fn a_shader_with_advance_gets_a_history_pass() {
    let source = format!(
        "{SHADE}\nfn advance(input: EffectInput) -> vec4<f32> {{\n    return mix(input.history, input.backdrop, 0.25);\n}}"
    );
    let shader = compile(&source, "spectrum.wgsl").expect("shader compiles");
    assert!(shader.has_advance());
    assert!(shader.source().contains("fn effect_advance_fs"));
}

#[test]
fn a_missing_shade_entry_point_is_reported() {
    let error = compile("fn other() -> f32 { return 1.0; }", "bad.wgsl")
        .expect_err("shader without shade fails");
    assert!(
        matches!(error, EffectError::MissingShade { .. }),
        "{error:?}"
    );
}

#[test]
fn a_syntax_error_points_at_the_user_source_line() {
    let error = compile(
        "fn shade(input: EffectInput) -> vec4<f32> {\n    let = 1.0;\n    return input.backdrop;\n}",
        "bad.wgsl",
    )
    .expect_err("broken shader fails");
    match error {
        EffectError::Parse { line, .. } => assert_eq!(line, 2),
        other => panic!("expected parse error, got {other:?}"),
    }
}

#[test]
fn a_wrong_shade_signature_is_rejected_by_validation() {
    let error = compile(
        "fn shade(input: EffectInput) -> f32 { return 1.0; }",
        "bad.wgsl",
    )
    .expect_err("wrong signature fails");
    let message = match error {
        EffectError::Parse { message, .. } | EffectError::Validate { message, .. } => message,
        other => panic!("expected shader error, got {other:?}"),
    };
    assert!(message.contains("rgb"), "{message}");
}

#[test]
fn uniforms_and_instances_have_the_declared_layout() {
    assert_eq!(EFFECT_FRAME_SIZE, 32);
    assert_eq!(EFFECT_INSTANCE_SIZE, 320);

    let frame = EffectFrame {
        time: 1.5,
        delta: 0.25,
        frame: 7,
        pad: 0,
        viewport: [800.0, 600.0, 0.0, 0.0],
    };
    let bytes = effect_frame_bytes(&frame);
    assert_eq!(bytes.len(), EFFECT_FRAME_SIZE as usize);
    assert_eq!(f32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 1.5);
    assert_eq!(u32::from_ne_bytes(bytes[8..12].try_into().unwrap()), 7);

    let instance = EffectInstance {
        bounds: [10.0, 20.0, 100.0, 50.0],
        uv: [0.0, 0.0, 1.0, 1.0],
        params: [[1.0, 0.0, 0.0, 1.0]; 16],
        meta: [2.0, 1.0, 0.5, 2.0],
        flags: [0; 4],
    };
    let bytes = effect_instance_bytes(&[instance]);
    assert_eq!(bytes.len(), EFFECT_INSTANCE_SIZE as usize);
    assert_eq!(f32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 10.0);
}

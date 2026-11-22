use brul::effect::{EffectDef, EffectField, EffectParams, EffectType, Effects};

brul::effect! {
    name: "cover",
    shader: "effects/cover.wgsl",
    pub struct CoverParams {
        pub tint: [f32; 4],
        pub speed: f32,
        pub bands: [[f32; 4]; 3],
    }
}

#[test]
fn effect_is_bound_to_a_shader_by_name() {
    assert_eq!(<CoverParams as EffectParams>::NAME, "cover");
    assert!(<CoverParams as EffectParams>::SHADER.contains("fn shade"));
    assert_eq!(<CoverParams as EffectParams>::SLOTS, 5);
    assert_eq!(BRUL_EFFECT_COVER.name, "cover");
    assert_eq!(BRUL_EFFECT_COVER.slots, 5);
}

#[test]
fn effect_fields_describe_the_uniform_layout() {
    assert_eq!(
        <CoverParams as EffectParams>::FIELDS,
        &[
            EffectField {
                name: "tint",
                ty: EffectType::Float4,
                slots: 1,
            },
            EffectField {
                name: "speed",
                ty: EffectType::Float,
                slots: 1,
            },
            EffectField {
                name: "bands",
                ty: EffectType::Float4Array,
                slots: 3,
            },
        ]
    );
}

#[test]
fn params_are_packed_into_uniform_slots() {
    let params = CoverParams {
        tint: [0.1, 0.2, 0.3, 0.4],
        speed: 2.5,
        bands: [[1.0; 4]; 3],
    };
    let slots = params.pack();
    assert_eq!(slots[0], params.tint);
    assert_eq!(slots[1][0], params.speed);
    assert_eq!(slots[2], params.bands[0]);
    assert_eq!(slots[4], params.bands[2]);
    assert_eq!(slots[5], [0.0; 4]);
}

#[test]
fn a_named_effect_compiles_and_can_be_looked_up() {
    let shader = BRUL_EFFECT_COVER.compile().expect("valid effect shader");
    assert!(!shader.source().is_empty());
    const EFFECTS: &[EffectDef] = &[BRUL_EFFECT_COVER];
    let effects = Effects::new(EFFECTS);
    assert!(effects.get("cover").is_some());
    assert!(effects.get("missing").is_none());
    assert_eq!(effects.iter().count(), 1);
}

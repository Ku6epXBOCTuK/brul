pub const EFFECT_SLOTS: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct EffectSpec {
    pub name: &'static str,
    pub params: [[f32; 4]; EFFECT_SLOTS],
    pub animated: bool,
}

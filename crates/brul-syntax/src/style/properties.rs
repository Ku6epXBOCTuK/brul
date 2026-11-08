use crate::style::value::Type;

#[derive(Clone, Copy)]
pub enum StatePolicy {
    Forbidden,
    Visual,
    TextWeight,
}

#[derive(Clone, Copy)]
pub struct Spec {
    pub name: &'static str,
    pub ty: Type,
    pub in_state: StatePolicy,
    pub themeable: bool,
}

pub static SPECS: &[Spec] = &[
    Spec::new("width", Type::Length, StatePolicy::Forbidden, true),
    Spec::new("height", Type::Length, StatePolicy::Forbidden, true),
    Spec::new("stretch", Type::Number, StatePolicy::Forbidden, true),
    Spec::new("padding", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("padding-left", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("padding-right", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("padding-top", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("padding-bottom", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("spacing", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("border-width", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("radius", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("font-size", Type::Px, StatePolicy::Forbidden, true),
    Spec::new("font-family", Type::String, StatePolicy::Forbidden, false),
    Spec::new(
        "justify-content",
        Type::Keywords(&["start", "center", "end", "space-between"]),
        StatePolicy::Forbidden,
        false,
    ),
    Spec::new(
        "align-items",
        Type::Keywords(&["start", "center", "end", "stretch"]),
        StatePolicy::Forbidden,
        false,
    ),
    Spec::new("background", Type::Paint, StatePolicy::Visual, true),
    Spec::new("border", Type::Border, StatePolicy::Visual, false),
    Spec::new("border-color", Type::Color, StatePolicy::Visual, true),
    Spec::new("color", Type::Color, StatePolicy::Visual, true),
    Spec::new("tint", Type::Color, StatePolicy::Visual, true),
    Spec::new(
        "text-align",
        Type::Keywords(&["left", "center", "right"]),
        StatePolicy::Visual,
        false,
    ),
    Spec::new(
        "vertical-align",
        Type::Keywords(&["top", "center", "bottom"]),
        StatePolicy::Visual,
        false,
    ),
    Spec::new(
        "font-weight",
        Type::NumberIn(&[400.0, 500.0, 600.0, 700.0]),
        StatePolicy::TextWeight,
        false,
    ),
    Spec::new(
        "opacity",
        Type::NumberRange(0.0, 1.0),
        StatePolicy::Visual,
        false,
    ),
];

impl Spec {
    const fn new(name: &'static str, ty: Type, in_state: StatePolicy, themeable: bool) -> Spec {
        Spec {
            name,
            ty,
            in_state,
            themeable,
        }
    }
}

pub fn get(name: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|spec| spec.name == name)
}

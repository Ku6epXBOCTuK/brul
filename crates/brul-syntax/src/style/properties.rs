use crate::style::value::Type;

#[derive(Clone, Copy)]
pub enum StatePolicy {
    Forbidden,
    Visual,
    TextWeight,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeType {
    Color,
    Px,
}

impl ThemeType {
    pub fn name(self) -> &'static str {
        match self {
            ThemeType::Color => "a color",
            ThemeType::Px => "a px length",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Spec {
    pub name: &'static str,
    pub ty: Type,
    pub in_state: StatePolicy,
}

pub static SPECS: &[Spec] = &[
    Spec::new("width", Type::Length, StatePolicy::Forbidden),
    Spec::new("height", Type::Length, StatePolicy::Forbidden),
    Spec::new("stretch", Type::Number, StatePolicy::Forbidden),
    Spec::new("padding", Type::Px, StatePolicy::Forbidden),
    Spec::new("padding-left", Type::Px, StatePolicy::Forbidden),
    Spec::new("padding-right", Type::Px, StatePolicy::Forbidden),
    Spec::new("padding-top", Type::Px, StatePolicy::Forbidden),
    Spec::new("padding-bottom", Type::Px, StatePolicy::Forbidden),
    Spec::new("spacing", Type::Px, StatePolicy::Forbidden),
    Spec::new("border-width", Type::Px, StatePolicy::Forbidden),
    Spec::new("radius", Type::Px, StatePolicy::Forbidden),
    Spec::new("font-size", Type::Px, StatePolicy::Forbidden),
    Spec::new("font-family", Type::String, StatePolicy::Forbidden),
    Spec::new(
        "justify-content",
        Type::Keywords(&["start", "center", "end", "space-between"]),
        StatePolicy::Forbidden,
    ),
    Spec::new(
        "align-items",
        Type::Keywords(&["start", "center", "end", "stretch"]),
        StatePolicy::Forbidden,
    ),
    Spec::new("background", Type::Paint, StatePolicy::Visual),
    Spec::new("border", Type::Border, StatePolicy::Visual),
    Spec::new("border-color", Type::Color, StatePolicy::Visual),
    Spec::new("color", Type::Color, StatePolicy::Visual),
    Spec::new("tint", Type::Color, StatePolicy::Visual),
    Spec::new(
        "text-align",
        Type::Keywords(&["left", "center", "right"]),
        StatePolicy::Visual,
    ),
    Spec::new(
        "vertical-align",
        Type::Keywords(&["top", "center", "bottom"]),
        StatePolicy::Visual,
    ),
    Spec::new(
        "font-weight",
        Type::NumberIn(&[400.0, 500.0, 600.0, 700.0]),
        StatePolicy::TextWeight,
    ),
    Spec::new("opacity", Type::NumberRange(0.0, 1.0), StatePolicy::Visual),
];

impl Spec {
    const fn new(name: &'static str, ty: Type, in_state: StatePolicy) -> Spec {
        Spec { name, ty, in_state }
    }

    pub fn theme_type(&self) -> Option<ThemeType> {
        match self.ty {
            Type::Px | Type::Length => Some(ThemeType::Px),
            Type::Color | Type::Paint => Some(ThemeType::Color),
            _ => None,
        }
    }
}

pub fn get(name: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|spec| spec.name == name)
}

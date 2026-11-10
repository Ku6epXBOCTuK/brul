use self::StatePolicy::{Forbidden, TextWeight, Visual};
use crate::style::value::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Width,
    Height,
    Stretch,
    PaddingSide(Side),
    Spacing,
    BorderWidth,
    Radius,
    FontSize,
    FontFamily,
    JustifyContent,
    AlignItems,
    Background,
    BorderColor,
    Color,
    Tint,
    TextAlign,
    VerticalAlign,
    FontWeight,
    Opacity,
}

impl Kind {
    pub fn spec(self) -> &'static Spec {
        SPECS
            .iter()
            .find(|spec| spec.kind == self)
            .expect("SPECS lists every kind")
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn from_name(name: &str) -> Option<Kind> {
        SPECS
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.kind)
    }

    pub fn theme_type(self) -> Option<ThemeType> {
        self.spec().theme_type()
    }

    pub fn in_state(self) -> StatePolicy {
        self.spec().in_state
    }
}

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
    pub kind: Kind,
    pub name: &'static str,
    pub ty: Type,
    pub in_state: StatePolicy,
}

pub static SPECS: &[Spec] = &[
    Spec::new(Kind::Width, "width", Type::Length, Forbidden),
    Spec::new(Kind::Height, "height", Type::Length, Forbidden),
    Spec::new(Kind::Stretch, "stretch", Type::Number, Forbidden),
    Spec::new(
        Kind::PaddingSide(Side::Left),
        "padding-left",
        Type::Px,
        Forbidden,
    ),
    Spec::new(
        Kind::PaddingSide(Side::Right),
        "padding-right",
        Type::Px,
        Forbidden,
    ),
    Spec::new(
        Kind::PaddingSide(Side::Top),
        "padding-top",
        Type::Px,
        Forbidden,
    ),
    Spec::new(
        Kind::PaddingSide(Side::Bottom),
        "padding-bottom",
        Type::Px,
        Forbidden,
    ),
    Spec::new(Kind::Spacing, "spacing", Type::Px, Forbidden),
    Spec::new(Kind::BorderWidth, "border-width", Type::Px, Forbidden),
    Spec::new(Kind::Radius, "radius", Type::Px, Forbidden),
    Spec::new(Kind::FontSize, "font-size", Type::Px, Forbidden),
    Spec::new(Kind::FontFamily, "font-family", Type::String, Forbidden),
    Spec::new(
        Kind::JustifyContent,
        "justify-content",
        Type::Keywords(&["start", "center", "end", "space-between"]),
        Forbidden,
    ),
    Spec::new(
        Kind::AlignItems,
        "align-items",
        Type::Keywords(&["start", "center", "end", "stretch"]),
        Forbidden,
    ),
    Spec::new(Kind::Background, "background", Type::Paint, Visual),
    Spec::new(Kind::BorderColor, "border-color", Type::Color, Visual),
    Spec::new(Kind::Color, "color", Type::Color, Visual),
    Spec::new(Kind::Tint, "tint", Type::Color, Visual),
    Spec::new(
        Kind::TextAlign,
        "text-align",
        Type::Keywords(&["left", "center", "right"]),
        Visual,
    ),
    Spec::new(
        Kind::VerticalAlign,
        "vertical-align",
        Type::Keywords(&["top", "center", "bottom"]),
        Visual,
    ),
    Spec::new(
        Kind::FontWeight,
        "font-weight",
        Type::NumberIn(&[400.0, 500.0, 600.0, 700.0]),
        TextWeight,
    ),
    Spec::new(
        Kind::Opacity,
        "opacity",
        Type::NumberRange(0.0, 1.0),
        Visual,
    ),
];

impl Spec {
    const fn new(kind: Kind, name: &'static str, ty: Type, in_state: StatePolicy) -> Spec {
        Spec {
            kind,
            name,
            ty,
            in_state,
        }
    }

    pub fn theme_type(&self) -> Option<ThemeType> {
        match self.ty {
            Type::Px | Type::Length => Some(ThemeType::Px),
            Type::Color | Type::Paint => Some(ThemeType::Color),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, SPECS, Side};

    #[test]
    fn every_kind_has_its_own_name_and_spec() {
        let mut names: Vec<&str> = SPECS.iter().map(|spec| spec.name).collect();
        for spec in SPECS {
            assert_eq!(spec.kind.name(), spec.name);
            assert_eq!(Kind::from_name(spec.name), Some(spec.kind));
        }
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SPECS.len());
    }

    #[test]
    fn every_side_has_its_own_property() {
        let sides = [
            (Side::Left, "padding-left"),
            (Side::Top, "padding-top"),
            (Side::Right, "padding-right"),
            (Side::Bottom, "padding-bottom"),
        ];
        for (side, name) in sides {
            assert_eq!(Kind::from_name(name), Some(Kind::PaddingSide(side)));
            assert_eq!(Kind::PaddingSide(side).name(), name);
        }
    }
}

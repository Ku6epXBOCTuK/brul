use crate::ty::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Layout(LayoutField),
    Style(StyleField),
    Text(TextField),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutField {
    Width,
    Height,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    Display,
    FlexDirection,
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridAutoFlow,
    GridTemplateAreas,
    GridArea,
    Padding(Side),
    Gap,
    RowGap,
    ColumnGap,
    BorderWidthSide(Side),
    Justify,
    Align,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleField {
    Background,
    BorderSide(Side),
    Text,
    Tint,
    BorderRadius,
    Opacity,
    Clip,
    Scroll,
    ShadowX,
    ShadowY,
    ShadowBlur,
    ShadowColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextField {
    Size,
    Family,
    Align,
    VerticalAlign,
    Weight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Width,
    Height,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    Display,
    FlexDirection,
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridAutoFlow,
    GridTemplateAreas,
    GridArea,
    PaddingSide(Side),
    Gap,
    RowGap,
    ColumnGap,
    BorderWidthSide(Side),
    BorderRadius,
    FontSize,
    FontFamily,
    JustifyContent,
    AlignItems,
    Background,
    BorderColorSide(Side),
    Color,
    Tint,
    TextAlign,
    VerticalAlign,
    FontWeight,
    Opacity,
    Clip,
    Scroll,
    ShadowOffsetX,
    ShadowOffsetY,
    ShadowBlur,
    ShadowColor,
}

macro_rules! target {
    (layout, $field:ident($side:expr)) => {
        Target::Layout(LayoutField::$field($side))
    };
    (layout, $field:ident) => {
        Target::Layout(LayoutField::$field)
    };
    (style, $field:ident($side:expr)) => {
        Target::Style(StyleField::$field($side))
    };
    (style, $field:ident) => {
        Target::Style(StyleField::$field)
    };
    (text, $field:ident) => {
        Target::Text(TextField::$field)
    };
}

macro_rules! properties {
    ($($kind:expr => ($name:literal, $type:expr, $policy:ident, $target:ident ( $($target_arg:tt)* ))),+ $(,)?) => {
        pub static SPECS: &[Spec] = &[
            $(Spec {
                kind: $kind,
                name: $name,
                ty: $type,
                in_state: StatePolicy::$policy,
                target: target!($target, $($target_arg)*),
            }),+
        ];

        impl Kind {
            pub const ALL: &'static [Kind] = &[$($kind),+];
        }
    };
}

properties! {
    Kind::Width => ("width", Type::Length, Forbidden, layout(Width)),
    Kind::Height => ("height", Type::Length, Forbidden, layout(Height)),
    Kind::FlexGrow => ("flex-grow", Type::Number, Forbidden, layout(FlexGrow)),
    Kind::FlexShrink => ("flex-shrink", Type::Number, Forbidden, layout(FlexShrink)),
    Kind::FlexBasis => ("flex-basis", Type::Length, Forbidden, layout(FlexBasis)),
    Kind::Display => ("display", Type::Keywords(&["flex", "grid"]), Forbidden, layout(Display)),
    Kind::FlexDirection => ("flex-direction", Type::Keywords(&["row", "column"]), Forbidden, layout(FlexDirection)),
    Kind::GridTemplateColumns => ("grid-template-columns", Type::TrackList, Forbidden, layout(GridTemplateColumns)),
    Kind::GridTemplateRows => ("grid-template-rows", Type::TrackList, Forbidden, layout(GridTemplateRows)),
    Kind::GridColumn => ("grid-column", Type::GridLine, Forbidden, layout(GridColumn)),
    Kind::GridRow => ("grid-row", Type::GridLine, Forbidden, layout(GridRow)),
    Kind::GridAutoFlow => ("grid-auto-flow", Type::Keywords(&["row", "column"]), Forbidden, layout(GridAutoFlow)),
    Kind::GridTemplateAreas => ("grid-template-areas", Type::AreaTemplate, Forbidden, layout(GridTemplateAreas)),
    Kind::GridArea => ("grid-area", Type::GridArea, Forbidden, layout(GridArea)),
    Kind::PaddingSide(Side::Left) => ("padding-left", Type::Px, Forbidden, layout(Padding(Side::Left))),
    Kind::PaddingSide(Side::Right) => ("padding-right", Type::Px, Forbidden, layout(Padding(Side::Right))),
    Kind::PaddingSide(Side::Top) => ("padding-top", Type::Px, Forbidden, layout(Padding(Side::Top))),
    Kind::PaddingSide(Side::Bottom) => ("padding-bottom", Type::Px, Forbidden, layout(Padding(Side::Bottom))),
    Kind::Gap => ("gap", Type::Px, Forbidden, layout(Gap)),
    Kind::RowGap => ("row-gap", Type::Px, Forbidden, layout(RowGap)),
    Kind::ColumnGap => ("column-gap", Type::Px, Forbidden, layout(ColumnGap)),
    Kind::BorderWidthSide(Side::Left) => ("border-left-width", Type::Px, Forbidden, layout(BorderWidthSide(Side::Left))),
    Kind::BorderWidthSide(Side::Top) => ("border-top-width", Type::Px, Forbidden, layout(BorderWidthSide(Side::Top))),
    Kind::BorderWidthSide(Side::Right) => ("border-right-width", Type::Px, Forbidden, layout(BorderWidthSide(Side::Right))),
    Kind::BorderWidthSide(Side::Bottom) => ("border-bottom-width", Type::Px, Forbidden, layout(BorderWidthSide(Side::Bottom))),
    Kind::BorderRadius => ("border-radius", Type::Px, Forbidden, style(BorderRadius)),
    Kind::FontSize => ("font-size", Type::Px, Forbidden, text(Size)),
    Kind::FontFamily => ("font-family", Type::String, Forbidden, text(Family)),
    Kind::JustifyContent => ("justify-content", Type::Keywords(&["start", "center", "end", "space-between"]), Forbidden, layout(Justify)),
    Kind::AlignItems => ("align-items", Type::Keywords(&["start", "center", "end", "stretch"]), Forbidden, layout(Align)),
    Kind::Background => ("background", Type::Paint, Visual, style(Background)),
    Kind::BorderColorSide(Side::Left) => ("border-left-color", Type::Color, Visual, style(BorderSide(Side::Left))),
    Kind::BorderColorSide(Side::Top) => ("border-top-color", Type::Color, Visual, style(BorderSide(Side::Top))),
    Kind::BorderColorSide(Side::Right) => ("border-right-color", Type::Color, Visual, style(BorderSide(Side::Right))),
    Kind::BorderColorSide(Side::Bottom) => ("border-bottom-color", Type::Color, Visual, style(BorderSide(Side::Bottom))),
    Kind::Color => ("color", Type::Color, Visual, style(Text)),
    Kind::Tint => ("tint", Type::Color, Visual, style(Tint)),
    Kind::TextAlign => ("text-align", Type::Keywords(&["left", "center", "right"]), Visual, text(Align)),
    Kind::VerticalAlign => ("vertical-align", Type::Keywords(&["top", "center", "bottom"]), Visual, text(VerticalAlign)),
    Kind::FontWeight => ("font-weight", Type::NumberIn(&[400.0, 500.0, 600.0, 700.0]), TextWeight, text(Weight)),
    Kind::Opacity => ("opacity", Type::NumberRange(0.0, 1.0), Visual, style(Opacity)),
    Kind::Clip => ("clip", Type::Bool, Visual, style(Clip)),
    Kind::Scroll => ("scroll", Type::Bool, Forbidden, style(Scroll)),
    Kind::ShadowOffsetX => ("box-shadow-offset-x", Type::SignedPx, Forbidden, style(ShadowX)),
    Kind::ShadowOffsetY => ("box-shadow-offset-y", Type::SignedPx, Forbidden, style(ShadowY)),
    Kind::ShadowBlur => ("box-shadow-blur", Type::Px, Forbidden, style(ShadowBlur)),
    Kind::ShadowColor => ("box-shadow-color", Type::Color, Visual, style(ShadowColor)),
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

    pub fn target(self) -> Target {
        self.spec().target
    }

    pub fn animatable(self) -> bool {
        matches!(
            self,
            Kind::Background
                | Kind::BorderColorSide(_)
                | Kind::Color
                | Kind::Tint
                | Kind::Opacity
                | Kind::ShadowColor
        )
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
    pub target: Target,
}

impl Spec {
    pub fn theme_type(&self) -> Option<ThemeType> {
        match self.ty {
            Type::Px | Type::SignedPx | Type::Length => Some(ThemeType::Px),
            Type::Color | Type::Paint => Some(ThemeType::Color),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, SPECS, Side, StatePolicy, Target};
    use crate::ty::Type;

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
    fn every_kind_is_listed_once() {
        assert_eq!(Kind::ALL.len(), SPECS.len());
        for kind in Kind::ALL {
            assert_eq!(SPECS.iter().filter(|spec| spec.kind == *kind).count(), 1);
        }
    }

    #[test]
    fn grid_properties_are_layout_geometry() {
        let grid = [
            Kind::GridTemplateColumns,
            Kind::GridTemplateRows,
            Kind::GridColumn,
            Kind::GridRow,
            Kind::GridAutoFlow,
        ];
        for kind in grid {
            let spec = kind.spec();
            assert!(matches!(spec.target, Target::Layout(_)));
            assert!(matches!(spec.in_state, StatePolicy::Forbidden));
        }
    }

    #[test]
    fn display_accepts_grid() {
        assert!(matches!(
            Kind::Display.spec().ty,
            Type::Keywords(names) if names.contains(&"grid")
        ));
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

    #[test]
    fn every_side_has_its_own_border_width_property() {
        let names = [
            (Side::Left, "border-left-width"),
            (Side::Top, "border-top-width"),
            (Side::Right, "border-right-width"),
            (Side::Bottom, "border-bottom-width"),
        ];
        for (side, name) in names {
            assert_eq!(Kind::from_name(name), Some(Kind::BorderWidthSide(side)));
            assert_eq!(Kind::BorderWidthSide(side).name(), name);
        }
    }

    #[test]
    fn every_side_has_its_own_border_color_property() {
        let names = [
            (Side::Left, "border-left-color"),
            (Side::Top, "border-top-color"),
            (Side::Right, "border-right-color"),
            (Side::Bottom, "border-bottom-color"),
        ];
        for (side, name) in names {
            assert_eq!(Kind::from_name(name), Some(Kind::BorderColorSide(side)));
            assert_eq!(Kind::BorderColorSide(side).name(), name);
            assert!(Kind::BorderColorSide(side).animatable());
        }
    }
}

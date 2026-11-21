use crate::ty::Type;

pub use brul_style_schema::{LayoutField, Side, StatePolicy, StyleField, Target, TextField};

include!(concat!(env!("OUT_DIR"), "/style_properties.rs"));

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
    pub animatable: bool,
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

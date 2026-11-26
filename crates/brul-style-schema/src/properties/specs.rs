use super::*;

pub(super) const fn scroll() -> PropertySpec {
    PropertySpec {
        key: PropertyKey::Scroll,
        name: "scroll",
        field: "scroll",
        ty: Type::Bool,
        in_state: Forbidden,
        target: Style(StyleField::Scroll),
        animatable: false,
        default: "false",
    }
}

pub(super) const fn cursor() -> PropertySpec {
    PropertySpec {
        key: PropertyKey::Cursor,
        name: "cursor",
        field: "cursor",
        ty: Type::Keywords(&[
            "default",
            "pointer",
            "text",
            "grab",
            "grabbing",
            "crosshair",
            "not-allowed",
            "move",
        ]),
        in_state: Visual,
        target: Style(StyleField::Cursor),
        animatable: false,
        default: "default",
    }
}

pub(super) const fn shadow_offset_x() -> PropertySpec {
    PropertySpec {
        key: PropertyKey::ShadowOffsetX,
        name: "box-shadow-offset-x",
        field: "shadow",
        ty: Type::SignedPx,
        in_state: Forbidden,
        target: Style(StyleField::ShadowX),
        animatable: false,
        default: "0",
    }
}

pub(super) const fn shadow_offset_y() -> PropertySpec {
    PropertySpec {
        key: PropertyKey::ShadowOffsetY,
        name: "box-shadow-offset-y",
        field: "shadow",
        ty: Type::SignedPx,
        in_state: Forbidden,
        target: Style(StyleField::ShadowY),
        animatable: false,
        default: "0",
    }
}

pub(super) const fn shadow_blur() -> PropertySpec {
    PropertySpec {
        key: PropertyKey::ShadowBlur,
        name: "box-shadow-blur",
        field: "shadow",
        ty: Type::Px,
        in_state: Forbidden,
        target: Style(StyleField::ShadowBlur),
        animatable: false,
        default: "0",
    }
}

pub(super) const fn shadow_color() -> PropertySpec {
    PropertySpec {
        key: PropertyKey::ShadowColor,
        name: "box-shadow-color",
        field: "shadow",
        ty: Type::Color,
        in_state: Visual,
        target: Style(StyleField::ShadowColor),
        animatable: true,
        default: "transparent",
    }
}

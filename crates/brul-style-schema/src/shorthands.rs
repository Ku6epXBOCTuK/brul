use crate::{PropertyKey, ShorthandParser, ShorthandSpec, Side::*};

pub static SHORTHANDS: &[ShorthandSpec] = &[
    ShorthandSpec {
        name: "border",
        expands_to: &[
            PropertyKey::BorderWidthSide(Top),
            PropertyKey::BorderWidthSide(Right),
            PropertyKey::BorderWidthSide(Bottom),
            PropertyKey::BorderWidthSide(Left),
            PropertyKey::BorderColorSide(Top),
            PropertyKey::BorderColorSide(Right),
            PropertyKey::BorderColorSide(Bottom),
            PropertyKey::BorderColorSide(Left),
        ],
        parser: ShorthandParser::Border,
    },
    ShorthandSpec {
        name: "border-color",
        expands_to: &[
            PropertyKey::BorderColorSide(Top),
            PropertyKey::BorderColorSide(Right),
            PropertyKey::BorderColorSide(Bottom),
            PropertyKey::BorderColorSide(Left),
        ],
        parser: ShorthandParser::BorderColor,
    },
    ShorthandSpec {
        name: "border-width",
        expands_to: &[
            PropertyKey::BorderWidthSide(Top),
            PropertyKey::BorderWidthSide(Right),
            PropertyKey::BorderWidthSide(Bottom),
            PropertyKey::BorderWidthSide(Left),
        ],
        parser: ShorthandParser::Box,
    },
    ShorthandSpec {
        name: "padding",
        expands_to: &[
            PropertyKey::PaddingSide(Top),
            PropertyKey::PaddingSide(Right),
            PropertyKey::PaddingSide(Bottom),
            PropertyKey::PaddingSide(Left),
        ],
        parser: ShorthandParser::Box,
    },
    ShorthandSpec {
        name: "box-shadow",
        expands_to: &[
            PropertyKey::ShadowOffsetX,
            PropertyKey::ShadowOffsetY,
            PropertyKey::ShadowBlur,
            PropertyKey::ShadowColor,
        ],
        parser: ShorthandParser::Shadow,
    },
];

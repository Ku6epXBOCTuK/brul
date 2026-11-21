use brul_style_schema::{
    Converter, FieldDefault, FieldSpec, FieldType, LayoutField, PatchField, PatchTarget,
    PropertyKey, PropertySpec, ShorthandParser, SidesType, StatePolicy, StyleField, Target,
    TweenKind, Type,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::util::{ident, owned_patch, side_ident, side_variant};

pub(crate) fn converter(converter: Converter) -> Option<TokenStream> {
    let name = match converter {
        Converter::Raw => return None,
        Converter::Dimension => "dimension",
        Converter::Sides(SidesType::Padding) => "padding",
        Converter::Sides(SidesType::BorderWidths) => "border_widths",
        Converter::Float => "float",
        Converter::String => "string",
        Converter::Paint => "paint",
        Converter::Color => "color",
        Converter::Duration => "millis",
        Converter::Tracks => "tracks",
        Converter::GridLine => "grid_line",
        Converter::AreaTemplate => "area_template",
        Converter::Display => "display",
        Converter::Direction => "direction",
        Converter::Justify => "justify",
        Converter::Align => "align",
        Converter::VerticalAlign => "vertical_align",
        Converter::TextAlign => "text_align",
        Converter::AutoFlow => "auto_flow",
    };
    let name = ident(name);
    Some(quote!(convert::#name))
}

pub(crate) fn field_type(ty: FieldType) -> TokenStream {
    match ty {
        FieldType::Dimension => quote!(Dimension),
        FieldType::U32 => quote!(u32),
        FieldType::I32 => quote!(i32),
        FieldType::F32 => quote!(f32),
        FieldType::Bool => quote!(bool),
        FieldType::SidesU32 => quote!(Sides<u32>),
        FieldType::Display => quote!(Display),
        FieldType::Direction => quote!(Direction),
        FieldType::Justify => quote!(Justify),
        FieldType::Align => quote!(Align),
        FieldType::TrackList => quote!(Vec<TrackSize>),
        FieldType::GridLine => quote!(GridLine),
        FieldType::GridAutoFlow => quote!(GridAutoFlow),
        FieldType::AreaTemplate => quote!(AreaTemplate),
        FieldType::String => quote!(String),
        FieldType::Paint => quote!(Paint),
        FieldType::Color => quote!(Color),
        FieldType::SidesColor => quote!(BorderColors),
        FieldType::Shadow => quote!(Shadow),
        FieldType::VerticalAlign => quote!(VerticalAlign),
        FieldType::OptionColor => quote!(Option<Color>),
        FieldType::OptionString => quote!(Option<String>),
        FieldType::TextAlign => quote!(TextAlign),
        FieldType::U16 => quote!(u16),
        FieldType::Duration => quote!(Duration),
    }
}

pub(crate) fn default_expr(spec: &FieldSpec) -> TokenStream {
    match spec.default {
        FieldDefault::Default => quote!(Default::default()),
        FieldDefault::Zero => match spec.ty {
            FieldType::F32 => quote!(0.0),
            _ => quote!(0),
        },
        FieldDefault::One => quote!(1.0),
        FieldDefault::None => quote!(None),
        FieldDefault::Text => quote!(Color::rgb(15, 23, 42)),
        FieldDefault::FontSize => quote!(14.0),
        FieldDefault::LineHeight => quote!(14.0 * 1.2),
        FieldDefault::Weight => quote!(400),
        FieldDefault::TransparentSides => quote!(BorderColors::all(Color::TRANSPARENT)),
    }
}

pub(crate) fn patch_field_type(spec: &FieldSpec, patch: &PatchField) -> TokenStream {
    match patch.target {
        PatchTarget::This => match spec.ty {
            FieldType::OptionColor => quote!(Color),
            FieldType::OptionString => quote!(String),
            ty => field_type(ty),
        },
        PatchTarget::Side(_) => quote!(Color),
        PatchTarget::Shadow(part) => field_type(part.ty()),
        PatchTarget::Scaled(_) => quote!(f32),
    }
}

pub(crate) fn patch_tween(spec: &FieldSpec, patch: &PatchField) -> TweenKind {
    if spec.tween == TweenKind::None {
        return TweenKind::None;
    }
    match (spec.ty, patch.target) {
        (FieldType::Paint, _) => TweenKind::Paint,
        (FieldType::OptionColor, _) => TweenKind::OptionColor,
        (FieldType::F32, PatchTarget::This) => TweenKind::Float,
        (FieldType::Color, _) => TweenKind::Color,
        (FieldType::SidesColor, PatchTarget::Side(_)) => TweenKind::Color,
        (FieldType::Shadow, PatchTarget::Shadow(part)) => part.tween(),
        _ => TweenKind::None,
    }
}

pub(crate) fn patch_target(spec: &FieldSpec, patch: &PatchField) -> TokenStream {
    let field = ident(spec.name);
    match patch.target {
        PatchTarget::This => quote!(#field),
        PatchTarget::Side(side) => {
            let side = side_ident(side);
            quote!(#field.#side)
        }
        PatchTarget::Shadow(part) => {
            let name = ident(part.name());
            quote!(#field.#name)
        }
        PatchTarget::Scaled(_) => quote!(#field),
    }
}

pub(crate) fn tween_expr(tween: TweenKind, target: &TokenStream) -> TokenStream {
    match tween {
        TweenKind::Paint => quote!(tween_paint(self.from.#target, self.to.#target, track.value())),
        TweenKind::OptionColor => {
            quote!(tween_tint(self.from.#target, self.to.#target, track.value()))
        }
        TweenKind::Color | TweenKind::Float => {
            quote!(self.from.#target.tween(self.to.#target, track.value()))
        }
        TweenKind::None => unreachable!("only tweenable fields reach tween_expr"),
    }
}

pub(crate) fn apply_line(spec: &FieldSpec, patch: &PatchField) -> TokenStream {
    let name = ident(patch.name);
    let target = patch_target(spec, patch);
    match patch.target {
        PatchTarget::Scaled(factor) => {
            quote! {
                if let Some(value) = patch.#name {
                    self.#target = value * #factor;
                }
            }
        }
        PatchTarget::This if spec.ty == FieldType::OptionString => quote! {
            if let Some(value) = &patch.#name {
                self.#target = Some(value.clone());
            }
        },
        PatchTarget::This if spec.ty == FieldType::OptionColor => quote! {
            if let Some(value) = patch.#name {
                self.#target = Some(value);
            }
        },
        _ => quote! {
            if let Some(value) = patch.#name {
                self.#target = value;
            }
        },
    }
}

pub(crate) fn merge_line(patch: &PatchField) -> TokenStream {
    let name = ident(patch.name);
    if owned_patch(patch) {
        quote! {
            if other.#name.is_some() {
                self.#name = other.#name.clone();
            }
        }
    } else {
        quote! {
            if other.#name.is_some() {
                self.#name = other.#name;
            }
        }
    }
}

pub(crate) fn spec(spec: &PropertySpec) -> TokenStream {
    let kind = kind(spec.key);
    let name = spec.name;
    let ty = ty(&spec.ty);
    let state = state(spec.in_state);
    let target = target(spec.target);
    let animatable = spec.animatable;
    quote! {
        Spec {
            kind: #kind,
            name: #name,
            ty: #ty,
            in_state: #state,
            target: #target,
            animatable: #animatable,
        }
    }
}

pub(crate) fn variant(key: PropertyKey) -> (proc_macro2::Ident, bool) {
    match key {
        PropertyKey::PaddingSide(_) => (format_ident!("PaddingSide"), true),
        PropertyKey::BorderWidthSide(_) => (format_ident!("BorderWidthSide"), true),
        PropertyKey::BorderColorSide(_) => (format_ident!("BorderColorSide"), true),
        key => (format_ident!("{key:?}"), false),
    }
}

pub(crate) fn kind(key: PropertyKey) -> TokenStream {
    let (name, sided) = variant(key);
    if sided {
        let side = side_variant(match key {
            PropertyKey::PaddingSide(side)
            | PropertyKey::BorderWidthSide(side)
            | PropertyKey::BorderColorSide(side) => side,
            _ => unreachable!("only three property keys are sided"),
        });
        quote!(Kind::#name(Side::#side))
    } else {
        quote!(Kind::#name)
    }
}

pub(crate) fn ty(ty: &Type) -> TokenStream {
    match ty {
        Type::Keywords(names) => quote!(Type::Keywords(&[#(#names),*])),
        Type::NumberRange(from, to) => quote!(Type::NumberRange(#from, #to)),
        Type::NumberIn(numbers) => quote!(Type::NumberIn(&[#(#numbers),*])),
        ty => {
            let name = format_ident!("{ty:?}");
            quote!(Type::#name)
        }
    }
}

pub(crate) fn state(policy: StatePolicy) -> TokenStream {
    let name = format_ident!("{policy:?}");
    quote!(StatePolicy::#name)
}

pub(crate) fn target(target: Target) -> TokenStream {
    match target {
        Target::Layout(field) => {
            let field = layout_field(field);
            quote!(Target::Layout(#field))
        }
        Target::Style(field) => {
            let field = style_field(field);
            quote!(Target::Style(#field))
        }
        Target::Text(field) => {
            let name = format_ident!("{field:?}");
            quote!(Target::Text(TextField::#name))
        }
    }
}

pub(crate) fn layout_field(field: LayoutField) -> TokenStream {
    match field {
        LayoutField::Padding(side) => {
            let side = side_variant(side);
            quote!(LayoutField::Padding(Side::#side))
        }
        LayoutField::BorderWidthSide(side) => {
            let side = side_variant(side);
            quote!(LayoutField::BorderWidthSide(Side::#side))
        }
        field => {
            let name = format_ident!("{field:?}");
            quote!(LayoutField::#name)
        }
    }
}

pub(crate) fn style_field(field: StyleField) -> TokenStream {
    match field {
        StyleField::BorderSide(side) => {
            let side = side_variant(side);
            quote!(StyleField::BorderSide(Side::#side))
        }
        field => {
            let name = format_ident!("{field:?}");
            quote!(StyleField::#name)
        }
    }
}

pub(crate) fn parser(parser: ShorthandParser) -> proc_macro2::Ident {
    match parser {
        ShorthandParser::Box => format_ident!("box_values"),
        ShorthandParser::Border => format_ident!("border_values"),
        ShorthandParser::BorderColor => format_ident!("color_values"),
        ShorthandParser::Shadow => format_ident!("shadow_values"),
    }
}

use std::collections::HashSet;

use brul_style_schema::{
    FIELDS, FieldSpec, FieldType, LayoutField, PROPERTIES, PatchField, PatchTarget, PropertySpec,
    ShadowPart, StyleField, StyleOwner, Target, TextField, Type,
};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::tokens::{field_type, kind};
use crate::util::{ident, side_variant};

fn owner(spec: &PropertySpec) -> StyleOwner {
    match spec.target {
        Target::Layout(_) => StyleOwner::Layout,
        Target::Style(_) => StyleOwner::Widget,
        Target::Text(TextField::VerticalAlign) => StyleOwner::Widget,
        Target::Text(_) => StyleOwner::Text,
    }
}

fn field(spec: &PropertySpec) -> &'static FieldSpec {
    FIELDS
        .iter()
        .find(|candidate| candidate.owner == owner(spec) && candidate.name == spec.field)
        .unwrap_or_else(|| panic!("field {} is missing for property {}", spec.field, spec.name))
}

fn patch(spec: &PropertySpec, field: &FieldSpec) -> &'static PatchField {
    let shadow = |part| {
        field.patches.iter().find(
            |candidate| matches!(candidate.target, PatchTarget::Shadow(value) if value == part),
        )
    };
    let chosen = match spec.target {
        Target::Style(StyleField::BorderSide(side)) => field.patches.iter().find(
            |candidate| matches!(candidate.target, PatchTarget::Side(value) if value == side),
        ),
        Target::Style(StyleField::ShadowX) => shadow(ShadowPart::X),
        Target::Style(StyleField::ShadowY) => shadow(ShadowPart::Y),
        Target::Style(StyleField::ShadowBlur) => shadow(ShadowPart::Blur),
        Target::Style(StyleField::ShadowColor) => shadow(ShadowPart::Color),
        _ => field
            .patches
            .iter()
            .find(|candidate| !matches!(candidate.target, PatchTarget::Scaled(_))),
    };
    chosen.unwrap_or_else(|| panic!("no patch for property {}", spec.name))
}

fn value_type(field: &FieldSpec, patch: &PatchField) -> FieldType {
    match patch.target {
        PatchTarget::This => match field.ty {
            FieldType::OptionColor => FieldType::Color,
            FieldType::OptionString => FieldType::String,
            ty => ty,
        },
        PatchTarget::Side(_) => FieldType::Color,
        PatchTarget::Shadow(part) => part.ty(),
        PatchTarget::Scaled(_) => FieldType::F32,
    }
}

fn reader(ty: &Type) -> Ident {
    let name = match ty {
        Type::ThemeKey => unreachable!("theme keys are resolved before a snapshot is read"),
        Type::Number | Type::NumberRange(..) | Type::NumberIn(..) => "NumberTy",
        Type::Px => "PxTy",
        Type::SignedPx => "SignedPxTy",
        Type::Length => "LengthTy",
        Type::Color => "ColorTy",
        Type::Paint => "PaintTy",
        Type::String => "TextTy",
        Type::Bool => "FlagTy",
        Type::Keywords(_) => "KeywordTy",
        Type::TrackList => "TrackListTy",
        Type::GridLine => "GridLineTy",
        Type::AreaTemplate => "AreaTemplateTy",
        Type::GridArea => "GridAreaTy",
    };
    ident(name)
}

fn keyword_fn(ty: FieldType) -> Ident {
    let name = match ty {
        FieldType::Display => "display",
        FieldType::Direction => "flex_direction",
        FieldType::Justify => "justify",
        FieldType::Align => "align",
        FieldType::AnchorSide => "anchor_side",
        FieldType::AnchorAlign => "anchor_align",
        FieldType::GridAutoFlow => "grid_auto_flow",
        FieldType::TextAlign => "text_align",
        FieldType::VerticalAlign => "vertical_align",
        FieldType::Cursor => "cursor",
        _ => unreachable!("keyword properties map to keyword fields"),
    };
    ident(name)
}

fn keyword_variant(name: &str) -> Ident {
    let value: String = name
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    format_ident!("{value}")
}

fn keyword_helper(field: &FieldSpec, names: &[&'static str]) -> TokenStream {
    let name = keyword_fn(field.ty);
    let ty = field_type(field.ty);
    let arms = names.iter().skip(1).map(|keyword| {
        let variant = keyword_variant(keyword);
        quote!(#keyword => #ty::#variant,)
    });
    let fallback = keyword_variant(names[0]);
    quote! {
        fn #name(value: &str) -> #ty {
            match value {
                #(#arms)*
                _ => #ty::#fallback,
            }
        }
    }
}

fn expression(spec: &PropertySpec, field: &FieldSpec, patch: &PatchField) -> TokenStream {
    let reader = reader(&spec.ty);
    let read = quote!(read::<#reader>(value, kind)?);
    match (&spec.ty, value_type(field, patch)) {
        (Type::Number | Type::NumberRange(..) | Type::NumberIn(..), FieldType::F32) => {
            quote!(#read as f32)
        }
        (Type::Number | Type::NumberRange(..) | Type::NumberIn(..), FieldType::U16) => {
            quote!(#read as u16)
        }
        (Type::Px, FieldType::F32) => quote!(#read as f32),
        (Type::String, _) => quote!(#read.to_owned()),
        (Type::Keywords(_), _) => {
            let helper = keyword_fn(field.ty);
            quote!(#helper(#read))
        }
        _ => read,
    }
}

fn base_target(spec: &PropertySpec, field: &FieldSpec, patch: &PatchField) -> TokenStream {
    let base = match owner(spec) {
        StyleOwner::Layout => quote!(layout),
        StyleOwner::Widget => quote!(style),
        StyleOwner::Text => quote!(text),
        StyleOwner::Transition => unreachable!("properties never target transitions"),
    };
    let name = ident(field.name);
    let sided = |side| {
        let side = side_variant(side);
        quote!(#base.#name[Side::#side])
    };
    match spec.target {
        Target::Layout(LayoutField::Padding(side)) => sided(side),
        Target::Layout(LayoutField::BorderWidthSide(side)) => sided(side),
        _ => match patch.target {
            PatchTarget::This => quote!(#base.#name),
            PatchTarget::Side(side) => sided(side),
            PatchTarget::Shadow(part) => {
                let part = ident(part.name());
                quote!(#base.#name.#part)
            }
            PatchTarget::Scaled(_) => unreachable!("scaled patches only extend font size"),
        },
    }
}

fn assignment(
    target: TokenStream,
    field: &FieldSpec,
    patch: &PatchField,
    value: TokenStream,
    wrap: bool,
) -> TokenStream {
    let value = if wrap
        || (matches!(patch.target, PatchTarget::This)
            && matches!(field.ty, FieldType::OptionColor | FieldType::OptionString))
    {
        quote!(Some(#value))
    } else {
        value
    };
    quote!(#target = #value;)
}

fn scaled(spec: &PropertySpec, patch: &PatchField) -> Option<(&'static FieldSpec, f32)> {
    FIELDS
        .iter()
        .filter(|candidate| candidate.owner == owner(spec))
        .find_map(|candidate| {
            candidate
                .patches
                .iter()
                .find_map(|candidate_patch| match candidate_patch.target {
                    PatchTarget::Scaled(factor) if candidate_patch.name == patch.name => {
                        Some((candidate, factor))
                    }
                    _ => None,
                })
        })
}

fn transition_field(name: &str) -> &'static FieldSpec {
    FIELDS
        .iter()
        .find(|candidate| {
            candidate.owner == StyleOwner::Transition
                && candidate.patches.iter().any(|patch| patch.name == name)
        })
        .unwrap_or_else(|| panic!("no transition field for patch {name}"))
}

pub(crate) fn snapshot() -> TokenStream {
    let mut helpers = Vec::new();
    let mut seen_helpers = HashSet::new();
    let mut apply = Vec::new();
    let mut patch_arms = Vec::new();
    let mut transition_arms = Vec::new();

    for spec in PROPERTIES {
        let kind = kind(spec.key);
        let field = field(spec);
        let property_patch = patch(spec, field);
        let value = expression(spec, field, property_patch);

        if let Type::Keywords(names) = &spec.ty {
            let helper = keyword_fn(field.ty).to_string();
            if seen_helpers.insert(helper) {
                helpers.push(keyword_helper(field, names));
            }
        }

        let target = base_target(spec, field, property_patch);
        let statements = match scaled(spec, property_patch) {
            Some((scaled_field, factor)) => {
                let name = ident(scaled_field.name);
                vec![quote! {
                    let value = #value;
                    #target = value;
                    text.#name = value * #factor;
                }]
            }
            None => vec![assignment(
                target,
                field,
                property_patch,
                value.clone(),
                false,
            )],
        };
        apply.push(quote!(#kind => { #(#statements)* }));

        let patch_statement = match owner(spec) {
            StyleOwner::Layout => quote!(),
            base => {
                let name = ident(property_patch.name);
                let base = match base {
                    StyleOwner::Widget => quote!(style),
                    StyleOwner::Text => quote!(text),
                    _ => unreachable!(),
                };
                assignment(quote!(#base.#name), field, property_patch, value, true)
            }
        };
        patch_arms.push(quote!(#kind => { #patch_statement }));

        let action = if spec.animatable {
            let name = ident(transition_field(property_patch.name).name);
            quote!(transitions.#name = Some(duration),)
        } else {
            quote!(
                return Err(StyleError::Value {
                    kind,
                    expected: "an animatable property",
                }),
            )
        };
        transition_arms.push(quote!(#kind => #action));
    }

    quote! {
        #(#helpers)*

        fn apply_property(
            property: &Property,
            layout: &mut LayoutStyle,
            style: &mut WidgetStyle,
            text: &mut TextStyle,
        ) -> Result<(), StyleError> {
            let kind = property.kind;
            let value = &property.value;
            match kind {
                #(#apply)*
            }
            Ok(())
        }

        fn patch_property(
            property: &Property,
            style: &mut WidgetStylePatch,
            text: &mut TextStylePatch,
        ) -> Result<(), StyleError> {
            let kind = property.kind;
            let value = &property.value;
            match kind {
                #(#patch_arms)*
            }
            Ok(())
        }

        fn transition_property(
            kind: Kind,
            duration: Duration,
            transitions: &mut Transitions,
        ) -> Result<(), StyleError> {
            match kind {
                #(#transition_arms)*
            }
            Ok(())
        }
    }
}

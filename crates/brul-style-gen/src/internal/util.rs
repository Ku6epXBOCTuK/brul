use std::collections::HashSet;

use brul_style_schema::{FIELDS, FieldSpec, FieldType, PatchField, PatchTarget, Side, StyleOwner};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::tokens::{apply_line, merge_line};

pub(crate) fn fields(owner: StyleOwner) -> Vec<&'static FieldSpec> {
    FIELDS.iter().filter(|spec| spec.owner == owner).collect()
}

pub(crate) fn patches(owner: StyleOwner) -> Vec<&'static PatchField> {
    let mut seen = HashSet::new();
    let mut result: Vec<_> = fields(owner)
        .iter()
        .flat_map(|spec| spec.patches.iter())
        .filter(|patch| seen.insert(patch.name))
        .collect();
    result.sort_by_key(|patch| patch.order);
    result
}

pub(crate) fn is_side_patch(spec: &FieldSpec, patch: &PatchField) -> bool {
    spec.ty == FieldType::SidesColor && matches!(patch.target, PatchTarget::Side(_))
}

pub(crate) fn side_of(patch: &PatchField) -> Side {
    match patch.target {
        PatchTarget::Side(side) => side,
        _ => unreachable!("only side patches reach side_of"),
    }
}

pub(crate) fn apply_lines(specs: &[&'static FieldSpec]) -> Vec<TokenStream> {
    let mut lines = Vec::new();
    for spec in specs {
        let sides: Vec<_> = spec
            .patches
            .iter()
            .filter(|patch| is_side_patch(spec, patch))
            .collect();
        if sides.is_empty() {
            lines.extend(spec.patches.iter().map(|patch| apply_line(spec, patch)));
            continue;
        }
        let field = ident(spec.name);
        let arms: Vec<_> = sides
            .iter()
            .map(|patch| {
                let side = side_variant(side_of(patch));
                let name = ident(patch.name);
                quote!(Side::#side => patch.#name,)
            })
            .collect();
        lines.push(quote! {
            for side in Side::ALL {
                let value = match side {
                    #(#arms)*
                };
                if let Some(value) = value {
                    self.#field[side] = value;
                }
            }
        });
    }
    lines
}

pub(crate) fn merge_lines(patches: &[&'static PatchField]) -> Vec<TokenStream> {
    let mut lines: Vec<_> = patches
        .iter()
        .filter(|patch| !matches!(patch.target, PatchTarget::Side(_)))
        .map(|patch| merge_line(patch))
        .collect();
    let sides: Vec<_> = patches
        .iter()
        .filter(|patch| matches!(patch.target, PatchTarget::Side(_)))
        .copied()
        .collect();
    if !sides.is_empty() {
        let arms: Vec<_> = sides
            .iter()
            .map(|patch| {
                let side = side_variant(side_of(patch));
                let name = ident(patch.name);
                quote!(Side::#side => (other.#name, &mut self.#name),)
            })
            .collect();
        lines.push(quote! {
            for side in Side::ALL {
                let (value, target) = match side {
                    #(#arms)*
                };
                if value.is_some() {
                    *target = value;
                }
            }
        });
    }
    lines
}

pub(crate) fn ident(name: &str) -> proc_macro2::Ident {
    format_ident!("{name}")
}

pub(crate) fn side_ident(side: Side) -> proc_macro2::Ident {
    format_ident!("{}", format!("{side:?}").to_lowercase())
}

pub(crate) fn side_variant(side: Side) -> proc_macro2::Ident {
    format_ident!("{side:?}")
}

pub(crate) fn is_copy(ty: FieldType) -> bool {
    !matches!(
        ty,
        FieldType::TrackList | FieldType::AreaTemplate | FieldType::String
    )
}

pub(crate) fn owned_patch(patch: &PatchField) -> bool {
    FIELDS.iter().any(|spec| {
        spec.ty == FieldType::OptionString
            && spec
                .patches
                .iter()
                .any(|candidate| candidate.name == patch.name)
    })
}

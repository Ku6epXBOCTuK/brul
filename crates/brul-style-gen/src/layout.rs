use brul_style_schema::StyleOwner;
use proc_macro2::TokenStream;
use quote::quote;

use crate::tokens::field_type;
use crate::util::{fields, ident, is_copy};

pub(crate) fn layout_style() -> TokenStream {
    let specs = fields(StyleOwner::Layout);
    let style_fields: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let ty = field_type(spec.ty);
            quote!(pub #name: #ty,)
        })
        .collect();
    let patch_fields: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let ty = field_type(spec.ty);
            quote!(pub #name: Option<#ty>,)
        })
        .collect();
    let defaults: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            quote!(#name: None,)
        })
        .collect();
    let diff: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            if is_copy(spec.ty) {
                quote!(#name: (style.#name != default.#name).then_some(style.#name),)
            } else {
                quote!(#name: (style.#name != default.#name).then(|| style.#name.clone()),)
            }
        })
        .collect();
    let apply: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            if is_copy(spec.ty) {
                quote! {
                    if let Some(value) = patch.#name {
                        self.#name = value;
                    }
                }
            } else {
                quote! {
                    if let Some(value) = &patch.#name {
                        self.#name = value.clone();
                    }
                }
            }
        })
        .collect();
    quote! {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutStyle {
            #(#style_fields)*
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutPatch {
            #(#patch_fields)*
        }

        impl LayoutPatch {
            pub const DEFAULT: Self = Self {
                #(#defaults)*
            };

            pub fn is_empty(&self) -> bool {
                *self == Self::DEFAULT
            }
        }

        impl From<&LayoutStyle> for LayoutPatch {
            fn from(style: &LayoutStyle) -> Self {
                let default = LayoutStyle::default();
                Self {
                    #(#diff)*
                }
            }
        }

        impl LayoutStyle {
            pub fn apply(&mut self, patch: &LayoutPatch) {
                #(#apply)*
            }

            pub fn row_gap(&self) -> u32 {
                if self.row_gap == 0 { self.gap } else { self.row_gap }
            }

            pub fn column_gap(&self) -> u32 {
                if self.column_gap == 0 { self.gap } else { self.column_gap }
            }
        }
    }
}

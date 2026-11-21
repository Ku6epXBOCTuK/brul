use brul_style_schema::StyleOwner;
use proc_macro2::TokenStream;
use quote::quote;

use crate::tokens::{default_expr, field_type, patch_field_type};
use crate::util::{apply_lines, fields, ident, merge_lines, patches};

pub(crate) fn text() -> TokenStream {
    let specs = fields(StyleOwner::Text);
    let style_fields: Vec<_> = specs
        .iter()
        .map(|spec| {
            let name = ident(spec.name);
            let ty = field_type(spec.ty);
            quote!(pub #name: #ty,)
        })
        .collect();
    let new_fields: Vec<_> = specs
        .iter()
        .map(|spec| match spec.name {
            "font_size" => quote!(font_size,),
            "line_height" => quote!(line_height: font_size * 1.2,),
            _ => {
                let name = ident(spec.name);
                let default = default_expr(spec);
                quote!(#name: #default,)
            }
        })
        .collect();
    let apply = apply_lines(&specs);
    let patch_list = patches(StyleOwner::Text);
    let patch_fields: Vec<_> = patch_list
        .iter()
        .map(|patch| {
            let spec = specs
                .iter()
                .find(|spec| {
                    spec.patches
                        .iter()
                        .any(|candidate| candidate.name == patch.name)
                })
                .expect("every patch belongs to a field of its owner");
            let name = ident(patch.name);
            let ty = patch_field_type(spec, patch);
            quote!(pub #name: Option<#ty>,)
        })
        .collect();
    let patch_defaults: Vec<_> = patch_list
        .iter()
        .map(|patch| {
            let name = ident(patch.name);
            quote!(#name: None,)
        })
        .collect();
    let merge = merge_lines(&patch_list);

    quote! {
        #[derive(Clone, Debug, PartialEq)]
        pub struct TextStyle {
            #(#style_fields)*
        }

        impl TextStyle {
            pub fn new(font_size: f32) -> Self {
                Self {
                    #(#new_fields)*
                }
            }

            pub(crate) fn apply(&mut self, patch: &TextStylePatch) {
                #(#apply)*
            }
        }

        impl Default for TextStyle {
            fn default() -> Self {
                Self::new(14.0)
            }
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct TextStylePatch {
            #(#patch_fields)*
        }

        impl TextStylePatch {
            pub const DEFAULT: Self = Self {
                #(#patch_defaults)*
            };

            pub(crate) fn merge(&mut self, other: &Self) {
                #(#merge)*
            }
        }
    }
}

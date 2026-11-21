use brul_style_schema::StyleOwner;
use proc_macro2::TokenStream;
use quote::quote;

use crate::tokens::converter;
use crate::util::{ident, patches};

pub(crate) fn codegen_fields() -> TokenStream {
    let emit = |owner, name, patch_type| {
        let fields: Vec<_> = patches(owner)
            .iter()
            .map(|patch| {
                let field = ident(patch.name);
                match converter(patch.converter) {
                    Some(converter) => quote!(#field => #converter,),
                    None => quote!(#field,),
                }
            })
            .collect();
        let name = ident(name);
        quote! {
            fn #name(patch: &#patch_type) -> String {
                let mut out = String::new();
                fields!(out, patch, #patch_type, { #(#fields)* });
                out
            }
        }
    };
    let layout = emit(StyleOwner::Layout, "layout", quote!(LayoutPatch));
    let widget = emit(StyleOwner::Widget, "widget_patch", quote!(WidgetStylePatch));
    let text = emit(StyleOwner::Text, "text_patch", quote!(TextStylePatch));
    let transitions = emit(StyleOwner::Transition, "transitions", quote!(Transitions));
    quote! {
        #layout
        #widget
        #text
        #transitions
    }
}

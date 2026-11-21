use brul_style_schema::SHORTHANDS;
use proc_macro2::TokenStream;
use quote::quote;

use crate::tokens::{kind, parser};

pub(crate) fn syntax() -> TokenStream {
    let shorthands = SHORTHANDS.iter().map(|spec| {
        let name = spec.name;
        let kinds = spec.expands_to.iter().map(|key| kind(*key));
        let parser = parser(spec.parser);
        quote!(Shorthand { name: #name, kinds: &[#(#kinds),*], parse: #parser })
    });
    quote! {
        pub static SHORTHANDS: &[Shorthand] = &[#(#shorthands),*];
    }
}

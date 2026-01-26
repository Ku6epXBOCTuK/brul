use proc_macro::TokenStream;
// use quote::quote;
// use syn::{Attribute, Item};

#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // let item = syn::parse_macro_input!(item as Item);

    // let item = match item {
    //     Item::Fn(item) => item,
    //     _ => panic!("Only functions are supported"),
    // };

    // let name = item.sig.ident.to_string();

    // println!("Attr: {:?}", attr);
    // println!("Item: {:?}", item);

    // TODO: implement configure macros function depends on argumens
    // now return unchanged

    item
}

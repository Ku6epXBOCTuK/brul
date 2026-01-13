// use proc_macro::TokenStream;
// use quote::quote;
// use syn::{Attribute, Item};

// #[proc_macro_attribute]
// pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let attr = syn::parse_macro_input!(attr as Attribute);
//     // let item = syn::parse_macro_input!(item as Item);

//     // let item = match item {
//     //     Item::Fn(item) => item,
//     //     _ => panic!("Only functions are supported"),
//     // };

//     // let name = item.sig.ident.to_string();

//     println!("{:?}", item);

//     item
// }

// // #[cfg(test)]
// // mod test {
// //     use super::*;

// //     #[command]
// //     struct Input {
// //         pub data: String,
// //     }

// //     #[test]
// //     fn test_parse() -> Result<(), Box<dyn std::error::Error>> {
// //         let input = r#""This is my problematic input""#;

// //         let stream: TokenStream = input
// //             .parse()
// //             .expect("std::error::Error is not implemented for LexError 😢");

// //         let parsed = match syn::parse2::<Input>(stream) {
// //             Ok(parsed) => parsed,
// //             Err(err) => panic!("{}", err),
// //         };

// //         todo!("Validate this: {:#?}", parsed);
// //     }
// // }

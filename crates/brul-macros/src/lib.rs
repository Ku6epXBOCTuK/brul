use brul_case::{camel, constant, snake};
use brul_command::Kind;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{FnArg, Generics, Ident, ItemFn, LitStr, Pat, Result, ReturnType, Token, Type};

#[proc_macro_attribute]
pub fn command(attribute: TokenStream, item: TokenStream) -> TokenStream {
    if !attribute.is_empty() {
        let error = syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[command] takes no arguments",
        );
        return error.to_compile_error().into();
    }
    let function = syn::parse_macro_input!(item as ItemFn);
    match expand(function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn host(input: TokenStream) -> TokenStream {
    let commands = syn::parse_macro_input!(input as Commands);
    commands.expand().into()
}

#[proc_macro]
pub fn effect(input: TokenStream) -> TokenStream {
    let definition = syn::parse_macro_input!(input as EffectDefinition);
    match expand_effect(definition) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn widget(attribute: TokenStream, item: TokenStream) -> TokenStream {
    if let Err(error) = brul_capability::parse_widget(attribute.into()) {
        return error.to_compile_error().into();
    }
    let function = syn::parse_macro_input!(item as ItemFn);
    match expand_widget(function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn attach(attribute: TokenStream, item: TokenStream) -> TokenStream {
    let arguments = match brul_capability::parse_attach(attribute.into()) {
        Ok(arguments) => arguments,
        Err(error) => return error.to_compile_error().into(),
    };
    let function = syn::parse_macro_input!(item as ItemFn);
    match expand_attach(arguments, function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[path = "parts/attach.rs"]
mod attach;
#[path = "parts/command.rs"]
mod command;
#[path = "parts/effect.rs"]
mod effect;
#[path = "parts/widget.rs"]
mod widget;

use attach::*;
use command::*;
use effect::*;
use widget::*;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_name_is_the_function_name_in_camel_case() {
        for (function, command) in [
            ("launch_browser", "launchBrowser"),
            ("detect_browsers", "detectBrowsers"),
            ("remove", "remove"),
            ("r#match", "match"),
        ] {
            assert_eq!(camel(function), command);
        }
    }

    #[test]
    fn an_argument_of_any_named_type_is_accepted() {
        let function: ItemFn =
            syn::parse_str("fn act(host: &mut Host, draft: Draft) {}").expect("fn");
        let signature = brul_command::parse(&function).expect("signature");
        assert_eq!(signature.params.len(), 1);
    }

    #[test]
    fn effect_field_types_map_to_uniform_slots() {
        for (source, slots) in [
            ("struct Params { speed: f32 }", 1),
            ("struct Params { color: [f32; 4] }", 1),
            ("struct Params { bands: [[f32; 4]; 3] }", 3),
        ] {
            let item: syn::ItemStruct = syn::parse_str(source).expect("struct");
            let field = item.fields.iter().next().expect("field");
            assert_eq!(
                FieldLayout::from_type(&field.ty).expect("layout").slots(),
                slots
            );
        }
    }

    #[test]
    fn unsupported_effect_field_is_rejected() {
        let item: syn::ItemStruct =
            syn::parse_str("struct Params { color: [f32; 5] }").expect("struct");
        let field = item.fields.iter().next().expect("field");
        assert!(FieldLayout::from_type(&field.ty).is_err());
    }
}

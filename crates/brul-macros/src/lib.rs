use brul_case::{camel, snake};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    FnArg, GenericArgument, Generics, Ident, ItemFn, Pat, PatType, PathArguments, Result,
    ReturnType, Token, Type,
};

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

struct Signature {
    host: Pat,
    host_type: Type,
    values: Vec<(Ident, Type)>,
}

fn signature(function: &ItemFn) -> Result<Signature> {
    let mut inputs = function.sig.inputs.iter();
    let Some(FnArg::Typed(PatType { pat, ty, .. })) = inputs.next() else {
        return Err(syn::Error::new_spanned(
            &function.sig,
            "a command takes the host as its first argument",
        ));
    };
    let Pat::Ident(host) = &**pat else {
        return Err(syn::Error::new_spanned(pat, "the host needs a name"));
    };
    let mut values = Vec::new();
    for input in inputs {
        let FnArg::Typed(PatType { pat, ty, .. }) = input else {
            return Err(syn::Error::new_spanned(input, "a command has no receiver"));
        };
        let Pat::Ident(name) = &**pat else {
            return Err(syn::Error::new_spanned(pat, "an argument needs a name"));
        };
        values.push((name.ident.clone(), (**ty).clone()));
    }
    Ok(Signature {
        host: Pat::Ident(host.clone()),
        host_type: (**ty).clone(),
        values,
    })
}

fn expand(function: ItemFn) -> Result<TokenStream2> {
    let signature = signature(&function)?;
    let name = &function.sig.ident;
    let hidden = format_ident!("__{}", name);
    let command = camel(&name.to_string());
    let attrs = &function.attrs;
    let vis = &function.vis;
    let generics = &function.sig.generics;
    let where_clause = &generics.where_clause;
    let declare = quote! { #generics #where_clause };
    let output = &function.sig.output;
    let host = &signature.host;
    let host_type = &signature.host_type;
    let body = &function.block;

    let values: Vec<Ident> = (0..signature.values.len())
        .map(|index| format_ident!("__value{index}"))
        .collect();
    let names: Vec<&Ident> = signature.values.iter().map(|(name, _)| name).collect();
    let types: Vec<&Type> = signature.values.iter().map(|(_, ty)| ty).collect();
    let conversions: Vec<TokenStream2> = signature
        .values
        .iter()
        .enumerate()
        .map(|(index, (name, ty))| {
            let value = &values[index];
            Ok(quote! {
                let #name: #ty = match ::std::convert::TryFrom::try_from(#value) {
                    ::std::result::Result::Ok(value) => value,
                    ::std::result::Result::Err(error) => {
                        return ::std::result::Result::Err(
                            ::brul::Error::at_command(error, #command),
                        );
                    }
                };
            })
        })
        .collect::<Result<_>>()?;

    let arity = match values.len() {
        0 => quote! {
            if !__arguments.is_empty() {
                return ::std::result::Result::Err(::brul::Error::Arity {
                    name: #command.to_owned(),
                    expected: "no",
                });
            }
        },
        count => {
            let word = match count {
                1 => "one",
                2 => "two",
                3 => "three",
                _ => "the declared number of",
            };
            quote! {
                let [#(#values),*] = __arguments else {
                    return ::std::result::Result::Err(::brul::Error::Arity {
                        name: #command.to_owned(),
                        expected: #word,
                    });
                };
                #(#conversions)*
            }
        }
    };

    let marker = format_ident!("__brul_command_{}", snake(&name.to_string()));
    let marker_types: Vec<TokenStream2> = signature
        .values
        .iter()
        .map(|(_, ty)| marker_type(ty))
        .collect();
    let argument_names: Vec<Ident> = (0..signature.values.len())
        .map(|index| format_ident!("__argument{index}"))
        .collect();
    let bindings: Vec<TokenStream2> = signature
        .values
        .iter()
        .zip(&values)
        .zip(&argument_names)
        .map(|(((_, ty), value), argument)| {
            let conversion = argument_value(argument, ty, &command);
            quote! { let #value: #ty = #conversion; }
        })
        .collect();
    let host_owned = owned_type(&signature.host_type);
    let result = result_type(&function.sig.output);
    let result_marker = result.map(marker_type).unwrap_or_else(|| quote!(()));
    let result_value = result
        .map(output_value)
        .unwrap_or_else(|| quote!(Ok(value)));

    Ok(quote! {
        #(#attrs)*
        #vis fn #name #declare (#host: #host_type, __arguments: &[::brul::Value]) #output {
            #arity
            #hidden(#host #(, #names)*)
        }

        #(#attrs)*
        #vis fn #hidden #declare (#host: #host_type, #(#names: #types),*) #output #body

        pub struct #marker;

        impl ::brul::HostCommand for #marker {
            type Arguments = (#(#marker_types,)*);
            type Output = #result_marker;

            const NAME: &'static str = #command;

            fn invoke(
                component: &mut ::brul::Component,
                arguments: Self::Arguments,
            ) -> ::std::result::Result<Self::Output, ::brul::Error> {
                let host = component
                    .host_mut::<#host_owned>()
                    .ok_or_else(|| ::brul::Error::UnknownFunction(#command.to_owned()))?;
                let (#(#argument_names,)*) = arguments;
                #(#bindings)*
                let value = #hidden(host #(, #values)*)?;
                #result_value
            }
        }
    })
}

struct Commands {
    host: Ident,
    generics: Generics,
    commands: Punctuated<Ident, Token![,]>,
}

impl Parse for Commands {
    fn parse(input: ParseStream) -> Result<Self> {
        let host = input.parse()?;
        let generics = input.parse()?;
        input.parse::<Token![:]>()?;
        let commands = Punctuated::parse_terminated(input)?;
        Ok(Commands {
            host,
            generics,
            commands,
        })
    }
}

impl Commands {
    fn expand(&self) -> TokenStream2 {
        let host = &self.host;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let names: Vec<String> = self
            .commands
            .iter()
            .map(|command| camel(&command.to_string()))
            .collect();
        let calls = self.commands.iter();
        quote! {
            impl #impl_generics ::brul::Host for #host #ty_generics #where_clause {
                fn call(
                    &mut self,
                    name: &str,
                    arguments: &[::brul::Value],
                ) -> ::std::result::Result<::brul::Value, ::brul::Error> {
                    match name {
                        #(#names => #calls(self, arguments),)*
                        _ => ::std::result::Result::Err(
                            ::brul::Error::UnknownFunction(name.to_owned()),
                        ),
                    }
                }

                fn commands(&self) -> ::std::vec::Vec<&'static str> {
                    ::std::vec![#(#names),*]
                }
            }
        }
    }
}

fn result_type(output: &ReturnType) -> Option<&Type> {
    let ReturnType::Type(_, ty) = output else {
        return None;
    };
    let Type::Path(path) = &**ty else {
        return Some(ty);
    };
    let Some(segment) = path.path.segments.last() else {
        return Some(ty);
    };
    if segment.ident != "Result" {
        return Some(ty);
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Some(ty);
    };
    arguments.args.iter().find_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    })
}

fn kind(ty: &Type) -> Kind {
    match ty {
        Type::Reference(reference) => kind(&reference.elem),
        Type::Path(path) => match path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .as_deref()
        {
            Some("str" | "String") => Kind::String,
            Some("f64" | "f32" | "usize" | "u64" | "u32" | "i64" | "i32") => Kind::Number,
            Some("bool") => Kind::Bool,
            Some("Value") => Kind::Value,
            _ => Kind::Other,
        },
        Type::Tuple(tuple) if tuple.elems.is_empty() => Kind::Null,
        _ => Kind::Other,
    }
}

enum Kind {
    String,
    Number,
    Bool,
    Value,
    Null,
    Other,
}

fn marker_type(ty: &Type) -> TokenStream2 {
    match kind(ty) {
        Kind::String => quote!(::std::string::String),
        Kind::Number => quote!(f64),
        Kind::Bool => quote!(bool),
        Kind::Null => quote!(()),
        Kind::Value | Kind::Other => quote!(::brul::Value),
    }
}

fn argument_value(argument: &Ident, ty: &Type, command: &str) -> TokenStream2 {
    match kind(ty) {
        Kind::String if matches!(ty, Type::Reference(_)) => quote!(#argument.as_str()),
        Kind::String | Kind::Bool | Kind::Null | Kind::Value => quote!(#argument),
        Kind::Number => quote!(#argument as #ty),
        Kind::Other => quote! {
            match <#ty as ::std::convert::TryFrom<&::brul::Value>>::try_from(&#argument) {
                ::std::result::Result::Ok(value) => value,
                ::std::result::Result::Err(error) => {
                    return ::std::result::Result::Err(
                        ::brul::Error::at_command(error, #command),
                    );
                }
            }
        },
    }
}

fn output_value(ty: &Type) -> TokenStream2 {
    match kind(ty) {
        Kind::String if matches!(ty, Type::Reference(_)) => quote!(Ok(value.to_owned())),
        _ => quote!(Ok(value)),
    }
}

fn owned_type(ty: &Type) -> &Type {
    match ty {
        Type::Reference(reference) => &reference.elem,
        _ => ty,
    }
}

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
        let signature = signature(&function).expect("signature");
        assert_eq!(signature.values.len(), 1);
    }
}

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{FnArg, Generics, Ident, ItemFn, Pat, PatType, Result, Token, Type};

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
        kind(ty)?;
        values.push((name.ident.clone(), (**ty).clone()));
    }
    Ok(Signature {
        host: Pat::Ident(host.clone()),
        host_type: (**ty).clone(),
        values,
    })
}

enum Kind {
    Text,
    Owned,
    Number,
    Flag,
    Index,
}

fn kind(ty: &Type) -> Result<Kind> {
    if let Type::Reference(reference) = ty
        && let Type::Path(path) = &*reference.elem
        && path.path.is_ident("str")
    {
        return Ok(Kind::Text);
    }
    let Type::Path(path) = ty else {
        return Err(syn::Error::new_spanned(ty, "unsupported argument type"));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new_spanned(ty, "unsupported argument type"));
    };
    if !matches!(segment.arguments, syn::PathArguments::None) {
        return Err(syn::Error::new_spanned(ty, "unsupported argument type"));
    }
    match segment.ident.to_string().as_str() {
        "String" => Ok(Kind::Owned),
        "f64" => Ok(Kind::Number),
        "bool" => Ok(Kind::Flag),
        "usize" => Ok(Kind::Index),
        _ => Err(syn::Error::new_spanned(
            ty,
            "a command argument is &str, String, f64, bool or usize",
        )),
    }
}

fn conversion(kind: Kind, value: &Ident, command: &str) -> TokenStream2 {
    let (matched, extracted, expected) = match kind {
        Kind::Text => (
            quote!(::brul::Value::String(text)),
            quote!(&**text),
            "String",
        ),
        Kind::Owned => (
            quote!(::brul::Value::String(text)),
            quote!(::std::string::ToString::to_string(text)),
            "String",
        ),
        Kind::Number => (
            quote!(::brul::Value::Number(number)),
            quote!(*number),
            "Number",
        ),
        Kind::Flag => (quote!(::brul::Value::Bool(flag)), quote!(*flag), "Bool"),
        Kind::Index => (
            quote!(::brul::Value::Number(number) if number.fract() == 0.0 && *number >= 0.0),
            quote!(*number as usize),
            "a non-negative integer",
        ),
    };
    quote! {
        match #value {
            #matched => #extracted,
            value => {
                return ::std::result::Result::Err(::brul::Error::Argument {
                    name: #command,
                    expected: #expected,
                    found: value.type_name(),
                });
            }
        }
    }
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
            let kind = kind(ty)?;
            let conversion = conversion(kind, &values[index], &command);
            Ok(quote! { let #name = #conversion; })
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

    Ok(quote! {
        #(#attrs)*
        #vis fn #name #declare (#host: #host_type, __arguments: &[::brul::Value]) #output {
            #arity
            #hidden(#host #(, #names)*)
        }

        #(#attrs)*
        #vis fn #hidden #declare (#host: #host_type, #(#names: #types),*) #output #body
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
            }
        }
    }
}

fn camel(name: &str) -> String {
    let name = name.trim_start_matches("r#");
    let mut output = String::with_capacity(name.len());
    let mut capitalize = false;
    for character in name.chars() {
        if character == '_' {
            capitalize = true;
        } else if capitalize {
            output.extend(character.to_uppercase());
            capitalize = false;
        } else {
            output.push(character);
        }
    }
    output
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
    fn a_text_argument_is_recognized() {
        assert!(matches!(
            kind(&syn::parse_str("&str").expect("type")),
            Ok(Kind::Text)
        ));
    }

    #[test]
    fn an_argument_of_an_unknown_type_is_rejected() {
        assert!(kind(&syn::parse_str("::brul::Value").expect("type")).is_err());
    }
}

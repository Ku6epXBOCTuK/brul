use brul_lang_schema::{TYPES, TypeSpec};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse_str;

fn ident(name: &str) -> syn::Ident {
    syn::Ident::new(name, Span::call_site())
}

fn pattern(spec: &TypeSpec) -> TokenStream {
    let variant = ident(spec.variant);
    if spec.payload.is_some() {
        quote!(Self::#variant(_))
    } else {
        quote!(Self::#variant)
    }
}

fn variant(spec: &TypeSpec) -> syn::Ident {
    ident(spec.variant)
}

pub fn types() -> TokenStream {
    let variants = TYPES.iter().map(|spec| {
        let variant = variant(spec);
        match spec.payload {
            Some(payload) => {
                let payload: syn::Type = parse_str(payload).expect("type payload parses");
                quote! { #variant(#payload), }
            }
            None => quote! { #variant, },
        }
    });

    let from_arms = TYPES
        .iter()
        .filter(|spec| spec.payload.is_none())
        .map(|spec| {
            let name = spec.name;
            let variant = variant(spec);
            quote! { #name => Self::#variant, }
        });

    let name_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let name = spec.name;
        quote! { #pattern => #name, }
    });

    let path_arms = TYPES
        .iter()
        .filter(|spec| spec.payload.is_none())
        .map(|spec| {
            let pattern = pattern(spec);
            let path = format!("PropType::{}", spec.variant);
            quote! { #pattern => #path.to_owned(), }
        });
    let path_array = quote! {
        Self::Array(item) => format!("PropType::Array(Box::new({}))", item.rust_path()),
    };

    let rust_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let rust = spec.rust;
        quote! { #pattern => #rust, }
    });
    let local_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let local = spec.local;
        quote! { #pattern => #local, }
    });

    let template_arms = |field: fn(&TypeSpec) -> &'static str| -> Vec<TokenStream> {
        TYPES
            .iter()
            .map(|spec| {
                let pattern = pattern(spec);
                let value = field(spec);
                quote! { #pattern => #value, }
            })
            .collect()
    };

    let wrap_arms = template_arms(|spec| spec.dyn_wrap);
    let wrap_local_arms = template_arms(|spec| spec.dyn_local);
    let convert_arms = template_arms(|spec| spec.convert);
    let default_arms = template_arms(|spec| spec.default);
    let expression_arms = template_arms(|spec| spec.prop_expression);

    let from_dyn_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        match spec.from_dyn {
            Some(value) => quote! { #pattern => Some(#value), },
            None => quote! { #pattern => None, },
        }
    });
    let clone_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let value = spec.clone_on_read;
        quote! { #pattern => #value, }
    });
    let value_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let names = spec.value_names;
        quote! { #pattern => [#(#names),*].contains(&name), }
    });
    let is_value_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let value = spec.is_value;
        quote! { #pattern => #value, }
    });
    let void_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let value = spec.variant == "Void";
        quote! { #pattern => #value, }
    });
    let scalar_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let value = matches!(spec.variant, "Number" | "String" | "Bool" | "Timer");
        quote! { #pattern => #value, }
    });
    let state_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        let value = spec.state;
        quote! { #pattern => #value, }
    });
    let state_read_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        if spec.state {
            let value = spec.state_read;
            quote! { #pattern => Some(#value.replace("{state}", state)), }
        } else {
            quote! { #pattern => None, }
        }
    });
    let state_write_arms = TYPES.iter().map(|spec| {
        let pattern = pattern(spec);
        if spec.state {
            let value = spec.state_write;
            quote! {
                #pattern => Some(#value.replace("{state}", state).replace("{value}", value)),
            }
        } else {
            quote! { #pattern => None, }
        }
    });

    quote! {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Type {
            #(#variants)*
        }

        impl Type {
            pub fn from_name(name: &str) -> Option<Self> {
                Some(match name {
                    #(#from_arms)*
                    _ => return None,
                })
            }

            pub fn rust_path(&self) -> String {
                match self {
                    #path_array
                    #(#path_arms)*
                }
            }

            pub fn rust_type(&self) -> &'static str {
                match self {
                    #(#rust_arms)*
                }
            }

            pub fn local_type(&self) -> &'static str {
                match self {
                    #(#local_arms)*
                }
            }

            pub fn dyn_wrap(&self, code: &str) -> String {
                match self {
                    #(#wrap_arms)*
                }
                .replace("{code}", code)
            }

            pub fn dyn_wrap_local(&self, code: &str) -> String {
                match self {
                    #(#wrap_local_arms)*
                }
                .replace("{code}", code)
            }

            pub fn convert(&self, code: &str) -> String {
                match self {
                    #(#convert_arms)*
                }
                .replace("{code}", code)
            }

            pub fn from_dyn(&self) -> Option<&'static str> {
                match self {
                    #(#from_dyn_arms)*
                }
            }

            pub fn clone_on_read(&self) -> bool {
                match self {
                    #(#clone_arms)*
                }
            }

            pub fn is_value(&self) -> bool {
                match self {
                    #(#is_value_arms)*
                }
            }

            pub fn is_void(&self) -> bool {
                match self {
                    #(#void_arms)*
                }
            }

            pub fn is_scalar(&self) -> bool {
                match self {
                    #(#scalar_arms)*
                }
            }

            pub fn accepts_value(&self, name: &str) -> bool {
                match self {
                    #(#value_arms)*
                }
            }

            pub fn scope_default(&self) -> &'static str {
                match self {
                    #(#default_arms)*
                }
            }

            pub fn prop_expression(&self) -> &'static str {
                match self {
                    #(#expression_arms)*
                }
            }

            pub fn supports_state(&self) -> bool {
                match self {
                    #(#state_arms)*
                }
            }

            pub fn state_read(&self, state: &str) -> Option<String> {
                match self {
                    #(#state_read_arms)*
                }
            }

            pub fn state_write(&self, state: &str, value: &str) -> Option<String> {
                match self {
                    #(#state_write_arms)*
                }
            }
        }

        impl TypeName for Type {
            fn type_name(&self) -> &'static str {
                match self {
                    #(#name_arms)*
                }
            }
        }
    }
}

use std::collections::HashSet;

use brul_style_schema::PROPERTIES;
use proc_macro2::TokenStream;
use quote::quote;

use crate::tokens::{kind, spec, variant};

pub(crate) fn style() -> TokenStream {
    let mut variants = Vec::new();
    let mut seen = HashSet::new();
    for spec in PROPERTIES {
        let (name, sided) = variant(spec.key);
        if seen.insert(name.to_string()) {
            variants.push(if sided {
                quote!(#name(Side))
            } else {
                quote!(#name)
            });
        }
    }
    let specs = PROPERTIES.iter().map(spec);
    let kinds = PROPERTIES.iter().map(|spec| kind(spec.key));
    quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Kind {
            #(#variants),*
        }

        pub static SPECS: &[Spec] = &[#(#specs),*];

        impl Kind {
            pub const ALL: &'static [Kind] = &[#(#kinds),*];

            pub fn spec(self) -> &'static Spec {
                SPECS.iter().find(|spec| spec.kind == self).expect("SPECS lists every kind")
            }

            pub fn name(self) -> &'static str {
                self.spec().name
            }

            pub fn from_name(name: &str) -> Option<Kind> {
                SPECS.iter().find(|spec| spec.name == name).map(|spec| spec.kind)
            }

            pub fn theme_type(self) -> Option<ThemeType> {
                self.spec().theme_type()
            }

            pub fn in_state(self) -> StatePolicy {
                self.spec().in_state
            }

            pub fn target(self) -> Target {
                self.spec().target
            }

            pub fn animatable(self) -> bool {
                self.spec().animatable
            }
        }
    }
}

pub(crate) fn codegen() -> TokenStream {
    let mut arms = Vec::new();
    let mut seen = HashSet::new();
    for spec in PROPERTIES {
        let (name, sided) = variant(spec.key);
        if !seen.insert(name.to_string()) {
            continue;
        }
        let arm = if sided {
            format!("Kind::{name}(side) => format!(\"s::Kind::{name}(s::Side::{{side:?}})\"),")
        } else {
            format!("Kind::{name} => \"s::Kind::{name}\".into(),")
        };
        arms.push(
            arm.parse::<TokenStream>()
                .expect("generated match arm parses"),
        );
    }
    quote! {
        fn kind(value: Kind) -> String {
            match value {
                #(#arms)*
            }
        }
    }
}

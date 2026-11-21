use std::collections::HashSet;
use std::fs;
use std::path::Path;

use brul_style_schema::{
    LayoutField, PROPERTIES, PropertyKey, PropertySpec, SHORTHANDS, ShorthandParser, Side,
    StatePolicy, StyleField, Target, Type,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub enum Output {
    Style,
    Syntax,
    Codegen,
}

pub fn generate(output: Output, out_dir: &Path) {
    let (file, tokens) = match output {
        Output::Style => ("style_properties.rs", style()),
        Output::Syntax => ("style_shorthands.rs", syntax()),
        Output::Codegen => ("style_kind.rs", codegen()),
    };
    let parsed = syn::parse2::<syn::File>(tokens).expect("generated style code parses");
    fs::write(out_dir.join(file), prettyplease::unparse(&parsed))
        .expect("cannot write generated style code");
}

fn style() -> TokenStream {
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

fn syntax() -> TokenStream {
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

fn codegen() -> TokenStream {
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

fn spec(spec: &PropertySpec) -> TokenStream {
    let kind = kind(spec.key);
    let name = spec.name;
    let ty = ty(&spec.ty);
    let state = state(spec.in_state);
    let target = target(spec.target);
    let animatable = spec.animatable;
    quote! {
        Spec {
            kind: #kind,
            name: #name,
            ty: #ty,
            in_state: #state,
            target: #target,
            animatable: #animatable,
        }
    }
}

fn variant(key: PropertyKey) -> (proc_macro2::Ident, bool) {
    match key {
        PropertyKey::PaddingSide(_) => (format_ident!("PaddingSide"), true),
        PropertyKey::BorderWidthSide(_) => (format_ident!("BorderWidthSide"), true),
        PropertyKey::BorderColorSide(_) => (format_ident!("BorderColorSide"), true),
        key => (format_ident!("{key:?}"), false),
    }
}

fn kind(key: PropertyKey) -> TokenStream {
    let (name, sided) = variant(key);
    if sided {
        let side = side_ident(match key {
            PropertyKey::PaddingSide(side)
            | PropertyKey::BorderWidthSide(side)
            | PropertyKey::BorderColorSide(side) => side,
            _ => unreachable!("only three property keys are sided"),
        });
        quote!(Kind::#name(Side::#side))
    } else {
        quote!(Kind::#name)
    }
}

fn side_ident(side: Side) -> proc_macro2::Ident {
    format_ident!("{side:?}")
}

fn ty(ty: &Type) -> TokenStream {
    match ty {
        Type::Keywords(names) => quote!(Type::Keywords(&[#(#names),*])),
        Type::NumberRange(from, to) => quote!(Type::NumberRange(#from, #to)),
        Type::NumberIn(numbers) => quote!(Type::NumberIn(&[#(#numbers),*])),
        ty => {
            let name = format_ident!("{ty:?}");
            quote!(Type::#name)
        }
    }
}

fn state(policy: StatePolicy) -> TokenStream {
    let name = format_ident!("{policy:?}");
    quote!(StatePolicy::#name)
}

fn target(target: Target) -> TokenStream {
    match target {
        Target::Layout(field) => {
            let field = layout(field);
            quote!(Target::Layout(#field))
        }
        Target::Style(field) => {
            let field = style_field(field);
            quote!(Target::Style(#field))
        }
        Target::Text(field) => {
            let name = format_ident!("{field:?}");
            quote!(Target::Text(TextField::#name))
        }
    }
}

fn layout(field: LayoutField) -> TokenStream {
    match field {
        LayoutField::Padding(side) => {
            let side = side_ident(side);
            quote!(LayoutField::Padding(Side::#side))
        }
        LayoutField::BorderWidthSide(side) => {
            let side = side_ident(side);
            quote!(LayoutField::BorderWidthSide(Side::#side))
        }
        field => {
            let name = format_ident!("{field:?}");
            quote!(LayoutField::#name)
        }
    }
}

fn style_field(field: StyleField) -> TokenStream {
    match field {
        StyleField::BorderSide(side) => {
            let side = side_ident(side);
            quote!(StyleField::BorderSide(Side::#side))
        }
        field => {
            let name = format_ident!("{field:?}");
            quote!(StyleField::#name)
        }
    }
}

fn parser(parser: ShorthandParser) -> proc_macro2::Ident {
    match parser {
        ShorthandParser::Box => format_ident!("box_values"),
        ShorthandParser::Border => format_ident!("border_values"),
        ShorthandParser::BorderColor => format_ident!("color_values"),
        ShorthandParser::Shadow => format_ident!("shadow_values"),
    }
}

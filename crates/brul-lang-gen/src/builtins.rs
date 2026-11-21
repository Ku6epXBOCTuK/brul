use brul_lang_schema::{ArgSpec, BUILTINS, EmitSpec};
use proc_macro2::{Span, TokenStream};
use quote::quote;

pub fn builtins() -> TokenStream {
    let entries = BUILTINS.iter().map(|spec| {
        let name = spec.name;
        let arguments = spec.arguments.iter().map(|argument| match argument {
            ArgSpec::Any => quote!(Argument::Any),
            ArgSpec::Function => quote!(Argument::Function),
            ArgSpec::Value(type_name) => {
                let ty = syn::Ident::new(type_name, Span::call_site());
                quote!(Argument::Value(Type::#ty))
            }
        });
        let result = syn::Ident::new(spec.result, Span::call_site());
        let kind = match spec.emit {
            EmitSpec::Host(marker) => quote!(BuiltinKind::Host { marker: #marker }),
            EmitSpec::StringFn => quote!(BuiltinKind::StringFn),
            EmitSpec::NumberFn => quote!(BuiltinKind::NumberFn),
            EmitSpec::Len => quote!(BuiltinKind::Len),
            EmitSpec::Every => quote!(BuiltinKind::Every),
            EmitSpec::After => quote!(BuiltinKind::After),
            EmitSpec::Cancel => quote!(BuiltinKind::Cancel),
        };
        quote! {
            Builtin {
                name: #name,
                arguments: &[#(#arguments),*],
                result: Type::#result,
                kind: #kind,
            },
        }
    });

    quote! {
        pub const BUILTINS: &[Builtin] = &[
            #(#entries)*
        ];
    }
}

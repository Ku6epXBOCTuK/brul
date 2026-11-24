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

fn expand_attach(arguments: brul_capability::AttachAttr, function: ItemFn) -> Result<TokenStream2> {
    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &function.sig.generics,
            "an attachment cannot be generic",
        ));
    }
    let mut inputs = function.sig.inputs.iter();
    if !matches!(inputs.next(), Some(FnArg::Typed(_))) {
        return Err(syn::Error::new_spanned(
            &function.sig,
            "an attachment takes a context first",
        ));
    }
    let mut names = Vec::new();
    let mut types = Vec::new();
    for input in inputs {
        let FnArg::Typed(argument) = input else {
            return Err(syn::Error::new_spanned(
                &function.sig,
                "an attachment has no receiver",
            ));
        };
        if !matches!(
            brul_command::kind(&argument.ty),
            Kind::String | Kind::Number | Kind::Bool
        ) {
            return Err(syn::Error::new_spanned(
                &argument.ty,
                "an attachment param must be Number, String or Bool",
            ));
        }
        let Pat::Ident(pattern) = &*argument.pat else {
            return Err(syn::Error::new_spanned(
                &argument.pat,
                "an attachment param needs a name",
            ));
        };
        names.push(pattern.ident.clone());
        types.push((*argument.ty).clone());
    }
    let state_ty = attachment_state_type(&function.sig.output)?;
    let name = &function.sig.ident;
    let shims = brul_capability::attach_shims(name);
    let apply = shims.apply;
    let event = shims.event;
    let raw_event = shims.raw_event;
    let cleanup = shims.cleanup;
    let indices = 0..names.len();
    let reads = indices
        .clone()
        .zip(types.iter())
        .map(|(index, ty)| {
            let value = format_ident!("param_{index}");
            quote! { let #value: #ty = params[#index].clone().try_into()?; }
        })
        .collect::<Vec<_>>();
    let values = (0..names.len())
        .map(|index| format_ident!("param_{index}"))
        .collect::<Vec<_>>();
    let update = match &arguments.update {
        Some(hook) => {
            quote! { #hook(&mut *state, ctx, #(Clone::clone(&#values)),*)?; let _ = &*state; }
        }
        None => quote! { let _ = (&mut *state, &*ctx, #(#values),*); },
    };
    let event_call = match &arguments.event {
        Some(hook) => quote! {
            if #hook(&mut *state, ctx, event) {
                ::brul::Dispatch::Stop
            } else {
                ::brul::Dispatch::Continue
            }
        },
        None => quote! {
            let _ = (&mut *state, &*ctx, event);
            ::brul::Dispatch::Continue
        },
    };
    let raw_event_call = match &arguments.raw_event {
        Some(hook) => quote! {
            if #hook(&mut *state, ctx, event) {
                ::brul::Dispatch::Stop
            } else {
                ::brul::Dispatch::Continue
            }
        },
        None => quote! {
            let _ = (&mut *state, &*ctx, event);
            ::brul::Dispatch::Continue
        },
    };
    let cleanup_call = match &arguments.cleanup {
        Some(hook) => quote! { #hook(&mut *state, ctx); },
        None => quote! { let _ = (&mut *state, &*ctx); },
    };
    Ok(quote! {
        #function

        pub fn #apply(
            ctx: &mut ::brul::Build,
            _state: ::brul::StateId,
            params: &[::brul::Dyn],
        ) -> Result<(), ::brul::ViewError> {
            #(#reads)*
            if ::brul::Build::get_state::<#state_ty>(&*ctx).is_none() {
                let state = #name(ctx, #(Clone::clone(&#values)),*)?;
                ctx.set_state(state);
            }
            let cell = ::brul::Build::get_state::<#state_ty>(&*ctx)
                .expect("attachment state was just created");
            if let Ok(mut state) = cell.try_borrow_mut() {
                #update
            }
            Ok(())
        }

        pub fn #event(
            ctx: &mut ::brul::Build,
            _state: ::brul::StateId,
            event: &::brul::UiEvent,
        ) -> ::brul::Dispatch {
            let Some(cell) = ::brul::Build::get_state::<#state_ty>(&*ctx) else {
                return ::brul::Dispatch::Continue;
            };
            let Ok(mut state) = cell.try_borrow_mut() else {
                return ::brul::Dispatch::Continue;
            };
            #event_call
        }

        pub fn #raw_event(
            ctx: &mut ::brul::Build,
            _state: ::brul::StateId,
            event: &::brul::RawUiEvent,
        ) -> ::brul::Dispatch {
            let Some(cell) = ::brul::Build::get_state::<#state_ty>(&*ctx) else {
                return ::brul::Dispatch::Continue;
            };
            let Ok(mut state) = cell.try_borrow_mut() else {
                return ::brul::Dispatch::Continue;
            };
            #raw_event_call
        }

        pub fn #cleanup(ctx: &mut ::brul::Build, _state: ::brul::StateId) {
            if let Some(cell) = ::brul::Build::get_state::<#state_ty>(&*ctx)
                && let Ok(mut state) = cell.try_borrow_mut()
            {
                #cleanup_call
            }
            ctx.remove_state();
        }
    })
}

fn attachment_state_type(output: &ReturnType) -> Result<Type> {
    let ReturnType::Type(_, ty) = output else {
        return Err(syn::Error::new_spanned(
            output,
            "an attachment returns a state",
        ));
    };
    let Type::Path(path) = &**ty else {
        return Err(syn::Error::new_spanned(ty, "an attachment returns a state"));
    };
    let segment = path.path.segments.last().expect("result type");
    if segment.ident != "Result" {
        return Ok((**ty).clone());
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(syn::Error::new_spanned(
            ty,
            "an attachment returns Result<State, Error>",
        ));
    };
    let Some(syn::GenericArgument::Type(state)) = arguments.args.first() else {
        return Err(syn::Error::new_spanned(
            ty,
            "an attachment returns Result<State, Error>",
        ));
    };
    Ok(state.clone())
}

fn expand_widget(function: ItemFn) -> Result<TokenStream2> {
    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &function.sig.generics,
            "a widget cannot be generic",
        ));
    }
    let mut inputs = function.sig.inputs.iter();
    for _ in 0..3 {
        if !matches!(inputs.next(), Some(FnArg::Typed(_))) {
            return Err(syn::Error::new_spanned(
                &function.sig,
                "a widget takes theme, build and component first",
            ));
        }
    }
    let mut names = Vec::new();
    let mut bindings = Vec::new();
    for input in inputs {
        let FnArg::Typed(argument) = input else {
            return Err(syn::Error::new_spanned(input, "a widget has no receiver"));
        };
        let Pat::Ident(pattern) = &*argument.pat else {
            return Err(syn::Error::new_spanned(
                &*argument.pat,
                "a widget prop needs a name",
            ));
        };
        let name = &pattern.ident;
        let key = LitStr::new(&name.to_string(), name.span());
        let ty = &*argument.ty;
        let binding = match brul_command::kind(ty) {
            Kind::String if matches!(ty, Type::Reference(_)) => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "a widget prop cannot be borrowed",
                ));
            }
            Kind::String => quote! {
                let #name: String = component.borrow_mut().seed(#key, __position)?;
            },
            Kind::Number => quote! {
                let #name: #ty = component.borrow_mut().seed::<f64>(#key, __position)? as #ty;
            },
            Kind::Bool => quote! {
                let #name: bool = component.borrow_mut().seed(#key, __position)?;
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "a widget prop must be Number, String or Bool",
                ));
            }
        };
        names.push(name);
        bindings.push(binding);
    }
    let name = function.sig.ident.clone();
    let wrapper = brul_capability::widget_shim(&name);
    let call = quote! { #name(theme, build, component #(, #names)*) };
    let result = if is_result(&function.sig.output) {
        call
    } else {
        quote! { ::std::result::Result::Ok(#call) }
    };
    let attrs = &function.attrs;
    let visibility = &function.vis;
    let signature = &function.sig;
    let body = &function.block;
    Ok(quote! {
        #(#attrs)*
        #visibility #signature #body

        #[doc(hidden)]
        pub fn #wrapper(
            theme: Option<&str>,
            build: &mut ::brul::Build,
            component: &::brul::ComponentHandle,
        ) -> ::std::result::Result<::brul::Built, ::brul::ViewError> {
            let __position = ::brul::ast::Position { line: 0, column: 0 };
            #(#bindings)*
            #result
        }
    })
}

fn is_result(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(path) = &**ty else {
        return false;
    };
    path.path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Result")
}

struct EffectDefinition {
    name: syn::LitStr,
    shader: syn::LitStr,
    animated: bool,
    item: syn::ItemStruct,
}

impl Parse for EffectDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: Ident = input.parse()?;
        if key != "name" {
            return Err(syn::Error::new(key.span(), "expected `name`"));
        }
        input.parse::<Token![:]>()?;
        let name: syn::LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let key: Ident = input.parse()?;
        if key != "shader" {
            return Err(syn::Error::new(key.span(), "expected `shader`"));
        }
        input.parse::<Token![:]>()?;
        let shader: syn::LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let mut animated = false;
        let ahead = input.fork();
        if let Ok(key) = ahead.parse::<Ident>()
            && key == "animated"
        {
            ahead.parse::<Token![:]>()?;
            animated = ahead.parse::<syn::LitBool>()?.value;
            ahead.parse::<Token![,]>()?;
            input.advance_to(&ahead);
        }
        let item: syn::ItemStruct = input.parse()?;
        if !item.generics.params.is_empty() {
            return Err(syn::Error::new_spanned(
                &item.generics,
                "effect parameters cannot be generic",
            ));
        }
        Ok(EffectDefinition {
            name,
            shader,
            animated,
            item,
        })
    }
}

#[derive(Clone, Copy)]
enum FieldLayout {
    Float,
    Float2,
    Float3,
    Float4,
    Float4Array(usize),
}

impl FieldLayout {
    fn from_type(ty: &Type) -> Result<Self> {
        if is_float(ty) {
            return Ok(Self::Float);
        }
        let syn::Type::Array(array) = ty else {
            return Err(unsupported(ty));
        };
        if is_float(&array.elem) {
            return match array_length(&array.len)? {
                2 => Ok(Self::Float2),
                3 => Ok(Self::Float3),
                4 => Ok(Self::Float4),
                _ => Err(unsupported(ty)),
            };
        }
        if let syn::Type::Array(rows) = &*array.elem
            && is_float(&rows.elem)
            && array_length(&rows.len)? == 4
        {
            return Ok(Self::Float4Array(array_length(&array.len)?));
        }
        Err(unsupported(ty))
    }

    fn slots(self) -> usize {
        match self {
            Self::Float | Self::Float2 | Self::Float3 | Self::Float4 => 1,
            Self::Float4Array(length) => length,
        }
    }

    fn width(self) -> usize {
        match self {
            Self::Float => 1,
            Self::Float2 => 2,
            Self::Float3 => 3,
            Self::Float4 | Self::Float4Array(_) => 4,
        }
    }

    fn effect_type(self) -> TokenStream2 {
        match self {
            Self::Float => quote!(::brul::effect::EffectType::Float),
            Self::Float2 => quote!(::brul::effect::EffectType::Float2),
            Self::Float3 => quote!(::brul::effect::EffectType::Float3),
            Self::Float4 => quote!(::brul::effect::EffectType::Float4),
            Self::Float4Array(_) => quote!(::brul::effect::EffectType::Float4Array),
        }
    }

    fn write(self, field: &Ident, at: usize, out: &mut Vec<TokenStream2>) {
        match self {
            Self::Float => out.push(quote!(slots[#at][0] = self.#field;)),
            Self::Float2 | Self::Float3 | Self::Float4 => {
                let length = self.width();
                out.push(quote!(slots[#at][0..#length].copy_from_slice(&self.#field);));
            }
            Self::Float4Array(_) => out.push(quote! {
                for (index, row) in self.#field.iter().enumerate() {
                    slots[#at + index] = *row;
                }
            }),
        }
    }
}

fn is_float(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => path.qself.is_none() && path.path.is_ident("f32"),
        _ => false,
    }
}

fn array_length(length: &syn::Expr) -> Result<usize> {
    match length {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(value),
            ..
        }) => value.base10_parse(),
        _ => Err(syn::Error::new_spanned(
            length,
            "effect array length must be a literal",
        )),
    }
}

fn unsupported(ty: &Type) -> syn::Error {
    syn::Error::new_spanned(
        ty,
        "effect field must be f32, [f32; 2..=4] or [[f32; 4]; N]",
    )
}

fn expand_effect(definition: EffectDefinition) -> Result<TokenStream2> {
    let name = &definition.name;
    let shader = &definition.shader;
    let animated = definition.animated;
    let ident = &definition.item.ident;
    let syn::Fields::Named(fields) = &definition.item.fields else {
        return Err(syn::Error::new_spanned(
            &definition.item,
            "effect parameters must have named fields",
        ));
    };

    let mut layouts = Vec::new();
    let mut writes = Vec::new();
    let mut slots = 0usize;
    for field in &fields.named {
        let name = field.ident.as_ref().expect("named field");
        let layout = FieldLayout::from_type(&field.ty)?;
        layout.write(name, slots, &mut writes);
        layouts.push((name, layout));
        slots += layout.slots();
    }

    let keys: Vec<_> = layouts.iter().map(|(field, _)| *field).collect();
    let types: Vec<_> = layouts
        .iter()
        .map(|(_, layout)| layout.effect_type())
        .collect();
    let counts: Vec<_> = layouts
        .iter()
        .map(|(_, layout)| layout.slots() as u16)
        .collect();
    let marker = format_ident!("BRUL_EFFECT_{}", constant(&name.value()));
    let mut item = definition.item.clone();
    item.attrs
        .push(syn::parse_quote!(#[derive(Clone, Copy, Debug, Default)]));

    Ok(quote! {
        #item

        impl ::brul::effect::EffectParams for #ident {
            const NAME: &'static str = #name;
            const SHADER: &'static str = include_str!(#shader);
            const ANIMATED: bool = #animated;
            const SLOTS: u16 = #slots as u16;
            const FIELDS: &'static [::brul::effect::EffectField] = &[
                #(
                    ::brul::effect::EffectField {
                        name: stringify!(#keys),
                        ty: #types,
                        slots: #counts,
                    },
                )*
            ];

            fn pack(&self) -> [[f32; 4]; ::brul::effect::EFFECT_SLOTS] {
                const _: () = assert!(#slots <= ::brul::effect::EFFECT_SLOTS);
                let mut slots = [[0.0f32; 4]; ::brul::effect::EFFECT_SLOTS];
                #(#writes)*
                slots
            }
        }

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        pub const #marker: ::brul::effect::EffectDef =
            <#ident as ::brul::effect::EffectParams>::DEF;
    })
}

fn expand(function: ItemFn) -> Result<TokenStream2> {
    let signature = brul_command::parse(&function)?;
    let name = &function.sig.ident;
    let hidden = format_ident!("__{}", name);
    let command = camel(&name.to_string());
    let attrs = &function.attrs;
    let vis = &function.vis;
    let generics = &function.sig.generics;
    let where_clause = &generics.where_clause;
    let declare = quote! { #generics #where_clause };
    let output = &function.sig.output;
    let host = signature.host;
    let host_type = signature.host_type;
    let body = &function.block;

    let values: Vec<Ident> = (0..signature.params.len())
        .map(|index| format_ident!("__value{index}"))
        .collect();
    let names: Vec<&Ident> = signature.params.iter().map(|(name, _)| *name).collect();
    let types: Vec<&Type> = signature.params.iter().map(|(_, ty)| *ty).collect();
    let conversions: Vec<TokenStream2> = signature
        .params
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
        .params
        .iter()
        .map(|(_, ty)| marker_type(ty))
        .collect();
    let argument_names: Vec<Ident> = (0..signature.params.len())
        .map(|index| format_ident!("__argument{index}"))
        .collect();
    let bindings: Vec<TokenStream2> = signature
        .params
        .iter()
        .zip(&values)
        .zip(&argument_names)
        .map(|(((_, ty), value), argument)| {
            let conversion = argument_value(argument, ty, &command);
            quote! { let #value: #ty = #conversion; }
        })
        .collect();
    let host_owned = owned_type(signature.host_type);
    let result = brul_command::result_type(signature.output);
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

fn marker_type(ty: &Type) -> TokenStream2 {
    match brul_command::kind(ty) {
        Kind::String => quote!(::std::string::String),
        Kind::Number => quote!(f64),
        Kind::Bool => quote!(bool),
        Kind::Null => quote!(()),
        Kind::Value | Kind::Other => quote!(::brul::Value),
    }
}

fn argument_value(argument: &Ident, ty: &Type, command: &str) -> TokenStream2 {
    match brul_command::kind(ty) {
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
    match brul_command::kind(ty) {
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

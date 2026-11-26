use super::*;
pub(super) fn expand_widget(function: ItemFn) -> Result<TokenStream2> {
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

pub(super) fn is_result(output: &ReturnType) -> bool {
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

pub(super) struct EffectDefinition {
    pub(super) name: syn::LitStr,
    pub(super) shader: syn::LitStr,
    pub(super) animated: bool,
    pub(super) item: syn::ItemStruct,
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
pub(super) enum FieldLayout {
    Float,
    Float2,
    Float3,
    Float4,
    Float4Array(usize),
}

impl FieldLayout {
    pub(super) fn from_type(ty: &Type) -> Result<Self> {
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

    pub(super) fn slots(self) -> usize {
        match self {
            Self::Float | Self::Float2 | Self::Float3 | Self::Float4 => 1,
            Self::Float4Array(length) => length,
        }
    }

    pub(super) fn width(self) -> usize {
        match self {
            Self::Float => 1,
            Self::Float2 => 2,
            Self::Float3 => 3,
            Self::Float4 | Self::Float4Array(_) => 4,
        }
    }

    pub(super) fn effect_type(self) -> TokenStream2 {
        match self {
            Self::Float => quote!(::brul::effect::EffectType::Float),
            Self::Float2 => quote!(::brul::effect::EffectType::Float2),
            Self::Float3 => quote!(::brul::effect::EffectType::Float3),
            Self::Float4 => quote!(::brul::effect::EffectType::Float4),
            Self::Float4Array(_) => quote!(::brul::effect::EffectType::Float4Array),
        }
    }

    pub(super) fn write(self, field: &Ident, at: usize, out: &mut Vec<TokenStream2>) {
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

pub(super) fn is_float(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => path.qself.is_none() && path.path.is_ident("f32"),
        _ => false,
    }
}

pub(super) fn array_length(length: &syn::Expr) -> Result<usize> {
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

pub(super) fn unsupported(ty: &Type) -> syn::Error {
    syn::Error::new_spanned(
        ty,
        "effect field must be f32, [f32; 2..=4] or [[f32; 4]; N]",
    )
}

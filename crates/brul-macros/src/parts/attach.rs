use super::*;
pub(super) fn expand_attach(
    arguments: brul_capability::AttachAttr,
    function: ItemFn,
) -> Result<TokenStream2> {
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
            ::brul::IntoDispatch::into_dispatch(#hook(&mut *state, ctx, event))
        },
        None => quote! {
            let _ = (&mut *state, &*ctx, event);
            ::brul::Dispatch::CONTINUE
        },
    };
    let raw_event_call = match &arguments.raw_event {
        Some(hook) => quote! {
            ::brul::IntoDispatch::into_dispatch(#hook(&mut *state, ctx, event))
        },
        None => quote! {
            let _ = (&mut *state, &*ctx, event);
            ::brul::Dispatch::CONTINUE
        },
    };
    let cleanup_call = match &arguments.cleanup {
        Some(hook) => quote! { #hook(&mut *state, ctx); },
        None => quote! { let _ = (&mut *state, &*ctx); },
    };
    Ok(quote! {
        #function

        pub fn #apply(
            ctx: &::brul::CapabilityContext,
            params: &[::brul::Dyn],
        ) -> Result<(), ::brul::ViewError> {
            #(#reads)*
            if ::brul::CapabilityContext::get_state::<#state_ty>(ctx).is_none() {
                let state = #name(ctx, #(Clone::clone(&#values)),*)?;
                ctx.set_state(state);
            }
            let cell = ::brul::CapabilityContext::get_state::<#state_ty>(ctx)
                .expect("attachment state was just created");
            if let Ok(mut state) = cell.try_borrow_mut() {
                #update
            }
            Ok(())
        }

        pub fn #event(
            ctx: &::brul::CapabilityContext,
            event: &::brul::UiEvent,
        ) -> ::brul::Dispatch {
            let Some(cell) = ::brul::CapabilityContext::get_state::<#state_ty>(ctx) else {
                return ::brul::Dispatch::CONTINUE;
            };
            let Ok(mut state) = cell.try_borrow_mut() else {
                return ::brul::Dispatch::CONTINUE;
            };
            #event_call
        }

        pub fn #raw_event(
            ctx: &::brul::CapabilityContext,
            event: &::brul::RawUiEvent,
        ) -> ::brul::Dispatch {
            let Some(cell) = ::brul::CapabilityContext::get_state::<#state_ty>(ctx) else {
                return ::brul::Dispatch::CONTINUE;
            };
            let Ok(mut state) = cell.try_borrow_mut() else {
                return ::brul::Dispatch::CONTINUE;
            };
            #raw_event_call
        }

        pub fn #cleanup(ctx: &::brul::CapabilityContext) {
            if let Some(cell) = ::brul::CapabilityContext::get_state::<#state_ty>(ctx)
                && let Ok(mut state) = cell.try_borrow_mut()
            {
                #cleanup_call
            }
            ctx.remove_state();
        }
    })
}

pub(super) fn attachment_state_type(output: &ReturnType) -> Result<Type> {
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

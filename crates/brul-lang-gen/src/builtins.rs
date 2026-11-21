use brul_lang_schema::{ArgSpec, BUILTINS, EmitSpec, HostSpecial};
use proc_macro2::{Span, TokenStream};
use quote::quote;

fn type_ident(name: &str) -> syn::Ident {
    syn::Ident::new(name, Span::call_site())
}

pub fn builtins() -> TokenStream {
    let entries = BUILTINS.iter().map(|spec| {
        let name = spec.name;
        let arguments = spec.arguments.iter().map(|argument| match argument {
            ArgSpec::Any => quote!(Argument::Any),
            ArgSpec::Function => quote!(Argument::Function),
            ArgSpec::Value(type_name) => {
                let ty = type_ident(type_name);
                quote!(Argument::Value(Type::#ty))
            }
        });
        let result = type_ident(spec.result);
        let emit = match spec.emit {
            EmitSpec::Function(path) => quote!(BuiltinEmit::Function { path: #path }),
            EmitSpec::Host(host) => {
                let marker = host.marker;
                quote!(BuiltinEmit::Host { marker: #marker })
            }
            EmitSpec::Method(name) => quote!(BuiltinEmit::Method { name: #name }),
        };
        quote! {
            Builtin {
                name: #name,
                arguments: &[#(#arguments),*],
                result: Type::#result,
                emit: #emit,
            },
        }
    });

    quote! {
        pub const BUILTINS: &[Builtin] = &[
            #(#entries)*
        ];
    }
}

pub fn dispatch() -> TokenStream {
    let arms = BUILTINS.iter().map(|spec| {
        let name = spec.name;
        let result = type_ident(spec.result);
        let call = match spec.emit {
            EmitSpec::Function(path) => quote! {
                call_value_function(#path, Type::#result, arguments, &context)
            },
            EmitSpec::Host(_) => quote! {
                call_host(builtin, arguments, &context)
            },
            EmitSpec::Method(method) => {
                let index = spec
                    .arguments
                    .iter()
                    .position(|argument| matches!(argument, ArgSpec::Function));
                let index = match index {
                    Some(index) => quote!(Some(#index)),
                    None => quote!(None),
                };
                quote! {
                    call_method(
                        #method,
                        #index,
                        Type::#result,
                        builtin,
                        arguments,
                        &context,
                    )
                }
            }
        };
        quote!(#name => #call,)
    });

    quote! {
        pub(super) fn emit_builtin(
            builtin: &Builtin,
            arguments: &[Expr],
            position: Position,
            names: &Names<'_>,
            state: &State,
            props: &HashMap<String, Type>,
            scopes: &Scopes,
        ) -> Result<Option<(String, Type)>, Error> {
            let context = Context {
                position,
                names,
                state,
                props,
                scopes,
            };
            match builtin.name {
                #(#arms)*
                _ => Ok(None),
            }
        }
    }
}

fn host_argument_type(name: &str) -> TokenStream {
    match name {
        "String" => quote!(String),
        "Bool" => quote!(bool),
        "Timer" => quote!(crate::TimerId),
        other => panic!("unsupported host argument type: {other}"),
    }
}

fn host_output_type(name: &str) -> TokenStream {
    match name {
        "String" => quote!(String),
        "Void" => quote!(()),
        other => panic!("unsupported host result type: {other}"),
    }
}

pub fn markers() -> TokenStream {
    let entries = BUILTINS.iter().filter_map(|spec| {
        let EmitSpec::Host(host) = spec.emit else {
            return None;
        };
        let marker = type_ident(host.marker);
        let name = spec.name;
        let output = host_output_type(spec.result);
        let (arguments, parameter, call) = match host.argument {
            None => (quote!(()), quote!(_arguments), quote!(&[])),
            Some(argument) => {
                let ty = host_argument_type(argument);
                (
                    quote!((#ty,)),
                    quote!(arguments),
                    quote!(&[crate::Value::from(arguments.0)]),
                )
            }
        };
        let invoke = match host.special {
            Some(HostSpecial::Theme) => quote! {
                let value = component.call(#name, &[])?;
                String::try_from(&value)
            },
            Some(HostSpecial::OpenWindow) => quote! {
                let _ = component.call(#name, #call)?;
                Ok(())
            },
            None => quote! {
                let _ = component.call(#name, #call)?;
                Ok(())
            },
        };
        Some(quote! {
            pub struct #marker;

            impl crate::HostCommand for #marker {
                type Arguments = #arguments;
                type Output = #output;

                const NAME: &'static str = #name;

                fn invoke(
                    component: &mut crate::Component,
                    #parameter: Self::Arguments,
                ) -> Result<Self::Output, crate::Error> {
                    #invoke
                }
            }
        })
    });

    let markers = BUILTINS.iter().filter_map(|spec| {
        let EmitSpec::Host(host) = spec.emit else {
            return None;
        };
        let marker = type_ident(host.marker);
        let name = spec.name;
        let marker_name = host.marker;
        Some(quote! {
            (#name, #marker_name, || <#marker as crate::HostCommand>::NAME)
        })
    });
    quote! {
        #(#entries)*

        #[cfg(test)]
        type MarkerEntry = (&'static str, &'static str, fn() -> &'static str);

        #[cfg(test)]
        pub(crate) const HOST_MARKERS: &[MarkerEntry] = &[
            #(#markers),*
        ];
    }
}

pub fn host_dispatch() -> TokenStream {
    let arms = BUILTINS.iter().filter_map(|spec| {
        let EmitSpec::Host(host) = spec.emit else {
            return None;
        };
        let name = spec.name;
        let body = match host.special {
            Some(HostSpecial::Theme) => quote! {
                no_arguments(arguments, name)?;
                Ok(match &*self.theme.borrow() {
                    Some(theme) => Value::String(theme.clone().into()),
                    None => Value::Null,
                })
            },
            Some(HostSpecial::OpenWindow) => quote! {
                let window: String = one_argument(arguments, #name)?;
                let Some(key) = self.windows.get(window.as_str()) else {
                    return Err(Error::UnknownWindow(window));
                };
                self.commands.borrow_mut().push(WindowCommand::Open(*key));
                Ok(Value::Null)
            },
            None => {
                let command = type_ident(host.command.expect("a host command"));
                match host.argument {
                    None => quote! {
                        no_arguments(arguments, name)?;
                        self.commands.borrow_mut().push(WindowCommand::#command);
                        Ok(Value::Null)
                    },
                    Some(argument) => {
                        let ty = if argument == "Bool" {
                            quote!(bool)
                        } else {
                            quote!(String)
                        };
                        quote! {
                            let value: #ty = one_argument(arguments, #name)?;
                            self.commands.borrow_mut().push(WindowCommand::#command(value));
                            Ok(Value::Null)
                        }
                    }
                }
            }
        };
        Some(quote! {
            #name => Some((|| -> Result<Value, Error> { #body })()),
        })
    });

    quote! {
        impl CommandHost {
            fn framework_call(
                &mut self,
                name: &str,
                arguments: &[Value],
            ) -> Option<Result<Value, Error>> {
                match name {
                    #(#arms)*
                    _ => None,
                }
            }
        }
    }
}

use syn::{FnArg, GenericArgument, Ident, ItemFn, Pat, PathArguments, ReturnType, Type};

pub struct Signature<'a> {
    pub name: &'a Ident,
    pub host: &'a Pat,
    pub host_type: &'a Type,
    pub params: Vec<(&'a Ident, &'a Type)>,
    pub output: &'a ReturnType,
}

pub fn is_command(function: &ItemFn) -> bool {
    function.attrs.iter().any(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "command")
    })
}

pub fn parse(function: &ItemFn) -> syn::Result<Signature<'_>> {
    let mut inputs = function.sig.inputs.iter();
    let Some(FnArg::Typed(host)) = inputs.next() else {
        return Err(syn::Error::new_spanned(
            &function.sig,
            "a command takes the host as its first argument",
        ));
    };
    if !matches!(&*host.pat, Pat::Ident(_)) {
        return Err(syn::Error::new_spanned(&*host.pat, "the host needs a name"));
    }
    let mut params = Vec::new();
    for input in inputs {
        let FnArg::Typed(argument) = input else {
            return Err(syn::Error::new_spanned(input, "a command has no receiver"));
        };
        let Pat::Ident(name) = &*argument.pat else {
            return Err(syn::Error::new_spanned(
                &*argument.pat,
                "an argument needs a name",
            ));
        };
        params.push((&name.ident, &*argument.ty));
    }
    Ok(Signature {
        name: &function.sig.ident,
        host: &host.pat,
        host_type: &host.ty,
        params,
        output: &function.sig.output,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    String,
    Number,
    Bool,
    Value,
    Null,
    Other,
}

pub fn kind(ty: &Type) -> Kind {
    match ty {
        Type::Reference(reference) => kind(&reference.elem),
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| match segment.ident.to_string().as_str() {
                "str" | "String" => Kind::String,
                "f64" | "f32" => Kind::Number,
                "bool" => Kind::Bool,
                "Value" => Kind::Value,
                "usize" | "u64" | "u32" | "i64" | "i32" => Kind::Number,
                _ => Kind::Other,
            })
            .unwrap_or(Kind::Other),
        Type::Tuple(tuple) if tuple.elems.is_empty() => Kind::Null,
        _ => Kind::Other,
    }
}

pub fn result_type(output: &ReturnType) -> Option<&Type> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn function(source: &str) -> ItemFn {
        syn::parse_str(source).expect("function")
    }

    #[test]
    fn a_signature_keeps_the_host_and_its_arguments() {
        let item = function("fn act(host: &mut Host, draft: Draft, count: usize) {}");
        let signature = parse(&item).expect("signature");
        assert_eq!(signature.name.to_string(), "act");
        assert_eq!(signature.params.len(), 2);
        assert_eq!(signature.params[0].0.to_string(), "draft");
        assert_eq!(signature.params[1].0.to_string(), "count");
    }

    #[test]
    fn a_result_output_resolves_to_its_type() {
        let item = function("fn act(host: &Host) -> Result<bool, Error> { unimplemented!() }");
        let signature = parse(&item).expect("signature");
        let ty = result_type(signature.output).expect("type");
        assert_eq!(kind(ty), Kind::Bool);
    }

    #[test]
    fn every_rust_kind_is_classified_once() {
        for (source, expected) in [
            ("&str", Kind::String),
            ("String", Kind::String),
            ("f64", Kind::Number),
            ("i32", Kind::Number),
            ("bool", Kind::Bool),
            ("Value", Kind::Value),
            ("()", Kind::Null),
            ("Draft", Kind::Other),
        ] {
            let ty = syn::parse_str(source).expect("type");
            assert_eq!(kind(&ty), expected, "{source}");
        }
    }

    #[test]
    fn a_signature_without_a_named_host_is_rejected() {
        let item = function("fn act(_: &Host) {}");
        let error = parse(&item).err().expect("signature");
        assert!(error.to_string().contains("the host needs a name"));
    }

    #[test]
    fn the_command_attribute_is_recognized_by_its_last_segment() {
        let item = function("#[brul::command] fn act(host: &Host) {}");
        assert!(is_command(&item));
        let plain = function("fn act(host: &Host) {}");
        assert!(!is_command(&plain));
    }
}

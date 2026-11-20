use std::collections::HashMap;

use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::types::Type;

use super::script::{ident, snake};

#[derive(Clone)]
pub(super) struct FunctionSig {
    pub(super) rust_name: String,
    pub(super) params: Vec<Type>,
    pub(super) return_type: Type,
}

pub(super) struct Scopes {
    frames: Vec<Vec<Binding>>,
    return_type: Type,
    functions: HashMap<String, FunctionSig>,
}

impl Default for Scopes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ScopeInfo {
    pub(super) struct_name: String,
    pub(super) outer: Option<usize>,
    pub(super) fields: Vec<ScopeField>,
}

#[derive(Clone)]
pub(super) struct ScopeField {
    pub(super) name: String,
    pub(super) field: String,
    pub(super) ty: Type,
}

struct Binding {
    name: String,
    access: String,
    ty: Type,
    constant: bool,
}

impl Scopes {
    pub(super) fn new() -> Self {
        Self {
            frames: vec![Vec::new()],
            return_type: Type::Void,
            functions: HashMap::new(),
        }
    }

    pub(super) fn with_return_type(mut self, return_type: &Type) -> Self {
        self.return_type = return_type.clone();
        self
    }

    pub(super) fn with_functions(mut self, functions: HashMap<String, FunctionSig>) -> Self {
        self.functions = functions;
        self
    }

    pub(super) fn return_type(&self) -> &Type {
        &self.return_type
    }

    pub(super) fn function(&self, name: &str) -> Option<&FunctionSig> {
        self.functions.get(name)
    }

    pub(super) fn push(&mut self) {
        self.frames.push(Vec::new());
    }

    pub(super) fn pop(&mut self) {
        self.frames.pop();
    }

    pub(super) fn declare(
        &mut self,
        name: &str,
        ty: &Type,
        constant: bool,
    ) -> Result<String, Error> {
        let frame = self.frames.last_mut().expect("scope frame");
        if frame.iter().any(|binding| binding.name == name) {
            return Err(crate::error_at(
                Position { line: 0, column: 0 },
                format!("'{name}' is already declared in this scope"),
            ));
        }
        let access = ident(&snake(name));
        frame.push(Binding {
            name: name.to_owned(),
            access: access.clone(),
            ty: ty.clone(),
            constant,
        });
        Ok(access)
    }

    pub(super) fn push_scope(&mut self, info: &ScopeInfo, outer: Option<&ScopeInfo>) {
        self.push();
        if let Some(outer) = outer {
            for field in &outer.fields {
                self.frames.last_mut().expect("scope frame").push(Binding {
                    name: field.name.clone(),
                    access: field.field.clone(),
                    ty: field.ty.clone(),
                    constant: true,
                });
            }
        }
        for field in &info.fields {
            self.frames.last_mut().expect("scope frame").push(Binding {
                name: field.name.clone(),
                access: field.field.clone(),
                ty: field.ty.clone(),
                constant: false,
            });
        }
    }

    pub(super) fn read(&self, name: &str) -> Option<String> {
        let binding = self.find(name)?;
        Some(if is_value(&binding.ty) {
            format!("{}.clone()", binding.access)
        } else {
            match binding.ty {
                Type::Number => format!("brul::Dyn::num({})", binding.access),
                Type::Bool => format!("brul::Dyn::flag({})", binding.access),
                Type::String => format!("brul::Dyn::text({}.as_str())", binding.access),
                Type::Null => "brul::Dyn::null()".to_owned(),
                _ => unreachable!("value type was checked"),
            }
        })
    }

    pub(super) fn read_typed(&self, name: &str) -> Option<(String, Type)> {
        let binding = self.find(name)?;
        let code = match binding.ty {
            Type::String | Type::Array(_) | Type::Object => format!("{}.clone()", binding.access),
            _ => binding.access.clone(),
        };
        Some((code, binding.ty.clone()))
    }

    pub(super) fn write(&self, name: &str, value: &str) -> Result<Option<String>, Error> {
        let Some(binding) = self.find(name) else {
            return Ok(None);
        };
        if binding.constant {
            return Err(crate::error_at(
                Position { line: 0, column: 0 },
                format!("cannot assign to constant '{name}'"),
            ));
        }
        Ok(Some(if is_value(&binding.ty) {
            format!("{} = {value}.clone();", binding.access)
        } else {
            format!("{} = (&{value}).try_into()?;", binding.access)
        }))
    }

    pub(super) fn local_type(ty: &Type) -> &'static str {
        match ty {
            Type::Number => "f64",
            Type::String => "String",
            Type::Bool => "bool",
            Type::Null => "()",
            _ => "brul::Dyn",
        }
    }

    pub(super) fn is_value(ty: &Type) -> bool {
        is_value(ty)
    }

    fn find(&self, name: &str) -> Option<&Binding> {
        self.frames
            .iter()
            .rev()
            .find_map(|frame| frame.iter().rev().find(|binding| binding.name == name))
    }
}

fn is_value(ty: &Type) -> bool {
    matches!(ty, Type::Array(_) | Type::Object)
}

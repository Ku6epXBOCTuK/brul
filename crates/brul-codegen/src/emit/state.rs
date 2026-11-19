use std::fmt::Write;

use brul_script::Names;
use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::script::{Program, Statement};
use brul_syntax::types::Type;

use super::names;
use super::script::ident;
use crate::commands::{Command, Commands};

#[derive(Default)]
pub struct State {
    type_name: String,
    fields: Vec<Field>,
    commands: Commands,
}

struct Field {
    name: String,
    ident: String,
    ty: Type,
    variable: String,
}

impl State {
    pub fn new(
        program: Option<&Program>,
        names: &Names<'_>,
        type_name: &str,
    ) -> Result<Self, Error> {
        let mut state = Self {
            type_name: type_name.to_owned(),
            fields: Vec::new(),
            commands: Commands::new(),
        };
        let Some(program) = program else {
            return Ok(state);
        };
        for statement in &program.statements {
            let Statement::Let { name, ty, .. } = statement else {
                continue;
            };
            if state.fields.iter().any(|field| field.name == *name) {
                continue;
            }
            if rust_type(ty).is_err() {
                continue;
            }
            let ident = ident(&super::script::snake(name));
            let ident = if state.fields.iter().any(|field| field.ident == ident) {
                format!("{ident}_{}", state.fields.len())
            } else {
                ident
            };
            let variable = names::variable_expr(name, Position { line: 0, column: 0 }, names)?;
            state.fields.push(Field {
                name: name.clone(),
                ident,
                ty: ty.clone(),
                variable,
            });
        }
        Ok(state)
    }

    pub fn with_commands(mut self, commands: &Commands) -> Self {
        self.commands = commands.clone();
        self
    }

    pub fn command(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    pub fn field_type(&self, name: &str) -> Option<&Type> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.ty)
    }

    pub fn definition(&self, init_name: &str) -> String {
        if self.fields.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        let _ = writeln!(
            out,
            "\n#[allow(dead_code)]\npub struct {} {{",
            self.type_name
        );
        for field in &self.fields {
            let _ = writeln!(
                out,
                "    pub {}: brul::Signal<{}>,",
                field.ident,
                rust_type(&field.ty).expect("state field type was checked")
            );
        }
        let _ = writeln!(
            out,
            "}}\n\n#[allow(dead_code)]\nfn {init_name}(component: &mut brul::Component) -> Result<(), brul::Error> {{"
        );
        for field in &self.fields {
            let ty = rust_type(&field.ty).expect("state field type was checked");
            let _ = writeln!(
                out,
                "    let {}: {ty} = (&component.read_id({})?).try_into()?;",
                field.ident, field.variable
            );
        }
        let values = self
            .fields
            .iter()
            .map(|field| format!("{}: brul::Signal::new({})", field.ident, field.ident))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "    component.set_state({} {{ {values} }});",
            self.type_name
        );
        out.push_str("    Ok(())\n}\n");
        out
    }

    pub fn init_call(&self, init_name: &str) -> String {
        if self.fields.is_empty() {
            String::new()
        } else {
            format!("    {init_name}(component)?;\n")
        }
    }

    pub fn read(&self, name: &str) -> Option<String> {
        let field = self.fields.iter().find(|field| field.name == name)?;
        let state = format!("component.state::<{}>().{}", self.type_name, field.ident);
        Some(match field.ty {
            Type::Number => format!("brul::Value::Number({state}.get())"),
            Type::Bool => format!("brul::Value::Bool({state}.get())"),
            Type::String => format!("brul::Value::String({state}.get().into())"),
            Type::Null => "brul::Value::Null".to_owned(),
            _ => unreachable!("state field type was checked"),
        })
    }

    pub fn read_typed(&self, name: &str) -> Option<(String, Type)> {
        let field = self.fields.iter().find(|field| field.name == name)?;
        let state = format!("component.state::<{}>().{}", self.type_name, field.ident);
        let code = match field.ty {
            Type::Number | Type::Bool | Type::Null => format!("{state}.get()"),
            Type::String => format!("{state}.get()"),
            _ => return None,
        };
        Some((code, field.ty.clone()))
    }

    pub fn write(&self, name: &str, value: &str) -> Option<String> {
        let field = self.fields.iter().find(|field| field.name == name)?;
        let state = format!("component.state::<{}>().{}", self.type_name, field.ident);
        Some(match field.ty {
            Type::Null => format!("{{ let _ = &{value}; {state}.set(()); }}"),
            _ => format!("{state}.set((&{value}).try_into()?);"),
        })
    }
}

fn rust_type(ty: &Type) -> Result<&'static str, Error> {
    match ty {
        Type::Number => Ok("f64"),
        Type::String => Ok("String"),
        Type::Bool => Ok("bool"),
        Type::Null => Ok("()"),
        _ => Err(crate::error_at(
            Position { line: 0, column: 0 },
            format!("type {ty} is not supported in application state"),
        )),
    }
}

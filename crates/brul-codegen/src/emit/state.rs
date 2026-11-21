use std::fmt::Write;

use brul_case::{ident, snake};
use brul_syntax::error::Error;
use brul_syntax::script::{Program, Statement};
use brul_syntax::types::Type;

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
}

impl State {
    pub fn new(program: Option<&Program>, type_name: &str) -> Result<Self, Error> {
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
            if !ty.supports_state() {
                continue;
            }
            let ident = ident(&snake(name));
            let ident = if state.fields.iter().any(|field| field.ident == ident) {
                format!("{ident}_{}", state.fields.len())
            } else {
                ident
            };
            state.fields.push(Field {
                name: name.clone(),
                ident,
                ty: ty.clone(),
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
                field.ty.local_type()
            );
        }
        let _ = writeln!(
            out,
            "}}\n\n#[allow(dead_code)]\nfn {init_name}(component: &mut brul::Component) -> Result<(), brul::Error> {{"
        );
        for field in &self.fields {
            let ty = field.ty.local_type();
            let _ = writeln!(
                out,
                "    let {}: {ty} = component.seed({:?}, brul::ast::Position {{ line: 0, column: 0 }})?;",
                field.ident, field.name
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
        Some(
            field
                .ty
                .state_read(&state)
                .expect("state field type was checked"),
        )
    }

    pub fn read_typed(&self, name: &str) -> Option<(String, Type)> {
        let field = self.fields.iter().find(|field| field.name == name)?;
        let state = format!("component.state::<{}>().{}", self.type_name, field.ident);
        if !field.ty.supports_state() {
            return None;
        }
        Some((format!("{state}.get()"), field.ty.clone()))
    }

    pub fn write(&self, name: &str, value: &str) -> Option<String> {
        let field = self.fields.iter().find(|field| field.name == name)?;
        let state = format!("component.state::<{}>().{}", self.type_name, field.ident);
        Some(
            field
                .ty
                .state_write(&state, value)
                .expect("state field type was checked"),
        )
    }
}

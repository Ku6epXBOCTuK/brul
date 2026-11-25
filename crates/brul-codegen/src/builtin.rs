use std::rc::Rc;

use brul_syntax::error::Error;
use brul_syntax::markup::{Arg, Declaration, DeclarationKind};
use brul_syntax::style::{Binding, BuiltinStyles, Target, Theme, parse_builtin};
use brul_widget_schema::{ELEMENTS, find};

use crate::capability::{CapabilityTable, POLICY_NAMES};
use crate::{Registry, Source};

const SOURCE: &str = include_str!("../../../builtin/widgets.brul");

pub fn load(
    palette: &[Theme],
    widgets: &CapabilityTable,
    components: &[String],
) -> Result<BuiltinStyles, Error> {
    let builtin = parse_builtin(SOURCE, palette)?;
    for binding in &builtin.bindings {
        if !known(binding, widgets, components) {
            return Err(Error::at(
                SOURCE,
                binding.at,
                Error::custom(format!("unknown binding target '{:?}'", binding.target)),
            ));
        }
    }
    Ok(builtin)
}

pub fn compose(
    source: &mut Source,
    registry: &mut Registry,
    widgets: &CapabilityTable,
) -> Result<(), Error> {
    let components: Vec<String> = registry
        .definitions
        .iter()
        .map(|definition| definition.name.clone())
        .collect();
    let builtin = load(&source.stylesheet.themes, widgets, &components)?;
    if let Some(declaration) = source.declaration.as_mut() {
        crate::bindings::apply(declaration, registry, &builtin.bindings);
        inject_window_policies(declaration);
    }
    source.stylesheet = crate::styles::compose(&source.stylesheet, &builtin.stylesheet);
    for definition in &mut registry.definitions {
        let sheet = parse_builtin(SOURCE, &definition.stylesheet.themes)?.stylesheet;
        definition.stylesheet = Rc::new(crate::styles::compose(&definition.stylesheet, &sheet));
        let is_window = definition.is_window();
        if is_window {
            inject_window_policies(&mut definition.declaration);
        }
    }
    Ok(())
}

fn inject_window_policies(declaration: &mut Declaration) {
    let position = declaration.position;
    let DeclarationKind::Window { args } = &mut declaration.kind else {
        return;
    };
    let mut overrides: Vec<(String, Arg)> = Vec::new();
    args.retain(|arg| {
        if let Arg::Attach { name, .. } = arg
            && POLICY_NAMES.contains(&name.as_str())
        {
            overrides.push((name.clone(), arg.clone()));
            return false;
        }
        true
    });
    let implicit = POLICY_NAMES.iter().map(|name| {
        overrides
            .iter()
            .find(|(override_name, _)| override_name == name)
            .map(|(_, arg)| arg.clone())
            .unwrap_or_else(|| Arg::Attach {
                name: (*name).to_owned(),
                params: Vec::new(),
                position,
            })
    });
    args.splice(0..0, implicit);
}

fn known(binding: &Binding, widgets: &CapabilityTable, components: &[String]) -> bool {
    match &binding.target {
        Target::Element(name) => {
            find(name).is_some()
                || widgets.contains_key(name)
                || components.iter().any(|component| component == name)
        }
        Target::Event(name) => {
            ELEMENTS.iter().any(|element| {
                element
                    .events
                    .iter()
                    .any(|event| event.name == name.as_str())
            }) || widgets
                .values()
                .any(|spec| spec.events.iter().any(|event| event.name == name.as_str()))
        }
    }
}

#[cfg(test)]
mod tests {
    use brul_syntax::ast::Position;

    use super::*;

    fn window(args: Vec<Arg>) -> Declaration {
        Declaration {
            name: "Test".to_owned(),
            members: Vec::new(),
            position: Position::default(),
            kind: DeclarationKind::Window { args },
        }
    }

    fn attach(name: &str, line: usize) -> Arg {
        Arg::Attach {
            name: name.to_owned(),
            params: Vec::new(),
            position: Position { line, column: 1 },
        }
    }

    fn names(declaration: &Declaration) -> Vec<&str> {
        declaration
            .args()
            .iter()
            .filter_map(|arg| match arg {
                Arg::Attach { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn injects_all_policies_before_the_user_arguments() {
        let mut declaration = window(vec![attach("keys", 7)]);
        inject_window_policies(&mut declaration);
        assert_eq!(
            names(&declaration),
            vec!["editor", "keyboard", "pointer", "scroll", "keys"]
        );
    }

    #[test]
    fn a_user_argument_replaces_only_its_own_policy() {
        let mut declaration = window(vec![attach("keyboard", 9), attach("keys", 7)]);
        inject_window_policies(&mut declaration);
        assert_eq!(
            names(&declaration),
            vec!["editor", "keyboard", "pointer", "scroll", "keys"]
        );
        let keyboard = declaration.args()[1].clone();
        assert_eq!(keyboard, attach("keyboard", 9));
    }
}

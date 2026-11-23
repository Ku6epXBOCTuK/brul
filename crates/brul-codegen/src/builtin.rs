use std::rc::Rc;

use brul_syntax::error::Error;
use brul_syntax::style::{Binding, BuiltinStyles, Target, Theme, parse_builtin};
use brul_widget_schema::{ELEMENTS, find};

use crate::capability::CapabilityTable;
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
    }
    source.stylesheet = crate::styles::compose(&source.stylesheet, &builtin.stylesheet);
    for definition in &mut registry.definitions {
        let sheet = parse_builtin(SOURCE, &definition.stylesheet.themes)?.stylesheet;
        definition.stylesheet = Rc::new(crate::styles::compose(&definition.stylesheet, &sheet));
    }
    Ok(())
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

use brul_style::{Stylesheet, Theme};

use super::parser::{Scope, parse};
use crate::error::Error;

pub use super::parser::{Binding, Target};

#[derive(Debug)]
pub struct BuiltinStyles {
    pub stylesheet: Stylesheet,
    pub bindings: Vec<Binding>,
}

pub fn parse_builtin(source: &str, palette: &[Theme]) -> Result<BuiltinStyles, Error> {
    let style = parse(source, Scope::Builtin(palette))?;
    Ok(BuiltinStyles {
        stylesheet: style.stylesheet,
        bindings: style.bindings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_are_parsed_once() {
        let source = "@apply TextInput brul-input;\n@apply on drag brul-draggable;\n.brul-input { cursor: text; }\n.brul-draggable { cursor: grab; }";
        let parsed = parse_builtin(source, &[]).unwrap();
        assert_eq!(parsed.bindings.len(), 2);
    }
}

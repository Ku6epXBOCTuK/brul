use brul_syntax::ast::Position;
use brul_syntax::error::Error;
use brul_syntax::style::Stylesheet;

use crate::error_at;

use super::Out;
use super::class;

const SOURCE: &str = include_str!("../../../../builtin/widgets.brul");
const INPUT: &str = "brul-input";
const DRAGGABLE: &str = "brul-draggable";

#[derive(Clone, Copy)]
pub(super) enum Class {
    Input,
    Draggable,
}

fn parse() -> Result<Stylesheet, Error> {
    brul_syntax::style::parse_file_in(SOURCE, &[])
}

pub(super) fn apply(out: &mut Out<'_>, builtin: Class) -> Result<(), Error> {
    let name = match builtin {
        Class::Input => INPUT,
        Class::Draggable => DRAGGABLE,
    };
    let stylesheet = parse()?;
    let resolved = brul_gui::style::class(&stylesheet, name, None)
        .map_err(|error| error_at(Position { line: 1, column: 1 }, error.to_string()))?;
    let mut literal = String::new();
    {
        let mut inner = Out::at(&mut literal, out.depth());
        class::literal(&mut inner, &resolved);
    }
    out.push(format_args!(".with_class(&{})", literal.trim()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use brul_gui::{Cursor, StyleState};

    use super::*;

    #[test]
    fn builtin_classes_resolve_their_default_cursors() {
        let stylesheet = parse().expect("builtin stylesheet");
        let input = brul_gui::style::class(&stylesheet, INPUT, None).expect("input class");
        assert_eq!(input.style.base().cursor, Some(Cursor::Text));
        let draggable = brul_gui::style::class(&stylesheet, DRAGGABLE, None).expect("drag class");
        assert_eq!(draggable.style.base().cursor, Some(Cursor::Grab));
        let pressed = draggable
            .style
            .patch(StyleState::Pressed)
            .expect("pressed patch");
        assert_eq!(pressed.cursor, Some(Cursor::Grabbing));
    }
}

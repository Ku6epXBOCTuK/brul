use brul_syntax::error::Error;
use brul_syntax::markup::{Declaration, Element, Member};

use crate::error_at;

pub(super) enum RootShape<'a> {
    Single(&'a Element),
    Fragment(Vec<&'a Member>),
}

fn items(declaration: &Declaration) -> Vec<&Member> {
    declaration
        .members
        .iter()
        .filter(|member| matches!(member, Member::Element(_) | Member::Slot(_)))
        .collect()
}

pub(super) fn root_shape(declaration: &Declaration) -> Result<RootShape<'_>, Error> {
    let roots = items(declaration);
    match roots.as_slice() {
        [] => Err(error_at(
            declaration.position,
            "component has no root element or slot",
        )),
        [Member::Element(element)] => Ok(RootShape::Single(element)),
        _ => Ok(RootShape::Fragment(roots)),
    }
}

pub(super) fn window_root(declaration: &Declaration) -> Result<&Element, Error> {
    let roots = items(declaration);
    match roots.as_slice() {
        [] => Err(error_at(declaration.position, "window has no root element")),
        [Member::Element(element)] => Ok(element),
        [_, _, ..] => Err(error_at(
            declaration.position,
            "window has more than one root element",
        )),
        [_] => Err(error_at(
            declaration.position,
            "window root must be an element",
        )),
    }
}

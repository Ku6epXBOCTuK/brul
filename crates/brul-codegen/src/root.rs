use brul_syntax::error::Error;
use brul_syntax::markup::{Declaration, Element, StructureRef};

use crate::error_at;

pub(super) enum RootShape<'a> {
    Single(&'a Element),
    Fragment(Vec<StructureRef<'a>>),
}

fn items(declaration: &Declaration) -> Vec<StructureRef<'_>> {
    declaration
        .members
        .iter()
        .filter_map(|member| member.structure())
        .collect()
}

pub(super) fn root_shape(declaration: &Declaration) -> Result<RootShape<'_>, Error> {
    let roots = items(declaration);
    match roots.as_slice() {
        [] => Err(error_at(
            declaration.position,
            "component has no root element or slot",
        )),
        [StructureRef::Element(element)] => Ok(RootShape::Single(element)),
        _ => Ok(RootShape::Fragment(roots)),
    }
}

pub(super) fn window_root(declaration: &Declaration) -> Result<&Element, Error> {
    let roots = items(declaration);
    match roots.as_slice() {
        [] => Err(error_at(declaration.position, "window has no root element")),
        [StructureRef::Element(element)] => Ok(element),
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

use std::collections::{HashMap, HashSet};
use std::fmt;

use brul_syntax::error::Error;
use brul_syntax::markup::{AnchorName, Arg, Block, BlockItem, Declaration, Element, Member};

use crate::{Definition, Source, error_at};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct AnchorId(u32);

impl fmt::Display for AnchorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub(super) type AnchorIds = HashMap<String, AnchorId>;

pub(super) fn collect(source: &Source, definitions: &[Definition]) -> Result<AnchorIds, Error> {
    let mut targets: Vec<&AnchorName> = Vec::new();
    let mut names = HashSet::new();
    if let Some(declaration) = &source.declaration {
        declaration_targets(declaration, &mut targets, &mut names)?;
    }
    for definition in definitions {
        declaration_targets(&definition.declaration, &mut targets, &mut names)?;
    }
    for target in &targets {
        if !names.contains(&target.name) {
            return Err(error_at(
                target.position,
                format!("anchor target '{}' is not defined", target.name),
            ));
        }
    }
    Ok(targets
        .into_iter()
        .enumerate()
        .map(|(index, anchor)| (anchor.name.clone(), AnchorId(0x8000_0000 | index as u32)))
        .collect())
}

pub(super) fn suffix(element: &Element, emit: &super::Emit<'_>) -> Result<String, Error> {
    let mut result = String::new();
    if let Some(name) = element.args.iter().find_map(|arg| match arg {
        Arg::Name(name) if emit.context.anchor_ids.contains_key(&name.name) => Some(&name.name),
        _ => None,
    }) {
        let id = emit.context.anchor_ids[name];
        result.push_str(&format!(".with_id(brul::gui::WidgetId::new({id}))"));
    }
    if let Some(anchor) = element.args.iter().find_map(|arg| match arg {
        Arg::Anchor(anchor) => Some(anchor),
        _ => None,
    }) {
        result.push_str(&format!(".anchored({})", expression(anchor, emit)?));
    }
    Ok(result)
}

fn expression(anchor: &AnchorName, emit: &super::Emit<'_>) -> Result<String, Error> {
    let Some(&id) = emit.context.anchor_ids.get(&anchor.name) else {
        return Err(error_at(
            anchor.position,
            format!("anchor target '{}' is not defined", anchor.name),
        ));
    };
    Ok(format!(
        "brul::gui::Anchor::new(brul::gui::WidgetId::new({id}))"
    ))
}

fn declaration_targets<'a>(
    declaration: &'a Declaration,
    targets: &mut Vec<&'a AnchorName>,
    names: &mut HashSet<String>,
) -> Result<(), Error> {
    let mut siblings = HashSet::new();
    for member in &declaration.members {
        if let Member::Element(element) = member {
            element_targets(element, targets, names, &mut siblings)?;
        }
    }
    Ok(())
}

fn element_targets<'a>(
    element: &'a Element,
    targets: &mut Vec<&'a AnchorName>,
    names: &mut HashSet<String>,
    siblings: &mut HashSet<String>,
) -> Result<(), Error> {
    for arg in &element.args {
        match arg {
            Arg::Anchor(anchor) => {
                if !targets.iter().any(|seen| seen.name == anchor.name) {
                    targets.push(anchor);
                }
            }
            Arg::Name(name) => {
                if !siblings.insert(name.name.clone()) {
                    return Err(error_at(
                        name.position,
                        format!("anchor name '{}' is declared more than once", name.name),
                    ));
                }
                names.insert(name.name.clone());
            }
            _ => {}
        }
    }
    block_targets(&element.block, targets, names)
}

fn block_targets<'a>(
    block: &'a Block,
    targets: &mut Vec<&'a AnchorName>,
    names: &mut HashSet<String>,
) -> Result<(), Error> {
    let mut siblings = HashSet::new();
    for item in &block.items {
        match item {
            BlockItem::Element(element) => element_targets(element, targets, names, &mut siblings)?,
            BlockItem::If(block) => {
                for branch in &block.branches {
                    block_targets(&branch.block, targets, names)?;
                }
                if let Some(block) = &block.else_block {
                    block_targets(block, targets, names)?;
                }
            }
            BlockItem::Each(block) => block_targets(&block.block, targets, names)?,
            BlockItem::Slot(slot) => {
                if let Some(block) = &slot.block {
                    block_targets(block, targets, names)?;
                }
            }
            BlockItem::Expr(_) => {}
        }
    }
    Ok(())
}

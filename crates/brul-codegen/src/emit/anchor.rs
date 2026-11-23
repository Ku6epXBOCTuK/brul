use std::collections::{HashMap, HashSet};
use std::fmt;

use brul_syntax::error::Error;
use brul_syntax::markup::{AnchorArg, Arg, Block, BlockItem, Declaration, Element, Member};

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
    let mut targets: Vec<&AnchorArg> = Vec::new();
    let mut classes = HashSet::new();
    if let Some(declaration) = &source.declaration {
        declaration_targets(declaration, &mut targets, &mut classes);
    }
    for definition in definitions {
        declaration_targets(&definition.declaration, &mut targets, &mut classes);
    }
    for anchor in &targets {
        if !classes.contains(&anchor.target) {
            return Err(error_at(
                anchor.position,
                format!("anchor target '{}' is not defined", anchor.target),
            ));
        }
    }
    Ok(targets
        .into_iter()
        .enumerate()
        .map(|(index, anchor)| (anchor.target.clone(), AnchorId(0x8000_0000 | index as u32)))
        .collect())
}

pub(super) fn suffix(element: &Element, emit: &super::Emit<'_>) -> Result<String, Error> {
    let mut result = String::new();
    if let Some(name) = element_names(element)
        .into_iter()
        .find(|name| emit.anchor_ids.contains_key(*name))
    {
        let id = emit.anchor_ids[name];
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

fn expression(anchor: &AnchorArg, emit: &super::Emit<'_>) -> Result<String, Error> {
    let Some(&id) = emit.anchor_ids.get(&anchor.target) else {
        return Err(error_at(
            anchor.position,
            format!("anchor target '{}' is not defined", anchor.target),
        ));
    };
    Ok(format!(
        "brul::gui::Anchor::new(brul::gui::WidgetId::new({id}))"
    ))
}

fn declaration_targets<'a>(
    declaration: &'a Declaration,
    targets: &mut Vec<&'a AnchorArg>,
    classes: &mut HashSet<String>,
) {
    for member in &declaration.members {
        if let Member::Element(element) = member {
            element_targets(element, targets, classes);
        }
    }
}

fn element_targets<'a>(
    element: &'a Element,
    targets: &mut Vec<&'a AnchorArg>,
    classes: &mut HashSet<String>,
) {
    for arg in &element.args {
        match arg {
            Arg::Anchor(anchor) => {
                if !targets.iter().any(|seen| seen.target == anchor.target) {
                    targets.push(anchor);
                }
            }
            Arg::Class(name) => {
                classes.insert(name.clone());
            }
            _ => {}
        }
    }
    block_targets(&element.block, targets, classes);
}

fn block_targets<'a>(
    block: &'a Block,
    targets: &mut Vec<&'a AnchorArg>,
    classes: &mut HashSet<String>,
) {
    for item in &block.items {
        match item {
            BlockItem::Element(element) => element_targets(element, targets, classes),
            BlockItem::If(block) => {
                for branch in &block.branches {
                    block_targets(&branch.block, targets, classes);
                }
                if let Some(block) = &block.else_block {
                    block_targets(block, targets, classes);
                }
            }
            BlockItem::Each(block) => block_targets(&block.block, targets, classes),
            BlockItem::Slot(slot) => {
                if let Some(block) = &slot.block {
                    block_targets(block, targets, classes);
                }
            }
            BlockItem::Expr(_) => {}
        }
    }
}

fn element_names(element: &Element) -> Vec<&str> {
    element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Class(name) => Some(name.as_str()),
            _ => None,
        })
        .collect()
}

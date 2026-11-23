use brul_syntax::markup::{Arg, Block, BlockItem, Declaration, Member};
use brul_syntax::style::{Binding, Target};

use crate::Registry;

pub fn apply(declaration: &mut Declaration, registry: &mut Registry, bindings: &[Binding]) {
    for member in &mut declaration.members {
        walk_member(member, bindings);
    }
    for definition in &mut registry.definitions {
        for item in &mut definition.declaration.members {
            walk_member(item, bindings);
        }
    }
}

fn walk_member(member: &mut Member, bindings: &[Binding]) {
    if let Member::Element(element) = member {
        walk_element(element, bindings);
    }
}

fn walk_element(element: &mut brul_syntax::markup::Element, bindings: &[Binding]) {
    walk_block(&mut element.block, bindings);
    let handlers: Vec<String> = element
        .args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Handler { event, .. } => Some(event.clone()),
            _ => None,
        })
        .collect();
    let implicit: Vec<Arg> = bindings
        .iter()
        .filter(|binding| match &binding.target {
            Target::Element(name) => *name == element.name,
            Target::Event(name) => handlers.iter().any(|handler| handler == name),
        })
        .map(|binding| Arg::Class(binding.class.clone()))
        .collect();
    element.args.splice(0..0, implicit);
}

fn walk_block(block: &mut Block, bindings: &[Binding]) {
    for item in &mut block.items {
        match item {
            BlockItem::Element(element) => walk_element(element, bindings),
            BlockItem::If(conditional) => {
                for branch in &mut conditional.branches {
                    walk_block(&mut branch.block, bindings);
                }
                if let Some(else_block) = &mut conditional.else_block {
                    walk_block(else_block, bindings);
                }
            }
            BlockItem::Each(each) => {
                walk_block(&mut each.block, bindings);
                if let Some(else_block) = &mut each.else_block {
                    walk_block(else_block, bindings);
                }
            }
            BlockItem::Slot(slot) => {
                if let Some(slot_block) = &mut slot.block {
                    walk_block(slot_block, bindings);
                }
            }
            BlockItem::Expr(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings() -> Vec<Binding> {
        let at = 0;
        vec![
            Binding {
                target: Target::Event("drag".into()),
                class: "brul-draggable".into(),
                at,
            },
            Binding {
                target: Target::Element("TextInput".into()),
                class: "brul-input".into(),
                at,
            },
        ]
    }

    fn registry() -> Registry {
        Registry {
            definitions: Vec::new(),
            imports: Vec::new(),
            resources: Vec::new(),
        }
    }

    fn classes(element: &brul_syntax::markup::Element) -> Vec<&str> {
        element
            .args
            .iter()
            .filter_map(|arg| match arg {
                Arg::Class(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn implicit_classes_come_before_user_classes() {
        let source = crate::source::parse(
            r#"<markup>
window Demo (title: "demo") { Container (field, on drag: dragWindow) { } }
</markup>"#,
        )
        .unwrap();
        let mut declaration = source.declaration.unwrap();
        apply(&mut declaration, &mut registry(), &bindings());
        let Member::Element(element) = declaration
            .members
            .iter()
            .find(|member| matches!(member, Member::Element(_)))
            .unwrap()
        else {
            panic!("window body");
        };
        assert_eq!(classes(element), ["brul-draggable", "field"]);
    }

    #[test]
    fn implicit_classes_reach_nested_elements() {
        let source = crate::source::parse(
            r#"<markup>
window Demo (title: "demo") { Container (field) { @each (items as item) { TextInput { item } } } }
</markup>"#,
        )
        .unwrap();
        let mut declaration = source.declaration.unwrap();
        apply(&mut declaration, &mut registry(), &bindings());
        let Member::Element(element) = declaration
            .members
            .iter()
            .find(|member| matches!(member, Member::Element(_)))
            .unwrap()
        else {
            panic!("window body");
        };
        let BlockItem::Each(each) = &element.block.items[0] else {
            panic!("each");
        };
        let BlockItem::Element(input) = &each.block.items[0] else {
            panic!("input");
        };
        assert_eq!(classes(input), ["brul-input"]);
    }
}

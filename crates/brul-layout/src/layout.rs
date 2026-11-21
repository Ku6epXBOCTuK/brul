use crate::{
    Direction, LayoutBox, LayoutError, LayoutNode, LayoutStyle, Layouter, NodeShape, Offset,
    ScrollOffset, Size, validate,
};

struct FlatNode<'a> {
    style: &'a LayoutStyle,
    direction: Direction,
    offset: Option<Offset>,
    scroll: ScrollOffset,
    content: Size,
    children: Vec<u32>,
}

fn flatten<'a>(node: &'a LayoutNode, nodes: &mut Vec<FlatNode<'a>>) -> Result<(), LayoutError> {
    let style = node.style();
    validate(style)?;

    let (direction, offset, scroll, content, children) = match node {
        LayoutNode::Leaf { size, .. } => (
            Direction::Row,
            node.offset(),
            ScrollOffset::ZERO,
            *size,
            Vec::new(),
        ),
        LayoutNode::Container {
            direction,
            children,
            scroll_offset,
            ..
        } => {
            let mut keys = Vec::with_capacity(children.len());
            for child in children {
                flatten(child, nodes)?;
                keys.push(nodes.len() as u32);
            }
            (
                *direction,
                node.offset(),
                *scroll_offset,
                Size::default(),
                keys,
            )
        }
    };
    nodes.push(FlatNode {
        style,
        direction,
        offset,
        scroll,
        content,
        children,
    });
    Ok(())
}

pub fn layout(root: &LayoutNode, viewport: Size) -> Result<LayoutBox, LayoutError> {
    let mut nodes = Vec::new();
    flatten(root, &mut nodes)?;
    let root_key = nodes.len() as u32;

    let mut layouter = Layouter::<Size>::new();
    layouter.begin();
    // NOTE: post-order only: `child_nodes` resolves child keys while the parent is registered
    for (index, node) in nodes.iter().enumerate() {
        layouter.node(
            index as u32 + 1,
            NodeShape {
                style: node.style,
                direction: node.direction,
                offset: node.offset,
                scroll: node.scroll,
                content: node.content,
                children: &node.children,
                transparent: false,
                layer: false,
                constrained: false,
            },
        );
    }
    layouter.finish();
    layouter.compute(root_key, viewport, |content, _width| *content);

    let mut flat = vec![LayoutBox::default(); nodes.len() + 1];
    layouter.collect_into(root_key, &mut flat);

    let mut nested: Vec<Option<LayoutBox>> = (0..nodes.len() + 1).map(|_| None).collect();
    for (index, node) in nodes.iter().enumerate() {
        let mut placed = flat[index + 1].clone();
        placed.children = node
            .children
            .iter()
            .map(|key| nested[*key as usize].take().expect("layout child"))
            .collect();
        nested[index + 1] = Some(placed);
    }
    Ok(nested[root_key as usize].take().expect("layout root"))
}

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::fmt;

use taffy::style_helpers::TaffyAuto;
use taffy::{
    AlignItems, AvailableSpace, BoxSizing, Dimension as TaffyDimension, FlexDirection,
    JustifyContent, LengthPercentage, LengthPercentageAuto, NodeId, Position, Rect as TaffyRect,
    Size as TaffySize, Style as TaffyStyle, TaffyTree, compute_leaf_layout,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    #[default]
    Auto,
    Px(u32),
    Percent(f32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Padding {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Padding {
    pub const fn all(value: u32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutStyle {
    pub width: Dimension,
    pub height: Dimension,
    pub padding: Padding,
    pub border_width: u32,
    pub gap: u32,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub justify: Justify,
    pub align: Align,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutPatch {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Option<Padding>,
    pub border_width: Option<u32>,
    pub gap: Option<u32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<Dimension>,
    pub justify: Option<Justify>,
    pub align: Option<Align>,
}

impl Default for LayoutPatch {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl LayoutPatch {
    pub const DEFAULT: Self = Self {
        width: None,
        height: None,
        padding: None,
        border_width: None,
        gap: None,
        flex_grow: None,
        flex_shrink: None,
        flex_basis: None,
        justify: None,
        align: None,
    };
    pub fn is_empty(&self) -> bool {
        self.width.is_none()
            && self.height.is_none()
            && self.padding.is_none()
            && self.border_width.is_none()
            && self.gap.is_none()
            && self.flex_grow.is_none()
            && self.flex_shrink.is_none()
            && self.flex_basis.is_none()
            && self.justify.is_none()
            && self.align.is_none()
    }
}

impl LayoutStyle {
    pub fn apply(&mut self, patch: &LayoutPatch) {
        if let Some(width) = patch.width {
            self.width = width;
        }
        if let Some(height) = patch.height {
            self.height = height;
        }
        if let Some(padding) = patch.padding {
            self.padding = padding;
        }
        if let Some(border_width) = patch.border_width {
            self.border_width = border_width;
        }
        if let Some(gap) = patch.gap {
            self.gap = gap;
        }
        if let Some(flex_grow) = patch.flex_grow {
            self.flex_grow = flex_grow;
        }
        if let Some(flex_shrink) = patch.flex_shrink {
            self.flex_shrink = flex_shrink;
        }
        if let Some(flex_basis) = patch.flex_basis {
            self.flex_basis = flex_basis;
        }
        if let Some(justify) = patch.justify {
            self.justify = justify;
        }
        if let Some(align) = patch.align {
            self.align = align;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScrollOffset(i32);

impl ScrollOffset {
    pub const ZERO: Self = Self(0);

    pub const fn new(pixels: i32) -> Self {
        Self(pixels)
    }

    pub const fn pixels(self) -> i32 {
        self.0
    }

    pub fn saturating_add(self, pixels: i32) -> Self {
        Self(self.0.saturating_add(pixels))
    }

    pub fn clamp(self, min: i32, max: i32) -> Self {
        Self(self.0.clamp(min, max))
    }
}

pub enum LayoutNode {
    Leaf {
        style: LayoutStyle,
        size: Size,
        offset: Option<Offset>,
    },
    Container {
        style: LayoutStyle,
        direction: Direction,
        children: Vec<LayoutNode>,
        offset: Option<Offset>,
        scroll_offset: ScrollOffset,
    },
}

impl LayoutNode {
    pub fn leaf(width: u32, height: u32) -> Self {
        Self::Leaf {
            style: LayoutStyle::default(),
            size: Size { width, height },
            offset: None,
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self::Container {
            style: LayoutStyle::default(),
            direction: Direction::Row,
            children,
            offset: None,
            scroll_offset: ScrollOffset::ZERO,
        }
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self::Container {
            style: LayoutStyle::default(),
            direction: Direction::Column,
            children,
            offset: None,
            scroll_offset: ScrollOffset::ZERO,
        }
    }

    pub fn with_style(mut self, style: LayoutStyle) -> Self {
        match &mut self {
            Self::Leaf { style: current, .. } | Self::Container { style: current, .. } => {
                *current = style
            }
        }
        self
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        let offset = Some(Offset { x, y });
        match &mut self {
            Self::Leaf {
                offset: current, ..
            }
            | Self::Container {
                offset: current, ..
            } => *current = offset,
        }
        self
    }

    pub fn with_scroll(mut self, scroll: ScrollOffset) -> Self {
        if let Self::Container {
            scroll_offset: current,
            ..
        } = &mut self
        {
            *current = scroll;
        }
        self
    }

    fn style(&self) -> &LayoutStyle {
        match self {
            Self::Leaf { style, .. } | Self::Container { style, .. } => style,
        }
    }

    fn offset(&self) -> Option<Offset> {
        match self {
            Self::Leaf { offset, .. } | Self::Container { offset, .. } => *offset,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayoutBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub children: Vec<LayoutBox>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutError {
    InvalidDimension,
    InvalidFlex,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidDimension => "invalid size: percent must be finite and non-negative",
            Self::InvalidFlex => {
                "invalid flex: factors and basis percent must be finite and non-negative"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LayoutError {}

pub fn validate(style: &LayoutStyle) -> Result<(), LayoutError> {
    validate_dimension(style.width)?;
    validate_dimension(style.height)?;
    validate_flex(style)
}

pub struct NodeShape<'a, C> {
    pub style: &'a LayoutStyle,
    pub direction: Direction,
    pub offset: Option<Offset>,
    pub scroll: ScrollOffset,
    pub content: C,
    pub children: &'a [u32],
    pub transparent: bool,
}

struct Entry<C> {
    node: Option<NodeId>,
    nodes: Vec<NodeId>,
    style: TaffyStyle,
    layout: LayoutStyle,
    direction: Direction,
    offset: Option<Offset>,
    scroll: ScrollOffset,
    content: C,
    children: Vec<u32>,
}

pub struct Layouter<C> {
    tree: TaffyTree<C>,
    entries: HashMap<u32, Entry<C>>,
    live: HashSet<u32>,
}

impl<C: PartialEq + Clone> Layouter<C> {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            entries: HashMap::new(),
            live: HashSet::new(),
        }
    }

    pub fn begin(&mut self) {
        self.live.clear();
    }

    pub fn finish(&mut self) {
        let gone = self
            .entries
            .keys()
            .filter(|key| !self.live.contains(key))
            .copied()
            .collect::<Vec<_>>();
        for key in gone {
            let entry = self.entries.remove(&key).expect("layout entry");
            if let Some(node) = entry.node {
                self.tree.remove(node).expect("layout tree");
            }
        }
    }

    pub fn node(&mut self, key: u32, shape: NodeShape<C>) {
        self.live.insert(key);
        let taffy = taffy_style(shape.style, shape.direction, shape.offset);
        let context = shape.content.clone();
        let nodes = self.child_nodes(shape.children);
        let placed = !shape.transparent;

        if self
            .entries
            .get(&key)
            .is_some_and(|entry| entry.node.is_some() != placed)
        {
            let entry = self.entries.remove(&key).expect("layout entry");
            if let Some(node) = entry.node {
                self.tree.remove(node).expect("layout tree");
            }
        }

        if let Some(entry) = self.entries.get_mut(&key) {
            entry.layout = *shape.style;
            entry.direction = shape.direction;
            entry.offset = shape.offset;
            entry.scroll = shape.scroll;
            entry.children = shape.children.to_vec();
            let Some(node) = entry.node else {
                entry.nodes = nodes;
                return;
            };
            if entry.style != taffy {
                entry.style = taffy.clone();
                self.tree.set_style(node, taffy).expect("layout tree");
            }
            if entry.content != context {
                entry.content = context.clone();
                self.tree
                    .set_node_context(node, Some(context))
                    .expect("layout tree");
            }
            if entry.nodes != nodes {
                entry.nodes = nodes;
                self.tree
                    .set_children(node, &entry.nodes)
                    .expect("layout tree");
            }
            return;
        }

        self.entries.insert(
            key,
            Entry {
                node: placed.then(|| {
                    let node = self
                        .tree
                        .new_leaf_with_context(taffy.clone(), context.clone())
                        .expect("layout tree");
                    if !nodes.is_empty() {
                        self.tree.set_children(node, &nodes).expect("layout tree");
                    }
                    node
                }),
                nodes,
                style: taffy,
                layout: *shape.style,
                direction: shape.direction,
                offset: shape.offset,
                scroll: shape.scroll,
                content: shape.content,
                children: shape.children.to_vec(),
            },
        );
    }

    fn child_nodes(&self, children: &[u32]) -> Vec<NodeId> {
        let mut nodes = Vec::new();
        for key in children {
            let entry = self.entries.get(key).expect("layout child");
            match entry.node {
                Some(node) => nodes.push(node),
                None => nodes.extend(self.child_nodes(&entry.children)),
            }
        }
        nodes
    }
}

impl<C> Layouter<C> {
    pub fn touch(&mut self, key: u32) {
        let Some(entry) = self.entries.get(&key) else {
            return;
        };
        match entry.node {
            Some(node) => self.tree.mark_dirty(node).expect("layout tree"),
            None => {
                for child in entry.children.clone() {
                    self.touch(child);
                }
            }
        }
    }

    pub fn compute(
        &mut self,
        root: u32,
        viewport: Size,
        mut measure: impl FnMut(&mut C, Option<u32>) -> Size,
    ) {
        let node = self
            .entries
            .get(&root)
            .expect("layout root")
            .node
            .expect("layout root");

        // NOTE: taffy only rejects ids it did not create, and every id here comes from this tree
        self.tree
            .compute_layout_with_measure(
                node,
                TaffySize {
                    width: AvailableSpace::Definite(viewport.width as f32),
                    height: AvailableSpace::Definite(viewport.height as f32),
                },
                |inputs, _node, context, style| {
                    compute_leaf_layout(
                        inputs,
                        style,
                        |_, _| 0.0,
                        |_, available| {
                            let width = available
                                .width
                                .into_option()
                                .map(|width| width.max(0.0) as u32);
                            let size = context
                                .map(|content| measure(content, width))
                                .unwrap_or_default();
                            TaffySize {
                                width: size.width as f32,
                                height: size.height as f32,
                            }
                        },
                    )
                },
            )
            .expect("layout tree");
    }

    pub fn collect_into(&self, root: u32, boxes: &mut [LayoutBox]) {
        let origin = Placement {
            frame: (0, 0),
            origin: (0, 0),
            direction: Direction::Row,
            scroll: 0,
        };
        self.collect_entry(root, 0, 0, origin, boxes);
    }

    fn collect_entry(
        &self,
        key: u32,
        x: i32,
        y: i32,
        placement: Placement,
        boxes: &mut [LayoutBox],
    ) {
        let entry = self.entries.get(&key).expect("layout entry");
        let own = match entry.node {
            Some(node) => {
                let placed = self.tree.layout(node).expect("layout tree");
                boxes[key as usize] = LayoutBox {
                    x,
                    y,
                    width: placed.size.width.max(0.0) as u32,
                    height: placed.size.height.max(0.0) as u32,
                    children: Vec::new(),
                };
                Placement {
                    frame: (x, y),
                    origin: (
                        clamp_i32(
                            i64::from(x)
                                + i64::from(entry.layout.border_width)
                                + i64::from(entry.layout.padding.left),
                        ),
                        clamp_i32(
                            i64::from(y)
                                + i64::from(entry.layout.border_width)
                                + i64::from(entry.layout.padding.top),
                        ),
                    ),
                    direction: entry.direction,
                    scroll: entry.scroll.pixels(),
                }
            }
            None => placement,
        };

        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for child_key in &entry.children {
            let child = self.entries.get(child_key).expect("layout child");
            let (child_x, child_y) = match child.node {
                Some(node) => {
                    let placed = self.tree.layout(node).expect("layout tree");
                    let location = (placed.location.x as i32, placed.location.y as i32);
                    child_position(
                        own.direction,
                        own.scroll,
                        own.frame,
                        own.origin,
                        child.offset,
                        location,
                    )
                }
                None => (own.frame.0, own.frame.1),
            };
            self.collect_entry(*child_key, child_x, child_y, own, boxes);
            if entry.node.is_none() {
                let placed = &boxes[*child_key as usize];
                let right = placed.x + placed.width as i32;
                let bottom = placed.y + placed.height as i32;
                bounds = Some(match bounds {
                    Some((left, top, right_old, bottom_old)) => (
                        left.min(placed.x),
                        top.min(placed.y),
                        right_old.max(right),
                        bottom_old.max(bottom),
                    ),
                    None => (placed.x, placed.y, right, bottom),
                });
            }
        }

        if entry.node.is_none() {
            let (left, top, right, bottom) = bounds.unwrap_or((x, y, x, y));
            boxes[key as usize] = LayoutBox {
                x: left,
                y: top,
                width: (right - left).max(0) as u32,
                height: (bottom - top).max(0) as u32,
                children: Vec::new(),
            };
        }
    }
}

#[derive(Clone, Copy)]
struct Placement {
    frame: (i32, i32),
    origin: (i32, i32),
    direction: Direction,
    scroll: i32,
}

impl<C: PartialEq + Clone> Default for Layouter<C> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn layout(root: &LayoutNode, viewport: Size) -> Result<LayoutBox, LayoutError> {
    let mut tree = TaffyTree::<Size>::new();
    let root_id = build(&mut tree, root)?;
    compute(&mut tree, root_id, viewport);
    Ok(collect(&tree, root_id, root, 0, 0))
}

fn build(tree: &mut TaffyTree<Size>, node: &LayoutNode) -> Result<NodeId, LayoutError> {
    let style = node.style();
    validate(style)?;

    let id = match node {
        LayoutNode::Leaf { size, offset, .. } => {
            let taffy = taffy_style(style, Direction::Row, *offset);
            tree.new_leaf_with_context(taffy, *size)
                .expect("layout tree")
        }
        LayoutNode::Container {
            direction,
            children,
            offset,
            ..
        } => {
            let ids = children
                .iter()
                .map(|child| build(tree, child))
                .collect::<Result<Vec<_>, LayoutError>>()?;
            let taffy = taffy_style(style, *direction, *offset);
            tree.new_with_children(taffy, &ids).expect("layout tree")
        }
    };

    Ok(id)
}

fn compute(tree: &mut TaffyTree<Size>, root: NodeId, viewport: Size) {
    // NOTE: taffy only rejects ids it did not create, and every id here comes from this tree
    tree.compute_layout_with_measure(
        root,
        TaffySize {
            width: AvailableSpace::Definite(viewport.width as f32),
            height: AvailableSpace::Definite(viewport.height as f32),
        },
        |inputs, _node, context, style| {
            compute_leaf_layout(
                inputs,
                style,
                |_, _| 0.0,
                |_, _| {
                    let size = context.map(|size| *size).unwrap_or_default();
                    TaffySize {
                        width: size.width as f32,
                        height: size.height as f32,
                    }
                },
            )
        },
    )
    .expect("layout tree");
}

fn taffy_style(style: &LayoutStyle, direction: Direction, offset: Option<Offset>) -> TaffyStyle {
    let length = |value: u32| LengthPercentage::length(value as f32);
    let mut output = TaffyStyle {
        box_sizing: BoxSizing::BorderBox,
        size: TaffySize {
            width: dimension(style.width),
            height: dimension(style.height),
        },
        padding: TaffyRect {
            left: length(style.padding.left),
            right: length(style.padding.right),
            top: length(style.padding.top),
            bottom: length(style.padding.bottom),
        },
        border: TaffyRect {
            left: length(style.border_width),
            right: length(style.border_width),
            top: length(style.border_width),
            bottom: length(style.border_width),
        },
        gap: TaffySize {
            width: length(style.gap),
            height: length(style.gap),
        },
        flex_direction: match direction {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        },
        align_items: match style.align {
            Align::Start => AlignItems::START,
            Align::Center => AlignItems::CENTER,
            Align::End => AlignItems::END,
            Align::Stretch => AlignItems::STRETCH,
        },
        justify_content: match style.justify {
            Justify::Start => JustifyContent::START,
            Justify::Center => JustifyContent::CENTER,
            Justify::End => JustifyContent::END,
            Justify::SpaceBetween => JustifyContent::SPACE_BETWEEN,
        },
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: dimension(style.flex_basis),
        ..TaffyStyle::default()
    };

    if let Some(offset) = offset {
        output.position = Position::Absolute;
        output.inset = TaffyRect {
            left: LengthPercentageAuto::length(offset.x as f32),
            right: LengthPercentageAuto::AUTO,
            top: LengthPercentageAuto::length(offset.y as f32),
            bottom: LengthPercentageAuto::AUTO,
        };
    }

    output
}

fn collect<C>(tree: &TaffyTree<C>, id: NodeId, node: &LayoutNode, x: i32, y: i32) -> LayoutBox {
    let placed = tree.layout(id).expect("layout tree");
    let mut output = LayoutBox {
        x,
        y,
        width: placed.size.width.max(0.0) as u32,
        height: placed.size.height.max(0.0) as u32,
        children: Vec::new(),
    };

    let LayoutNode::Container {
        style,
        direction,
        children,
        scroll_offset,
        ..
    } = node
    else {
        return output;
    };

    let ids = tree.children(id).expect("layout tree");
    let origin = (
        clamp_i32(i64::from(x) + i64::from(style.border_width) + i64::from(style.padding.left)),
        clamp_i32(i64::from(y) + i64::from(style.border_width) + i64::from(style.padding.top)),
    );
    let scroll = scroll_offset.pixels();

    output.children.reserve(children.len());
    for (child, child_id) in children.iter().zip(ids) {
        let child_placed = tree.layout(child_id).expect("layout tree");
        let location = (
            child_placed.location.x as i32,
            child_placed.location.y as i32,
        );
        let (child_x, child_y) =
            child_position(*direction, scroll, (x, y), origin, child.offset(), location);
        output
            .children
            .push(collect(tree, child_id, child, child_x, child_y));
    }

    output
}

fn child_position(
    direction: Direction,
    scroll: i32,
    frame: (i32, i32),
    origin: (i32, i32),
    offset: Option<Offset>,
    location: (i32, i32),
) -> (i32, i32) {
    match offset {
        Some(offset) => (
            clamp_i32(i64::from(origin.0) + i64::from(offset.x)),
            clamp_i32(i64::from(origin.1) + i64::from(offset.y)),
        ),
        None => {
            let mut x = clamp_i32(i64::from(frame.0) + i64::from(location.0));
            let mut y = clamp_i32(i64::from(frame.1) + i64::from(location.1));
            match direction {
                Direction::Row => x = clamp_i32(i64::from(x) - i64::from(scroll)),
                Direction::Column => y = clamp_i32(i64::from(y) - i64::from(scroll)),
            }
            (x, y)
        }
    }
}

pub fn inset_x(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.left)
        .saturating_add(style.padding.right)
}

pub fn inset_y(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.top)
        .saturating_add(style.padding.bottom)
}

fn validate_dimension(dimension: Dimension) -> Result<(), LayoutError> {
    if let Dimension::Percent(percent) = dimension
        && (!percent.is_finite() || percent < 0.0)
    {
        return Err(LayoutError::InvalidDimension);
    }
    Ok(())
}

fn dimension(value: Dimension) -> TaffyDimension {
    match value {
        Dimension::Auto => TaffyDimension::AUTO,
        Dimension::Px(value) => TaffyDimension::length(value as f32),
        Dimension::Percent(percent) => TaffyDimension::percent(percent / 100.0),
    }
}

fn validate_flex(style: &LayoutStyle) -> Result<(), LayoutError> {
    let factors = [style.flex_grow, style.flex_shrink];
    if factors
        .iter()
        .any(|factor| !factor.is_finite() || *factor < 0.0)
    {
        return Err(LayoutError::InvalidFlex);
    }
    if let Dimension::Percent(percent) = style.flex_basis
        && (!percent.is_finite() || percent < 0.0)
    {
        return Err(LayoutError::InvalidFlex);
    }
    Ok(())
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Clone)]
    struct Probe {
        id: u32,
        calls: Rc<Cell<usize>>,
    }

    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    #[test]
    fn every_layout_error_has_its_own_message() {
        let messages = [LayoutError::InvalidDimension, LayoutError::InvalidFlex]
            .map(|error| error.to_string());

        assert!(messages.iter().all(|message| !message.is_empty()));
        assert_ne!(messages[0], messages[1]);
    }

    #[test]
    fn a_clean_tree_is_not_measured_again() {
        let calls = Rc::new(Cell::new(0));
        let root_style = LayoutStyle {
            width: Dimension::Px(100),
            height: Dimension::Px(50),
            ..LayoutStyle::default()
        };
        let child_style = LayoutStyle {
            width: Dimension::Px(20),
            height: Dimension::Px(10),
            ..LayoutStyle::default()
        };
        let viewport = Size {
            width: 100,
            height: 50,
        };
        let mut layouter = Layouter::<Probe>::new();

        let build = |layouter: &mut Layouter<Probe>| {
            layouter.begin();
            layouter.node(
                1,
                NodeShape {
                    style: &child_style,
                    direction: Direction::Row,
                    offset: None,
                    scroll: ScrollOffset::ZERO,
                    content: Probe {
                        id: 1,
                        calls: Rc::clone(&calls),
                    },
                    children: &[],
                    transparent: false,
                },
            );
            layouter.node(
                0,
                NodeShape {
                    style: &root_style,
                    direction: Direction::Column,
                    offset: None,
                    scroll: ScrollOffset::ZERO,
                    content: Probe {
                        id: 0,
                        calls: Rc::clone(&calls),
                    },
                    children: &[1],
                    transparent: false,
                },
            );
            layouter.finish();
            layouter.compute(0, viewport, |content, _width| {
                content.calls.set(content.calls.get() + 1);
                Size {
                    width: 20,
                    height: 10,
                }
            });
        };

        build(&mut layouter);
        let measured = calls.get();
        assert!(measured >= 1);

        let mut first = vec![LayoutBox::default(); 2];
        layouter.collect_into(0, &mut first);
        assert_eq!(first[1].width, 20);
        assert_eq!(first[0].width, 100);

        build(&mut layouter);
        assert_eq!(calls.get(), measured);

        let mut second = vec![LayoutBox::default(); 2];
        layouter.collect_into(0, &mut second);
        assert_eq!(second, first);
    }

    fn columns(grow: f32, basis: Dimension) -> LayoutNode {
        let style = LayoutStyle {
            flex_grow: grow,
            flex_basis: basis,
            ..LayoutStyle::default()
        };
        let root = LayoutStyle {
            width: Dimension::Px(200),
            gap: 8,
            ..LayoutStyle::default()
        };
        LayoutNode::row(vec![
            LayoutNode::leaf(20, 10).with_style(style),
            LayoutNode::leaf(40, 10).with_style(style),
        ])
        .with_style(root)
    }

    #[test]
    fn flex_zero_basis_splits_the_whole_row() {
        let viewport = Size {
            width: 200,
            height: 50,
        };
        let placed = layout(&columns(1.0, Dimension::Px(0)), viewport).expect("layout");

        assert_eq!(placed.children[0].width, 96);
        assert_eq!(placed.children[1].width, 96);
        assert_eq!(placed.children[1].x, 104);
    }

    #[test]
    fn flex_without_basis_grows_from_content() {
        let viewport = Size {
            width: 200,
            height: 50,
        };
        let placed = layout(&columns(1.0, Dimension::Auto), viewport).expect("layout");

        assert_eq!(placed.children[0].width, 86);
        assert_eq!(placed.children[1].width, 106);
    }

    #[test]
    fn percent_child_of_a_flex_column_takes_the_column_width() {
        let column = || {
            let child = LayoutNode::leaf(0, 30).with_style(LayoutStyle {
                width: Dimension::Percent(100.0),
                height: Dimension::Px(30),
                ..LayoutStyle::default()
            });
            LayoutNode::column(vec![child]).with_style(LayoutStyle {
                flex_grow: 1.0,
                flex_basis: Dimension::Px(0),
                ..LayoutStyle::default()
            })
        };
        let root = LayoutNode::row(vec![column(), column()]).with_style(LayoutStyle {
            width: Dimension::Px(480),
            gap: 10,
            ..LayoutStyle::default()
        });

        let placed = layout(
            &root,
            Size {
                width: 480,
                height: 200,
            },
        )
        .expect("layout");

        assert_eq!(placed.children[0].width, 235);
        assert_eq!(placed.children[1].x, 245);
        assert_eq!(placed.children[0].children[0].width, 235);
        assert_eq!(placed.children[1].children[0].width, 235);
    }
}

use std::collections::{HashMap, HashSet};
use std::fmt;

use taffy::style_helpers::TaffyAuto;
use taffy::{
    AlignItems, AvailableSpace, BoxSizing, Dimension as TaffyDimension, FlexDirection,
    JustifyContent, LayoutInput, LayoutOutput, LengthPercentage, LengthPercentageAuto, NodeId,
    Position, Rect as TaffyRect, Size as TaffySize, Style as TaffyStyle, TaffyTree,
    compute_leaf_layout,
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
    pub spacing: u32,
    pub stretch: f32,
    pub justify: Justify,
    pub align: Align,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutPatch {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Option<Padding>,
    pub border_width: Option<u32>,
    pub spacing: Option<u32>,
    pub stretch: Option<f32>,
    pub justify: Option<Justify>,
    pub align: Option<Align>,
}

impl LayoutPatch {
    pub fn is_empty(&self) -> bool {
        self.width.is_none()
            && self.height.is_none()
            && self.padding.is_none()
            && self.border_width.is_none()
            && self.spacing.is_none()
            && self.stretch.is_none()
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
        if let Some(spacing) = patch.spacing {
            self.spacing = spacing;
        }
        if let Some(stretch) = patch.stretch {
            self.stretch = stretch;
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
    InvalidStretch,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidDimension => "invalid size: percent must be finite and non-negative",
            Self::InvalidStretch => "invalid stretch: factor must be finite and non-negative",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LayoutError {}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Available {
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Prepared {
    pub(crate) available: Available,
    width: Option<u32>,
    height: Option<u32>,
}

pub(crate) fn prepare(style: &LayoutStyle, available: Available) -> Result<Prepared, LayoutError> {
    validate_stretch(style)?;
    let width = resolve(style.width, available.width)?;
    let height = resolve(style.height, available.height)?;
    Ok(Prepared {
        available: Available {
            width: width
                .or(available.width)
                .map(|value| value.saturating_sub(inset_x(style))),
            height: height
                .or(available.height)
                .map(|value| value.saturating_sub(inset_y(style))),
        },
        width,
        height,
    })
}

pub(crate) struct NodeShape<'a, C> {
    pub(crate) style: &'a LayoutStyle,
    pub(crate) direction: Direction,
    pub(crate) offset: Option<Offset>,
    pub(crate) content: C,
    pub(crate) prepared: Prepared,
    pub(crate) children: &'a [u32],
}

struct Ctx<C> {
    content: C,
    inset: Size,
}

struct Entry<C> {
    node: NodeId,
    style: TaffyStyle,
    content: C,
    children: Vec<u32>,
}

pub(crate) struct Layouter<C> {
    tree: TaffyTree<Ctx<C>>,
    entries: HashMap<u32, Entry<C>>,
    live: HashSet<u32>,
}

impl<C: PartialEq + Clone> Layouter<C> {
    pub(crate) fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            entries: HashMap::new(),
            live: HashSet::new(),
        }
    }

    pub(crate) fn begin(&mut self) {
        self.live.clear();
    }

    pub(crate) fn finish(&mut self) {
        let gone = self
            .entries
            .keys()
            .filter(|key| !self.live.contains(key))
            .copied()
            .collect::<Vec<_>>();
        for key in gone {
            let entry = self.entries.remove(&key).expect("layout entry");
            self.tree.remove(entry.node).expect("layout tree");
        }
    }

    pub(crate) fn node(&mut self, key: u32, shape: NodeShape<C>) {
        let taffy = taffy_style(
            shape.style,
            shape.direction,
            shape.prepared.width,
            shape.prepared.height,
            shape.offset,
        );
        let context = Ctx {
            content: shape.content.clone(),
            inset: Size {
                width: inset_x(shape.style),
                height: inset_y(shape.style),
            },
        };
        let nodes = shape
            .children
            .iter()
            .map(|child| self.entries.get(child).expect("layout child").node)
            .collect::<Vec<_>>();
        self.live.insert(key);

        match self.entries.get_mut(&key) {
            Some(entry) => {
                if entry.style != taffy {
                    entry.style = taffy.clone();
                    self.tree.set_style(entry.node, taffy).expect("layout tree");
                }
                if entry.content != context.content {
                    entry.content = context.content.clone();
                    self.tree
                        .set_node_context(entry.node, Some(context))
                        .expect("layout tree");
                }
                if entry.children.as_slice() != shape.children {
                    entry.children = shape.children.to_vec();
                    self.tree
                        .set_children(entry.node, &nodes)
                        .expect("layout tree");
                }
            }
            None => {
                let node = self
                    .tree
                    .new_leaf_with_context(taffy.clone(), context)
                    .expect("layout tree");
                if !nodes.is_empty() {
                    self.tree.set_children(node, &nodes).expect("layout tree");
                }
                self.entries.insert(
                    key,
                    Entry {
                        node,
                        style: taffy,
                        content: shape.content,
                        children: shape.children.to_vec(),
                    },
                );
            }
        }
    }
}

impl<C> Layouter<C> {
    pub(crate) fn touch(&mut self, key: u32) {
        if let Some(entry) = self.entries.get(&key) {
            self.tree.mark_dirty(entry.node).expect("layout tree");
        }
    }

    pub(crate) fn compute(
        &mut self,
        root: u32,
        viewport: Size,
        mut measure: impl FnMut(&mut C, Option<u32>) -> Size,
    ) {
        let node = self.entries.get(&root).expect("layout root").node;

        // NOTE: taffy only rejects ids it did not create, and every id here comes from this tree
        self.tree
            .compute_layout_with_measure(
                node,
                TaffySize {
                    width: AvailableSpace::Definite(viewport.width as f32),
                    height: AvailableSpace::Definite(viewport.height as f32),
                },
                |inputs, _node, context, style| {
                    let content = match context {
                        Some(context) => {
                            let width = measured_width(inputs)
                                .map(|width| width.saturating_sub(context.inset.width));
                            measure(&mut context.content, width)
                        }
                        None => Size::default(),
                    };
                    leaf_layout(inputs, style, content)
                },
            )
            .expect("layout tree");
    }

    pub(crate) fn collect(&self, root: u32, shape: &LayoutNode) -> LayoutBox {
        let node = self.entries.get(&root).expect("layout root").node;
        collect(&self.tree, node, shape, 0, 0)
    }
}

impl<C: PartialEq + Clone> Default for Layouter<C> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn layout(root: &LayoutNode, viewport: Size) -> Result<LayoutBox, LayoutError> {
    let available = Available {
        width: Some(viewport.width),
        height: Some(viewport.height),
    };
    let mut tree = TaffyTree::<Size>::new();
    let root_id = build(&mut tree, root, available)?;
    compute(&mut tree, root_id, viewport);
    Ok(collect(&tree, root_id, root, 0, 0))
}

fn build(
    tree: &mut TaffyTree<Size>,
    node: &LayoutNode,
    available: Available,
) -> Result<NodeId, LayoutError> {
    let style = node.style();
    let prepared = prepare(style, available)?;

    let id = match node {
        LayoutNode::Leaf { size, offset, .. } => {
            let taffy = taffy_style(
                style,
                Direction::Row,
                prepared.width,
                prepared.height,
                *offset,
            );
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
                .map(|child| build(tree, child, prepared.available))
                .collect::<Result<Vec<_>, LayoutError>>()?;
            let taffy = taffy_style(style, *direction, prepared.width, prepared.height, *offset);
            tree.new_with_children(taffy, &ids).expect("layout tree")
        }
    };

    Ok(id)
}

fn measured_width(inputs: LayoutInput) -> Option<u32> {
    inputs
        .known_dimensions
        .width
        .or(inputs.available_space.width.into_option())
        .map(|width| width.max(0.0) as u32)
}

fn leaf_layout(inputs: LayoutInput, style: &TaffyStyle, content: Size) -> LayoutOutput {
    compute_leaf_layout(
        inputs,
        style,
        |_, _| 0.0,
        |_, _| TaffySize {
            width: content.width as f32,
            height: content.height as f32,
        },
    )
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
            let content = context.map(|size| *size).unwrap_or_default();
            leaf_layout(inputs, style, content)
        },
    )
    .expect("layout tree");
}

fn taffy_style(
    style: &LayoutStyle,
    direction: Direction,
    width: Option<u32>,
    height: Option<u32>,
    offset: Option<Offset>,
) -> TaffyStyle {
    let length = |value: u32| LengthPercentage::length(value as f32);
    let mut output = TaffyStyle {
        box_sizing: BoxSizing::BorderBox,
        size: TaffySize {
            width: width.map_or(TaffyDimension::AUTO, |value| {
                TaffyDimension::length(value as f32)
            }),
            height: height.map_or(TaffyDimension::AUTO, |value| {
                TaffyDimension::length(value as f32)
            }),
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
            width: length(style.spacing),
            height: length(style.spacing),
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
        flex_grow: style.stretch,
        flex_shrink: 0.0,
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
        width: extent(placed.size.width),
        height: extent(placed.size.height),
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
    let origin_x =
        clamp_i32(i64::from(x) + i64::from(style.border_width) + i64::from(style.padding.left));
    let origin_y =
        clamp_i32(i64::from(y) + i64::from(style.border_width) + i64::from(style.padding.top));
    let scroll = scroll_offset.pixels();

    output.children.reserve(children.len());
    for (child, child_id) in children.iter().zip(ids) {
        let child_placed = tree.layout(child_id).expect("layout tree");
        let (child_x, child_y) = match child.offset() {
            Some(offset) => (
                clamp_i32(i64::from(origin_x) + i64::from(offset.x)),
                clamp_i32(i64::from(origin_y) + i64::from(offset.y)),
            ),
            None => {
                let mut child_x =
                    clamp_i32(i64::from(x) + i64::from(signed(child_placed.location.x)));
                let mut child_y =
                    clamp_i32(i64::from(y) + i64::from(signed(child_placed.location.y)));
                match direction {
                    Direction::Row => {
                        child_x = clamp_i32(i64::from(child_x) - i64::from(scroll));
                    }
                    Direction::Column => {
                        child_y = clamp_i32(i64::from(child_y) - i64::from(scroll));
                    }
                }
                (child_x, child_y)
            }
        };
        output
            .children
            .push(collect(tree, child_id, child, child_x, child_y));
    }

    output
}

fn extent(value: f32) -> u32 {
    value.max(0.0).round() as u32
}

fn signed(value: f32) -> i32 {
    value.round() as i32
}

pub(crate) fn inset_x(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.left)
        .saturating_add(style.padding.right)
}

pub(crate) fn inset_y(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.top)
        .saturating_add(style.padding.bottom)
}

pub(crate) fn resolve(dimension: Dimension, base: Option<u32>) -> Result<Option<u32>, LayoutError> {
    match dimension {
        Dimension::Auto => Ok(None),
        Dimension::Px(value) => Ok(Some(value)),
        Dimension::Percent(percent) => {
            if !percent.is_finite() || percent < 0.0 {
                return Err(LayoutError::InvalidDimension);
            }
            Ok(base.map(|base| (f64::from(base) * f64::from(percent) / 100.0).round() as u32))
        }
    }
}

fn validate_stretch(style: &LayoutStyle) -> Result<(), LayoutError> {
    if !style.stretch.is_finite() || style.stretch < 0.0 {
        return Err(LayoutError::InvalidStretch);
    }
    Ok(())
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}
#[cfg(test)]
mod tests {
    use super::LayoutError;

    #[test]
    fn every_layout_error_has_its_own_message() {
        let messages = [LayoutError::InvalidDimension, LayoutError::InvalidStretch]
            .map(|error| error.to_string());

        assert!(messages.iter().all(|message| !message.is_empty()));
        assert_ne!(messages[0], messages[1]);
    }
}

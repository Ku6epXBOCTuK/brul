use std::collections::{HashMap, HashSet};

pub use brul_style::{GridLine, LinePlacement, RepeatCount, TrackSize};
use taffy::style_helpers::{TaffyAuto, TaffyGridLine};
use taffy::{
    AlignItems, AvailableSpace, BoxSizing, CheapCloneStr, Dimension as TaffyDimension,
    Display as TaffyDisplay, FlexDirection, GridAutoFlow as TaffyAutoFlow, GridPlacement,
    GridTemplateComponent, GridTemplateRepetition, JustifyContent, LengthPercentage,
    LengthPercentageAuto, Line as TaffyLine, MaxTrackSizingFunction, MinMax,
    MinTrackSizingFunction, NodeId, Position, Rect as TaffyRect,
    RepetitionCount as TaffyRepetitionCount, Size as TaffySize, Style as TaffyStyle, TaffyTree,
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Display {
    #[default]
    Flex,
    Grid,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Direction {
    #[default]
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GridAutoFlow {
    #[default]
    Row,
    Column,
}

macro_rules! layout_patch {
    ($($field:ident: $ty:ty),+ $(,)?) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutStyle {
            $(pub $field: $ty,)+
        }

        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct LayoutPatch {
            $(pub $field: Option<$ty>,)+
        }

        impl LayoutPatch {
            pub const DEFAULT: Self = Self {
                $($field: None,)+
            };

            pub fn is_empty(&self) -> bool {
                *self == Self::DEFAULT
            }
        }

        impl From<&LayoutStyle> for LayoutPatch {
            fn from(style: &LayoutStyle) -> Self {
                let default = LayoutStyle::default();
                Self {
                    $($field: (style.$field != default.$field).then(|| style.$field.clone()),)+
                }
            }
        }

        impl LayoutStyle {
            pub fn apply(&mut self, patch: &LayoutPatch) {
                $(
                    if let Some(value) = &patch.$field {
                        self.$field = value.clone();
                    }
                )+
            }
        }
    };
}

layout_patch! {
    width: Dimension,
    height: Dimension,
    padding: Padding,
    border_width: u32,
    gap: u32,
    flex_grow: f32,
    flex_shrink: f32,
    flex_basis: Dimension,
    display: Display,
    flex_direction: Direction,
    justify: Justify,
    align: Align,
    grid_template_columns: Vec<TrackSize>,
    grid_template_rows: Vec<TrackSize>,
    grid_column: GridLine,
    grid_row: GridLine,
    grid_auto_flow: GridAutoFlow,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LayoutError {
    #[error("invalid size: percent must be finite and non-negative")]
    InvalidDimension,
    #[error("invalid flex: factors and basis percent must be finite and non-negative")]
    InvalidFlex,
    #[error("invalid grid line: lines must be non-zero i16 and spans positive")]
    InvalidGridLine,
}

pub fn validate(style: &LayoutStyle) -> Result<(), LayoutError> {
    validate_dimension(style.width)?;
    validate_dimension(style.height)?;
    validate_flex(style)?;
    validate_grid_line(&style.grid_column)?;
    validate_grid_line(&style.grid_row)
}

pub struct NodeShape<'a, C> {
    pub style: &'a LayoutStyle,
    pub direction: Direction,
    pub offset: Option<Offset>,
    pub scroll: ScrollOffset,
    pub content: C,
    pub children: &'a [u32],
    pub transparent: bool,
    pub layer: bool,
    pub constrained: bool,
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
    layer: bool,
    constrained: bool,
}

pub struct Layouter<C> {
    tree: TaffyTree<C>,
    entries: HashMap<u32, Entry<C>>,
    live: HashSet<u32>,
    scratch: Vec<NodeId>,
    stack: Vec<u32>,
}

impl<C: PartialEq + Clone> Layouter<C> {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            entries: HashMap::new(),
            live: HashSet::new(),
            scratch: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn begin(&mut self) {
        self.live.clear();
    }

    pub fn finish(&mut self) {
        let Self {
            entries,
            live,
            tree,
            ..
        } = self;
        entries.retain(|key, entry| {
            if live.contains(key) {
                return true;
            }
            if let Some(node) = entry.node {
                tree.remove(node).expect("layout tree");
            }
            false
        });
    }

    pub fn node(&mut self, key: u32, shape: NodeShape<C>) {
        self.live.insert(key);
        self.child_nodes(shape.children);
        let placed = !shape.transparent;

        let Self {
            tree,
            entries,
            scratch,
            ..
        } = self;

        if entries
            .get(&key)
            .is_some_and(|entry| entry.node.is_some() != placed)
        {
            let entry = entries.remove(&key).expect("layout entry");
            if let Some(node) = entry.node {
                tree.remove(node).expect("layout tree");
            }
        }

        if let Some(entry) = entries.get_mut(&key) {
            entry.scroll = shape.scroll;
            entry.layer = shape.layer;
            entry.constrained = shape.constrained;
            if entry.children.as_slice() != shape.children {
                entry.children.clear();
                entry.children.extend_from_slice(shape.children);
            }
            update_nodes(entry, tree, scratch);

            let Some(node) = entry.node else {
                entry.layout = shape.style.clone();
                entry.direction = shape.direction;
                entry.offset = shape.offset;
                entry.constrained = shape.constrained;
                entry.content = shape.content;
                return;
            };
            update_style(entry, tree, node, &shape);
            if entry.content != shape.content {
                entry.content = shape.content.clone();
                tree.set_node_context(node, Some(shape.content))
                    .expect("layout tree");
            }
            return;
        }

        let taffy = taffy_style(
            shape.style,
            shape.direction,
            shape.offset,
            shape.constrained,
        );
        let content = shape.content;
        entries.insert(
            key,
            Entry {
                node: placed.then(|| {
                    let node = tree
                        .new_leaf_with_context(taffy.clone(), content.clone())
                        .expect("layout tree");
                    if !scratch.is_empty() {
                        tree.set_children(node, scratch).expect("layout tree");
                    }
                    node
                }),
                nodes: scratch.to_vec(),
                style: taffy,
                layout: shape.style.clone(),
                direction: shape.direction,
                offset: shape.offset,
                scroll: shape.scroll,
                content,
                children: shape.children.to_vec(),
                layer: shape.layer,
                constrained: shape.constrained,
            },
        );
    }

    fn child_nodes(&mut self, children: &[u32]) {
        let Self {
            entries,
            scratch,
            stack,
            ..
        } = self;
        scratch.clear();
        stack.clear();
        stack.extend(children.iter().copied().rev());
        while let Some(key) = stack.pop() {
            let entry = entries.get(&key).expect("layout child");
            match entry.node {
                Some(node) => scratch.push(node),
                None => stack.extend(entry.children.iter().copied().rev()),
            }
        }
    }
}

fn update_nodes<C>(entry: &mut Entry<C>, tree: &mut TaffyTree<C>, nodes: &[NodeId]) {
    if entry.nodes.as_slice() != nodes {
        entry.nodes.clear();
        entry.nodes.extend_from_slice(nodes);
        if let Some(node) = entry.node {
            tree.set_children(node, nodes).expect("layout tree");
        }
    }
}

fn update_style<C>(
    entry: &mut Entry<C>,
    tree: &mut TaffyTree<C>,
    node: NodeId,
    shape: &NodeShape<'_, C>,
) {
    if entry.layout != shape.style.clone()
        || entry.direction != shape.direction
        || entry.offset != shape.offset
        || entry.constrained != shape.constrained
    {
        let taffy = taffy_style(
            shape.style,
            shape.direction,
            shape.offset,
            shape.constrained,
        );
        entry.style = taffy.clone();
        tree.set_style(node, taffy).expect("layout tree");
        entry.layout = shape.style.clone();
        entry.direction = shape.direction;
        entry.offset = shape.offset;
        entry.constrained = shape.constrained;
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
            let (child_x, child_y) = if child.layer {
                (0, 0)
            } else {
                match child.node {
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
                }
            };
            self.collect_entry(*child_key, child_x, child_y, own, boxes);
            if entry.node.is_none() && !child.layer {
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

fn valid_placement(placement: LinePlacement) -> bool {
    match placement {
        LinePlacement::Auto => true,
        LinePlacement::Line(value) => i16::try_from(value).is_ok_and(|line| line != 0),
        LinePlacement::Span(value) => value > 0,
    }
}

fn validate_grid_line(line: &GridLine) -> Result<(), LayoutError> {
    let mut placements = std::iter::once(line.start).chain(line.end);
    if placements.all(valid_placement) {
        Ok(())
    } else {
        Err(LayoutError::InvalidGridLine)
    }
}

fn grid_placement(placement: LinePlacement) -> GridPlacement {
    match placement {
        LinePlacement::Auto => GridPlacement::Auto,
        LinePlacement::Line(value) => GridPlacement::from_line_index(value as i16),
        LinePlacement::Span(value) => GridPlacement::Span(value),
    }
}

fn grid_line(line: &GridLine) -> TaffyLine<GridPlacement> {
    TaffyLine {
        start: grid_placement(line.start),
        end: line.end.map_or(GridPlacement::Auto, grid_placement),
    }
}

fn min_track(size: &TrackSize) -> MinTrackSizingFunction {
    match size {
        TrackSize::Px(value) => MinTrackSizingFunction::length(*value as f32),
        TrackSize::Percent(value) => MinTrackSizingFunction::percent(*value as f32 / 100.0),
        TrackSize::Fr(_) => MinTrackSizingFunction::auto(),
        TrackSize::Minmax(min, _) => min_track(min),
        TrackSize::Repeat(..) => MinTrackSizingFunction::auto(),
    }
}

fn max_track(size: &TrackSize) -> MaxTrackSizingFunction {
    match size {
        TrackSize::Px(value) => MaxTrackSizingFunction::length(*value as f32),
        TrackSize::Percent(value) => MaxTrackSizingFunction::percent(*value as f32 / 100.0),
        TrackSize::Fr(value) => MaxTrackSizingFunction::fr(*value as f32),
        TrackSize::Minmax(_, max) => max_track(max),
        TrackSize::Repeat(..) => MaxTrackSizingFunction::auto(),
    }
}

fn repetition_count(count: &RepeatCount) -> TaffyRepetitionCount {
    match count {
        RepeatCount::Count(value) => TaffyRepetitionCount::Count(*value),
        RepeatCount::AutoFill => TaffyRepetitionCount::AutoFill,
        RepeatCount::AutoFit => TaffyRepetitionCount::AutoFit,
    }
}

fn track_component<S: CheapCloneStr>(size: &TrackSize) -> GridTemplateComponent<S> {
    match size {
        TrackSize::Repeat(count, tracks) => GridTemplateComponent::Repeat(GridTemplateRepetition {
            count: repetition_count(count),
            tracks: tracks
                .iter()
                .map(|track| MinMax {
                    min: min_track(track),
                    max: max_track(track),
                })
                .collect(),
            line_names: vec![vec![]; tracks.len() + 1],
        }),
        _ => GridTemplateComponent::Single(MinMax {
            min: min_track(size),
            max: max_track(size),
        }),
    }
}

fn taffy_style(
    style: &LayoutStyle,
    direction: Direction,
    offset: Option<Offset>,
    constrained: bool,
) -> TaffyStyle {
    let length = |value: u32| LengthPercentage::length(value as f32);
    let mut output = TaffyStyle {
        box_sizing: BoxSizing::BorderBox,
        display: match style.display {
            Display::Flex => TaffyDisplay::Flex,
            Display::Grid => TaffyDisplay::Grid,
        },
        grid_template_columns: style
            .grid_template_columns
            .iter()
            .map(track_component)
            .collect(),
        grid_template_rows: style
            .grid_template_rows
            .iter()
            .map(track_component)
            .collect(),
        grid_auto_flow: match style.grid_auto_flow {
            GridAutoFlow::Row => TaffyAutoFlow::Row,
            GridAutoFlow::Column => TaffyAutoFlow::Column,
        },
        grid_column: grid_line(&style.grid_column),
        grid_row: grid_line(&style.grid_row),
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
    if constrained {
        match direction {
            Direction::Column => output.min_size.height = length(0).into(),
            Direction::Row => output.min_size.width = length(0).into(),
        }
        if style.flex_grow > 0.0 {
            output.flex_shrink = 1.0;
        }
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

    struct Counted {
        id: u32,
        clones: Rc<Cell<usize>>,
    }

    impl PartialEq for Counted {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    impl Clone for Counted {
        fn clone(&self) -> Self {
            self.clones.set(self.clones.get() + 1);
            Self {
                id: self.id,
                clones: Rc::clone(&self.clones),
            }
        }
    }

    #[test]
    fn a_repeat_pass_reuses_layout_buffers() {
        let clones = Rc::new(Cell::new(0));
        let style = LayoutStyle::default();
        let empty: &[u32] = &[];
        let single: &[u32] = &[1];
        let mut layouter = Layouter::<Counted>::new();

        let pass = |layouter: &mut Layouter<Counted>| {
            layouter.begin();
            for (key, children) in [(1, empty), (0, single)] {
                layouter.node(
                    key,
                    NodeShape {
                        style: &style,
                        direction: Direction::Row,
                        offset: None,
                        scroll: ScrollOffset::ZERO,
                        content: Counted {
                            id: key,
                            clones: Rc::clone(&clones),
                        },
                        children,
                        transparent: false,
                        layer: false,
                        constrained: false,
                    },
                );
            }
            layouter.finish();
        };

        pass(&mut layouter);
        let baseline = clones.get();
        let (children, nodes) = {
            let root = layouter.entries.get(&0).expect("layout entry");
            (root.children.as_ptr(), root.nodes.as_ptr())
        };

        pass(&mut layouter);
        let root = layouter.entries.get(&0).expect("layout entry");
        assert_eq!(clones.get(), baseline);
        assert_eq!(root.children.as_ptr(), children);
        assert_eq!(root.nodes.as_ptr(), nodes);
    }

    #[test]
    fn every_layout_error_has_its_own_message() {
        let messages = [
            LayoutError::InvalidDimension,
            LayoutError::InvalidFlex,
            LayoutError::InvalidGridLine,
        ]
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
                    layer: false,
                    constrained: false,
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
                    layer: false,
                    constrained: false,
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

    #[test]
    fn one_shot_forwards_insets_scroll_and_offset() {
        let viewport = Size {
            width: 200,
            height: 120,
        };
        let absolute = LayoutNode::row(vec![LayoutNode::leaf(10, 10).with_offset(5, 3)])
            .with_style(LayoutStyle {
                border_width: 2,
                padding: Padding {
                    left: 10,
                    top: 4,
                    right: 1,
                    bottom: 1,
                },
                ..LayoutStyle::default()
            });
        let placed = layout(&absolute, viewport).expect("layout");
        assert_eq!((placed.children[0].x, placed.children[0].y), (17, 9));

        let flowing = |scroll| {
            LayoutNode::row(vec![LayoutNode::leaf(10, 10), LayoutNode::leaf(10, 10)])
                .with_style(LayoutStyle {
                    width: Dimension::Px(200),
                    ..LayoutStyle::default()
                })
                .with_scroll(scroll)
        };
        let plain = layout(&flowing(ScrollOffset::ZERO), viewport).expect("layout");
        let shifted = layout(&flowing(ScrollOffset::new(7)), viewport).expect("layout");
        assert_eq!(plain.children[1].x - shifted.children[1].x, 7);
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
            LayoutNode::leaf(20, 10).with_style(style.clone()),
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

    #[test]
    fn grid_tracks_reach_taffy() {
        let style = LayoutStyle {
            display: Display::Grid,
            grid_template_columns: vec![
                TrackSize::Px(60.0),
                TrackSize::Fr(2.0),
                TrackSize::Minmax(Box::new(TrackSize::Px(10.0)), Box::new(TrackSize::Px(80.0))),
                TrackSize::Percent(25.0),
            ],
            grid_template_rows: vec![TrackSize::Fr(1.0)],
            grid_auto_flow: GridAutoFlow::Column,
            ..LayoutStyle::default()
        };
        let taffy = taffy_style(&style, Direction::Row, None, false);

        assert_eq!(taffy.display, TaffyDisplay::Grid);
        assert_eq!(taffy.grid_auto_flow, TaffyAutoFlow::Column);
        assert_eq!(taffy.grid_template_columns.len(), 4);
        assert_eq!(taffy.grid_template_rows.len(), 1);
        let Some(GridTemplateComponent::Single(first)) = taffy.grid_template_columns.first() else {
            panic!("a track component");
        };
        assert_eq!(first.min, MinTrackSizingFunction::length(60.0));
        assert_eq!(first.max, MaxTrackSizingFunction::length(60.0));
        let Some(GridTemplateComponent::Single(flexible)) = taffy.grid_template_columns.get(1)
        else {
            panic!("a track component");
        };
        assert_eq!(flexible.min, MinTrackSizingFunction::auto());
        assert_eq!(flexible.max, MaxTrackSizingFunction::fr(2.0));
    }

    #[test]
    fn grid_repeat_tracks_reach_taffy() {
        let style = LayoutStyle {
            display: Display::Grid,
            grid_template_columns: vec![TrackSize::Repeat(
                RepeatCount::Count(2),
                vec![TrackSize::Px(40.0), TrackSize::Fr(1.0)],
            )],
            grid_template_rows: vec![TrackSize::Px(20.0)],
            ..LayoutStyle::default()
        };
        let taffy = taffy_style(&style, Direction::Row, None, false);
        let Some(GridTemplateComponent::Repeat(repeat)) = taffy.grid_template_columns.first()
        else {
            panic!("a repeat component");
        };
        assert_eq!(repeat.count, TaffyRepetitionCount::Count(2));
        assert_eq!(repeat.tracks.len(), 2);
        assert_eq!(repeat.tracks[0].max, MaxTrackSizingFunction::length(40.0));
        assert_eq!(repeat.line_names.len(), 3);

        let style = LayoutStyle {
            display: Display::Grid,
            width: Dimension::Px(200),
            grid_template_columns: vec![TrackSize::Repeat(
                RepeatCount::Count(2),
                vec![TrackSize::Px(80.0)],
            )],
            grid_template_rows: vec![TrackSize::Px(10.0)],
            ..LayoutStyle::default()
        };
        let grid = LayoutNode::row(vec![LayoutNode::leaf(10, 10), LayoutNode::leaf(10, 10)])
            .with_style(style.clone());
        let placed = layout(
            &grid,
            Size {
                width: 200,
                height: 200,
            },
        )
        .expect("repeat grid");
        assert_eq!((placed.children[0].x, placed.children[1].x), (0, 80));

        let style = LayoutStyle {
            grid_template_columns: vec![TrackSize::Repeat(
                RepeatCount::AutoFill,
                vec![TrackSize::Px(80.0)],
            )],
            ..style
        };
        let grid = LayoutNode::row(vec![LayoutNode::leaf(10, 10), LayoutNode::leaf(10, 10)])
            .with_style(style);
        let placed = layout(
            &grid,
            Size {
                width: 200,
                height: 200,
            },
        )
        .expect("auto-fill grid");
        assert_eq!((placed.children[0].x, placed.children[1].x), (0, 80));
    }

    #[test]
    fn grid_children_obey_explicit_lines() {
        let grid = LayoutNode::row(vec![
            LayoutNode::leaf(10, 10).with_style(LayoutStyle {
                grid_column: GridLine {
                    start: LinePlacement::Line(2),
                    end: None,
                },
                grid_row: GridLine {
                    start: LinePlacement::Line(1),
                    end: None,
                },
                ..LayoutStyle::default()
            }),
            LayoutNode::leaf(10, 10).with_style(LayoutStyle {
                grid_column: GridLine {
                    start: LinePlacement::Line(1),
                    end: Some(LinePlacement::Line(3)),
                },
                grid_row: GridLine {
                    start: LinePlacement::Line(2),
                    end: None,
                },
                ..LayoutStyle::default()
            }),
            LayoutNode::leaf(10, 10).with_style(LayoutStyle {
                grid_column: GridLine {
                    start: LinePlacement::Line(-2),
                    end: None,
                },
                grid_row: GridLine {
                    start: LinePlacement::Line(-2),
                    end: None,
                },
                ..LayoutStyle::default()
            }),
            LayoutNode::leaf(10, 10),
        ])
        .with_style(LayoutStyle {
            display: Display::Grid,
            width: Dimension::Px(200),
            grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
            grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
            ..LayoutStyle::default()
        });

        let placed = layout(
            &grid,
            Size {
                width: 200,
                height: 200,
            },
        )
        .expect("grid placement");

        assert_eq!((placed.children[0].x, placed.children[0].y), (80, 0));
        assert_eq!((placed.children[1].x, placed.children[1].y), (0, 10));
        assert_eq!(placed.children[1].width, 160);
        assert_eq!((placed.children[2].x, placed.children[2].y), (80, 10));
        assert_eq!((placed.children[3].x, placed.children[3].y), (0, 0));
    }

    #[test]
    fn grid_span_placement_covers_its_tracks() {
        let grid = LayoutNode::row(vec![
            LayoutNode::leaf(10, 10).with_style(LayoutStyle {
                grid_column: GridLine {
                    start: LinePlacement::Span(2),
                    end: None,
                },
                ..LayoutStyle::default()
            }),
            LayoutNode::leaf(10, 10),
        ])
        .with_style(LayoutStyle {
            display: Display::Grid,
            width: Dimension::Px(200),
            grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
            grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
            ..LayoutStyle::default()
        });

        let placed = layout(
            &grid,
            Size {
                width: 200,
                height: 200,
            },
        )
        .expect("grid span");

        assert_eq!(placed.children[0].width, 160);
        assert_eq!((placed.children[1].x, placed.children[1].y), (0, 10));
    }

    #[test]
    fn grid_children_reject_zero_lines() {
        let grid = LayoutNode::row(vec![LayoutNode::leaf(10, 10).with_style(LayoutStyle {
            grid_column: GridLine {
                start: LinePlacement::Line(0),
                end: None,
            },
            ..LayoutStyle::default()
        })]);

        assert_eq!(
            layout(
                &grid,
                Size {
                    width: 200,
                    height: 200,
                },
            ),
            Err(LayoutError::InvalidGridLine)
        );
    }

    #[test]
    fn grid_auto_flow_places_children() {
        let grid = |flow| {
            LayoutNode::row(vec![
                LayoutNode::leaf(10, 10),
                LayoutNode::leaf(10, 10),
                LayoutNode::leaf(10, 10),
                LayoutNode::leaf(10, 10),
            ])
            .with_style(LayoutStyle {
                display: Display::Grid,
                width: Dimension::Px(200),
                grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
                grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
                grid_auto_flow: flow,
                ..LayoutStyle::default()
            })
        };
        let viewport = Size {
            width: 200,
            height: 200,
        };

        let row = layout(&grid(GridAutoFlow::Row), viewport).expect("row flow");
        assert_eq!((row.children[1].x, row.children[1].y), (80, 0));
        assert_eq!((row.children[2].x, row.children[2].y), (0, 10));

        let column = layout(&grid(GridAutoFlow::Column), viewport).expect("column flow");
        assert_eq!((column.children[1].x, column.children[1].y), (0, 10));
        assert_eq!((column.children[2].x, column.children[2].y), (80, 0));
    }
}

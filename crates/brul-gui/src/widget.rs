use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::Color;
use crate::layout::{
    self, Direction, LayoutBox, LayoutError, LayoutNode, LayoutStyle, Offset, Size,
};
use crate::text::{TextEngine, TextError, TextLayout, TextStyle, VerticalAlign};
use crate::texture::Texture;

static NEXT_WIDGET_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WidgetId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetKind {
    View,
    Text,
    Image,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WidgetState {
    pub hovered: bool,
    pub pressed: bool,
    pub checked: bool,
    pub selected: bool,
    pub disabled: bool,
    pub visible: bool,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            hovered: false,
            pressed: false,
            checked: false,
            selected: false,
            disabled: false,
            visible: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetStyle {
    pub background: Color,
    pub border: Color,
    pub text: Color,
    pub radius: u32,
    pub tint: Option<Color>,
    pub opacity: f32,
    pub vertical_align: VerticalAlign,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            border: Color::TRANSPARENT,
            text: Color::rgb(15, 23, 42),
            radius: 0,
            tint: None,
            opacity: 1.0,
            vertical_align: VerticalAlign::Top,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StyleState {
    Hovered,
    Pressed,
    Checked,
    Selected,
    Disabled,
}

impl StyleState {
    pub const ALL: [StyleState; 5] = [
        StyleState::Hovered,
        StyleState::Pressed,
        StyleState::Checked,
        StyleState::Selected,
        StyleState::Disabled,
    ];

    pub fn flags(self) -> WidgetState {
        match self {
            StyleState::Hovered => WidgetState {
                hovered: true,
                ..WidgetState::default()
            },
            StyleState::Pressed => WidgetState {
                pressed: true,
                ..WidgetState::default()
            },
            StyleState::Checked => WidgetState {
                checked: true,
                ..WidgetState::default()
            },
            StyleState::Selected => WidgetState {
                selected: true,
                ..WidgetState::default()
            },
            StyleState::Disabled => WidgetState {
                disabled: true,
                ..WidgetState::default()
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StyleSet {
    base: WidgetStyle,
    hovered: Option<WidgetStyle>,
    pressed: Option<WidgetStyle>,
    checked: Option<WidgetStyle>,
    selected: Option<WidgetStyle>,
    disabled: Option<WidgetStyle>,
}

impl StyleSet {
    pub fn new(base: WidgetStyle) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn with(mut self, state: StyleState, style: WidgetStyle) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(style),
            StyleState::Pressed => self.pressed = Some(style),
            StyleState::Checked => self.checked = Some(style),
            StyleState::Selected => self.selected = Some(style),
            StyleState::Disabled => self.disabled = Some(style),
        }
        self
    }

    pub fn base(&self) -> WidgetStyle {
        self.base
    }

    pub fn resolve(&self, state: WidgetState) -> WidgetStyle {
        let mut style = self.base;
        if state.selected {
            style = self.selected.unwrap_or(style);
        }
        if state.checked {
            style = self.checked.unwrap_or(style);
        }
        if state.hovered {
            style = self.hovered.unwrap_or(style);
        }
        if state.pressed {
            style = self.pressed.unwrap_or(style);
        }
        if state.disabled {
            style = self.disabled.unwrap_or(style);
        }
        style
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyleSet {
    base: TextStyle,
    hovered: Option<TextStyle>,
    pressed: Option<TextStyle>,
    checked: Option<TextStyle>,
    selected: Option<TextStyle>,
    disabled: Option<TextStyle>,
}

impl TextStyleSet {
    pub fn new(base: TextStyle) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn with(mut self, state: StyleState, style: TextStyle) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(style),
            StyleState::Pressed => self.pressed = Some(style),
            StyleState::Checked => self.checked = Some(style),
            StyleState::Selected => self.selected = Some(style),
            StyleState::Disabled => self.disabled = Some(style),
        }
        self
    }

    pub fn base(&self) -> &TextStyle {
        &self.base
    }

    pub fn state(&self, state: StyleState) -> Option<&TextStyle> {
        match state {
            StyleState::Hovered => self.hovered.as_ref(),
            StyleState::Pressed => self.pressed.as_ref(),
            StyleState::Checked => self.checked.as_ref(),
            StyleState::Selected => self.selected.as_ref(),
            StyleState::Disabled => self.disabled.as_ref(),
        }
    }

    pub fn resolve(&self, state: WidgetState) -> TextStyle {
        let mut style = self.base.clone();
        if state.selected {
            style = self.selected.clone().unwrap_or(style);
        }
        if state.checked {
            style = self.checked.clone().unwrap_or(style);
        }
        if state.hovered {
            style = self.hovered.clone().unwrap_or(style);
        }
        if state.pressed {
            style = self.pressed.clone().unwrap_or(style);
        }
        if state.disabled {
            style = self.disabled.clone().unwrap_or(style);
        }
        style
    }
}

pub struct Widget {
    id: WidgetId,
    data: WidgetData,
    layout: LayoutStyle,
    styles: StyleSet,
    state: WidgetState,
    offset: Option<Offset>,
    draggable: bool,
}

enum WidgetData {
    View {
        direction: Direction,
        children: Vec<Widget>,
    },
    Text {
        content: String,
        styles: Box<TextStyleSet>,
    },
    Image(Rc<Texture>),
}

impl Widget {
    pub fn view(direction: Direction, children: Vec<Self>) -> Self {
        Self::new(WidgetData::View {
            direction,
            children,
        })
    }

    pub fn set_children(&mut self, children: Vec<Self>) {
        if let WidgetData::View {
            children: current, ..
        } = &mut self.data
        {
            *current = children;
        }
    }

    pub fn into_children(self) -> Vec<Self> {
        let Widget {
            id,
            data,
            layout,
            styles,
            state,
            offset,
            draggable,
        } = self;
        match data {
            WidgetData::View { children, .. } => children,
            data => vec![Widget {
                id,
                data,
                layout,
                styles,
                state,
                offset,
                draggable,
            }],
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self::view(Direction::Row, children)
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self::view(Direction::Column, children)
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self::styled_text(content, TextStyle::default())
    }

    pub fn styled_text(content: impl Into<String>, style: TextStyle) -> Self {
        Self::text_with_styles(content, TextStyleSet::new(style))
    }

    pub fn text_with_styles(content: impl Into<String>, styles: TextStyleSet) -> Self {
        Self::new(WidgetData::Text {
            content: content.into(),
            styles: Box::new(styles),
        })
    }

    pub fn image(texture: impl Into<Rc<Texture>>) -> Self {
        Self::new(WidgetData::Image(texture.into()))
    }

    fn new(data: WidgetData) -> Self {
        Self {
            id: WidgetId(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed)),
            data,
            layout: LayoutStyle::default(),
            styles: StyleSet::default(),
            state: WidgetState::default(),
            offset: None,
            draggable: false,
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn kind(&self) -> WidgetKind {
        match &self.data {
            WidgetData::View { .. } => WidgetKind::View,
            WidgetData::Text { .. } => WidgetKind::Text,
            WidgetData::Image(_) => WidgetKind::Image,
        }
    }

    pub fn with_layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_styles(mut self, styles: StyleSet) -> Self {
        self.styles = styles;
        self
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset = Some(Offset { x, y });
        self
    }

    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.state.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.state.visible = visible;
        self
    }
}

#[derive(Clone)]
pub struct WidgetSnapshot {
    pub size: Size,
    pub nodes: Vec<WidgetSnapshotNode>,
}

impl WidgetSnapshot {
    pub fn node(&self, id: WidgetId) -> Option<&WidgetSnapshotNode> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

#[derive(Clone)]
pub struct WidgetSnapshotNode {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub border_width: u32,
    pub content_x: i32,
    pub content_y: i32,
    pub content_width: u32,
    pub content_height: u32,
    pub state: WidgetState,
    pub style: WidgetStyle,
    pub text: Option<TextLayout>,
    pub texture: Option<Rc<Texture>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WidgetEvent {
    PointerEnter(WidgetId),
    PointerLeave(WidgetId),
    Press(WidgetId),
    Release(WidgetId),
    Click(WidgetId),
    Drag(WidgetId),
    Key(String),
    Timer(String),
}

#[derive(Clone, Debug, Default)]
pub struct Dirty {
    pub full: bool,
    pub ids: Vec<WidgetId>,
}

impl Dirty {
    pub fn all() -> Self {
        Self {
            full: true,
            ids: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.full && self.ids.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetError {
    NoLayout,
    InvalidScale,
    Layout(LayoutError),
    Text(TextError),
}

impl fmt::Display for WidgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLayout => formatter.write_str("widget tree is not laid out yet"),
            Self::InvalidScale => formatter.write_str("invalid DPI scale"),
            Self::Layout(error) => write!(formatter, "layout: {error}"),
            Self::Text(error) => write!(formatter, "text: {error}"),
        }
    }
}

impl std::error::Error for WidgetError {}

impl From<LayoutError> for WidgetError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<TextError> for WidgetError {
    fn from(error: TextError) -> Self {
        Self::Text(error)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Slot(u32);

impl Slot {
    fn index(self) -> usize {
        self.0 as usize
    }
}

struct Node {
    id: WidgetId,
    data: NodeData,
    layout: LayoutStyle,
    styles: StyleSet,
    state: WidgetState,
    offset: Option<Offset>,
    draggable: bool,
    children: Vec<Slot>,
}

enum NodeData {
    View {
        direction: Direction,
    },
    Text {
        content: String,
        styles: Box<TextStyleSet>,
    },
    Image(Rc<Texture>),
}

impl Node {
    fn kind(&self) -> WidgetKind {
        match &self.data {
            NodeData::View { .. } => WidgetKind::View,
            NodeData::Text { .. } => WidgetKind::Text,
            NodeData::Image(_) => WidgetKind::Image,
        }
    }
}

pub struct WidgetTree {
    nodes: Vec<Node>,
    index: HashMap<WidgetId, Slot>,
    free: Vec<Slot>,
    root: Slot,
    geometry: Option<Geometry>,
    snapshot: Option<WidgetSnapshot>,
    hovered: Vec<Slot>,
    pressed: Vec<Slot>,
    dirty: HashSet<WidgetId>,
    full_dirty: bool,
    layout_dirty: bool,
}

struct Geometry {
    viewport: Size,
    scale: f32,
    layout: LayoutBox,
    texts: HashMap<WidgetId, TextVariants>,
}

#[derive(Clone, Debug)]
struct TextVariants {
    base: TextLayout,
    hovered: Option<TextLayout>,
    pressed: Option<TextLayout>,
    checked: Option<TextLayout>,
    selected: Option<TextLayout>,
    disabled: Option<TextLayout>,
}

impl TextVariants {
    fn new(base: TextLayout) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            checked: None,
            selected: None,
            disabled: None,
        }
    }

    fn with(mut self, state: StyleState, layout: TextLayout) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(layout),
            StyleState::Pressed => self.pressed = Some(layout),
            StyleState::Checked => self.checked = Some(layout),
            StyleState::Selected => self.selected = Some(layout),
            StyleState::Disabled => self.disabled = Some(layout),
        }
        self
    }

    fn resolve(&self, state: WidgetState) -> TextLayout {
        let mut layout = &self.base;
        if state.selected {
            layout = self.selected.as_ref().unwrap_or(layout);
        }
        if state.checked {
            layout = self.checked.as_ref().unwrap_or(layout);
        }
        if state.hovered {
            layout = self.hovered.as_ref().unwrap_or(layout);
        }
        if state.pressed {
            layout = self.pressed.as_ref().unwrap_or(layout);
        }
        if state.disabled {
            layout = self.disabled.as_ref().unwrap_or(layout);
        }
        layout.clone()
    }
}

impl WidgetTree {
    pub fn new(root: Widget) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            index: HashMap::new(),
            free: Vec::new(),
            root: Slot(0),
            geometry: None,
            snapshot: None,
            hovered: Vec::new(),
            pressed: Vec::new(),
            dirty: HashSet::new(),
            full_dirty: true,
            layout_dirty: true,
        };
        tree.root = tree.insert(root);
        tree
    }

    fn insert(&mut self, widget: Widget) -> Slot {
        let Widget {
            id,
            data,
            layout,
            styles,
            state,
            offset,
            draggable,
        } = widget;
        let (data, children) = match data {
            WidgetData::View {
                direction,
                children,
            } => (NodeData::View { direction }, children),
            WidgetData::Text { content, styles } => {
                (NodeData::Text { content, styles }, Vec::new())
            }
            WidgetData::Image(texture) => (NodeData::Image(texture), Vec::new()),
        };
        let mut slots = Vec::with_capacity(children.len());
        for child in children {
            slots.push(self.insert(child));
        }
        let node = Node {
            id,
            data,
            layout,
            styles,
            state,
            offset,
            draggable,
            children: slots,
        };
        let slot = match self.free.pop() {
            Some(slot) => {
                self.nodes[slot.index()] = node;
                slot
            }
            None => {
                self.nodes.push(node);
                Slot((self.nodes.len() - 1) as u32)
            }
        };
        self.index.insert(id, slot);
        slot
    }

    fn extract(&mut self, slot: Slot) -> Widget {
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        let children = children
            .into_iter()
            .map(|child| self.extract(child))
            .collect();
        let node = &mut self.nodes[slot.index()];
        let data = match &mut node.data {
            NodeData::View { direction } => WidgetData::View {
                direction: *direction,
                children,
            },
            NodeData::Text { content, styles } => WidgetData::Text {
                content: std::mem::take(content),
                styles: Box::new(std::mem::take(&mut **styles)),
            },
            NodeData::Image(texture) => WidgetData::Image(Rc::clone(texture)),
        };
        let widget = Widget {
            id: node.id,
            data,
            layout: node.layout,
            styles: node.styles.clone(),
            state: node.state,
            offset: node.offset,
            draggable: node.draggable,
        };
        self.index.remove(&widget.id);
        self.free.push(slot);
        widget
    }

    fn remove(&mut self, slot: Slot) {
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        for child in children {
            self.remove(child);
        }
        self.index.remove(&self.nodes[slot.index()].id);
        self.free.push(slot);
    }

    fn node(&self, id: WidgetId) -> Option<&Node> {
        self.index.get(&id).map(|slot| &self.nodes[slot.index()])
    }

    fn node_mut(&mut self, id: WidgetId) -> Option<&mut Node> {
        let slot = *self.index.get(&id)?;
        Some(&mut self.nodes[slot.index()])
    }

    pub fn replace_root(&mut self, root: Widget) {
        self.nodes.clear();
        self.index.clear();
        self.free.clear();
        self.clear_interaction();
        self.root = self.insert(root);
        self.invalidate();
    }

    pub fn replace_children(
        &mut self,
        parent: WidgetId,
        start: usize,
        count: usize,
        children: Vec<Widget>,
    ) -> bool {
        let Some(slot) = self.index.get(&parent).copied() else {
            return false;
        };
        if !matches!(self.nodes[slot.index()].data, NodeData::View { .. }) {
            return false;
        }
        let current = self.nodes[slot.index()].children.len();
        if start > current || count > current - start {
            return false;
        }
        self.clear_interaction();
        let removed: Vec<Slot> = self.nodes[slot.index()]
            .children
            .splice(start..start + count, [])
            .collect();
        for child in removed {
            self.remove(child);
        }
        let mut inserted = Vec::with_capacity(children.len());
        for child in children {
            inserted.push(self.insert(child));
        }
        self.nodes[slot.index()]
            .children
            .splice(start..start, inserted);
        self.invalidate();
        true
    }

    pub fn replace_all_children(&mut self, parent: WidgetId, children: Vec<Widget>) -> bool {
        let Some(slot) = self.index.get(&parent).copied() else {
            return false;
        };
        if !matches!(self.nodes[slot.index()].data, NodeData::View { .. }) {
            return false;
        }
        let count = self.nodes[slot.index()].children.len();
        self.replace_children(parent, 0, count, children)
    }

    pub fn take_children(&mut self, parent: WidgetId) -> Option<Vec<Widget>> {
        let slot = *self.index.get(&parent)?;
        if !matches!(self.nodes[slot.index()].data, NodeData::View { .. }) {
            return None;
        }
        self.clear_interaction();
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        let children = children
            .into_iter()
            .map(|child| self.extract(child))
            .collect();
        self.invalidate();
        Some(children)
    }

    pub fn contains(&self, id: WidgetId) -> bool {
        self.index.contains_key(&id)
    }

    fn invalidate(&mut self) {
        self.geometry = None;
        self.snapshot = None;
        self.dirty.clear();
        self.full_dirty = true;
        self.layout_dirty = true;
    }

    pub fn layout(
        &mut self,
        engine: &mut TextEngine,
        viewport: Size,
        scale: f32,
    ) -> Result<(), WidgetError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(WidgetError::InvalidScale);
        }

        let mut texts = HashMap::new();
        let node = build_layout(
            &self.nodes,
            self.root,
            engine,
            Some(viewport.width),
            scale,
            &mut texts,
        )?;
        let placed = layout::layout(&node, viewport)?;
        let changed = self
            .geometry
            .as_ref()
            .is_none_or(|geometry| geometry.viewport != viewport || geometry.scale != scale);

        self.geometry = Some(Geometry {
            viewport,
            scale,
            layout: placed,
            texts,
        });
        self.snapshot = None;
        self.full_dirty = true;
        self.layout_dirty = false;
        self.dirty.clear();
        if changed {
            self.clear_interaction();
        }
        Ok(())
    }

    pub fn relayout(&mut self, engine: &mut TextEngine) -> Result<(), WidgetError> {
        let Some((viewport, scale)) = self
            .geometry
            .as_ref()
            .map(|geometry| (geometry.viewport, geometry.scale))
        else {
            return Ok(());
        };
        self.layout(engine, viewport, scale)
    }

    pub fn layout_dirty(&self) -> bool {
        self.layout_dirty
    }

    pub fn snapshot(&mut self) -> Result<&WidgetSnapshot, WidgetError> {
        if self.snapshot.is_none() {
            let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
            let snapshot = build_snapshot(&self.nodes, self.root, geometry);
            self.snapshot = Some(snapshot);
        }
        Ok(self.snapshot.as_ref().expect("snapshot built"))
    }

    pub fn snapshot_dirty(&self) -> bool {
        self.snapshot.is_none()
    }

    pub fn take_dirty(&mut self) -> Dirty {
        let full = std::mem::take(&mut self.full_dirty);
        Dirty {
            full,
            ids: if full {
                self.dirty.clear();
                Vec::new()
            } else {
                self.dirty.drain().collect()
            },
        }
    }

    pub fn state(&self, id: WidgetId) -> Option<WidgetState> {
        Some(self.node(id)?.state)
    }

    pub fn set_checked(&mut self, id: WidgetId, checked: bool) -> bool {
        self.update_state(id, |state| state.checked = checked)
    }

    pub fn set_selected(&mut self, id: WidgetId, selected: bool) -> bool {
        self.update_state(id, |state| state.selected = selected)
    }

    pub fn set_disabled(&mut self, id: WidgetId, disabled: bool) -> bool {
        self.update_state(id, |state| state.disabled = disabled)
    }

    pub fn set_visible(&mut self, id: WidgetId, visible: bool) -> bool {
        if !self.update_state(id, |state| state.visible = visible) {
            return false;
        }
        self.full_dirty = true;
        self.dirty.clear();
        true
    }

    pub fn set_text(&mut self, id: WidgetId, content: String) -> bool {
        let Some(node) = self.node_mut(id) else {
            return false;
        };
        let NodeData::Text {
            content: current, ..
        } = &mut node.data
        else {
            return false;
        };
        if *current == content {
            return false;
        }
        *current = content;
        self.invalidate_layout();
        true
    }

    pub fn set_offset(&mut self, id: WidgetId, x: i32, y: i32) -> bool {
        let Some(node) = self.node_mut(id) else {
            return false;
        };
        let offset = Offset { x, y };
        if node.offset == Some(offset) {
            return false;
        }
        node.offset = Some(offset);
        self.invalidate_layout();
        true
    }

    fn invalidate_layout(&mut self) {
        self.snapshot = None;
        self.full_dirty = true;
        self.layout_dirty = true;
        self.dirty.clear();
    }

    pub fn toggle_checked(&mut self, id: WidgetId) -> Option<bool> {
        let checked = {
            let node = self.node_mut(id)?;
            node.state.checked = !node.state.checked;
            node.state.checked
        };
        self.snapshot = None;
        self.mark_dirty(id);
        Some(checked)
    }

    pub fn hit(&self, x: i32, y: i32) -> Result<Option<WidgetId>, WidgetError> {
        Ok(self
            .hit_path(x, y)?
            .last()
            .map(|slot| self.nodes[slot.index()].id))
    }

    pub fn pointer_moved(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        let common = common_prefix(&self.hovered, &path);
        if common == self.hovered.len() && common == path.len() {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        let leaving: Vec<Slot> = self.hovered[common..].iter().rev().copied().collect();
        for slot in leaving {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = false;
            let id = node.id;
            self.mark_dirty(id);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for &slot in &path[common..] {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = true;
            let id = node.id;
            self.mark_dirty(id);
            events.push(WidgetEvent::PointerEnter(id));
        }
        self.hovered = path;
        self.snapshot = None;
        Ok(events)
    }

    pub fn pointer_down(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        self.set_pressed(false);

        let mut events = Vec::with_capacity(path.len());
        for &slot in path.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = true;
            let id = node.id;
            let draggable = node.draggable;
            self.mark_dirty(id);
            events.push(WidgetEvent::Press(id));
            if draggable {
                events.push(WidgetEvent::Drag(id));
            }
        }
        if !path.is_empty() {
            self.snapshot = None;
        }
        self.pressed = path;
        Ok(events)
    }

    pub fn pointer_up(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        let pressed = std::mem::take(&mut self.pressed);

        let mut events = Vec::new();
        for &slot in pressed.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = false;
            let id = node.id;
            self.mark_dirty(id);
            events.push(WidgetEvent::Release(id));
        }
        if !pressed.is_empty() && pressed.last() == path.last() {
            for &slot in path.iter().rev() {
                events.push(WidgetEvent::Click(self.nodes[slot.index()].id));
            }
        }
        if !pressed.is_empty() {
            self.snapshot = None;
        }
        Ok(events)
    }

    pub fn pointer_leave(&mut self) -> Vec<WidgetEvent> {
        let hovered = std::mem::take(&mut self.hovered);
        let pressed = std::mem::take(&mut self.pressed);
        let mut events = Vec::with_capacity(hovered.len() + pressed.len());

        for slot in hovered.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = false;
            let id = node.id;
            self.mark_dirty(id);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for slot in pressed.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = false;
            let id = node.id;
            self.mark_dirty(id);
            events.push(WidgetEvent::Release(id));
        }
        if !hovered.is_empty() || !pressed.is_empty() {
            self.snapshot = None;
        }
        events
    }

    fn update_state(&mut self, id: WidgetId, update: impl FnOnce(&mut WidgetState)) -> bool {
        let changed = {
            let Some(state) = self.node_mut(id).map(|node| &mut node.state) else {
                return false;
            };
            let before = *state;
            update(state);
            *state != before
        };
        if !changed {
            return false;
        }
        self.snapshot = None;
        self.mark_dirty(id);
        true
    }

    fn mark_dirty(&mut self, id: WidgetId) {
        if !self.full_dirty {
            self.dirty.insert(id);
        }
    }

    fn clear_interaction(&mut self) {
        let hovered = std::mem::take(&mut self.hovered);
        let pressed = std::mem::take(&mut self.pressed);
        for slot in hovered {
            if let Some(node) = self.nodes.get_mut(slot.index()) {
                node.state.hovered = false;
            }
        }
        for slot in pressed {
            if let Some(node) = self.nodes.get_mut(slot.index()) {
                node.state.pressed = false;
            }
        }
    }

    fn set_pressed(&mut self, pressed: bool) {
        let had_pressed = !self.pressed.is_empty();
        let slots = std::mem::take(&mut self.pressed);
        for &slot in &slots {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = pressed;
            let id = node.id;
            self.mark_dirty(id);
        }
        if had_pressed {
            self.snapshot = None;
        }
    }

    fn hit_path(&self, x: i32, y: i32) -> Result<Vec<Slot>, WidgetError> {
        let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
        let mut path = Vec::new();
        collect_hit(&self.nodes, self.root, &geometry.layout, x, y, &mut path);
        Ok(path)
    }

    fn active_path(&self, path: &[Slot]) -> Vec<Slot> {
        let mut active = Vec::with_capacity(path.len());
        for &slot in path {
            let Some(node) = self.nodes.get(slot.index()) else {
                break;
            };
            if node.state.disabled || !node.state.visible {
                break;
            }
            active.push(slot);
        }
        active
    }
}

fn build_layout(
    tree: &[Node],
    slot: Slot,
    engine: &mut TextEngine,
    available_width: Option<u32>,
    scale: f32,
    texts: &mut HashMap<WidgetId, TextVariants>,
) -> Result<LayoutNode, WidgetError> {
    let node = &tree[slot.index()];
    let resolved = layout::resolve(node.layout.width, available_width)?;
    let content_width = resolved
        .map(|width| width.saturating_sub(layout::inset_x(&node.layout)))
        .or(available_width);

    let layout_node = match &node.data {
        NodeData::View { direction } => {
            let mut children = Vec::with_capacity(node.children.len());
            for &child in &node.children {
                children.push(build_layout(
                    tree,
                    child,
                    engine,
                    content_width,
                    scale,
                    texts,
                )?);
            }
            match direction {
                Direction::Row => LayoutNode::row(children),
                Direction::Column => LayoutNode::column(children),
            }
        }
        NodeData::Text { content, styles } => {
            let max_width = content_width.map(|width| width as f32 / scale);
            let mut variants =
                TextVariants::new(engine.layout(content, styles.base(), max_width, scale)?);
            for state in StyleState::ALL {
                let Some(style) = styles.state(state) else {
                    continue;
                };
                if style == styles.base() {
                    continue;
                }
                variants = variants.with(state, engine.layout(content, style, max_width, scale)?);
            }
            let size = variants.base.size;
            texts.insert(node.id, variants);
            LayoutNode::leaf(size.width, size.height)
        }
        NodeData::Image(texture) => LayoutNode::leaf(texture.width(), texture.height()),
    };

    let layout_node = layout_node.with_style(node.layout);
    Ok(match node.offset {
        Some(offset) => layout_node.with_offset(offset.x, offset.y),
        None => layout_node,
    })
}

fn build_snapshot(tree: &[Node], root: Slot, geometry: &Geometry) -> WidgetSnapshot {
    let mut nodes = Vec::new();
    snapshot_widget(tree, root, &geometry.layout, &geometry.texts, &mut nodes);
    WidgetSnapshot {
        size: Size {
            width: geometry.layout.width,
            height: geometry.layout.height,
        },
        nodes,
    }
}

fn snapshot_widget(
    tree: &[Node],
    slot: Slot,
    layout_box: &LayoutBox,
    texts: &HashMap<WidgetId, TextVariants>,
    nodes: &mut Vec<WidgetSnapshotNode>,
) {
    let node = &tree[slot.index()];
    if !node.state.visible {
        return;
    }

    let style = node.styles.resolve(node.state);
    let content_x = layout_box.x.saturating_add(
        node.layout
            .border_width
            .saturating_add(node.layout.padding.left) as i32,
    );
    let content_y = layout_box.y.saturating_add(
        node.layout
            .border_width
            .saturating_add(node.layout.padding.top) as i32,
    );
    let content_width = layout_box
        .width
        .saturating_sub(layout::inset_x(&node.layout));
    let content_height = layout_box
        .height
        .saturating_sub(layout::inset_y(&node.layout));
    let text = match &node.data {
        NodeData::Text { .. } => texts
            .get(&node.id)
            .map(|variants| variants.resolve(node.state)),
        _ => None,
    };
    let texture = match &node.data {
        NodeData::Image(texture) => Some(Rc::clone(texture)),
        _ => None,
    };

    nodes.push(WidgetSnapshotNode {
        id: node.id,
        kind: node.kind(),
        x: layout_box.x,
        y: layout_box.y,
        width: layout_box.width,
        height: layout_box.height,
        border_width: node.layout.border_width,
        content_x,
        content_y,
        content_width,
        content_height,
        state: node.state,
        style,
        text,
        texture,
    });

    for (&child, child_box) in node.children.iter().zip(&layout_box.children) {
        snapshot_widget(tree, child, child_box, texts, nodes);
    }
}

fn collect_hit(
    tree: &[Node],
    slot: Slot,
    layout_box: &LayoutBox,
    x: i32,
    y: i32,
    path: &mut Vec<Slot>,
) -> bool {
    let node = &tree[slot.index()];
    if !node.state.visible || !contains(layout_box, x, y) {
        return false;
    }

    path.push(slot);
    for (&child, child_box) in node.children.iter().zip(&layout_box.children).rev() {
        if collect_hit(tree, child, child_box, x, y, path) {
            return true;
        }
    }
    true
}

fn contains(layout_box: &LayoutBox, x: i32, y: i32) -> bool {
    let right = i64::from(layout_box.x) + i64::from(layout_box.width);
    let bottom = i64::from(layout_box.y) + i64::from(layout_box.height);
    i64::from(x) >= i64::from(layout_box.x)
        && i64::from(y) >= i64::from(layout_box.y)
        && i64::from(x) < right
        && i64::from(y) < bottom
}

fn common_prefix(left: &[Slot], right: &[Slot]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use super::WidgetError;
    use crate::layout::LayoutError;
    use crate::text::TextError;

    #[test]
    fn widget_error_names_the_wrapped_source() {
        assert_ne!(
            WidgetError::NoLayout.to_string(),
            WidgetError::InvalidScale.to_string()
        );
        assert_eq!(
            WidgetError::from(LayoutError::InvalidDimension).to_string(),
            format!("layout: {}", LayoutError::InvalidDimension)
        );
        assert_eq!(
            WidgetError::from(TextError::AtlasFull).to_string(),
            format!("text: {}", TextError::AtlasFull)
        );
    }
}

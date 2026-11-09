use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::Color;
use crate::layout::{
    self, Direction, LayoutBox, LayoutError, LayoutNode, LayoutStyle, Offset, Size,
};
use crate::text::{TextEngine, TextError, TextLayout, TextStyle};
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
        style: TextStyle,
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
        Self::new(WidgetData::Text {
            content: content.into(),
            style,
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

    fn children(&self) -> &[Self] {
        match &self.data {
            WidgetData::View { children, .. } => children,
            WidgetData::Text { .. } | WidgetData::Image(_) => &[],
        }
    }

    fn children_mut(&mut self) -> &mut [Self] {
        match &mut self.data {
            WidgetData::View { children, .. } => children,
            WidgetData::Text { .. } | WidgetData::Image(_) => &mut [],
        }
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

pub struct WidgetTree {
    root: Widget,
    states: HashMap<WidgetId, WidgetState>,
    geometry: Option<Geometry>,
    snapshot: Option<WidgetSnapshot>,
    hovered: Vec<WidgetId>,
    pressed: Vec<WidgetId>,
    dirty: Vec<WidgetId>,
    full_dirty: bool,
    layout_dirty: bool,
}

struct Geometry {
    viewport: Size,
    scale: f32,
    layout: LayoutBox,
    texts: HashMap<WidgetId, TextLayout>,
}

impl WidgetTree {
    pub fn new(root: Widget) -> Self {
        let mut states = HashMap::new();
        collect_states(&root, &mut states);
        Self {
            root,
            states,
            geometry: None,
            snapshot: None,
            hovered: Vec::new(),
            pressed: Vec::new(),
            dirty: Vec::new(),
            full_dirty: true,
            layout_dirty: true,
        }
    }

    pub fn replace_root(&mut self, root: Widget) {
        self.root = root;
        self.refresh();
    }

    pub fn replace_children(
        &mut self,
        parent: WidgetId,
        start: usize,
        count: usize,
        children: Vec<Widget>,
    ) -> bool {
        let Some(widget) = find_widget_mut(&mut self.root, parent) else {
            return false;
        };
        let WidgetData::View {
            children: current, ..
        } = &mut widget.data
        else {
            return false;
        };
        if start > current.len() || count > current.len() - start {
            return false;
        }
        current.splice(start..start + count, children);
        self.refresh();
        true
    }

    pub fn replace_all_children(&mut self, parent: WidgetId, children: Vec<Widget>) -> bool {
        let count = {
            let Some(widget) = find_widget_mut(&mut self.root, parent) else {
                return false;
            };
            let WidgetData::View { children, .. } = &widget.data else {
                return false;
            };
            children.len()
        };
        self.replace_children(parent, 0, count, children)
    }

    pub fn take_children(&mut self, parent: WidgetId) -> Option<Vec<Widget>> {
        let children = {
            let widget = find_widget_mut(&mut self.root, parent)?;
            let WidgetData::View { children, .. } = &mut widget.data else {
                return None;
            };
            std::mem::take(children)
        };
        self.refresh();
        Some(children)
    }

    pub fn contains(&self, id: WidgetId) -> bool {
        fn contains(widget: &Widget, id: WidgetId) -> bool {
            widget.id == id || widget.children().iter().any(|child| contains(child, id))
        }
        contains(&self.root, id)
    }

    fn refresh(&mut self) {
        let mut states = HashMap::new();
        collect_states(&self.root, &mut states);
        self.states = states;
        self.geometry = None;
        self.snapshot = None;
        self.hovered.clear();
        self.pressed.clear();
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
        let node = build_layout(&self.root, engine, Some(viewport.width), scale, &mut texts)?;
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
            let snapshot = build_snapshot(&self.root, geometry, &self.states);
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
                Vec::new()
            } else {
                std::mem::take(&mut self.dirty)
            },
        }
    }

    pub fn state(&self, id: WidgetId) -> Option<WidgetState> {
        self.states.get(&id).copied()
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
        let Some(widget) = find_widget_mut(&mut self.root, id) else {
            return false;
        };
        let WidgetData::Text {
            content: current, ..
        } = &mut widget.data
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
        let Some(widget) = find_widget_mut(&mut self.root, id) else {
            return false;
        };
        let offset = Offset { x, y };
        if widget.offset == Some(offset) {
            return false;
        }
        widget.offset = Some(offset);
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
            let state = self.states.get_mut(&id)?;
            state.checked = !state.checked;
            state.checked
        };
        self.snapshot = None;
        self.mark_dirty(id);
        Some(checked)
    }

    pub fn hit(&self, x: i32, y: i32) -> Result<Option<WidgetId>, WidgetError> {
        Ok(self.hit_path(x, y)?.last().copied())
    }

    pub fn pointer_moved(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        let common = common_prefix(&self.hovered, &path);
        if common == self.hovered.len() && common == path.len() {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        let leaving: Vec<WidgetId> = self.hovered[common..].iter().rev().copied().collect();
        for id in leaving {
            if let Some(state) = self.states.get_mut(&id) {
                state.hovered = false;
            }
            self.mark_dirty(id);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for &id in &path[common..] {
            if let Some(state) = self.states.get_mut(&id) {
                state.hovered = true;
            }
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
        for &id in path.iter().rev() {
            if let Some(state) = self.states.get_mut(&id) {
                state.pressed = true;
            }
            self.mark_dirty(id);
            events.push(WidgetEvent::Press(id));
            if is_draggable(&self.root, id) {
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
        for &id in pressed.iter().rev() {
            if let Some(state) = self.states.get_mut(&id) {
                state.pressed = false;
            }
            self.mark_dirty(id);
            events.push(WidgetEvent::Release(id));
        }
        if !pressed.is_empty() && pressed.last() == path.last() {
            for &id in path.iter().rev() {
                events.push(WidgetEvent::Click(id));
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

        for &id in hovered.iter().rev() {
            if let Some(state) = self.states.get_mut(&id) {
                state.hovered = false;
            }
            self.mark_dirty(id);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for &id in pressed.iter().rev() {
            if let Some(state) = self.states.get_mut(&id) {
                state.pressed = false;
            }
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
            let Some(state) = self.states.get_mut(&id) else {
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
        if !self.full_dirty && !self.dirty.contains(&id) {
            self.dirty.push(id);
        }
    }

    fn clear_interaction(&mut self) {
        for &id in &self.hovered {
            if let Some(state) = self.states.get_mut(&id) {
                state.hovered = false;
            }
        }
        for &id in &self.pressed {
            if let Some(state) = self.states.get_mut(&id) {
                state.pressed = false;
            }
        }
        self.hovered.clear();
        self.pressed.clear();
    }

    fn set_pressed(&mut self, pressed: bool) {
        let had_pressed = !self.pressed.is_empty();
        let ids = std::mem::take(&mut self.pressed);
        for id in &ids {
            if let Some(state) = self.states.get_mut(id) {
                state.pressed = pressed;
            }
            self.mark_dirty(*id);
        }
        if had_pressed {
            self.snapshot = None;
        }
    }

    fn hit_path(&self, x: i32, y: i32) -> Result<Vec<WidgetId>, WidgetError> {
        let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
        let mut path = Vec::new();
        collect_hit(&self.root, &geometry.layout, &self.states, x, y, &mut path);
        Ok(path)
    }

    fn active_path(&self, path: &[WidgetId]) -> Vec<WidgetId> {
        let mut active = Vec::with_capacity(path.len());
        for &id in path {
            let Some(state) = self.states.get(&id) else {
                break;
            };
            if state.disabled || !state.visible {
                break;
            }
            active.push(id);
        }
        active
    }
}

fn collect_states(widget: &Widget, states: &mut HashMap<WidgetId, WidgetState>) {
    states.insert(widget.id, widget.state);
    for child in widget.children() {
        collect_states(child, states);
    }
}

fn find_widget_mut(widget: &mut Widget, id: WidgetId) -> Option<&mut Widget> {
    if widget.id == id {
        return Some(widget);
    }
    for child in widget.children_mut() {
        if let Some(found) = find_widget_mut(child, id) {
            return Some(found);
        }
    }
    None
}

fn is_draggable(widget: &Widget, id: WidgetId) -> bool {
    if widget.id == id {
        return widget.draggable;
    }
    widget
        .children()
        .iter()
        .any(|child| is_draggable(child, id))
}

fn build_layout(
    widget: &Widget,
    engine: &mut TextEngine,
    available_width: Option<u32>,
    scale: f32,
    texts: &mut HashMap<WidgetId, TextLayout>,
) -> Result<LayoutNode, WidgetError> {
    let resolved = layout::resolve(widget.layout.width, available_width)?;
    let content_width = resolved
        .map(|width| width.saturating_sub(layout::inset_x(&widget.layout)))
        .or(available_width);

    let node = match &widget.data {
        WidgetData::View {
            direction,
            children,
        } => {
            let mut nodes = Vec::with_capacity(children.len());
            for child in children {
                nodes.push(build_layout(child, engine, content_width, scale, texts)?);
            }
            match direction {
                Direction::Row => LayoutNode::row(nodes),
                Direction::Column => LayoutNode::column(nodes),
            }
        }
        WidgetData::Text { content, style } => {
            let max_width = content_width.map(|width| width as f32 / scale);
            let layout = engine.layout(content, style, max_width, scale)?;
            texts.insert(widget.id, layout.clone());
            LayoutNode::leaf(layout.size.width, layout.size.height)
        }
        WidgetData::Image(texture) => LayoutNode::leaf(texture.width(), texture.height()),
    };

    let node = node.with_style(widget.layout);
    Ok(match widget.offset {
        Some(offset) => node.with_offset(offset.x, offset.y),
        None => node,
    })
}

fn build_snapshot(
    root: &Widget,
    geometry: &Geometry,
    states: &HashMap<WidgetId, WidgetState>,
) -> WidgetSnapshot {
    let mut nodes = Vec::new();
    snapshot_widget(root, &geometry.layout, states, &geometry.texts, &mut nodes);
    WidgetSnapshot {
        size: Size {
            width: geometry.layout.width,
            height: geometry.layout.height,
        },
        nodes,
    }
}

fn snapshot_widget(
    widget: &Widget,
    layout_box: &LayoutBox,
    states: &HashMap<WidgetId, WidgetState>,
    texts: &HashMap<WidgetId, TextLayout>,
    nodes: &mut Vec<WidgetSnapshotNode>,
) {
    let state = *states.get(&widget.id).expect("widget state");
    if !state.visible {
        return;
    }

    let style = widget.styles.resolve(state);
    let content_x = layout_box.x.saturating_add(
        widget
            .layout
            .border_width
            .saturating_add(widget.layout.padding.left) as i32,
    );
    let content_y = layout_box.y.saturating_add(
        widget
            .layout
            .border_width
            .saturating_add(widget.layout.padding.top) as i32,
    );
    let content_width = layout_box
        .width
        .saturating_sub(layout::inset_x(&widget.layout));
    let content_height = layout_box
        .height
        .saturating_sub(layout::inset_y(&widget.layout));
    let text = match &widget.data {
        WidgetData::Text { .. } => texts.get(&widget.id).cloned(),
        _ => None,
    };
    let texture = match &widget.data {
        WidgetData::Image(texture) => Some(Rc::clone(texture)),
        _ => None,
    };

    nodes.push(WidgetSnapshotNode {
        id: widget.id,
        kind: widget.kind(),
        x: layout_box.x,
        y: layout_box.y,
        width: layout_box.width,
        height: layout_box.height,
        border_width: widget.layout.border_width,
        content_x,
        content_y,
        content_width,
        content_height,
        state,
        style,
        text,
        texture,
    });

    for (child, child_box) in widget.children().iter().zip(&layout_box.children) {
        snapshot_widget(child, child_box, states, texts, nodes);
    }
}

fn collect_hit(
    widget: &Widget,
    layout_box: &LayoutBox,
    states: &HashMap<WidgetId, WidgetState>,
    x: i32,
    y: i32,
    path: &mut Vec<WidgetId>,
) -> bool {
    let state = states.get(&widget.id).expect("widget state");
    if !state.visible || !contains(layout_box, x, y) {
        return false;
    }

    path.push(widget.id);
    for (child, child_box) in widget.children().iter().zip(&layout_box.children).rev() {
        if collect_hit(child, child_box, states, x, y, path) {
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

fn common_prefix(left: &[WidgetId], right: &[WidgetId]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

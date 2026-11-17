use super::style::Transition;
use super::*;
use crate::layout::{Dimension, LayoutBox, Layouter, Offset, ScrollOffset, Size};
use crate::text::TextLayout;
use std::collections::{HashMap, HashSet};

mod caret;
mod edit;
mod focus;
mod frame;
mod input;
mod pass;
mod pointer;
mod render;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Slot(u32);

impl Slot {
    fn index(self) -> usize {
        self.0 as usize
    }
}

struct Node {
    id: WidgetId,
    data: Content,
    layout: LayoutStyle,
    styles: StyleSet,
    class: Option<Class>,
    state: WidgetState,
    offset: Option<Offset>,
    draggable: bool,
    focusable: bool,
    layer: bool,
    modal: bool,
    scroll_offset: ScrollOffset,
    children: Vec<Slot>,
    transition: Option<Transition>,
    shown: WidgetStyle,
}

impl Node {
    fn style(&self) -> WidgetStyle {
        match &self.transition {
            Some(transition) => transition.style(),
            None => self.styles().resolve(self.state),
        }
    }

    fn styles(&self) -> StyleSet {
        let mut styles = self.styles.clone();
        if let Some(class) = &self.class {
            styles.apply_layers(&class.style, class.transitions);
        }
        styles
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut layout = self.layout;
        if let Some(class) = &self.class {
            layout.apply(&class.layout);
        }
        layout
    }
}

pub struct WidgetTree {
    nodes: Vec<Node>,
    index: HashMap<WidgetId, Slot>,
    free: Vec<Slot>,
    root: Slot,
    root_scroll: bool,
    geometry: Option<Geometry>,
    snapshot: Option<WidgetSnapshot>,
    snapshot_stale: bool,
    pending: Vec<WidgetId>,
    texts: HashMap<WidgetId, ShapedText>,
    hovered: Vec<Slot>,
    pressed: Vec<Slot>,
    focused: Option<Slot>,
    modal_focus: Option<Slot>,
    modifiers: KeyModifiers,
    dirty: HashSet<WidgetId>,
    stale: HashSet<Slot>,
    full_dirty: bool,
    layout_dirty: bool,
    active_transitions: usize,
    layouter: Layouter<Slot>,
}

struct Geometry {
    viewport: Size,
    scale: f32,
    boxes: Vec<LayoutBox>,
}

struct ShapedText {
    content: String,
    width: Option<u32>,
    scale: f32,
    variants: TextVariants,
    preedit: Option<TextLayout>,
    preedit_text: String,
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
            root_scroll: false,
            geometry: None,
            snapshot: None,
            snapshot_stale: false,
            pending: Vec::new(),
            texts: HashMap::new(),
            hovered: Vec::new(),
            pressed: Vec::new(),
            focused: None,
            modal_focus: None,
            modifiers: KeyModifiers::default(),
            dirty: HashSet::new(),
            stale: HashSet::new(),
            full_dirty: true,
            layout_dirty: true,
            active_transitions: 0,
            layouter: Layouter::new(),
        };
        tree.root = tree.insert(root);
        tree
    }

    fn insert(&mut self, widget: Widget) -> Slot {
        let Widget {
            id,
            data,
            children,
            layout,
            styles,
            state,
            offset,
            draggable,
            focusable,
            layer,
            modal,
        } = widget;
        let mut slots = Vec::with_capacity(children.len());
        for child in children {
            slots.push(self.insert(child));
        }
        let shown = styles.resolve(state);
        let node = Node {
            id,
            data,
            layout,
            styles,
            class: None,
            state,
            offset,
            draggable,
            focusable,
            layer,
            modal,
            scroll_offset: ScrollOffset::ZERO,
            children: slots,
            transition: None,
            shown,
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
        self.stale.insert(slot);
        slot
    }

    fn extract(&mut self, slot: Slot) -> Widget {
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        let children = children
            .into_iter()
            .map(|child| self.extract(child))
            .collect();
        self.release_transition(slot);
        let node = &mut self.nodes[slot.index()];
        // NOTE: the slot is freed and refilled by insert, so the placeholder never reaches a snapshot
        let data = std::mem::replace(
            &mut node.data,
            Content::View {
                direction: Direction::Column,
            },
        );
        let widget = Widget {
            id: node.id,
            data,
            children,
            layout: node.layout,
            styles: node.styles.clone(),
            state: node.state,
            offset: node.offset,
            draggable: node.draggable,
            focusable: node.focusable,
            layer: node.layer,
            modal: node.modal,
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
        self.release_transition(slot);
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

    fn set_transition(&mut self, slot: Slot, transition: Option<Transition>) {
        let was_active = self.nodes[slot.index()].transition.is_some();
        let is_active = transition.is_some();
        match (was_active, is_active) {
            (false, true) => self.active_transitions += 1,
            (true, false) => self.active_transitions -= 1,
            _ => {}
        }
        self.nodes[slot.index()].transition = transition;
    }

    fn release_transition(&mut self, slot: Slot) {
        if self.nodes[slot.index()].transition.take().is_some() {
            self.active_transitions -= 1;
        }
    }

    pub fn set_root_scroll(&mut self, scroll: bool) {
        self.root_scroll = scroll;
    }

    pub fn replace_root(&mut self, root: Widget) {
        self.nodes.clear();
        self.index.clear();
        self.free.clear();
        self.stale.clear();
        self.active_transitions = 0;
        self.geometry = None;
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
        if !self.nodes[slot.index()].data.is_group() {
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
        self.stale.insert(slot);
        self.invalidate();
        true
    }

    pub fn replace_all_children(&mut self, parent: WidgetId, children: Vec<Widget>) -> bool {
        let Some(slot) = self.index.get(&parent).copied() else {
            return false;
        };
        if !self.nodes[slot.index()].data.is_group() {
            return false;
        }
        let count = self.nodes[slot.index()].children.len();
        self.replace_children(parent, 0, count, children)
    }

    pub fn take_children(&mut self, parent: WidgetId) -> Option<Vec<Widget>> {
        let slot = *self.index.get(&parent)?;
        if !self.nodes[slot.index()].data.is_group() {
            return None;
        }
        self.clear_interaction();
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        let children = children
            .into_iter()
            .map(|child| self.extract(child))
            .collect();
        self.stale.insert(slot);
        self.invalidate();
        Some(children)
    }

    pub fn contains(&self, id: WidgetId) -> bool {
        self.index.contains_key(&id)
    }

    fn invalidate(&mut self) {
        self.snapshot = None;
        self.dirty.clear();
        self.full_dirty = true;
        self.layout_dirty = true;
    }
}

impl WidgetTree {
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

    pub fn apply_class(&mut self, id: WidgetId, class: Option<&Class>) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let class = class.cloned().filter(|class| !class.is_empty());
        let node = &mut self.nodes[slot.index()];
        if node.class == class {
            return false;
        }
        let old = node.class.clone();
        let layout = node.layout_style();
        let old_text = old
            .as_ref()
            .map(|class| class.text.clone())
            .unwrap_or_default();
        let style = node.style();
        node.class = class;
        let fresh = node.style();
        let relayout = node.layout_style() != layout
            || node
                .class
                .as_ref()
                .map(|class| class.text.clone())
                .unwrap_or_default()
                != old_text
            || style.border_radius != fresh.border_radius
            || style.scroll != fresh.scroll
            || style.shadow.x != fresh.shadow.x
            || style.shadow.y != fresh.shadow.y
            || style.shadow.blur != fresh.shadow.blur;
        if relayout {
            self.stale.insert(slot);
            self.texts.remove(&id);
            self.invalidate_layout();
            return true;
        }
        self.retarget(slot);
        self.mark_dirty(id);
        true
    }

    pub fn set_visible(&mut self, id: WidgetId, visible: bool) -> bool {
        if !self.update_state(id, |state| state.visible = visible) {
            return false;
        }
        self.sync_modal_focus();
        self.snapshot = None;
        self.full_dirty = true;
        self.dirty.clear();
        true
    }

    pub fn set_text(&mut self, id: WidgetId, content: String) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let Content::Text {
            content: current, ..
        } = &mut self.nodes[slot.index()].data
        else {
            return false;
        };
        if *current == content {
            return false;
        }
        *current = content;
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub fn set_image(&mut self, id: WidgetId, image: Image) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let Content::Image(current) = &mut self.nodes[slot.index()].data else {
            return false;
        };
        if current.ptr_eq(&image) {
            return false;
        }
        *current = image;
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub fn set_offset(&mut self, id: WidgetId, x: i32, y: i32) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let offset = Offset { x, y };
        if self.nodes[slot.index()].offset == Some(offset) {
            return false;
        }
        self.nodes[slot.index()].offset = Some(offset);
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    fn invalidate_layout(&mut self) {
        self.invalidate_snapshot();
        self.layout_dirty = true;
    }

    fn invalidate_snapshot(&mut self) {
        self.snapshot = None;
        self.full_dirty = true;
        self.dirty.clear();
    }

    pub fn toggle_checked(&mut self, id: WidgetId) -> Option<bool> {
        let checked = {
            let node = self.node_mut(id)?;
            node.state.checked = !node.state.checked;
            node.state.checked
        };
        if let Some(&slot) = self.index.get(&id) {
            self.retarget(slot);
        }
        self.mark_dirty(id);
        Some(checked)
    }
}

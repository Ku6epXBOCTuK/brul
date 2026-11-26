use super::style::Transition;
use super::*;
use crate::canvas::CanvasPainter;
use crate::effect::EffectSpec;
use crate::layout::{Dimension, LayoutBox, Layouter, Offset, ScrollOffset, Size};
use crate::text::TextLayout;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use self::frame::HitScratch;

mod caret;
mod edit;
mod focus;
mod frame;
mod input;
mod mechanism;
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

#[cfg(debug_assertions)]
fn debug_ordinal(number: usize) -> String {
    let suffix = match number % 100 {
        11..=13 => "th",
        _ => match number % 10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    format!("{number}{suffix}")
}

struct Node {
    id: WidgetId,
    data: Content,
    layout: LayoutStyle,
    styles: StyleSet,
    class: Option<Class>,
    state: WidgetState,
    offset: Option<Offset>,
    anchor: Option<Anchor>,
    draggable: bool,
    focusable: bool,
    layer: bool,
    modal: bool,
    effect: Option<EffectSpec>,
    #[cfg(debug_assertions)]
    name: Option<&'static str>,
    #[cfg(debug_assertions)]
    source: Option<&'static str>,
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
        let mut layout = self.layout.clone();
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
    hit_scratch: HitScratch,
    focused: Option<WidgetId>,
    modal_focus: Option<WidgetId>,
    modifiers: KeyModifiers,
    dirty: HashSet<WidgetId>,
    painters: HashMap<WidgetId, Rc<dyn CanvasPainter>>,
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
    preedit: Option<Rc<TextLayout>>,
    preedit_text: String,
}

#[derive(Clone, Debug)]
struct TextVariants {
    base: Rc<TextLayout>,
    hovered: Option<Rc<TextLayout>>,
    pressed: Option<Rc<TextLayout>>,
    checked: Option<Rc<TextLayout>>,
    selected: Option<Rc<TextLayout>>,
    disabled: Option<Rc<TextLayout>>,
    focused: Option<Rc<TextLayout>>,
}

impl TextVariants {
    fn new(base: Rc<TextLayout>) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            checked: None,
            selected: None,
            disabled: None,
            focused: None,
        }
    }

    fn with(mut self, state: StyleState, layout: Rc<TextLayout>) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(layout),
            StyleState::Pressed => self.pressed = Some(layout),
            StyleState::Checked => self.checked = Some(layout),
            StyleState::Selected => self.selected = Some(layout),
            StyleState::Disabled => self.disabled = Some(layout),
            StyleState::Focused => self.focused = Some(layout),
        }
        self
    }

    fn resolve(&self, state: WidgetState) -> Rc<TextLayout> {
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
        if state.focused {
            layout = self.focused.as_ref().unwrap_or(layout);
        }
        if state.pressed {
            layout = self.pressed.as_ref().unwrap_or(layout);
        }
        if state.disabled {
            layout = self.disabled.as_ref().unwrap_or(layout);
        }
        Rc::clone(layout)
    }
}

#[path = "parts/methods.rs"]
mod methods;

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
            || !style.relayout().same_layout(&fresh.relayout());
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

    pub fn set_effect(&mut self, id: WidgetId, effect: EffectSpec) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        if self.nodes[slot.index()].effect.as_ref() == Some(&effect) {
            return false;
        }
        self.nodes[slot.index()].effect = Some(effect);
        self.mark_dirty(id);
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

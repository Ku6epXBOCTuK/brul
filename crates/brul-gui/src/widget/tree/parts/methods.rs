use super::*;
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
            hit_scratch: HitScratch::default(),
            focused: None,
            modal_focus: None,
            modifiers: KeyModifiers::default(),
            dirty: HashSet::new(),
            painters: HashMap::new(),
            stale: HashSet::new(),
            full_dirty: true,
            layout_dirty: true,
            active_transitions: 0,
            layouter: Layouter::new(),
        };
        tree.root = tree.insert(root);
        tree
    }

    #[cfg(debug_assertions)]
    pub fn debug_path(&self, id: WidgetId) -> String {
        let Some(slot) = self.index.get(&id).copied() else {
            return format!("Widget({})", id.get());
        };
        let path = self.path_to(slot);
        let source = self.nodes[slot.index()].source.unwrap_or("unknown.brul");
        let segments = path
            .iter()
            .enumerate()
            .map(|(index, &current)| {
                let node = &self.nodes[current.index()];
                let name = node.name.unwrap_or_else(|| match node.data.kind() {
                    WidgetKind::View => "View",
                    WidgetKind::Structure => "Structure",
                    WidgetKind::Text => "Text",
                    WidgetKind::Input => "Input",
                    WidgetKind::Image => "Image",
                    WidgetKind::Canvas => "Canvas",
                    WidgetKind::PlatformView => "PlatformView",
                });
                let ordinal = if index == 0 {
                    1
                } else {
                    self.nodes[path[index - 1].index()]
                        .children
                        .iter()
                        .position(|&child| child == current)
                        .map_or(1, |position| position + 1)
                };
                format!("{}:{}", name, debug_ordinal(ordinal))
            })
            .collect::<Vec<_>>()
            .join(" > ");
        format!("Widget({}) {source} > {segments}", id.get())
    }

    #[cfg(debug_assertions)]
    pub fn debug_target(&self, target: EventTarget) -> String {
        match target {
            EventTarget::Window => "Window".to_owned(),
            EventTarget::Widget(id) => self.debug_path(id),
        }
    }

    fn insert(&mut self, widget: Widget) -> Slot {
        self.insert_with_source(widget, None)
    }

    fn insert_with_source(&mut self, widget: Widget, inherited: Option<&'static str>) -> Slot {
        let Widget {
            id,
            data,
            children,
            layout,
            styles,
            state,
            offset,
            anchor,
            draggable,
            focusable,
            layer,
            modal,
            effect,
            #[cfg(debug_assertions)]
            name,
            #[cfg(debug_assertions)]
            source,
        } = widget;
        #[cfg(debug_assertions)]
        let source = source.or(inherited);
        #[cfg(not(debug_assertions))]
        let source = inherited;
        let mut slots = Vec::with_capacity(children.len());
        for child in children {
            slots.push(self.insert_with_source(child, source));
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
            anchor,
            draggable,
            focusable,
            layer,
            modal,
            effect,
            #[cfg(debug_assertions)]
            name,
            #[cfg(debug_assertions)]
            source,
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
        let data = std::mem::replace(&mut node.data, Content::View);
        let widget = Widget {
            id: node.id,
            data,
            children,
            layout: node.layout.clone(),
            styles: node.styles.clone(),
            state: node.state,
            offset: node.offset,
            anchor: node.anchor,
            draggable: node.draggable,
            focusable: node.focusable,
            layer: node.layer,
            modal: node.modal,
            effect: node.effect.clone(),
            #[cfg(debug_assertions)]
            name: node.name,
            #[cfg(debug_assertions)]
            source: node.source,
        };
        self.index.remove(&widget.id);
        self.free.push(slot);
        widget
    }

    fn remove(&mut self, slot: Slot) {
        let id = self.nodes[slot.index()].id;
        if self.focused == Some(id) {
            self.focused = None;
        }
        if self.modal_focus == Some(id) {
            self.modal_focus = None;
        }
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        for child in children {
            self.remove(child);
        }
        self.release_transition(slot);
        self.index.remove(&self.nodes[slot.index()].id);
        self.free.push(slot);
    }

    pub(super) fn focused_slot(&self) -> Option<Slot> {
        self.focused.and_then(|id| self.index.get(&id).copied())
    }

    pub fn platform_view(&self, id: WidgetId) -> Option<PlatformViewId> {
        match &self.node(id)?.data {
            Content::PlatformView(spec) => Some(spec.id),
            _ => None,
        }
    }

    pub fn focused_platform_view(&self) -> Option<PlatformViewId> {
        let slot = self.focused_slot()?;
        match &self.nodes[slot.index()].data {
            Content::PlatformView(spec) => Some(spec.id),
            _ => None,
        }
    }

    pub(super) fn node(&self, id: WidgetId) -> Option<&Node> {
        self.index.get(&id).map(|slot| &self.nodes[slot.index()])
    }

    pub(super) fn node_mut(&mut self, id: WidgetId) -> Option<&mut Node> {
        let slot = *self.index.get(&id)?;
        Some(&mut self.nodes[slot.index()])
    }

    pub(super) fn set_transition(&mut self, slot: Slot, transition: Option<Transition>) {
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
        self.painters.clear();
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

    pub fn set_painter(&mut self, id: WidgetId, painter: Rc<dyn CanvasPainter>) {
        if !self.index.contains_key(&id) {
            return;
        }
        self.painters.insert(id, painter);
        self.mark_dirty(id);
    }

    pub fn remove_painter(&mut self, id: WidgetId) {
        if self.painters.remove(&id).is_some() {
            self.mark_dirty(id);
        }
    }

    fn invalidate(&mut self) {
        self.snapshot = None;
        self.dirty.clear();
        self.full_dirty = true;
        self.layout_dirty = true;
    }
}

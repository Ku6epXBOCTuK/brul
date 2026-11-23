use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use brul_gui::{PlatformViewId, WidgetKind, WidgetSnapshot};
use raw_window_handle::{HasWindowHandle, WindowHandle};

use crate::window::WindowError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformViewBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub trait PlatformViewBackend {
    fn create(&mut self, id: PlatformViewId, source: &str, bounds: PlatformViewBounds, scale: f64);
    fn set_bounds(&mut self, id: PlatformViewId, bounds: PlatformViewBounds, scale: f64);
    fn set_source(&mut self, id: PlatformViewId, source: &str);
    fn set_visible(&mut self, id: PlatformViewId, visible: bool);
    fn focus(&mut self, id: PlatformViewId);
    fn blur(&mut self);
    fn remove(&mut self, id: PlatformViewId);
}

pub type PlatformMessageHandler = Arc<dyn Fn(PlatformViewId, String) + Send + Sync + 'static>;

pub struct PlatformWindow<'a> {
    handle: WindowHandle<'a>,
}

impl<'a> PlatformWindow<'a> {
    pub fn new(handle: WindowHandle<'a>) -> Self {
        Self { handle }
    }

    pub fn handle(&self) -> WindowHandle<'_> {
        self.handle
    }
}

impl HasWindowHandle for PlatformWindow<'_> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
        Ok(self.handle)
    }
}

pub trait PlatformBackendFactory {
    fn create(
        &self,
        window: &PlatformWindow<'_>,
        messages: PlatformMessageHandler,
    ) -> Result<Box<dyn PlatformViewBackend>, WindowError>;
}

pub struct NoopPlatformBackendFactory;

impl PlatformBackendFactory for NoopPlatformBackendFactory {
    fn create(
        &self,
        _: &PlatformWindow<'_>,
        _: PlatformMessageHandler,
    ) -> Result<Box<dyn PlatformViewBackend>, WindowError> {
        Ok(Box::new(NoopBackend))
    }
}

#[derive(Clone)]
pub struct PlatformSpec {
    pub factory: Arc<dyn PlatformBackendFactory>,
    pub messages: PlatformMessageHandler,
}

struct NoopBackend;

impl PlatformViewBackend for NoopBackend {
    fn create(&mut self, _: PlatformViewId, _: &str, _: PlatformViewBounds, _: f64) {}
    fn set_bounds(&mut self, _: PlatformViewId, _: PlatformViewBounds, _: f64) {}
    fn set_source(&mut self, _: PlatformViewId, _: &str) {}
    fn set_visible(&mut self, _: PlatformViewId, _: bool) {}
    fn focus(&mut self, _: PlatformViewId) {}
    fn blur(&mut self) {}
    fn remove(&mut self, _: PlatformViewId) {}
}

#[derive(Clone)]
struct Entry {
    source: String,
    bounds: PlatformViewBounds,
    scale: f64,
    visible: bool,
}

pub(crate) struct PlatformViews {
    backend: Box<dyn PlatformViewBackend>,
    views: HashMap<PlatformViewId, Entry>,
    focused: Option<PlatformViewId>,
}

impl Default for PlatformViews {
    fn default() -> Self {
        Self::new(Box::new(NoopBackend))
    }
}

impl PlatformViews {
    pub(crate) fn new(backend: Box<dyn PlatformViewBackend>) -> Self {
        Self {
            backend,
            views: HashMap::new(),
            focused: None,
        }
    }

    pub(crate) fn set_backend(&mut self, backend: Box<dyn PlatformViewBackend>) {
        self.views.clear();
        self.focused = None;
        self.backend = backend;
    }

    pub(crate) fn focus(&mut self, id: PlatformViewId) {
        if self.focused == Some(id) {
            return;
        }
        self.focused = Some(id);
        self.backend.focus(id);
    }

    pub(crate) fn blur(&mut self) {
        if self.focused.take().is_some() {
            self.backend.blur();
        }
    }

    pub(crate) fn sync(
        &mut self,
        snapshot: &WidgetSnapshot,
        scale: f64,
    ) -> Result<(), WindowError> {
        let fresh: Vec<(PlatformViewId, String, PlatformViewBounds, bool)> = snapshot
            .nodes
            .iter()
            .filter(|node| node.kind == WidgetKind::PlatformView)
            .filter_map(|node| {
                let spec = node.platform_view.as_ref()?;
                Some((
                    spec.id,
                    spec.source.clone(),
                    PlatformViewBounds {
                        x: node.x,
                        y: node.y,
                        width: node.width,
                        height: node.height,
                    },
                    node.state.visible,
                ))
            })
            .collect();
        let current: HashSet<PlatformViewId> = fresh.iter().map(|view| view.0).collect();
        for id in self.views.keys().copied().collect::<Vec<_>>() {
            if !current.contains(&id) {
                if self.focused == Some(id) {
                    self.blur();
                }
                self.backend.remove(id);
                self.views.remove(&id);
            }
        }
        for (id, source, bounds, wanted) in fresh {
            let visible = wanted && bounds.width > 0 && bounds.height > 0;
            match self.views.get_mut(&id) {
                Some(entry) => {
                    if entry.source != source {
                        self.backend.set_source(id, &source);
                        entry.source = source;
                    }
                    if entry.bounds != bounds || entry.scale != scale {
                        self.backend.set_bounds(id, bounds, scale);
                        entry.bounds = bounds;
                        entry.scale = scale;
                    }
                    if entry.visible != visible {
                        self.backend.set_visible(id, visible);
                        entry.visible = visible;
                    }
                }
                None => {
                    self.backend.create(id, &source, bounds, scale);
                    if !visible {
                        self.backend.set_visible(id, false);
                    }
                    self.views.insert(
                        id,
                        Entry {
                            source,
                            bounds,
                            scale,
                            visible,
                        },
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brul_gui::{Dimension, LayoutStyle, Size, TextEngine, Widget, WidgetTree};
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Clone, Debug, PartialEq)]
    enum Call {
        Create(PlatformViewId, String, PlatformViewBounds, u32),
        Bounds(PlatformViewId, PlatformViewBounds, u32),
        Source(PlatformViewId, String),
        Visible(PlatformViewId, bool),
        Focus(PlatformViewId),
        Blur,
        Remove(PlatformViewId),
    }

    #[derive(Default)]
    struct Recorder {
        calls: Rc<RefCell<Vec<Call>>>,
    }

    impl Recorder {
        fn push(&self, call: Call) {
            self.calls.borrow_mut().push(call);
        }
    }

    impl PlatformViewBackend for Recorder {
        fn create(
            &mut self,
            id: PlatformViewId,
            source: &str,
            bounds: PlatformViewBounds,
            scale: f64,
        ) {
            self.push(Call::Create(id, source.to_owned(), bounds, scale as u32));
        }
        fn set_bounds(&mut self, id: PlatformViewId, bounds: PlatformViewBounds, scale: f64) {
            self.push(Call::Bounds(id, bounds, scale as u32));
        }
        fn set_source(&mut self, id: PlatformViewId, source: &str) {
            self.push(Call::Source(id, source.to_owned()));
        }
        fn set_visible(&mut self, id: PlatformViewId, visible: bool) {
            self.push(Call::Visible(id, visible));
        }
        fn focus(&mut self, id: PlatformViewId) {
            self.push(Call::Focus(id));
        }
        fn blur(&mut self) {
            self.push(Call::Blur);
        }
        fn remove(&mut self, id: PlatformViewId) {
            self.push(Call::Remove(id));
        }
    }

    fn snapshot(source: &str, width: u32) -> WidgetSnapshot {
        let mut engine = TextEngine::new();
        let mut tree = WidgetTree::new(
            Widget::platform_view(PlatformViewId::new(1), source).with_layout(LayoutStyle {
                width: Dimension::Px(width),
                height: Dimension::Px(8),
                ..LayoutStyle::default()
            }),
        );
        tree.layout(
            &mut engine,
            Size {
                width: 40,
                height: 40,
            },
            2.0,
        )
        .expect("layout");
        tree.snapshot().expect("snapshot").clone()
    }

    fn empty_snapshot() -> WidgetSnapshot {
        let mut engine = TextEngine::new();
        let mut tree = WidgetTree::new(Widget::text(""));
        tree.layout(
            &mut engine,
            Size {
                width: 40,
                height: 40,
            },
            2.0,
        )
        .expect("layout");
        tree.snapshot().expect("snapshot").clone()
    }

    #[test]
    fn a_platform_view_is_created_from_the_snapshot() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut views = PlatformViews::new(Box::new(Recorder {
            calls: Rc::clone(&calls),
        }));
        views.sync(&snapshot("about:blank", 10), 2.0).expect("sync");
        assert_eq!(
            calls.borrow().as_slice(),
            &[Call::Create(
                PlatformViewId::new(1),
                "about:blank".to_owned(),
                PlatformViewBounds {
                    x: 0,
                    y: 0,
                    width: 10,
                    height: 8,
                },
                2,
            )]
        );
    }

    #[test]
    fn only_changed_geometry_is_pushed_to_the_backend() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut views = PlatformViews::new(Box::new(Recorder {
            calls: Rc::clone(&calls),
        }));
        views.sync(&snapshot("about:blank", 10), 2.0).expect("sync");
        calls.borrow_mut().clear();
        views.sync(&snapshot("about:blank", 10), 2.0).expect("sync");
        assert!(calls.borrow().is_empty());
        views.sync(&snapshot("about:blank", 12), 2.0).expect("sync");
        assert_eq!(calls.borrow().len(), 1);
    }

    #[test]
    fn a_removed_node_releases_the_backend_view() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut views = PlatformViews::new(Box::new(Recorder {
            calls: Rc::clone(&calls),
        }));
        views.sync(&snapshot("about:blank", 10), 2.0).expect("sync");
        views.focus(PlatformViewId::new(1));
        calls.borrow_mut().clear();
        views.sync(&empty_snapshot(), 2.0).expect("sync");
        assert_eq!(
            calls.borrow().as_slice(),
            &[Call::Blur, Call::Remove(PlatformViewId::new(1))]
        );
    }

    #[test]
    fn focus_and_blur_are_forwarded_once() {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let mut views = PlatformViews::new(Box::new(Recorder {
            calls: Rc::clone(&calls),
        }));
        views.focus(PlatformViewId::new(1));
        views.focus(PlatformViewId::new(1));
        views.blur();
        views.blur();
        assert_eq!(
            calls.borrow().as_slice(),
            &[Call::Focus(PlatformViewId::new(1)), Call::Blur]
        );
    }
}

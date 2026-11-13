use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use brul_gui::WidgetEvent;
use tracing::{debug, error, info, trace};
use winit::application::ApplicationHandler;
use winit::data_transfer::{DataTransferSendBuilder, TypeHint};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

use crate::input::{scheme, system_scheme};
use crate::tree::{Step, TreeApp, tree_app, wakeup_at};
use crate::window::{ColorScheme, WindowCommand, WindowError, WindowKey, WindowSpec};

pub(crate) fn run<C, H, F>(
    initial: WindowKey,
    create: C,
    handler: H,
    closed: F,
) -> Result<(), WindowError>
where
    C: FnMut(WindowKey, ColorScheme) -> Result<WindowSpec, String> + 'static,
    H: FnMut(WindowKey, &[WidgetEvent]) -> Vec<WindowCommand> + 'static,
    F: FnMut(WindowKey) + 'static,
{
    let event_loop = EventLoop::new().map_err(|error| WindowError::stage("event loop", error))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let error = Rc::new(Cell::new(None));
    let app = Windows::new(initial, create, handler, closed, Rc::clone(&error));
    let result = event_loop.run_app(app);

    if let Some(error) = error.take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::stage("event loop", error))
}

struct Windows<C, H, F> {
    initial: WindowKey,
    create: Rc<RefCell<C>>,
    handler: Rc<RefCell<H>>,
    closed: F,
    error: Rc<Cell<Option<WindowError>>>,
    windows: Vec<(WindowKey, TreeApp)>,
    opened: bool,
}

impl<C, H, F> Windows<C, H, F>
where
    C: FnMut(WindowKey, ColorScheme) -> Result<WindowSpec, String> + 'static,
    H: FnMut(WindowKey, &[WidgetEvent]) -> Vec<WindowCommand> + 'static,
    F: FnMut(WindowKey) + 'static,
{
    fn new(
        initial: WindowKey,
        create: C,
        handler: H,
        closed: F,
        error: Rc<Cell<Option<WindowError>>>,
    ) -> Self {
        Self {
            initial,
            create: Rc::new(RefCell::new(create)),
            handler: Rc::new(RefCell::new(handler)),
            closed,
            error,
            windows: Vec::new(),
            opened: false,
        }
    }

    fn create(&mut self, key: WindowKey, scheme: ColorScheme) -> Option<WindowSpec> {
        match (self.create.borrow_mut())(key, scheme) {
            Ok(spec) => Some(spec),
            Err(message) => {
                error!(%message, "window creation failed");
                self.error.set(Some(WindowError::Stage {
                    stage: "window",
                    message,
                }));
                None
            }
        }
    }

    fn open(&mut self, event_loop: &dyn ActiveEventLoop, key: WindowKey) {
        if let Some(index) = self.windows.iter().position(|(open, _)| *open == key) {
            debug!(%key, "focusing an open window");
            self.windows[index].1.focus();
            return;
        }

        info!(%key, "opening a window");
        let scheme = system_scheme(event_loop.system_theme().as_ref());
        let Some(spec) = self.create(key, scheme) else {
            event_loop.exit();
            return;
        };
        if let Err(error) = spec.config.validate() {
            self.error.set(Some(error));
            event_loop.exit();
            return;
        }

        let handler = Rc::clone(&self.handler);
        let events = Box::new(move |events: &[WidgetEvent]| (handler.borrow_mut())(key, events));
        let mut app = tree_app(
            spec.config,
            Rc::clone(&spec.tree),
            None,
            events,
            Rc::clone(&self.error),
            spec.commands,
        );

        if app.open(event_loop, Some(spec.tree)) == Step::Exit {
            event_loop.exit();
            return;
        }
        self.windows.push((key, app));
    }

    fn index_of(&self, window_id: WindowId) -> Option<usize> {
        self.windows
            .iter()
            .position(|(_, app)| app.window_id() == Some(window_id))
    }

    fn close(&mut self, event_loop: &dyn ActiveEventLoop, index: usize) {
        let (key, _) = self.windows.remove(index);
        info!(%key, "window closed");
        (self.closed)(key);
        if self.windows.is_empty() {
            event_loop.exit();
        }
    }

    fn clipboard(&mut self, event_loop: &dyn ActiveEventLoop, index: usize) {
        if let Some(text) = self.windows[index].1.take_clipboard() {
            let data = DataTransferSendBuilder::new(text)
                .with_type(TypeHint::Plaintext, |text: &String, _| Some(text.clone()))
                .build();
            if let Err(error) = event_loop.set_clipboard(data) {
                tracing::warn!(%error, "clipboard write failed");
            }
        }
        if self.windows[index].1.take_paste() {
            match event_loop.clipboard() {
                Ok(Some(id)) => {
                    if let Err(error) = event_loop.fetch_data_transfer(id, &TypeHint::Plaintext) {
                        tracing::warn!(%error, "clipboard read failed");
                    }
                }
                Ok(None) => {}
                Err(error) => tracing::warn!(%error, "clipboard unavailable"),
            }
        }
    }

    fn apply(&mut self, event_loop: &dyn ActiveEventLoop, index: usize, step: Step) {
        match step {
            Step::Continue => {}
            Step::Close => self.close(event_loop, index),
            Step::Open(key) => self.open(event_loop, key),
            Step::Exit => event_loop.exit(),
        }
    }
}

impl<C, H, F> ApplicationHandler for Windows<C, H, F>
where
    C: FnMut(WindowKey, ColorScheme) -> Result<WindowSpec, String> + 'static,
    H: FnMut(WindowKey, &[WidgetEvent]) -> Vec<WindowCommand> + 'static,
    F: FnMut(WindowKey) + 'static,
{
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        if self.opened {
            return;
        }
        self.opened = true;
        self.open(event_loop, self.initial);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(index) = self.index_of(window_id) else {
            trace!(?window_id, "event for an unknown window");
            return;
        };
        if let WindowEvent::ThemeChanged(theme) = &event {
            let key = self.windows[index].0;
            info!(%key, ?theme, "system theme changed");
            let Some(spec) = self.create(key, scheme(theme)) else {
                event_loop.exit();
                return;
            };
            let step = self.windows[index].1.replace_tree(spec.tree);
            self.apply(event_loop, index, step);
            return;
        }

        let step = self.windows[index].1.event(window_id, &event);
        self.clipboard(event_loop, index);
        self.apply(event_loop, index, step);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        let now = Instant::now();
        let mut index = 0;
        while index < self.windows.len() {
            let step = self.windows[index].1.pump(now);
            match step {
                Step::Close => {
                    self.close(event_loop, index);
                    if self.windows.is_empty() {
                        return;
                    }
                    continue;
                }
                Step::Open(key) => self.open(event_loop, key),
                Step::Exit => {
                    event_loop.exit();
                    return;
                }
                Step::Continue => {}
            }
            index += 1;
        }

        let deadline = self
            .windows
            .iter()
            .filter_map(|(_, app)| app.deadline())
            .min();
        event_loop.set_control_flow(wakeup_at(deadline));
    }
}

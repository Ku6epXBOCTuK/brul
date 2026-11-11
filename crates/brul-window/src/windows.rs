use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use brul_gui::WidgetEvent;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

use crate::input::system_scheme;
use crate::tree::{Step, TreeApp, tree_app, wakeup_at};
use crate::window::{ColorScheme, WindowCommand, WindowError, WindowKey, WindowSpec};

pub(crate) fn run<C, H>(initial: WindowKey, create: C, handler: H) -> Result<(), WindowError>
where
    C: FnMut(WindowKey, ColorScheme) -> WindowSpec + 'static,
    H: FnMut(WindowKey, &[WidgetEvent]) -> Vec<WindowCommand> + 'static,
{
    let event_loop = EventLoop::new().map_err(|error| WindowError::stage("event loop", error))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let error = Rc::new(Cell::new(None));
    let app = Windows::new(initial, create, handler, Rc::clone(&error));
    let result = event_loop.run_app(app);

    if let Some(error) = error.take() {
        return Err(error);
    }
    result.map_err(|error| WindowError::stage("event loop", error))
}

struct Windows<C, H> {
    initial: WindowKey,
    create: Rc<RefCell<C>>,
    handler: Rc<RefCell<H>>,
    error: Rc<Cell<Option<WindowError>>>,
    windows: Vec<(WindowKey, TreeApp)>,
    opened: bool,
}

impl<C, H> Windows<C, H>
where
    C: FnMut(WindowKey, ColorScheme) -> WindowSpec + 'static,
    H: FnMut(WindowKey, &[WidgetEvent]) -> Vec<WindowCommand> + 'static,
{
    fn new(
        initial: WindowKey,
        create: C,
        handler: H,
        error: Rc<Cell<Option<WindowError>>>,
    ) -> Self {
        Self {
            initial,
            create: Rc::new(RefCell::new(create)),
            handler: Rc::new(RefCell::new(handler)),
            error,
            windows: Vec::new(),
            opened: false,
        }
    }

    fn open(&mut self, event_loop: &dyn ActiveEventLoop, key: WindowKey) {
        if let Some(index) = self.windows.iter().position(|(open, _)| *open == key) {
            self.windows[index].1.focus();
            return;
        }

        let scheme = system_scheme(event_loop.system_theme().as_ref());
        let spec = (self.create.borrow_mut())(key, scheme);
        if let Err(error) = spec.config.validate() {
            self.error.set(Some(error));
            event_loop.exit();
            return;
        }

        let factory = Rc::clone(&self.create);
        let handler = Rc::clone(&self.handler);
        let build = Box::new(move |scheme| (factory.borrow_mut())(key, scheme).tree);
        let events = Box::new(move |events: &[WidgetEvent]| (handler.borrow_mut())(key, events));
        let mut app = tree_app(
            spec.config,
            Rc::clone(&spec.tree),
            build,
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

    fn apply(&mut self, event_loop: &dyn ActiveEventLoop, index: usize, step: Step) {
        match step {
            Step::Continue => {}
            Step::Close => {
                self.windows.remove(index);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            Step::Open(key) => self.open(event_loop, key),
            Step::Exit => event_loop.exit(),
        }
    }
}

impl<C, H> ApplicationHandler for Windows<C, H>
where
    C: FnMut(WindowKey, ColorScheme) -> WindowSpec + 'static,
    H: FnMut(WindowKey, &[WidgetEvent]) -> Vec<WindowCommand> + 'static,
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
            return;
        };
        let step = self.windows[index].1.event(window_id, &event);
        self.apply(event_loop, index, step);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        let now = Instant::now();
        let mut index = 0;
        while index < self.windows.len() {
            let step = self.windows[index].1.pump(now);
            match step {
                Step::Close => {
                    self.windows.remove(index);
                    if self.windows.is_empty() {
                        event_loop.exit();
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

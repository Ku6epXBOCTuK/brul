use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use brul_gui::{CaretArea, Frame, Size, TextEngine, WidgetEvent, WidgetTree};
use tracing::{error, info};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::input::system_scheme;
use crate::surface::{Renderer, Surface, create_renderer};
use crate::window::{
    ColorScheme, WindowCommand, WindowConfig, WindowError, WindowKey, create_window,
};

mod command;
mod event;
mod frame;
mod timer;

#[cfg(test)]
mod tests;

use timer::earliest;

pub(crate) use timer::{Timer, due, wakeup_at};

type EventHandler = Box<dyn FnMut(&[WidgetEvent]) -> Vec<WindowCommand>>;
type TreeBuilder = Box<dyn FnMut(ColorScheme) -> Rc<RefCell<WidgetTree>>>;

const FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Continue,
    Close,
    Open(WindowKey),
    Exit,
}

pub(crate) struct TreeApp {
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    build: Option<TreeBuilder>,
    handler: EventHandler,
    window: Option<Arc<dyn Window>>,
    renderer: Option<Box<dyn Renderer>>,
    engine: Option<TextEngine>,
    frame: Option<Frame>,
    error: Rc<Cell<Option<WindowError>>>,
    pub(crate) timers: Vec<Timer>,
    cursor: (i32, i32),
    tick: Instant,
    next_frame: Instant,
    clipboard: Option<String>,
    paste: bool,
    ime_enabled: bool,
    ime_area: Option<CaretArea>,
    initial: Vec<WindowCommand>,
    revealed: bool,
    presented: bool,
}

impl ApplicationHandler for TreeApp {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let step = self.open(event_loop, None);
        self.finish(event_loop, step);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let step = self.event(window_id, &event);
        self.finish(event_loop, step);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        let step = self.pump(Instant::now());
        self.finish(event_loop, step);
        event_loop.set_control_flow(wakeup_at(self.deadline()));
    }
}

pub(crate) fn tree_app(
    config: WindowConfig,
    tree: Rc<RefCell<WidgetTree>>,
    build: Option<TreeBuilder>,
    handler: EventHandler,
    error: Rc<Cell<Option<WindowError>>>,
    initial: Vec<WindowCommand>,
) -> TreeApp {
    let mut app = TreeApp {
        config,
        tree,
        build,
        handler,
        window: None,
        renderer: None,
        engine: None,
        frame: None,
        error,
        timers: Vec::new(),
        cursor: (0, 0),
        tick: Instant::now(),
        next_frame: Instant::now(),
        clipboard: None,
        paste: false,
        ime_enabled: false,
        ime_area: None,
        initial: Vec::new(),
        revealed: false,
        presented: false,
    };
    let (timers, initial) = timer::split_initial(initial);
    app.timers = timers;
    app.initial = initial;
    app
}

impl TreeApp {
    pub(crate) fn open(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        tree: Option<Rc<RefCell<WidgetTree>>>,
    ) -> Step {
        if self.window.is_some() {
            return Step::Continue;
        }

        let window = match create_window(event_loop, &self.config) {
            Ok(window) => window,
            Err(error) => return self.fail(error),
        };
        let renderer = match create_renderer(self.config.backend, &window) {
            Ok(renderer) => renderer,
            Err(error) => return self.fail(error),
        };
        self.tree = match tree {
            Some(tree) => tree,
            None => match self.build.as_mut() {
                Some(build) => {
                    let scheme = system_scheme(event_loop.system_theme().as_ref());
                    build(scheme)
                }
                None => {
                    return self.fail(WindowError::NoContent);
                }
            },
        };
        let mut engine = TextEngine::new();
        let size = window.surface_size();
        let viewport = Size {
            width: size.width,
            height: size.height,
        };
        let layout =
            self.tree
                .borrow_mut()
                .layout(&mut engine, viewport, window.scale_factor() as f32);
        if let Err(error) = layout {
            return self.fail(error.into());
        }

        self.window = Some(Arc::clone(&window));
        self.renderer = Some(renderer);
        self.engine = Some(engine);
        self.frame = Some(Frame::new(size.width.max(1), size.height.max(1)));
        info!(title = %self.config.title, width = size.width, height = size.height, "window opened");
        let commands = std::mem::take(&mut self.initial);
        let step = self.run_commands(&window, commands);
        if !matches!(step, Step::Continue) {
            return step;
        }
        self.render(&window)
    }

    pub(crate) fn pump(&mut self, now: Instant) -> Step {
        brul_reactive::flush();
        if self.tree.borrow().snapshot_dirty()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }

        let mut step = Step::Continue;
        let events = due(&mut self.timers, now);
        if !events.is_empty()
            && let Some(window) = self.window.clone()
        {
            step = self.dispatch_events(&window, events);
        }

        if self.tree.borrow().animating() && now >= self.next_frame {
            self.next_frame = now + FRAME_INTERVAL;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        step
    }

    pub(crate) fn deadline(&self) -> Option<Instant> {
        let timer = self.timers.iter().map(|timer| timer.next).min();
        let frame = if self.tree.borrow().animating() {
            Some(self.next_frame)
        } else {
            None
        };
        earliest(timer, frame)
    }

    pub(crate) fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|window| window.id())
    }

    pub(crate) fn apply(&mut self, commands: Vec<WindowCommand>) -> Step {
        match self.window.clone() {
            Some(window) => self.run_commands(&window, commands),
            None => Step::Continue,
        }
    }

    pub(crate) fn focus(&self) {
        if let Some(window) = &self.window {
            window.focus_window();
        }
    }

    pub(crate) fn take_clipboard(&mut self) -> Option<String> {
        self.clipboard.take()
    }

    pub(crate) fn take_paste(&mut self) -> bool {
        std::mem::take(&mut self.paste)
    }

    fn fail(&mut self, error: WindowError) -> Step {
        error!(%error, "window failed");
        self.error.set(Some(error));
        Step::Exit
    }

    fn finish(&mut self, event_loop: &dyn ActiveEventLoop, step: Step) {
        if matches!(step, Step::Close | Step::Exit) {
            event_loop.exit();
        }
    }
}

use std::sync::Arc;
use std::time::{Duration, Instant};

use brul_gui::{
    DispatchEvent, Event, EventTarget, ImeKind, KeyPress, RawEvent, RawUiEvent, UiEvent,
};
use tracing::{debug, trace, warn};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::window::{
    ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData, Window, WindowId,
};

use crate::input::{ImeEvent, Input, event_input, point, window_input};
use crate::surface::Surface;

use super::{Step, TreeApp};

const SLOW_INPUT: Duration = Duration::from_millis(50);

impl TreeApp {
    pub(crate) fn event(&mut self, window_id: WindowId, event: &WindowEvent) -> Step {
        let Some(window) = self
            .window
            .as_ref()
            .filter(|window| window.id() == window_id)
        else {
            return Step::Continue;
        };
        let window = Arc::clone(window);

        let input = window_input(event).or_else(|| event_input(event));
        match &input {
            Some(Input::Moved(..)) | Some(Input::Redraw) => trace!(?input, "window input"),
            Some(_) => debug!(?input, "window input"),
            None => {}
        }
        let started = Instant::now();
        let step = match input {
            Some(Input::Exit) => self.dispatch_events(
                &window,
                vec![DispatchEvent::Semantic(UiEvent::window(Event::close()))],
            ),
            Some(Input::Redraw) => self.render(&window),
            Some(input) => {
                let step = self.handle(&window, input);
                self.sync_ime(window.as_ref());
                step
            }
            None => Step::Continue,
        };
        let elapsed = started.elapsed();
        if elapsed >= SLOW_INPUT {
            warn!(?elapsed, "input took too long");
        }
        step
    }

    pub(super) fn handle(&mut self, surface: &dyn Surface, input: Input) -> Step {
        if self.engine.is_some() && !self.tree.borrow().has_layout() {
            let step = self.resize_to_surface(surface);
            if !matches!(step, Step::Continue) {
                return step;
            }
            if !self.tree.borrow().has_layout() {
                debug!(?input, "input dropped until the tree is laid out");
                return Step::Continue;
            }
        }
        if self.tree.borrow().focused_platform_view().is_some()
            && !matches!(&input, Input::Key(key) if key.code == "Tab")
            && matches!(&input, Input::Key(_) | Input::Ime(_) | Input::Paste(_))
        {
            return Step::Continue;
        }
        match input {
            Input::Key(key) => self.press_key(surface, key),
            Input::Modifiers(modifiers) => {
                self.tree.borrow_mut().set_modifiers(modifiers);
                Step::Continue
            }
            Input::Paste(text) => {
                let events = self.tree.borrow_mut().paste_text(&text);
                self.dispatch_events(surface, events)
            }
            Input::Ime(ime) => self.ime(surface, ime),
            Input::Moved(x, y) => self.move_pointer(surface, x, y),
            Input::Left => self.leave_pointer(surface),
            Input::Down(x, y) => self.press_pointer(surface, x, y),
            Input::Up(x, y) => self.release_pointer(surface, x, y),
            Input::Wheel(pixels) => self.wheel(surface, pixels),
            other => self.window_action(surface, other),
        }
    }

    pub(super) fn window_action(&mut self, surface: &dyn Surface, input: Input) -> Step {
        match input {
            Input::Exit => Step::Close,
            Input::Theme(scheme) => self.change_theme(surface, scheme),
            _ => self.resize_to_surface(surface),
        }
    }

    pub(super) fn press_key(&mut self, surface: &dyn Surface, key: KeyPress) -> Step {
        let events = self.tree.borrow_mut().press_key(key);
        self.dispatch_events(surface, events)
    }

    pub(super) fn ime(&mut self, surface: &dyn Surface, ime: ImeEvent) -> Step {
        let kind = match &ime {
            ImeEvent::Enabled => ImeKind::Enabled,
            ImeEvent::Preedit(text) => ImeKind::Preedit(text.clone()),
            ImeEvent::Commit(text) => ImeKind::Commit(text.clone()),
            ImeEvent::DeleteSurrounding {
                before_bytes,
                after_bytes,
            } => ImeKind::DeleteSurrounding {
                before_bytes: *before_bytes,
                after_bytes: *after_bytes,
            },
            ImeEvent::Disabled => ImeKind::Disabled,
        };
        let events = self.tree.borrow_mut().ime(kind);
        self.dispatch_events(surface, events)
    }

    pub(super) fn sync_ime(&mut self, window: &dyn Window) {
        let area = self.tree.borrow().caret_area();
        if self.ime_area == area {
            return;
        }
        self.ime_area = area;
        let request = match area {
            Some(area) => {
                let data = ImeRequestData::default().with_cursor_area(
                    PhysicalPosition::new(area.x, area.y).into(),
                    PhysicalSize::new(area.width, area.height).into(),
                );
                if self.ime_enabled {
                    ImeRequest::Update(data)
                } else {
                    let capabilities = ImeCapabilities::new().with_cursor_area();
                    match ImeEnableRequest::new(capabilities, data) {
                        Some(enable) => ImeRequest::Enable(enable),
                        None => return,
                    }
                }
            }
            None => {
                if !self.ime_enabled {
                    return;
                }
                ImeRequest::Disable
            }
        };
        match window.request_ime_update(request) {
            Ok(()) => self.ime_enabled = area.is_some(),
            Err(error) => debug!(%error, "ime request failed"),
        }
    }

    pub(super) fn move_pointer(&mut self, surface: &dyn Surface, x: f64, y: f64) -> Step {
        let (x, y) = point(x, y);
        self.cursor = (x, y);
        let events = self.tree.borrow_mut().pointer_moved(x, y);
        self.dispatch(surface, events)
    }

    pub(super) fn wheel(&mut self, surface: &dyn Surface, pixels: f64) -> Step {
        if !pixels.is_finite() {
            return Step::Continue;
        }
        let (x, y) = self.cursor;
        let dy = pixels.round() as i32;
        let target = self
            .tree
            .borrow()
            .hit(x, y)
            .ok()
            .flatten()
            .map_or(EventTarget::Window, EventTarget::Widget);
        let scrolled = self.tree.borrow_mut().wheel(x, y, dy);
        if scrolled {
            surface.redraw();
        }
        self.dispatch_events(
            surface,
            vec![DispatchEvent::Raw(RawUiEvent::at(
                target,
                RawEvent::wheel(dy),
            ))],
        )
    }

    pub(super) fn leave_pointer(&mut self, surface: &dyn Surface) -> Step {
        let events = self.tree.borrow_mut().pointer_leave();
        self.dispatch_events(surface, events)
    }

    pub(super) fn press_pointer(&mut self, surface: &dyn Surface, x: f64, y: f64) -> Step {
        if self.config.is_header(y, surface.scale_factor()) {
            surface.drag();
            return Step::Continue;
        }
        let (x, y) = point(x, y);
        let events = self.tree.borrow_mut().pointer_down(x, y);
        self.dispatch(surface, events)
    }

    pub(super) fn release_pointer(&mut self, surface: &dyn Surface, x: f64, y: f64) -> Step {
        let (x, y) = point(x, y);
        let events = self.tree.borrow_mut().pointer_up(x, y);
        self.dispatch(surface, events)
    }

    pub(super) fn dispatch(
        &mut self,
        surface: &dyn Surface,
        events: Result<Vec<DispatchEvent>, brul_gui::WidgetError>,
    ) -> Step {
        match events {
            Ok(events) => self.dispatch_events(surface, events),
            Err(error) => self.fail(error.into()),
        }
    }

    pub(super) fn dispatch_events(
        &mut self,
        surface: &dyn Surface,
        events: Vec<DispatchEvent>,
    ) -> Step {
        self.sync_cursor(surface);
        if events.is_empty() {
            return Step::Continue;
        }
        for event in &events {
            let DispatchEvent::Semantic(event) = event else {
                continue;
            };
            let view = match (&event.event, event.target.widget()) {
                (Event::Focus { .. } | Event::Blur { .. }, Some(id)) => {
                    self.tree.borrow().platform_view(id)
                }
                _ => None,
            };
            match (&event.event, view) {
                (Event::Focus { .. }, Some(id)) => self.platform.focus(id),
                (Event::Blur { .. }, Some(_)) => self.platform.blur(),
                _ => {}
            }
        }
        if events.iter().any(|event| {
            matches!(
                event,
                DispatchEvent::Raw(RawUiEvent {
                    event: RawEvent::Drag { .. },
                    ..
                })
            )
        }) {
            surface.drag();
        }
        for event in &events {
            match event {
                DispatchEvent::Semantic(UiEvent {
                    event: Event::Copy { text },
                    ..
                }) => self.clipboard = (!text.is_empty()).then(|| text.clone()),
                DispatchEvent::Raw(RawUiEvent {
                    event: RawEvent::Paste { .. },
                    ..
                }) => self.paste = true,
                _ => {}
            }
        }
        debug!(?events, "widget events");
        let commands = (self.handler)(&events);
        debug!(?commands, "window commands");
        let step = self.run_commands(surface, &commands);
        brul_reactive::flush();
        self.sync_cursor(surface);
        if self.tree.borrow().snapshot_dirty() {
            surface.redraw();
        }
        step
    }
}

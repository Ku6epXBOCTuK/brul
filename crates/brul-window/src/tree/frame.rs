use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use brul_gui::{Dirty, Size, WidgetTree};
use tracing::{debug, info, trace, warn};

use crate::surface::{FrameInfo, Surface};
use crate::window::ColorScheme;

use super::{Step, TreeApp};

impl TreeApp {
    pub(super) fn change_theme(&mut self, surface: &dyn Surface, scheme: ColorScheme) -> Step {
        let Some(build) = self.build.as_mut() else {
            return Step::Continue;
        };
        info!(?scheme, "system theme changed");
        self.tree = build(scheme);
        self.resize_to_surface(surface)
    }

    pub(crate) fn replace_tree(&mut self, tree: Rc<RefCell<WidgetTree>>) -> Step {
        debug!("replacing the widget tree");
        self.tree = tree;
        match self.window.clone() {
            Some(window) => self.resize_to_surface(&window),
            None => Step::Continue,
        }
    }

    pub(super) fn resize_to_surface(&mut self, surface: &dyn Surface) -> Step {
        let (width, height) = surface.size();
        self.resize(surface, width, height)
    }

    pub(super) fn resize(&mut self, surface: &dyn Surface, width: u32, height: u32) -> Step {
        if width == 0 || height == 0 {
            return Step::Continue;
        }
        debug!(width, height, "window resized");
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.resize(width, height);
        }
        let layout = match self.engine.as_mut() {
            Some(engine) => self.tree.borrow_mut().layout(
                engine,
                Size { width, height },
                surface.scale_factor() as f32,
            ),
            None => Ok(()),
        };
        if let Err(error) = layout {
            return self.fail(error.into());
        }
        self.presented = false;
        surface.redraw();
        Step::Continue
    }

    pub(super) fn render(&mut self, surface: &dyn Surface) -> Step {
        brul_reactive::flush();
        match self.render_frame(surface) {
            Ok(()) => Step::Continue,
            Err(error) => self.fail(error.into()),
        }
    }

    pub(super) fn render_frame(
        &mut self,
        surface: &dyn Surface,
    ) -> Result<(), brul_gui::WidgetError> {
        trace!("rendering a frame");
        let now = Instant::now();
        let delta = now.saturating_duration_since(self.tick);
        self.tick = now;
        let delta_secs = delta.as_secs_f32();
        self.elapsed += delta_secs;
        let info = FrameInfo {
            time: self.elapsed,
            delta: delta_secs,
            frame: self.frames,
        };
        self.tree.borrow_mut().advance(delta);
        if self.tree.borrow().layout_dirty()
            && let Some(engine) = self.engine.as_mut()
        {
            if self.tree.borrow().has_layout() {
                self.tree.borrow_mut().relayout(engine)?;
            } else {
                let (width, height) = surface.size();
                self.tree.borrow_mut().layout(
                    engine,
                    Size { width, height },
                    surface.scale_factor() as f32,
                )?;
            }
        }
        let (dirty, rebuild) = {
            let mut tree = self.tree.borrow_mut();
            (tree.take_dirty(), tree.snapshot_dirty())
        };
        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(());
        };
        let Some(engine) = self.engine.as_ref() else {
            return Ok(());
        };
        let effects = self.config.effects;

        if rebuild || !dirty.is_empty() || !self.presented || crate::perf::measuring() {
            let mut tree = self.tree.borrow_mut();
            let snapshot = tree.snapshot()?;
            let paint = if dirty.is_empty() {
                Dirty::all()
            } else {
                dirty
            };
            let started = Instant::now();
            self.presented =
                renderer.render(surface, snapshot, engine.atlas(), &paint, info, &effects);
            crate::perf::raster(self.config.backend, started.elapsed());
            self.frames += 1;
            if !self.dumped
                && self.frames >= 60
                && let Some(path) = crate::perf::dump_path()
                && let Some(pixels) = renderer.capture()
            {
                self.dumped = true;
                if let Err(error) = std::fs::write(path, pixels) {
                    warn!(?error, "frame dump failed");
                }
            }
        }
        let (width, height) = surface.size();
        crate::perf::startup(self.config.backend, surface.scale_factor(), width, height);
        if !self.revealed {
            self.revealed = true;
            surface.set_visible(self.config.visible);
            crate::perf::first_frame();
        }
        Ok(())
    }
}

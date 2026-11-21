use crate::surface::Surface;
use crate::window::WindowCommand;

use super::{Step, Timer, TreeApp};

impl TreeApp {
    pub(super) fn run_commands(
        &mut self,
        surface: &dyn Surface,
        commands: &[WindowCommand],
    ) -> Step {
        for command in commands {
            match command {
                WindowCommand::Exit => return Step::Exit,
                WindowCommand::Close => return Step::Close,
                WindowCommand::Open(key) => return Step::Open(*key),
                WindowCommand::Drag => surface.drag(),
                WindowCommand::Show => {
                    self.revealed = true;
                    surface.set_visible(true);
                }
                WindowCommand::Hide => {
                    self.revealed = true;
                    surface.set_visible(false);
                }
                WindowCommand::Focus => surface.focus(),
                WindowCommand::Every {
                    interval,
                    function,
                    owner,
                } => {
                    self.timers
                        .push(Timer::new(*interval, *owner, function.clone()));
                }
                WindowCommand::SetTheme(_) => {}
                WindowCommand::AlwaysOnTop(on_top) => surface.set_always_on_top(*on_top),
            }
        }
        Step::Continue
    }
}

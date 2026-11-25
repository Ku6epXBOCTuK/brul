use std::rc::Rc;
use std::time::Instant;

use brul::gui::{CanvasPainter, Color, PaintCommand, PaintCtx, PaintRect};
use brul::{Build, PainterGuard, ViewError};

pub struct Waveform {
    _guard: PainterGuard,
}

#[brul::attach(name = "waveform")]
pub fn waveform(ctx: &Build) -> Result<Waveform, ViewError> {
    let painter = ctx.element().set_painter(Rc::new(WavePainter::new()));
    Ok(Waveform { _guard: painter })
}

struct WavePainter {
    started: Instant,
}

impl WavePainter {
    fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl CanvasPainter for WavePainter {
    fn paint(&self, ctx: &PaintCtx<'_>) -> Vec<PaintCommand> {
        let rect = ctx.rect;
        let step = (rect.width / 24) as i32;
        let bar = (step / 2).max(1);
        let phase = self.started.elapsed().as_secs_f32();
        (0..24)
            .map(|index| {
                let amplitude =
                    ((index as f32 + phase * 6.0).sin() * 0.4 + 0.5) * rect.height as f32;
                let height = amplitude as i32;
                let x = rect.x + index * step;
                PaintCommand::Rect(PaintRect {
                    rect: [x, rect.y + (rect.height as i32 - height) / 2, bar, height],
                    color: Color::rgb(76, 141, 255),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use brul::gui::{CanvasPainter, Rect, WidgetStyle};

    use super::{PaintCommand, PaintCtx, WavePainter};

    #[test]
    fn bars_stay_inside_the_canvas_rect() {
        let rect = Rect::new(10, 20, 120, 40);
        let style = WidgetStyle::default();
        let commands = WavePainter::new().paint(&PaintCtx {
            rect,
            style: &style,
        });
        assert_eq!(commands.len(), 24);
        for command in commands {
            let PaintCommand::Rect(bar) = command else {
                panic!("expected a rectangle");
            };
            assert!(bar.rect[0] >= rect.x);
            assert!(bar.rect[1] >= rect.y);
            assert!(bar.rect[0] + bar.rect[2] <= rect.x + rect.width as i32);
            assert!(bar.rect[1] + bar.rect[3] <= rect.y + rect.height as i32);
        }
    }
}

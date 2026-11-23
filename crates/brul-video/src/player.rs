use std::cell::{Cell, RefCell, UnsafeCell};
use std::sync::Arc;
use std::time::{Duration, Instant};

use brul_codegen::runtime::{EventSink, StateId, emit_widget_event};
use brul_gui::{ExternalTexture, Revision, TextureFrame};

use crate::decoder::{Decoder, Frame, FrameData};

struct Planes {
    width: u32,
    height: u32,
    data: FrameData,
}

struct Inner {
    decoder: Box<dyn Decoder>,
    playing: bool,
    elapsed: Duration,
    next: Duration,
    last: Instant,
}

pub struct Player {
    state: StateId,
    sink: Option<Arc<dyn EventSink>>,
    revision: Cell<Revision>,
    // NOTE: frame() hands out slices of these planes; advance() is the only writer
    // and runs on the UI thread between draws, never while a frame slice is alive.
    planes: UnsafeCell<Planes>,
    inner: RefCell<Inner>,
}

impl Player {
    pub fn new(
        state: StateId,
        decoder: Box<dyn Decoder>,
        first: Frame,
        sink: Option<Arc<dyn EventSink>>,
    ) -> Self {
        let width = first.width;
        let height = first.height;
        let next = first.duration;
        let player = Self {
            state,
            sink,
            revision: Cell::new(Revision::START),
            planes: UnsafeCell::new(Planes {
                width,
                height,
                data: first.data,
            }),
            inner: RefCell::new(Inner {
                decoder,
                playing: true,
                elapsed: Duration::ZERO,
                next,
                last: Instant::now(),
            }),
        };
        player.revision.set(player.revision.get().next());
        player
    }

    pub fn width(&self) -> u32 {
        unsafe { (*self.planes.get()).width }
    }

    pub fn height(&self) -> u32 {
        unsafe { (*self.planes.get()).height }
    }

    pub fn is_playing(&self) -> bool {
        self.inner.borrow().playing
    }

    pub fn set_playing(&self, playing: bool) {
        let mut inner = self.inner.borrow_mut();
        if inner.playing == playing {
            return;
        }
        inner.playing = playing;
        inner.last = Instant::now();
    }

    fn advance(&self) {
        let mut inner = self.inner.borrow_mut();
        if !inner.playing {
            return;
        }
        let now = Instant::now();
        let last = inner.last;
        inner.last = now;
        inner.elapsed += now.saturating_duration_since(last);
        while inner.elapsed >= inner.next {
            let Some(frame) = inner.decoder.next() else {
                inner.playing = false;
                emit_widget_event(&self.sink, self.state, "ended", Vec::new());
                break;
            };
            let next = frame.duration;
            let due = inner.next;
            inner.elapsed -= due;
            inner.next = next;
            // NOTE: the previous frame slices are not alive here: revision()
            // and frame() both finish advance() before touching the planes.
            unsafe {
                let planes = &mut *self.planes.get();
                planes.width = frame.width;
                planes.height = frame.height;
                planes.data = frame.data;
            }
            self.revision.set(self.revision.get().next());
        }
    }
}

impl ExternalTexture for Player {
    fn revision(&self) -> Revision {
        self.advance();
        self.revision.get()
    }

    fn frame(&self) -> TextureFrame<'_> {
        self.advance();
        let planes = unsafe { &*self.planes.get() };
        match &planes.data {
            FrameData::Rgba(pixels) => TextureFrame::Rgba(pixels),
            FrameData::Yuv420 { y, u, v, strides } => TextureFrame::Yuv420 {
                y,
                u,
                v,
                strides: *strides,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use brul_script::Value;

    use super::*;
    use crate::decoder::Frames;

    #[derive(Default)]
    struct Recorder(Arc<Mutex<Vec<String>>>);

    impl EventSink for Recorder {
        fn widget(&self, _state: StateId, event: &str, _arguments: Vec<Value>) {
            self.0
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(event.to_owned());
        }
    }

    fn frame(pixel: u8) -> Frame {
        Frame::rgba(
            1,
            1,
            vec![pixel, pixel, pixel, 255],
            Duration::from_millis(5),
        )
    }

    #[test]
    fn a_player_advances_its_frame_and_revision() {
        let first = frame(10);
        let decoder = Frames::new(vec![first, frame(20)]).unwrap();
        let player = Player::new(StateId::new(1), Box::new(decoder), frame(10), None);
        let revision = player.revision();
        std::thread::sleep(Duration::from_millis(20));
        assert!(player.revision() > revision);
        let TextureFrame::Rgba(pixels) = player.frame() else {
            panic!("expected rgba frame");
        };
        assert_eq!(pixels[0], 20);
    }

    #[test]
    fn a_finished_player_emits_ended_once() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(Recorder(Arc::clone(&events)));
        let decoder = Frames::once(vec![frame(10)]).unwrap();
        let player = Player::new(StateId::new(2), Box::new(decoder), frame(10), Some(sink));
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(player.revision(), player.revision());
        assert!(!player.is_playing());
        assert_eq!(*events.lock().unwrap(), vec!["ended".to_owned()]);
    }
}

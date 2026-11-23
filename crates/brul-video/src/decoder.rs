use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

use image::AnimationDecoder;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("cannot open video source: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot decode video frame: {0}")]
    Image(#[from] image::ImageError),
    #[error("video source has no frames")]
    Empty,
}

#[derive(Clone, Debug)]
pub enum FrameData {
    Rgba(Vec<u8>),
    Yuv420 {
        y: Vec<u8>,
        u: Vec<u8>,
        v: Vec<u8>,
        strides: [usize; 3],
    },
}

#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub duration: Duration,
    pub data: FrameData,
}

impl Frame {
    pub fn rgba(width: u32, height: u32, pixels: Vec<u8>, duration: Duration) -> Self {
        Self {
            width,
            height,
            duration,
            data: FrameData::Rgba(pixels),
        }
    }
}

pub trait Decoder: Send {
    fn next(&mut self) -> Option<Frame>;
}

#[derive(Default)]
pub struct Frames {
    frames: Vec<Frame>,
    next: usize,
    loops: bool,
}

impl Frames {
    pub fn new(frames: Vec<Frame>) -> Result<Self, DecodeError> {
        if frames.is_empty() {
            return Err(DecodeError::Empty);
        }
        Ok(Self {
            frames,
            next: 0,
            loops: true,
        })
    }

    pub fn once(frames: Vec<Frame>) -> Result<Self, DecodeError> {
        let mut decoder = Self::new(frames)?;
        decoder.loops = false;
        Ok(decoder)
    }
}

impl Frames {
    pub fn first(&self) -> Frame {
        self.frames[0].clone()
    }
}

impl Decoder for Frames {
    fn next(&mut self) -> Option<Frame> {
        if self.next >= self.frames.len() {
            if !self.loops {
                return None;
            }
            self.next = 0;
        }
        let frame = self.frames[self.next].clone();
        self.next += 1;
        Some(frame)
    }
}

pub struct Gif {
    frames: Frames,
}

impl Gif {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DecodeError> {
        let source = BufReader::new(File::open(path)?);
        let decoder = image::codecs::gif::GifDecoder::new(source)?;
        let mut frames = Vec::new();
        for item in decoder.into_frames() {
            let item = item?;
            let (numerator, denominator) = item.delay().numer_denom_ms();
            let duration =
                Duration::from_millis(u64::from(numerator) / u64::from(denominator.max(1)));
            let buffer = item.into_buffer();
            let (width, height) = buffer.dimensions();
            frames.push(Frame::rgba(
                width,
                height,
                buffer.into_raw(),
                duration.max(Duration::from_millis(10)),
            ));
        }
        Ok(Self {
            frames: Frames::new(frames)?,
        })
    }

    pub fn first(&self) -> Frame {
        self.frames.first()
    }
}

impl Decoder for Gif {
    fn next(&mut self) -> Option<Frame> {
        self.frames.next()
    }
}

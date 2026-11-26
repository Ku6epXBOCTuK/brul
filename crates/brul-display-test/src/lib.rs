use std::env;
use std::fs;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use brul_gui::{Dirty, Frame, Position, Size, TextEngine, Widget, WidgetId, WidgetTree};
use image::{ImageFormat, RgbaImage};

const FONT: &[u8] = include_bytes!("../fixtures/font/FiraMono-Medium.ttf");

pub trait DisplayScene {
    fn root() -> Widget;

    fn prepare(_tree: &mut WidgetTree, _engine: &mut TextEngine) {}
}

#[derive(Clone, Debug)]
pub enum Action {
    Focus(WidgetId),
    Hover(i32, i32),
    Press(i32, i32),
    Release(i32, i32),
    Leave,
    Key(String),
    Checked(WidgetId, bool),
    Caret(WidgetId, usize, usize),
    Advance(u64),
    Shot(String),
}

#[derive(Debug)]
pub struct Meta {
    pub size: Size,
    pub theme: Option<String>,
    pub actions: Vec<Action>,
}

impl Meta {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut size = None;
        let mut theme = None;
        let mut actions = Vec::new();
        for (index, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let verb = parts.next().expect("non-empty line");
            let action = match verb {
                "size" => {
                    let value = parts.next().ok_or_else(|| missing(index, verb))?;
                    let (width, height) = value
                        .split_once('x')
                        .ok_or_else(|| format!("line {}: expected WxH, got {value}", index + 1))?;
                    size = Some(Size {
                        width: width.parse().map_err(|_| number(index, width))?,
                        height: height.parse().map_err(|_| number(index, height))?,
                    });
                    continue;
                }
                "theme" => {
                    theme = Some(parts.next().ok_or_else(|| missing(index, verb))?.to_owned());
                    continue;
                }
                "focus" => Action::Focus(id(parts.next(), index)?),
                "hover" => Action::Hover(xy(parts.next(), index)?, xy(parts.next(), index)?),
                "press" => Action::Press(xy(parts.next(), index)?, xy(parts.next(), index)?),
                "release" => Action::Release(xy(parts.next(), index)?, xy(parts.next(), index)?),
                "leave" => Action::Leave,
                "key" => Action::Key(parts.next().ok_or_else(|| missing(index, verb))?.to_owned()),
                "checked" => {
                    let id = id(parts.next(), index)?;
                    let value = parts.next().ok_or_else(|| missing(index, verb))?;
                    let checked = match value {
                        "true" => true,
                        "false" => false,
                        _ => return Err(format!("line {}: expected true or false", index + 1)),
                    };
                    Action::Checked(id, checked)
                }
                "caret" => {
                    let id = id(parts.next(), index)?;
                    let fragment = parts
                        .next()
                        .ok_or_else(|| missing(index, verb))?
                        .parse()
                        .map_err(|_| number(index, "fragment"))?;
                    let offset = parts
                        .next()
                        .ok_or_else(|| missing(index, verb))?
                        .parse()
                        .map_err(|_| number(index, "offset"))?;
                    Action::Caret(id, fragment, offset)
                }
                "advance" => Action::Advance(
                    parts
                        .next()
                        .ok_or_else(|| missing(index, verb))?
                        .parse()
                        .map_err(|_| number(index, "milliseconds"))?,
                ),
                "shot" => {
                    Action::Shot(parts.next().ok_or_else(|| missing(index, verb))?.to_owned())
                }
                other => return Err(format!("line {}: unknown action {other}", index + 1)),
            };
            if parts.next().is_some() {
                return Err(format!("line {}: trailing arguments", index + 1));
            }
            actions.push(action);
        }
        let size = size.ok_or_else(|| "no size line".to_owned())?;
        Ok(Self {
            size,
            theme,
            actions,
        })
    }
}

fn missing(index: usize, verb: &str) -> String {
    format!("line {}: {verb} needs arguments", index + 1)
}

fn number(index: usize, value: &str) -> String {
    format!("line {}: invalid number {value}", index + 1)
}

fn id(value: Option<&str>, index: usize) -> Result<WidgetId, String> {
    let value = value.ok_or_else(|| missing(index, "id"))?;
    value
        .parse()
        .map(WidgetId::new)
        .map_err(|_| number(index, value))
}

fn xy(value: Option<&str>, index: usize) -> Result<i32, String> {
    let value = value.ok_or_else(|| missing(index, "coordinate"))?;
    value.parse().map_err(|_| number(index, value))
}

pub struct Display {
    size: Size,
    engine: TextEngine,
    frame: Frame,
    actual: Vec<u8>,
    labels: Vec<String>,
    first: bool,
}

impl Display {
    pub fn new(size: Size) -> Self {
        let mut db = fontdb::Database::new();
        db.load_font_source(fontdb::Source::Binary(Arc::new(FONT)));
        db.set_sans_serif_family("Fira Mono");
        db.set_serif_family("Fira Mono");
        db.set_monospace_family("Fira Mono");
        let font_system = cosmic_text::FontSystem::new_with_locale_and_db("en".to_owned(), db);
        Self {
            size,
            engine: TextEngine::with_font_system(font_system),
            frame: Frame::new(size.width, size.height),
            actual: Vec::new(),
            labels: Vec::new(),
            first: true,
        }
    }

    pub fn engine_mut(&mut self) -> &mut TextEngine {
        &mut self.engine
    }

    pub fn paint(&mut self, tree: &mut WidgetTree) {
        let dirty = tree.take_dirty();
        let snapshot = tree.snapshot().expect("snapshot");
        if self.first {
            self.frame
                .paint(snapshot, self.engine.atlas(), &Dirty::all());
        } else if !dirty.is_empty() {
            self.frame.paint(snapshot, self.engine.atlas(), &dirty);
        }
        self.first = false;
    }

    pub fn shot(&mut self, label: &str) {
        self.actual.extend_from_slice(self.frame.pixels());
        self.labels.push(label.to_owned());
    }

    pub fn finish(self, name: &str, expected: &[u8], expected_path: &str) {
        let png = png_bytes(&self.size, &self.actual);
        if enabled("BRUL_DISPLAY_BLESS") {
            fs::write(expected_path, &png).expect("bless expected.png");
            return;
        }
        let expected = image::load_from_memory_with_format(expected, ImageFormat::Png)
            .expect("expected png")
            .into_rgba8();
        assert_eq!(expected.width(), self.size.width, "{name}: expected width");
        assert_eq!(
            expected.height() % self.size.height,
            0,
            "{name}: expected height"
        );
        let expected = expected.as_raw().as_slice();
        if self.actual.as_slice() == expected {
            return;
        }
        assert_eq!(
            self.actual.len(),
            expected.len(),
            "{name}: expected {} bytes, got {}",
            expected.len(),
            self.actual.len()
        );
        let frame_len = (self.size.width * self.size.height * 4) as usize;
        assert!(
            frame_len > 0 && self.actual.len().is_multiple_of(frame_len),
            "{name}: broken frame size"
        );
        let at = self
            .actual
            .iter()
            .zip(expected)
            .position(|(actual, expected)| actual != expected)
            .expect("different buffers");
        let frame = at / frame_len;
        let within = at % frame_len;
        let pixel = within / 4;
        let x = pixel % self.size.width as usize;
        let y = pixel / self.size.width as usize;
        let channel = within % 4;
        panic!(
            "{name}: frame {} ({}) pixel ({x},{y}) channel {channel}: expected {}, got {}",
            frame,
            self.labels.get(frame).map(String::as_str).unwrap_or("?"),
            expected[at],
            self.actual[at]
        );
    }
}

pub fn run<S: DisplayScene>(name: &str, meta: &str, expected: &[u8], expected_path: &str) {
    let meta = Meta::parse(meta).unwrap_or_else(|error| panic!("{name}: {error}"));
    let mut display = Display::new(meta.size);
    let mut tree = WidgetTree::new(S::root());
    S::prepare(&mut tree, display.engine_mut());
    tree.layout(display.engine_mut(), meta.size, 1.0)
        .expect("layout");

    for action in &meta.actions {
        match action {
            Action::Shot(label) => {
                display.paint(&mut tree);
                display.shot(label);
            }
            Action::Focus(id) => {
                tree.set_focused(Some(*id));
            }
            Action::Hover(x, y) => {
                tree.pointer_raw_moved(*x, *y).expect("hover");
            }
            Action::Press(x, y) => {
                tree.pointer_raw_down(*x, *y).expect("press");
            }
            Action::Release(x, y) => {
                tree.pointer_raw_up(*x, *y).expect("release");
            }
            Action::Leave => {
                tree.pointer_raw_leave();
            }
            Action::Key(code) => {
                tree.raw_key(code.as_str());
            }
            Action::Checked(id, checked) => {
                tree.set_checked(*id, *checked);
            }
            Action::Caret(id, fragment, offset) => {
                tree.set_caret_position(*id, Position::new(*fragment, *offset));
            }
            Action::Advance(milliseconds) => {
                tree.advance(Duration::from_millis(*milliseconds));
            }
        }
    }

    display.finish(name, expected, expected_path);
}

fn enabled(name: &str) -> bool {
    env::var_os(name).is_some_and(|value| !value.is_empty() && value != "0")
}

fn png_bytes(size: &Size, pixels: &[u8]) -> Vec<u8> {
    let frames = pixels.len() / (size.width * size.height * 4) as usize;
    let image = RgbaImage::from_raw(size.width, size.height * frames as u32, pixels.to_vec())
        .expect("golden image");
    let mut bytes = Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Png)
        .expect("golden png");
    bytes.into_inner()
}

#[cfg(test)]
mod tests {
    use brul_gui::Size;

    use super::{Meta, png_bytes};

    #[test]
    fn meta_rejects_an_unknown_action() {
        let error = Meta::parse("size 8x8\nteleport 1 2\n").expect_err("unknown action");
        assert_eq!(error, "line 2: unknown action teleport");
    }

    #[test]
    fn meta_requires_a_size() {
        let error = Meta::parse("shot base\n").expect_err("missing size");
        assert_eq!(error, "no size line");
    }

    #[test]
    fn meta_keeps_the_theme() {
        let meta = Meta::parse("size 8x8\ntheme light\nshot base\n").expect("meta");
        assert_eq!(meta.theme.as_deref(), Some("light"));
    }

    #[test]
    fn png_round_trips_the_raw_pixels() {
        let size = Size {
            width: 2,
            height: 1,
        };
        let pixels = [1, 2, 3, 4, 5, 6, 7, 8];
        let png = png_bytes(&size, &pixels);
        let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png)
            .expect("decoded png")
            .into_rgba8();
        assert_eq!(decoded.as_raw().as_slice(), pixels.as_slice());
    }
}

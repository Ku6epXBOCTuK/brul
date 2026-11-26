use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use brul_gui::{Dirty, Frame, Position, Size, TextEngine, Widget, WidgetId, WidgetTree};

const FONT: &[u8] = include_bytes!("../fixtures/font/FiraMono-Medium.ttf");

pub trait DisplayScene {
    fn root() -> Widget;

    fn prepare(_tree: &mut WidgetTree, _engine: &mut TextEngine) {}
}

#[derive(Clone, Debug)]
enum Action {
    Focus(WidgetId),
    Hover(i32, i32),
    Press(i32, i32),
    Release(i32, i32),
    Leave,
    Checked(WidgetId, bool),
    Caret(WidgetId, usize, usize),
    Shot(String),
}

#[derive(Debug)]
struct Meta {
    size: Size,
    actions: Vec<Action>,
}

impl Meta {
    fn parse(source: &str) -> Result<Self, String> {
        let mut size = None;
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
                "focus" => Action::Focus(id(parts.next(), index)?),
                "hover" => Action::Hover(xy(parts.next(), index)?, xy(parts.next(), index)?),
                "press" => Action::Press(xy(parts.next(), index)?, xy(parts.next(), index)?),
                "release" => Action::Release(xy(parts.next(), index)?, xy(parts.next(), index)?),
                "leave" => Action::Leave,
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
        Ok(Self { size, actions })
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

fn engine() -> TextEngine {
    TextEngine::with_fonts([fontdb::Source::Binary(Arc::new(FONT))])
}

fn paint(tree: &mut WidgetTree, engine: &mut TextEngine, frame: &mut Frame, first: bool) {
    let dirty = tree.take_dirty();
    let snapshot = tree.snapshot().expect("snapshot");
    if first {
        frame.paint(snapshot, engine.atlas(), &Dirty::all());
    } else if !dirty.is_empty() {
        frame.paint(snapshot, engine.atlas(), &dirty);
    }
}

pub fn run<S: DisplayScene>(name: &str, meta: &str, expected: &[u8], expected_path: &str) {
    let meta = Meta::parse(meta).unwrap_or_else(|error| panic!("{name}: {error}"));
    let mut engine = engine();
    let mut tree = WidgetTree::new(S::root());
    S::prepare(&mut tree, &mut engine);
    tree.layout(&mut engine, meta.size, 1.0).expect("layout");

    let mut frame = Frame::new(meta.size.width, meta.size.height);
    let mut actual = Vec::new();
    let mut shots = Vec::new();
    for action in &meta.actions {
        match action {
            Action::Shot(label) => {
                paint(&mut tree, &mut engine, &mut frame, shots.is_empty());
                actual.extend_from_slice(frame.pixels());
                shots.push(label.as_str());
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
            Action::Checked(id, checked) => {
                tree.set_checked(*id, *checked);
            }
            Action::Caret(id, fragment, offset) => {
                tree.set_caret_position(*id, Position::new(*fragment, *offset));
            }
        }
    }

    dump_png(name, &meta, &actual);
    if enabled("BRUL_DISPLAY_BLESS") {
        fs::write(expected_path, &actual).expect("bless expected.rgba");
        return;
    }
    if actual == expected {
        return;
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "{name}: expected {} bytes, got {}",
        expected.len(),
        actual.len()
    );
    let frame_len = (meta.size.width * meta.size.height * 4) as usize;
    assert!(
        frame_len > 0 && actual.len() % frame_len == 0,
        "{name}: broken frame size"
    );
    let at = actual
        .iter()
        .zip(expected)
        .position(|(actual, expected)| actual != expected)
        .expect("different buffers");
    let frame = at / frame_len;
    let within = at % frame_len;
    let pixel = within / 4;
    let x = pixel % meta.size.width as usize;
    let y = pixel / meta.size.width as usize;
    let channel = within % 4;
    panic!(
        "{name}: frame {} ({}) pixel ({x},{y}) channel {channel}: expected {}, got {}",
        frame,
        shots.get(frame).copied().unwrap_or("?"),
        expected[at],
        actual[at]
    );
}

fn enabled(name: &str) -> bool {
    env::var_os(name).is_some_and(|value| !value.is_empty() && value != "0")
}

fn dump_png(name: &str, meta: &Meta, pixels: &[u8]) {
    if !enabled("BRUL_DISPLAY_PNG") {
        return;
    }
    let width = meta.size.width;
    let frames = pixels.len() / (width * meta.size.height * 4) as usize;
    let image =
        image::RgbaImage::from_raw(width, meta.size.height * frames as u32, pixels.to_vec())
            .expect("preview image");
    let dir = PathBuf::from(
        env::var_os("BRUL_DISPLAY_PNG_DIR").unwrap_or_else(|| "target/display".into()),
    );
    fs::create_dir_all(&dir).expect("preview directory");
    image
        .save(dir.join(format!("{name}.png")))
        .expect("preview png");
}

#[cfg(test)]
mod tests {
    use super::Meta;

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
}

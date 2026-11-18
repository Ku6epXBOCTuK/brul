use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use brul_gui::{Image, Texture, WidgetId, WidgetTree};
use brul_script::{Component, Value};
use tracing::error;

use super::binding::Expression;

#[derive(Default)]
pub struct TextureCache {
    pub(crate) loaded: HashMap<PathBuf, Image>,
    pub(crate) failed: HashSet<PathBuf>,
}

pub type Textures = Rc<RefCell<TextureCache>>;

pub(crate) fn texture(
    component: &RefCell<Component>,
    expression: Expression,
    textures: &Textures,
) -> Option<Image> {
    let value = match expression(&mut component.borrow_mut()) {
        Ok(value) => value,
        Err(error) => {
            error!(%error, "binding expression failed");
            return None;
        }
    };
    match &value {
        Value::Image(path) => load_texture(Path::new(path.as_ref()), textures),
        Value::ImageBytes(bytes) => match Image::from_bytes(bytes) {
            Ok(image) => Some(image),
            Err(error) => {
                error!(%error, "cannot decode embedded image");
                None
            }
        },
        Value::Null => Some(Image::Raster(Rc::new(Texture::empty()))),
        _ => None,
    }
}

fn load_texture(source: &Path, textures: &Textures) -> Option<Image> {
    if textures.borrow().failed.contains(source) {
        return None;
    }
    let key = match fs::canonicalize(source) {
        Ok(key) => key,
        Err(error) => {
            error!(path = %source.display(), %error, "cannot read image");
            textures.borrow_mut().failed.insert(source.to_path_buf());
            return None;
        }
    };
    if let Some(image) = textures.borrow().loaded.get(&key) {
        return Some(image.clone());
    }
    if textures.borrow().failed.contains(&key) {
        return None;
    }
    let bytes = match fs::read(&key) {
        Ok(bytes) => bytes,
        Err(error) => {
            error!(path = %source.display(), %error, "cannot read image");
            textures.borrow_mut().failed.insert(key);
            return None;
        }
    };
    match Image::from_bytes(&bytes) {
        Ok(image) => {
            textures.borrow_mut().loaded.insert(key, image.clone());
            Some(image)
        }
        Err(error) => {
            error!(path = %source.display(), %error, "cannot decode image");
            textures.borrow_mut().failed.insert(key);
            None
        }
    }
}

pub fn set_texture(tree: &mut WidgetTree, widget: WidgetId, value: Option<Image>) {
    if let Some(value) = value {
        tree.set_image(widget, value);
    }
}

pub fn set_text(tree: &mut WidgetTree, widget: WidgetId, value: Option<String>) {
    if let Some(value) = value {
        tree.set_text(widget, value);
    }
}

pub fn set_input(tree: &mut WidgetTree, widget: WidgetId, value: Option<String>) {
    if let Some(value) = value {
        tree.set_input_text(widget, value);
    }
}

pub fn set_flag(
    tree: &mut WidgetTree,
    widget: WidgetId,
    value: Option<bool>,
    set: fn(&mut WidgetTree, WidgetId, bool) -> bool,
) {
    if let Some(value) = value {
        set(tree, widget, value);
    }
}

pub fn set_offset(tree: &mut WidgetTree, widget: WidgetId, x: Option<i32>, y: Option<i32>) {
    if let (Some(x), Some(y)) = (x, y) {
        tree.set_offset(widget, x, y);
    }
}

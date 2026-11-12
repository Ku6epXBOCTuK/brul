use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use resvg::usvg;

use crate::{Color, Texture, TextureError};

const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
const MAX_SIDE: u32 = 4096;

#[derive(Clone)]
pub enum Image {
    Raster(Rc<Texture>),
    Vector(Rc<Vector>),
}

impl Image {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TextureError> {
        if bytes.starts_with(&PNG) {
            return Ok(Self::Raster(Rc::new(Texture::from_png(bytes)?)));
        }
        let source =
            std::str::from_utf8(bytes).map_err(|error| TextureError::Decode(error.to_string()))?;
        Ok(Self::Vector(Rc::new(Vector::from_svg(source)?)))
    }

    pub fn width(&self) -> u32 {
        match self {
            Self::Raster(texture) => texture.width(),
            Self::Vector(vector) => vector.width(),
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Raster(texture) => texture.height(),
            Self::Vector(vector) => vector.height(),
        }
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Raster(left), Self::Raster(right)) => Rc::ptr_eq(left, right),
            (Self::Vector(left), Self::Vector(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}

impl From<Texture> for Image {
    fn from(texture: Texture) -> Self {
        Self::Raster(Rc::new(texture))
    }
}

impl From<Rc<Texture>> for Image {
    fn from(texture: Rc<Texture>) -> Self {
        Self::Raster(texture)
    }
}

pub struct Vector {
    source: Rc<str>,
    width: u32,
    height: u32,
    trees: RefCell<HashMap<u32, Rc<usvg::Tree>>>,
    rasters: RefCell<HashMap<(u32, u32, u32), Rc<Texture>>>,
}

impl Vector {
    pub fn from_svg(source: &str) -> Result<Self, TextureError> {
        let tree = parse(source)?;
        let size = tree.size();

        Ok(Self {
            source: Rc::from(source),
            width: size.width().max(1.0).round() as u32,
            height: size.height().max(1.0).round() as u32,
            trees: RefCell::new(HashMap::new()),
            rasters: RefCell::new(HashMap::new()),
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn raster(&self, width: u32, height: u32, current: Color) -> Option<Rc<Texture>> {
        if width == 0 || height == 0 {
            return None;
        }
        let width = width.min(MAX_SIDE);
        let height = height.min(MAX_SIDE);
        let key = (width, height, pack(current));
        if let Some(texture) = self.rasters.borrow().get(&key) {
            return Some(Rc::clone(texture));
        }

        let tree = self.tree(current)?;
        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
        let transform = resvg::tiny_skia::Transform::from_scale(
            width as f32 / self.width as f32,
            height as f32 / self.height as f32,
        );
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let mut pixels = pixmap.take();
        for pixel in pixels.as_chunks_mut::<4>().0 {
            let alpha = u32::from(pixel[3]);
            if alpha == 0 || alpha == 255 {
                continue;
            }
            for channel in &mut pixel[..3] {
                *channel = (u32::from(*channel) * 255 / alpha).min(255) as u8;
            }
        }

        let texture = Rc::new(Texture::from_rgba(width, height, &pixels).ok()?);
        self.rasters.borrow_mut().insert(key, Rc::clone(&texture));
        Some(texture)
    }

    fn tree(&self, current: Color) -> Option<Rc<usvg::Tree>> {
        let key = pack(current);
        if let Some(tree) = self.trees.borrow().get(&key) {
            return Some(Rc::clone(tree));
        }
        let source = self.source.replace("currentColor", &hex(current));
        let tree = Rc::new(parse(&source).ok()?);
        self.trees.borrow_mut().insert(key, Rc::clone(&tree));
        Some(tree)
    }
}

fn parse(source: &str) -> Result<usvg::Tree, TextureError> {
    usvg::Tree::from_str(source, &usvg::Options::default())
        .map_err(|error| TextureError::Decode(error.to_string()))
}

fn pack(color: Color) -> u32 {
    u32::from(color.r) << 16 | u32::from(color.g) << 8 | u32::from(color.b)
}

fn hex(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::{Image, Vector};

    const ICON: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 6">
        <rect width="12" height="6" fill="currentColor"/>
    </svg>"#;

    #[test]
    fn a_svg_rasterizes_at_the_requested_size_and_records_current_color() {
        let vector = Vector::from_svg(ICON).expect("vector");
        assert_eq!((vector.width(), vector.height()), (12, 6));

        let texture = vector.raster(24, 12, color()).expect("texture");

        assert_eq!((texture.width(), texture.height()), (24, 12));
        assert_eq!(texture.color_at(1, 1).map(|color| color.r), Some(0x12));
    }

    #[test]
    fn a_svg_keeps_one_raster_per_size_and_color() {
        let vector = Vector::from_svg(ICON).expect("vector");
        let first = vector.raster(12, 6, color()).expect("texture");
        let same = vector.raster(12, 6, color()).expect("texture");
        let other = vector
            .raster(12, 6, color().multiply(color()))
            .expect("texture");

        assert!(std::rc::Rc::ptr_eq(&first, &same));
        assert!(!std::rc::Rc::ptr_eq(&first, &other));
    }

    fn color() -> crate::Color {
        crate::Color::rgb(0x12, 0x34, 0x56)
    }

    #[test]
    fn bytes_sniff_between_a_png_and_a_svg() {
        let svg = Image::from_bytes(ICON.as_bytes()).expect("svg");
        assert!(matches!(svg, Image::Vector(_)));
        assert_eq!((svg.width(), svg.height()), (12, 6));
    }
}

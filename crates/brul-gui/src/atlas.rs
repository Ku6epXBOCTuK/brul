use std::collections::HashMap;

use cosmic_text::{CacheKey, SwashContent, SwashImage};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 1024;
const GAP: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AtlasFormat {
    Mask,
    Color,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtlasSlot {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub format: AtlasFormat,
}

#[derive(Debug)]
struct Shelf {
    y: u32,
    height: u32,
    next_x: u32,
}

#[derive(Debug)]
pub struct Atlas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    shelves: Vec<Shelf>,
    entries: HashMap<CacheKey, AtlasSlot>,
}

impl Atlas {
    pub fn new() -> Self {
        Self {
            width: WIDTH,
            height: HEIGHT,
            pixels: vec![0; (WIDTH * HEIGHT * 4) as usize],
            shelves: Vec::new(),
            entries: HashMap::new(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn insert(&mut self, key: CacheKey, image: &SwashImage) -> Option<AtlasSlot> {
        if let Some(slot) = self.entries.get(&key) {
            return Some(*slot);
        }

        let width = image.placement.width;
        let height = image.placement.height;
        let format = match image.content {
            SwashContent::Color => AtlasFormat::Color,
            SwashContent::Mask | SwashContent::SubpixelMask => AtlasFormat::Mask,
        };

        if width == 0 || height == 0 {
            let slot = AtlasSlot {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                format,
            };
            self.entries.insert(key, slot);
            return Some(slot);
        }

        if width > self.width || height > self.height {
            return None;
        }

        let (x, y) = self.allocate(width, height)?;
        self.copy_pixels(x, y, image);
        let slot = AtlasSlot {
            x,
            y,
            width,
            height,
            format,
        };
        self.entries.insert(key, slot);
        Some(slot)
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        for shelf in &mut self.shelves {
            if height <= shelf.height && shelf.next_x + width <= self.width {
                let x = shelf.next_x;
                let y = shelf.y;
                shelf.next_x += width + GAP;
                return Some((x, y));
            }
        }

        let y = match self.shelves.last() {
            Some(shelf) => shelf.y + shelf.height + GAP,
            None => 0,
        };
        if y + height > self.height {
            return None;
        }

        self.shelves.push(Shelf {
            y,
            height,
            next_x: width + GAP,
        });
        Some((0, y))
    }

    fn copy_pixels(&mut self, x: u32, y: u32, image: &SwashImage) {
        let width = image.placement.width;
        let height = image.placement.height;

        for row in 0..height {
            for column in 0..width {
                let source = (row * width + column) as usize;
                let pixel = match image.content {
                    SwashContent::Mask => [255, 255, 255, image.data[source]],
                    SwashContent::Color => [
                        image.data[source * 4],
                        image.data[source * 4 + 1],
                        image.data[source * 4 + 2],
                        image.data[source * 4 + 3],
                    ],
                    SwashContent::SubpixelMask => {
                        let alpha = (u16::from(image.data[source * 4])
                            + u16::from(image.data[source * 4 + 1])
                            + u16::from(image.data[source * 4 + 2]))
                            / 3;
                        [255, 255, 255, alpha as u8]
                    }
                };

                let target = (((y + row) * self.width + x + column) * 4) as usize;
                self.pixels[target..target + 4].copy_from_slice(&pixel);
            }
        }
    }
}

impl Default for Atlas {
    fn default() -> Self {
        Self::new()
    }
}

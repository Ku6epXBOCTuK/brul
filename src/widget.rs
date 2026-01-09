use crate::core::{Color, Rect};
use crate::events::Event;

pub trait Widget {
    fn draw(&self);
    fn handle_event(&mut self, event: &Event);
    fn bounds(&self) -> Rect;
}

pub struct Rectangle {
    pub bounds: Rect,
    pub color: Color,
}

impl Rectangle {
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            color: Color::GRAY,
        }
    }
}

impl Widget for Rectangle {
    fn draw(&self) {
        // TODO: Draw the rectangle
        todo!();
    }

    fn handle_event(&mut self, event: &Event) {
        // TODO: Handle events
        match event {
            Event::MouseDown { x, y, button } => {
                println!("{:?} mouse down at ({}, {})", button, x, y);
            }
            Event::MouseUp { button, x, y } => {
                println!("{:?} mouse up at ({}, {})", button, x, y);
            }
            _ => {}
        }
    }

    fn bounds(&self) -> Rect {
        self.bounds.clone()
    }
}

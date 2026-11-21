use std::ops::{Index, IndexMut};

use crate::Side;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sides<T> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T,
}

impl<T> Sides<T> {
    pub const fn new(left: T, top: T, right: T, bottom: T) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn all(value: T) -> Self
    where
        T: Copy,
    {
        Self::new(value, value, value, value)
    }
}

impl<T> From<T> for Sides<T>
where
    T: Copy,
{
    fn from(value: T) -> Self {
        Self::all(value)
    }
}

impl<T> Index<Side> for Sides<T> {
    type Output = T;

    fn index(&self, side: Side) -> &Self::Output {
        match side {
            Side::Left => &self.left,
            Side::Top => &self.top,
            Side::Right => &self.right,
            Side::Bottom => &self.bottom,
        }
    }
}

impl<T> IndexMut<Side> for Sides<T> {
    fn index_mut(&mut self, side: Side) -> &mut Self::Output {
        match side {
            Side::Left => &mut self.left,
            Side::Top => &mut self.top,
            Side::Right => &mut self.right,
            Side::Bottom => &mut self.bottom,
        }
    }
}

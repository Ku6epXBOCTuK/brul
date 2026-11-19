use std::any::Any;
use std::rc::Rc;

#[derive(Clone)]
pub enum Scope {
    Root,
    Each(Rc<dyn Any>),
}

impl Scope {
    pub fn root() -> Self {
        Self::Root
    }

    pub fn each<T: 'static>(data: T) -> Self {
        Self::Each(Rc::new(data))
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        match self {
            Self::Each(data) => data.downcast_ref::<T>(),
            Self::Root => None,
        }
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::root()
    }
}

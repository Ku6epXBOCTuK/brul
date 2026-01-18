use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::Deref,
    pin::Pin,
    sync::Mutex,
};

pub struct State<'s, T: Send + Sync + 'static>(&'s T);

impl<'s, T: Send + Sync + 'static> State<'s, T> {
    pub fn inner(&self) -> &T {
        self.0
    }
}

impl<T: Send + Sync + 'static> Deref for State<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0
    }
}

impl<T: Send + Sync + 'static> Clone for State<'_, T> {
    fn clone(&self) -> Self {
        State(self.0)
    }
}

impl<T: Send + Sync + 'static + PartialEq> PartialEq for State<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Send + Sync + std::fmt::Debug> std::fmt::Debug for State<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("State").field(&self.0).finish()
    }
}

#[derive(Debug)]
pub struct StateManager {
    map: Mutex<HashMap<TypeId, Pin<Box<dyn Send + Sync>>>>,
}

impl StateManager {
    pub(crate) fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn set<T: Send + Sync + 'static>(&self, state: T) -> bool {
        let mut map = self.map.lock().unwrap();

        if map.contains_key(&TypeId::of::<T>()) {
            return false;
        }

        let box_ptr = Box::pin(state) as Pin<Box<dyn Send + Sync>>;

        map.insert(TypeId::of::<T>(), box_ptr);

        true
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> State<'_, T> {
        self.try_get()
            .unwrap_or_else(|| panic!("State not found for type {}", std::any::type_name::<T>()))
    }

    pub fn try_get<T: Send + Sync + 'static>(&self) -> Option<State<'_, T>> {
        let map = self.map.lock().unwrap();

        let type_id = TypeId::of::<T>();

        let box_ptr = map.get(&type_id)?;

        let v_ref = unsafe {
            // SAFETY: We know that the type_id is the same as the type of T, so we can safely downcast it.
            let value = box_ptr.downcast_ref::<T>().unwrap_unchecked();
            // SAFETY: We know that the value is a valid pointer to T, so we can safely create a State from it.
            &*(value as *const T)
        };

        Some(State(v_ref))
    }
}

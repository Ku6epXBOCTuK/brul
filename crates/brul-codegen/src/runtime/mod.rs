mod binding;
mod build;
mod component;
mod error;
mod event;
mod paint;
mod slot;
mod structure;

#[cfg(test)]
mod tests;

pub use binding::{
    Binding, BindingTarget, Bindings, ClassTable, Expression, PropBinding, Write, boolean,
    evaluate, offset, text, write,
};
pub use build::Build;
pub use component::{
    ComponentHandle, Context, Instance, PropArgument, accepts_prop, instance, prop_name,
};
pub use error::ViewError;
pub use event::{ComponentEvent, Handler, HandlerEvent, PendingEvent, take_event};
pub use paint::{TextureCache, Textures, set_flag, set_input, set_offset, set_text, set_texture};
pub use slot::{DEFAULT_SLOT, SlotArgument, Slots, slot};
pub use structure::{
    Built, Rebuilt, RowInfo, RowKey, Structure, StructureRebuild, StructureValue, View,
    register_structure,
};

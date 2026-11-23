mod binding;
mod build;
mod capability;
mod component;
mod error;
mod event;
mod event_sink;
mod hooks;
mod paint;
mod slot;
mod state;
mod structure;

#[cfg(test)]
mod tests;

pub use binding::{
    Binding, BindingTarget, Bindings, ClassTable, Expression, FlagExpression, NumberExpression,
    PropBinding, PropExpression, TextExpression, Write, boolean, evaluate, offset, text, write,
};
pub use brul_event::{Event, EventKind, EventSet, RawEvent, RawEventSet, RawKind};
pub use brul_gui::{DispatchEvent, EventTarget, RawUiEvent, UiEvent};
pub use build::Build;
pub use capability::Capability;
pub use component::{ComponentHandle, Context, Instance, PropArgument, accepts_prop, instance};
pub use error::ViewError;
pub use event::{ComponentEvent, Handler, Pending, PendingEvent, push_ui_event, take_pending};
pub use event_sink::{EventSink, emit as emit_widget_event};
pub use hooks::{
    CleanupHook, Dispatch, Element, EventHook, Hooks, Param, apply as apply_capability,
};
pub use paint::{TextureCache, Textures, set_flag, set_input, set_offset, set_text, set_texture};
pub use slot::{DEFAULT_SLOT, SlotArgument, Slots, slot};
pub use state::{SharedStates, StateId, States};
pub use structure::{
    Built, Rebuilt, RowInfo, RowKey, Structure, StructureRebuild, StructureValue, View,
    register_structure,
};

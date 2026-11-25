//! Builtin `Canvas` view for the `.brul` markup.

use crate::ast::Position;
use crate::gui::{CanvasSpec, Widget};
use crate::{Build, Built, ComponentHandle, ViewError};

pub fn view(
    _theme: Option<&str>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let animated = component
        .borrow_mut()
        .seed::<bool>("animated", Position::default())?;
    Ok(Built {
        widget: Widget::canvas(CanvasSpec::new().animated(animated)),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

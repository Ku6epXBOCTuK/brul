use brul_codegen::runtime::{Build, Built, ComponentHandle, ViewError};
use brul_gui::{PlatformViewId, Widget};
use brul_syntax::ast::Position;

const POSITION: Position = Position { line: 0, column: 0 };

pub fn view(
    _theme: Option<&str>,
    _build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let state = component.borrow_mut().seed::<usize>("state", POSITION)?;
    let source = component.borrow_mut().seed::<String>("source", POSITION)?;
    let visible = component.borrow_mut().seed::<bool>("visible", POSITION)?;
    Ok(Built {
        widget: Widget::platform_view(PlatformViewId::new(state as u64), source).visible(visible),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

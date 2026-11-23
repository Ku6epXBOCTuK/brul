use brul_codegen::runtime::{Build, Built, ComponentHandle, ViewError};
use brul_gui::{PlatformViewId, Widget};
use brul_syntax::ast::Position;

const POSITION: Position = Position { line: 0, column: 0 };

pub fn view(
    _theme: Option<&str>,
    build: &mut Build,
    component: &ComponentHandle,
) -> Result<Built, ViewError> {
    let source = component.borrow_mut().seed::<String>("source", POSITION)?;
    let visible = component.borrow_mut().seed::<bool>("visible", POSITION)?;
    Ok(Built {
        widget: Widget::platform_view(PlatformViewId::new(build.state_id().get()), source)
            .visible(visible),
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}

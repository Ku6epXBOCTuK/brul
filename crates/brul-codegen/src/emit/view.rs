use std::fmt::Write;

use brul_syntax::error::Error;
use brul_syntax::markup::Element;

use crate::runtime::HandlerEvent;

use super::Emit;
use super::Frame;
use super::element::widget;

pub(super) fn view(
    out: &mut String,
    name: &str,
    root: &Element,
    handlers: &[(HandlerEvent, String)],
    emit: &mut Emit<'_>,
) -> Result<(), Error> {
    let _ = writeln!(
        out,
        "pub fn {name}(theme: Option<&str>, component: &brul::ComponentHandle) -> Result<brul::Built, String> {{"
    );
    out.push_str("    component.borrow_mut().set_names(names());\n");
    let _ = writeln!(
        out,
        "    brul::gui::style::check_theme({}styles(), theme).map_err(|error| error.to_string())?;",
        emit.resource_prefix
    );
    out.push_str("    let mut bindings = Vec::new();\n");
    out.push_str("    let mut handlers = Vec::new();\n");
    out.push_str("    let mut props = Vec::new();\n");
    out.push_str("    let widget = {\n");
    let mut counter = 0;
    widget(
        out,
        root,
        emit,
        Frame::root(emit.default_theme(), 2),
        &mut counter,
    )?;
    out.push_str("\n    };\n");
    for (event, handler) in handlers {
        let _ = writeln!(
            out,
            "    handlers.push(brul::Handler::new(component, None, brul::HandlerEvent::{event:?}, {handler:?}));"
        );
    }
    out.push_str("    Ok(brul::Built {\n");
    out.push_str("        widget,\n");
    out.push_str("        bindings,\n");
    out.push_str("        handlers,\n");
    out.push_str("        props,\n");
    out.push_str("    })\n}\n");
    Ok(())
}

use brul_syntax::error::Error;
use brul_syntax::markup::Element;

use crate::runtime::HandlerEvent;

use super::Emit;
use super::Frame;
use super::Out;
use super::element::widget;

pub(super) fn view(
    out: &mut String,
    name: &str,
    root: &Element,
    handlers: &[(HandlerEvent, String)],
    emit: &mut Emit<'_>,
    typed: bool,
) -> Result<(), Error> {
    let theme = match typed {
        true => "Option<ThemeId>",
        false => "Option<&str>",
    };
    let frame = Frame::root(emit.default_theme());
    let mut out = Out::new(out);
    out.line(format_args!(
        "pub fn {name}(theme: {theme}, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {{"
    ));
    out.indented(|out| -> Result<(), Error> {
        if typed {
            out.line(format_args!("let theme = theme.map(brul::Theme::name);"));
        }
        out.line(format_args!("component.borrow_mut().set_names(names());"));
        out.line(format_args!("let mut bindings = Vec::new();"));
        out.line(format_args!("let mut handlers = Vec::new();"));
        out.line(format_args!("let mut props = Vec::new();"));
        out.line(format_args!("let widget = {{"));
        let mut counter = 0;
        out.indented(|out| widget(out, root, emit, frame, &mut counter))?;
        out.end();
        out.line(format_args!("}};"));
        for (event, handler) in handlers {
            out.line(format_args!(
                "handlers.push(brul::Handler::new(component, None, brul::HandlerEvent::{event:?}, {handler:?}));"
            ));
        }
        out.line(format_args!("Ok(brul::Built {{"));
        out.indented(|out| {
            out.line(format_args!("widget,"));
            out.line(format_args!("bindings,"));
            out.line(format_args!("handlers,"));
            out.line(format_args!("props,"));
        });
        out.line(format_args!("}})"));
        Ok(())
    })?;
    out.line(format_args!("}}"));
    Ok(())
}

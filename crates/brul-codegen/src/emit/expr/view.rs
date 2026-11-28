use brul_syntax::error::Error;
use brul_syntax::markup::{Block, BlockItem, Element, Member};

use crate::window::AttachArg;

use super::Emit;
use super::Frame;
use super::Out;
use super::StateKey;
use super::element::{HooksSpec, hooks_binding, items_spliced, widget};
use super::events::HandlerSpec;
use super::module::ViewKind;

pub(super) enum ViewRoot<'a> {
    Element(&'a Element),
    Members(Vec<&'a Member>),
}

pub(super) struct ViewSpec<'a> {
    pub(super) name: &'a str,
    pub(super) source: &'a str,
    pub(super) root: ViewRoot<'a>,
    pub(super) handlers: &'a [HandlerSpec],
    pub(super) attaches: &'a [AttachArg],
    pub(super) attach_states: &'a [StateKey],
    pub(super) kind: ViewKind,
}

pub(super) fn view(
    out: &mut String,
    spec: &ViewSpec<'_>,
    emit: &mut Emit<'_>,
) -> Result<(), Error> {
    let ViewSpec {
        name,
        source,
        root,
        handlers,
        attaches,
        attach_states,
        kind,
    } = spec;
    let theme = match kind.typed() {
        true => "Option<ThemeId>",
        false => "Option<&str>",
    };
    let frame = Frame::root(emit.default_theme());
    let mut out = Out::new(out);
    out.line(format_args!(
        "pub fn {name}(theme: {theme}, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {{"
    ));
    out.indented(|out| -> Result<(), Error> {
        if kind.typed() {
            out.line(format_args!("let theme = theme.map(brul::Theme::name);"));
        }
        out.line(format_args!("component.borrow_mut().set_names(names());"));
        out.line(format_args!("let mut bindings = Vec::new();"));
        out.line(format_args!("let mut handlers = Vec::new();"));
        out.line(format_args!("let mut props = Vec::new();"));
        for (attach, state) in attaches.iter().zip(*attach_states) {
            hooks_binding(
                out,
                HooksSpec {
                    name: &attach.name,
                    params: &attach.params,
                    position: attach.position,
                    target: "brul::EventTarget::Window",
                    id: state,
                    context: emit.context,
                    scopes: &[],
                    scope: None,
                },
            )?;
        }
        out.line(format_args!("let widget = {{"));
        let mut counter = 0;
        match root {
            ViewRoot::Element(element) => {
                out.indented(|out| widget(out, element, emit, frame, &mut counter))?;
            }
            ViewRoot::Members(members) => {
                let items: Vec<BlockItem> = members
                    .iter()
                    .filter_map(|member| match member {
                        Member::Element(element) => Some(BlockItem::Element((*element).clone())),
                        Member::Slot(slot) => Some(BlockItem::Slot((*slot).clone())),
                        Member::Prop(_) | Member::Event(_) => None,
                    })
                    .collect();
                let block = Block { items };
                out.indented(|out| -> Result<(), Error> {
                    out.line(format_args!("let mut children = Vec::new();"));
                    items_spliced(out, &block, emit, frame, &mut counter)?;
                    if !block.items.is_empty() {
                        out.end();
                    }
                    out.line(format_args!("brul::gui::Widget::structure(children)"));
                    Ok(())
                })?;
            }
        }
        out.push(format_args!(".debug_source({source:?})"));
        out.end();
        out.line(format_args!("}};"));
        for handler in *handlers {
            out.line(format_args!(
                "handlers.push(brul::Handler::new(component, None, brul::EventKind::{:?}, {}));",
                handler.event,
                super::script::event_call(emit.context.script_name, &handler.function)
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

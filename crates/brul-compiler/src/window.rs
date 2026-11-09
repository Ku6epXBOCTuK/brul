use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use brul_codegen::runtime::{
    Binding, ComponentHandle, Context, Handler, HandlerEvent, PropBinding, Structure,
};
use brul_codegen::{Registry, error_at};
use brul_gui::Widget;
use brul_gui::window::WindowConfig;
use brul_script::{Component, Value, execute_program};
use brul_syntax::error::Error;
use brul_syntax::markup::{Element, Window};
use brul_syntax::script::Program;

use crate::build_bound_tree;
use crate::tree::script_error;

pub struct Loaded {
    pub config: WindowConfig,
    pub tree: Widget,
    pub structures: Vec<Structure>,
    pub bindings: Vec<Binding>,
    pub props: Vec<PropBinding>,
    pub handlers: Vec<Handler>,
    pub component: ComponentHandle,
    pub uses: Vec<brul_syntax::uses::Use>,
    pub registry: Rc<Registry>,
    pub context: Context,
    pub program: Option<Program>,
    pub root: Element,
    pub theme: Option<String>,
}

pub fn load(source: &str, theme: Option<&str>) -> Result<Loaded, Error> {
    load_with(source, theme, &[])
}

pub fn load_with(
    source: &str,
    theme: Option<&str>,
    seeds: &[(&str, Value)],
) -> Result<Loaded, Error> {
    load_from(source, None, theme, seeds)
}

pub fn load_file(path: impl AsRef<Path>, theme: Option<&str>) -> Result<Loaded, Error> {
    load_file_with(path, theme, &[])
}

pub fn load_file_with(
    path: impl AsRef<Path>,
    theme: Option<&str>,
    seeds: &[(&str, Value)],
) -> Result<Loaded, Error> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|error| Error {
        message: format!("cannot read '{}': {error}", path.display()),
        line: 1,
        column: 1,
    })?;
    load_from(&source, path.parent(), theme, seeds)
}

fn load_from(
    source: &str,
    base: Option<&Path>,
    theme: Option<&str>,
    seeds: &[(&str, Value)],
) -> Result<Loaded, Error> {
    let parsed = brul_codegen::parse_source(source)?;
    let stylesheet = Rc::new(parsed.stylesheet);
    let uses = parsed.uses;
    let registry = Rc::new(brul_codegen::resolve(&uses, base)?);
    let context = Context {
        registry: Rc::clone(&registry),
        imports: registry.imports.clone(),
        stylesheet: Rc::clone(&stylesheet),
    };
    let program = parsed.program;

    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    for (name, value) in seeds {
        component
            .borrow_mut()
            .declare(name, value.clone(), true)
            .map_err(script_error)?;
    }
    if let Some(program) = &program {
        execute_program(&mut component.borrow_mut(), program).map_err(script_error)?;
    }

    let root = brul_codegen::root(&parsed.window)?.to_owned();
    let (config, window_handlers) = arguments(&parsed.window, &component)?;
    let mut built = build_bound_tree(&root, &component, &context, theme)?;
    built.handlers.extend(window_handlers);

    Ok(Loaded {
        config,
        tree: built.widget,
        structures: built.structures,
        bindings: built.bindings,
        props: built.props,
        handlers: built.handlers,
        component,
        uses,
        registry,
        context,
        program,
        root,
        theme: theme.map(str::to_owned),
    })
}

fn arguments(
    window: &Window,
    component: &ComponentHandle,
) -> Result<(WindowConfig, Vec<Handler>), Error> {
    let (config, raw) = brul_codegen::arguments(window)?;
    let mut handlers = Vec::new();
    for item in raw {
        let Some(parsed) = HandlerEvent::parse(&item.event) else {
            return Err(error_at(
                item.position,
                format!("event '{}' is not supported yet", item.event),
            ));
        };
        if parsed != HandlerEvent::Key {
            return Err(error_at(
                item.position,
                format!("window event '{}' is not supported yet", parsed.name()),
            ));
        }
        if handlers.iter().any(|other: &Handler| other.event == parsed) {
            return Err(error_at(
                item.position,
                format!("event '{}' is handled twice", parsed.name()),
            ));
        }
        if !component.borrow().has_function(&item.function) {
            return Err(error_at(
                item.position,
                format!("function '{}' is not defined", item.function),
            ));
        }
        handlers.push(Handler {
            widget: None,
            event: parsed,
            function: item.function,
            component: Rc::clone(component),
        });
    }
    Ok((config, handlers))
}

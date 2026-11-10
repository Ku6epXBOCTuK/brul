use std::cell::RefCell;
use std::rc::Rc;

use brul_window::{ColorScheme, run_tree_themed};

use crate::{
    Built, Component, ComponentHandle, Host, Ui, Value, WindowConfig, execute_program, script,
};
/// Запускает окно по модулю, сгенерированному из `.brul`.
pub fn run_generated(
    config: WindowConfig,
    script: Option<script::Program>,
    view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
    theme: Option<&str>,
    seeds: &[(&str, Value)],
    host: Option<Box<dyn Host>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (component, built) = generated_built(view, theme, seeds, script.as_ref())?;
    let ui = Ui::assemble(component, built, view, theme.map(str::to_owned), host);
    run_ui(config, theme.map(str::to_owned), ui)
}

pub(crate) fn generated_built(
    view: fn(Option<&str>, &ComponentHandle) -> Result<Built, String>,
    theme: Option<&str>,
    seeds: &[(&str, Value)],
    script: Option<&script::Program>,
) -> Result<(ComponentHandle, Built), Box<dyn std::error::Error>> {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    for (name, value) in seeds {
        component.borrow_mut().declare(name, value.clone(), true)?;
    }
    if let Some(program) = script {
        execute_program(&mut component.borrow_mut(), program)?;
    }
    let built = view(theme, &component).map_err(std::io::Error::other)?;
    Ok((component, built))
}

fn run_ui(
    config: WindowConfig,
    theme: Option<String>,
    ui: Ui,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = Rc::new(RefCell::new(ui));
    let initial = ui.borrow_mut().dispatch(&[]);
    let builder = Rc::clone(&ui);
    let handler = Rc::clone(&ui);
    run_tree_themed(
        config,
        initial,
        move |scheme| {
            let mut ui = builder.borrow_mut();
            if let Err(error) = ui.set_theme(scheme_theme(theme.as_deref(), scheme))
                && !error.message.contains("unknown theme")
            {
                eprintln!("{error}");
            }
            Rc::clone(&ui.tree)
        },
        move |events| {
            let commands = handler.borrow_mut().dispatch(events);
            brul_reactive::flush();
            handler.borrow_mut().settle();
            commands
        },
    )?;
    Ok(())
}

pub(crate) fn scheme_theme(fixed: Option<&str>, scheme: ColorScheme) -> &str {
    fixed.unwrap_or(match scheme {
        ColorScheme::Light => "light",
        ColorScheme::Dark => "dark",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use brul_gui::{Widget, WidgetId};
    use brul_syntax::ast::{Expr, ExprKind, Position};
    use brul_window::ColorScheme;

    use crate::ui::Ui;
    use crate::{Binding, BindingTarget, Built, ComponentHandle, Value};

    fn generated_view(theme: Option<&str>, component: &ComponentHandle) -> Result<Built, String> {
        if theme.is_some() {
            return Err("unknown theme".to_owned());
        }
        let widget = Widget::text("");
        let id = widget.id();
        Ok(Built {
            widget,
            bindings: vec![Binding::new(
                component,
                id,
                BindingTarget::Text(Expr::new(
                    ExprKind::Ident("name".to_owned()),
                    Position::default(),
                )),
            )],
            handlers: Vec::new(),
            structures: Vec::new(),
            props: Vec::new(),
        })
    }

    fn assembled(
        theme: Option<&str>,
        seeds: &[(&str, Value)],
        script: Option<&script::Program>,
    ) -> (WidgetId, Ui) {
        let (component, built) =
            generated_built(generated_view, theme, seeds, script).expect("launch succeeds");
        let id = built.widget.id();
        (
            id,
            Ui::assemble(component, built, generated_view, None, None),
        )
    }

    #[test]
    fn generated_launch_declares_seeds() {
        let seeds = [("name", Value::String("from seed".to_owned()))];
        let (id, ui) = assembled(None, &seeds, None);
        brul_reactive::flush();
        assert!(!ui.tree.borrow_mut().set_text(id, "from seed".to_owned()));
    }

    #[test]
    fn generated_launch_runs_the_script() {
        let program = script::Program {
            functions: Vec::new(),
            statements: vec![script::Statement::Let {
                constant: false,
                name: "name".to_owned(),
                value: Some(Expr::new(
                    ExprKind::String("from script".to_owned()),
                    Position::default(),
                )),
            }],
        };
        let (id, ui) = assembled(None, &[], Some(&program));
        brul_reactive::flush();
        assert!(!ui.tree.borrow_mut().set_text(id, "from script".to_owned()));
    }

    #[test]
    fn explicit_theme_ignores_the_color_scheme() {
        assert_eq!(scheme_theme(Some("dark"), ColorScheme::Light), "dark");
        assert_eq!(scheme_theme(Some("light"), ColorScheme::Dark), "light");
    }

    #[test]
    fn system_theme_follows_the_color_scheme() {
        assert_eq!(scheme_theme(None, ColorScheme::Light), "light");
        assert_eq!(scheme_theme(None, ColorScheme::Dark), "dark");
    }
}

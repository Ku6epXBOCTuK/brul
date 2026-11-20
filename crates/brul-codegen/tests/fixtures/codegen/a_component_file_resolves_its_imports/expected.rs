use super::*;
#[allow(unused_mut, unused_variables, clippy::let_and_return)]
pub fn input_view(theme: Option<&str>, build: &mut brul::Build, component: &brul::ComponentHandle) -> Result<brul::Built, brul::ViewError> {
    component.borrow_mut().set_names(names());
    let mut bindings = Vec::new();
    let mut handlers = Vec::new();
    let mut props = Vec::new();
    let widget = {
        {
            let built = brul::instance(build, component, brul::Instance {
                theme,
                view: label_view,
                script: label_script(),
                name: "Label",
                names: names(),
                props: &[
                ],
                defaults: &[
                ],
                events: &[
                ],
                slots: &[],
            })?;
            bindings.extend(built.bindings);
            handlers.extend(built.handlers);
            props.extend(built.props);
            let widget0 = built.widget;
            widget0
        }
    };
    Ok(brul::Built {
        widget,
        bindings,
        handlers,
        props,
    })
}

#[allow(dead_code)]
static INPUT_CLASSES_NONE: std::sync::LazyLock<[brul::gui::Class; 0]> = std::sync::LazyLock::new(|| [
]);

fn input_classes(_theme: Option<&str>) -> &'static [brul::gui::Class] {
    &*INPUT_CLASSES_NONE
}

#[allow(dead_code)]
fn input_styles() -> &'static brul::style::Stylesheet {
    use brul::style as s;

    static STYLES: std::sync::LazyLock<s::Stylesheet> = std::sync::LazyLock::new(|| s::Stylesheet {
        themes: vec![],
        rules: vec![],
    });
    &STYLES
}

fn input_styled(
    mut widget: brul::gui::Widget,
    indices: &[usize],
    theme: Option<&str>,
) -> brul::gui::Widget {
    let table = input_classes(theme);
    for index in indices {
        widget = widget.with_class(&table[*index]);
    }
    widget
}

pub fn input_script() -> Option<brul::Script> {
    None
}

use brul_gui::Class;
use brul_syntax::style::Stylesheet;

use crate::runtime::ViewError;

pub use brul_gui::style::Resolved;

pub fn compose(user: &Stylesheet, builtin: &Stylesheet) -> Stylesheet {
    let mut composed = builtin.clone();
    composed.rules.extend(user.rules.iter().cloned());
    composed
}

pub fn resolve(
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<Resolved, ViewError> {
    Ok(brul_gui::style::resolve(stylesheet, classes, theme)?)
}

pub fn class(stylesheet: &Stylesheet, name: &str, theme: Option<&str>) -> Result<Class, ViewError> {
    Ok(brul_gui::style::class(stylesheet, name, theme)?)
}

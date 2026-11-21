use brul_style::{Kind, Snapshot, SnapshotError, State, Stylesheet};

use crate::widget::{Class, Layers, StyleSet, StyleState, TextStylePatch, WidgetStylePatch};
use crate::{LayoutPatch, LayoutStyle, TextStyleSet};

mod snapshot;
mod value;

use snapshot::{base_styles, state_patch, transitions};

pub struct Resolved {
    pub layout: LayoutStyle,
    pub styles: StyleSet,
    pub text: TextStyleSet,
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum StyleError {
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error("expected {} for '{}'", .expected, .kind.name())]
    Value { kind: Kind, expected: &'static str },
}

pub fn check_theme(stylesheet: &Stylesheet, theme: Option<&str>) -> Result<(), StyleError> {
    match theme {
        Some(name) if !stylesheet.has_theme(name) => {
            Err(SnapshotError::UnknownTheme(name.to_owned()).into())
        }
        _ => Ok(()),
    }
}

pub fn styled(
    mut widget: crate::Widget,
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<crate::Widget, StyleError> {
    for name in classes {
        widget = widget.with_class(&class(stylesheet, name, theme)?);
    }
    Ok(widget)
}

pub fn resolve(
    stylesheet: &Stylesheet,
    classes: &[&str],
    theme: Option<&str>,
) -> Result<Resolved, StyleError> {
    let base = snapshot(stylesheet, classes, None, theme)?;
    let (layout, base_style, base_text) = base_styles(&base)?;
    let mut styles = StyleSet::new(base_style).with_transitions(transitions(&base)?);
    let mut text = TextStyleSet::new(base_text.clone());
    for &state in StyleState::ALL {
        let stated = stylesheet.stated(classes, syntax_state(state), theme)?;
        let (patch, text_patch) = state_patch(&stated)?;
        if patch != WidgetStylePatch::default() {
            styles = styles.with(state, patch);
        }
        if text_patch != TextStylePatch::default() {
            text = text.with(state, text_patch);
        }
    }
    Ok(Resolved {
        layout,
        styles,
        text,
    })
}

pub fn class(
    stylesheet: &Stylesheet,
    name: &str,
    theme: Option<&str>,
) -> Result<Class, StyleError> {
    let base = snapshot(stylesheet, &[name], None, theme)?;
    let (layout, _, _) = base_styles(&base)?;
    let (style_patch, text_patch) = state_patch(&base)?;
    let mut style = Layers::new(style_patch);
    let mut text = Layers::new(text_patch);
    for &state in StyleState::ALL {
        let stated = stylesheet.stated(&[name], syntax_state(state), theme)?;
        let (style_patch, text_patch) = state_patch(&stated)?;
        if style_patch != WidgetStylePatch::default() {
            style = style.with(state, style_patch);
        }
        if text_patch != TextStylePatch::default() {
            text = text.with(state, text_patch);
        }
    }
    Ok(Class {
        layout: LayoutPatch::from(&layout),
        style,
        text,
        transitions: transitions(&base)?,
    })
}

fn snapshot(
    stylesheet: &Stylesheet,
    classes: &[&str],
    state: Option<State>,
    theme: Option<&str>,
) -> Result<Snapshot, StyleError> {
    Ok(stylesheet.snapshot(classes, state, theme)?)
}

macro_rules! syntax_states {
    ($(($syntax:ident, $gui:ident, $field:ident, $keyword:literal)),+ $(,)?) => {
        fn syntax_state(state: StyleState) -> State {
            match state {
                $(StyleState::$gui => State::$syntax,)+
            }
        }
    };
}

brul_style::states!(syntax_states);

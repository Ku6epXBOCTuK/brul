use std::time::Duration;

use super::WidgetState;
use crate::animation::{Track, Tween};
use crate::gradient::Paint;
use crate::layout::LayoutPatch;
use crate::rect::Shadow;
use crate::text::{TextStylePatch, VerticalAlign};
use crate::{BorderColors, Color};

macro_rules! resolve_layers {
    ($(($syntax:ident, $gui:ident, $field:ident, $keyword:literal)),+ $(,)?) => {
        pub fn resolve(&self, state: WidgetState) -> WidgetStyle {
            let mut style = self.base;
            let layers = [$( (state.$field, self.$field), )+];
            for (active, patch) in layers {
                if let (true, Some(patch)) = (active, patch) {
                    style.apply(&patch);
                }
            }
            style
        }
    };
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Relayout {
    geometry: LayoutGeometry,
    clip: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct LayoutGeometry {
    border_radius: u32,
    scroll: bool,
    shadow_x: i32,
    shadow_y: i32,
    shadow_blur: u32,
}

impl Relayout {
    pub(crate) fn same_layout(&self, other: &Self) -> bool {
        self.geometry == other.geometry
    }
}

include!(concat!(env!("OUT_DIR"), "/style_widget.rs"));

impl WidgetStyle {
    pub(crate) fn relayout(&self) -> Relayout {
        Relayout {
            geometry: LayoutGeometry {
                border_radius: self.border_radius,
                scroll: self.scroll,
                shadow_x: self.shadow.x,
                shadow_y: self.shadow.y,
                shadow_blur: self.shadow.blur,
            },
            clip: self.clip,
        }
    }
}

macro_rules! define_layers {
    ($(($syntax:ident, $gui:ident, $field:ident, $keyword:literal)),+ $(,)?) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct Layers<P> {
            base: P,
            $($field: Option<P>,)+
        }

        impl<P> Layers<P> {
            pub const fn new(base: P) -> Self {
                Self {
                    base,
                    $($field: None,)+
                }
            }

            pub fn with(mut self, state: StyleState, patch: P) -> Self {
                *self.slot_mut(state) = Some(patch);
                self
            }

            pub fn base(&self) -> &P {
                &self.base
            }

            pub fn patch(&self, state: StyleState) -> Option<&P> {
                self.slot(state).as_ref()
            }

            const fn slot(&self, state: StyleState) -> &Option<P> {
                match state {
                    $(StyleState::$gui => &self.$field,)+
                }
            }

            const fn slot_mut(&mut self, state: StyleState) -> &mut Option<P> {
                match state {
                    $(StyleState::$gui => &mut self.$field,)+
                }
            }
        }

        impl<P: Default + PartialEq> Layers<P> {
            pub fn is_empty(&self) -> bool {
                self.base == P::default()
                    && StyleState::ALL
                        .iter()
                        .all(|state| self.slot(*state).is_none())
            }
        }
    };
}

brul_style::states!(define_layers);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Class {
    pub layout: LayoutPatch,
    pub style: Layers<WidgetStylePatch>,
    pub text: Layers<TextStylePatch>,
    pub transitions: Transitions,
}

impl Class {
    pub fn is_empty(&self) -> bool {
        self.layout.is_empty()
            && self.style.is_empty()
            && self.text.is_empty()
            && self.transitions.is_empty()
    }
}

macro_rules! define_style_state {
    ($(($syntax:ident, $gui:ident, $field:ident, $keyword:literal)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum StyleState {
            $($gui,)+
        }

        impl StyleState {
            pub const ALL: &'static [StyleState] = &[$(StyleState::$gui),+];

            pub fn flags(self) -> WidgetState {
                match self {
                    $(StyleState::$gui => WidgetState {
                        $field: true,
                        ..WidgetState::default()
                    },)+
                }
            }
        }
    };
}

brul_style::states!(define_style_state);

#[derive(Clone, Debug, Default)]
pub struct StyleSet {
    base: WidgetStyle,
    transitions: Transitions,
    hovered: Option<WidgetStylePatch>,
    pressed: Option<WidgetStylePatch>,
    checked: Option<WidgetStylePatch>,
    selected: Option<WidgetStylePatch>,
    disabled: Option<WidgetStylePatch>,
    focused: Option<WidgetStylePatch>,
}

impl StyleSet {
    pub fn new(base: WidgetStyle) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn with_transitions(mut self, transitions: Transitions) -> Self {
        self.transitions = transitions;
        self
    }

    pub fn with(mut self, state: StyleState, patch: WidgetStylePatch) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(patch),
            StyleState::Pressed => self.pressed = Some(patch),
            StyleState::Checked => self.checked = Some(patch),
            StyleState::Selected => self.selected = Some(patch),
            StyleState::Disabled => self.disabled = Some(patch),
            StyleState::Focused => self.focused = Some(patch),
        }
        self
    }

    pub fn base(&self) -> WidgetStyle {
        self.base
    }

    pub fn patch(&self, state: StyleState) -> Option<&WidgetStylePatch> {
        match state {
            StyleState::Hovered => self.hovered.as_ref(),
            StyleState::Pressed => self.pressed.as_ref(),
            StyleState::Checked => self.checked.as_ref(),
            StyleState::Selected => self.selected.as_ref(),
            StyleState::Disabled => self.disabled.as_ref(),
            StyleState::Focused => self.focused.as_ref(),
        }
    }

    pub fn transitions(&self) -> Transitions {
        self.transitions
    }

    pub fn merge(&mut self, patch: &WidgetStylePatch) {
        self.base.apply(patch);
    }

    pub(crate) fn apply_layers(
        &mut self,
        layers: &Layers<WidgetStylePatch>,
        transitions: Transitions,
    ) {
        self.base.apply(layers.base());
        for &state in StyleState::ALL {
            if let Some(patch) = layers.patch(state) {
                self.state_mut(state).get_or_insert_default().merge(patch);
            }
        }
        self.transitions.merge(&transitions);
    }

    fn state_mut(&mut self, state: StyleState) -> &mut Option<WidgetStylePatch> {
        match state {
            StyleState::Hovered => &mut self.hovered,
            StyleState::Pressed => &mut self.pressed,
            StyleState::Checked => &mut self.checked,
            StyleState::Selected => &mut self.selected,
            StyleState::Disabled => &mut self.disabled,
            StyleState::Focused => &mut self.focused,
        }
    }

    brul_style::states!(resolve_layers);
}

fn tween_paint(from: Paint, to: Paint, progress: f32) -> Paint {
    match (from, to) {
        (Paint::Color(from), Paint::Color(to)) => Paint::Color(from.tween(to, progress)),
        _ => to,
    }
}

fn tween_tint(from: Option<Color>, to: Option<Color>, progress: f32) -> Option<Color> {
    match (from, to) {
        (Some(from), Some(to)) => Some(from.tween(to, progress)),
        _ => to,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    static LIGHT: LazyLock<[Class; 1]> = LazyLock::new(|| {
        [Class {
            layout: LayoutPatch::DEFAULT,
            style: Layers::new(WidgetStylePatch::DEFAULT).with(
                StyleState::Hovered,
                WidgetStylePatch {
                    opacity: Some(0.5),
                    ..WidgetStylePatch::DEFAULT
                },
            ),
            text: Layers::new(TextStylePatch::DEFAULT),
            transitions: Transitions::DEFAULT,
        }]
    });

    #[test]
    fn a_class_table_is_built_in_a_static() {
        assert_eq!(
            LIGHT[0].style.patch(StyleState::Hovered),
            Some(&WidgetStylePatch {
                opacity: Some(0.5),
                ..WidgetStylePatch::DEFAULT
            })
        );
        assert_eq!(LIGHT[0].style.base(), &WidgetStylePatch::DEFAULT);
        assert!(LIGHT[0].style.patch(StyleState::Pressed).is_none());
        assert!(LIGHT[0].text.is_empty());
    }
}

use std::time::Duration;

use super::text::TextStylePatch;
use super::{WidgetState, set_if_some};
use crate::Color;
use crate::animation::{Track, Tween};
use crate::gradient::Paint;
use crate::layout::LayoutPatch;
use crate::rect::Shadow;
use crate::text::VerticalAlign;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetStyle {
    pub background: Paint,
    pub border: Color,
    pub text: Color,
    pub border_radius: u32,
    pub tint: Option<Color>,
    pub opacity: f32,
    pub vertical_align: VerticalAlign,
    pub clip: bool,
    pub scroll: bool,
    pub shadow: Shadow,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            background: Paint::default(),
            border: Color::TRANSPARENT,
            text: Color::rgb(15, 23, 42),
            border_radius: 0,
            tint: None,
            opacity: 1.0,
            vertical_align: VerticalAlign::Top,
            clip: false,
            scroll: false,
            shadow: Shadow::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WidgetStylePatch {
    pub background: Option<Paint>,
    pub border: Option<Color>,
    pub text: Option<Color>,
    pub tint: Option<Color>,
    pub opacity: Option<f32>,
    pub vertical_align: Option<VerticalAlign>,
    pub clip: Option<bool>,
    pub shadow: Option<Color>,
    pub border_radius: Option<u32>,
    pub scroll: Option<bool>,
    pub shadow_x: Option<i32>,
    pub shadow_y: Option<i32>,
    pub shadow_blur: Option<u32>,
}

patch!(pub(crate) WidgetStylePatch {
    background,
    border,
    text,
    tint,
    opacity,
    vertical_align,
    clip,
    shadow,
    border_radius,
    scroll,
    shadow_x,
    shadow_y,
    shadow_blur,
});

impl WidgetStyle {
    fn apply(&mut self, patch: &WidgetStylePatch) {
        let WidgetStylePatch {
            background,
            border,
            text,
            tint,
            opacity,
            vertical_align,
            clip,
            shadow,
            border_radius,
            scroll,
            shadow_x,
            shadow_y,
            shadow_blur,
        } = *patch;
        set_if_some(&mut self.background, background);
        set_if_some(&mut self.border, border);
        set_if_some(&mut self.text, text);
        self.tint = tint.or(self.tint);
        set_if_some(&mut self.opacity, opacity);
        set_if_some(&mut self.vertical_align, vertical_align);
        set_if_some(&mut self.clip, clip);
        set_if_some(&mut self.shadow.color, shadow);
        set_if_some(&mut self.border_radius, border_radius);
        set_if_some(&mut self.scroll, scroll);
        set_if_some(&mut self.shadow.x, shadow_x);
        set_if_some(&mut self.shadow.y, shadow_y);
        set_if_some(&mut self.shadow.blur, shadow_blur);
    }

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layers<P> {
    base: P,
    hovered: Option<P>,
    pressed: Option<P>,
    checked: Option<P>,
    selected: Option<P>,
    disabled: Option<P>,
    focused: Option<P>,
}

impl<P> Layers<P> {
    pub const fn new(base: P) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            checked: None,
            selected: None,
            disabled: None,
            focused: None,
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
            StyleState::Hovered => &self.hovered,
            StyleState::Pressed => &self.pressed,
            StyleState::Checked => &self.checked,
            StyleState::Selected => &self.selected,
            StyleState::Disabled => &self.disabled,
            StyleState::Focused => &self.focused,
        }
    }

    const fn slot_mut(&mut self, state: StyleState) -> &mut Option<P> {
        match state {
            StyleState::Hovered => &mut self.hovered,
            StyleState::Pressed => &mut self.pressed,
            StyleState::Checked => &mut self.checked,
            StyleState::Selected => &mut self.selected,
            StyleState::Disabled => &mut self.disabled,
            StyleState::Focused => &mut self.focused,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StyleState {
    Hovered,
    Pressed,
    Checked,
    Selected,
    Disabled,
    Focused,
}

impl StyleState {
    pub const ALL: [StyleState; 6] = [
        StyleState::Hovered,
        StyleState::Pressed,
        StyleState::Checked,
        StyleState::Selected,
        StyleState::Disabled,
        StyleState::Focused,
    ];

    pub fn flags(self) -> WidgetState {
        match self {
            StyleState::Hovered => WidgetState {
                hovered: true,
                ..WidgetState::default()
            },
            StyleState::Pressed => WidgetState {
                pressed: true,
                ..WidgetState::default()
            },
            StyleState::Checked => WidgetState {
                checked: true,
                ..WidgetState::default()
            },
            StyleState::Selected => WidgetState {
                selected: true,
                ..WidgetState::default()
            },
            StyleState::Disabled => WidgetState {
                disabled: true,
                ..WidgetState::default()
            },
            StyleState::Focused => WidgetState {
                focused: true,
                ..WidgetState::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Transitions {
    pub background: Option<Duration>,
    pub border: Option<Duration>,
    pub text: Option<Duration>,
    pub tint: Option<Duration>,
    pub opacity: Option<Duration>,
    pub shadow: Option<Duration>,
}

patch!(pub(crate) Transitions {
    background,
    border,
    text,
    tint,
    opacity,
    shadow,
});

impl Transitions {
    pub fn is_empty(&self) -> bool {
        *self == Self::DEFAULT
    }
}

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
        for state in StyleState::ALL {
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

    pub fn resolve(&self, state: WidgetState) -> WidgetStyle {
        let mut style = self.base;
        let layers = [
            (state.selected, self.selected),
            (state.checked, self.checked),
            (state.hovered, self.hovered),
            (state.focused, self.focused),
            (state.pressed, self.pressed),
            (state.disabled, self.disabled),
        ];
        for (active, patch) in layers {
            if let (true, Some(patch)) = (active, patch) {
                style.apply(&patch);
            }
        }
        style
    }
}

pub(super) struct Transition {
    from: WidgetStyle,
    pub(super) to: WidgetStyle,
    background: Option<Track>,
    border: Option<Track>,
    text: Option<Track>,
    tint: Option<Track>,
    opacity: Option<Track>,
    shadow: Option<Track>,
}

impl Transition {
    pub(super) fn new(from: WidgetStyle, to: WidgetStyle, durations: Transitions) -> Self {
        Self {
            from,
            to,
            background: durations.background.map(Track::new),
            border: durations.border.map(Track::new),
            text: durations.text.map(Track::new),
            tint: durations.tint.map(Track::new),
            opacity: durations.opacity.map(Track::new),
            shadow: durations.shadow.map(Track::new),
        }
    }

    pub(super) fn style(&self) -> WidgetStyle {
        let mut style = self.to;
        if let Some(track) = self.background {
            let progress = track.value();
            style.background = tween_paint(self.from.background, self.to.background, progress);
        }
        if let Some(track) = self.border {
            style.border = self.from.border.tween(self.to.border, track.value());
        }
        if let Some(track) = self.text {
            style.text = self.from.text.tween(self.to.text, track.value());
        }
        if let Some(track) = self.tint {
            style.tint = tween_tint(self.from.tint, self.to.tint, track.value());
        }
        if let Some(track) = self.opacity {
            style.opacity = self.from.opacity.tween(self.to.opacity, track.value());
        }
        if let Some(track) = self.shadow {
            style.shadow.color = self
                .from
                .shadow
                .color
                .tween(self.to.shadow.color, track.value());
        }
        style
    }

    pub(super) fn advance(&mut self, delta: Duration) -> bool {
        let tracks = [
            &mut self.background,
            &mut self.border,
            &mut self.text,
            &mut self.tint,
            &mut self.opacity,
            &mut self.shadow,
        ];
        let mut running = false;
        for track in tracks.into_iter().flatten() {
            running |= track.advance(delta);
        }
        running
    }
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

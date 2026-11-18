use super::{Layers, StyleState, WidgetState, set_if_some};
use crate::text::{TextAlign, TextStyle};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStylePatch {
    pub weight: Option<u16>,
    pub align: Option<TextAlign>,
    pub size: Option<f32>,
    pub family: Option<String>,
}

impl TextStylePatch {
    pub const DEFAULT: Self = Self {
        weight: None,
        align: None,
        size: None,
        family: None,
    };

    fn merge(&mut self, other: &Self) {
        if other.weight.is_some() {
            self.weight = other.weight;
        }
        if other.align.is_some() {
            self.align = other.align;
        }
        if other.size.is_some() {
            self.size = other.size;
        }
        if other.family.is_some() {
            self.family = other.family.clone();
        }
    }
}

fn apply_size(style: &mut TextStyle, size: Option<f32>) {
    if let Some(size) = size {
        style.font_size = size;
        style.line_height = size * 1.2;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyleSet {
    base: TextStyle,
    hovered: Option<TextStylePatch>,
    pressed: Option<TextStylePatch>,
    checked: Option<TextStylePatch>,
    selected: Option<TextStylePatch>,
    disabled: Option<TextStylePatch>,
    focused: Option<TextStylePatch>,
}

impl TextStyleSet {
    pub fn new(base: TextStyle) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn with(mut self, state: StyleState, patch: TextStylePatch) -> Self {
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

    pub fn base(&self) -> &TextStyle {
        &self.base
    }

    pub fn patch(&self, state: StyleState) -> Option<&TextStylePatch> {
        match state {
            StyleState::Hovered => self.hovered.as_ref(),
            StyleState::Pressed => self.pressed.as_ref(),
            StyleState::Checked => self.checked.as_ref(),
            StyleState::Selected => self.selected.as_ref(),
            StyleState::Disabled => self.disabled.as_ref(),
            StyleState::Focused => self.focused.as_ref(),
        }
    }

    pub fn merge(&mut self, patch: &TextStylePatch) {
        set_if_some(&mut self.base.weight, patch.weight);
        set_if_some(&mut self.base.align, patch.align);
        apply_size(&mut self.base, patch.size);
        if let Some(family) = &patch.family {
            self.base.family = Some(family.clone());
        }
    }

    pub(crate) fn apply_layers(&mut self, layers: &Layers<TextStylePatch>) {
        self.merge(layers.base());
        for state in StyleState::ALL {
            if let Some(patch) = layers.patch(state) {
                self.state_mut(state).get_or_insert_default().merge(patch);
            }
        }
    }

    fn state_mut(&mut self, state: StyleState) -> &mut Option<TextStylePatch> {
        match state {
            StyleState::Hovered => &mut self.hovered,
            StyleState::Pressed => &mut self.pressed,
            StyleState::Checked => &mut self.checked,
            StyleState::Selected => &mut self.selected,
            StyleState::Disabled => &mut self.disabled,
            StyleState::Focused => &mut self.focused,
        }
    }

    pub fn resolve(&self, state: WidgetState) -> TextStyle {
        let mut style = self.base.clone();
        let layers = [
            (state.selected, self.selected.as_ref()),
            (state.checked, self.checked.as_ref()),
            (state.hovered, self.hovered.as_ref()),
            (state.focused, self.focused.as_ref()),
            (state.pressed, self.pressed.as_ref()),
            (state.disabled, self.disabled.as_ref()),
        ];
        for (active, patch) in layers {
            let Some(patch) = patch.filter(|_| active) else {
                continue;
            };
            set_if_some(&mut style.weight, patch.weight);
            set_if_some(&mut style.align, patch.align);
            apply_size(&mut style, patch.size);
            if let Some(family) = &patch.family {
                style.family = Some(family.clone());
            }
        }
        style
    }
}

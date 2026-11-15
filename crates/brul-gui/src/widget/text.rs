use super::{Layers, StyleState, WidgetState};
use crate::text::{TextAlign, TextStyle};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStylePatch {
    pub weight: Option<u16>,
    pub align: Option<TextAlign>,
}

impl Default for TextStylePatch {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl TextStylePatch {
    pub const DEFAULT: Self = Self {
        weight: None,
        align: None,
    };
    fn merge(&mut self, other: &Self) {
        if other.weight.is_some() {
            self.weight = other.weight;
        }
        if other.align.is_some() {
            self.align = other.align;
        }
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
        }
    }

    pub fn merge(&mut self, patch: &TextStylePatch) {
        if let Some(weight) = patch.weight {
            self.base.weight = weight;
        }
        if let Some(align) = patch.align {
            self.base.align = align;
        }
    }

    pub(crate) fn apply_layers(&mut self, layers: &Layers<TextStylePatch>) {
        self.merge(&layers.base());
        for state in StyleState::ALL {
            if let Some(patch) = layers.patch(state) {
                self.state_mut(state).get_or_insert_default().merge(&patch);
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
        }
    }

    pub fn resolve(&self, state: WidgetState) -> TextStyle {
        let mut style = self.base.clone();
        let layers = [
            (state.selected, self.selected),
            (state.checked, self.checked),
            (state.hovered, self.hovered),
            (state.pressed, self.pressed),
            (state.disabled, self.disabled),
        ];
        for (active, patch) in layers {
            let Some(patch) = patch.filter(|_| active) else {
                continue;
            };
            if let Some(weight) = patch.weight {
                style.weight = weight;
            }
            if let Some(align) = patch.align {
                style.align = align;
            }
        }
        style
    }
}

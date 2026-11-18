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

macro_rules! define_text_style_set {
    ($(($syntax:ident, $gui:ident, $field:ident, $keyword:literal)),+ $(,)?) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct TextStyleSet {
            base: TextStyle,
            $($field: Option<TextStylePatch>,)+
        }

        impl TextStyleSet {
            pub fn new(base: TextStyle) -> Self {
                Self {
                    base,
                    ..Self::default()
                }
            }

            pub fn with(mut self, state: StyleState, patch: TextStylePatch) -> Self {
                *self.state_mut(state) = Some(patch);
                self
            }

            pub fn base(&self) -> &TextStyle {
                &self.base
            }

            pub fn patch(&self, state: StyleState) -> Option<&TextStylePatch> {
                self.state(state).as_ref()
            }

            fn state(&self, state: StyleState) -> &Option<TextStylePatch> {
                match state {
                    $(StyleState::$gui => &self.$field,)+
                }
            }

            fn state_mut(&mut self, state: StyleState) -> &mut Option<TextStylePatch> {
                match state {
                    $(StyleState::$gui => &mut self.$field,)+
                }
            }
        }

        impl TextStyleSet {
            pub fn resolve(&self, state: WidgetState) -> TextStyle {
                let mut style = self.base.clone();
                let layers = [$( (state.$field, self.$field.as_ref()), )+];
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
    };
}

brul_style::states!(define_text_style_set);

impl TextStyleSet {
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
        for &state in StyleState::ALL {
            if let Some(patch) = layers.patch(state) {
                self.state_mut(state).get_or_insert_default().merge(patch);
            }
        }
    }
}

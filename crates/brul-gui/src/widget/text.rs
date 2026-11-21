use super::{Layers, StyleState, WidgetState};
use crate::text::{TextStyle, TextStylePatch};

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
                    style.apply(patch);
                }
                style
            }
        }
    };
}

brul_style::states!(define_text_style_set);

impl TextStyleSet {
    pub fn merge(&mut self, patch: &TextStylePatch) {
        self.base.apply(patch);
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

use super::{EventTarget, WidgetId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StateId {
    module: &'static str,
    path: &'static str,
    widget: u32,
}

impl StateId {
    pub const fn source(module: &'static str, path: &'static str) -> Self {
        Self {
            module,
            path,
            widget: 0,
        }
    }

    pub const fn path(self) -> &'static str {
        self.path
    }

    pub const fn for_widget(self, widget: WidgetId) -> Self {
        Self {
            module: self.module,
            path: self.path,
            widget: widget.get() + 1,
        }
    }

    pub const fn for_target(self, target: &EventTarget) -> Self {
        match target {
            EventTarget::Widget(widget) => self.for_widget(*widget),
            EventTarget::Window => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_addresses_are_distinct_from_the_window_address() {
        let source = StateId::source("editor", "Input/attach:editor");
        let first = source.for_widget(WidgetId::new(0));
        let second = source.for_widget(WidgetId::new(1));

        assert_ne!(source, first);
        assert_ne!(first, second);
        assert_eq!(
            source.for_target(&EventTarget::Widget(WidgetId::new(1))),
            second
        );
        assert_eq!(source.for_target(&EventTarget::Window), source);
    }
}

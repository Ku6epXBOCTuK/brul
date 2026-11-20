macro_rules! widgets {
    ($($widget:ident => $content:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Widget {
            $($widget),+
        }

        impl Widget {
            pub const ALL: &'static [Widget] = &[$(Widget::$widget),+];

            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $(stringify!($widget) => Some(Widget::$widget),)+
                    _ => None,
                }
            }

            pub fn name(self) -> &'static str {
                match self {
                    $(Widget::$widget => stringify!($widget),)+
                }
            }

            pub fn is_leaf(self) -> bool {
                match self {
                    $(Widget::$widget => widgets!($content),)+
                }
            }
        }
    };
    (leaf) => {
        true
    };
    (container) => {
        false
    };
}

widgets! {
    Container => container,
    Text => leaf,
    TextInput => leaf,
    Image => leaf,
    Layer => container,
}

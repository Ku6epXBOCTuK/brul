use crate::properties::Kind;

#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub themes: Vec<Theme>,
    pub rules: Vec<ClassRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub properties: Vec<ThemeEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeEntry {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassRule {
    pub class: String,
    pub state: Option<State>,
    pub properties: Vec<Property>,
    pub animations: Vec<Animation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Hover,
    Pressed,
    Checked,
    Selected,
    Disabled,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Hover => "hover",
            State::Pressed => "pressed",
            State::Checked => "checked",
            State::Selected => "selected",
            State::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub kind: Kind,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub property: Kind,
    pub duration: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Px(f64),
    Percent(f64),
    String(String),
    Keyword(String),
    Color(Color),
    ThemeRef(String),
    Gradient(Gradient),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub angle: f64,
    pub from: GradientColor,
    pub to: GradientColor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GradientColor {
    Color(Color),
    Theme(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const WHITE: Color = Color {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    pub fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }
}

#[cfg(test)]
mod tests {
    use super::State;

    const ALL: [State; 5] = [
        State::Hover,
        State::Pressed,
        State::Checked,
        State::Selected,
        State::Disabled,
    ];

    #[test]
    fn every_state_has_its_own_name() {
        let mut names: Vec<&str> = ALL.iter().map(|state| state.name()).collect();
        assert!(names.iter().all(|name| !name.is_empty()));
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ALL.len());
    }
}

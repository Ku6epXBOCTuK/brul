#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub themes: Vec<Theme>,
    pub rules: Vec<ClassRule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassRule {
    pub class: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: String,
    pub value: Value,
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
    Border(Border),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Border {
    pub width: f64,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub angle: f64,
    pub from: Color,
    pub to: Color,
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

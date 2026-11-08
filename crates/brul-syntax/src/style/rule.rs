#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Stylesheet,
    ThemeBlock,
    ThemeProp,
    ClassRule,
    State,
    Prop,
    Value,
    BorderValue,
    Keyword,
    Gradient,
    ThemeRef,
    Ident,
    Number,
    String,
    Color,
}

impl Rule {
    pub const ALL: &[Rule] = &[
        Rule::Stylesheet,
        Rule::ThemeBlock,
        Rule::ThemeProp,
        Rule::ClassRule,
        Rule::State,
        Rule::Prop,
        Rule::Value,
        Rule::BorderValue,
        Rule::Keyword,
        Rule::Gradient,
        Rule::ThemeRef,
        Rule::Ident,
        Rule::Number,
        Rule::String,
        Rule::Color,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Rule::Stylesheet => "Stylesheet",
            Rule::ThemeBlock => "ThemeBlock",
            Rule::ThemeProp => "ThemeProp",
            Rule::ClassRule => "ClassRule",
            Rule::State => "State",
            Rule::Prop => "Prop",
            Rule::Value => "Value",
            Rule::BorderValue => "BorderValue",
            Rule::Keyword => "Keyword",
            Rule::Gradient => "Gradient",
            Rule::ThemeRef => "ThemeRef",
            Rule::Ident => "Ident",
            Rule::Number => "Number",
            Rule::String => "String",
            Rule::Color => "Color",
        }
    }
}

use heck::{ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};

const RESERVED: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
    "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in",
    "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref",
    "return", "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe",
    "unsized", "use", "virtual", "where", "while", "yield",
];

pub fn snake(name: &str) -> String {
    name.trim_start_matches("r#").to_snake_case()
}

pub fn camel(name: &str) -> String {
    name.trim_start_matches("r#").to_lower_camel_case()
}

pub fn pascal(name: &str) -> String {
    name.trim_start_matches("r#").to_upper_camel_case()
}

pub fn constant(name: &str) -> String {
    name.trim_start_matches("r#").to_shouty_snake_case()
}

pub fn identifier(name: &str) -> String {
    let mut name = snake(name);
    if name.is_empty() || name.starts_with(|character: char| character.is_ascii_digit()) {
        name.insert(0, '_');
    }
    name
}

pub fn label(name: &str) -> String {
    name.replace(|character: char| !character.is_ascii_alphanumeric(), "_")
}

pub fn ident(name: &str) -> String {
    if RESERVED.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_become_snake_case() {
        assert_eq!(snake("launchBrowser"), "launch_browser");
        assert_eq!(snake("foo__Bar"), "foo_bar");
        assert_eq!(snake("r#match"), "match");
    }

    #[test]
    fn names_become_camel_case() {
        assert_eq!(camel("launch_browser"), "launchBrowser");
        assert_eq!(camel("r#match"), "match");
    }

    #[test]
    fn names_become_pascal_case() {
        assert_eq!(pascal("launch_browser"), "LaunchBrowser");
        assert_eq!(pascal("high-contrast"), "HighContrast");
        assert_eq!(pascal("myTheme"), "MyTheme");
    }

    #[test]
    fn names_become_constants() {
        assert_eq!(constant("high-contrast"), "HIGH_CONTRAST");
        assert_eq!(constant("launchBrowser"), "LAUNCH_BROWSER");
    }

    #[test]
    fn names_become_valid_identifiers() {
        assert_eq!(identifier("bc-badge"), "bc_badge");
        assert_eq!(identifier("2d"), "_2d");
        assert_eq!(identifier(""), "_");
        assert_eq!(label("High Contrast"), "High_Contrast");
    }

    #[test]
    fn reserved_names_are_escaped() {
        assert_eq!(ident("match"), "r#match");
        assert_eq!(ident("count"), "count");
    }
}

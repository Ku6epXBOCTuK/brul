use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bropicker::{Config, ConfigError, ResolveSource, Theme, domain_of};

const CONFIG: &str = r#"
theme = "dark"

[[browsers]]
name = "Firefox"
path = "firefox"
default = true

[[browsers]]
name = "Chrome"
path = "chrome"
args = '--profile-directory="Work"'
emoji = "work"

[rules]
"docs.example.com" = "Chrome"

[remembered]
"github.com" = "Chrome"

[settings]
remember_choice = false
always_ask = false
"#;

fn temp_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("brul-picker-{}-{nanos}.toml", std::process::id()))
}

#[test]
fn parses_browsers_rules_and_settings() {
    let config = Config::parse(CONFIG).expect("config");

    assert_eq!(config.theme, Theme::Dark);
    assert_eq!(config.browsers.len(), 2);
    assert_eq!(config.browsers[0].name, "Firefox");
    assert!(config.browsers[0].is_default);
    assert_eq!(config.browsers[1].flags, "--profile-directory=\"Work\"");
    assert_eq!(config.browsers[1].emoji, "work");
    assert_eq!(
        config.rules.get("docs.example.com").map(String::as_str),
        Some("Chrome")
    );
    assert_eq!(
        config.remembered.get("github.com").map(String::as_str),
        Some("Chrome")
    );
    assert!(!config.settings.remember_choice);
    assert!(!config.settings.always_ask);
}

#[test]
fn empty_config_has_safe_defaults() {
    let config = Config::parse("").expect("config");

    assert_eq!(config.theme, Theme::System);
    assert!(config.browsers.is_empty());
    assert!(config.settings.remember_choice);
    assert!(config.settings.always_ask);
    assert!(config.resolve("https://example.com").is_none());
}

#[test]
fn resolve_prefers_remembered_then_rule_then_default() {
    let config = Config::parse(CONFIG).expect("config");

    let remembered = config
        .resolve("https://github.com/user")
        .expect("remembered");
    assert_eq!(remembered.browser.name, "Chrome");
    assert_eq!(remembered.source, ResolveSource::Remembered);
    assert_eq!(remembered.domain, "github.com");
    assert!(remembered.auto_launch);

    let rule = config
        .resolve("https://docs.example.com/page")
        .expect("rule");
    assert_eq!(rule.source, ResolveSource::Rule);
    assert!(rule.auto_launch);

    let fallback = config
        .resolve("https://other.example/page")
        .expect("default");
    assert_eq!(fallback.browser.name, "Firefox");
    assert_eq!(fallback.source, ResolveSource::Default);
    assert!(!fallback.auto_launch);
}

#[test]
fn always_ask_blocks_auto_launch() {
    let source = CONFIG.replace("always_ask = false", "always_ask = true");
    let config = Config::parse(&source).expect("config");

    assert!(
        !config
            .resolve("https://github.com")
            .expect("remembered")
            .auto_launch
    );
    assert!(
        !config
            .resolve("https://docs.example.com")
            .expect("rule")
            .auto_launch
    );
}

#[test]
fn unknown_remembered_browser_falls_through() {
    let config = Config::parse(
        r#"
[[browsers]]
name = "Firefox"
path = "firefox"
default = true

[remembered]
"github.com" = "Missing"
"#,
    )
    .expect("config");

    let resolved = config.resolve("https://github.com").expect("default");
    assert_eq!(resolved.source, ResolveSource::Default);
}

#[test]
fn remembered_wins_over_rule_for_same_domain() {
    let config = Config::parse(
        r#"
[[browsers]]
name = "Firefox"
path = "firefox"

[[browsers]]
name = "Chrome"
path = "chrome"

[rules]
"example.com" = "Chrome"

[remembered]
"example.com" = "Firefox"
"#,
    )
    .expect("config");

    let resolved = config.resolve("https://example.com").expect("remembered");
    assert_eq!(resolved.source, ResolveSource::Remembered);
    assert_eq!(resolved.browser.name, "Firefox");
}

#[test]
fn rule_keys_are_normalized() {
    let config = Config::parse(
        r#"
[[browsers]]
name = "Chrome"
path = "chrome"

[rules]
"WWW.Example.COM" = "Chrome"
"#,
    )
    .expect("config");

    let resolved = config.resolve("https://example.com").expect("rule");
    assert_eq!(resolved.source, ResolveSource::Rule);
    assert_eq!(resolved.browser.name, "Chrome");
}

#[test]
fn domain_extraction_normalizes_urls() {
    assert_eq!(
        domain_of("https://User:Pass@WWW.Example.COM:8443/path?q=1#x"),
        "example.com"
    );
    assert_eq!(domain_of("example.com/x"), "example.com");
    assert_eq!(domain_of("https://example.com."), "example.com");
    assert_eq!(domain_of("http://[::1]:8080/"), "::1");
    assert_eq!(domain_of("https://"), "");
}

#[test]
fn rejects_invalid_configs() {
    let duplicate = r#"
[[browsers]]
name = "Firefox"
path = "firefox"

[[browsers]]
name = "Firefox"
path = "firefox2"
"#;
    assert!(matches!(
        Config::parse(duplicate),
        Err(ConfigError::Invalid(_))
    ));

    let multiple_defaults = r#"
[[browsers]]
name = "Firefox"
path = "firefox"
default = true

[[browsers]]
name = "Chrome"
path = "chrome"
default = true
"#;
    assert!(matches!(
        Config::parse(multiple_defaults),
        Err(ConfigError::Invalid(_))
    ));

    let empty_path = r#"
[[browsers]]
name = "Firefox"
path = ""
"#;
    assert!(matches!(
        Config::parse(empty_path),
        Err(ConfigError::Invalid(_))
    ));

    assert!(matches!(
        Config::parse("not toml"),
        Err(ConfigError::Parse { .. })
    ));
}

#[test]
fn loads_config_from_file() {
    let path = temp_path();
    fs::write(&path, CONFIG).expect("write config");

    let config = Config::load(&path).expect("load config");
    assert_eq!(config.browsers.len(), 2);
    assert_eq!(config.theme, Theme::Dark);

    let _ = fs::remove_file(path);
}

#[test]
fn missing_file_is_an_error_for_explicit_load() {
    let path = temp_path();

    assert!(matches!(Config::load(path), Err(ConfigError::Read { .. })));
}

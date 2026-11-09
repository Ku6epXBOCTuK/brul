use brul_syntax::error::Error;
use brul_syntax::script::{Program, parse, validate};

fn program(source: &str) -> Program {
    parse(source).unwrap()
}

fn accepts(source: &str, external: &[&str]) {
    validate(&program(source), external).unwrap();
}

fn failure(source: &str, external: &[&str]) -> Error {
    validate(&program(source), external).unwrap_err()
}

fn at(message: &str, line: usize, column: usize) -> Error {
    Error {
        message: message.to_owned(),
        line,
        column,
    }
}

#[test]
fn names_from_every_source_resolve() {
    accepts(
        "let browsers = loadBrowsers();\n\
         if (len(browsers) > 0) {\n\
           let first = browsers[0];\n\
           openWith(first.path, String(first.flags));\n\
         }\n\
         function refresh() {\n\
           reloadServices();\n\
           exit();\n\
         }\n\
         every(3000, refresh);\n",
        &["loadBrowsers", "openWith", "reloadServices"],
    );
}

#[test]
fn undefined_variable_points_at_the_name() {
    assert_eq!(
        failure("let value = missing;", &[]),
        at("variable 'missing' is not defined", 1, 13)
    );
}

#[test]
fn undefined_assignment_target_points_at_the_name() {
    assert_eq!(
        failure("missing = 1;", &[]),
        at("variable 'missing' is not defined", 1, 1)
    );
}

#[test]
fn undefined_function_points_at_the_call() {
    assert_eq!(
        failure("launchBrowser(2);", &[]),
        at("function 'launchBrowser' is not defined", 1, 1)
    );
}

#[test]
fn external_names_cover_properties_and_host_functions() {
    accepts(
        "url = String(count);\n\
         if (selected) { launch(url); }",
        &["url", "count", "selected", "launch"],
    );
}

#[test]
fn function_parameters_and_locals_resolve() {
    accepts(
        "function launch(i) {\n\
           let browser = browsers[i];\n\
           browser.flags = browser.flags + \"\";\n\
         }",
        &["browsers"],
    );
}

#[test]
fn function_locals_do_not_escape_the_body() {
    assert_eq!(
        failure("function run() { let inner = 1; }\ninner;", &[]),
        at("variable 'inner' is not defined", 2, 1)
    );
}

#[test]
fn functions_resolve_before_their_declaration() {
    accepts(
        "start();\nfunction start() { stop(); }\nfunction stop() {}",
        &[],
    );
}

#[test]
fn declarations_inside_blocks_and_branches_resolve() {
    accepts(
        "if (true) { let value = 1; } else { let value = 2; }\nvalue = value + 1;",
        &[],
    );
}

#[test]
fn field_and_index_keys_are_not_variables() {
    accepts(
        "let answer = settings[\"always-ask\"];\nlet path = browser[\"profile-dir\"];",
        &["settings", "browser"],
    );
}

#[test]
fn arguments_and_template_parts_are_checked() {
    assert_eq!(
        failure("len(unknown);", &[]),
        at("variable 'unknown' is not defined", 1, 5)
    );
    assert_eq!(
        failure("let text = `value: ${missing}`;", &[]),
        at("variable 'missing' is not defined", 1, 22)
    );
}

#[test]
fn a_variable_is_not_visible_in_its_own_initializer() {
    assert_eq!(
        failure("let value = value;", &[]),
        at("variable 'value' is not defined", 1, 13)
    );
}

#[test]
fn a_declaration_without_an_initializer_resolves() {
    accepts("let value;\nvalue = 1;", &[]);
}

#[test]
fn every_expression_kind_visits_its_children() {
    for source in [
        "let x = missing.field;",
        "let x = -missing;",
        "let x = missing[0];",
        "let x = missing + 1;",
        "let x = missing ? 1 : 2;",
        "let x = missing = 1;",
        "let x = [missing];",
        "let x = { key: missing };",
        "let x = `${missing}`;",
    ] {
        let error = failure(source, &[]);
        assert_eq!(
            error.message, "variable 'missing' is not defined",
            "{source}"
        );
    }
}

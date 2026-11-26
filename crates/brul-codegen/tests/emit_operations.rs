use std::path::Path;
use std::path::PathBuf;

use brul_codegen::emit;
use brul_codegen::{Capabilities, commands, generate as generate_module, generate_with};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codegen")
        .join(name)
}

fn generate(name: &str) -> Result<String, brul_syntax::error::Error> {
    let dir = fixture(name);
    let input = dir.join("input.brul");
    let commands_path = dir.join("commands.txt");
    let module = if commands_path.is_file() {
        let capabilities =
            Capabilities::discover_with_commands(&dir, commands::read(&commands_path)?)?;
        generate_with(&input, &capabilities)?
    } else {
        generate_module(&input, &dir)?
    };
    Ok(module.flat().to_owned())
}
fn operated(source: &str) -> String {
    let expression = brul_syntax::parser::parse_expression(source).expect("expression parses");
    let names = emit::identifiers(&expression);
    emit::operation(
        &expression,
        &names,
        &emit::State::default(),
        &std::collections::HashMap::new(),
    )
    .expect("operation")
}

fn operated_error(source: &str) -> brul_syntax::error::Error {
    let expression = brul_syntax::parser::parse_expression(source).expect("expression parses");
    let names = emit::identifiers(&expression);
    emit::operation(
        &expression,
        &names,
        &emit::State::default(),
        &std::collections::HashMap::new(),
    )
    .expect_err("error")
}

fn operated_with_command(source: &str, name: &str) -> String {
    let expression = brul_syntax::parser::parse_expression(source).expect("expression parses");
    let names = emit::identifiers(&expression);
    let mut commands = commands::Commands::new();
    commands.insert(
        name.to_owned(),
        commands::Command {
            name: name.to_owned(),
            path: format!("crate::{name}"),
            params: Vec::new(),
            result: brul_syntax::types::Type::Void,
        },
    );
    let state = emit::State::default().with_commands(&commands);
    emit::operation(
        &expression,
        &names,
        &state,
        &std::collections::HashMap::new(),
    )
    .expect("operation")
}

#[test]
fn emits_value_literals_and_reads() {
    assert_eq!(
        operated("x"),
        "brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)"
    );
    assert_eq!(operated("42"), "brul::Dyn::num(42.0_f64)");
    assert_eq!(operated("\"hi\""), "brul::Dyn::text(\"hi\".to_owned())");
    assert_eq!(operated("true"), "brul::Dyn::flag(true)");
    assert_eq!(operated("null"), "brul::Dyn::null()");
    assert_eq!(
        operated("a.b"),
        "(brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)).field(\"b\")?"
    );
    assert_eq!(
        operated("a[0]"),
        "(brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)).index(&(brul::Dyn::num(0.0_f64)))?"
    );
}

#[test]
fn emits_value_operations() {
    assert_eq!(
        operated("!x"),
        "brul::Dyn::unary(brul::ast::UnaryOp::Not, &(brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)))?"
    );
    assert_eq!(operated("1 + 2"), "brul::Dyn::num((1.0_f64) + (2.0_f64))");
    assert_eq!(
        operated("[1, x]"),
        "brul::Dyn::array(vec![brul::Dyn::num(1.0_f64), brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)])"
    );
    assert_eq!(
        operated("{ a: 1 }"),
        "brul::Dyn::object(vec![(\"a\".to_owned(), brul::Dyn::num(1.0_f64))])"
    );
    assert_eq!(
        operated("`v=${x}`"),
        "brul::Dyn::template(vec![brul::Dyn::text(\"v=\"), brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?)])?"
    );
}

#[test]
fn emits_value_calls() {
    assert_eq!(
        operated("f(1)"),
        "{ let arguments = vec![brul::Dyn::num(1.0_f64)]; brul::Dyn::call(component, \"f\", arguments)? }"
    );
    assert!(
        operated_error("every(100, tick)")
            .message()
            .contains("function 'tick' is not defined")
    );
    assert!(
        operated_error("every(100)")
            .message()
            .contains("builtin 'every' expects 2 arguments")
    );
    assert!(
        operated_error("every(100, 5)")
            .message()
            .contains("every expects a function name")
    );
    assert!(operated("a.b()").contains("Unsupported(\"method call\")"));
    assert!(operated("a ? b : c").contains(".boolean()?"));
    assert!(!operated("a ? b : c").contains("match ("));
}

#[test]
fn emits_typed_builtin_values() {
    assert_eq!(
        operated("String(42)"),
        "brul::Dyn::text(brul::builtins::string(brul::Dyn::num(42.0_f64))?)"
    );
    assert_eq!(
        operated("Number(\"42\")"),
        "brul::Dyn::num(brul::builtins::number(brul::Dyn::text(\"42\".to_owned()))?)"
    );
    assert_eq!(
        operated("len([1, 2])"),
        "brul::Dyn::num(brul::builtins::len(brul::Dyn::array(vec![brul::Dyn::num(1.0_f64), brul::Dyn::num(2.0_f64)]))?)"
    );
    assert_eq!(
        operated("theme()"),
        "brul::Dyn::text(<brul::builtins::Theme as brul::HostCommand>::invoke(component, ())?)"
    );
    assert_eq!(operated("setTheme(\"dark\")"), "brul::Dyn::null()");
}

#[test]
fn emits_typed_timer_builtins() {
    assert_eq!(
        operated_with_command("every(100, tick)", "tick"),
        "brul::Dyn::from({ let __brul_interval = 100.0_f64; component.every(__brul_interval, \"tick\")? })"
    );
    assert_eq!(
        operated_with_command("after(250, tick)", "tick"),
        "brul::Dyn::from({ let __brul_interval = 250.0_f64; component.after(__brul_interval, \"tick\")? })"
    );
    assert!(
        operated_error("every(100, tick, more)")
            .message()
            .contains("builtin 'every' expects 2 arguments")
    );
    assert!(
        operated_error("every(\"100\", tick)")
            .message()
            .contains("argument expects Number, found String")
    );
}

#[test]
fn emits_after_and_cancel_calls() {
    let dir = fixture("emits_typed_after_and_cancel_calls");
    let expected = std::fs::read_to_string(dir.join("expected.rs")).unwrap();
    assert_eq!(
        generate("emits_typed_after_and_cancel_calls").unwrap(),
        expected
    );
}

#[test]
fn emits_value_assignments() {
    let source = operated("a.b = 1");
    assert!(
        source.contains("let value = brul::Dyn::num(1.0_f64);"),
        "{source}"
    );
    assert!(
        source
            .contains("let mut container0 = brul::Dyn::from(component.read_id(brul::VariableId::from_index(0))?);"),
        "{source}"
    );
    assert!(
        source.contains("container0.assign_field(\"b\", &value)?;"),
        "{source}"
    );
    assert!(
        source.contains(
            "component.assign_id(brul::VariableId::from_index(0), (container0.clone()).into())?;"
        ),
        "{source}"
    );
    assert!(source.ends_with("value }"), "{source}");

    assert!(
        operated("items[0] = x").contains(".assign_index("),
        "{}",
        operated("items[0] = x")
    );
    assert!(
        operated("x += 1").contains("brul::Dyn::binary(brul::ast::BinaryOp::Add"),
        "{}",
        operated("x += 1")
    );
    assert!(operated("x = [1, 2]").contains("brul::Dyn::array"));
}

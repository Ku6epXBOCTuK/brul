use brul_script::Error;
use brul_syntax::ast::Position;

#[test]
fn reason_matches_display_for_an_unplaced_error() {
    let error = Error::Undefined(String::from("count"));

    assert_eq!(error.reason(), error.to_string());
}

#[test]
fn reason_drops_the_position_of_a_placed_error() {
    let error = Error::DivisionByZero;
    let placed = error.clone().at(Position { line: 3, column: 5 });

    assert_eq!(placed.reason(), error.to_string());
    assert!(!placed.reason().contains("line 3"));
}

fn shows(error: Error, parts: &[&str]) {
    let text = error.to_string();
    let mut at = 0;
    for part in parts {
        let found = text[at..]
            .find(part)
            .unwrap_or_else(|| panic!("'{part}' is missing from '{text}'"));
        at += found + part.len();
    }
}

#[test]
fn every_message_carries_its_payload() {
    shows(
        Error::At(
            Position { line: 7, column: 3 },
            Box::new(Error::Undefined(String::from("alpha"))),
        ),
        &["7", "3", "alpha"],
    );
    shows(Error::FunctionRedeclared(String::from("alpha")), &["alpha"]);
    shows(
        Error::FunctionArity {
            name: String::from("alpha"),
            expected: 2,
            found: 1,
        },
        &["alpha", "2", "1"],
    );
    shows(Error::Constant(String::from("alpha")), &["alpha"]);
    shows(
        Error::Mismatch {
            operation: "*",
            left: "Number",
            right: "String",
        },
        &["*", "Number", "String"],
    );
    shows(
        Error::Expected {
            operation: "!",
            expected: "Bool",
            found: "Number",
        },
        &["!", "Bool", "Number"],
    );
    shows(
        Error::OutOfBounds {
            index: 2,
            length: 9,
        },
        &["2", "9"],
    );
    shows(Error::MissingField(String::from("alpha")), &["alpha"]);
    shows(Error::UnknownFunction(String::from("alpha")), &["alpha"]);
    shows(
        Error::Arity {
            name: String::from("alpha"),
            expected: "two",
        },
        &["alpha", "two"],
    );
    shows(
        Error::Argument {
            name: "alpha",
            expected: "Number",
            found: "String",
        },
        &["alpha", "Number", "String"],
    );
    shows(
        Error::EventArgument {
            name: String::from("alpha"),
            expected: "Number",
            found: "String",
        },
        &["alpha", "Number", "String"],
    );
    shows(Error::InvalidNumber(String::from("alpha")), &["alpha"]);
    shows(Error::Unsupported("alpha"), &["alpha"]);
}

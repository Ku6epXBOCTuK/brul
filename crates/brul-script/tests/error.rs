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

use brul_syntax::ast::ExprKind;
use brul_syntax::style::parse_root;

#[test]
fn an_effect_rule_keeps_its_arguments() {
    let style = parse_root(".a { effect: cover(1, x + 2); }").expect("parse");
    assert_eq!(style.effects.len(), 1);
    let effect = &style.effects[0];
    assert_eq!(effect.class, "a");
    assert_eq!(effect.name, "cover");
    assert_eq!(effect.args.len(), 2);
    assert!(matches!(effect.args[0].kind, ExprKind::Number(value) if value == 1.0));
    assert!(matches!(effect.args[1].kind, ExprKind::Binary { .. }));
}

#[test]
fn an_effect_in_a_state_rule_is_rejected() {
    let error = parse_root(".a:hover { effect: cover(1); }").expect_err("state effect");
    assert_eq!(error.message(), "effect is not allowed in a state rule");
}

#[test]
fn an_effect_inside_an_override_is_rejected() {
    let error =
        parse_root("override Card { .card { effect: cover(1); } }").expect_err("override effect");
    assert_eq!(error.message(), "effect is not allowed inside an override");
}

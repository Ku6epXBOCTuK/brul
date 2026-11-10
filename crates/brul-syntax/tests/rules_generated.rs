use brul_syntax::ebnf;

const FILES: [(&str, &str, &str); 4] = [
    (
        "markup",
        include_str!("../../../docs/lang/markup.ebnf"),
        include_str!("../src/markup/rule.rs"),
    ),
    (
        "style",
        include_str!("../../../docs/lang/style.ebnf"),
        include_str!("../src/style/rule.rs"),
    ),
    (
        "script",
        include_str!("../../../docs/lang/script.ebnf"),
        include_str!("../src/script/rule.rs"),
    ),
    (
        "use",
        include_str!("../../../docs/lang/use.ebnf"),
        include_str!("../src/uses/rule.rs"),
    ),
];

#[test]
fn rule_files_match_the_grammars() {
    for (file, grammar, current) in FILES {
        let rules = ebnf::parse(file, grammar).unwrap_or_else(|error| panic!("{error}"));
        assert!(
            ebnf::render(file, &rules) == current,
            "{file}: rule.rs is out of date, run `cargo run -p brul-syntax --bin rules`"
        );
    }
}

use brul_syntax::ebnf;
use std::fs;
use std::path::PathBuf;

const GRAMMARS: [(&str, &str, &str); 4] = [
    (
        "markup",
        "src/markup/rule.rs",
        include_str!("../../../../docs/lang/markup.ebnf"),
    ),
    (
        "style",
        "src/style/rule.rs",
        include_str!("../../../../docs/lang/style.ebnf"),
    ),
    (
        "script",
        "src/script/rule.rs",
        include_str!("../../../../docs/lang/script.ebnf"),
    ),
    (
        "use",
        "src/uses/rule.rs",
        include_str!("../../../../docs/lang/use.ebnf"),
    ),
];

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (grammar, target, source) in GRAMMARS {
        let rules = ebnf::parse(grammar, source)?;
        let path = root.join(target);
        fs::write(&path, ebnf::render(grammar, &rules))
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}

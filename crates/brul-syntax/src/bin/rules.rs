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

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("{0}")]
    Ebnf(#[from] ebnf::Error),
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

fn main() -> Result<(), Error> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (grammar, target, source) in GRAMMARS {
        let rules = ebnf::parse(grammar, source)?;
        let path = root.join(target);
        fs::write(&path, ebnf::render(grammar, &rules))
            .map_err(|source| Error::Write { path, source })?;
    }
    Ok(())
}

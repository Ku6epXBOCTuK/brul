use brul_syntax::ebnf::{self, Target};
use std::collections::BTreeMap;

const MARKUP: &str = include_str!("../../../docs/lang/markup.ebnf");
const STYLE: &str = include_str!("../../../docs/lang/style.ebnf");
const SCRIPT: &str = include_str!("../../../docs/lang/script.ebnf");
const USES: &str = include_str!("../../../docs/lang/use.ebnf");

fn files() -> [(&'static str, &'static str); 4] {
    [
        ("Markup", MARKUP),
        ("Style", STYLE),
        ("Script", SCRIPT),
        ("Uses", USES),
    ]
}

fn tables() -> BTreeMap<&'static str, Vec<ebnf::Rule>> {
    let mut tables = BTreeMap::new();
    for (file, source) in files() {
        let rules = ebnf::parse(file, source).unwrap_or_else(|error| panic!("{error}"));
        tables.insert(file, rules);
    }
    tables
}

#[test]
fn spec_grammar_is_well_formed() {
    assert_eq!(tables().len(), 4);
}

#[test]
fn spec_rule_references_resolve() {
    let tables = tables();
    for (file, source) in files() {
        let rules = &tables[file];
        for rule in rules {
            for reference in &rule.references {
                let at = ebnf::line_of(source, reference.pos);
                match &reference.target {
                    Target::Local(name) => assert!(
                        rules.iter().any(|candidate| &candidate.name == name),
                        "{file}:{at}: {} references unknown {name}",
                        rule.name
                    ),
                    Target::Qualified(prefix, name) => {
                        let target = tables.get(prefix.as_str()).unwrap_or_else(|| {
                            panic!(
                                "{file}:{at}: {} references unknown file {prefix}",
                                rule.name
                            )
                        });
                        assert!(
                            target.iter().any(|candidate| &candidate.name == name),
                            "{file}:{at}: {} references unknown {prefix}.{name}",
                            rule.name
                        );
                    }
                }
            }
        }
    }
}

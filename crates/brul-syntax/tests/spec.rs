use brul_syntax::markup::Rule as ParserRule;
use brul_syntax::script::Rule as ScriptRule;
use brul_syntax::style::Rule as StyleRule;
use brul_syntax::uses::Rule as UsesRule;
use std::collections::{BTreeMap, BTreeSet};

const MARKUP: &str = include_str!("../../../docs/lang/markup.ebnf");
const STYLE: &str = include_str!("../../../docs/lang/style.ebnf");
const SCRIPT: &str = include_str!("../../../docs/lang/script.ebnf");
const USES: &str = include_str!("../../../docs/lang/use.ebnf");

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Name(String),
    Text,
    Class,
    Sym(char),
}

#[derive(Debug)]
enum Target {
    Local(String),
    Qualified(String, String),
}

#[derive(Debug)]
struct Ref {
    target: Target,
    pos: usize,
}

#[derive(Debug)]
struct Rule {
    name: String,
    refs: Vec<Ref>,
}

fn files() -> [(&'static str, &'static str); 4] {
    [
        ("Markup", MARKUP),
        ("Style", STYLE),
        ("Script", SCRIPT),
        ("Uses", USES),
    ]
}

fn line_of(src: &str, pos: usize) -> usize {
    src[..pos].bytes().filter(|b| *b == b'\n').count() + 1
}

fn lex(file: &str, src: &str) -> Result<Vec<(Token, usize)>, String> {
    let err = |pos: usize, msg: &str| format!("{file}:{}: {msg}", line_of(src, pos));
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < src.len() {
        let rest = &src[pos..];
        let ch = rest.chars().next().unwrap();
        if ch.is_whitespace() {
            pos += ch.len_utf8();
            continue;
        }
        if rest.starts_with("(*") {
            let end = rest
                .find("*)")
                .ok_or_else(|| err(pos, "unterminated comment"))?;
            pos += end + 2;
            continue;
        }
        match ch {
            '=' | ';' | '|' | '[' | ']' | '{' | '}' | '(' | ')' | '.' => {
                out.push((Token::Sym(ch), pos));
                pos += 1;
            }
            '"' => {
                let end = rest[1..]
                    .find('"')
                    .ok_or_else(|| err(pos, "unterminated string"))?;
                out.push((Token::Text, pos));
                pos += end + 2;
            }
            '\'' => {
                let end = rest[1..]
                    .find('\'')
                    .ok_or_else(|| err(pos, "unterminated string"))?;
                out.push((Token::Text, pos));
                pos += end + 2;
            }
            '?' => {
                let end = rest[1..]
                    .find('?')
                    .ok_or_else(|| err(pos, "unterminated character class"))?;
                out.push((Token::Class, pos));
                pos += end + 2;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let len = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len());
                out.push((Token::Name(rest[..len].to_string()), pos));
                pos += len;
            }
            other => return Err(err(pos, &format!("unexpected character {other:?}"))),
        }
    }
    Ok(out)
}

struct Parser<'a> {
    file: &'a str,
    src: &'a str,
    tokens: Vec<(Token, usize)>,
    at: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, pos: usize, msg: &str) -> String {
        format!("{}:{}: {msg}", self.file, line_of(self.src, pos))
    }

    fn pos(&self) -> usize {
        self.tokens
            .get(self.at)
            .map_or(self.src.len(), |(_, pos)| *pos)
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).map(|(token, _)| token.clone())
    }

    fn name(&mut self) -> Result<(String, usize), String> {
        let pos = self.pos();
        match self.peek() {
            Some(Token::Name(name)) => {
                self.at += 1;
                Ok((name, pos))
            }
            _ => Err(self.err(pos, "expected name")),
        }
    }

    fn expect(&mut self, sym: char) -> Result<(), String> {
        let pos = self.pos();
        if self.peek() == Some(Token::Sym(sym)) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.err(pos, &format!("expected {sym:?}")))
        }
    }

    fn starts_factor(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token::Name(_) | Token::Text | Token::Class | Token::Sym('[' | '{' | '('))
        )
    }

    fn factor(&mut self, refs: &mut Vec<Ref>) -> Result<(), String> {
        let pos = self.pos();
        match self.peek() {
            Some(Token::Name(_)) => {
                let (first, _) = self.name()?;
                if self.peek() == Some(Token::Sym('.')) {
                    self.at += 1;
                    let (second, _) = self.name()?;
                    refs.push(Ref {
                        target: Target::Qualified(first, second),
                        pos,
                    });
                } else {
                    refs.push(Ref {
                        target: Target::Local(first),
                        pos,
                    });
                }
            }
            Some(Token::Text | Token::Class) => self.at += 1,
            Some(Token::Sym(open @ ('[' | '{' | '('))) => {
                self.at += 1;
                self.expr(refs)?;
                let close = match open {
                    '[' => ']',
                    '{' => '}',
                    _ => ')',
                };
                self.expect(close)?;
            }
            _ => return Err(self.err(pos, "expected factor")),
        }
        Ok(())
    }

    fn term(&mut self, refs: &mut Vec<Ref>) -> Result<(), String> {
        if !self.starts_factor() {
            return Err(self.err(self.pos(), "expected at least one factor"));
        }
        while self.starts_factor() {
            self.factor(refs)?;
        }
        Ok(())
    }

    fn expr(&mut self, refs: &mut Vec<Ref>) -> Result<(), String> {
        self.term(refs)?;
        while self.peek() == Some(Token::Sym('|')) {
            self.at += 1;
            self.term(refs)?;
        }
        Ok(())
    }

    fn rule(&mut self) -> Result<Rule, String> {
        let (name, _) = self.name()?;
        self.expect('=')?;
        let mut refs = Vec::new();
        self.expr(&mut refs)?;
        self.expect(';')?;
        Ok(Rule { name, refs })
    }
}

fn parse(file: &str, src: &str) -> Result<Vec<Rule>, String> {
    let mut parser = Parser {
        file,
        src,
        tokens: lex(file, src)?,
        at: 0,
    };
    let mut rules = Vec::new();
    while parser.at < parser.tokens.len() {
        rules.push(parser.rule()?);
    }
    Ok(rules)
}

fn tables() -> BTreeMap<&'static str, BTreeMap<String, Rule>> {
    let mut tables = BTreeMap::new();
    for (file, src) in files() {
        let mut table = BTreeMap::new();
        for rule in parse(file, src).unwrap_or_else(|error| panic!("{error}")) {
            let name = rule.name.clone();
            assert!(
                table.insert(name.clone(), rule).is_none(),
                "{file}: duplicate rule {name}"
            );
        }
        assert!(!table.is_empty(), "{file}: no rules");
        tables.insert(file, table);
    }
    tables
}

#[test]
fn spec_grammar_is_well_formed() {
    let tables = tables();
    assert_eq!(tables.len(), 4);
}

#[test]
fn markup_rules_match_the_spec() {
    let tables = tables();
    let spec: BTreeSet<&str> = tables["Markup"]
        .keys()
        .map(String::as_str)
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .collect();
    let parser: BTreeSet<&str> = ParserRule::ALL.iter().map(|rule| rule.name()).collect();
    for name in &spec {
        assert!(parser.contains(name), "no parser rule for {name}");
    }
    for name in &parser {
        assert!(
            spec.contains(name),
            "parser rule {name} is missing from markup.ebnf"
        );
    }
}

#[test]
fn style_rules_match_the_spec() {
    let tables = tables();
    let spec: BTreeSet<&str> = tables["Style"]
        .keys()
        .map(String::as_str)
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .collect();
    let parser: BTreeSet<&str> = StyleRule::ALL.iter().map(|rule| rule.name()).collect();
    for name in &spec {
        assert!(parser.contains(name), "no parser rule for {name}");
    }
    for name in &parser {
        assert!(
            spec.contains(name),
            "parser rule {name} is missing from style.ebnf"
        );
    }
}

#[test]
fn script_rules_match_the_spec() {
    let tables = tables();
    let spec: BTreeSet<&str> = tables["Script"]
        .keys()
        .map(String::as_str)
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .collect();
    let parser: BTreeSet<&str> = ScriptRule::ALL.iter().map(|rule| rule.name()).collect();
    for name in &spec {
        assert!(parser.contains(name), "no parser rule for {name}");
    }
    for name in &parser {
        assert!(
            spec.contains(name),
            "parser rule {name} is missing from script.ebnf"
        );
    }
}

#[test]
fn use_rules_match_the_spec() {
    let tables = tables();
    let spec: BTreeSet<&str> = tables["Uses"]
        .keys()
        .map(String::as_str)
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .collect();
    let parser: BTreeSet<&str> = UsesRule::ALL.iter().map(|rule| rule.name()).collect();
    for name in &spec {
        assert!(parser.contains(name), "no parser rule for {name}");
    }
    for name in &parser {
        assert!(
            spec.contains(name),
            "parser rule {name} is missing from use.ebnf"
        );
    }
}

#[test]
fn spec_rule_references_resolve() {
    let tables = tables();
    for (file, src) in files() {
        let table = &tables[file];
        for rule in table.values() {
            for reference in &rule.refs {
                let at = line_of(src, reference.pos);
                match &reference.target {
                    Target::Local(name) => assert!(
                        table.contains_key(name),
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
                            target.contains_key(name),
                            "{file}:{at}: {} references unknown {prefix}.{name}",
                            rule.name
                        );
                    }
                }
            }
        }
    }
}

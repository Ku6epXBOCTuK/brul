use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Name(String),
    Text,
    Class,
    Sym(char),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Local(String),
    Qualified(String, String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub target: Target,
    pub pos: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub name: String,
    pub references: Vec<Reference>,
}

pub fn line_of(source: &str, pos: usize) -> usize {
    source[..pos].bytes().filter(|byte| *byte == b'\n').count() + 1
}

fn lex(file: &str, source: &str) -> Result<Vec<(Token, usize)>, String> {
    let error = |pos: usize, message: &str| format!("{file}:{}: {message}", line_of(source, pos));
    let mut tokens = Vec::new();
    let mut pos = 0;
    while pos < source.len() {
        let rest = &source[pos..];
        let ch = rest.chars().next().unwrap();
        if ch.is_whitespace() {
            pos += ch.len_utf8();
            continue;
        }
        if rest.starts_with("(*") {
            let end = rest
                .find("*)")
                .ok_or_else(|| error(pos, "unterminated comment"))?;
            pos += end + 2;
            continue;
        }
        match ch {
            '=' | ';' | '|' | '[' | ']' | '{' | '}' | '(' | ')' | '.' => {
                tokens.push((Token::Sym(ch), pos));
                pos += 1;
            }
            '"' | '\'' => {
                let end = rest[1..]
                    .find(ch)
                    .ok_or_else(|| error(pos, "unterminated string"))?;
                tokens.push((Token::Text, pos));
                pos += end + 2;
            }
            '?' => {
                let end = rest[1..]
                    .find('?')
                    .ok_or_else(|| error(pos, "unterminated character class"))?;
                tokens.push((Token::Class, pos));
                pos += end + 2;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let len = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(rest.len());
                tokens.push((Token::Name(rest[..len].to_string()), pos));
                pos += len;
            }
            other => return Err(error(pos, &format!("unexpected character {other:?}"))),
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    file: &'a str,
    source: &'a str,
    tokens: Vec<(Token, usize)>,
    at: usize,
}

impl Parser<'_> {
    fn error(&self, pos: usize, message: &str) -> String {
        format!("{}:{}: {message}", self.file, line_of(self.source, pos))
    }

    fn pos(&self) -> usize {
        self.tokens
            .get(self.at)
            .map_or(self.source.len(), |(_, pos)| *pos)
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
            _ => Err(self.error(pos, "expected name")),
        }
    }

    fn expect(&mut self, sym: char) -> Result<(), String> {
        let pos = self.pos();
        if self.peek() == Some(Token::Sym(sym)) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(pos, &format!("expected {sym:?}")))
        }
    }

    fn starts_factor(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token::Name(_) | Token::Text | Token::Class | Token::Sym('[' | '{' | '('))
        )
    }

    fn factor(&mut self, references: &mut Vec<Reference>) -> Result<(), String> {
        let pos = self.pos();
        match self.peek() {
            Some(Token::Name(_)) => {
                let (first, _) = self.name()?;
                if self.peek() == Some(Token::Sym('.')) {
                    self.at += 1;
                    let (second, _) = self.name()?;
                    references.push(Reference {
                        target: Target::Qualified(first, second),
                        pos,
                    });
                } else {
                    references.push(Reference {
                        target: Target::Local(first),
                        pos,
                    });
                }
            }
            Some(Token::Text | Token::Class) => self.at += 1,
            Some(Token::Sym(open @ ('[' | '{' | '('))) => {
                self.at += 1;
                self.expr(references)?;
                let close = match open {
                    '[' => ']',
                    '{' => '}',
                    _ => ')',
                };
                self.expect(close)?;
            }
            _ => return Err(self.error(pos, "expected factor")),
        }
        Ok(())
    }

    fn term(&mut self, references: &mut Vec<Reference>) -> Result<(), String> {
        if !self.starts_factor() {
            return Err(self.error(self.pos(), "expected at least one factor"));
        }
        while self.starts_factor() {
            self.factor(references)?;
        }
        Ok(())
    }

    fn expr(&mut self, references: &mut Vec<Reference>) -> Result<(), String> {
        self.term(references)?;
        while self.peek() == Some(Token::Sym('|')) {
            self.at += 1;
            self.term(references)?;
        }
        Ok(())
    }

    fn rule(&mut self) -> Result<Rule, String> {
        let (name, _) = self.name()?;
        self.expect('=')?;
        let mut references = Vec::new();
        self.expr(&mut references)?;
        self.expect(';')?;
        Ok(Rule { name, references })
    }
}

pub fn parse(file: &str, source: &str) -> Result<Vec<Rule>, String> {
    let mut parser = Parser {
        file,
        source,
        tokens: lex(file, source)?,
        at: 0,
    };
    let mut rules = Vec::new();
    while parser.at < parser.tokens.len() {
        let rule = parser.rule()?;
        if rules.iter().any(|seen: &Rule| seen.name == rule.name) {
            return Err(format!("{file}: duplicate rule {}", rule.name));
        }
        rules.push(rule);
    }
    if rules.is_empty() {
        return Err(format!("{file}: no rules"));
    }
    Ok(rules)
}

pub fn render(file: &str, rules: &[Rule]) -> String {
    let mut out = format!("// NOTE: generated from docs/lang/{file}.ebnf; do not edit.\n");
    out.push_str("rule! {\n    pub enum Rule {\n");
    for rule in rules {
        if rule.name.chars().next().is_some_and(char::is_uppercase) {
            writeln!(out, "        {},", rule.name).expect("writing to a string");
        }
    }
    out.push_str("    }\n}\n");
    out
}

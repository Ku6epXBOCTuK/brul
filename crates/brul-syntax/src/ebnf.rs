use std::fmt::Write as _;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{file}:{line}: {kind}")]
    Syntax {
        file: String,
        line: usize,
        kind: SyntaxError,
    },
    #[error("{file}: duplicate rule {name}")]
    DuplicateRule { file: String, name: String },
    #[error("{file}: no rules")]
    NoRules { file: String },
}

#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("unterminated comment")]
    UnterminatedComment,
    #[error("unterminated string")]
    UnterminatedString,
    #[error("unterminated character class")]
    UnterminatedClass,
    #[error("unexpected character {symbol:?}")]
    UnexpectedCharacter { symbol: char },
    #[error("expected name")]
    ExpectedName,
    #[error("expected {symbol:?}")]
    ExpectedSymbol { symbol: char },
    #[error("expected factor")]
    ExpectedFactor,
    #[error("expected at least one factor")]
    ExpectedTerm,
}

impl Error {
    fn syntax(file: &str, source: &str, pos: usize, kind: SyntaxError) -> Self {
        Self::Syntax {
            file: file.to_string(),
            line: line_of(source, pos),
            kind,
        }
    }
}

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

fn lex(file: &str, source: &str) -> Result<Vec<(Token, usize)>, Error> {
    let error = |pos: usize, kind| Error::syntax(file, source, pos, kind);
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
                .ok_or_else(|| error(pos, SyntaxError::UnterminatedComment))?;
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
                    .ok_or_else(|| error(pos, SyntaxError::UnterminatedString))?;
                tokens.push((Token::Text, pos));
                pos += end + 2;
            }
            '?' => {
                let end = rest[1..]
                    .find('?')
                    .ok_or_else(|| error(pos, SyntaxError::UnterminatedClass))?;
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
            other => {
                return Err(error(
                    pos,
                    SyntaxError::UnexpectedCharacter { symbol: other },
                ));
            }
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
    fn error(&self, pos: usize, kind: SyntaxError) -> Error {
        Error::syntax(self.file, self.source, pos, kind)
    }

    fn pos(&self) -> usize {
        self.tokens
            .get(self.at)
            .map_or(self.source.len(), |(_, pos)| *pos)
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.at).map(|(token, _)| token.clone())
    }

    fn name(&mut self) -> Result<(String, usize), Error> {
        let pos = self.pos();
        match self.peek() {
            Some(Token::Name(name)) => {
                self.at += 1;
                Ok((name, pos))
            }
            _ => Err(self.error(pos, SyntaxError::ExpectedName)),
        }
    }

    fn expect(&mut self, sym: char) -> Result<(), Error> {
        let pos = self.pos();
        if self.peek() == Some(Token::Sym(sym)) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(pos, SyntaxError::ExpectedSymbol { symbol: sym }))
        }
    }

    fn starts_factor(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token::Name(_) | Token::Text | Token::Class | Token::Sym('[' | '{' | '('))
        )
    }

    fn factor(&mut self, references: &mut Vec<Reference>) -> Result<(), Error> {
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
            _ => return Err(self.error(pos, SyntaxError::ExpectedFactor)),
        }
        Ok(())
    }

    fn term(&mut self, references: &mut Vec<Reference>) -> Result<(), Error> {
        if !self.starts_factor() {
            return Err(self.error(self.pos(), SyntaxError::ExpectedTerm));
        }
        while self.starts_factor() {
            self.factor(references)?;
        }
        Ok(())
    }

    fn expr(&mut self, references: &mut Vec<Reference>) -> Result<(), Error> {
        self.term(references)?;
        while self.peek() == Some(Token::Sym('|')) {
            self.at += 1;
            self.term(references)?;
        }
        Ok(())
    }

    fn rule(&mut self) -> Result<Rule, Error> {
        let (name, _) = self.name()?;
        self.expect('=')?;
        let mut references = Vec::new();
        self.expr(&mut references)?;
        self.expect(';')?;
        Ok(Rule { name, references })
    }
}

pub fn parse(file: &str, source: &str) -> Result<Vec<Rule>, Error> {
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
            return Err(Error::DuplicateRule {
                file: file.to_string(),
                name: rule.name,
            });
        }
        rules.push(rule);
    }
    if rules.is_empty() {
        return Err(Error::NoRules {
            file: file.to_string(),
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_keep_their_kind() {
        assert!(matches!(
            parse("test", "a = b; a = c;"),
            Err(Error::DuplicateRule { .. })
        ));
        assert!(matches!(parse("test", ""), Err(Error::NoRules { .. })));
        assert!(matches!(
            parse("test", "a = @;"),
            Err(Error::Syntax {
                kind: SyntaxError::UnexpectedCharacter { symbol: '@' },
                ..
            })
        ));
        assert!(matches!(
            parse("test", "a = ;"),
            Err(Error::Syntax {
                kind: SyntaxError::ExpectedTerm,
                ..
            })
        ));
    }

    #[test]
    fn syntax_error_renders_position() {
        let error = parse("grammar", "a = ;").unwrap_err();
        assert_eq!(error.to_string(), "grammar:1: expected at least one factor");
    }
}

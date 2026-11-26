mod rule;

use std::ops::{Deref, DerefMut};

use crate::error::{Error, Expected};
use crate::lexer::{self, Punct, TokenStream};

pub use rule::Rule;

#[derive(Debug, Clone, PartialEq)]
pub struct Use {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resource {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Block {
    pub uses: Vec<Use>,
    pub resources: Vec<Resource>,
}

pub fn parse(source: &str) -> Result<Block, Error> {
    let tokens = lexer::tokenize(source)?;
    let mut parser = Parser {
        stream: TokenStream::new(source, tokens),
    };
    let mut block = Block::default();
    while !parser.at_end()? {
        if parser.at_keyword("resource")? {
            block.resources.push(parser.resource_item()?);
        } else {
            block.uses.push(parser.use_item()?);
        }
    }
    Ok(block)
}

struct Parser<'a> {
    stream: TokenStream<'a>,
}

impl<'a> Deref for Parser<'a> {
    type Target = TokenStream<'a>;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl DerefMut for Parser<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl Parser<'_> {
    fn use_item(&mut self) -> Result<Use, Error> {
        self.expect_keyword("import")?;
        let name = self.ident(Expected::Word("import name"))?;
        self.expect_keyword("from")?;
        let path = self.string(Expected::Word("import path"))?;
        self.expect_punct(Punct::Semicolon, Punct::Semicolon)?;
        Ok(Use { name, path })
    }

    fn resource_item(&mut self) -> Result<Resource, Error> {
        self.expect_keyword("resource")?;
        let name = self.ident(Expected::Word("resource name"))?;
        self.expect_keyword("from")?;
        let path = self.string(Expected::Word("resource path"))?;
        self.expect_punct(Punct::Semicolon, Punct::Semicolon)?;
        Ok(Resource { name, path })
    }
}

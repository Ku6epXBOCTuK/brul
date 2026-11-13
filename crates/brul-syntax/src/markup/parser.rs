use crate::ast::Position;
use crate::cursor::Cursor;
use crate::error::Error;
use crate::lexer::Span;
use crate::markup::ast::{
    Arg, Block, BlockItem, Component, Declaration, EachBlock, Element, Event, EventParam, File,
    IfBlock, Member, Prop, PropType, Window,
};

pub fn parse_file(source: &str) -> Result<File, Error> {
    let mut cursor = Cursor::new(source);
    let mut declarations = Vec::new();
    loop {
        cursor.skip_trivia();
        if cursor.peek().is_none() {
            break;
        }
        declarations.push(cursor.declaration()?);
    }
    Ok(File { declarations })
}

fn position(cursor: &Cursor<'_>, pos: usize) -> Position {
    let (line, column) = Span {
        start: pos,
        end: pos,
    }
    .line_col(cursor.source());
    Position { line, column }
}

impl Cursor<'_> {
    fn declaration(&mut self) -> Result<Declaration, Error> {
        self.skip_trivia();
        let start = self.pos();
        let keyword = self.ident()?;
        match keyword.as_str() {
            "component" => self.component(),
            "window" => self.window(),
            _ => Err(self.error_at(start, "expected 'component' or 'window'")),
        }
    }

    fn component(&mut self) -> Result<Declaration, Error> {
        let name = self.ident()?;
        let members = self.members()?;
        Ok(Declaration::Component(Component { name, members }))
    }

    fn window(&mut self) -> Result<Declaration, Error> {
        let start = self.pos();
        let name = self.ident()?;
        let args = self.arg_list()?;
        let members = self.members()?;
        Ok(Declaration::Window(Window {
            name,
            args,
            members,
            position: position(self, start),
        }))
    }

    fn members(&mut self) -> Result<Vec<Member>, Error> {
        self.expect('{')?;
        let mut members = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok(members);
                }
                None => return Err(self.error("expected '}'")),
                _ => members.push(self.member()?),
            }
        }
    }

    fn member(&mut self) -> Result<Member, Error> {
        self.skip_trivia();
        let start = self.pos();
        let name = self.ident()?;
        match name.as_str() {
            "prop" => self.prop(),
            "event" => self.event(),
            _ => self.element(name, start).map(Member::Element),
        }
    }

    fn element(&mut self, name: String, start: usize) -> Result<Element, Error> {
        self.skip_trivia();
        let args = if self.peek() == Some('(') {
            self.arg_list()?
        } else {
            Vec::new()
        };
        let block = if name == "Text" || name == "TextInput" || name == "Image" {
            self.leaf_block(&name)?
        } else {
            self.block()?
        };
        Ok(Element {
            name,
            args,
            block,
            position: position(self, start),
        })
    }

    fn leaf_block(&mut self, name: &str) -> Result<Block, Error> {
        self.expect('{')?;
        let (expr, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
        self.set_pos(end);
        self.skip_trivia();
        if self.peek() == Some('{') {
            return Err(self.error(format!(
                "{name} is a leaf widget: nested elements are not allowed"
            )));
        }
        self.expect('}')?;
        Ok(Block {
            items: vec![BlockItem::Expr(expr)],
        })
    }

    fn arg_list(&mut self) -> Result<Vec<Arg>, Error> {
        self.expect('(')?;
        let mut args = vec![self.arg()?];
        loop {
            self.skip_trivia();
            match self.peek() {
                Some(',') => {
                    self.bump();
                    args.push(self.arg()?);
                }
                Some(')') => {
                    self.bump();
                    return Ok(args);
                }
                _ => return Err(self.error("expected ',' or ')'")),
            }
        }
    }

    fn arg(&mut self) -> Result<Arg, Error> {
        let name = self.ident()?;
        if name == "on" {
            let event = self.ident()?;
            self.expect(':')?;
            let handler = self.ident()?;
            return Ok(Arg::Handler { event, handler });
        }
        self.skip_trivia();
        if self.peek() == Some(':') {
            self.bump();
            let (value, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
            self.set_pos(end);
            return Ok(Arg::Named { name, value });
        }
        Ok(Arg::Class(name))
    }

    fn block(&mut self) -> Result<Block, Error> {
        self.expect('{')?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            match self.peek() {
                Some('}') => {
                    self.bump();
                    return Ok(Block { items });
                }
                None => return Err(self.error("expected '}'")),
                _ => items.push(self.block_item()?),
            }
        }
    }

    fn block_item(&mut self) -> Result<BlockItem, Error> {
        let start = self.pos();
        let name = self.ident()?;
        match name.as_str() {
            "if" => self.if_block().map(BlockItem::If),
            "each" => self.each_block().map(BlockItem::Each),
            _ => self.element(name, start).map(BlockItem::Element),
        }
    }

    fn each_block(&mut self) -> Result<EachBlock, Error> {
        self.expect('(')?;
        let (collection, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
        self.set_pos(end);
        self.skip_trivia();
        let start = self.pos();
        if self.ident()? != "as" {
            return Err(self.error_at(start, "expected 'as'"));
        }
        let item = self.ident()?;
        self.skip_trivia();
        let index = if self.peek() == Some(',') {
            self.bump();
            Some(self.ident()?)
        } else {
            None
        };
        self.skip_trivia();
        let key = if self.peek() == Some('(') {
            self.bump();
            let (key, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
            self.set_pos(end);
            self.expect(')')?;
            Some(key)
        } else {
            None
        };
        self.expect(')')?;
        let block = self.block()?;
        Ok(EachBlock {
            collection,
            item,
            index,
            key,
            block,
        })
    }

    fn if_block(&mut self) -> Result<IfBlock, Error> {
        self.expect('(')?;
        let (condition, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
        self.set_pos(end);
        self.expect(')')?;
        let then_block = self.block()?;
        let else_block = if self.eat_keyword("else") {
            Some(self.block()?)
        } else {
            None
        };
        Ok(IfBlock {
            condition,
            then_block,
            else_block,
        })
    }

    fn prop(&mut self) -> Result<Member, Error> {
        let name = self.ident()?;
        self.expect(':')?;
        let ty = self.prop_type()?;
        self.skip_trivia();
        let default = if self.peek() == Some('=') {
            self.bump();
            let (expr, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
            self.set_pos(end);
            Some(expr)
        } else {
            None
        };
        self.expect(';')?;
        Ok(Member::Prop(Prop { name, ty, default }))
    }

    fn event(&mut self) -> Result<Member, Error> {
        let name = self.ident()?;
        let mut params = Vec::new();
        self.skip_trivia();
        if self.peek() == Some('(') {
            self.bump();
            self.skip_trivia();
            if self.peek() == Some(')') {
                self.bump();
            } else {
                loop {
                    let param = self.ident()?;
                    self.expect(':')?;
                    let ty = self.prop_type()?;
                    params.push(EventParam { name: param, ty });
                    self.skip_trivia();
                    match self.peek() {
                        Some(',') => self.bump(),
                        Some(')') => {
                            self.bump();
                            break;
                        }
                        _ => return Err(self.error("expected ',' or ')'")),
                    }
                }
            }
        }
        self.expect(';')?;
        Ok(Member::Event(Event { name, params }))
    }

    fn prop_type(&mut self) -> Result<PropType, Error> {
        self.skip_trivia();
        let start = self.pos();
        let name = self.ident()?;
        match name.as_str() {
            "Bool" => Ok(PropType::Bool),
            "Number" => Ok(PropType::Number),
            "String" => Ok(PropType::String),
            "Image" => Ok(PropType::Image),
            _ => Err(self.error_at(start, "expected property type")),
        }
    }
}

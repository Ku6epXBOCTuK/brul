use crate::ast::Position;
use crate::cursor::Cursor;
use crate::error::{Error, Expected};
use crate::lexer::Span;
use crate::markup::ast::{
    Arg, Block, BlockItem, Branch, Declaration, DeclarationKind, EachBlock, Element, Event,
    EventParam, File, IfBlock, Member, Prop, PropType, Slot,
};
use crate::markup::widget::Widget;

pub fn parse_file(source: &str) -> Result<File, Error> {
    let mut cursor = Cursor::new(source);
    let mut declaration = None;
    loop {
        cursor.skip_trivia();
        if cursor.peek().is_none() {
            break;
        }
        if declaration.is_some() {
            return Err(cursor.error(Error::MultipleDeclarations));
        }
        declaration = Some(cursor.declaration()?);
    }
    Ok(File { declaration })
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
            _ => Err(self.error_at(start, Expected::OneOf("'component' or 'window'"))),
        }
    }

    fn component(&mut self) -> Result<Declaration, Error> {
        let start = self.pos();
        let name = self.ident()?;
        let members = self.members()?;
        Ok(Declaration {
            name,
            members,
            position: position(self, start),
            kind: DeclarationKind::Component,
        })
    }

    fn window(&mut self) -> Result<Declaration, Error> {
        let start = self.pos();
        let name = self.ident()?;
        let args = self.arg_list()?;
        let members = self.members()?;
        Ok(Declaration {
            name,
            members,
            position: position(self, start),
            kind: DeclarationKind::Window { args },
        })
    }

    fn members(&mut self) -> Result<Vec<Member>, Error> {
        self.braced_items(Self::member)
    }

    fn member(&mut self) -> Result<Member, Error> {
        self.skip_trivia();
        let start = self.pos();
        if self.eat('@') {
            let name = self.ident()?;
            let reason = match name.as_str() {
                "slot" => Error::SlotAtRoot,
                _ => return Err(self.error_at(start, Error::UnexpectedDirective(name.clone()))),
            };
            return Err(self.error_at(start, reason));
        }
        let name = self.ident()?;
        match name.as_str() {
            "prop" => self.prop(),
            "event" => self.event(),
            _ => self.element(name, start).map(Member::Element),
        }
    }

    fn element(&mut self, name: String, start: usize) -> Result<Element, Error> {
        self.skip_trivia();
        let widget = Widget::from_name(&name);
        let args = if self.peek() == Some('(') {
            self.arg_list()?
        } else {
            Vec::new()
        };
        let block = if widget.is_some_and(Widget::is_leaf) {
            self.leaf_block(&name)?
        } else {
            self.block()?
        };
        Ok(Element {
            name,
            widget,
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
            return Err(self.error(Error::LeafWidget(name.to_owned())));
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
                _ => return Err(self.error(Expected::OneOf("',' or ')'"))),
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
        Ok(Block {
            items: self.braced_items(Self::block_item)?,
        })
    }

    fn block_item(&mut self) -> Result<BlockItem, Error> {
        self.skip_trivia();
        let start = self.pos();
        if self.eat('@') {
            let name = self.ident()?;
            return match name.as_str() {
                "if" => self.if_block(start).map(BlockItem::If),
                "each" => self.each_block(start).map(BlockItem::Each),
                "slot" => self.slot(start),
                _ => Err(self.error_at(start, Error::UnexpectedDirective(name.clone()))),
            };
        }
        let name = self.ident()?;
        if matches!(name.as_str(), "if" | "each" | "else" | "slot") {
            return Err(self.error_at(start, Error::ExpectedAtDirective(name.clone())));
        }
        self.element(name, start).map(BlockItem::Element)
    }

    fn slot(&mut self, start: usize) -> Result<BlockItem, Error> {
        let name = if self.peek() == Some('(') {
            self.bump();
            let name = self.ident()?;
            self.expect(')')?;
            Some(name)
        } else {
            None
        };
        self.skip_trivia();
        let block = if self.peek() == Some('{') {
            Some(self.block()?)
        } else {
            None
        };
        Ok(BlockItem::Slot(Slot {
            name,
            block,
            position: position(self, start),
        }))
    }

    fn eat_directive(&mut self, name: &str) -> bool {
        self.skip_trivia();
        if self.peek() != Some('@') {
            return false;
        }
        let start = self.pos();
        self.bump();
        if self.eat_keyword(name) {
            true
        } else {
            self.set_pos(start);
            false
        }
    }

    fn each_block(&mut self, start: usize) -> Result<EachBlock, Error> {
        self.expect('(')?;
        let (collection, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
        self.set_pos(end);
        self.skip_trivia();
        let as_start = self.pos();
        if self.ident()? != "as" {
            return Err(self.error_at(as_start, Expected::Keyword("as")));
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
        let else_block = if self.eat_directive("else") {
            Some(self.block()?)
        } else {
            None
        };
        Ok(EachBlock {
            collection,
            item,
            index,
            key,
            block,
            else_block,
            position: position(self, start),
        })
    }

    fn if_block(&mut self, start: usize) -> Result<IfBlock, Error> {
        let mut branches = vec![self.branch()?];
        let mut else_block = None;
        while self.eat_directive("else") {
            if self.eat_keyword("if") {
                branches.push(self.branch()?);
            } else {
                else_block = Some(self.block()?);
                break;
            }
        }
        Ok(IfBlock {
            branches,
            else_block,
            position: position(self, start),
        })
    }

    fn branch(&mut self) -> Result<Branch, Error> {
        let start = self.value_start();
        self.expect('(')?;
        let (condition, end) = crate::parser::parse_expression_at(self.source(), self.pos())?;
        self.set_pos(end);
        self.expect(')')?;
        let block = self.block()?;
        Ok(Branch {
            condition,
            block,
            position: position(self, start),
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
                        _ => return Err(self.error(Expected::OneOf("',' or ')'"))),
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
        if name == "Array" {
            self.skip_trivia();
            self.expect('<')?;
            let item = self.prop_type()?;
            self.skip_trivia();
            self.expect('>')?;
            return Ok(PropType::Array(Box::new(item)));
        }
        PropType::from_name(&name)
            .ok_or_else(|| self.error_at(start, Expected::Word("property type")))
    }
}

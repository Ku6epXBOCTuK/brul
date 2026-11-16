use std::fmt;
use std::fmt::Write;

pub struct Out<'a> {
    text: &'a mut String,
    depth: usize,
}

impl<'a> Out<'a> {
    pub(super) fn new(text: &'a mut String) -> Self {
        Self { text, depth: 0 }
    }

    pub(super) fn line(&mut self, args: fmt::Arguments<'_>) {
        self.start(args);
        self.end();
    }

    pub(super) fn start(&mut self, args: fmt::Arguments<'_>) {
        for _ in 0..self.depth {
            self.text.push_str("    ");
        }
        self.push(args);
    }

    pub(super) fn push(&mut self, args: fmt::Arguments<'_>) {
        let _ = self.text.write_fmt(args);
    }

    pub(super) fn end(&mut self) {
        self.text.push('\n');
    }

    pub(super) fn indented<R>(&mut self, body: impl FnOnce(&mut Self) -> R) -> R {
        self.depth += 1;
        let result = body(self);
        self.depth -= 1;
        result
    }

    pub(super) fn raw(&mut self) -> &mut String {
        self.text
    }
}

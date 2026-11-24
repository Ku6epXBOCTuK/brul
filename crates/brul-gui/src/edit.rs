use std::ops::Range;

use crate::Image;

#[derive(Clone)]
pub enum Fragment {
    Text(String),
    Image(Image),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub fragment: usize,
    pub offset: usize,
}

impl Position {
    pub const fn new(fragment: usize, offset: usize) -> Self {
        Self { fragment, offset }
    }
}

#[derive(Clone)]
pub enum ClipboardData {
    Text(String),
    Image(Image),
}

impl From<String> for ClipboardData {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for ClipboardData {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Clone)]
pub struct EditBuffer {
    fragments: Vec<Fragment>,
    caret: Position,
    anchor: Position,
    preedit: String,
    scroll: i32,
}

impl EditBuffer {
    pub fn from_text(text: impl Into<String>) -> Self {
        let text = text.into();
        let offset = text.len();
        Self {
            fragments: vec![Fragment::Text(text)],
            caret: Position::new(0, offset),
            anchor: Position::new(0, offset),
            preedit: String::new(),
            scroll: 0,
        }
    }

    pub fn text(&self) -> &str {
        match self.fragments.first() {
            Some(Fragment::Text(text)) => text,
            _ => "",
        }
    }

    pub fn fragments(&self) -> &[Fragment] {
        &self.fragments
    }

    pub fn caret(&self) -> Position {
        self.caret
    }

    pub fn set_caret(&mut self, position: Position) {
        self.caret = self.clamp(position);
    }

    pub fn anchor(&self) -> Position {
        self.anchor
    }

    pub fn set_anchor(&mut self, position: Position) {
        self.anchor = self.clamp(position);
    }

    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    pub fn set_preedit(&mut self, text: impl Into<String>) {
        self.preedit = text.into();
    }

    pub fn scroll(&self) -> i32 {
        self.scroll
    }

    pub fn set_scroll(&mut self, scroll: i32) {
        self.scroll = scroll;
    }

    pub fn insert_text(&mut self, at: Position, text: &str) {
        let Some(Fragment::Text(fragment)) = self.fragments.get_mut(at.fragment) else {
            return;
        };
        let offset = at.offset.min(fragment.len());
        if !fragment.is_char_boundary(offset) || text.is_empty() {
            return;
        }
        fragment.insert_str(offset, text);
        for position in [&mut self.caret, &mut self.anchor] {
            if position.fragment == at.fragment && position.offset >= offset {
                position.offset += text.len();
            }
        }
    }

    pub fn insert_fragment(&mut self, at: Position, fragment: Fragment) {
        let index = at.fragment.min(self.fragments.len());
        let split = match self.fragments.get_mut(index) {
            Some(Fragment::Text(text)) => {
                let offset = at.offset.min(text.len());
                if !text.is_char_boundary(offset) {
                    return;
                }
                if offset == 0 {
                    self.fragments.insert(index, fragment);
                    None
                } else {
                    let tail = text.split_off(offset);
                    self.fragments.insert(index + 1, fragment);
                    if !tail.is_empty() {
                        self.fragments.insert(index + 2, Fragment::Text(tail));
                    }
                    Some(offset)
                }
            }
            _ => {
                self.fragments.insert(index, fragment);
                None
            }
        };
        let shift = if split.is_some() { 2 } else { 1 };
        for position in [&mut self.caret, &mut self.anchor] {
            if let Some(offset) = split {
                if position.fragment == index && position.offset > offset {
                    position.fragment += 2;
                    position.offset -= offset;
                } else if position.fragment > index {
                    position.fragment += shift;
                }
            } else if position.fragment > index {
                position.fragment += shift;
            }
        }
    }

    pub fn remove(&mut self, range: Range<Position>) {
        let start = self.clamp(range.start);
        let end = self.clamp(range.end);
        if (start.fragment, start.offset) >= (end.fragment, end.offset) {
            return;
        }
        if start.fragment == end.fragment {
            if let Some(Fragment::Text(text)) = self.fragments.get_mut(start.fragment) {
                text.replace_range(start.offset..end.offset, "");
            }
        } else {
            if let Some(Fragment::Text(text)) = self.fragments.get_mut(start.fragment) {
                text.truncate(start.offset);
            }
            if let Some(Fragment::Text(text)) = self.fragments.get_mut(end.fragment) {
                text.replace_range(..end.offset, "");
            }
            for index in (start.fragment + 1..end.fragment).rev() {
                self.fragments.remove(index);
            }
        }
        self.normalize();
    }

    pub fn selection(&self) -> Range<Position> {
        let caret = self.clamp(self.caret);
        let anchor = self.clamp(self.anchor);
        if (anchor.fragment, anchor.offset) <= (caret.fragment, caret.offset) {
            anchor..caret
        } else {
            caret..anchor
        }
    }

    pub fn selected_text(&self) -> Option<&str> {
        let range = self.selection();
        if range.start == range.end || range.start.fragment != range.end.fragment {
            return None;
        }
        let Some(Fragment::Text(text)) = self.fragments.get(range.start.fragment) else {
            return None;
        };
        Some(&text[range.start.offset..range.end.offset])
    }

    pub fn delete_surrounding(&mut self, before_bytes: usize, after_bytes: usize) {
        let caret = self.caret;
        let fragment = caret.fragment;
        let (start, end) = match self.fragments.get_mut(fragment) {
            Some(Fragment::Text(text)) => {
                let mut start = caret.offset.saturating_sub(before_bytes);
                while start > 0 && !text.is_char_boundary(start) {
                    start -= 1;
                }
                let mut end = caret.offset.saturating_add(after_bytes).min(text.len());
                while end < text.len() && !text.is_char_boundary(end) {
                    end += 1;
                }
                if start == end {
                    return;
                }
                text.replace_range(start..end, "");
                (start, end)
            }
            _ => return,
        };
        for position in [&mut self.caret, &mut self.anchor] {
            if position.fragment != fragment {
                continue;
            }
            if position.offset >= end {
                position.offset -= end - start;
            } else if position.offset > start {
                position.offset = start;
            }
        }
        self.normalize();
    }

    fn clamp(&self, mut position: Position) -> Position {
        let index = position
            .fragment
            .min(self.fragments.len().saturating_sub(1));
        position.fragment = index;
        let Some(Fragment::Text(text)) = self.fragments.get(index) else {
            position.offset = 0;
            return position;
        };
        let mut offset = position.offset.min(text.len());
        while offset > 0 && !text.is_char_boundary(offset) {
            offset -= 1;
        }
        position.offset = offset;
        position
    }

    fn normalize(&mut self) {
        let mut fragments = Vec::with_capacity(self.fragments.len());
        for fragment in std::mem::take(&mut self.fragments) {
            if let (Some(Fragment::Text(left)), Fragment::Text(right)) =
                (fragments.last_mut(), &fragment)
            {
                left.push_str(right);
            } else {
                fragments.push(fragment);
            }
        }
        if fragments.is_empty() {
            fragments.push(Fragment::Text(String::new()));
        }
        self.fragments = fragments;
        self.caret = self.clamp(self.caret);
        self.anchor = self.clamp(self.anchor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Direction, Texture, Widget, WidgetTree};

    #[test]
    fn a_text_snapshot_exposes_its_caret_anchor_preedit_and_scroll() {
        let mut buffer = EditBuffer::from_text("hello");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.caret(), Position::new(0, 5));
        assert_eq!(buffer.fragments().len(), 1);

        buffer.set_caret(Position::new(0, 3));
        buffer.set_anchor(Position::new(0, 1));
        buffer.set_preedit("he");
        buffer.set_scroll(7);

        assert_eq!(buffer.caret(), Position::new(0, 3));
        assert_eq!(buffer.anchor(), Position::new(0, 1));
        assert_eq!(buffer.preedit(), "he");
        assert_eq!(buffer.scroll(), 7);
        assert_eq!(buffer.selection(), Position::new(0, 1)..Position::new(0, 3));
    }

    #[test]
    fn insert_text_shifts_the_caret_and_the_anchor() {
        let mut buffer = EditBuffer::from_text("ab");
        buffer.set_caret(Position::new(0, 0));
        buffer.set_anchor(Position::new(0, 2));

        buffer.insert_text(Position::new(0, 0), "xy");

        assert_eq!(buffer.text(), "xyab");
        assert_eq!(buffer.caret(), Position::new(0, 2));
        assert_eq!(buffer.anchor(), Position::new(0, 4));
    }

    #[test]
    fn a_fragment_is_split_at_its_position_and_removal_merges_the_text_back() {
        let mut buffer = EditBuffer::from_text("abcd");
        buffer.insert_fragment(Position::new(0, 2), Fragment::Text(String::from("XY")));
        assert_eq!(buffer.fragments().len(), 3);
        assert!(matches!(&buffer.fragments()[1], Fragment::Text(text) if text == "XY"));

        buffer.remove(Position::new(1, 0)..Position::new(1, 2));

        assert_eq!(buffer.fragments().len(), 1);
        assert_eq!(buffer.text(), "abcd");
    }

    #[test]
    fn selected_text_returns_only_a_single_text_selection() {
        let mut buffer = EditBuffer::from_text("hello");
        assert_eq!(buffer.selected_text(), None);
        buffer.set_anchor(Position::new(0, 1));
        buffer.set_caret(Position::new(0, 4));
        assert_eq!(buffer.selected_text(), Some("ell"));

        buffer.insert_fragment(Position::new(0, 2), Fragment::Text(String::from("XY")));
        assert_eq!(buffer.selected_text(), None);
    }

    #[test]
    fn delete_surrounding_clamps_to_character_boundaries_and_moves_the_anchor() {
        let mut buffer = EditBuffer::from_text("a\u{20ac}b");
        buffer.set_caret(Position::new(0, 4));
        buffer.set_anchor(Position::new(0, 5));
        buffer.delete_surrounding(1, 2);

        assert_eq!(buffer.text(), "a");
        assert_eq!(buffer.caret(), Position::new(0, 1));
        assert_eq!(buffer.anchor(), Position::new(0, 1));
        assert_eq!(buffer.selected_text(), None);
    }

    #[test]
    fn a_clipboard_from_a_string_holds_text() {
        let clipboard = ClipboardData::from(String::from("copied"));
        assert!(matches!(clipboard, ClipboardData::Text(text) if text == "copied"));
    }

    #[test]
    fn a_commit_with_an_image_fragment_is_rejected_without_touching_the_field() {
        let input = Widget::input("hi");
        let id = input.id();
        let mut tree = WidgetTree::new(Widget::view(Direction::Row, vec![input]));
        let mut buffer = tree.edit_buffer(id).expect("input snapshot");

        buffer.insert_fragment(
            Position::new(0, 1),
            Fragment::Image(Image::from(Texture::empty())),
        );

        assert!(!tree.commit_edit(id, &buffer));
        assert_eq!(tree.edit_buffer(id).expect("input snapshot").text(), "hi");
    }
}

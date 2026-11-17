use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use brul_gui::{Widget, WidgetId, WidgetTree};
use brul_script::Value;

use crate::{RowInfo, RowKey};

pub(crate) fn reconcile_each(
    tree: &mut WidgetTree,
    parent: WidgetId,
    old_rows: &[RowInfo],
    new_widget: Widget,
    new_rows: &[RowInfo],
) -> Vec<Widget> {
    let Some(old_children) = tree.take_children(parent) else {
        return new_widget.into_children();
    };
    let mut old_widgets = group(old_children, old_rows);
    let mut new_widgets = group(new_widget.into_children(), new_rows);

    let mut children = Vec::new();
    for (index, row) in new_rows.iter().enumerate() {
        match row.reuse.and_then(|other| old_widgets.get_mut(other)) {
            Some(reused) => children.append(reused),
            None => children.append(&mut new_widgets[index]),
        }
    }
    children
}

fn group(children: Vec<Widget>, rows: &[RowInfo]) -> Vec<Vec<Widget>> {
    let mut remaining = children;
    rows.iter()
        .map(|row| {
            let take = row.count.min(remaining.len());
            remaining.drain(..take).collect()
        })
        .collect()
}

pub struct RowMatcher<'a> {
    old: &'a [RowInfo],
    taken: Vec<bool>,
    keyed: HashMap<u64, Vec<usize>>,
}

impl<'a> RowMatcher<'a> {
    pub fn new(old: &'a [RowInfo]) -> Self {
        let mut keyed: HashMap<u64, Vec<usize>> = HashMap::new();
        for (index, row) in old.iter().enumerate() {
            if let RowKey::Explicit(key) = &row.key {
                keyed.entry(fingerprint(key)).or_default().push(index);
            }
        }
        Self {
            old,
            taken: vec![false; old.len()],
            keyed,
        }
    }

    pub fn take(&mut self, key: &RowKey, item: &Value, index: usize) -> Option<usize> {
        let RowKey::Explicit(key) = key else {
            let same_place = !self.taken.get(index).copied().unwrap_or(true)
                && self
                    .old
                    .get(index)
                    .is_some_and(|old| matches!(old.key, RowKey::Index(_)) && old.item == *item);
            return same_place.then(|| {
                self.taken[index] = true;
                index
            });
        };

        let at = self
            .keyed
            .get(&fingerprint(key))?
            .iter()
            .copied()
            .find(|&at| {
                !self.taken[at]
                    && matches!(&self.old[at].key, RowKey::Explicit(old) if old == key)
                    && self.old[at].item == *item
            })?;
        self.taken[at] = true;
        Some(at)
    }
}

fn fingerprint(key: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_value(key, &mut hasher);
    hasher.finish()
}

fn hash_value(value: &Value, hasher: &mut DefaultHasher) {
    match value {
        Value::Number(number) => (0u8, number.to_bits()).hash(hasher),
        Value::String(text) => (1u8, text.as_ref()).hash(hasher),
        Value::Bool(flag) => (2u8, flag).hash(hasher),
        Value::Null => 3u8.hash(hasher),
        Value::Array(items) => {
            4u8.hash(hasher);
            for item in items.iter() {
                hash_value(item, hasher);
            }
        }
        Value::Object(entries) => {
            5u8.hash(hasher);
            for (name, value) in entries.iter() {
                name.hash(hasher);
                hash_value(value, hasher);
            }
        }
        Value::Image(path) => (6u8, path.as_ref()).hash(hasher),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use brul_gui::Direction;

    use crate::Value;

    fn row(key: Option<Value>, item: &str, count: usize) -> RowInfo {
        RowInfo {
            key: key.map_or(RowKey::Index(0), RowKey::Explicit),
            item: Value::String(item.into()),
            start: 0,
            count,
            reuse: None,
        }
    }

    fn matched(old: &[RowInfo], new: Vec<RowInfo>) -> Vec<RowInfo> {
        let mut matcher = RowMatcher::new(old);
        new.into_iter()
            .enumerate()
            .map(|(index, row)| RowInfo {
                reuse: matcher.take(&row.key, &row.item, index),
                ..row
            })
            .collect()
    }

    #[test]
    fn each_without_old_children_builds_the_new_ones() {
        let mut tree = WidgetTree::new(Widget::view(Direction::Column, Vec::new()));
        let absent = Widget::text("absent").id();
        let fresh = Widget::view(Direction::Column, vec![Widget::text("a")]);
        let new_id = fresh.id();
        let old_rows = [row(None, "a", 1)];
        let new_rows = matched(&old_rows, vec![row(None, "a", 1)]);
        let children = reconcile_each(&mut tree, absent, &old_rows, fresh, &new_rows);
        assert_eq!(children.len(), 1);
        assert_ne!(children[0].id(), new_id);
    }

    #[test]
    fn an_empty_branch_row_replaces_the_items() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let mut tree = WidgetTree::new(root);
        let old_rows = [row(None, "a", 1)];
        let new_rows = matched(
            &old_rows,
            vec![RowInfo {
                key: RowKey::Index(0),
                item: Value::Array(std::sync::Arc::new(Vec::new())),
                start: 0,
                count: 1,
                reuse: None,
            }],
        );
        let children = reconcile_each(
            &mut tree,
            parent,
            &old_rows,
            Widget::view(Direction::Column, vec![Widget::text("empty")]),
            &new_rows,
        );
        assert_eq!(children.len(), 1);
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn an_unkeyed_row_reuses_the_widget_in_the_same_place() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let mut tree = WidgetTree::new(root);
        let old_rows = [row(None, "a", 1)];
        let new_rows = matched(&old_rows, vec![row(None, "a", 1)]);
        let children = reconcile_each(
            &mut tree,
            parent,
            &old_rows,
            Widget::view(Direction::Column, vec![Widget::text("a")]),
            &new_rows,
        );
        assert_eq!(children[0].id(), first_id);
    }

    #[test]
    fn an_unkeyed_row_with_a_different_item_is_rebuilt() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let mut tree = WidgetTree::new(root);
        let old_rows = [row(None, "a", 1)];
        let new_rows = matched(&old_rows, vec![row(None, "b", 1)]);
        let children = reconcile_each(
            &mut tree,
            parent,
            &old_rows,
            Widget::view(Direction::Column, vec![Widget::text("b")]),
            &new_rows,
        );
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn a_keyed_row_is_rebuilt_when_its_item_changes() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let mut tree = WidgetTree::new(root);
        let key = Some(Value::Number(1.0));
        let old_rows = [row(key.clone(), "a", 1)];
        let new_rows = matched(&old_rows, vec![row(key, "b", 1)]);
        let children = reconcile_each(
            &mut tree,
            parent,
            &old_rows,
            Widget::view(Direction::Column, vec![Widget::text("b")]),
            &new_rows,
        );
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn a_keyed_row_takes_its_widget_only_once() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let mut tree = WidgetTree::new(root);
        let key = Some(Value::Number(1.0));
        let fresh = Widget::view(
            Direction::Column,
            vec![Widget::text("a"), Widget::text("a")],
        );
        let old_rows = [row(key.clone(), "a", 1)];
        let new_rows = matched(&old_rows, vec![row(key.clone(), "a", 1), row(key, "a", 1)]);
        let children = reconcile_each(&mut tree, parent, &old_rows, fresh, &new_rows);
        assert_eq!(children[0].id(), first_id);
        assert_ne!(children[1].id(), first_id);
    }

    #[test]
    fn a_keyed_row_does_not_take_an_unkeyed_widget() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let mut tree = WidgetTree::new(root);
        let old_rows = [row(None, "a", 1)];
        let new_rows = matched(&old_rows, vec![row(Some(Value::Number(1.0)), "a", 1)]);
        let children = reconcile_each(
            &mut tree,
            parent,
            &old_rows,
            Widget::view(Direction::Column, vec![Widget::text("a")]),
            &new_rows,
        );
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn each_reuses_rows_with_the_same_key() {
        let first = Widget::text("a");
        let second = Widget::text("b");
        let (first_id, second_id) = (first.id(), second.id());
        let root = Widget::view(Direction::Column, vec![first, second]);
        let parent = root.id();
        let mut tree = WidgetTree::new(root);
        let row = |key: f64, item: &str, start: usize| RowInfo {
            key: RowKey::Explicit(Value::Number(key)),
            item: Value::String(item.into()),
            start,
            count: 1,
            reuse: None,
        };
        let old_rows = [row(1.0, "a", 0), row(2.0, "b", 1)];
        let new_rows = matched(&old_rows, vec![row(2.0, "b", 0), row(1.0, "a", 1)]);
        let fresh = Widget::view(
            Direction::Column,
            vec![Widget::text("b"), Widget::text("a")],
        );

        let children = reconcile_each(&mut tree, parent, &old_rows, fresh, &new_rows);
        let ids: Vec<WidgetId> = children.iter().map(Widget::id).collect();
        assert_eq!(ids, vec![second_id, first_id]);
    }

    #[test]
    fn a_matcher_gives_a_keyed_row_the_same_place_after_a_move() {
        let old = [row(Some(Value::Number(1.0)), "a", 0)];
        let mut matcher = RowMatcher::new(&old);
        let key = RowKey::Explicit(Value::Number(1.0));
        assert_eq!(matcher.take(&key, &Value::String("a".into()), 3), Some(0));
    }

    #[test]
    fn a_matcher_ignores_the_position_of_a_keyed_row() {
        let old = [
            row(Some(Value::Number(1.0)), "a", 0),
            row(Some(Value::Number(2.0)), "b", 1),
        ];
        let mut matcher = RowMatcher::new(&old);
        let key = RowKey::Explicit(Value::Number(2.0));
        assert_eq!(matcher.take(&key, &Value::String("b".into()), 0), Some(1));
        assert_eq!(matcher.take(&key, &Value::String("b".into()), 1), None);
    }

    #[test]
    fn a_matcher_needs_both_the_key_and_the_item_to_match() {
        let old = [row(Some(Value::Number(1.0)), "a", 0)];
        let mut matcher = RowMatcher::new(&old);
        let key = RowKey::Explicit(Value::Number(1.0));
        assert_eq!(matcher.take(&key, &Value::String("b".into()), 0), None);
    }

    #[test]
    fn a_matcher_keeps_unkeyed_rows_in_place() {
        let old = [row(None, "a", 0), row(None, "b", 1)];
        let mut matcher = RowMatcher::new(&old);
        assert_eq!(
            matcher.take(&RowKey::Index(1), &Value::String("b".into()), 1),
            Some(1)
        );
        assert_eq!(
            matcher.take(&RowKey::Index(0), &Value::String("a".into()), 0),
            Some(0)
        );
        assert_eq!(
            matcher.take(&RowKey::Index(0), &Value::String("a".into()), 0),
            None
        );
    }
}

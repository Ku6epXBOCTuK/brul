use std::cell::RefCell;

use brul_gui::{Widget, WidgetId, WidgetTree};

use crate::RowInfo;
pub(crate) fn reconcile_each(
    tree: &RefCell<WidgetTree>,
    parent: WidgetId,
    old_rows: &[RowInfo],
    new_widget: Widget,
    new_rows: &[RowInfo],
) -> Vec<Widget> {
    let Some(old_children) = tree.borrow_mut().take_children(parent) else {
        return new_widget.into_children();
    };
    let mut old_widgets = group(old_children, old_rows);
    let mut new_widgets = group(new_widget.into_children(), new_rows);

    let mut used = vec![false; old_widgets.len()];
    let mut children = Vec::new();
    for (index, row) in new_rows.iter().enumerate() {
        match reuse_index(old_rows, row, index, &used) {
            Some(other) => {
                used[other] = true;
                children.append(&mut old_widgets[other]);
            }
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

fn reuse_index(old_rows: &[RowInfo], row: &RowInfo, index: usize, used: &[bool]) -> Option<usize> {
    let Some(key) = &row.key else {
        let same_place = index < old_rows.len()
            && !used[index]
            && old_rows[index].key.is_none()
            && old_rows[index].item == row.item;
        return same_place.then_some(index);
    };
    old_rows.iter().enumerate().position(|(other, old)| {
        !used[other] && old.key.as_ref() == Some(key) && old.item == row.item
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use brul_gui::Direction;

    use crate::Value;

    fn row(key: Option<Value>, item: &str, count: usize) -> RowInfo {
        RowInfo {
            key,
            item: Value::String(item.to_owned()),
            start: 0,
            count,
        }
    }

    #[test]
    fn each_without_old_children_builds_the_new_ones() {
        let tree = RefCell::new(WidgetTree::new(Widget::view(Direction::Column, Vec::new())));
        let absent = Widget::text("absent").id();
        let fresh = Widget::view(Direction::Column, vec![Widget::text("a")]);
        let new_id = fresh.id();
        let children = reconcile_each(
            &tree,
            absent,
            &[row(None, "a", 1)],
            fresh,
            &[row(None, "a", 1)],
        );
        assert_eq!(children.len(), 1);
        assert_ne!(children[0].id(), new_id);
    }

    #[test]
    fn an_unkeyed_row_reuses_the_widget_in_the_same_place() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(None, "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("a")]),
            &[row(None, "a", 1)],
        );
        assert_eq!(children[0].id(), first_id);
    }

    #[test]
    fn an_unkeyed_row_with_a_different_item_is_rebuilt() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(None, "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("b")]),
            &[row(None, "b", 1)],
        );
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn a_keyed_row_is_rebuilt_when_its_item_changes() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let key = Some(Value::Number(1.0));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(key.clone(), "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("b")]),
            &[row(key, "b", 1)],
        );
        assert_ne!(children[0].id(), first_id);
    }

    #[test]
    fn a_keyed_row_takes_its_widget_only_once() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let key = Some(Value::Number(1.0));
        let fresh = Widget::view(
            Direction::Column,
            vec![Widget::text("a"), Widget::text("a")],
        );
        let children = reconcile_each(
            &tree,
            parent,
            &[row(key.clone(), "a", 1)],
            fresh,
            &[row(key.clone(), "a", 1), row(key, "a", 1)],
        );
        assert_eq!(children[0].id(), first_id);
        assert_ne!(children[1].id(), first_id);
    }

    #[test]
    fn a_keyed_row_does_not_take_an_unkeyed_widget() {
        let first = Widget::text("a");
        let first_id = first.id();
        let root = Widget::view(Direction::Column, vec![first]);
        let parent = root.id();
        let tree = RefCell::new(WidgetTree::new(root));
        let children = reconcile_each(
            &tree,
            parent,
            &[row(None, "a", 1)],
            Widget::view(Direction::Column, vec![Widget::text("a")]),
            &[row(Some(Value::Number(1.0)), "a", 1)],
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
        let tree = RefCell::new(WidgetTree::new(root));
        let row = |key: f64, item: &str, start: usize| RowInfo {
            key: Some(Value::Number(key)),
            item: Value::String(item.to_owned()),
            start,
            count: 1,
        };
        let old_rows = [row(1.0, "a", 0), row(2.0, "b", 1)];
        let new_rows = [row(2.0, "b", 0), row(1.0, "a", 1)];
        let fresh = Widget::view(
            Direction::Column,
            vec![Widget::text("b"), Widget::text("a")],
        );

        let children = reconcile_each(&tree, parent, &old_rows, fresh, &new_rows);
        let ids: Vec<WidgetId> = children.iter().map(Widget::id).collect();
        assert_eq!(ids, vec![second_id, first_id]);
    }
}

use brul_gui::{
    Anchor, AnchorAlign, AnchorSide, Dimension, LayoutStyle, Size, TextEngine, Widget, WidgetId,
    WidgetTree,
};

fn fixed(width: u32, height: u32) -> LayoutStyle {
    LayoutStyle {
        width: Dimension::Px(width),
        height: Dimension::Px(height),
        ..LayoutStyle::default()
    }
}

fn anchored(
    width: u32,
    height: u32,
    target: WidgetId,
    side: AnchorSide,
    gap: i32,
    align: AnchorAlign,
    flip: bool,
) -> Widget {
    Widget::text("tip")
        .with_layout(LayoutStyle {
            anchor_side: side,
            anchor_gap: gap,
            anchor_align: align,
            anchor_flip: flip,
            ..fixed(width, height)
        })
        .anchored(Anchor::new(target))
}

fn layout(mut tree: WidgetTree) -> WidgetTree {
    let mut engine = TextEngine::new();
    tree.layout(
        &mut engine,
        Size {
            width: 100,
            height: 100,
        },
        1.0,
    )
    .expect("layout");
    tree
}

#[test]
fn an_anchor_places_a_widget_below_and_centered_on_its_target() {
    let target = Widget::text("target").with_layout(fixed(40, 20));
    let target_id = target.id();
    let overlay = anchored(
        30,
        10,
        target_id,
        AnchorSide::Bottom,
        6,
        AnchorAlign::Center,
        false,
    );
    let overlay_id = overlay.id();
    let tree = layout(WidgetTree::new(Widget::column(vec![target, overlay])));
    let rect = tree.rect(overlay_id).expect("overlay rect");
    assert_eq!((rect.x, rect.y, rect.width, rect.height), (5, 26, 30, 10));
}

#[test]
fn an_anchor_flips_when_the_preferred_side_overflows() {
    let spacer = Widget::text("spacer").with_layout(fixed(40, 70));
    let target = Widget::text("target").with_layout(fixed(40, 20));
    let target_id = target.id();
    let overlay = Widget::text("tip")
        .with_layout(LayoutStyle {
            anchor_side: AnchorSide::Bottom,
            anchor_gap: 6,
            ..fixed(30, 10)
        })
        .anchored(Anchor::new(target_id));
    let overlay_id = overlay.id();
    let tree = layout(WidgetTree::new(Widget::column(vec![
        spacer, target, overlay,
    ])));
    let rect = tree.rect(overlay_id).expect("overlay rect");
    assert_eq!((rect.x, rect.y), (5, 54));
}

#[test]
fn an_anchor_moves_the_whole_subtree() {
    let target = Widget::text("target").with_layout(fixed(40, 20));
    let target_id = target.id();
    let child = Widget::text("child").with_layout(fixed(10, 10));
    let child_id = child.id();
    let overlay = Widget::row(vec![child])
        .with_layout(LayoutStyle {
            anchor_side: AnchorSide::Bottom,
            anchor_gap: 6,
            anchor_align: AnchorAlign::Start,
            anchor_flip: false,
            ..fixed(30, 10)
        })
        .anchored(Anchor::new(target_id));
    let tree = layout(WidgetTree::new(Widget::column(vec![target, overlay])));
    let rect = tree.rect(child_id).expect("child rect");
    assert_eq!((rect.x, rect.y), (0, 26));
}

#[test]
fn repeated_instances_resolve_their_own_target() {
    let target_id = WidgetId::new(1);
    let first_id = WidgetId::new(2);
    let second_id = WidgetId::new(3);
    let first = Widget::column(vec![
        Widget::text("target")
            .with_id(target_id)
            .with_layout(fixed(40, 20)),
        anchored(
            30,
            10,
            target_id,
            AnchorSide::Bottom,
            6,
            AnchorAlign::Center,
            false,
        )
        .with_id(first_id),
    ]);
    let second = Widget::column(vec![
        Widget::text("target")
            .with_id(target_id)
            .with_layout(fixed(40, 20)),
        anchored(
            30,
            10,
            target_id,
            AnchorSide::Bottom,
            6,
            AnchorAlign::Center,
            false,
        )
        .with_id(second_id),
    ]);
    let tree = layout(WidgetTree::new(Widget::column(vec![first, second])));
    let first_rect = tree.rect(first_id).expect("first overlay rect");
    let second_rect = tree.rect(second_id).expect("second overlay rect");
    assert_eq!((first_rect.x, first_rect.y), (5, 26));
    assert_eq!((second_rect.x, second_rect.y), (5, 56));
}

#[test]
fn a_target_in_a_sibling_branch_is_found() {
    let target_id = WidgetId::new(1);
    let overlay_id = WidgetId::new(2);
    let overlay = anchored(
        20,
        10,
        target_id,
        AnchorSide::Bottom,
        6,
        AnchorAlign::Center,
        false,
    )
    .with_id(overlay_id);
    let target = Widget::text("target")
        .with_id(target_id)
        .with_layout(fixed(40, 20));
    let tree = layout(WidgetTree::new(Widget::column(vec![
        Widget::column(vec![overlay]),
        Widget::column(vec![target]),
    ])));
    let target_rect = tree.rect(target_id).expect("target rect");
    let rect = tree.rect(overlay_id).expect("overlay rect");
    let center = (target_rect.width as i32 - rect.width as i32) / 2;
    assert_eq!(rect.x, target_rect.x + center);
    assert_eq!(rect.y, target_rect.y + target_rect.height as i32 + 6);
}

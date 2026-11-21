use super::*;
use crate::*;

#[test]
fn grid_tracks_reach_taffy() {
    let style = LayoutStyle {
        display: Display::Grid,
        grid_template_columns: vec![
            TrackSize::Px(60.0),
            TrackSize::Fr(2.0),
            TrackSize::Minmax(Box::new(TrackSize::Px(10.0)), Box::new(TrackSize::Px(80.0))),
            TrackSize::Percent(25.0),
        ],
        grid_template_rows: vec![TrackSize::Fr(1.0)],
        grid_auto_flow: GridAutoFlow::Column,
        ..LayoutStyle::default()
    };
    let taffy = taffy_style(&style, Direction::Row, None, false);

    assert_eq!(taffy.display, TaffyDisplay::Grid);
    assert_eq!(taffy.grid_auto_flow, TaffyAutoFlow::Column);
    assert_eq!(taffy.grid_template_columns.len(), 4);
    assert_eq!(taffy.grid_template_rows.len(), 1);
    let Some(GridTemplateComponent::Single(first)) = taffy.grid_template_columns.first() else {
        panic!("a track component");
    };
    assert_eq!(first.min, MinTrackSizingFunction::length(60.0));
    assert_eq!(first.max, MaxTrackSizingFunction::length(60.0));
    let Some(GridTemplateComponent::Single(flexible)) = taffy.grid_template_columns.get(1) else {
        panic!("a track component");
    };
    assert_eq!(flexible.min, MinTrackSizingFunction::auto());
    assert_eq!(flexible.max, MaxTrackSizingFunction::fr(2.0));
}

#[test]
fn grid_repeat_tracks_reach_taffy() {
    let style = LayoutStyle {
        display: Display::Grid,
        grid_template_columns: vec![TrackSize::Repeat(
            RepeatCount::Count(2),
            vec![TrackSize::Px(40.0), TrackSize::Fr(1.0)],
        )],
        grid_template_rows: vec![TrackSize::Px(20.0)],
        ..LayoutStyle::default()
    };
    let taffy = taffy_style(&style, Direction::Row, None, false);
    let Some(GridTemplateComponent::Repeat(repeat)) = taffy.grid_template_columns.first() else {
        panic!("a repeat component");
    };
    assert_eq!(repeat.count, TaffyRepetitionCount::Count(2));
    assert_eq!(repeat.tracks.len(), 2);
    assert_eq!(repeat.tracks[0].max, MaxTrackSizingFunction::length(40.0));
    assert_eq!(repeat.line_names.len(), 3);

    let style = LayoutStyle {
        display: Display::Grid,
        width: Dimension::Px(200),
        grid_template_columns: vec![TrackSize::Repeat(
            RepeatCount::Count(2),
            vec![TrackSize::Px(80.0)],
        )],
        grid_template_rows: vec![TrackSize::Px(10.0)],
        ..LayoutStyle::default()
    };
    let grid = LayoutNode::row(vec![LayoutNode::leaf(10, 10), LayoutNode::leaf(10, 10)])
        .with_style(style.clone());
    let placed = layout(
        &grid,
        Size {
            width: 200,
            height: 200,
        },
    )
    .expect("repeat grid");
    assert_eq!((placed.children[0].x, placed.children[1].x), (0, 80));

    let style = LayoutStyle {
        grid_template_columns: vec![TrackSize::Repeat(
            RepeatCount::AutoFill,
            vec![TrackSize::Px(80.0)],
        )],
        ..style
    };
    let grid =
        LayoutNode::row(vec![LayoutNode::leaf(10, 10), LayoutNode::leaf(10, 10)]).with_style(style);
    let placed = layout(
        &grid,
        Size {
            width: 200,
            height: 200,
        },
    )
    .expect("auto-fill grid");
    assert_eq!((placed.children[0].x, placed.children[1].x), (0, 80));
}

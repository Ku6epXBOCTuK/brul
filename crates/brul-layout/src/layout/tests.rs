use super::*;
use crate::*;

#[test]
fn one_shot_forwards_insets_scroll_and_offset() {
    let viewport = Size {
        width: 200,
        height: 120,
    };
    let absolute =
        LayoutNode::row(vec![LayoutNode::leaf(10, 10).with_offset(5, 3)]).with_style(LayoutStyle {
            border_width: BorderWidths::all(2),
            padding: Padding {
                left: 10,
                top: 4,
                right: 1,
                bottom: 1,
            },
            ..LayoutStyle::default()
        });
    let placed = layout(&absolute, viewport).expect("layout");
    assert_eq!((placed.children[0].x, placed.children[0].y), (17, 9));

    let flowing = |scroll| {
        LayoutNode::row(vec![LayoutNode::leaf(10, 10), LayoutNode::leaf(10, 10)])
            .with_style(LayoutStyle {
                width: Dimension::Px(200),
                ..LayoutStyle::default()
            })
            .with_scroll(scroll)
    };
    let plain = layout(&flowing(ScrollOffset::ZERO), viewport).expect("layout");
    let shifted = layout(&flowing(ScrollOffset::new(7)), viewport).expect("layout");
    assert_eq!(plain.children[1].x - shifted.children[1].x, 7);
}

fn columns(grow: f32, basis: Dimension) -> LayoutNode {
    let style = LayoutStyle {
        flex_grow: grow,
        flex_basis: basis,
        ..LayoutStyle::default()
    };
    let root = LayoutStyle {
        width: Dimension::Px(200),
        gap: 8,
        ..LayoutStyle::default()
    };
    LayoutNode::row(vec![
        LayoutNode::leaf(20, 10).with_style(style.clone()),
        LayoutNode::leaf(40, 10).with_style(style),
    ])
    .with_style(root)
}

#[test]
fn flex_zero_basis_splits_the_whole_row() {
    let viewport = Size {
        width: 200,
        height: 50,
    };
    let placed = layout(&columns(1.0, Dimension::Px(0)), viewport).expect("layout");

    assert_eq!(placed.children[0].width, 96);
    assert_eq!(placed.children[1].width, 96);
    assert_eq!(placed.children[1].x, 104);
}

#[test]
fn flex_without_basis_grows_from_content() {
    let viewport = Size {
        width: 200,
        height: 50,
    };
    let placed = layout(&columns(1.0, Dimension::Auto), viewport).expect("layout");

    assert_eq!(placed.children[0].width, 86);
    assert_eq!(placed.children[1].width, 106);
}

#[test]
fn percent_child_of_a_flex_column_takes_the_column_width() {
    let column = || {
        let child = LayoutNode::leaf(0, 30).with_style(LayoutStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(30),
            ..LayoutStyle::default()
        });
        LayoutNode::column(vec![child]).with_style(LayoutStyle {
            flex_grow: 1.0,
            flex_basis: Dimension::Px(0),
            ..LayoutStyle::default()
        })
    };
    let root = LayoutNode::row(vec![column(), column()]).with_style(LayoutStyle {
        width: Dimension::Px(480),
        gap: 10,
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 480,
            height: 200,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[0].width, 235);
    assert_eq!(placed.children[1].x, 245);
    assert_eq!(placed.children[0].children[0].width, 235);
    assert_eq!(placed.children[1].children[0].width, 235);
}

#[test]
fn grid_children_obey_explicit_lines() {
    let grid = LayoutNode::row(vec![
        LayoutNode::leaf(10, 10).with_style(LayoutStyle {
            grid_column: GridLine {
                start: LinePlacement::Line(2),
                end: None,
            },
            grid_row: GridLine {
                start: LinePlacement::Line(1),
                end: None,
            },
            ..LayoutStyle::default()
        }),
        LayoutNode::leaf(10, 10).with_style(LayoutStyle {
            grid_column: GridLine {
                start: LinePlacement::Line(1),
                end: Some(LinePlacement::Line(3)),
            },
            grid_row: GridLine {
                start: LinePlacement::Line(2),
                end: None,
            },
            ..LayoutStyle::default()
        }),
        LayoutNode::leaf(10, 10).with_style(LayoutStyle {
            grid_column: GridLine {
                start: LinePlacement::Line(-2),
                end: None,
            },
            grid_row: GridLine {
                start: LinePlacement::Line(-2),
                end: None,
            },
            ..LayoutStyle::default()
        }),
        LayoutNode::leaf(10, 10),
    ])
    .with_style(LayoutStyle {
        display: Display::Grid,
        width: Dimension::Px(200),
        grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
        grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
        ..LayoutStyle::default()
    });

    let placed = layout(
        &grid,
        Size {
            width: 200,
            height: 200,
        },
    )
    .expect("grid placement");

    assert_eq!((placed.children[0].x, placed.children[0].y), (80, 0));
    assert_eq!((placed.children[1].x, placed.children[1].y), (0, 10));
    assert_eq!(placed.children[1].width, 160);
    assert_eq!((placed.children[2].x, placed.children[2].y), (80, 10));
    assert_eq!((placed.children[3].x, placed.children[3].y), (0, 0));
}

#[test]
fn grid_span_placement_covers_its_tracks() {
    let grid = LayoutNode::row(vec![
        LayoutNode::leaf(10, 10).with_style(LayoutStyle {
            grid_column: GridLine {
                start: LinePlacement::Span(2),
                end: None,
            },
            ..LayoutStyle::default()
        }),
        LayoutNode::leaf(10, 10),
    ])
    .with_style(LayoutStyle {
        display: Display::Grid,
        width: Dimension::Px(200),
        grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
        grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
        ..LayoutStyle::default()
    });

    let placed = layout(
        &grid,
        Size {
            width: 200,
            height: 200,
        },
    )
    .expect("grid span");

    assert_eq!(placed.children[0].width, 160);
    assert_eq!((placed.children[1].x, placed.children[1].y), (0, 10));
}

#[test]
fn grid_auto_flow_places_children() {
    let grid = |flow| {
        LayoutNode::row(vec![
            LayoutNode::leaf(10, 10),
            LayoutNode::leaf(10, 10),
            LayoutNode::leaf(10, 10),
            LayoutNode::leaf(10, 10),
        ])
        .with_style(LayoutStyle {
            display: Display::Grid,
            width: Dimension::Px(200),
            grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
            grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
            grid_auto_flow: flow,
            ..LayoutStyle::default()
        })
    };
    let viewport = Size {
        width: 200,
        height: 200,
    };

    let row = layout(&grid(GridAutoFlow::Row), viewport).expect("row flow");
    assert_eq!((row.children[1].x, row.children[1].y), (80, 0));
    assert_eq!((row.children[2].x, row.children[2].y), (0, 10));

    let column = layout(&grid(GridAutoFlow::Column), viewport).expect("column flow");
    assert_eq!((column.children[1].x, column.children[1].y), (0, 10));
    assert_eq!((column.children[2].x, column.children[2].y), (80, 0));
}

#[test]
fn per_axis_gaps_apply_to_their_own_axes() {
    let grid = LayoutNode::row(vec![
        LayoutNode::leaf(10, 10),
        LayoutNode::leaf(10, 10),
        LayoutNode::leaf(10, 10),
        LayoutNode::leaf(10, 10),
    ])
    .with_style(LayoutStyle {
        display: Display::Grid,
        width: Dimension::Px(200),
        grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
        grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
        row_gap: 5,
        column_gap: 7,
        ..LayoutStyle::default()
    });

    let placed = layout(
        &grid,
        Size {
            width: 200,
            height: 200,
        },
    )
    .expect("per-axis gaps");

    assert_eq!((placed.children[1].x, placed.children[1].y), (87, 0));
    assert_eq!((placed.children[2].x, placed.children[2].y), (0, 15));
}

#[test]
fn grid_named_areas_place_their_children() {
    let area = |name: &str| {
        LayoutNode::leaf(10, 10).with_style(LayoutStyle {
            grid_area: name.into(),
            ..LayoutStyle::default()
        })
    };
    let grid =
        LayoutNode::row(vec![area("head"), area("side"), area("main")]).with_style(LayoutStyle {
            display: Display::Grid,
            width: Dimension::Px(200),
            grid_template_columns: vec![TrackSize::Px(80.0), TrackSize::Px(80.0)],
            grid_template_rows: vec![TrackSize::Px(10.0), TrackSize::Px(10.0)],
            grid_template_areas: AreaTemplate::parse(&["head head".into(), "side main".into()])
                .expect("area template"),
            ..LayoutStyle::default()
        });

    let placed = layout(
        &grid,
        Size {
            width: 200,
            height: 200,
        },
    )
    .expect("named areas");

    assert_eq!((placed.children[0].x, placed.children[0].y), (0, 0));
    assert_eq!(placed.children[0].width, 160);
    assert_eq!((placed.children[1].x, placed.children[1].y), (0, 10));
    assert_eq!((placed.children[2].x, placed.children[2].y), (80, 10));
}

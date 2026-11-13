use brul_gui::{
    Align, Dimension, Justify, LayoutError, LayoutNode, LayoutStyle, Padding, Size, layout,
};

#[test]
fn column_stacks_with_padding_and_spacing() {
    let root = LayoutNode::column(vec![LayoutNode::leaf(50, 20), LayoutNode::leaf(50, 20)])
        .with_style(LayoutStyle {
            width: Dimension::Px(200),
            height: Dimension::Px(300),
            padding: Padding::all(10),
            spacing: 5,
            ..LayoutStyle::default()
        });

    let placed = layout(
        &root,
        Size {
            width: 200,
            height: 300,
        },
    )
    .expect("layout");

    assert_eq!((placed.width, placed.height), (200, 300));
    assert_eq!((placed.children[0].x, placed.children[0].y), (10, 10));
    assert_eq!((placed.children[1].x, placed.children[1].y), (10, 35));
}

#[test]
fn an_empty_child_takes_no_spacing() {
    let root = LayoutNode::column(vec![
        LayoutNode::leaf(50, 20),
        LayoutNode::column(Vec::new()),
        LayoutNode::leaf(50, 20),
    ])
    .with_style(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(100),
        spacing: 5,
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 100,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[0].y, 0);
    assert_eq!(placed.children[2].y, 25);

    let leading = LayoutNode::column(vec![
        LayoutNode::column(Vec::new()),
        LayoutNode::leaf(50, 20),
    ])
    .with_style(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(100),
        spacing: 5,
        ..LayoutStyle::default()
    });

    let placed = layout(
        &leading,
        Size {
            width: 100,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[1].y, 0);
}

#[test]
fn percent_width_uses_parent_content_width() {
    let second = LayoutNode::leaf(0, 30).with_style(LayoutStyle {
        width: Dimension::Percent(50.0),
        height: Dimension::Px(30),
        ..LayoutStyle::default()
    });
    let root = LayoutNode::row(vec![LayoutNode::leaf(40, 20), second]).with_style(LayoutStyle {
        width: Dimension::Px(200),
        height: Dimension::Px(50),
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 200,
            height: 50,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[1].width, 100);
    assert_eq!(placed.children[1].x, 40);
}

#[test]
fn percent_width_inside_auto_container_uses_available_width() {
    let child = LayoutNode::leaf(0, 20).with_style(LayoutStyle {
        width: Dimension::Percent(100.0),
        ..LayoutStyle::default()
    });
    let wrapper = LayoutNode::column(vec![child]);
    let root = LayoutNode::column(vec![wrapper]).with_style(LayoutStyle {
        width: Dimension::Px(300),
        padding: Padding::all(10),
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 300,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[0].width, 280);
    assert_eq!(placed.children[0].children[0].width, 280);
}

#[test]
fn percent_height_inside_auto_container_uses_available_height() {
    let child = LayoutNode::leaf(40, 0).with_style(LayoutStyle {
        height: Dimension::Percent(50.0),
        ..LayoutStyle::default()
    });
    let wrapper = LayoutNode::column(vec![child]);
    let root = LayoutNode::column(vec![wrapper]).with_style(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(200),
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 100,
            height: 200,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[0].height, 100);
}

#[test]
fn stretch_distributes_remaining_main_space() {
    let first = LayoutNode::leaf(50, 10);
    let second = LayoutNode::leaf(50, 10).with_style(LayoutStyle {
        stretch: 1.0,
        ..LayoutStyle::default()
    });
    let third = LayoutNode::leaf(50, 10).with_style(LayoutStyle {
        stretch: 2.0,
        ..LayoutStyle::default()
    });
    let root = LayoutNode::row(vec![first, second, third]).with_style(LayoutStyle {
        width: Dimension::Px(300),
        height: Dimension::Px(40),
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 300,
            height: 40,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[0].width, 50);
    assert_eq!(placed.children[1].width, 100);
    assert_eq!(placed.children[2].width, 150);
    assert_eq!(placed.children[1].x, 50);
    assert_eq!(placed.children[2].x, 150);
}

#[test]
fn justify_and_align_center_single_child() {
    let root = LayoutNode::row(vec![LayoutNode::leaf(50, 20)]).with_style(LayoutStyle {
        width: Dimension::Px(200),
        height: Dimension::Px(100),
        justify: Justify::Center,
        align: Align::Center,
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 200,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!((placed.children[0].x, placed.children[0].y), (75, 40));
}

#[test]
fn space_between_spreads_flow_children() {
    let root = LayoutNode::row(vec![LayoutNode::leaf(50, 10), LayoutNode::leaf(50, 10)])
        .with_style(LayoutStyle {
            width: Dimension::Px(200),
            height: Dimension::Px(40),
            justify: Justify::SpaceBetween,
            ..LayoutStyle::default()
        });

    let placed = layout(
        &root,
        Size {
            width: 200,
            height: 40,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[0].x, 0);
    assert_eq!(placed.children[1].x, 150);
}

#[test]
fn absolute_child_is_out_of_flow() {
    let absolute = LayoutNode::leaf(30, 30).with_offset(5, 7);
    let root =
        LayoutNode::column(vec![LayoutNode::leaf(50, 20), absolute]).with_style(LayoutStyle {
            width: Dimension::Px(100),
            height: Dimension::Auto,
            padding: Padding::all(10),
            ..LayoutStyle::default()
        });

    let placed = layout(
        &root,
        Size {
            width: 100,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!(placed.height, 40);
    assert_eq!((placed.children[0].x, placed.children[0].y), (10, 10));
    assert_eq!((placed.children[1].x, placed.children[1].y), (15, 17));
}

#[test]
fn auto_parent_fits_content() {
    let root = LayoutNode::column(vec![LayoutNode::leaf(30, 10), LayoutNode::leaf(30, 10)])
        .with_style(LayoutStyle {
            padding: Padding::all(5),
            spacing: 2,
            ..LayoutStyle::default()
        });

    let placed = layout(
        &root,
        Size {
            width: 100,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!((placed.width, placed.height), (40, 32));
    assert_eq!((placed.children[0].x, placed.children[0].y), (5, 5));
    assert_eq!((placed.children[1].x, placed.children[1].y), (5, 17));
}

#[test]
fn fixed_dimension_overrides_intrinsic_size() {
    let root = LayoutNode::leaf(99, 99).with_style(LayoutStyle {
        width: Dimension::Px(10),
        height: Dimension::Px(10),
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 100,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!((placed.width, placed.height), (10, 10));
}

#[test]
fn align_stretch_fills_cross_axis() {
    let root = LayoutNode::row(vec![LayoutNode::leaf(20, 10)]).with_style(LayoutStyle {
        width: Dimension::Px(100),
        height: Dimension::Px(80),
        padding: Padding::all(10),
        align: Align::Stretch,
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 100,
            height: 80,
        },
    )
    .expect("layout");

    assert_eq!(placed.children[0].height, 60);
    assert_eq!((placed.children[0].x, placed.children[0].y), (10, 10));
}

#[test]
fn border_and_padding_are_inside_size() {
    let root = LayoutNode::column(vec![LayoutNode::leaf(10, 10)]).with_style(LayoutStyle {
        padding: Padding::all(5),
        border_width: 2,
        ..LayoutStyle::default()
    });

    let placed = layout(
        &root,
        Size {
            width: 100,
            height: 100,
        },
    )
    .expect("layout");

    assert_eq!((placed.width, placed.height), (24, 24));
    assert_eq!((placed.children[0].x, placed.children[0].y), (7, 7));
}

#[test]
fn invalid_dimension_is_rejected() {
    let root = LayoutNode::leaf(10, 10).with_style(LayoutStyle {
        width: Dimension::Percent(f32::NAN),
        ..LayoutStyle::default()
    });

    assert_eq!(
        layout(
            &root,
            Size {
                width: 100,
                height: 100
            }
        ),
        Err(LayoutError::InvalidDimension)
    );
}

#[test]
fn invalid_stretch_is_rejected() {
    let root = LayoutNode::leaf(10, 10).with_style(LayoutStyle {
        stretch: -1.0,
        ..LayoutStyle::default()
    });

    assert_eq!(
        layout(
            &root,
            Size {
                width: 100,
                height: 100
            }
        ),
        Err(LayoutError::InvalidStretch)
    );
}

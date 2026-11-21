use crate::*;

#[test]
fn every_layout_error_has_its_own_message() {
    let messages = [
        LayoutError::InvalidDimension,
        LayoutError::InvalidFlex,
        LayoutError::InvalidGridLine,
    ]
    .map(|error| error.to_string());

    assert!(messages.iter().all(|message| !message.is_empty()));
    assert_ne!(messages[0], messages[1]);
}

#[test]
fn grid_children_reject_zero_lines() {
    let grid = LayoutNode::row(vec![LayoutNode::leaf(10, 10).with_style(LayoutStyle {
        grid_column: GridLine {
            start: LinePlacement::Line(0),
            end: None,
        },
        ..LayoutStyle::default()
    })]);

    assert_eq!(
        layout(
            &grid,
            Size {
                width: 200,
                height: 200,
            },
        ),
        Err(LayoutError::InvalidGridLine)
    );
}

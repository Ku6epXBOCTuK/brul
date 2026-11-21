use taffy::style_helpers::{TaffyAuto, TaffyGridLine};
use taffy::{
    AlignItems, BoxSizing, CheapCloneStr, Dimension as TaffyDimension, Display as TaffyDisplay,
    FlexDirection, GridAutoFlow as TaffyAutoFlow, GridPlacement, GridTemplateArea,
    GridTemplateAreas, GridTemplateComponent, GridTemplateRepetition, JustifyContent,
    LengthPercentage, LengthPercentageAuto, Line as TaffyLine, MaxTrackSizingFunction, MinMax,
    MinTrackSizingFunction, Position, Rect as TaffyRect, RepetitionCount as TaffyRepetitionCount,
    Size as TaffySize, Style as TaffyStyle,
};

use crate::{
    Align, AreaTemplate, Dimension, Direction, Display, GridAutoFlow, GridLine, Justify,
    LayoutStyle, LinePlacement, Offset, RepeatCount, TrackSize,
};

fn grid_placement(placement: LinePlacement) -> GridPlacement {
    match placement {
        LinePlacement::Auto => GridPlacement::Auto,
        LinePlacement::Line(value) => GridPlacement::from_line_index(value as i16),
        LinePlacement::Span(value) => GridPlacement::Span(value),
    }
}

fn grid_line(line: &GridLine) -> TaffyLine<GridPlacement> {
    TaffyLine {
        start: grid_placement(line.start),
        end: line.end.map_or(GridPlacement::Auto, grid_placement),
    }
}

fn min_track(size: &TrackSize) -> MinTrackSizingFunction {
    match size {
        TrackSize::Px(value) => MinTrackSizingFunction::length(*value as f32),
        TrackSize::Percent(value) => MinTrackSizingFunction::percent(*value as f32 / 100.0),
        TrackSize::Fr(_) => MinTrackSizingFunction::auto(),
        TrackSize::Minmax(min, _) => min_track(min),
        TrackSize::Repeat(..) => MinTrackSizingFunction::auto(),
    }
}

fn max_track(size: &TrackSize) -> MaxTrackSizingFunction {
    match size {
        TrackSize::Px(value) => MaxTrackSizingFunction::length(*value as f32),
        TrackSize::Percent(value) => MaxTrackSizingFunction::percent(*value as f32 / 100.0),
        TrackSize::Fr(value) => MaxTrackSizingFunction::fr(*value as f32),
        TrackSize::Minmax(_, max) => max_track(max),
        TrackSize::Repeat(..) => MaxTrackSizingFunction::auto(),
    }
}

fn repetition_count(count: &RepeatCount) -> TaffyRepetitionCount {
    match count {
        RepeatCount::Count(value) => TaffyRepetitionCount::Count(*value),
        RepeatCount::AutoFill => TaffyRepetitionCount::AutoFill,
        RepeatCount::AutoFit => TaffyRepetitionCount::AutoFit,
    }
}

fn track_component<S: CheapCloneStr>(size: &TrackSize) -> GridTemplateComponent<S> {
    match size {
        TrackSize::Repeat(count, tracks) => GridTemplateComponent::Repeat(GridTemplateRepetition {
            count: repetition_count(count),
            tracks: tracks
                .iter()
                .map(|track| MinMax {
                    min: min_track(track),
                    max: max_track(track),
                })
                .collect(),
            line_names: vec![vec![]; tracks.len() + 1],
        }),
        _ => GridTemplateComponent::Single(MinMax {
            min: min_track(size),
            max: max_track(size),
        }),
    }
}

fn taffy_areas(template: &AreaTemplate) -> Option<GridTemplateAreas<String>> {
    if template.rows.is_empty() {
        return None;
    }
    let areas = template
        .areas()
        .ok()?
        .into_iter()
        .map(|area| GridTemplateArea {
            name: area.name,
            row_start: area.row_start,
            row_end: area.row_end,
            column_start: area.column_start,
            column_end: area.column_end,
        })
        .collect();
    Some(GridTemplateAreas {
        areas,
        row_count: template.row_count(),
        column_count: template.column_count(),
    })
}

fn named_grid_line(name: &str) -> TaffyLine<GridPlacement> {
    TaffyLine {
        start: GridPlacement::NamedLine(name.to_owned(), 0),
        end: GridPlacement::NamedLine(name.to_owned(), 0),
    }
}

pub(crate) fn taffy_style(
    style: &LayoutStyle,
    direction: Direction,
    offset: Option<Offset>,
    constrained: bool,
) -> TaffyStyle {
    let length = |value: u32| LengthPercentage::length(value as f32);
    let mut output = TaffyStyle {
        box_sizing: BoxSizing::BorderBox,
        display: match style.display {
            Display::Flex => TaffyDisplay::Flex,
            Display::Grid => TaffyDisplay::Grid,
        },
        grid_template_columns: style
            .grid_template_columns
            .iter()
            .map(track_component)
            .collect(),
        grid_template_rows: style
            .grid_template_rows
            .iter()
            .map(track_component)
            .collect(),
        grid_auto_flow: match style.grid_auto_flow {
            GridAutoFlow::Row => TaffyAutoFlow::Row,
            GridAutoFlow::Column => TaffyAutoFlow::Column,
        },
        grid_template_areas: taffy_areas(&style.grid_template_areas),
        grid_column: if style.grid_area.is_empty() {
            grid_line(&style.grid_column)
        } else {
            named_grid_line(&style.grid_area)
        },
        grid_row: if style.grid_area.is_empty() {
            grid_line(&style.grid_row)
        } else {
            named_grid_line(&style.grid_area)
        },
        size: TaffySize {
            width: dimension(style.width),
            height: dimension(style.height),
        },
        padding: TaffyRect {
            left: length(style.padding.left),
            right: length(style.padding.right),
            top: length(style.padding.top),
            bottom: length(style.padding.bottom),
        },
        border: TaffyRect {
            left: length(style.border_width),
            right: length(style.border_width),
            top: length(style.border_width),
            bottom: length(style.border_width),
        },
        gap: TaffySize {
            width: length(style.column_gap()),
            height: length(style.row_gap()),
        },
        flex_direction: match direction {
            Direction::Row => FlexDirection::Row,
            Direction::Column => FlexDirection::Column,
        },
        align_items: match style.align {
            Align::Start => AlignItems::START,
            Align::Center => AlignItems::CENTER,
            Align::End => AlignItems::END,
            Align::Stretch => AlignItems::STRETCH,
        },
        justify_content: match style.justify {
            Justify::Start => JustifyContent::START,
            Justify::Center => JustifyContent::CENTER,
            Justify::End => JustifyContent::END,
            Justify::SpaceBetween => JustifyContent::SPACE_BETWEEN,
        },
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: dimension(style.flex_basis),
        ..TaffyStyle::default()
    };

    if let Some(offset) = offset {
        output.position = Position::Absolute;
        output.inset = TaffyRect {
            left: LengthPercentageAuto::length(offset.x as f32),
            right: LengthPercentageAuto::AUTO,
            top: LengthPercentageAuto::length(offset.y as f32),
            bottom: LengthPercentageAuto::AUTO,
        };
    }
    if constrained {
        match direction {
            Direction::Column => output.min_size.height = length(0).into(),
            Direction::Row => output.min_size.width = length(0).into(),
        }
        if style.flex_grow > 0.0 {
            output.flex_shrink = 1.0;
        }
    }

    output
}

fn dimension(value: Dimension) -> TaffyDimension {
    match value {
        Dimension::Auto => TaffyDimension::AUTO,
        Dimension::Px(value) => TaffyDimension::length(value as f32),
        Dimension::Percent(percent) => TaffyDimension::percent(percent / 100.0),
    }
}

#[cfg(test)]
mod tests;

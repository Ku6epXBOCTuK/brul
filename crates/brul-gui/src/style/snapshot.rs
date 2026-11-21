use std::time::Duration;

use brul_style::{LayoutField, Property, Side, Snapshot, StyleField, Target, TextField};

use super::StyleError;
use super::value::{
    AreaTemplateTy, ColorTy, FlagTy, GridAreaTy, GridLineTy, KeywordTy, LengthTy, NumberTy,
    PaintTy, PxTy, SignedPxTy, TextTy, TrackListTy, read,
};
use crate::widget::{TextStylePatch, Transitions, WidgetStyle, WidgetStylePatch};
use crate::{
    Align, Direction, Display, GridAutoFlow, Justify, LayoutStyle, Padding, TextAlign, TextStyle,
    VerticalAlign,
};

pub(super) fn base_styles(
    snapshot: &Snapshot,
) -> Result<(LayoutStyle, WidgetStyle, TextStyle), StyleError> {
    let mut layout = LayoutStyle::default();
    let mut style = WidgetStyle::default();
    let mut text = TextStyle::default();
    for property in snapshot.properties() {
        apply(property, &mut layout, &mut style, &mut text)?;
    }
    Ok((layout, style, text))
}

pub(super) fn state_patch(
    snapshot: &Snapshot,
) -> Result<(WidgetStylePatch, TextStylePatch), StyleError> {
    let mut style = WidgetStylePatch::default();
    let mut text = TextStylePatch::default();
    for property in snapshot.properties() {
        let kind = property.kind;
        let value = &property.value;
        match kind.target() {
            Target::Style(StyleField::Background) => {
                style.background = Some(read::<PaintTy>(value, kind)?)
            }
            Target::Style(StyleField::Border) => style.border = Some(read::<ColorTy>(value, kind)?),
            Target::Style(StyleField::Text) => style.text = Some(read::<ColorTy>(value, kind)?),
            Target::Style(StyleField::Tint) => style.tint = Some(read::<ColorTy>(value, kind)?),
            Target::Style(StyleField::Opacity) => {
                style.opacity = Some(read::<NumberTy>(value, kind)? as f32)
            }
            Target::Style(StyleField::Clip) => style.clip = Some(read::<FlagTy>(value, kind)?),
            Target::Style(StyleField::ShadowColor) => {
                style.shadow = Some(read::<ColorTy>(value, kind)?)
            }
            Target::Style(StyleField::BorderRadius) => {
                style.border_radius = Some(read::<PxTy>(value, kind)?)
            }
            Target::Style(StyleField::Scroll) => style.scroll = Some(read::<FlagTy>(value, kind)?),
            Target::Style(StyleField::ShadowX) => {
                style.shadow_x = Some(read::<SignedPxTy>(value, kind)?)
            }
            Target::Style(StyleField::ShadowY) => {
                style.shadow_y = Some(read::<SignedPxTy>(value, kind)?)
            }
            Target::Style(StyleField::ShadowBlur) => {
                style.shadow_blur = Some(read::<PxTy>(value, kind)?)
            }
            Target::Text(TextField::VerticalAlign) => {
                style.vertical_align = Some(vertical_align(read::<KeywordTy>(value, kind)?));
            }
            Target::Text(TextField::Align) => {
                text.align = Some(text_align(read::<KeywordTy>(value, kind)?));
            }
            Target::Text(TextField::Weight) => {
                text.weight = Some(read::<NumberTy>(value, kind)? as u16)
            }
            Target::Text(TextField::Size) => text.size = Some(read::<PxTy>(value, kind)? as f32),
            Target::Text(TextField::Family) => {
                text.family = Some(read::<TextTy>(value, kind)?.to_owned())
            }
            _ => {}
        }
    }
    Ok((style, text))
}

pub(super) fn transitions(snapshot: &Snapshot) -> Result<Transitions, StyleError> {
    let mut transitions = Transitions::default();
    for animation in snapshot.animations() {
        let field = match animation.property.target() {
            Target::Style(StyleField::Background) => &mut transitions.background,
            Target::Style(StyleField::Border) => &mut transitions.border,
            Target::Style(StyleField::Text) => &mut transitions.text,
            Target::Style(StyleField::Tint) => &mut transitions.tint,
            Target::Style(StyleField::Opacity) => &mut transitions.opacity,
            Target::Style(StyleField::ShadowColor) => &mut transitions.shadow,
            _ => {
                return Err(StyleError::Value {
                    kind: animation.property,
                    expected: "an animatable property",
                });
            }
        };
        *field = Some(Duration::from_millis(animation.duration.round() as u64));
    }
    Ok(transitions)
}

fn apply(
    property: &Property,
    layout: &mut LayoutStyle,
    style: &mut WidgetStyle,
    text: &mut TextStyle,
) -> Result<(), StyleError> {
    let kind = property.kind;
    let value = &property.value;
    match kind.target() {
        Target::Layout(LayoutField::Width) => layout.width = read::<LengthTy>(value, kind)?,
        Target::Layout(LayoutField::Height) => layout.height = read::<LengthTy>(value, kind)?,
        Target::Layout(LayoutField::FlexGrow) => {
            layout.flex_grow = read::<NumberTy>(value, kind)? as f32
        }
        Target::Layout(LayoutField::FlexShrink) => {
            layout.flex_shrink = read::<NumberTy>(value, kind)? as f32
        }
        Target::Layout(LayoutField::FlexBasis) => {
            layout.flex_basis = read::<LengthTy>(value, kind)?
        }
        Target::Layout(LayoutField::Display) => {
            layout.display = match read::<KeywordTy>(value, kind)? {
                "grid" => Display::Grid,
                _ => Display::Flex,
            }
        }
        Target::Layout(LayoutField::FlexDirection) => {
            layout.flex_direction = flex_direction(read::<KeywordTy>(value, kind)?)
        }
        Target::Layout(LayoutField::Padding(side)) => {
            *edge(&mut layout.padding, side) = read::<PxTy>(value, kind)?;
        }
        Target::Layout(LayoutField::Gap) => layout.gap = read::<PxTy>(value, kind)?,
        Target::Layout(LayoutField::RowGap) => layout.row_gap = read::<PxTy>(value, kind)?,
        Target::Layout(LayoutField::ColumnGap) => layout.column_gap = read::<PxTy>(value, kind)?,
        Target::Layout(LayoutField::BorderWidth) => {
            layout.border_width = read::<PxTy>(value, kind)?
        }
        Target::Layout(LayoutField::Justify) => {
            layout.justify = justify(read::<KeywordTy>(value, kind)?)
        }
        Target::Layout(LayoutField::GridTemplateColumns) => {
            layout.grid_template_columns = read::<TrackListTy>(value, kind)?
        }
        Target::Layout(LayoutField::GridTemplateRows) => {
            layout.grid_template_rows = read::<TrackListTy>(value, kind)?
        }
        Target::Layout(LayoutField::GridColumn) => {
            layout.grid_column = read::<GridLineTy>(value, kind)?
        }
        Target::Layout(LayoutField::GridRow) => layout.grid_row = read::<GridLineTy>(value, kind)?,
        Target::Layout(LayoutField::GridAutoFlow) => {
            layout.grid_auto_flow = match read::<KeywordTy>(value, kind)? {
                "column" => GridAutoFlow::Column,
                _ => GridAutoFlow::Row,
            }
        }
        Target::Layout(LayoutField::GridTemplateAreas) => {
            layout.grid_template_areas = read::<AreaTemplateTy>(value, kind)?
        }
        Target::Layout(LayoutField::GridArea) => {
            layout.grid_area = read::<GridAreaTy>(value, kind)?
        }
        Target::Layout(LayoutField::Align) => layout.align = align(read::<KeywordTy>(value, kind)?),
        Target::Style(StyleField::Background) => style.background = read::<PaintTy>(value, kind)?,
        Target::Style(StyleField::Border) => style.border = read::<ColorTy>(value, kind)?,
        Target::Style(StyleField::Text) => style.text = read::<ColorTy>(value, kind)?,
        Target::Style(StyleField::Tint) => style.tint = Some(read::<ColorTy>(value, kind)?),
        Target::Style(StyleField::BorderRadius) => style.border_radius = read::<PxTy>(value, kind)?,
        Target::Style(StyleField::Opacity) => style.opacity = read::<NumberTy>(value, kind)? as f32,
        Target::Style(StyleField::Clip) => style.clip = read::<FlagTy>(value, kind)?,
        Target::Style(StyleField::Scroll) => style.scroll = read::<FlagTy>(value, kind)?,
        Target::Style(StyleField::ShadowX) => style.shadow.x = read::<SignedPxTy>(value, kind)?,
        Target::Style(StyleField::ShadowY) => style.shadow.y = read::<SignedPxTy>(value, kind)?,
        Target::Style(StyleField::ShadowBlur) => style.shadow.blur = read::<PxTy>(value, kind)?,
        Target::Style(StyleField::ShadowColor) => {
            style.shadow.color = read::<ColorTy>(value, kind)?
        }
        Target::Text(TextField::Size) => {
            text.font_size = read::<PxTy>(value, kind)? as f32;
            text.line_height = text.font_size * 1.2;
        }
        Target::Text(TextField::Family) => {
            text.family = Some(read::<TextTy>(value, kind)?.to_owned())
        }
        Target::Text(TextField::Align) => text.align = text_align(read::<KeywordTy>(value, kind)?),
        Target::Text(TextField::VerticalAlign) => {
            style.vertical_align = vertical_align(read::<KeywordTy>(value, kind)?)
        }
        Target::Text(TextField::Weight) => text.weight = read::<NumberTy>(value, kind)? as u16,
    }
    Ok(())
}

fn edge(padding: &mut Padding, side: Side) -> &mut u32 {
    match side {
        Side::Left => &mut padding.left,
        Side::Top => &mut padding.top,
        Side::Right => &mut padding.right,
        Side::Bottom => &mut padding.bottom,
    }
}

fn flex_direction(value: &str) -> Direction {
    match value {
        "column" => Direction::Column,
        _ => Direction::Row,
    }
}

fn justify(value: &str) -> Justify {
    match value {
        "center" => Justify::Center,
        "end" => Justify::End,
        "space-between" => Justify::SpaceBetween,
        _ => Justify::Start,
    }
}

fn align(value: &str) -> Align {
    match value {
        "center" => Align::Center,
        "end" => Align::End,
        "stretch" => Align::Stretch,
        _ => Align::Start,
    }
}

fn text_align(value: &str) -> TextAlign {
    match value {
        "center" => TextAlign::Center,
        "right" => TextAlign::Right,
        _ => TextAlign::Left,
    }
}

fn vertical_align(value: &str) -> VerticalAlign {
    match value {
        "center" => VerticalAlign::Center,
        "bottom" => VerticalAlign::Bottom,
        _ => VerticalAlign::Top,
    }
}

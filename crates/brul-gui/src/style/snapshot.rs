use std::time::Duration;

use brul_style::{Kind, Property, Side, Snapshot};

use super::StyleError;
use super::value::{
    AreaTemplateTy, ColorTy, FlagTy, GridAreaTy, GridLineTy, KeywordTy, LengthTy, NumberTy,
    PaintTy, PxTy, SignedPxTy, TextTy, TrackListTy, read,
};
use crate::widget::{TextStylePatch, Transitions, WidgetStyle, WidgetStylePatch};
use crate::{
    Align, Direction, Display, GridAutoFlow, Justify, LayoutStyle, TextAlign, TextStyle,
    VerticalAlign,
};

include!(concat!(env!("OUT_DIR"), "/style_snapshot.rs"));

pub(super) fn base_styles(
    snapshot: &Snapshot,
) -> Result<(LayoutStyle, WidgetStyle, TextStyle), StyleError> {
    let mut layout = LayoutStyle::default();
    let mut style = WidgetStyle::default();
    let mut text = TextStyle::default();
    for property in snapshot.properties() {
        apply_property(property, &mut layout, &mut style, &mut text)?;
    }
    Ok((layout, style, text))
}

pub(super) fn state_patch(
    snapshot: &Snapshot,
) -> Result<(WidgetStylePatch, TextStylePatch), StyleError> {
    let mut style = WidgetStylePatch::default();
    let mut text = TextStylePatch::default();
    for property in snapshot.properties() {
        patch_property(property, &mut style, &mut text)?;
    }
    Ok((style, text))
}

pub(super) fn transitions(snapshot: &Snapshot) -> Result<Transitions, StyleError> {
    let mut transitions = Transitions::default();
    for animation in snapshot.animations() {
        let duration = Duration::from_millis(animation.duration.round() as u64);
        transition_property(animation.property, duration, &mut transitions)?;
    }
    Ok(transitions)
}

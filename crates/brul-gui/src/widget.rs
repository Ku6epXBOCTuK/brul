use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::Color;
use crate::animation::{Track, Tween};
use crate::gradient::Paint;
use crate::image::Image;
use crate::layout::{
    self, Direction, LayoutBox, LayoutError, LayoutNode, LayoutPatch, LayoutStyle, Offset,
    ScrollOffset, Size,
};
use crate::rect::{ClipRect, Shadow};
use crate::text::{TextAlign, TextEngine, TextError, TextLayout, TextStyle, VerticalAlign};

static NEXT_WIDGET_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WidgetId(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetKind {
    View,
    Text,
    Image,
    Input,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WidgetState {
    pub hovered: bool,
    pub pressed: bool,
    pub checked: bool,
    pub selected: bool,
    pub disabled: bool,
    pub visible: bool,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            hovered: false,
            pressed: false,
            checked: false,
            selected: false,
            disabled: false,
            visible: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidgetStyle {
    pub background: Paint,
    pub border: Color,
    pub text: Color,
    pub radius: u32,
    pub tint: Option<Color>,
    pub opacity: f32,
    pub vertical_align: VerticalAlign,
    pub clip: bool,
    pub scroll: bool,
    pub shadow: Shadow,
}

impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            background: Paint::default(),
            border: Color::TRANSPARENT,
            text: Color::rgb(15, 23, 42),
            radius: 0,
            tint: None,
            opacity: 1.0,
            vertical_align: VerticalAlign::Top,
            clip: false,
            scroll: false,
            shadow: Shadow::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WidgetStylePatch {
    pub background: Option<Paint>,
    pub border: Option<Color>,
    pub text: Option<Color>,
    pub tint: Option<Color>,
    pub opacity: Option<f32>,
    pub vertical_align: Option<VerticalAlign>,
    pub clip: Option<bool>,
    pub shadow: Option<Color>,
}

impl WidgetStylePatch {
    pub(crate) fn merge(&mut self, other: &Self) {
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.border.is_some() {
            self.border = other.border;
        }
        if other.text.is_some() {
            self.text = other.text;
        }
        if other.tint.is_some() {
            self.tint = other.tint;
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity;
        }
        if other.vertical_align.is_some() {
            self.vertical_align = other.vertical_align;
        }
        if other.clip.is_some() {
            self.clip = other.clip;
        }
        if other.shadow.is_some() {
            self.shadow = other.shadow;
        }
    }
}

impl WidgetStyle {
    fn apply(&mut self, patch: &WidgetStylePatch) {
        if let Some(background) = patch.background {
            self.background = background;
        }
        if let Some(border) = patch.border {
            self.border = border;
        }
        if let Some(text) = patch.text {
            self.text = text;
        }
        if let Some(tint) = patch.tint {
            self.tint = Some(tint);
        }
        if let Some(opacity) = patch.opacity {
            self.opacity = opacity;
        }
        if let Some(align) = patch.vertical_align {
            self.vertical_align = align;
        }
        if let Some(clip) = patch.clip {
            self.clip = clip;
        }
        if let Some(color) = patch.shadow {
            self.shadow.color = color;
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layers<P> {
    base: P,
    hovered: Option<P>,
    pressed: Option<P>,
    checked: Option<P>,
    selected: Option<P>,
    disabled: Option<P>,
}

impl<P: Copy> Layers<P> {
    pub fn new(base: P) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            checked: None,
            selected: None,
            disabled: None,
        }
    }

    pub fn with(mut self, state: StyleState, patch: P) -> Self {
        *self.slot_mut(state) = Some(patch);
        self
    }

    pub fn base(&self) -> P {
        self.base
    }

    pub fn patch(&self, state: StyleState) -> Option<P> {
        *self.slot(state)
    }

    fn slot(&self, state: StyleState) -> &Option<P> {
        match state {
            StyleState::Hovered => &self.hovered,
            StyleState::Pressed => &self.pressed,
            StyleState::Checked => &self.checked,
            StyleState::Selected => &self.selected,
            StyleState::Disabled => &self.disabled,
        }
    }

    fn slot_mut(&mut self, state: StyleState) -> &mut Option<P> {
        match state {
            StyleState::Hovered => &mut self.hovered,
            StyleState::Pressed => &mut self.pressed,
            StyleState::Checked => &mut self.checked,
            StyleState::Selected => &mut self.selected,
            StyleState::Disabled => &mut self.disabled,
        }
    }
}

impl<P: Copy + Default + PartialEq> Layers<P> {
    pub fn is_empty(&self) -> bool {
        self.base == P::default()
            && StyleState::ALL
                .iter()
                .all(|state| self.slot(*state).is_none())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Class {
    pub layout: LayoutPatch,
    pub style: Layers<WidgetStylePatch>,
    pub text: Layers<TextStylePatch>,
    pub transitions: Transitions,
}

impl Class {
    pub fn is_empty(&self) -> bool {
        self.layout.is_empty()
            && self.style.is_empty()
            && self.text.is_empty()
            && self.transitions.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StyleState {
    Hovered,
    Pressed,
    Checked,
    Selected,
    Disabled,
}

impl StyleState {
    pub const ALL: [StyleState; 5] = [
        StyleState::Hovered,
        StyleState::Pressed,
        StyleState::Checked,
        StyleState::Selected,
        StyleState::Disabled,
    ];

    pub fn flags(self) -> WidgetState {
        match self {
            StyleState::Hovered => WidgetState {
                hovered: true,
                ..WidgetState::default()
            },
            StyleState::Pressed => WidgetState {
                pressed: true,
                ..WidgetState::default()
            },
            StyleState::Checked => WidgetState {
                checked: true,
                ..WidgetState::default()
            },
            StyleState::Selected => WidgetState {
                selected: true,
                ..WidgetState::default()
            },
            StyleState::Disabled => WidgetState {
                disabled: true,
                ..WidgetState::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Transitions {
    pub background: Option<Duration>,
    pub border: Option<Duration>,
    pub text: Option<Duration>,
    pub tint: Option<Duration>,
    pub opacity: Option<Duration>,
    pub shadow: Option<Duration>,
}

impl Transitions {
    pub fn is_empty(&self) -> bool {
        self.background.is_none()
            && self.border.is_none()
            && self.text.is_none()
            && self.tint.is_none()
            && self.opacity.is_none()
            && self.shadow.is_none()
    }

    fn merge(&mut self, other: Self) {
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.border.is_some() {
            self.border = other.border;
        }
        if other.text.is_some() {
            self.text = other.text;
        }
        if other.tint.is_some() {
            self.tint = other.tint;
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity;
        }
        if other.shadow.is_some() {
            self.shadow = other.shadow;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StyleSet {
    base: WidgetStyle,
    transitions: Transitions,
    hovered: Option<WidgetStylePatch>,
    pressed: Option<WidgetStylePatch>,
    checked: Option<WidgetStylePatch>,
    selected: Option<WidgetStylePatch>,
    disabled: Option<WidgetStylePatch>,
}

impl StyleSet {
    pub fn new(base: WidgetStyle) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn with_transitions(mut self, transitions: Transitions) -> Self {
        self.transitions = transitions;
        self
    }

    pub fn with(mut self, state: StyleState, patch: WidgetStylePatch) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(patch),
            StyleState::Pressed => self.pressed = Some(patch),
            StyleState::Checked => self.checked = Some(patch),
            StyleState::Selected => self.selected = Some(patch),
            StyleState::Disabled => self.disabled = Some(patch),
        }
        self
    }

    pub fn base(&self) -> WidgetStyle {
        self.base
    }

    pub fn patch(&self, state: StyleState) -> Option<&WidgetStylePatch> {
        match state {
            StyleState::Hovered => self.hovered.as_ref(),
            StyleState::Pressed => self.pressed.as_ref(),
            StyleState::Checked => self.checked.as_ref(),
            StyleState::Selected => self.selected.as_ref(),
            StyleState::Disabled => self.disabled.as_ref(),
        }
    }

    pub fn transitions(&self) -> Transitions {
        self.transitions
    }

    pub fn merge(&mut self, patch: &WidgetStylePatch) {
        self.base.apply(patch);
    }

    pub(crate) fn apply_layers(
        &mut self,
        layers: &Layers<WidgetStylePatch>,
        transitions: Transitions,
    ) {
        self.base.apply(&layers.base());
        for state in StyleState::ALL {
            if let Some(patch) = layers.patch(state) {
                self.state_mut(state).get_or_insert_default().merge(&patch);
            }
        }
        self.transitions.merge(transitions);
    }

    fn state_mut(&mut self, state: StyleState) -> &mut Option<WidgetStylePatch> {
        match state {
            StyleState::Hovered => &mut self.hovered,
            StyleState::Pressed => &mut self.pressed,
            StyleState::Checked => &mut self.checked,
            StyleState::Selected => &mut self.selected,
            StyleState::Disabled => &mut self.disabled,
        }
    }

    pub fn resolve(&self, state: WidgetState) -> WidgetStyle {
        let mut style = self.base;
        let layers = [
            (state.selected, self.selected),
            (state.checked, self.checked),
            (state.hovered, self.hovered),
            (state.pressed, self.pressed),
            (state.disabled, self.disabled),
        ];
        for (active, patch) in layers {
            if let (true, Some(patch)) = (active, patch) {
                style.apply(&patch);
            }
        }
        style
    }
}

struct Transition {
    from: WidgetStyle,
    to: WidgetStyle,
    background: Option<Track>,
    border: Option<Track>,
    text: Option<Track>,
    tint: Option<Track>,
    opacity: Option<Track>,
    shadow: Option<Track>,
}

impl Transition {
    fn new(from: WidgetStyle, to: WidgetStyle, durations: Transitions) -> Self {
        Self {
            from,
            to,
            background: durations.background.map(Track::new),
            border: durations.border.map(Track::new),
            text: durations.text.map(Track::new),
            tint: durations.tint.map(Track::new),
            opacity: durations.opacity.map(Track::new),
            shadow: durations.shadow.map(Track::new),
        }
    }

    fn style(&self) -> WidgetStyle {
        let mut style = self.to;
        if let Some(track) = self.background {
            let progress = track.value();
            style.background = tween_paint(self.from.background, self.to.background, progress);
        }
        if let Some(track) = self.border {
            style.border = self.from.border.tween(self.to.border, track.value());
        }
        if let Some(track) = self.text {
            style.text = self.from.text.tween(self.to.text, track.value());
        }
        if let Some(track) = self.tint {
            style.tint = tween_tint(self.from.tint, self.to.tint, track.value());
        }
        if let Some(track) = self.opacity {
            style.opacity = self.from.opacity.tween(self.to.opacity, track.value());
        }
        if let Some(track) = self.shadow {
            style.shadow.color = self
                .from
                .shadow
                .color
                .tween(self.to.shadow.color, track.value());
        }
        style
    }

    fn advance(&mut self, delta: Duration) -> bool {
        let tracks = [
            &mut self.background,
            &mut self.border,
            &mut self.text,
            &mut self.tint,
            &mut self.opacity,
            &mut self.shadow,
        ];
        let mut running = false;
        for track in tracks.into_iter().flatten() {
            running |= track.advance(delta);
        }
        running
    }
}

fn tween_paint(from: Paint, to: Paint, progress: f32) -> Paint {
    match (from, to) {
        (Paint::Color(from), Paint::Color(to)) => Paint::Color(from.tween(to, progress)),
        _ => to,
    }
}

fn tween_tint(from: Option<Color>, to: Option<Color>, progress: f32) -> Option<Color> {
    match (from, to) {
        (Some(from), Some(to)) => Some(from.tween(to, progress)),
        _ => to,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextStylePatch {
    pub weight: Option<u16>,
    pub align: Option<TextAlign>,
}

impl TextStylePatch {
    fn merge(&mut self, other: &Self) {
        if other.weight.is_some() {
            self.weight = other.weight;
        }
        if other.align.is_some() {
            self.align = other.align;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStyleSet {
    base: TextStyle,
    hovered: Option<TextStylePatch>,
    pressed: Option<TextStylePatch>,
    checked: Option<TextStylePatch>,
    selected: Option<TextStylePatch>,
    disabled: Option<TextStylePatch>,
}

impl TextStyleSet {
    pub fn new(base: TextStyle) -> Self {
        Self {
            base,
            ..Self::default()
        }
    }

    pub fn with(mut self, state: StyleState, patch: TextStylePatch) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(patch),
            StyleState::Pressed => self.pressed = Some(patch),
            StyleState::Checked => self.checked = Some(patch),
            StyleState::Selected => self.selected = Some(patch),
            StyleState::Disabled => self.disabled = Some(patch),
        }
        self
    }

    pub fn base(&self) -> &TextStyle {
        &self.base
    }

    pub fn patch(&self, state: StyleState) -> Option<&TextStylePatch> {
        match state {
            StyleState::Hovered => self.hovered.as_ref(),
            StyleState::Pressed => self.pressed.as_ref(),
            StyleState::Checked => self.checked.as_ref(),
            StyleState::Selected => self.selected.as_ref(),
            StyleState::Disabled => self.disabled.as_ref(),
        }
    }

    pub fn merge(&mut self, patch: &TextStylePatch) {
        if let Some(weight) = patch.weight {
            self.base.weight = weight;
        }
        if let Some(align) = patch.align {
            self.base.align = align;
        }
    }

    pub(crate) fn apply_layers(&mut self, layers: &Layers<TextStylePatch>) {
        self.merge(&layers.base());
        for state in StyleState::ALL {
            if let Some(patch) = layers.patch(state) {
                self.state_mut(state).get_or_insert_default().merge(&patch);
            }
        }
    }

    fn state_mut(&mut self, state: StyleState) -> &mut Option<TextStylePatch> {
        match state {
            StyleState::Hovered => &mut self.hovered,
            StyleState::Pressed => &mut self.pressed,
            StyleState::Checked => &mut self.checked,
            StyleState::Selected => &mut self.selected,
            StyleState::Disabled => &mut self.disabled,
        }
    }

    pub fn resolve(&self, state: WidgetState) -> TextStyle {
        let mut style = self.base.clone();
        let layers = [
            (state.selected, self.selected),
            (state.checked, self.checked),
            (state.hovered, self.hovered),
            (state.pressed, self.pressed),
            (state.disabled, self.disabled),
        ];
        for (active, patch) in layers {
            let Some(patch) = patch.filter(|_| active) else {
                continue;
            };
            if let Some(weight) = patch.weight {
                style.weight = weight;
            }
            if let Some(align) = patch.align {
                style.align = align;
            }
        }
        style
    }
}

pub struct Widget {
    id: WidgetId,
    data: WidgetData,
    layout: LayoutStyle,
    styles: StyleSet,
    state: WidgetState,
    offset: Option<Offset>,
    draggable: bool,
    focusable: bool,
}

enum WidgetData {
    View {
        direction: Direction,
        children: Vec<Widget>,
    },
    Text {
        content: String,
        styles: Box<TextStyleSet>,
    },
    Input {
        value: String,
        styles: Box<TextStyleSet>,
        caret: usize,
    },
    Image(Image),
}

impl Widget {
    pub fn view(direction: Direction, children: Vec<Self>) -> Self {
        Self::new(WidgetData::View {
            direction,
            children,
        })
    }

    pub fn set_children(&mut self, children: Vec<Self>) {
        if let WidgetData::View {
            children: current, ..
        } = &mut self.data
        {
            *current = children;
        }
    }

    pub fn into_children(self) -> Vec<Self> {
        let Widget {
            id,
            data,
            layout,
            styles,
            state,
            offset,
            draggable,
            focusable,
        } = self;
        match data {
            WidgetData::View { children, .. } => children,
            data => vec![Widget {
                id,
                data,
                layout,
                styles,
                state,
                offset,
                draggable,
                focusable,
            }],
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self::view(Direction::Row, children)
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self::view(Direction::Column, children)
    }

    pub fn text(content: impl Into<String>) -> Self {
        Self::styled_text(content, TextStyle::default())
    }

    pub fn styled_text(content: impl Into<String>, style: TextStyle) -> Self {
        Self::text_with_styles(content, TextStyleSet::new(style))
    }

    pub fn text_with_styles(content: impl Into<String>, styles: TextStyleSet) -> Self {
        Self::new(WidgetData::Text {
            content: content.into(),
            styles: Box::new(styles),
        })
    }

    pub fn image(image: impl Into<Image>) -> Self {
        Self::new(WidgetData::Image(image.into()))
    }

    pub fn input(value: impl Into<String>) -> Self {
        Self::input_with_styles(value, TextStyleSet::default())
    }

    pub fn input_with_styles(value: impl Into<String>, styles: TextStyleSet) -> Self {
        let value = value.into();
        let caret = value.len();
        Self::new(WidgetData::Input {
            value,
            styles: Box::new(styles),
            caret,
        })
        .focusable(true)
    }

    fn new(data: WidgetData) -> Self {
        Self {
            id: WidgetId(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed)),
            data,
            layout: LayoutStyle::default(),
            styles: StyleSet::default(),
            state: WidgetState::default(),
            offset: None,
            draggable: false,
            focusable: false,
        }
    }

    pub fn id(&self) -> WidgetId {
        self.id
    }

    pub fn kind(&self) -> WidgetKind {
        match &self.data {
            WidgetData::View { .. } => WidgetKind::View,
            WidgetData::Text { .. } => WidgetKind::Text,
            WidgetData::Input { .. } => WidgetKind::Input,
            WidgetData::Image(_) => WidgetKind::Image,
        }
    }

    pub fn with_layout(mut self, layout: LayoutStyle) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_styles(mut self, styles: StyleSet) -> Self {
        self.styles = styles;
        self
    }

    pub fn with_text_styles(mut self, styles: TextStyleSet) -> Self {
        match &mut self.data {
            WidgetData::Text {
                styles: current, ..
            }
            | WidgetData::Input {
                styles: current, ..
            } => **current = styles,
            WidgetData::View { .. } | WidgetData::Image(_) => {}
        }
        self
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        self.offset = Some(Offset { x, y });
        self
    }

    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.state.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.state.visible = visible;
        self
    }
}

#[derive(Clone)]
pub struct WidgetSnapshot {
    pub size: Size,
    pub nodes: Vec<WidgetSnapshotNode>,
}

impl WidgetSnapshot {
    pub fn node(&self, id: WidgetId) -> Option<&WidgetSnapshotNode> {
        self.nodes.iter().find(|node| node.id == id)
    }
}

#[derive(Clone)]
pub struct WidgetSnapshotNode {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub border_width: u32,
    pub content_x: i32,
    pub content_y: i32,
    pub content_width: u32,
    pub content_height: u32,
    pub state: WidgetState,
    pub style: WidgetStyle,
    pub text: Option<TextLayout>,
    pub caret: Option<i32>,
    pub image: Option<Image>,
    pub clips: Vec<ClipRect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WidgetEvent {
    PointerEnter(WidgetId),
    PointerLeave(WidgetId),
    Press(WidgetId),
    Release(WidgetId),
    Click(WidgetId),
    Drag(WidgetId),
    Key(String),
    Timer(String),
    Focus(WidgetId),
    Blur(WidgetId),
    Input(WidgetId, String, usize),
    Submit(WidgetId),
}

#[derive(Clone, Debug, Default)]
pub struct Dirty {
    pub full: bool,
    pub ids: Vec<WidgetId>,
}

impl Dirty {
    pub fn all() -> Self {
        Self {
            full: true,
            ids: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        !self.full && self.ids.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetError {
    NoLayout,
    InvalidScale,
    Layout(LayoutError),
    Text(TextError),
}

impl fmt::Display for WidgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLayout => formatter.write_str("widget tree is not laid out yet"),
            Self::InvalidScale => formatter.write_str("invalid DPI scale"),
            Self::Layout(error) => write!(formatter, "layout: {error}"),
            Self::Text(error) => write!(formatter, "text: {error}"),
        }
    }
}

impl std::error::Error for WidgetError {}

impl From<LayoutError> for WidgetError {
    fn from(error: LayoutError) -> Self {
        Self::Layout(error)
    }
}

impl From<TextError> for WidgetError {
    fn from(error: TextError) -> Self {
        Self::Text(error)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Slot(u32);

impl Slot {
    fn index(self) -> usize {
        self.0 as usize
    }
}

struct Node {
    id: WidgetId,
    data: NodeData,
    layout: LayoutStyle,
    styles: StyleSet,
    class: Option<Class>,
    state: WidgetState,
    offset: Option<Offset>,
    draggable: bool,
    focusable: bool,
    scroll_offset: ScrollOffset,
    children: Vec<Slot>,
    transition: Option<Transition>,
    shown: WidgetStyle,
}

enum NodeData {
    View {
        direction: Direction,
    },
    Text {
        content: String,
        styles: Box<TextStyleSet>,
    },
    Input {
        value: String,
        styles: Box<TextStyleSet>,
        caret: usize,
    },
    Image(Image),
}

impl Node {
    fn kind(&self) -> WidgetKind {
        match &self.data {
            NodeData::View { .. } => WidgetKind::View,
            NodeData::Text { .. } => WidgetKind::Text,
            NodeData::Input { .. } => WidgetKind::Input,
            NodeData::Image(_) => WidgetKind::Image,
        }
    }

    fn style(&self) -> WidgetStyle {
        match &self.transition {
            Some(transition) => transition.style(),
            None => self.styles().resolve(self.state),
        }
    }

    fn styles(&self) -> StyleSet {
        let mut styles = self.styles.clone();
        if let Some(class) = &self.class {
            styles.apply_layers(&class.style, class.transitions);
        }
        styles
    }

    fn layout_style(&self) -> LayoutStyle {
        let mut layout = self.layout;
        if let Some(class) = &self.class {
            layout.apply(&class.layout);
        }
        layout
    }
}

pub struct WidgetTree {
    nodes: Vec<Node>,
    index: HashMap<WidgetId, Slot>,
    free: Vec<Slot>,
    root: Slot,
    geometry: Option<Geometry>,
    snapshot: Option<WidgetSnapshot>,
    texts: HashMap<WidgetId, ShapedText>,
    hovered: Vec<Slot>,
    pressed: Vec<Slot>,
    focused: Option<Slot>,
    dirty: HashSet<WidgetId>,
    stale: HashSet<Slot>,
    full_dirty: bool,
    layout_dirty: bool,
}

struct Geometry {
    viewport: Size,
    scale: f32,
    boxes: Vec<LayoutBox>,
}

struct ShapedText {
    content: String,
    width: Option<u32>,
    scale: f32,
    variants: TextVariants,
}

#[derive(Clone, Debug)]
struct TextVariants {
    base: TextLayout,
    hovered: Option<TextLayout>,
    pressed: Option<TextLayout>,
    checked: Option<TextLayout>,
    selected: Option<TextLayout>,
    disabled: Option<TextLayout>,
}

impl TextVariants {
    fn new(base: TextLayout) -> Self {
        Self {
            base,
            hovered: None,
            pressed: None,
            checked: None,
            selected: None,
            disabled: None,
        }
    }

    fn with(mut self, state: StyleState, layout: TextLayout) -> Self {
        match state {
            StyleState::Hovered => self.hovered = Some(layout),
            StyleState::Pressed => self.pressed = Some(layout),
            StyleState::Checked => self.checked = Some(layout),
            StyleState::Selected => self.selected = Some(layout),
            StyleState::Disabled => self.disabled = Some(layout),
        }
        self
    }

    fn resolve(&self, state: WidgetState) -> TextLayout {
        let mut layout = &self.base;
        if state.selected {
            layout = self.selected.as_ref().unwrap_or(layout);
        }
        if state.checked {
            layout = self.checked.as_ref().unwrap_or(layout);
        }
        if state.hovered {
            layout = self.hovered.as_ref().unwrap_or(layout);
        }
        if state.pressed {
            layout = self.pressed.as_ref().unwrap_or(layout);
        }
        if state.disabled {
            layout = self.disabled.as_ref().unwrap_or(layout);
        }
        layout.clone()
    }
}

impl WidgetTree {
    pub fn new(root: Widget) -> Self {
        let mut tree = Self {
            nodes: Vec::new(),
            index: HashMap::new(),
            free: Vec::new(),
            root: Slot(0),
            geometry: None,
            snapshot: None,
            texts: HashMap::new(),
            hovered: Vec::new(),
            pressed: Vec::new(),
            focused: None,
            dirty: HashSet::new(),
            stale: HashSet::new(),
            full_dirty: true,
            layout_dirty: true,
        };
        tree.root = tree.insert(root);
        tree
    }

    fn insert(&mut self, widget: Widget) -> Slot {
        let Widget {
            id,
            data,
            layout,
            styles,
            state,
            offset,
            draggable,
            focusable,
        } = widget;
        let (data, children) = match data {
            WidgetData::View {
                direction,
                children,
            } => (NodeData::View { direction }, children),
            WidgetData::Text { content, styles } => {
                (NodeData::Text { content, styles }, Vec::new())
            }
            WidgetData::Input {
                value,
                styles,
                caret,
            } => (
                NodeData::Input {
                    value,
                    styles,
                    caret,
                },
                Vec::new(),
            ),
            WidgetData::Image(image) => (NodeData::Image(image), Vec::new()),
        };
        let mut slots = Vec::with_capacity(children.len());
        for child in children {
            slots.push(self.insert(child));
        }
        let shown = styles.resolve(state);
        let node = Node {
            id,
            data,
            layout,
            styles,
            class: None,
            state,
            offset,
            draggable,
            focusable,
            scroll_offset: ScrollOffset::ZERO,
            children: slots,
            transition: None,
            shown,
        };
        let slot = match self.free.pop() {
            Some(slot) => {
                self.nodes[slot.index()] = node;
                slot
            }
            None => {
                self.nodes.push(node);
                Slot((self.nodes.len() - 1) as u32)
            }
        };
        self.index.insert(id, slot);
        self.stale.insert(slot);
        slot
    }

    fn extract(&mut self, slot: Slot) -> Widget {
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        let children = children
            .into_iter()
            .map(|child| self.extract(child))
            .collect();
        let node = &mut self.nodes[slot.index()];
        let data = match &mut node.data {
            NodeData::View { direction } => WidgetData::View {
                direction: *direction,
                children,
            },
            NodeData::Text { content, styles } => WidgetData::Text {
                content: std::mem::take(content),
                styles: Box::new(std::mem::take(&mut **styles)),
            },
            NodeData::Input {
                value,
                styles,
                caret,
            } => WidgetData::Input {
                value: std::mem::take(value),
                styles: Box::new(std::mem::take(&mut **styles)),
                caret: *caret,
            },
            NodeData::Image(image) => WidgetData::Image(image.clone()),
        };
        let widget = Widget {
            id: node.id,
            data,
            layout: node.layout,
            styles: node.styles.clone(),
            state: node.state,
            offset: node.offset,
            draggable: node.draggable,
            focusable: node.focusable,
        };
        self.index.remove(&widget.id);
        self.free.push(slot);
        widget
    }

    fn remove(&mut self, slot: Slot) {
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        for child in children {
            self.remove(child);
        }
        self.index.remove(&self.nodes[slot.index()].id);
        self.free.push(slot);
    }

    fn node(&self, id: WidgetId) -> Option<&Node> {
        self.index.get(&id).map(|slot| &self.nodes[slot.index()])
    }

    fn node_mut(&mut self, id: WidgetId) -> Option<&mut Node> {
        let slot = *self.index.get(&id)?;
        Some(&mut self.nodes[slot.index()])
    }

    pub fn replace_root(&mut self, root: Widget) {
        self.nodes.clear();
        self.index.clear();
        self.free.clear();
        self.stale.clear();
        self.geometry = None;
        self.clear_interaction();
        self.root = self.insert(root);
        self.invalidate();
    }

    pub fn replace_children(
        &mut self,
        parent: WidgetId,
        start: usize,
        count: usize,
        children: Vec<Widget>,
    ) -> bool {
        let Some(slot) = self.index.get(&parent).copied() else {
            return false;
        };
        if !matches!(self.nodes[slot.index()].data, NodeData::View { .. }) {
            return false;
        }
        let current = self.nodes[slot.index()].children.len();
        if start > current || count > current - start {
            return false;
        }
        self.clear_interaction();
        let removed: Vec<Slot> = self.nodes[slot.index()]
            .children
            .splice(start..start + count, [])
            .collect();
        for child in removed {
            self.remove(child);
        }
        let mut inserted = Vec::with_capacity(children.len());
        for child in children {
            inserted.push(self.insert(child));
        }
        self.nodes[slot.index()]
            .children
            .splice(start..start, inserted);
        self.stale.insert(slot);
        self.invalidate();
        true
    }

    pub fn replace_all_children(&mut self, parent: WidgetId, children: Vec<Widget>) -> bool {
        let Some(slot) = self.index.get(&parent).copied() else {
            return false;
        };
        if !matches!(self.nodes[slot.index()].data, NodeData::View { .. }) {
            return false;
        }
        let count = self.nodes[slot.index()].children.len();
        self.replace_children(parent, 0, count, children)
    }

    pub fn take_children(&mut self, parent: WidgetId) -> Option<Vec<Widget>> {
        let slot = *self.index.get(&parent)?;
        if !matches!(self.nodes[slot.index()].data, NodeData::View { .. }) {
            return None;
        }
        self.clear_interaction();
        let children = std::mem::take(&mut self.nodes[slot.index()].children);
        let children = children
            .into_iter()
            .map(|child| self.extract(child))
            .collect();
        self.stale.insert(slot);
        self.invalidate();
        Some(children)
    }

    pub fn contains(&self, id: WidgetId) -> bool {
        self.index.contains_key(&id)
    }

    fn invalidate(&mut self) {
        self.snapshot = None;
        self.dirty.clear();
        self.full_dirty = true;
        self.layout_dirty = true;
    }

    pub fn layout(
        &mut self,
        engine: &mut TextEngine,
        viewport: Size,
        scale: f32,
    ) -> Result<(), WidgetError> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err(WidgetError::InvalidScale);
        }

        let reuse = self
            .geometry
            .as_ref()
            .is_some_and(|geometry| geometry.viewport == viewport && geometry.scale == scale);
        let frame_changed = !reuse;
        if self.layout_pass(engine, viewport, scale, reuse, frame_changed)? {
            self.layout_pass(engine, viewport, scale, false, frame_changed)?;
        }
        Ok(())
    }

    fn layout_pass(
        &mut self,
        engine: &mut TextEngine,
        viewport: Size,
        scale: f32,
        reuse: bool,
        frame_changed: bool,
    ) -> Result<bool, WidgetError> {
        let previous = self.geometry.take();
        let mut boxes = previous.map(|geometry| geometry.boxes).unwrap_or_default();
        boxes.resize(self.nodes.len(), LayoutBox::default());

        let (node, order, frozen) = {
            let mut pass = Pass {
                tree: &self.nodes,
                engine,
                texts: &mut self.texts,
                scale,
                cached: reuse.then_some(&boxes),
                blocked: dirty_subtrees(&self.nodes, self.root, &self.stale),
                order: Vec::with_capacity(self.nodes.len()),
                frozen: Vec::new(),
            };
            let node = pass.build(self.root, Some(viewport.width))?;
            (node, pass.order, pass.frozen)
        };
        self.texts.retain(|id, _| self.index.contains_key(id));

        let placed = layout::layout(&node, viewport)?;
        let mut cursor = 0;
        flatten_boxes(&placed, &order, &mut cursor, &mut boxes);

        let resized = frozen.iter().any(|(slot, cached)| {
            let placed = &boxes[slot.index()];
            placed.x != cached.x
                || placed.y != cached.y
                || placed.width != cached.width
                || placed.height != cached.height
        });
        if resized {
            return Ok(true);
        }
        for (slot, cached) in frozen {
            let placed = &boxes[slot.index()];
            let dx = placed.x - cached.x;
            let dy = placed.y - cached.y;
            if dx != 0 || dy != 0 {
                move_children(&self.nodes, slot, dx, dy, &mut boxes);
            }
        }

        self.geometry = Some(Geometry {
            viewport,
            scale,
            boxes,
        });
        self.stale.clear();
        self.snapshot = None;
        self.full_dirty = true;
        self.layout_dirty = false;
        self.dirty.clear();
        if frame_changed {
            self.clear_interaction();
        }
        Ok(false)
    }

    pub fn relayout(&mut self, engine: &mut TextEngine) -> Result<(), WidgetError> {
        let Some((viewport, scale)) = self
            .geometry
            .as_ref()
            .map(|geometry| (geometry.viewport, geometry.scale))
        else {
            return Ok(());
        };
        self.layout(engine, viewport, scale)
    }

    pub fn layout_dirty(&self) -> bool {
        self.layout_dirty
    }

    pub fn has_layout(&self) -> bool {
        self.geometry.is_some()
    }

    pub fn snapshot(&mut self) -> Result<&WidgetSnapshot, WidgetError> {
        if self.snapshot.is_none() {
            let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
            let snapshot =
                build_snapshot(&self.nodes, self.root, geometry, &self.texts, self.focused);
            self.snapshot = Some(snapshot);
        }
        Ok(self.snapshot.as_ref().expect("snapshot built"))
    }

    pub fn snapshot_dirty(&self) -> bool {
        self.snapshot.is_none()
    }

    pub fn take_dirty(&mut self) -> Dirty {
        let full = std::mem::take(&mut self.full_dirty);
        Dirty {
            full,
            ids: if full {
                self.dirty.clear();
                Vec::new()
            } else {
                self.dirty.drain().collect()
            },
        }
    }

    pub fn animating(&self) -> bool {
        self.index
            .values()
            .any(|slot| self.nodes[slot.index()].transition.is_some())
    }

    pub fn advance(&mut self, delta: Duration) -> bool {
        if delta.is_zero() {
            return self.animating();
        }
        let slots: Vec<Slot> = self.index.values().copied().collect();
        let mut running = false;
        let mut changed = false;
        for slot in slots {
            let node = &mut self.nodes[slot.index()];
            let Some(transition) = node.transition.as_mut() else {
                continue;
            };
            let active = transition.advance(delta);
            if !active {
                node.transition = None;
            }
            let id = node.id;
            running |= active;
            changed = true;
            self.mark_dirty(id);
        }
        if changed {
            self.snapshot = None;
        }
        running
    }

    fn retarget(&mut self, slot: Slot) {
        let styles = self.nodes[slot.index()].styles();
        let node = &mut self.nodes[slot.index()];
        let durations = styles.transitions();
        let target = styles.resolve(node.state);
        let from = match node.transition.take() {
            Some(transition) if transition.to != target => transition.style(),
            Some(transition) => {
                node.transition = Some(transition);
                return;
            }
            None => node.shown,
        };
        node.shown = target;
        if durations.is_empty() {
            node.transition = None;
            return;
        }
        node.transition = (from != target).then(|| Transition::new(from, target, durations));
    }

    pub fn state(&self, id: WidgetId) -> Option<WidgetState> {
        Some(self.node(id)?.state)
    }

    pub fn set_checked(&mut self, id: WidgetId, checked: bool) -> bool {
        self.update_state(id, |state| state.checked = checked)
    }

    pub fn set_selected(&mut self, id: WidgetId, selected: bool) -> bool {
        self.update_state(id, |state| state.selected = selected)
    }

    pub fn set_disabled(&mut self, id: WidgetId, disabled: bool) -> bool {
        self.update_state(id, |state| state.disabled = disabled)
    }

    pub fn apply_class(&mut self, id: WidgetId, class: Option<&Class>) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let class = class.copied().filter(|class| !class.is_empty());
        let node = &mut self.nodes[slot.index()];
        if node.class == class {
            return false;
        }
        let layout = node.layout_style();
        let text = node.class.map(|class| class.text).unwrap_or_default();
        node.class = class;
        let relayout = node.layout_style() != layout
            || node.class.map(|class| class.text).unwrap_or_default() != text;
        if relayout {
            self.stale.insert(slot);
            self.texts.remove(&id);
            self.invalidate_layout();
            return true;
        }
        self.retarget(slot);
        self.snapshot = None;
        self.mark_dirty(id);
        true
    }

    pub fn set_visible(&mut self, id: WidgetId, visible: bool) -> bool {
        if !self.update_state(id, |state| state.visible = visible) {
            return false;
        }
        self.full_dirty = true;
        self.dirty.clear();
        true
    }

    pub fn set_text(&mut self, id: WidgetId, content: String) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let NodeData::Text {
            content: current, ..
        } = &mut self.nodes[slot.index()].data
        else {
            return false;
        };
        if *current == content {
            return false;
        }
        *current = content;
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub fn set_image(&mut self, id: WidgetId, image: Image) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let NodeData::Image(current) = &mut self.nodes[slot.index()].data else {
            return false;
        };
        if current.ptr_eq(&image) {
            return false;
        }
        *current = image;
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    pub fn set_offset(&mut self, id: WidgetId, x: i32, y: i32) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let offset = Offset { x, y };
        if self.nodes[slot.index()].offset == Some(offset) {
            return false;
        }
        self.nodes[slot.index()].offset = Some(offset);
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    fn invalidate_layout(&mut self) {
        self.invalidate_snapshot();
        self.layout_dirty = true;
    }

    fn invalidate_snapshot(&mut self) {
        self.snapshot = None;
        self.full_dirty = true;
        self.dirty.clear();
    }

    pub fn toggle_checked(&mut self, id: WidgetId) -> Option<bool> {
        let checked = {
            let node = self.node_mut(id)?;
            node.state.checked = !node.state.checked;
            node.state.checked
        };
        if let Some(&slot) = self.index.get(&id) {
            self.retarget(slot);
        }
        self.snapshot = None;
        self.mark_dirty(id);
        Some(checked)
    }

    pub fn hit(&self, x: i32, y: i32) -> Result<Option<WidgetId>, WidgetError> {
        Ok(self
            .hit_path(x, y)?
            .last()
            .map(|slot| self.nodes[slot.index()].id))
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.focused.map(|slot| self.nodes[slot.index()].id)
    }

    fn place_caret(&mut self, slot: Slot, x: i32) {
        let Some(geometry) = self.geometry.as_ref() else {
            return;
        };
        let node = &self.nodes[slot.index()];
        let NodeData::Input { caret, .. } = &node.data else {
            return;
        };
        let Some(text) = self.texts.get(&node.id) else {
            return;
        };
        let layout_style = node.layout_style();
        let boxed = &geometry.boxes[slot.index()];
        let origin = boxed.x.saturating_add(
            layout_style
                .border_width
                .saturating_add(layout_style.padding.left) as i32,
        );
        let offset = x.saturating_sub(origin);
        let byte = text
            .variants
            .base
            .carets
            .iter()
            .min_by_key(|stop| (stop.x - offset).abs())
            .map_or(0, |stop| stop.byte);
        let caret = *caret;
        if let NodeData::Input { caret, .. } = &mut self.nodes[slot.index()].data {
            *caret = byte;
        }
        if caret != byte {
            self.invalidate_snapshot();
        }
    }

    pub fn press_key(&mut self, code: &str) -> Vec<WidgetEvent> {
        if code == "Tab" {
            return self.focus_next();
        }
        let Some(slot) = self
            .focused
            .filter(|slot| matches!(self.nodes[slot.index()].data, NodeData::Input { .. }))
        else {
            return vec![WidgetEvent::Key(code.to_owned())];
        };
        match code {
            "Escape" | "ArrowUp" | "ArrowDown" => vec![WidgetEvent::Key(code.to_owned())],
            "Enter" => vec![WidgetEvent::Submit(self.nodes[slot.index()].id)],
            "ArrowLeft" | "ArrowRight" | "Home" | "End" => {
                self.edit_input(slot, code);
                Vec::new()
            }
            text => {
                if !self.edit_input(slot, text) {
                    return Vec::new();
                }
                let node = &self.nodes[slot.index()];
                let NodeData::Input { value, caret, .. } = &node.data else {
                    return Vec::new();
                };
                vec![WidgetEvent::Input(node.id, value.clone(), *caret)]
            }
        }
    }

    fn edit_input(&mut self, slot: Slot, code: &str) -> bool {
        let NodeData::Input { value, caret, .. } = &mut self.nodes[slot.index()].data else {
            return false;
        };
        let previous = value.clone();
        match code {
            "Backspace" => {
                if let Some((index, _)) = value[..*caret].char_indices().next_back() {
                    value.remove(index);
                    *caret = index;
                }
            }
            "Delete" => {
                if *caret < value.len() {
                    value.remove(*caret);
                }
            }
            "ArrowLeft" => {
                *caret = value[..*caret]
                    .char_indices()
                    .next_back()
                    .map_or(0, |(index, _)| index);
            }
            "ArrowRight" => {
                if let Some(character) = value[*caret..].chars().next() {
                    *caret += character.len_utf8();
                }
            }
            "Home" => *caret = 0,
            "End" => *caret = value.len(),
            text => {
                if text.is_empty() || text.chars().any(char::is_control) {
                    return false;
                }
                value.insert_str(*caret, text);
                *caret += text.len();
            }
        }
        let changed = *value != previous;
        self.stale.insert(slot);
        self.invalidate_layout();
        changed
    }

    pub fn set_input_text(&mut self, id: WidgetId, text: String) -> bool {
        let Some(&slot) = self.index.get(&id) else {
            return false;
        };
        let NodeData::Input { value, caret, .. } = &mut self.nodes[slot.index()].data else {
            return false;
        };
        if *value == text {
            return false;
        }
        *value = text;
        *caret = (*caret).min(value.len());
        while !value.is_char_boundary(*caret) {
            *caret -= 1;
        }
        self.stale.insert(slot);
        self.invalidate_layout();
        true
    }

    fn focus_next(&mut self) -> Vec<WidgetEvent> {
        let slots = self.focusable_slots();
        let Some(first) = slots.first().copied() else {
            return self.focus_events(None);
        };
        let target = match self
            .focused
            .and_then(|current| slots.iter().position(|slot| *slot == current))
        {
            Some(index) => slots[(index + 1) % slots.len()],
            None => first,
        };
        self.focus_events(Some(target))
    }

    fn focusable_slots(&self) -> Vec<Slot> {
        let mut slots: Vec<Slot> = self
            .index
            .values()
            .copied()
            .filter(|slot| {
                let node = &self.nodes[slot.index()];
                node.focusable && node.state.visible && !node.state.disabled
            })
            .collect();
        slots.sort_by_key(|slot| slot.index());
        slots
    }

    fn focus_events(&mut self, slot: Option<Slot>) -> Vec<WidgetEvent> {
        if self.focused == slot {
            return Vec::new();
        }
        let mut events = Vec::with_capacity(2);
        if let Some(previous) = self.focused {
            events.push(WidgetEvent::Blur(self.nodes[previous.index()].id));
        }
        if let Some(next) = slot {
            events.push(WidgetEvent::Focus(self.nodes[next.index()].id));
        }
        self.focused = slot;
        self.invalidate_snapshot();
        events
    }

    pub fn wheel(&mut self, x: i32, y: i32, dy: i32) -> bool {
        if dy == 0 {
            return false;
        }
        let Ok(path) = self.hit_path(x, y) else {
            return false;
        };
        let target = path.iter().rev().copied().find(|slot| {
            let node = &self.nodes[slot.index()];
            matches!(node.data, NodeData::View { .. }) && node.styles.resolve(node.state).scroll
        });
        let Some(target) = target else {
            return false;
        };
        let Some(geometry) = self.geometry.as_ref() else {
            return false;
        };
        let Some(max) = max_scroll_offset(&self.nodes, target, &geometry.boxes) else {
            return false;
        };
        let node = &mut self.nodes[target.index()];
        let scroll_offset = node.scroll_offset.saturating_add(dy).clamp(0, max);
        if scroll_offset == node.scroll_offset {
            return false;
        }
        node.scroll_offset = scroll_offset;
        self.stale.insert(target);
        self.invalidate();
        true
    }

    pub fn pointer_moved(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        let common = common_prefix(&self.hovered, &path);
        if common == self.hovered.len() && common == path.len() {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        let leaving: Vec<Slot> = self.hovered[common..].iter().rev().copied().collect();
        for slot in leaving {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for &slot in &path[common..] {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = true;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::PointerEnter(id));
        }
        self.hovered = path;
        self.snapshot = None;
        Ok(events)
    }

    pub fn pointer_down(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        self.set_pressed(false);

        let focus = path
            .iter()
            .rev()
            .copied()
            .find(|slot| self.nodes[slot.index()].focusable);
        let mut events = self.focus_events(focus);
        if let Some(slot) = focus {
            self.place_caret(slot, x);
        }
        for &slot in path.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = true;
            let id = node.id;
            let draggable = node.draggable;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::Press(id));
            if draggable {
                events.push(WidgetEvent::Drag(id));
            }
        }
        if !path.is_empty() {
            self.snapshot = None;
        }
        self.pressed = path;
        Ok(events)
    }

    pub fn pointer_up(&mut self, x: i32, y: i32) -> Result<Vec<WidgetEvent>, WidgetError> {
        let path = self.active_path(&self.hit_path(x, y)?);
        let pressed = std::mem::take(&mut self.pressed);

        let mut events = Vec::new();
        for &slot in pressed.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
            events.push(WidgetEvent::Release(id));
        }
        if !pressed.is_empty() && pressed.last() == path.last() {
            for &slot in path.iter().rev() {
                events.push(WidgetEvent::Click(self.nodes[slot.index()].id));
            }
        }
        if !pressed.is_empty() {
            self.snapshot = None;
        }
        Ok(events)
    }

    pub fn pointer_leave(&mut self) -> Vec<WidgetEvent> {
        let hovered = std::mem::take(&mut self.hovered);
        let pressed = std::mem::take(&mut self.pressed);
        let mut events = Vec::with_capacity(hovered.len() + pressed.len());

        for slot in hovered.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.hovered = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(*slot);
            events.push(WidgetEvent::PointerLeave(id));
        }
        for slot in pressed.iter().rev() {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = false;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(*slot);
            events.push(WidgetEvent::Release(id));
        }
        if !hovered.is_empty() || !pressed.is_empty() {
            self.snapshot = None;
        }
        events
    }

    fn update_state(&mut self, id: WidgetId, update: impl FnOnce(&mut WidgetState)) -> bool {
        let changed = {
            let Some(state) = self.node_mut(id).map(|node| &mut node.state) else {
                return false;
            };
            let before = *state;
            update(state);
            *state != before
        };
        if !changed {
            return false;
        }
        if let Some(&slot) = self.index.get(&id) {
            self.retarget(slot);
        }
        self.snapshot = None;
        self.mark_dirty(id);
        true
    }

    fn mark_dirty(&mut self, id: WidgetId) {
        if !self.full_dirty {
            self.dirty.insert(id);
        }
    }

    fn clear_interaction(&mut self) {
        let hovered = std::mem::take(&mut self.hovered);
        let pressed = std::mem::take(&mut self.pressed);
        for slot in hovered {
            if let Some(node) = self.nodes.get_mut(slot.index()) {
                node.state.hovered = false;
                node.shown = node.styles.resolve(node.state);
            }
        }
        for slot in pressed {
            if let Some(node) = self.nodes.get_mut(slot.index()) {
                node.state.pressed = false;
                node.shown = node.styles.resolve(node.state);
            }
        }
    }

    fn set_pressed(&mut self, pressed: bool) {
        let had_pressed = !self.pressed.is_empty();
        let slots = std::mem::take(&mut self.pressed);
        for &slot in &slots {
            let node = &mut self.nodes[slot.index()];
            node.state.pressed = pressed;
            let id = node.id;
            self.mark_dirty(id);
            self.retarget(slot);
        }
        if had_pressed {
            self.snapshot = None;
        }
    }

    fn hit_path(&self, x: i32, y: i32) -> Result<Vec<Slot>, WidgetError> {
        let geometry = self.geometry.as_ref().ok_or(WidgetError::NoLayout)?;
        let mut path = Vec::new();
        collect_hit(
            &self.nodes,
            self.root,
            &geometry.boxes,
            &[],
            x,
            y,
            &mut path,
        );
        Ok(path)
    }

    fn active_path(&self, path: &[Slot]) -> Vec<Slot> {
        let mut active = Vec::with_capacity(path.len());
        for &slot in path {
            let Some(node) = self.nodes.get(slot.index()) else {
                break;
            };
            if node.state.disabled || !node.state.visible {
                break;
            }
            active.push(slot);
        }
        active
    }
}

struct Pass<'a> {
    tree: &'a [Node],
    engine: &'a mut TextEngine,
    texts: &'a mut HashMap<WidgetId, ShapedText>,
    scale: f32,
    cached: Option<&'a [LayoutBox]>,
    blocked: Vec<bool>,
    order: Vec<Slot>,
    frozen: Vec<(Slot, LayoutBox)>,
}

fn dirty_subtrees(tree: &[Node], root: Slot, stale: &HashSet<Slot>) -> Vec<bool> {
    fn walk(tree: &[Node], slot: Slot, stale: &HashSet<Slot>, blocked: &mut [bool]) -> bool {
        let node = &tree[slot.index()];
        let mut dirty = stale.contains(&slot);
        for &child in &node.children {
            dirty |= walk(tree, child, stale, blocked);
        }
        blocked[slot.index()] = dirty;
        dirty
    }
    let mut blocked = vec![false; tree.len()];
    walk(tree, root, stale, &mut blocked);
    blocked
}

fn move_children(tree: &[Node], slot: Slot, dx: i32, dy: i32, boxes: &mut [LayoutBox]) {
    for &child in &tree[slot.index()].children {
        let placed = &mut boxes[child.index()];
        placed.x = placed.x.saturating_add(dx);
        placed.y = placed.y.saturating_add(dy);
        move_children(tree, child, dx, dy, boxes);
    }
}

impl Pass<'_> {
    fn build(
        &mut self,
        slot: Slot,
        available_width: Option<u32>,
    ) -> Result<LayoutNode, WidgetError> {
        let node = &self.tree[slot.index()];
        let layout_style = node.layout_style();
        self.order.push(slot);
        if let (Some(cached), false) = (self.cached, self.blocked[slot.index()])
            && let Some(previous) = cached.get(slot.index())
        {
            let size = Size {
                width: previous
                    .width
                    .saturating_sub(layout::inset_x(&layout_style)),
                height: previous
                    .height
                    .saturating_sub(layout::inset_y(&layout_style)),
            };
            self.frozen.push((slot, previous.clone()));
            let layout_node = LayoutNode::leaf(size.width, size.height).with_style(layout_style);
            return Ok(match node.offset {
                Some(offset) => layout_node.with_offset(offset.x, offset.y),
                None => layout_node,
            });
        }

        let resolved = layout::resolve(layout_style.width, available_width)?;
        let content_width = resolved
            .or(available_width)
            .map(|width| width.saturating_sub(layout::inset_x(&layout_style)));

        let layout_node = match &node.data {
            NodeData::View { direction } => {
                let mut children = Vec::with_capacity(node.children.len());
                for &child in &node.children {
                    children.push(self.build(child, content_width)?);
                }
                match direction {
                    Direction::Row => LayoutNode::row(children),
                    Direction::Column => {
                        LayoutNode::column(children).with_scroll(node.scroll_offset)
                    }
                }
            }
            NodeData::Text { content, styles } => {
                self.build_text(node, content, styles, content_width)?
            }
            NodeData::Input { value, styles, .. } => {
                self.build_text(node, value, styles, content_width)?
            }
            NodeData::Image(image) => LayoutNode::leaf(image.width(), image.height()),
        };

        let layout_node = layout_node.with_style(layout_style);
        Ok(match node.offset {
            Some(offset) => layout_node.with_offset(offset.x, offset.y),
            None => layout_node,
        })
    }

    fn build_text(
        &mut self,
        node: &Node,
        content: &str,
        styles: &TextStyleSet,
        content_width: Option<u32>,
    ) -> Result<LayoutNode, WidgetError> {
        let mut styles = styles.clone();
        if let Some(class) = &node.class {
            styles.apply_layers(&class.text);
        }
        let max_width = content_width.map(|width| width as f32 / self.scale);
        let cached = self.texts.get(&node.id).filter(|cached| {
            cached.width == content_width && cached.scale == self.scale && cached.content == content
        });
        if cached.is_none() {
            let mut variants = TextVariants::new(self.engine.layout(
                content,
                styles.base(),
                max_width,
                self.scale,
            )?);
            for state in StyleState::ALL {
                let style = styles.resolve(state.flags());
                if style == *styles.base() {
                    continue;
                }
                variants = variants.with(
                    state,
                    self.engine.layout(content, &style, max_width, self.scale)?,
                );
            }
            self.texts.insert(
                node.id,
                ShapedText {
                    content: content.to_owned(),
                    width: content_width,
                    scale: self.scale,
                    variants,
                },
            );
        }
        let size = self
            .texts
            .get(&node.id)
            .expect("shaped text cached")
            .variants
            .base
            .size;
        Ok(LayoutNode::leaf(size.width, size.height))
    }
}

fn max_scroll_offset(nodes: &[Node], slot: Slot, boxes: &[LayoutBox]) -> Option<i32> {
    let node = &nodes[slot.index()];
    let viewport = &boxes[slot.index()];
    let bottom = node
        .children
        .iter()
        .map(|&child| {
            let placed = &boxes[child.index()];
            i64::from(placed.y) + i64::from(placed.height)
        })
        .max()?;
    let overflow = bottom + i64::from(node.scroll_offset.pixels())
        - i64::from(viewport.y)
        - i64::from(viewport.height);
    i32::try_from(overflow.max(0)).ok()
}

fn flatten_boxes(boxed: &LayoutBox, order: &[Slot], cursor: &mut usize, boxes: &mut [LayoutBox]) {
    let slot = order[*cursor];
    *cursor += 1;
    boxes[slot.index()] = LayoutBox {
        x: boxed.x,
        y: boxed.y,
        width: boxed.width,
        height: boxed.height,
        children: Vec::new(),
    };
    for child in &boxed.children {
        flatten_boxes(child, order, cursor, boxes);
    }
}

fn build_snapshot(
    tree: &[Node],
    root: Slot,
    geometry: &Geometry,
    texts: &HashMap<WidgetId, ShapedText>,
    focused: Option<Slot>,
) -> WidgetSnapshot {
    let mut nodes = Vec::new();
    snapshot_widget(tree, root, &geometry.boxes, texts, &[], focused, &mut nodes);
    let root_box = &geometry.boxes[root.index()];
    WidgetSnapshot {
        size: Size {
            width: root_box.width,
            height: root_box.height,
        },
        nodes,
    }
}

fn snapshot_widget(
    tree: &[Node],
    slot: Slot,
    boxes: &[LayoutBox],
    texts: &HashMap<WidgetId, ShapedText>,
    clips: &[ClipRect],
    focused: Option<Slot>,
    nodes: &mut Vec<WidgetSnapshotNode>,
) {
    let node = &tree[slot.index()];
    if !node.state.visible {
        return;
    }

    let layout_box = &boxes[slot.index()];
    let layout_style = node.layout_style();
    let style = node.style();
    let content_x = layout_box.x.saturating_add(
        layout_style
            .border_width
            .saturating_add(layout_style.padding.left) as i32,
    );
    let content_y = layout_box.y.saturating_add(
        layout_style
            .border_width
            .saturating_add(layout_style.padding.top) as i32,
    );
    let content_width = layout_box
        .width
        .saturating_sub(layout::inset_x(&layout_style));
    let content_height = layout_box
        .height
        .saturating_sub(layout::inset_y(&layout_style));
    let text = match &node.data {
        NodeData::Text { .. } | NodeData::Input { .. } => texts
            .get(&node.id)
            .map(|cached| cached.variants.resolve(node.state)),
        _ => None,
    };
    let caret = match &node.data {
        NodeData::Input { caret, .. } if focused == Some(slot) => texts
            .get(&node.id)
            .map(|cached| cached.variants.base.caret_x(*caret)),
        _ => None,
    };
    let image = match &node.data {
        NodeData::Image(image) => Some(image.clone()),
        _ => None,
    };

    nodes.push(WidgetSnapshotNode {
        id: node.id,
        kind: node.kind(),
        x: layout_box.x,
        y: layout_box.y,
        width: layout_box.width,
        height: layout_box.height,
        border_width: layout_style.border_width,
        content_x,
        content_y,
        content_width,
        content_height,
        state: node.state,
        style,
        text,
        caret,
        image,
        clips: clips.to_vec(),
    });

    let mut child_clips = clips.to_vec();
    if style.clip {
        child_clips.push(ClipRect::new(
            layout_box.x,
            layout_box.y,
            layout_box.width,
            layout_box.height,
            style.radius,
        ));
    }
    for &child in &node.children {
        snapshot_widget(tree, child, boxes, texts, &child_clips, focused, nodes);
    }
}

fn collect_hit(
    tree: &[Node],
    slot: Slot,
    boxes: &[LayoutBox],
    clips: &[ClipRect],
    x: i32,
    y: i32,
    path: &mut Vec<Slot>,
) -> bool {
    let node = &tree[slot.index()];
    if !node.state.visible
        || !contains(&boxes[slot.index()], x, y)
        || !clips.iter().all(|clip| clip.contains(x, y))
    {
        return false;
    }

    let style = node.styles.resolve(node.state);
    let mut child_clips = clips.to_vec();
    if style.clip {
        let layout_box = &boxes[slot.index()];
        child_clips.push(ClipRect::new(
            layout_box.x,
            layout_box.y,
            layout_box.width,
            layout_box.height,
            style.radius,
        ));
    }

    path.push(slot);
    for &child in node.children.iter().rev() {
        if collect_hit(tree, child, boxes, &child_clips, x, y, path) {
            return true;
        }
    }
    true
}

fn contains(layout_box: &LayoutBox, x: i32, y: i32) -> bool {
    let right = i64::from(layout_box.x) + i64::from(layout_box.width);
    let bottom = i64::from(layout_box.y) + i64::from(layout_box.height);
    i64::from(x) >= i64::from(layout_box.x)
        && i64::from(y) >= i64::from(layout_box.y)
        && i64::from(x) < right
        && i64::from(y) < bottom
}

fn common_prefix(left: &[Slot], right: &[Slot]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use super::WidgetError;
    use crate::layout::LayoutError;
    use crate::text::TextError;

    #[test]
    fn widget_error_names_the_wrapped_source() {
        assert_ne!(
            WidgetError::NoLayout.to_string(),
            WidgetError::InvalidScale.to_string()
        );
        assert_eq!(
            WidgetError::from(LayoutError::InvalidDimension).to_string(),
            format!("layout: {}", LayoutError::InvalidDimension)
        );
        assert_eq!(
            WidgetError::from(TextError::AtlasFull).to_string(),
            format!("text: {}", TextError::AtlasFull)
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Left, Side::Top, Side::Right, Side::Bottom];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    ThemeKey,
    Number,
    Px,
    SignedPx,
    Length,
    Color,
    Paint,
    String,
    Bool,
    Keywords(&'static [&'static str]),
    TrackList,
    GridLine,
    AreaTemplate,
    GridArea,
    NumberRange(f64, f64),
    NumberIn(&'static [f64]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatePolicy {
    Forbidden,
    Visual,
    TextWeight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutField {
    Width,
    Height,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    Display,
    FlexDirection,
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridAutoFlow,
    GridTemplateAreas,
    GridArea,
    Padding(Side),
    Gap,
    RowGap,
    ColumnGap,
    BorderWidthSide(Side),
    Justify,
    Align,
    AnchorSide,
    AnchorGap,
    AnchorAlign,
    AnchorFlip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleField {
    Background,
    BorderSide(Side),
    Text,
    Tint,
    BorderRadius,
    Opacity,
    Clip,
    Scroll,
    Cursor,
    ShadowX,
    ShadowY,
    ShadowBlur,
    ShadowColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextField {
    Size,
    Family,
    Align,
    VerticalAlign,
    Weight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Layout(LayoutField),
    Style(StyleField),
    Text(TextField),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyKey {
    Width,
    Height,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    Display,
    FlexDirection,
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridAutoFlow,
    GridTemplateAreas,
    GridArea,
    PaddingSide(Side),
    Gap,
    RowGap,
    ColumnGap,
    BorderWidthSide(Side),
    AnchorSide,
    AnchorGap,
    AnchorAlign,
    AnchorFlip,
    BorderRadius,
    FontSize,
    FontFamily,
    JustifyContent,
    AlignItems,
    Background,
    BorderColorSide(Side),
    Color,
    Tint,
    TextAlign,
    VerticalAlign,
    FontWeight,
    Opacity,
    Clip,
    Scroll,
    Cursor,
    ShadowOffsetX,
    ShadowOffsetY,
    ShadowBlur,
    ShadowColor,
}

#[derive(Clone, Copy)]
pub struct PropertySpec {
    pub key: PropertyKey,
    pub name: &'static str,
    pub field: &'static str,
    pub ty: Type,
    pub in_state: StatePolicy,
    pub target: Target,
    pub animatable: bool,
    pub default: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShorthandParser {
    Box,
    Border,
    BorderColor,
    Shadow,
}

#[derive(Clone, Copy)]
pub struct ShorthandSpec {
    pub name: &'static str,
    pub expands_to: &'static [PropertyKey],
    pub parser: ShorthandParser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleOwner {
    Layout,
    Widget,
    Text,
    Transition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldType {
    Dimension,
    U32,
    I32,
    F32,
    Bool,
    SidesU32,
    Display,
    Direction,
    Justify,
    Align,
    AnchorSide,
    AnchorAlign,
    TrackList,
    GridLine,
    GridAutoFlow,
    AreaTemplate,
    String,
    Paint,
    Color,
    SidesColor,
    Shadow,
    VerticalAlign,
    OptionColor,
    OptionString,
    TextAlign,
    U16,
    Duration,
    Cursor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldDefault {
    Default,
    Zero,
    One,
    True,
    None,
    Text,
    FontSize,
    LineHeight,
    Weight,
    TransparentSides,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TweenKind {
    None,
    Paint,
    Color,
    OptionColor,
    Float,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowPart {
    X,
    Y,
    Blur,
    Color,
}

impl ShadowPart {
    pub const fn name(self) -> &'static str {
        match self {
            ShadowPart::X => "x",
            ShadowPart::Y => "y",
            ShadowPart::Blur => "blur",
            ShadowPart::Color => "color",
        }
    }

    pub const fn ty(self) -> FieldType {
        match self {
            ShadowPart::X | ShadowPart::Y => FieldType::I32,
            ShadowPart::Blur => FieldType::U32,
            ShadowPart::Color => FieldType::Color,
        }
    }

    pub const fn tween(self) -> TweenKind {
        match self {
            ShadowPart::Color => TweenKind::Color,
            _ => TweenKind::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PatchTarget {
    This,
    Side(Side),
    Shadow(ShadowPart),
    Scaled(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidesType {
    Padding,
    BorderWidths,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Converter {
    Raw,
    Dimension,
    Sides(SidesType),
    Float,
    String,
    Paint,
    Color,
    Duration,
    Tracks,
    GridLine,
    AreaTemplate,
    Display,
    Direction,
    Justify,
    Align,
    AnchorSide,
    AnchorAlign,
    VerticalAlign,
    TextAlign,
    AutoFlow,
    Cursor,
}

#[derive(Clone, Copy)]
pub struct PatchField {
    pub name: &'static str,
    pub order: u16,
    pub target: PatchTarget,
    pub converter: Converter,
}

#[derive(Clone, Copy)]
pub struct FieldSpec {
    pub owner: StyleOwner,
    pub name: &'static str,
    pub ty: FieldType,
    pub default: FieldDefault,
    pub tween: TweenKind,
    pub patches: &'static [PatchField],
}

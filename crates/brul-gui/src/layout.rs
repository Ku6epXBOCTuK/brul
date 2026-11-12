use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Dimension {
    #[default]
    Auto,
    Px(u32),
    Percent(f32),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Padding {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Padding {
    pub const fn all(value: u32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutStyle {
    pub width: Dimension,
    pub height: Dimension,
    pub padding: Padding,
    pub border_width: u32,
    pub spacing: u32,
    pub stretch: f32,
    pub justify: Justify,
    pub align: Align,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Offset {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScrollOffset(i32);

impl ScrollOffset {
    pub const ZERO: Self = Self(0);

    pub const fn new(pixels: i32) -> Self {
        Self(pixels)
    }

    pub const fn pixels(self) -> i32 {
        self.0
    }

    pub fn saturating_add(self, pixels: i32) -> Self {
        Self(self.0.saturating_add(pixels))
    }

    pub fn clamp(self, min: i32, max: i32) -> Self {
        Self(self.0.clamp(min, max))
    }
}

pub enum LayoutNode {
    Leaf {
        style: LayoutStyle,
        size: Size,
        offset: Option<Offset>,
    },
    Container {
        style: LayoutStyle,
        direction: Direction,
        children: Vec<LayoutNode>,
        offset: Option<Offset>,
        scroll_offset: ScrollOffset,
    },
}

impl LayoutNode {
    pub fn leaf(width: u32, height: u32) -> Self {
        Self::Leaf {
            style: LayoutStyle::default(),
            size: Size { width, height },
            offset: None,
        }
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self::Container {
            style: LayoutStyle::default(),
            direction: Direction::Row,
            children,
            offset: None,
            scroll_offset: ScrollOffset::ZERO,
        }
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self::Container {
            style: LayoutStyle::default(),
            direction: Direction::Column,
            children,
            offset: None,
            scroll_offset: ScrollOffset::ZERO,
        }
    }

    pub fn with_style(mut self, style: LayoutStyle) -> Self {
        match &mut self {
            Self::Leaf { style: current, .. } | Self::Container { style: current, .. } => {
                *current = style
            }
        }
        self
    }

    pub fn with_offset(mut self, x: i32, y: i32) -> Self {
        let offset = Some(Offset { x, y });
        match &mut self {
            Self::Leaf {
                offset: current, ..
            }
            | Self::Container {
                offset: current, ..
            } => *current = offset,
        }
        self
    }

    pub fn with_scroll(mut self, scroll: ScrollOffset) -> Self {
        if let Self::Container {
            scroll_offset: current,
            ..
        } = &mut self
        {
            *current = scroll;
        }
        self
    }

    fn style(&self) -> &LayoutStyle {
        match self {
            Self::Leaf { style, .. } | Self::Container { style, .. } => style,
        }
    }

    fn offset(&self) -> Option<Offset> {
        match self {
            Self::Leaf { offset, .. } | Self::Container { offset, .. } => *offset,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LayoutBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub children: Vec<LayoutBox>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutError {
    InvalidDimension,
    InvalidStretch,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidDimension => "invalid size: percent must be finite and non-negative",
            Self::InvalidStretch => "invalid stretch: factor must be finite and non-negative",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for LayoutError {}

#[derive(Clone, Copy, Debug, Default)]
struct Constraints {
    width: Option<u32>,
    height: Option<u32>,
}

#[derive(Clone, Debug)]
struct Measured {
    width: u32,
    height: u32,
    auto_width: bool,
    auto_height: bool,
    children: Vec<Measured>,
}

pub fn layout(root: &LayoutNode, viewport: Size) -> Result<LayoutBox, LayoutError> {
    let constraints = Constraints {
        width: Some(viewport.width),
        height: Some(viewport.height),
    };
    let measured = measure(root, constraints)?;
    let size = Size {
        width: measured.width,
        height: measured.height,
    };
    let mut result = LayoutBox {
        width: size.width,
        height: size.height,
        ..LayoutBox::default()
    };
    arrange(root, &measured, size, 0, 0, &mut result);
    Ok(result)
}

fn measure(node: &LayoutNode, constraints: Constraints) -> Result<Measured, LayoutError> {
    match node {
        LayoutNode::Leaf { style, size, .. } => measure_leaf(*style, *size, constraints),
        LayoutNode::Container {
            style,
            direction,
            children,
            ..
        } => measure_container(*style, *direction, children, constraints),
    }
}

fn measure_leaf(
    style: LayoutStyle,
    size: Size,
    constraints: Constraints,
) -> Result<Measured, LayoutError> {
    validate_stretch(&style)?;
    let resolved_width = resolve(style.width, constraints.width)?;
    let resolved_height = resolve(style.height, constraints.height)?;
    let width = resolved_width.unwrap_or_else(|| size.width.saturating_add(inset_x(&style)));
    let height = resolved_height.unwrap_or_else(|| size.height.saturating_add(inset_y(&style)));

    Ok(Measured {
        width,
        height,
        auto_width: resolved_width.is_none(),
        auto_height: resolved_height.is_none(),
        children: Vec::new(),
    })
}

fn measure_container(
    style: LayoutStyle,
    direction: Direction,
    children: &[LayoutNode],
    constraints: Constraints,
) -> Result<Measured, LayoutError> {
    validate_stretch(&style)?;
    let resolved_width = resolve(style.width, constraints.width)?;
    let resolved_height = resolve(style.height, constraints.height)?;
    let child_constraints = Constraints {
        width: resolved_width
            .or(constraints.width)
            .map(|width| width.saturating_sub(inset_x(&style))),
        height: resolved_height
            .or(constraints.height)
            .map(|height| height.saturating_sub(inset_y(&style))),
    };
    let mut measured_children = Vec::with_capacity(children.len());
    for child in children {
        measured_children.push(measure(child, child_constraints)?);
    }

    let flow = flow_indices(children);
    let gap = gap_size(&style, flow.len());
    let (auto_width, auto_height) = auto_size(&style, direction, &measured_children, &flow, gap);
    let width = resolved_width.unwrap_or(auto_width);
    let height = resolved_height.unwrap_or(auto_height);

    Ok(Measured {
        width,
        height,
        auto_width: resolved_width.is_none(),
        auto_height: resolved_height.is_none(),
        children: measured_children,
    })
}

fn auto_size(
    style: &LayoutStyle,
    direction: Direction,
    children: &[Measured],
    flow: &[usize],
    gap: u32,
) -> (u32, u32) {
    let sum_main = flow
        .iter()
        .fold(0u32, |total, &index| {
            total.saturating_add(main_size(child_size(&children[index]), direction))
        })
        .saturating_add(gap);
    let max_cross = flow.iter().fold(0u32, |max, &index| {
        max.max(cross_size(child_size(&children[index]), direction))
    });

    match direction {
        Direction::Row => (
            sum_main.saturating_add(inset_x(style)),
            max_cross.saturating_add(inset_y(style)),
        ),
        Direction::Column => (
            max_cross.saturating_add(inset_x(style)),
            sum_main.saturating_add(inset_y(style)),
        ),
    }
}

fn arrange(
    node: &LayoutNode,
    measured: &Measured,
    size: Size,
    x: i32,
    y: i32,
    out: &mut LayoutBox,
) {
    out.x = x;
    out.y = y;
    out.width = size.width;
    out.height = size.height;

    let LayoutNode::Container {
        style,
        direction,
        children,
        scroll_offset,
        ..
    } = node
    else {
        return;
    };

    let content = content_size(style, size);
    let flow = flow_indices(children);
    let mut sizes: Vec<Size> = measured.children.iter().map(child_size).collect();
    let gap = gap_size(style, flow.len());
    let mut used_main = flow.iter().fold(0u32, |total, &index| {
        total.saturating_add(main_size(sizes[index], *direction))
    });
    let content_main = main_size(content, *direction);
    let extra = content_main.saturating_sub(used_main.saturating_add(gap));

    stretch_main(children, &flow, &mut sizes, *direction, extra);
    align_cross(
        children,
        measured,
        &flow,
        &mut sizes,
        *direction,
        cross_size(content, *direction),
        style.align,
    );

    used_main = flow.iter().fold(0u32, |total, &index| {
        total.saturating_add(main_size(sizes[index], *direction))
    });
    let remaining = content_main.saturating_sub(used_main.saturating_add(gap));
    let (start, between) = justify_offsets(style.justify, remaining, flow.len());

    let origin_x = clamp_i32(x as i64 + style.border_width as i64 + style.padding.left as i64);
    let origin_y = clamp_i32(y as i64 + style.border_width as i64 + style.padding.top as i64);
    let content_cross = cross_size(content, *direction);
    let mut spots: Vec<Option<(i32, i32)>> = vec![None; children.len()];
    let mut cursor = i64::from(start);

    for &index in &flow {
        let child_cross = cross_size(sizes[index], *direction);
        let cross_offset = match style.align {
            Align::Start | Align::Stretch => 0,
            Align::Center => (i64::from(content_cross) - i64::from(child_cross)) / 2,
            Align::End => i64::from(content_cross) - i64::from(child_cross),
        };
        let position = match direction {
            Direction::Row => (
                i64::from(origin_x) + cursor,
                i64::from(origin_y) + cross_offset,
            ),
            Direction::Column => (
                i64::from(origin_x) + cross_offset,
                i64::from(origin_y) + cursor,
            ),
        };
        spots[index] = Some((clamp_i32(position.0), clamp_i32(position.1)));
        cursor += i64::from(main_size(sizes[index], *direction))
            + i64::from(style.spacing)
            + i64::from(between);
    }

    for (index, child) in children.iter().enumerate() {
        if let Some(offset) = child.offset() {
            spots[index] = Some((
                clamp_i32(i64::from(origin_x) + i64::from(offset.x)),
                clamp_i32(i64::from(origin_y) + i64::from(offset.y)),
            ));
        } else if scroll_offset.pixels() != 0 {
            let shift = i64::from(scroll_offset.pixels());
            let Some((x, y)) = spots[index] else {
                continue;
            };
            spots[index] = Some(match direction {
                Direction::Row => (clamp_i32(i64::from(x) - shift), y),
                Direction::Column => (x, clamp_i32(i64::from(y) - shift)),
            });
        }
    }

    for (index, child) in children.iter().enumerate() {
        let (child_x, child_y) = spots[index].expect("layout position");
        let mut child_box = LayoutBox::default();
        arrange(
            child,
            &measured.children[index],
            sizes[index],
            child_x,
            child_y,
            &mut child_box,
        );
        out.children.push(child_box);
    }
}

fn stretch_main(
    children: &[LayoutNode],
    flow: &[usize],
    sizes: &mut [Size],
    direction: Direction,
    extra: u32,
) {
    let total: f64 = flow
        .iter()
        .map(|&index| f64::from(children[index].style().stretch))
        .filter(|weight| *weight > 0.0)
        .sum();
    if extra == 0 || total == 0.0 {
        return;
    }

    let mut allocated = 0u32;
    let mut accumulated = 0.0f64;
    for &index in flow {
        let weight = f64::from(children[index].style().stretch);
        if weight <= 0.0 {
            continue;
        }
        accumulated += weight;
        let target = (f64::from(extra) * accumulated / total).round() as u32;
        let addition = target.saturating_sub(allocated);
        allocated = target;
        let current = main_size(sizes[index], direction);
        *main_size_mut(&mut sizes[index], direction) = current.saturating_add(addition);
    }
}

fn align_cross(
    children: &[LayoutNode],
    measured: &Measured,
    flow: &[usize],
    sizes: &mut [Size],
    direction: Direction,
    content_cross: u32,
    align: Align,
) {
    if align != Align::Stretch {
        return;
    }

    for &index in flow {
        let auto_cross = match direction {
            Direction::Row => measured.children[index].auto_height,
            Direction::Column => measured.children[index].auto_width,
        };
        if auto_cross && children[index].offset().is_none() {
            *cross_size_mut(&mut sizes[index], direction) = content_cross;
        }
    }
}

fn justify_offsets(justify: Justify, remaining: u32, count: usize) -> (u32, u32) {
    match justify {
        Justify::Start => (0, 0),
        Justify::Center => (remaining / 2, 0),
        Justify::End => (remaining, 0),
        Justify::SpaceBetween if count > 1 => (0, remaining / (count as u32 - 1)),
        Justify::SpaceBetween => (0, 0),
    }
}

fn flow_indices(children: &[LayoutNode]) -> Vec<usize> {
    children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| child.offset().is_none().then_some(index))
        .collect()
}

fn gap_size(style: &LayoutStyle, count: usize) -> u32 {
    style.spacing.saturating_mul(count.saturating_sub(1) as u32)
}

fn child_size(measured: &Measured) -> Size {
    Size {
        width: measured.width,
        height: measured.height,
    }
}

fn main_size(size: Size, direction: Direction) -> u32 {
    match direction {
        Direction::Row => size.width,
        Direction::Column => size.height,
    }
}

fn cross_size(size: Size, direction: Direction) -> u32 {
    match direction {
        Direction::Row => size.height,
        Direction::Column => size.width,
    }
}

fn main_size_mut(size: &mut Size, direction: Direction) -> &mut u32 {
    match direction {
        Direction::Row => &mut size.width,
        Direction::Column => &mut size.height,
    }
}

fn cross_size_mut(size: &mut Size, direction: Direction) -> &mut u32 {
    match direction {
        Direction::Row => &mut size.height,
        Direction::Column => &mut size.width,
    }
}

pub(crate) fn inset_x(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.left)
        .saturating_add(style.padding.right)
}

pub(crate) fn inset_y(style: &LayoutStyle) -> u32 {
    style
        .border_width
        .saturating_mul(2)
        .saturating_add(style.padding.top)
        .saturating_add(style.padding.bottom)
}

fn content_size(style: &LayoutStyle, size: Size) -> Size {
    Size {
        width: size.width.saturating_sub(inset_x(style)),
        height: size.height.saturating_sub(inset_y(style)),
    }
}

pub(crate) fn resolve(dimension: Dimension, base: Option<u32>) -> Result<Option<u32>, LayoutError> {
    match dimension {
        Dimension::Auto => Ok(None),
        Dimension::Px(value) => Ok(Some(value)),
        Dimension::Percent(percent) => {
            if !percent.is_finite() || percent < 0.0 {
                return Err(LayoutError::InvalidDimension);
            }
            Ok(base.map(|base| (f64::from(base) * f64::from(percent) / 100.0).round() as u32))
        }
    }
}

fn validate_stretch(style: &LayoutStyle) -> Result<(), LayoutError> {
    if !style.stretch.is_finite() || style.stretch < 0.0 {
        return Err(LayoutError::InvalidStretch);
    }
    Ok(())
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::LayoutError;

    #[test]
    fn every_layout_error_has_its_own_message() {
        let messages = [LayoutError::InvalidDimension, LayoutError::InvalidStretch]
            .map(|error| error.to_string());

        assert!(messages.iter().all(|message| !message.is_empty()));
        assert_ne!(messages[0], messages[1]);
    }
}

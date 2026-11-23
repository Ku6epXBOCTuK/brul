//! BRUL rendering layer: primitives, text, layout and widgets.
//!
//! ```
//! use brul_gui::{Color, Paint, Rect};
//!
//! let accent = Color::rgb(59, 130, 246);
//! let rect = Rect::new(8, 8, 32, 24)
//!     .with_radius(6)
//!     .with_fill(Paint::Color(accent));
//! assert_eq!(rect.sample(20, 20).fill_coverage, 1.0);
//! assert_eq!(rect.sample(8, 8).fill_coverage, 0.0);
//! assert!(rect.sample(9, 9).fill_coverage > 0.0);
//! ```
//!
//! ```
//! use brul_gui::{Color, Texture};
//!
//! let tinted = Texture::from_rgba(1, 1, &[100, 50, 25, 255])
//!     .expect("texture")
//!     .with_tint(Color::rgb(255, 128, 255));
//! assert_eq!(tinted.color_at(0, 0), Some(Color::rgb(100, 25, 25)));
//! ```
//!
//! ```
//! use brul_gui::{TextEngine, TextStyle};
//!
//! let mut engine = TextEngine::new();
//! let layout = engine
//!     .layout("Hello", &TextStyle::new(16.0), None, 1.0)
//!     .expect("layout");
//! assert_eq!(layout.glyphs.len(), 5);
//! ```
//!
//! ```
//! use brul_gui::{Dimension, LayoutNode, LayoutStyle, Padding, Size, layout};
//!
//! let root = LayoutNode::column(vec![
//!     LayoutNode::leaf(50, 20),
//!     LayoutNode::leaf(50, 20),
//! ])
//! .with_style(LayoutStyle {
//!     width: Dimension::Px(200),
//!     height: Dimension::Px(120),
//!     padding: Padding::all(10),
//!     gap: 5,
//!     ..LayoutStyle::default()
//! });
//! let placed = layout(&root, Size { width: 200, height: 120 }).expect("layout");
//! assert_eq!(placed.children[1].y, 35);
//! ```
//!
//! ```
//! use brul_gui::{Direction, Size, TextEngine, Widget, WidgetTree};
//!
//! let root = Widget::view(Direction::Column, vec![Widget::text("Hello")]);
//! let mut tree = WidgetTree::new(root);
//! let mut engine = TextEngine::new();
//! tree.layout(&mut engine, Size { width: 160, height: 80 }, 1.0)
//!     .expect("layout");
//! let snapshot = tree.snapshot().expect("snapshot");
//! assert_eq!(snapshot.nodes.len(), 2);
//! ```

mod animation;
mod atlas;
mod border;
mod color;
mod cursor;
mod draw;
mod effect;
mod gradient;
mod image;
mod paint;
mod rect;
pub mod style;
mod text;
mod texture;
mod theme;
mod widget;

use brul_layout as layout;

pub use animation::{Track, Tween, ease_out};
pub use atlas::{Atlas, AtlasFormat, AtlasSlot};
pub use border::{Border, BorderArrays};
pub use color::Color;
pub use cursor::Cursor;
pub type BorderColors = Sides<Color>;
pub use draw::{DrawCommand, GlyphCommand, ImageCommand, RectCommand, ShapeCommand, draw_commands};
pub use effect::{EFFECT_SLOTS, EffectSpec};
pub use gradient::{Gradient, Paint};
pub use image::{Image, Vector};
pub use layout::{
    Align, AnchorAlign, AnchorSide, Area, AreaTemplate, BorderWidths, Dimension, Direction,
    Display, GridAutoFlow, GridLine, Justify, LayoutBox, LayoutError, LayoutNode, LayoutPatch,
    LayoutStyle, LinePlacement, Offset, Padding, RepeatCount, Sides, Size, TrackSize, layout,
};
pub use paint::Frame;
pub use rect::{ClipRect, Rect, RectSample, Shadow};
pub use text::{
    CaretPosition, CaretStop, PlacedGlyph, SelectionRect, TextAlign, TextEngine, TextError,
    TextLayout, TextSize, TextStyle, VerticalAlign,
};
pub use texture::{ExternalTexture, Revision, Texture, TextureError, TextureFrame};
pub use theme::{Theme, UnknownTheme};
pub use widget::{
    Anchor, CaretArea, Class, Dirty, FunctionName, KeyModifiers, KeyPress, Layers, PlatformViewId,
    PlatformViewSpec, StyleSet, StyleState, TextStylePatch, TextStyleSet, TimerOwner, Transitions,
    Widget, WidgetError, WidgetEvent, WidgetId, WidgetKind, WidgetSnapshot, WidgetSnapshotNode,
    WidgetState, WidgetStyle, WidgetStylePatch, WidgetTree,
};

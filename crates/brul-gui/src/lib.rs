//! Слой отрисовки BRUL: примитивы и текст.
//!
//! # Модель
//!
//! Прямоугольник — [`Rect`]: целые физические координаты, ширина и высота,
//! радиус, заливка и граница. Граница лежит внутри (border-box), радиус
//! ограничен половиной меньшей стороны. [`Rect::sample`] берёт центр пикселя и
//! говорит, что в нём: заливка, граница или пусто; это эталон геометрии, пока
//! отрисовщика нет.
//!
//! ```
//! use brul_gui::{Color, Rect, RectSample};
//!
//! let accent = Color::rgb(59, 130, 246);
//! let rect = Rect::new(8, 8, 32, 24).with_radius(6).with_fill(accent);
//! assert_eq!(rect.sample(20, 20), RectSample::Fill(accent));
//! assert_eq!(rect.sample(8, 8), RectSample::Outside);
//! ```
//!
//! [`TextEngine`] держит системные шрифты, кэш растеризации и атлас. Текст
//! задаётся [`TextStyle`] в логических пикселях; `scale` переводит их в
//! физические пиксели экрана. Размеры в [`TextLayout`] и координаты глифов —
//! физические целые пиксели.
//!
//! Атлас — одна текстура RGBA8 1024×1024. Глифы кладутся полками с зазором в
//! 1 px и переиспользуются по ключу растеризации: размер шрифта и DPI входят в
//! ключ, поэтому один и тот же символ при разных масштабах — разные ячейки.
//! Маска хранится белой с альфой, цветной эмодзи — как есть; [`AtlasSlot`]
//! говорит, что именно лежит в ячейке.
//!
//! ```
//! use brul_gui::{TextEngine, TextStyle};
//!
//! let mut engine = TextEngine::new();
//! let layout = engine
//!     .layout("Привет", &TextStyle::new(16.0), None, 1.0)
//!     .expect("layout");
//! assert_eq!(layout.glyphs.len(), 6);
//! ```
//!
//! # Пределы MVP
//!
//! Окна и GPU ещё нет: наружу отдаются геометрия прямоугольников, атлас и
//! список глифов. Атлас не растёт и не чистится; когда места нет,
//! [`TextEngine::layout`] возвращает [`TextError::AtlasFull`]. Субпиксельное
//! сглаживание сведено к маске, цвет обычных глифов задаёт отрисовщик, а не
//! движок текста.

mod atlas;
mod color;
mod rect;
mod text;

pub use atlas::{Atlas, AtlasFormat, AtlasSlot};
pub use color::Color;
pub use rect::{Rect, RectSample};
pub use text::{PlacedGlyph, TextAlign, TextEngine, TextError, TextLayout, TextSize, TextStyle};

//! Слой отрисовки BRUL: примитивы, текст, окно, лейаут и виджеты.
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
//! Картинка — [`Texture`]: PNG декодируется в RGBA8; [`Texture::color_at`] отдаёт
//! пиксель с учётом tint. Tint умножает RGBA на цвет, непрозрачный tint не меняет
//! прозрачность.
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
//! Окно — [`window::run`] с [`window::WindowConfig`]: без рамки, прозрачное,
//! фиксированного размера. [`window::run_tree`] принимает [`WidgetTree`] и
//! колбэк событий: раскладывает дерево, собирает [`WidgetSnapshot`], рисует его в
//! [`Frame`] на CPU и отдаёт кадр на wgpu-поверхность одной текстурой. Меняются
//! только dirty-регионы; размер и DPI перерисовывают весь кадр. Конфиг задан в
//! логических пикселях, поверхность — в физических; масштаб берётся у окна.
//! Высота хедера задаёт зону перетаскивания, Esc и закрытие окна завершают цикл.
//! winit 0.31-beta прибит точной версией и спрятан в модуле `window`.
//!
//! ```no_run
//! use brul_gui::window::{WindowConfig, run};
//!
//! run(WindowConfig::default()).expect("window");
//! ```
//!
//! Лейаут — [`layout`]: меряет листья снизу вверх, затем размещает контейнеры
//! сверху вниз. [`Dimension`] задаёт auto/px/%, [`LayoutStyle::stretch`] делит
//! остаток главной оси, `justify` и `align` двигают по главной и поперечной.
//! Координаты и размеры результата — целые физические пиксели.
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
//!     spacing: 5,
//!     ..LayoutStyle::default()
//! });
//! let placed = layout(&root, Size { width: 200, height: 120 }).expect("layout");
//! assert_eq!(placed.children[1].y, 35);
//! ```
//!
//! Виджеты — [`Widget`]: View/Row/Column, Text и Image. [`WidgetTree`] хранит
//! состояния `hover`/`pressed`/`checked`/`selected`/`disabled`/`visible`,
//! раскладывает дерево (Text меряется через [`TextEngine`]) и отдаёт
//! [`WidgetSnapshot`] — плоский список узлов с геометрией, выбранным стилем,
//! глифами и текстурой. [`StyleSet`] переключает снимок стиля по состоянию;
//! события указателя возвращаются списком [`WidgetEvent`] и всплывают от листа
//! к корню.
//!
//! ```
//! use brul_gui::{Direction, Size, TextEngine, Widget, WidgetTree};
//!
//! let root = Widget::view(Direction::Column, vec![Widget::text("Привет")]);
//! let mut tree = WidgetTree::new(root);
//! let mut engine = TextEngine::new();
//! tree.layout(&mut engine, Size { width: 160, height: 80 }, 1.0)
//!     .expect("layout");
//! let snapshot = tree.snapshot().expect("snapshot");
//! assert_eq!(snapshot.nodes.len(), 2);
//! ```
//!
//! # Пределы MVP
//!
//! Окно рисует снапшот через CPU-кадр и один GPU-blit: отдельного GPU-рендера
//! примитивов нет, картинки масштабируются nearest-neighbor, клип только по
//! границам кадра, а не по родителю. Стили задаются вручную через [`StyleSet`];
//! CSS-парсер и снапшоты классов появятся позже. Декодируется только PNG. Атлас
//! не растёт и не чистится; когда места нет, [`TextEngine::layout`] возвращает
//! [`TextError::AtlasFull`]. Субпиксельное сглаживание сведено к маске, цвет
//! обычных глифов задаёт отрисовщик, а не движок текста. Режим прозрачности
//! выбирается из возможностей поверхности; если остаётся `Opaque`, окно будет
//! непрозрачным.

mod atlas;
mod color;
mod layout;
mod paint;
mod rect;
mod text;
mod texture;
mod widget;
pub mod window;

pub use atlas::{Atlas, AtlasFormat, AtlasSlot};
pub use color::Color;
pub use layout::{
    Align, Dimension, Direction, Justify, LayoutBox, LayoutError, LayoutNode, LayoutStyle, Offset,
    Padding, Size, layout,
};
pub use paint::Frame;
pub use rect::{Rect, RectSample};
pub use text::{PlacedGlyph, TextAlign, TextEngine, TextError, TextLayout, TextSize, TextStyle};
pub use texture::{Texture, TextureError};
pub use widget::{
    Dirty, StyleSet, StyleState, Widget, WidgetError, WidgetEvent, WidgetId, WidgetKind,
    WidgetSnapshot, WidgetSnapshotNode, WidgetState, WidgetStyle, WidgetTree,
};

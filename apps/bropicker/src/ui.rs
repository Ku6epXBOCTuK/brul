use crate::{BrowserEntry, Theme};
use brul_gui::{
    Align, Color, Dimension, Direction, Justify, LayoutStyle, Padding, StyleSet, StyleState,
    TextAlign, TextStyle, Widget, WidgetEvent, WidgetId, WidgetStyle, WidgetTree,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub item: Color,
    pub hover: Color,
    pub selected: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_text: Color,
}

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Light => Palette {
            background: Color::rgb(244, 246, 249),
            surface: Color::rgb(255, 255, 255),
            item: Color::rgb(238, 241, 246),
            hover: Color::rgb(230, 235, 242),
            selected: Color::rgb(219, 232, 252),
            border: Color::rgb(214, 220, 229),
            text: Color::rgb(23, 28, 36),
            muted: Color::rgb(100, 112, 128),
            accent: Color::rgb(37, 99, 235),
            accent_text: Color::rgb(255, 255, 255),
        },
        Theme::Dark | Theme::System => Palette {
            background: Color::rgb(20, 23, 29),
            surface: Color::rgb(28, 32, 40),
            item: Color::rgb(35, 40, 49),
            hover: Color::rgb(43, 49, 60),
            selected: Color::rgb(45, 58, 78),
            border: Color::rgb(51, 58, 70),
            text: Color::rgb(240, 243, 248),
            muted: Color::rgb(152, 162, 179),
            accent: Color::rgb(96, 165, 250),
            accent_text: Color::rgb(10, 18, 30),
        },
    }
}

pub fn demo_browsers() -> Vec<BrowserEntry> {
    vec![
        BrowserEntry {
            name: "Firefox".to_string(),
            path: "firefox".to_string(),
            flags: String::new(),
            icon: String::new(),
            emoji: "🦊".to_string(),
            is_default: true,
        },
        BrowserEntry {
            name: "Chrome".to_string(),
            path: "chrome".to_string(),
            flags: String::new(),
            icon: String::new(),
            emoji: "🌐".to_string(),
            is_default: false,
        },
        BrowserEntry {
            name: "Brave".to_string(),
            path: "brave".to_string(),
            flags: String::new(),
            icon: String::new(),
            emoji: "🦁".to_string(),
            is_default: false,
        },
        BrowserEntry {
            name: "Edge".to_string(),
            path: "msedge".to_string(),
            flags: String::new(),
            icon: String::new(),
            emoji: "🌊".to_string(),
            is_default: false,
        },
    ]
}

#[derive(Clone, Debug)]
pub struct CheckboxIds {
    pub row: WidgetId,
    pub mark: WidgetId,
}

#[derive(Clone, Debug)]
pub struct PickerIds {
    pub rows: Vec<WidgetId>,
    pub remember: CheckboxIds,
    pub always_ask: CheckboxIds,
}

pub struct PickerView {
    pub root: Widget,
    pub ids: PickerIds,
}

pub fn build_view(
    browsers: &[BrowserEntry],
    url: &str,
    theme: Theme,
    remember_choice: bool,
    always_ask: bool,
) -> PickerView {
    let colors = palette(theme);
    let selected = browsers
        .iter()
        .position(|browser| browser.is_default)
        .unwrap_or(0);

    let mut rows = Vec::with_capacity(browsers.len());
    let mut items = Vec::with_capacity(browsers.len());
    for (index, browser) in browsers.iter().enumerate() {
        let (item, id) = item(browser, colors, index == selected);
        rows.push(id);
        items.push(item);
    }

    let list = Widget::column(items).with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        stretch: 1.0,
        spacing: 8,
        ..LayoutStyle::default()
    });
    let (footer, remember, always_ask_ids) = footer(colors, remember_choice, always_ask);
    let root = Widget::column(vec![header(url, colors), list, footer])
        .with_layout(LayoutStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            padding: Padding::all(16),
            spacing: 12,
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: colors.background,
            radius: 22,
            ..WidgetStyle::default()
        }));

    PickerView {
        root,
        ids: PickerIds {
            rows,
            remember,
            always_ask: always_ask_ids,
        },
    }
}

pub fn apply_events(
    tree: &mut WidgetTree,
    ids: &PickerIds,
    events: &[WidgetEvent],
) -> Option<usize> {
    let mut selected = None;

    for event in events {
        let WidgetEvent::Click(id) = *event else {
            continue;
        };

        if id == ids.remember.row {
            toggle_checkbox(tree, &ids.remember);
        } else if id == ids.always_ask.row {
            toggle_checkbox(tree, &ids.always_ask);
        } else if let Some(index) = ids.rows.iter().position(|row| *row == id) {
            for (row_index, row) in ids.rows.iter().enumerate() {
                tree.set_selected(*row, row_index == index);
            }
            selected = Some(index);
        }
    }

    selected
}

fn toggle_checkbox(tree: &mut WidgetTree, ids: &CheckboxIds) {
    let next = tree.state(ids.mark).is_none_or(|state| !state.checked);
    tree.set_checked(ids.mark, next);
    tree.set_checked(ids.row, next);
}

fn header(url: &str, colors: Palette) -> Widget {
    Widget::column(vec![
        label("bropicker", 12.0, 600, colors.muted),
        label(url, 16.0, 600, colors.text),
    ])
    .with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(56),
        padding: Padding {
            left: 14,
            top: 8,
            right: 14,
            bottom: 8,
        },
        spacing: 2,
        ..LayoutStyle::default()
    })
    .with_styles(StyleSet::new(WidgetStyle {
        background: colors.surface,
        radius: 14,
        ..WidgetStyle::default()
    }))
}

fn item(browser: &BrowserEntry, colors: Palette, selected: bool) -> (Widget, WidgetId) {
    let icon = icon(browser, colors);
    let details = Widget::column(vec![
        label(&browser.name, 15.0, 600, colors.text),
        label(&browser.path, 12.0, 400, colors.muted),
    ])
    .with_layout(LayoutStyle {
        stretch: 1.0,
        spacing: 2,
        ..LayoutStyle::default()
    });

    let badge = label("default", 11.0, 600, colors.accent_text)
        .with_layout(LayoutStyle {
            padding: Padding {
                left: 8,
                top: 3,
                right: 8,
                bottom: 3,
            },
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: colors.accent,
            text: colors.accent_text,
            radius: 8,
            ..WidgetStyle::default()
        }))
        .visible(browser.is_default);

    let row = Widget::row(vec![icon, details, badge])
        .with_layout(LayoutStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(72),
            padding: Padding {
                left: 12,
                top: 10,
                right: 12,
                bottom: 10,
            },
            border_width: 1,
            spacing: 12,
            align: Align::Center,
            ..LayoutStyle::default()
        })
        .with_styles(
            StyleSet::new(item_style(colors.item, colors.border))
                .with(StyleState::Hovered, item_style(colors.hover, colors.border))
                .with(
                    StyleState::Pressed,
                    item_style(colors.selected, colors.accent),
                )
                .with(
                    StyleState::Selected,
                    item_style(colors.selected, colors.accent),
                ),
        )
        .selected(selected);

    let id = row.id();
    (row, id)
}

fn item_style(background: Color, border: Color) -> WidgetStyle {
    WidgetStyle {
        background,
        border,
        radius: 14,
        ..WidgetStyle::default()
    }
}

fn icon(browser: &BrowserEntry, colors: Palette) -> Widget {
    let fallback = browser
        .name
        .chars()
        .next()
        .map(|character| character.to_uppercase().to_string())
        .unwrap_or_default();
    let text = if browser.emoji.is_empty() {
        fallback
    } else {
        browser.emoji.clone()
    };

    Widget::row(vec![label(&text, 22.0, 600, colors.accent_text)])
        .with_layout(LayoutStyle {
            width: Dimension::Px(44),
            height: Dimension::Px(44),
            justify: Justify::Center,
            align: Align::Center,
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: colors.accent,
            radius: 12,
            ..WidgetStyle::default()
        }))
}

fn footer(
    colors: Palette,
    remember_choice: bool,
    always_ask: bool,
) -> (Widget, CheckboxIds, CheckboxIds) {
    let (remember, remember_ids) = checkbox("Remember choice", colors, remember_choice);
    let (always_ask, always_ask_ids) = checkbox("Always ask", colors, always_ask);
    let row = Widget::row(vec![remember, always_ask])
        .with_layout(LayoutStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(44),
            padding: Padding {
                left: 12,
                top: 8,
                right: 12,
                bottom: 8,
            },
            spacing: 24,
            align: Align::Center,
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: colors.surface,
            radius: 12,
            ..WidgetStyle::default()
        }));

    (row, remember_ids, always_ask_ids)
}

fn checkbox(text: &str, colors: Palette, checked: bool) -> (Widget, CheckboxIds) {
    let mark = Widget::view(Direction::Row, Vec::new())
        .with_layout(LayoutStyle {
            width: Dimension::Px(16),
            height: Dimension::Px(16),
            border_width: 1,
            ..LayoutStyle::default()
        })
        .with_styles(
            StyleSet::new(WidgetStyle {
                background: colors.item,
                border: colors.border,
                radius: 4,
                ..WidgetStyle::default()
            })
            .with(
                StyleState::Checked,
                WidgetStyle {
                    background: colors.accent,
                    border: colors.accent,
                    radius: 4,
                    ..WidgetStyle::default()
                },
            ),
        )
        .checked(checked);
    let mark_id = mark.id();

    let row = Widget::row(vec![mark, label(text, 12.0, 500, colors.muted)])
        .with_layout(LayoutStyle {
            spacing: 8,
            align: Align::Center,
            ..LayoutStyle::default()
        })
        .checked(checked);
    let row_id = row.id();

    (
        row,
        CheckboxIds {
            row: row_id,
            mark: mark_id,
        },
    )
}

fn label(text: &str, size: f32, weight: u16, color: Color) -> Widget {
    let mut style = TextStyle::new(size);
    style.weight = weight;
    style.align = TextAlign::Left;

    Widget::styled_text(text.to_string(), style).with_styles(StyleSet::new(WidgetStyle {
        text: color,
        ..WidgetStyle::default()
    }))
}

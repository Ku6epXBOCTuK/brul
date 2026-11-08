use crate::{BrowserEntry, Theme};
use brul_gui::{
    Align, Color, Dimension, Direction, Justify, LayoutStyle, Padding, StyleSet, TextAlign,
    TextStyle, Widget, WidgetStyle,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub item: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_text: Color,
}

pub fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Dark | Theme::System => Palette {
            background: Color::rgb(20, 23, 29),
            surface: Color::rgb(28, 32, 40),
            item: Color::rgb(35, 40, 49),
            border: Color::rgb(51, 58, 70),
            text: Color::rgb(240, 243, 248),
            muted: Color::rgb(152, 162, 179),
            accent: Color::rgb(96, 165, 250),
            accent_text: Color::rgb(10, 18, 30),
        },
        Theme::Light => Palette {
            background: Color::rgb(244, 246, 249),
            surface: Color::rgb(255, 255, 255),
            item: Color::rgb(238, 241, 246),
            border: Color::rgb(214, 220, 229),
            text: Color::rgb(23, 28, 36),
            muted: Color::rgb(100, 112, 128),
            accent: Color::rgb(37, 99, 235),
            accent_text: Color::rgb(255, 255, 255),
        },
    }
}

pub fn demo_browsers() -> Vec<BrowserEntry> {
    [
        ("Firefox", "firefox", "🦊", true),
        ("Chrome", "chrome", "🌐", false),
        ("Brave", "brave", "🦁", false),
        ("Edge", "msedge", "🌊", false),
    ]
    .into_iter()
    .map(|(name, path, emoji, is_default)| BrowserEntry {
        name: name.to_string(),
        path: path.to_string(),
        flags: String::new(),
        icon: String::new(),
        emoji: emoji.to_string(),
        is_default,
    })
    .collect()
}

pub fn build_tree(browsers: &[BrowserEntry], url: &str, theme: Theme) -> Widget {
    let palette = palette(theme);
    let list = Widget::column(
        browsers
            .iter()
            .map(|browser| item(browser, palette))
            .collect(),
    )
    .with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        stretch: 1.0,
        spacing: 8,
        ..LayoutStyle::default()
    });

    Widget::column(vec![header(url, palette), list, footer(palette)])
        .with_layout(LayoutStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Percent(100.0),
            padding: Padding::all(16),
            spacing: 12,
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: palette.background,
            radius: 22,
            ..WidgetStyle::default()
        }))
}

fn header(url: &str, palette: Palette) -> Widget {
    Widget::column(vec![
        label("bropicker", 12.0, 600, palette.muted),
        label(url, 16.0, 600, palette.text),
    ])
    .with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(56),
        padding: Padding {
            left: 14,
            right: 14,
            top: 8,
            bottom: 8,
        },
        spacing: 2,
        ..LayoutStyle::default()
    })
    .with_styles(StyleSet::new(WidgetStyle {
        background: palette.surface,
        radius: 14,
        ..WidgetStyle::default()
    }))
}

fn item(browser: &BrowserEntry, palette: Palette) -> Widget {
    let badge = label("default", 11.0, 600, palette.accent_text)
        .with_layout(LayoutStyle {
            padding: Padding {
                left: 8,
                right: 8,
                top: 3,
                bottom: 3,
            },
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: palette.accent,
            radius: 8,
            ..WidgetStyle::default()
        }))
        .visible(browser.is_default);

    let details = Widget::column(vec![
        label(&browser.name, 15.0, 600, palette.text),
        label(&browser.path, 12.0, 400, palette.muted),
    ])
    .with_layout(LayoutStyle {
        stretch: 1.0,
        spacing: 2,
        ..LayoutStyle::default()
    });

    Widget::row(vec![icon(browser, palette), details, badge])
        .with_layout(LayoutStyle {
            width: Dimension::Percent(100.0),
            height: Dimension::Px(72),
            padding: Padding {
                left: 12,
                right: 12,
                top: 10,
                bottom: 10,
            },
            spacing: 12,
            align: Align::Center,
            ..LayoutStyle::default()
        })
        .with_styles(StyleSet::new(WidgetStyle {
            background: palette.item,
            border: palette.border,
            radius: 14,
            ..WidgetStyle::default()
        }))
}

fn icon(browser: &BrowserEntry, palette: Palette) -> Widget {
    let glyph = if browser.emoji.is_empty() {
        browser
            .name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string()
    } else {
        browser.emoji.clone()
    };
    let mut style = TextStyle::new(22.0);
    style.align = TextAlign::Center;

    Widget::view(
        Direction::Row,
        vec![
            Widget::styled_text(glyph, style).with_styles(StyleSet::new(WidgetStyle {
                text: palette.accent_text,
                ..WidgetStyle::default()
            })),
        ],
    )
    .with_layout(LayoutStyle {
        width: Dimension::Px(44),
        height: Dimension::Px(44),
        justify: Justify::Center,
        align: Align::Center,
        ..LayoutStyle::default()
    })
    .with_styles(StyleSet::new(WidgetStyle {
        background: palette.accent,
        radius: 12,
        ..WidgetStyle::default()
    }))
}

fn footer(palette: Palette) -> Widget {
    Widget::row(vec![
        checkbox("Remember choice", palette),
        checkbox("Always ask", palette),
    ])
    .with_layout(LayoutStyle {
        width: Dimension::Percent(100.0),
        height: Dimension::Px(44),
        padding: Padding {
            left: 12,
            right: 12,
            top: 8,
            bottom: 8,
        },
        spacing: 24,
        align: Align::Center,
        ..LayoutStyle::default()
    })
    .with_styles(StyleSet::new(WidgetStyle {
        background: palette.surface,
        radius: 12,
        ..WidgetStyle::default()
    }))
}

fn checkbox(text: &str, palette: Palette) -> Widget {
    Widget::row(vec![
        Widget::row(Vec::new())
            .with_layout(LayoutStyle {
                width: Dimension::Px(16),
                height: Dimension::Px(16),
                border_width: 1,
                ..LayoutStyle::default()
            })
            .with_styles(StyleSet::new(WidgetStyle {
                background: palette.item,
                border: palette.border,
                radius: 4,
                ..WidgetStyle::default()
            })),
        label(text, 12.0, 500, palette.muted),
    ])
    .with_layout(LayoutStyle {
        spacing: 8,
        align: Align::Center,
        ..LayoutStyle::default()
    })
}

fn label(text: &str, size: f32, weight: u16, color: Color) -> Widget {
    let mut style = TextStyle::new(size);
    style.weight = weight;

    Widget::styled_text(text, style).with_styles(StyleSet::new(WidgetStyle {
        text: color,
        ..WidgetStyle::default()
    }))
}

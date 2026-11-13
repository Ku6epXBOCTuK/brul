//! Значок в трее: меню и левый клик возвращают окно.

use std::cell::RefCell;

use brul::Handle;
use brul::logging::{info, warn};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const SIZE: u32 = 32;

const ACCENT: [u8; 3] = [76, 141, 255];

struct Tray {
    _icon: TrayIcon,
    _items: Vec<MenuItem>,
}

thread_local! {
    static TRAY: RefCell<Option<Tray>> = const { RefCell::new(None) };
}

pub fn install(handle: Handle) -> Result<(), String> {
    let show = MenuItem::new("Показать", true, None);
    let clear = MenuItem::new("Очистить", true, None);
    let separator = PredefinedMenuItem::separator();
    let quit = MenuItem::new("Выход", true, None);
    let menu = Menu::new();
    menu.append_items(&[&show, &clear, &separator, &quit])
        .map_err(|error| error.to_string())?;

    let show_id = show.id().clone();
    let clear_id = clear.id().clone();
    let quit_id = quit.id().clone();
    let menu_handle = handle.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let id = event.id();
        if id == &show_id {
            menu_handle.call("trayShow");
        } else if id == &clear_id {
            menu_handle.call("trayClear");
        } else if id == &quit_id {
            menu_handle.call("trayQuit");
        }
    }));

    let click_handle = handle;
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            click_handle.call("trayShow");
        }
    }));

    let icon = Icon::from_rgba(rgba(ACCENT), SIZE, SIZE).map_err(|error| error.to_string())?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("nobs")
        .with_icon(icon)
        .build()
        .map_err(|error| error.to_string())?;

    info!("tray icon created");
    TRAY.with(|slot| {
        *slot.borrow_mut() = Some(Tray {
            _icon: tray,
            _items: vec![show, clear, quit],
        });
    });
    Ok(())
}

fn rgba(color: [u8; 3]) -> Vec<u8> {
    let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = f64::from(x) - 15.5;
            let dy = f64::from(y) - 15.5;
            if dx * dx + dy * dy > 13.5 * 13.5 {
                continue;
            }
            let bar = (10..=21).contains(&x) && ((13..=15).contains(&y) || (19..=21).contains(&y));
            let (red, green, blue) = if bar {
                (255, 255, 255)
            } else {
                (color[0], color[1], color[2])
            };
            let index = ((y * SIZE + x) * 4) as usize;
            pixels[index] = red;
            pixels[index + 1] = green;
            pixels[index + 2] = blue;
            pixels[index + 3] = 255;
        }
    }
    pixels
}

pub fn warn_missing(result: Result<(), String>) {
    if let Err(message) = result {
        warn!(%message, "tray icon is unavailable");
    }
}

#[cfg(test)]
mod tests {
    use super::{ACCENT, SIZE, rgba};

    #[test]
    fn the_icon_is_a_square_of_rgba_pixels() {
        let pixels = rgba(ACCENT);
        assert_eq!(pixels.len(), (SIZE * SIZE * 4) as usize);
    }
}

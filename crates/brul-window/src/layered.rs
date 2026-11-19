use std::cell::RefCell;
use std::ffi::c_void;
use std::mem;
use std::ptr;
use std::sync::Arc;

use tracing::warn;
use windows::Win32::Foundation::{COLORREF, HWND, POINT, SIZE};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_BITFIELDS, BITMAPINFO, BITMAPV5HEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP,
    HDC, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, ULW_ALPHA, UpdateLayeredWindow, WS_EX_LAYERED,
};
use windows::core::Error as WinError;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use crate::window::{CLEAR, WindowError};

thread_local! {
    static LAYERED: RefCell<Vec<isize>> = const { RefCell::new(Vec::new()) };
}

fn apply_layered_style(hwnd: HWND) {
    // NOTE: winit rewrites GWL_EXSTYLE whenever window flags change, so the layered
    // bit must be restored after each such change.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if style & WS_EX_LAYERED.0 as isize != 0 {
            return;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

pub(crate) fn restore_layered_style(window: &Arc<dyn Window>) {
    let Ok(raw) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = raw.as_raw() else {
        return;
    };
    let value = handle.hwnd.get();
    if LAYERED.with(|registered| registered.borrow().contains(&value)) {
        apply_layered_style(HWND(value as *mut c_void));
    }
}

pub(crate) struct LayeredSurface {
    hwnd: HWND,
    screen_dc: HDC,
    mem_dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
    size: (u32, u32),
}

impl LayeredSurface {
    pub(crate) fn new(window: &Arc<dyn Window>) -> Result<Self, WindowError> {
        let raw = window.window_handle().map_err(WindowError::window)?;
        let RawWindowHandle::Win32(handle) = raw.as_raw() else {
            return Err(WindowError::app(std::io::Error::other(
                "not a Win32 window",
            )));
        };
        // NOTE: the Win32 handle stays valid for the lifetime of the winit window,
        // which outlives this presenter.
        let hwnd = HWND(handle.hwnd.get() as *mut c_void);

        // NOTE: hwnd is the live window passed in by winit.
        unsafe {
            let screen_dc = GetDC(Some(hwnd));
            if screen_dc.is_invalid() {
                return Err(WindowError::window(WinError::from_thread()));
            }
            let mem_dc = CreateCompatibleDC(Some(screen_dc));
            if mem_dc.is_invalid() {
                ReleaseDC(Some(hwnd), screen_dc);
                return Err(WindowError::window(WinError::from_thread()));
            }
            apply_layered_style(hwnd);
            LAYERED.with(|registered| registered.borrow_mut().push(handle.hwnd.get()));

            Ok(Self {
                hwnd,
                screen_dc,
                mem_dc,
                bitmap: HBITMAP::default(),
                previous: HGDIOBJ::default(),
                pixels: ptr::null_mut(),
                size: (0, 0),
            })
        }
    }

    fn set_size(&mut self, width: u32, height: u32) -> bool {
        if width == 0 || height == 0 || self.size == (width, height) {
            return self.size == (width, height) && width > 0;
        }
        // NOTE: mem_dc is a live compatible DC owned by this presenter.
        unsafe {
            if !self.previous.is_invalid() {
                SelectObject(self.mem_dc, self.previous);
            }
            if !self.bitmap.is_invalid() {
                let _ = DeleteObject(self.bitmap.into());
            }
            let header = BITMAPV5HEADER {
                bV5Size: mem::size_of::<BITMAPV5HEADER>() as u32,
                bV5Width: width as i32,
                bV5Height: -(height as i32),
                bV5Planes: 1,
                bV5BitCount: 32,
                bV5Compression: BI_BITFIELDS,
                bV5RedMask: 0x00_ff_00_00,
                bV5GreenMask: 0x00_00_ff_00,
                bV5BlueMask: 0x00_00_00_ff,
                bV5AlphaMask: 0xff_00_00_00,
                ..Default::default()
            };
            let mut bits = ptr::null_mut();
            let bitmap = CreateDIBSection(
                Some(self.mem_dc),
                &header as *const BITMAPV5HEADER as *const BITMAPINFO,
                DIB_RGB_COLORS,
                &mut bits,
                None,
                0,
            );
            let Ok(bitmap) = bitmap else {
                warn!("CreateDIBSection failed");
                self.bitmap = HBITMAP::default();
                self.previous = HGDIOBJ::default();
                self.pixels = ptr::null_mut();
                self.size = (0, 0);
                return false;
            };
            self.previous = SelectObject(self.mem_dc, bitmap.into());
            self.bitmap = bitmap;
            self.pixels = bits as *mut u8;
            self.size = (width, height);
            true
        }
    }

    fn update(&self) -> bool {
        apply_layered_style(self.hwnd);
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        // NOTE: mem_dc selects a DIB whose alpha mask is set; the window is layered.
        let point = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: self.size.0 as i32,
            cy: self.size.1 as i32,
        };
        unsafe {
            match UpdateLayeredWindow(
                self.hwnd,
                None,
                None,
                Some(&size),
                Some(self.mem_dc),
                Some(&point),
                COLORREF(0),
                Some(&blend as *const BLENDFUNCTION),
                ULW_ALPHA,
            ) {
                Ok(()) => true,
                Err(error) => {
                    warn!(?error, "UpdateLayeredWindow failed");
                    false
                }
            }
        }
    }
}

impl LayeredSurface {
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.set_size(width, height);
    }

    pub(crate) fn clear(&mut self) -> bool {
        let (width, height) = self.size;
        if width == 0 || height == 0 {
            return false;
        }
        // NOTE: pixels points at width * height * 4 bytes of the DIB section.
        unsafe {
            ptr::write_bytes(self.pixels, 0, width as usize * height as usize * 4);
        }
        self.update()
    }

    pub(crate) fn clear_opaque(&mut self) -> bool {
        let (width, height) = self.size;
        if width == 0 || height == 0 {
            return false;
        }
        let channel = |value: f64| (value * 255.0).round() as u8;
        let background = pack_argb(&[channel(CLEAR.r), channel(CLEAR.g), channel(CLEAR.b), 255]);
        unsafe {
            let target = self.pixels as *mut u32;
            for index in 0..(width as usize * height as usize) {
                *target.add(index) = background;
            }
        }
        self.update()
    }

    pub(crate) fn present(&mut self, pixels: &[u8]) -> bool {
        let (width, height) = self.size;
        if width == 0 || height == 0 || pixels.len() != width as usize * height as usize * 4 {
            return false;
        }
        // NOTE: the DIB owns width * height * 4 bytes and Frame RGB is premultiplied,
        // which is what AC_SRC_ALPHA expects.
        unsafe {
            let target = self.pixels as *mut u32;
            for (index, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
                *target.add(index) = pack_argb(pixel);
            }
        }
        self.update()
    }
}

fn pack_argb(pixel: &[u8; 4]) -> u32 {
    let red = u32::from(pixel[0]);
    let green = u32::from(pixel[1]);
    let blue = u32::from(pixel[2]);
    let alpha = u32::from(pixel[3]);
    (alpha << 24) | (red << 16) | (green << 8) | blue
}

impl Drop for LayeredSurface {
    fn drop(&mut self) {
        let value = self.hwnd.0 as isize;
        LAYERED.with(|registered| registered.borrow_mut().retain(|entry| *entry != value));
        // NOTE: every handle was created by this presenter and is still live.
        unsafe {
            if !self.previous.is_invalid() {
                SelectObject(self.mem_dc, self.previous);
            }
            if !self.bitmap.is_invalid() {
                let _ = DeleteObject(self.bitmap.into());
            }
            if !self.mem_dc.is_invalid() {
                let _ = DeleteDC(self.mem_dc);
            }
            if !self.screen_dc.is_invalid() {
                ReleaseDC(Some(self.hwnd), self.screen_dc);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opaque_pixel_keeps_its_alpha() {
        assert_eq!(pack_argb(&[0x12, 0x34, 0x56, 255]), 0xff_12_34_56);
    }

    #[test]
    fn a_transparent_pixel_is_not_composited_against_the_clear_color() {
        assert_eq!(pack_argb(&[0, 0, 0, 0]), 0);
    }

    #[test]
    fn a_premultiplied_pixel_is_not_multiplied_again() {
        assert_eq!(pack_argb(&[0x08, 0x10, 0x18, 64]), 0x40_08_10_18);
    }
}

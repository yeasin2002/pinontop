use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::win::are_animations_enabled;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
#[cfg(windows)]
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION,
    DIB_RGB_COLORS, HBRUSH,
};
#[cfg(windows)]
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowRect, RegisterClassExW,
    SetWindowPos, ShowWindow, UnregisterClassW, UpdateLayeredWindow, CW_USEDEFAULT,
    HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
    ULW_ALPHA, WNDCLASSEXW, WNDCLASS_STYLES,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_POPUP,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlashKind {
    Pin,   // Orange #C49464
    Unpin, // Grey #8A857C
    Hover, // Orange #C49464 (persistent)
}

impl FlashKind {
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            FlashKind::Pin | FlashKind::Hover => (0xC4, 0x94, 0x64),
            FlashKind::Unpin => (0x8A, 0x85, 0x7C),
        }
    }
}

static OVERLAY_HWND: AtomicU32 = AtomicU32::new(0);
static CURRENT_ANIMATION_ID: AtomicU32 = AtomicU32::new(0);

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
unsafe extern "system" fn overlay_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Ensures the singleton overlay window is created.
pub fn init_overlay_window() -> Option<isize> {
    #[cfg(windows)]
    unsafe {
        let existing = OVERLAY_HWND.load(Ordering::SeqCst);
        if existing != 0 {
            return Some(existing as isize);
        }

        let class_name_w = to_wide("PinItFlashOverlay");
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(overlay_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: windows::Win32::UI::WindowsAndMessaging::HICON::default(),
            hCursor: windows::Win32::UI::WindowsAndMessaging::HCURSOR::default(),
            hbrBackground: HBRUSH::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name_w.as_ptr()),
            hIconSm: windows::Win32::UI::WindowsAndMessaging::HICON::default(),
        };

        let _ = RegisterClassExW(&wnd_class);

        let ex_style = WS_EX_LAYERED
            | WS_EX_TRANSPARENT
            | WS_EX_TOOLWINDOW
            | WS_EX_TOPMOST
            | WS_EX_NOACTIVATE;

        let hwnd_res = CreateWindowExW(
            ex_style,
            PCWSTR(class_name_w.as_ptr()),
            PCWSTR(class_name_w.as_ptr()),
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            None,
            None,
            Some(windows::Win32::Foundation::HINSTANCE(hinstance.0)),
            None,
        );

        if let Ok(hwnd) = hwnd_res
            && !hwnd.0.is_null() {
                OVERLAY_HWND.store(hwnd.0 as usize as u32, Ordering::SeqCst);
                return Some(hwnd.0 as isize);
            }
    }

    None
}

/// Retrieves the visible window frame rect using `DwmGetWindowAttribute`,
/// avoiding invisible resize drop-shadow margins.
pub fn get_target_frame_rect(target_hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    #[cfg(windows)]
    unsafe {
        let h = HWND(target_hwnd as *mut std::ffi::c_void);
        let mut rect = RECT::default();

        let dwm_res = DwmGetWindowAttribute(
            h,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );

        if dwm_res.is_err() {
            let _ = GetWindowRect(h, &mut rect);
        }

        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        if width <= 0 || height <= 0 {
            None
        } else {
            Some((rect.left, rect.top, width, height))
        }
    }

    #[cfg(not(windows))]
    {
        let _ = target_hwnd;
        None
    }
}

/// Renders a premultiplied BGRA 3-4 pixel border bitmap into an in-memory DIB DC
/// and applies it to the layered overlay window via `UpdateLayeredWindow`.
#[cfg(windows)]
unsafe fn render_outline(
    overlay_hwnd: HWND,
    width: i32,
    height: i32,
    color: (u8, u8, u8),
    constant_alpha: u8,
) -> bool {
    unsafe {
        let stroke_width = 3;
        let (r, g, b) = color;

        let hdc_screen = windows::Win32::Graphics::Gdi::GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: (width * height * 4) as u32,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [windows::Win32::Graphics::Gdi::RGBQUAD::default()],
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(
            Some(hdc_mem),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        );

        if hbitmap.is_err() || bits.is_null() {
            let _ = DeleteDC(hdc_mem);
            let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);
            return false;
        }

        let hbmp = hbitmap.unwrap();
        let old_bmp = SelectObject(hdc_mem, hbmp.into());

        let pixel_slice = std::slice::from_raw_parts_mut(bits as *mut u8, (width * height * 4) as usize);

        // Draw high-contrast solid stroke border with premultiplied BGRA
        for y in 0..height {
            let is_y_border = y < stroke_width || y >= height - stroke_width;
            let row_offset = (y * width * 4) as usize;

            for x in 0..width {
                let is_x_border = x < stroke_width || x >= width - stroke_width;
                let offset = row_offset + (x * 4) as usize;

                if is_x_border || is_y_border {
                    // Border pixel: solid color with alpha = 255
                    pixel_slice[offset] = b;     // Blue
                    pixel_slice[offset + 1] = g; // Green
                    pixel_slice[offset + 2] = r; // Red
                    pixel_slice[offset + 3] = 255; // Alpha
                } else {
                    // Interior pixel: fully transparent
                    pixel_slice[offset] = 0;
                    pixel_slice[offset + 1] = 0;
                    pixel_slice[offset + 2] = 0;
                    pixel_slice[offset + 3] = 0;
                }
            }
        }

        let pt_src = POINT { x: 0, y: 0 };
        let size = SIZE { cx: width, cy: height };

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: constant_alpha,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };

        let update_res = UpdateLayeredWindow(
            overlay_hwnd,
            None,
            None,
            Some(&size),
            Some(hdc_mem),
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        let _ = SelectObject(hdc_mem, old_bmp);
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        let _ = windows::Win32::Graphics::Gdi::ReleaseDC(None, hdc_screen);

        update_res.is_ok()
    }
}

/// Triggers a flash overlay on the specified target window.
///
/// Behavior:
/// - `FlashKind::Pin`: Orange outline, 300ms hold + 250ms quadratic ease-out fade.
/// - `FlashKind::Unpin`: Grey outline, 300ms hold + 250ms quadratic ease-out fade.
/// - `FlashKind::Hover`: Persistent orange outline until `hide_flash()` is called.
pub fn flash_window(target_hwnd: isize, kind: FlashKind) {
    let (x, y, width, height) = match get_target_frame_rect(target_hwnd) {
        Some(rect) => rect,
        None => return,
    };

    let overlay_val = match init_overlay_window() {
        Some(val) => val,
        None => return,
    };

    #[cfg(windows)]
    unsafe {
        let overlay_hwnd = HWND(overlay_val as *mut std::ffi::c_void);
        let color = kind.rgb();

        // 1. Render border at full alpha
        if !render_outline(overlay_hwnd, width, height, color, 255) {
            return;
        }

        // 2. Position over target window and show without stealing focus
        let _ = SetWindowPos(
            overlay_hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );

        if kind == FlashKind::Hover {
            // Hover is persistent, no fade timer
            return;
        }

        // 3. For timed Pin/Unpin: 300ms hold + 250ms quadratic ease-out fade
        let anim_id = CURRENT_ANIMATION_ID.fetch_add(1, Ordering::SeqCst) + 1;
        let animations_enabled = are_animations_enabled();

        thread::spawn(move || {
            let hold_duration = Duration::from_millis(300);
            let fade_duration = Duration::from_millis(250);

            thread::sleep(hold_duration);

            if CURRENT_ANIMATION_ID.load(Ordering::SeqCst) != anim_id {
                return; // Newer flash preempted this animation
            }

            let overlay_hwnd = HWND(overlay_val as *mut std::ffi::c_void);

            if !animations_enabled {
                // Accessibility mode: skip animation and hide instantly
                let _ = ShowWindow(overlay_hwnd, SW_HIDE);
                return;
            }

            let start_fade = Instant::now();
            loop {
                let elapsed = start_fade.elapsed();
                if elapsed >= fade_duration {
                    break;
                }

                if CURRENT_ANIMATION_ID.load(Ordering::SeqCst) != anim_id {
                    return; // Preempted
                }

                let t = elapsed.as_secs_f32() / fade_duration.as_secs_f32();
                // Quadratic ease-out: opacity = 1.0 - t^2
                let opacity = (1.0 - t * t).clamp(0.0, 1.0);
                let alpha = (opacity * 255.0) as u8;

                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: alpha,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };

                let _ = UpdateLayeredWindow(
                    overlay_hwnd,
                    None,
                    None,
                    None,
                    None,
                    None,
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                );

                thread::sleep(Duration::from_millis(16)); // ~60fps
            }

            if CURRENT_ANIMATION_ID.load(Ordering::SeqCst) == anim_id {
                let _ = ShowWindow(overlay_hwnd, SW_HIDE);
            }
        });
    }

    #[cfg(not(windows))]
    {
        let _ = (target_hwnd, kind, x, y, width, height, overlay_val);
    }
}

/// Immediately hides the flash overlay (e.g. on hover exit).
pub fn hide_flash() {
    #[cfg(windows)]
    unsafe {
        let val = OVERLAY_HWND.load(Ordering::SeqCst);
        if val != 0 {
            CURRENT_ANIMATION_ID.fetch_add(1, Ordering::SeqCst);
            let overlay_hwnd = HWND(val as *mut std::ffi::c_void);
            let _ = ShowWindow(overlay_hwnd, SW_HIDE);
        }
    }
}

/// Cleans up the overlay window on application exit.
pub fn cleanup_overlay_window() {
    #[cfg(windows)]
    unsafe {
        let val = OVERLAY_HWND.swap(0, Ordering::SeqCst);
        if val != 0 {
            let hwnd = HWND(val as *mut std::ffi::c_void);
            let _ = DestroyWindow(hwnd);
            let class_name_w = to_wide("PinItFlashOverlay");
            let hinstance = GetModuleHandleW(None).unwrap_or_default();
            let _ = UnregisterClassW(PCWSTR(class_name_w.as_ptr()), Some(hinstance.into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_kind_colors() {
        assert_eq!(FlashKind::Pin.rgb(), (0xC4, 0x94, 0x64));
        assert_eq!(FlashKind::Unpin.rgb(), (0x8A, 0x85, 0x7C));
        assert_eq!(FlashKind::Hover.rgb(), (0xC4, 0x94, 0x64));
    }
}

//! 窗口几何与原生窗口层级。
//!
//! 旧 OpenHop（Tauri）在 macOS 上把岛窗口设成 `NSStatusWindowLevel(25)`，
//! 高于菜单栏 `NSMainMenuWindowLevel(24)`，并加上 canJoinAllSpaces /
//! stationary / fullScreenAuxiliary，才能盖在刘海与全屏应用之上。
//! egui/eframe 的 `with_always_on_top()` 只给到 NSFloatingWindowLevel(3)，
//! 所以这里通过 Frame 暴露的 raw window handle 拿 NSWindow 自己设置。

use crate::state::{Dock, Settings, Shape};
use openhop_core::platform;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// 主屏刘海尺寸（点）。调用必须在主线程。
///
/// 启动早期（NSApplication 尚未就绪）AppKit 可能返回 None，所以「没查到」不缓存，
/// 之后每 2 秒重试一次；查到之后永久缓存，避免每帧问 AppKit。
#[cfg(target_os = "macos")]
pub fn notch_size() -> Option<(f64, f64)> {
    struct Cache {
        value: Option<(f64, f64)>,
        last_try: Option<Instant>,
    }
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let mut c = CACHE
        .get_or_init(|| Mutex::new(Cache { value: None, last_try: None }))
        .lock()
        .unwrap();
    if let Some(v) = c.value {
        return Some(v);
    }
    if c.last_try.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
        return None;
    }
    c.last_try = Some(Instant::now());
    c.value = openhop_core::extras::system::notch();
    if let Some((w, h)) = c.value {
        log::info!("notch detected: {w:.0} x {h:.0} pt");
    }
    c.value
}

#[cfg(not(target_os = "macos"))]
pub fn notch_size() -> Option<(f64, f64)> {
    None
}

fn screen() -> (i32, i32, i32, i32) {
    match platform::monitors().first() {
        Some(m) => (m.x, m.y, m.w, m.h),
        None => (0, 0, 1512, 982),
    }
}

fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    v.max(lo).min(hi)
}

/// 收起 / 展开两种状态下岛的位置与大小（逻辑点）。
pub fn geometry(s: &Settings, expanded: bool) -> Geometry {
    let (mx, my, mw, mh) = screen();
    let k = s.scale.max(0.8).min(1.4);

    // 收起态：刘海机型 + 选了「刘海」形态时，胶囊贴住刘海并把它包进去。
    let notch = if s.dock == Dock::Top && s.shape == Shape::Notch {
        notch_size()
    } else {
        None
    };
    let (cw, ch, cx, cy) = if let Some((nw, nh)) = notch {
        let w = (nw as f32 + 18.0) * k;
        let h = (nh as f32).max(30.0) * k;
        let x = mx as f32 + (mw as f32 - w) / 2.0;
        (w, h, x, my as f32)
    } else {
        let w = 236.0 * k;
        let h = 40.0 * k;
        let (x, y) = match s.dock {
            Dock::Top => (mx as f32 + (mw as f32 - w) / 2.0, my as f32 + 6.0),
            Dock::Left => (mx as f32 + 8.0, my as f32 + (mh as f32 - h) / 2.0),
            Dock::Right => (
                mx as f32 + mw as f32 - w - 8.0,
                my as f32 + (mh as f32 - h) / 2.0,
            ),
        };
        (w, h, x, y)
    };

    if !expanded {
        return Geometry { x: cx, y: cy, w: cw, h: ch };
    }

    let w = 520.0 * k;
    let h = 420.0 * k;
    let x = match s.dock {
        Dock::Top => clamp(cx + (cw - w) / 2.0, mx as f32 + 8.0, mx as f32 + mw as f32 - w - 8.0),
        Dock::Left => mx as f32 + 8.0,
        Dock::Right => mx as f32 + mw as f32 - w - 8.0,
    };
    let y = match s.dock {
        Dock::Top => cy + ch + 6.0,
        _ => clamp(cy + (ch - h) / 2.0, my as f32 + 8.0, my as f32 + mh as f32 - h - 8.0),
    };
    Geometry { x, y, w, h }
}

/// 把 NSWindow 抬到菜单栏之上。返回是否成功（首帧 NSView 可能还没挂到窗口上）。
#[cfg(target_os = "macos")]
pub fn elevate(frame: &eframe::Frame) -> bool {
    use objc2::runtime::{AnyObject, Bool};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = frame.window_handle() else { return false };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return false };
    // raw-window-handle 0.6 只暴露 NSView，NSWindow 要经 [view window] 取。
    let view = h.ns_view.as_ptr() as *mut AnyObject;
    if view.is_null() { return false; }
    let ns: *mut AnyObject = unsafe { objc2::msg_send![view, window] };
    if ns.is_null() { return false; }
    unsafe {
        // NSStatusWindowLevel = 25 > NSMainMenuWindowLevel = 24
        let _: () = objc2::msg_send![ns, setLevel: 25isize];
        // canJoinAllSpaces(1) | stationary(16) | fullScreenAuxiliary(256)
        let _: () = objc2::msg_send![ns, setCollectionBehavior: (1usize | 16 | 256)];
        let _: () = objc2::msg_send![ns, setHasShadow: Bool::NO];
        let _: () = objc2::msg_send![ns, setOpaque: Bool::NO];
    }
    log::info!("island window raised to NSStatusWindowLevel(25) with all-spaces collection behavior");
    true
}

#[cfg(not(target_os = "macos"))]
pub fn elevate(_frame: &eframe::Frame) -> bool { true }

/// 打开系统权限设置页（用于辅助功能 / 输入监控引导）。
pub fn open_privacy_settings(page: &str) {
    #[cfg(target_os = "macos")]
    {
        let url = match page {
            "input" => "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
            _ => "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
        };
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", "ms-settings:privacy-accessibility"])
            .spawn();
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    let _ = page;
}

//! The dock hiding on its own: when it tucks away and what brings it back.

use super::*;

impl Dock {
    /// Starts or stops looking at the pointer, as the setting says; with it
    /// off, the dock comes back in place.
    pub(super) fn arm_auto_hide(&mut self) {
        if self.data.settings.dock.auto_hide {
            unsafe { SetTimer(Some(self.hwnd), AUTO_HIDE_TIMER, AUTO_HIDE_EVERY, None) };
        } else {
            let _ = unsafe { KillTimer(Some(self.hwnd), AUTO_HIDE_TIMER) };
            self.untuck();
        }
    }

    /// Brings the dock up; hiding on its own, it goes down again once the
    /// pointer is away.
    pub(super) fn untuck(&mut self) {
        self.on_edge_since = None;
        self.away_since = None;
        if self.tucking || self.tucked > 0.0 {
            self.tucking = false;
            if self.visible {
                self.show_window();
            }
            self.animate();
        }
    }

    /// One look at the pointer while the dock hides on its own: down once
    /// the pointer is away for a moment, unless something of the dock is in
    /// use; up once it rests on the screen edge.
    pub(super) fn watch_pointer(&mut self) {
        if !self.data.settings.dock.auto_hide || !self.visible {
            return;
        }
        let mut at = POINT::default();
        if unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at) }.is_err() {
            return;
        }
        let point = (at.x, at.y);
        let now = Instant::now();
        let edge = self.edge();
        let dock = &self.data.settings.dock;
        let reveal_after = Duration::from_millis(u64::from(dock.auto_hide_reveal));
        let tuck_after = Duration::from_millis(u64::from(dock.auto_hide_delay));
        if self.tucking {
            if at_screen_edge(point, self.monitor.rect, edge) {
                let since = *self.on_edge_since.get_or_insert(now);
                if now.duration_since(since) >= reveal_after {
                    self.untuck();
                }
            } else {
                self.on_edge_since = None;
            }
            return;
        }
        let busy = self.open.is_some()
            || self.keyboard.is_some()
            || self.press.is_some()
            || self.landing.is_some()
            || self.menu_open
            || self.previewing()
            || menu_on_screen();
        if busy || near_dock(point, self.window, self.monitor.rect, edge) {
            self.away_since = None;
            return;
        }
        let since = *self.away_since.get_or_insert(now);
        if now.duration_since(since) >= tuck_after {
            self.away_since = None;
            self.end_preview();
            self.tucking = true;
            self.hovered = None;
            self.status_hovered = false;
            self.strip_hovered = None;
            self.animate();
        }
    }

    pub(super) fn hide(&mut self) {
        self.visible = false;
        self.keyboard = None;
        self.before = None;
        access::set_focus(self.hwnd.0 as isize, None);
        self.hovered = None;
        self.press = None;
        self.pressed = None;
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        self.publish();
    }
}

/// Whether the pointer at `point` rests on the screen edge the dock is on,
/// on its monitor (physical pixels): what brings a dock hiding on its own
/// up.
pub(super) fn at_screen_edge(point: (i32, i32), monitor: Rect, edge: DockEdge) -> bool {
    let (x, y) = point;
    let right = monitor.x.saturating_add_unsigned(monitor.width);
    let bottom = monitor.y.saturating_add_unsigned(monitor.height);
    if x < monitor.x || x >= right {
        return false;
    }
    match edge {
        DockEdge::Bottom => y >= bottom - 2 && y < bottom,
        DockEdge::Top => y >= monitor.y && y < monitor.y + 2,
    }
}

/// Whether the pointer is on the dock or between it and the screen edge:
/// where the dock stays up.
pub(super) fn near_dock(point: (i32, i32), window: Rect, monitor: Rect, edge: DockEdge) -> bool {
    let (x, y) = point;
    let right = window.x.saturating_add_unsigned(window.width);
    if x < window.x || x >= right {
        return false;
    }
    match edge {
        DockEdge::Bottom => y >= window.y && y < monitor.y.saturating_add_unsigned(monitor.height),
        DockEdge::Top => y >= monitor.y && y < window.y.saturating_add_unsigned(window.height),
    }
}

/// Whether a menu is on the screen (the dock's, or one a tray icon in the
/// dock opened): the dock stays up under it.
pub(super) fn menu_on_screen() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible};
    // Windows' menus (a tray icon's) and the Dockyard's own (spec 0014 R3).
    [w!("#32768"), menu::CLASS].into_iter().any(|class| {
        unsafe { FindWindowW(class, None) }
            .is_ok_and(|menu| unsafe { IsWindowVisible(menu) }.as_bool())
    })
}

/// One frame of the dock going out or coming in (`now` and `target` from 0,
/// in place, to 1, all out): a part of the way, and there once close.
pub(super) fn tuck_step(now: f64, target: f64) -> f64 {
    let next = now + (target - now) * 0.25;
    if (target - next).abs() < 0.01 {
        target
    } else {
        next
    }
}

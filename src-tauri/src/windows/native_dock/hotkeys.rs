//! The hotkeys of the dock: switching workspaces and taking the focus.

use super::*;

/// How many `Ctrl+Alt+<number>` shortcuts are registered (spec 0013): one
/// per workspace, and none with a single workspace, so that `AltGr+<number>`
/// (which Windows reports as `Ctrl+Alt`) still types where there is nothing to
/// switch to.
pub(super) static HOTKEYS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Registers `Ctrl+Alt+1…N` for `count` workspaces on the wake window. A
/// shortcut another program already has is skipped, with a line in the log.
pub(super) fn arm_hotkeys(count: usize) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
    };
    let wanted = if count >= 2 { count.min(9) } else { 0 };
    let had = HOTKEYS.swap(wanted, Ordering::SeqCst);
    if had == wanted {
        return;
    }
    let wake = HWND(WAKE.load(Ordering::SeqCst) as _);
    for number in 1..=9i32 {
        let _ = unsafe { UnregisterHotKey(Some(wake), number) };
    }
    for number in 1..=wanted as i32 {
        let key = 0x30 + number as u32;
        let registered = unsafe {
            RegisterHotKey(
                Some(wake),
                number,
                MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                key,
            )
        };
        if registered.is_err() {
            log::warn!("dock: Ctrl+Alt+{number} is taken by another program");
        }
    }
}

/// The monitor the pointer is on, as a number (an `HMONITOR`).
pub(super) fn pointer_monitor() -> isize {
    let mut at = POINT::default();
    let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut at) };
    let monitor = unsafe {
        windows::Win32::Graphics::Gdi::MonitorFromPoint(
            at,
            windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
        )
    };
    monitor.0 as isize
}

/// The id of the hotkey that takes the keyboard to the dock (M14), on the
/// wake window: past the workspaces' 1–9.
pub(super) const FOCUS_HOTKEY: i32 = 10;
/// Takes that hotkey back if a capture never ended (Configurações closed in
/// the middle), on the wake window; past the docks' timers.
pub(super) const FOCUS_CAPTURE_TIMER: usize = 10;
pub(super) const FOCUS_CAPTURE_FOR: u32 = 60_000;

/// Configurações captures a new hotkey: the dock's is let go meanwhile.
pub(super) static FOCUS_CAPTURING: AtomicBool = AtomicBool::new(false);
/// The dock's hotkey last asked of Windows (`None` inside: none, while a
/// capture runs), so that one another program holds is tried, and logged,
/// only once per change of the settings.
pub(super) static FOCUS_ARMED: Mutex<Option<Option<String>>> = Mutex::new(None);

/// Asks Windows for the dock's hotkey of the settings on the wake window,
/// when it changed. One another program holds is skipped, with a line in
/// the log. On the main thread.
pub(super) fn arm_focus_hotkey() {
    use crate::domain::hotkey;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
    };
    let (Some(app), wake) = (APP.get(), WAKE.load(Ordering::SeqCst)) else {
        return;
    };
    if wake == 0 {
        return;
    }
    let wanted = (!FOCUS_CAPTURING.load(Ordering::SeqCst))
        .then(|| crate::commands::settings::current(app).dock_hotkey);
    {
        let mut armed = FOCUS_ARMED.lock().unwrap_or_else(PoisonError::into_inner);
        if armed.as_ref() == Some(&wanted) {
            return;
        }
        *armed = Some(wanted.clone());
    }
    let wake = HWND(wake as _);
    let _ = unsafe { UnregisterHotKey(Some(wake), FOCUS_HOTKEY) };
    let Some(text) = wanted else {
        return;
    };
    let Ok(hotkey) = hotkey::check(&text) else {
        log::warn!("dock: the hotkey of the settings is not one it can take");
        return;
    };
    let registered = unsafe {
        RegisterHotKey(
            Some(wake),
            FOCUS_HOTKEY,
            HOT_KEY_MODIFIERS(hotkey.modifiers()) | MOD_NOREPEAT,
            hotkey.key.vk(),
        )
    };
    if let Err(error) = registered {
        log::warn!("dock: {hotkey} is held by another program: {error}");
    }
}

/// Configurações captures a new hotkey (`on`): the dock's is let go, or the
/// combination would take the keyboard to the dock. It comes back when the
/// capture ends, or on its own after `FOCUS_CAPTURE_FOR`. From any thread.
pub fn capture_hotkey(on: bool) {
    queue(Box::new(move |_| {
        FOCUS_CAPTURING.store(on, Ordering::SeqCst);
        let wake = HWND(WAKE.load(Ordering::SeqCst) as _);
        unsafe {
            if on {
                SetTimer(Some(wake), FOCUS_CAPTURE_TIMER, FOCUS_CAPTURE_FOR, None);
            } else {
                let _ = KillTimer(Some(wake), FOCUS_CAPTURE_TIMER);
            }
        }
        arm_focus_hotkey();
    }));
}

/// The dock's hotkey: the keyboard goes to the dock on the monitor the
/// pointer is on (or the first one on screen), and Esc brings it back to
/// the window that has it now.
pub(super) fn take_focus() {
    let foreground = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    let monitor = pointer_monitor();
    with_docks(|docks| {
        // Pressed again from a dock: the window from before that one.
        let before = match docks.iter().find(|dock| dock.hwnd == foreground) {
            Some(dock) => dock.before,
            None => (!foreground.is_invalid()).then_some(foreground),
        };
        let target = docks
            .iter()
            .position(|dock| dock.visible && dock.monitor.handle == monitor)
            .or_else(|| docks.iter().position(|dock| dock.visible));
        let Some(target) = target else {
            return;
        };
        // Activating one dock reaches the others' procedure while they are
        // borrowed here, so they let go of the keyboard now.
        for (index, dock) in docks.iter_mut().enumerate() {
            if index != target {
                dock.lose_keyboard();
            }
        }
        docks[target].take_keyboard(before);
    });
}

/// `Ctrl+Alt+<number>`: the dock on the monitor the pointer is on goes to
/// that workspace.
pub(super) fn hotkey(number: usize) {
    let Some(index) = number.checked_sub(1) else {
        return;
    };
    let monitor = pointer_monitor();
    let key = {
        let shared = shared();
        shared
            .docks
            .iter()
            .find(|placed| placed.handle == monitor)
            .or(shared.docks.first())
            .map(|placed| placed.monitor.clone())
    };
    if let (Some(key), Some(app)) = (key, APP.get()) {
        crate::commands::workspaces::switch_to_index(app, &key, index);
    }
}

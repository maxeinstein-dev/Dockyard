//! The Layout do teclado (F9 of estudo 0002): the keyboard layout of the
//! window in front, as the taskbar's language button shows it, and the next
//! one on a click.
//!
//! Windows keeps a layout per thread: the one that matters is the window in
//! front's, which a click on the dock leaves in front (`WS_EX_NOACTIVATE`).
//! A switch is asked of that window (`WM_INPUTLANGCHANGEREQUEST`), as the
//! taskbar's button does; the window takes it a moment later.

use crate::domain::widgets::keyboard;

/// What the tile shows: the short name of the layout in front (`PTB`);
/// `None` when it can't be read.
#[cfg(windows)]
pub fn label() -> Option<String> {
    let (_, layout) = front()?;
    abbreviation(layout).map(|name| keyboard::short_name(&name))
}

/// The layout's full name, for the tip: `Português (Brasil)`.
#[cfg(windows)]
pub fn name() -> Option<String> {
    let (_, layout) = front()?;
    locale_info(
        layout,
        windows::Win32::Globalization::LOCALE_SLOCALIZEDDISPLAYNAME,
    )
}

/// Asks the window in front to go to the next installed layout. Nothing
/// with a single layout.
#[cfg(windows)]
pub fn switch_next() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayoutList;
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_INPUTLANGCHANGEREQUEST};

    let Some((window, current)) = front() else {
        return;
    };
    let count = unsafe { GetKeyboardLayoutList(None) };
    let mut layouts = vec![Default::default(); usize::try_from(count).unwrap_or(0)];
    let read = unsafe { GetKeyboardLayoutList(Some(&mut layouts)) };
    layouts.truncate(usize::try_from(read).unwrap_or(0));
    let ids: Vec<isize> = layouts.iter().map(|layout| layout.0 as isize).collect();
    let Some(next) = keyboard::next(&ids, current.0 as isize) else {
        return;
    };
    let posted = unsafe {
        PostMessageW(
            Some(window),
            WM_INPUTLANGCHANGEREQUEST,
            WPARAM(0),
            LPARAM(next),
        )
    };
    if let Err(error) = posted {
        log::warn!("keyboard: the switch was not asked: {error}");
    }
}

/// The window in front and its thread's layout.
#[cfg(windows)]
fn front() -> Option<(
    windows::Win32::Foundation::HWND,
    windows::Win32::UI::Input::KeyboardAndMouse::HKL,
)> {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    let window = unsafe { GetForegroundWindow() };
    if window.is_invalid() {
        return None;
    }
    let thread = unsafe { GetWindowThreadProcessId(window, None) };
    if thread == 0 {
        return None;
    }
    let layout = unsafe { GetKeyboardLayout(thread) };
    (!layout.is_invalid()).then_some((window, layout))
}

/// The language's abbreviation Windows gives (`PTB`, `ENU`).
#[cfg(windows)]
fn abbreviation(layout: windows::Win32::UI::Input::KeyboardAndMouse::HKL) -> Option<String> {
    locale_info(
        layout,
        windows::Win32::Globalization::LOCALE_SABBREVLANGNAME,
    )
}

/// A text of the locale of `layout`'s language (its low word).
#[cfg(windows)]
fn locale_info(
    layout: windows::Win32::UI::Input::KeyboardAndMouse::HKL,
    what: u32,
) -> Option<String> {
    use windows::Win32::Globalization::{GetLocaleInfoEx, LCIDToLocaleName};
    use windows::core::PCWSTR;

    let language = (layout.0 as usize & 0xFFFF) as u32;
    let mut locale = [0u16; 85];
    let length = unsafe { LCIDToLocaleName(language, Some(&mut locale), 0) };
    if length <= 0 {
        return None;
    }
    let mut text = [0u16; 128];
    let written = unsafe { GetLocaleInfoEx(PCWSTR(locale.as_ptr()), what, Some(&mut text)) };
    let written = usize::try_from(written).ok().filter(|n| *n > 1)?;
    Some(String::from_utf16_lossy(&text[..written - 1]))
}

#[cfg(not(windows))]
pub fn label() -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn name() -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn switch_next() {}

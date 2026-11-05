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
    let short = keyboard::short_name(&abbreviation(layout)?);
    // The other layouts of its language, by name: the tile tells them apart.
    let language = |hkl: isize| hkl & 0xFFFF;
    let own = layout.0 as isize;
    let others: Vec<String> = installed()
        .into_iter()
        .filter(|other| {
            let other = other.0 as isize;
            other != own && language(other) == language(own)
        })
        .filter_map(layout_name)
        .collect();
    if others.is_empty() {
        return Some(short);
    }
    let Some(name) = layout_name(layout) else {
        return Some(short);
    };
    Some(keyboard::distinguished(&short, &name, &others))
}

/// The layout's full name, for the tip: `Português (Brasil ABNT2)`, as
/// Windows lists it, or its language's (`Português (Brasil)`).
#[cfg(windows)]
pub fn name() -> Option<String> {
    let (_, layout) = front()?;
    layout_name(layout).or_else(|| {
        locale_info(
            layout,
            windows::Win32::Globalization::LOCALE_SLOCALIZEDDISPLAYNAME,
        )
    })
}

/// The installed layouts, in Windows' order.
#[cfg(windows)]
fn installed() -> Vec<windows::Win32::UI::Input::KeyboardAndMouse::HKL> {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayoutList;
    let count = unsafe { GetKeyboardLayoutList(None) };
    let mut layouts = vec![Default::default(); usize::try_from(count).unwrap_or(0)];
    let read = unsafe { GetKeyboardLayoutList(Some(&mut layouts)) };
    layouts.truncate(usize::try_from(read).unwrap_or(0));
    layouts
}

/// Where Windows keeps the layouts, one key per KLID.
#[cfg(windows)]
const LAYOUTS_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Keyboard Layouts";

/// The layout's own name (its `Layout Display Name`, in the user's
/// language, or its `Layout Text`), from its key in the registry.
/// Read once per layout: the dock asks for the label every few seconds,
/// and a variant's name means walking the registry's list of layouts.
#[cfg(windows)]
fn layout_name(layout: windows::Win32::UI::Input::KeyboardAndMouse::HKL) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, PoisonError};
    static NAMES: Mutex<Option<HashMap<isize, Option<String>>>> = Mutex::new(None);
    let key = layout.0 as isize;
    let mut names = NAMES.lock().unwrap_or_else(PoisonError::into_inner);
    names
        .get_or_insert_with(HashMap::new)
        .entry(key)
        .or_insert_with(|| read_layout_name(layout))
        .clone()
}

#[cfg(windows)]
fn read_layout_name(layout: windows::Win32::UI::Input::KeyboardAndMouse::HKL) -> Option<String> {
    let device = ((layout.0 as usize >> 16) & 0xFFFF) as u16;
    let klid = keyboard::klid_of(device).or_else(|| variant_klid(device & 0x0FFF))?;
    let key = format!(r"{LAYOUTS_KEY}\{klid}");
    let shown = registry_text(&key, "Layout Display Name")
        .map(|text| super::jumplist::resolved_text(&text))
        .filter(|text| !text.trim().is_empty());
    shown.or_else(|| registry_text(&key, "Layout Text"))
}

/// The KLID of the variant whose `Layout Id` is `id`.
#[cfg(windows)]
fn variant_klid(id: u16) -> Option<String> {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
    };
    use windows::core::{HSTRING, PWSTR};
    let mut root = HKEY::default();
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            &HSTRING::from(LAYOUTS_KEY),
            None,
            KEY_READ,
            &mut root,
        )
    };
    if opened.is_err() {
        return None;
    }
    let wanted = format!("{id:04x}");
    let mut found = None;
    for index in 0.. {
        let mut name = [0u16; 16];
        let mut length = name.len() as u32;
        let listed = unsafe {
            RegEnumKeyExW(
                root,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut length,
                None,
                None,
                None,
                None,
            )
        };
        if listed.is_err() {
            break;
        }
        let klid = String::from_utf16_lossy(&name[..length as usize]);
        let key = format!(r"{LAYOUTS_KEY}\{klid}");
        if registry_text(&key, "Layout Id").is_some_and(|text| text.eq_ignore_ascii_case(&wanted)) {
            found = Some(klid);
            break;
        }
    }
    let _ = unsafe { RegCloseKey(root) };
    found
}

/// A text value of a key under `HKEY_LOCAL_MACHINE`.
#[cfg(windows)]
fn registry_text(key: &str, value: &str) -> Option<String> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::HSTRING;
    let mut text = [0u16; 260];
    let mut size = (text.len() * 2) as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_SZ,
            None,
            Some(text.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if read.is_err() {
        return None;
    }
    let length = (size as usize / 2).saturating_sub(1).min(text.len());
    Some(String::from_utf16_lossy(&text[..length]))
}

/// Asks the window in front to go to the next installed layout. Nothing
/// with a single layout.
#[cfg(windows)]
pub fn switch_next() {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_INPUTLANGCHANGEREQUEST};

    let Some((window, current)) = front() else {
        return;
    };
    let ids: Vec<isize> = installed().iter().map(|layout| layout.0 as isize).collect();
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

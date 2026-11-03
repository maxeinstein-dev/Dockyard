//! When a WebView2 window (the panel's, Configurações) can't be made: with
//! the disk full, WebView2 can't write its profile and the window never comes
//! up, and the log can't be written either. So the user is told, once per
//! run and per window, by a message of Windows' own, and the failures are
//! counted for the log.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

/// Failures so far in this run, and whether the user was told, by window.
static FAILURES: Mutex<Option<HashMap<&'static str, (u32, bool)>>> = Mutex::new(None);

fn with_failures<T>(change: impl FnOnce(&mut HashMap<&'static str, (u32, bool)>) -> T) -> T {
    let mut failures = FAILURES.lock().unwrap_or_else(PoisonError::into_inner);
    change(failures.get_or_insert_with(HashMap::new))
}

/// Counts one more failure of the window `which`: how many this run.
pub fn count(which: &'static str) -> u32 {
    with_failures(|failures| {
        let entry = failures.entry(which).or_insert((0, false));
        entry.0 += 1;
        entry.0
    })
}

/// Whether the user is to be told now of a failure of `which`: the first
/// time in the run only.
fn first_telling(which: &'static str) -> bool {
    with_failures(|failures| {
        let entry = failures.entry(which).or_insert((0, false));
        !std::mem::replace(&mut entry.1, true)
    })
}

/// Shows `line` in a message box, the first time only for `which`. Off the
/// caller's thread: the box waits for its click.
pub fn tell_once(which: &'static str, line: &'static str) {
    if !first_telling(which) {
        return;
    }
    #[cfg(windows)]
    {
        let spawned = std::thread::Builder::new()
            .name("webview-failure".into())
            .spawn(move || {
                use windows::Win32::UI::WindowsAndMessaging::{
                    MB_ICONWARNING, MB_OK, MB_SETFOREGROUND, MessageBoxW,
                };
                let title = windows::core::HSTRING::from("Dockyard");
                let line = windows::core::HSTRING::from(line);
                unsafe {
                    MessageBoxW(
                        None,
                        &line,
                        &title,
                        MB_OK | MB_ICONWARNING | MB_SETFOREGROUND,
                    )
                };
            });
        if let Err(error) = spawned {
            log::warn!("webview: the message could not be shown: {error}");
        }
    }
    #[cfg(not(windows))]
    let _ = line;
}

/// Whether the test build was told to fail making the window `which`
/// (`DOCKYARD_FAIL_WEBVIEW`, a list such as `panel,settings`), to see what
/// the user gets when WebView2 can't start without filling the disk. Never
/// in the shipped build.
pub fn fails_for_test(which: &str) -> bool {
    #[cfg(feature = "e2e")]
    {
        std::env::var("DOCKYARD_FAIL_WEBVIEW")
            .is_ok_and(|list| list.split(',').any(|name| name.trim() == which))
    }
    #[cfg(not(feature = "e2e"))]
    {
        let _ = which;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_are_counted_by_window() {
        let first = count("test-a");
        assert_eq!(count("test-a"), first + 1);
        assert_eq!(count("test-b"), 1);
    }

    #[test]
    fn the_user_is_told_once_per_window() {
        assert!(first_telling("test-c"));
        assert!(!first_telling("test-c"));
        assert!(first_telling("test-d"));
    }
}

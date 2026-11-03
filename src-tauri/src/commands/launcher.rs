//! What Configurações asks of the launcher (spec 0012): whether another
//! program holds the hotkey, forgetting what it opened (L1), and a new
//! hotkey (L3). Also the dock's hotkey (M14), which takes the keyboard to the
//! dock: the same rules, and never the launcher's.

use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, State};

use super::widgets::WidgetCommandError;
use crate::domain::hotkey::{self, Problem};
use crate::domain::settings::Settings;
use crate::persistence::database::Storage;
use crate::persistence::launcher_repo;

/// What Configurações shows about the launcher; matches `LauncherStatus` in
/// `src/core/launcher.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    /// Another program holds Alt+Space: the launcher opens only from the
    /// menus.
    pub hotkey_taken: bool,
}

#[tauri::command]
pub fn launcher_status() -> LauncherStatus {
    #[cfg(windows)]
    let hotkey_taken = crate::windows::launcher::status().hotkey_taken;
    #[cfg(not(windows))]
    let hotkey_taken = false;
    LauncherStatus { hotkey_taken }
}

/// Forgets everything the launcher opened ("Limpar o histórico").
#[tauri::command]
pub fn clear_launcher_history(state: State<'_, Mutex<Storage>>) -> Result<(), WidgetCommandError> {
    let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
    let conn = storage.connection()?;
    launcher_repo::clear(conn)?;
    log::info!("launcher: history cleared");
    Ok(())
}

/// Why a new hotkey was refused, as the interface sees it; matches
/// `HotkeyErrorCode` in `src/core/launcher.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyErrorCode {
    Malformed,
    NoModifier,
    TakesTyping,
    Reserved,
    /// Another program holds it.
    Taken,
    /// It is the other hotkey's: the launcher's for the dock, the dock's
    /// for the launcher.
    Clash,
    /// It could not be saved; the hotkey before it stays.
    NotSaved,
}

/// `wanted` as a hotkey, or why it can't be one beside the other hotkey,
/// `other` (the dock's for the launcher, the launcher's for the dock).
fn refuse(wanted: &str, other: &str) -> Result<hotkey::Hotkey, HotkeyErrorCode> {
    let parsed = hotkey::check(wanted).map_err(HotkeyErrorCode::from)?;
    if parsed.to_string() == other {
        return Err(HotkeyErrorCode::Clash);
    }
    Ok(parsed)
}

impl From<Problem> for HotkeyErrorCode {
    fn from(problem: Problem) -> Self {
        match problem {
            Problem::Malformed => HotkeyErrorCode::Malformed,
            Problem::NoModifier => HotkeyErrorCode::NoModifier,
            Problem::TakesTyping => HotkeyErrorCode::TakesTyping,
            Problem::Reserved => HotkeyErrorCode::Reserved,
        }
    }
}

/// `{ "code": "taken" }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HotkeyCommandError {
    code: HotkeyErrorCode,
}

impl From<HotkeyErrorCode> for HotkeyCommandError {
    fn from(code: HotkeyErrorCode) -> Self {
        HotkeyCommandError { code }
    }
}

/// Makes `hotkey` (as `hotkey::Hotkey` writes it) the launcher's: refused
/// when the launcher can't take it; with the launcher on, tried for real
/// first, and refused when another program holds it. Async: the launcher
/// answers on the main thread, which this must not hold.
#[tauri::command]
pub async fn set_launcher_hotkey(
    app: AppHandle,
    hotkey: String,
) -> Result<Settings, HotkeyCommandError> {
    let current = super::settings::current(&app);
    let refused = refuse(&hotkey, &current.dock_hotkey);
    if refused.is_err() {
        // The capture ends without a new one: both are held again.
        #[cfg(windows)]
        end_captures();
    }
    let parsed = refused?;
    // The launcher's capture ends with the hotkey tried; the dock's now.
    #[cfg(windows)]
    crate::windows::native_dock::capture_hotkey(false);
    let on = current.launcher;
    #[cfg(windows)]
    if on {
        use crate::windows::launcher::{TryError, try_hotkey};
        try_hotkey(parsed).map_err(|error| match error {
            TryError::Taken => HotkeyErrorCode::Taken,
            TryError::NotReady => HotkeyErrorCode::NotSaved,
        })?;
    }
    #[cfg(not(windows))]
    let _ = (on, parsed);
    let saved = super::settings::change(&app, |settings| {
        settings.launcher_hotkey = parsed.to_string();
    });
    match saved {
        Ok(settings) => {
            log::info!("launcher: the hotkey is now {parsed}");
            Ok(settings)
        }
        Err(error) => {
            log::warn!("launcher: the hotkey could not be saved: {error}");
            // The one tried goes; the one saved comes back.
            #[cfg(windows)]
            crate::windows::launcher::set_enabled(on);
            Err(HotkeyErrorCode::NotSaved.into())
        }
    }
}

/// Configurações captures a new hotkey (`on`) or is done: while it does,
/// the current one is let go, or pressing it would open the launcher.
#[tauri::command]
pub fn capture_launcher_hotkey(on: bool) {
    capture_both(on);
}

/// Both hotkeys are let go while either is captured (`on`): pressing the
/// other one should reach the capture, and be refused there, rather than
/// open the launcher or take the keyboard to the dock.
fn capture_both(on: bool) {
    #[cfg(windows)]
    {
        crate::windows::launcher::capture(on);
        crate::windows::native_dock::capture_hotkey(on);
    }
    #[cfg(not(windows))]
    let _ = on;
}

#[cfg(windows)]
fn end_captures() {
    capture_both(false);
}

/// Makes `hotkey` (as `hotkey::Hotkey` writes it) the dock's (M14): refused
/// when it breaks the launcher's rules or is the launcher's. It is not tried
/// first: when another program holds it, the dock says so in the log. The
/// capture ends either way.
#[tauri::command]
pub fn set_dock_hotkey(app: AppHandle, hotkey: String) -> Result<Settings, HotkeyCommandError> {
    let current = super::settings::current(&app);
    let result = refuse(&hotkey, &current.launcher_hotkey).and_then(|parsed| {
        super::settings::change(&app, |settings| {
            settings.dock_hotkey = parsed.to_string();
        })
        .inspect(|_| log::info!("dock: the hotkey is now {parsed}"))
        .map_err(|error| {
            log::warn!("dock: the hotkey could not be saved: {error}");
            HotkeyErrorCode::NotSaved
        })
    });
    // Saved first: the dock takes the new one as the capture ends.
    #[cfg(windows)]
    end_captures();
    result.map_err(HotkeyCommandError::from)
}

/// Configurações captures the dock's new hotkey (`on`) or is done.
#[tauri::command]
pub fn capture_dock_hotkey(on: bool) {
    capture_both(on);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hotkey_is_refused_by_the_rules_or_for_being_the_others() {
        assert_eq!(
            refuse("Alt+Win+Z", "Alt+Space").unwrap().to_string(),
            "Alt+Win+Z"
        );
        assert_eq!(
            refuse("Alt+Space", "Alt+Space"),
            Err(HotkeyErrorCode::Clash)
        );
        assert_eq!(
            refuse("Win+Alt+Z", "Alt+Space"),
            Err(HotkeyErrorCode::Malformed)
        );
        assert_eq!(
            refuse("Ctrl+C", "Alt+Space"),
            Err(HotkeyErrorCode::TakesTyping)
        );
        assert_eq!(refuse("Win+L", "Alt+Space"), Err(HotkeyErrorCode::Reserved));
    }

    #[test]
    fn the_new_code_reaches_the_interface_as_clash() {
        assert_eq!(
            serde_json::to_value(HotkeyCommandError::from(HotkeyErrorCode::Clash)).unwrap(),
            serde_json::json!({ "code": "clash" })
        );
    }
}

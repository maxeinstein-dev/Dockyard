//! The commands of the Capturas de tela's panel (F9 of estudo 0002): the
//! newest screenshots, each opened or dragged out by its key, and the
//! folder opened in File Explorer.
//!
//! The page never gets a path: as with a pinned folder's entries (spec
//! 0025), it gets keys, and Rust keeps what each key stands for, from the
//! last listing only.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;

use super::apps::AppCommandError;
use crate::domain::apps::AppError;
use crate::windows::screenshots;

/// The side of a screenshot's picture, in pixels: twice the panel's cell,
/// sharp at 200 %.
const PICTURE_SIDE: u32 = 160;

/// The files of the last listing, by key.
static SHOWN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// One screenshot, for the panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotView {
    pub key: u32,
    /// The file's name, for its accessible name and its tip.
    pub name: String,
    /// When it was saved, in milliseconds since 1970.
    pub modified: u64,
    /// A PNG `data:` URL; `None` when the image could not be read.
    pub picture: Option<String>,
}

/// The newest screenshots, the newest first; `folder` is false when Windows
/// has no Screenshots folder yet (nothing was ever saved there).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotsView {
    pub folder: bool,
    pub items: Vec<ScreenshotView>,
}

/// The newest screenshots with their pictures. Read off the window's
/// thread: the folder and the images take the disk's time.
#[tauri::command]
pub async fn list_screenshots() -> Result<ScreenshotsView, AppCommandError> {
    let read = tauri::async_runtime::spawn_blocking(|| {
        let Some(folder) = screenshots::folder().filter(|folder| folder.is_dir()) else {
            return (
                Vec::new(),
                ScreenshotsView {
                    folder: false,
                    items: Vec::new(),
                },
            );
        };
        let entries = screenshots::newest(&folder).unwrap_or_else(|error| {
            log::warn!(
                "screenshots: the folder could not be read: {}",
                error.kind()
            );
            Vec::new()
        });
        let mut paths = Vec::new();
        let items = entries
            .into_iter()
            .map(|entry| {
                let path = folder.join(&entry.name);
                let picture = screenshots::thumbnail(&path, PICTURE_SIDE)
                    .map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)));
                paths.push(path);
                ScreenshotView {
                    key: (paths.len() - 1) as u32,
                    name: entry.name,
                    modified: entry.modified,
                    picture,
                }
            })
            .collect();
        (
            paths,
            ScreenshotsView {
                folder: true,
                items,
            },
        )
    })
    .await
    .map_err(|_| AppError::Unavailable)?;
    let (paths, view) = read;
    *SHOWN.lock().unwrap_or_else(PoisonError::into_inner) = paths;
    Ok(view)
}

/// The file of `key` in the last listing.
fn path_of(key: u32) -> Result<PathBuf, AppError> {
    SHOWN
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(key as usize)
        .cloned()
        .ok_or(AppError::NotFound)
}

/// Opens the screenshot `key` with its program, as a double click would.
#[tauri::command]
pub async fn open_screenshot(key: u32) -> Result<(), AppCommandError> {
    let path = path_of(key)?;
    open(path.to_string_lossy().into_owned()).await
}

/// Opens the Screenshots folder in File Explorer.
#[tauri::command]
pub async fn open_screenshots_folder() -> Result<(), AppCommandError> {
    let folder = screenshots::folder()
        .filter(|folder| folder.is_dir())
        .ok_or(AppError::NotFound)?;
    open(folder.to_string_lossy().into_owned()).await
}

async fn open(target: String) -> Result<(), AppCommandError> {
    #[cfg(windows)]
    {
        use crate::windows::jumplist::{Runnable, run};
        tauri::async_runtime::spawn_blocking(move || run(&Runnable::Open(target)))
            .await
            .map_err(|_| AppError::LaunchFailed)?
            .map_err(|error| {
                log::warn!("screenshots: could not be opened: {error}");
                AppError::LaunchFailed
            })?;
    }
    #[cfg(not(windows))]
    let _ = target;
    Ok(())
}

/// Drags the screenshot `key` out of the panel, as File Explorer drags a
/// file. Synchronous on purpose: Tauri runs it on the window's thread,
/// whose OLE the drag needs, while the button is still held; it returns
/// once the file is dropped or let go.
#[tauri::command]
pub fn drag_screenshot(key: u32) -> Result<(), AppCommandError> {
    let path = path_of(key)?;
    #[cfg(windows)]
    screenshots::drag(&path).map_err(|error| {
        log::warn!("screenshots: the drag failed: {error}");
        AppError::LaunchFailed
    })?;
    #[cfg(not(windows))]
    let _ = path;
    Ok(())
}

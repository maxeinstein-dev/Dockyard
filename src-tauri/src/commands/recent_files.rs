//! The commands of the Arquivos' panel (F9 of estudo 0002): the files
//! opened last, each opened or dragged out by its key.
//!
//! The page never gets a path, as with the Capturas de tela: it gets keys,
//! and Rust keeps what each key stands for, from the last listing only.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use serde::Serialize;

use super::apps::AppCommandError;
use crate::domain::apps::AppError;
use crate::domain::folders::{EntryKind, kind_of};
use crate::windows::recent_files;

/// The files of the last listing, by key.
static SHOWN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// One recent file, for the panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentFileView {
    pub key: u32,
    /// The file's name.
    pub name: String,
    /// The name of the folder it is in (`Documentos`), never its path.
    pub folder: String,
    /// What it is, for its icon, as a pinned folder's entries.
    pub kind: EntryKind,
}

/// The files opened last, the newest first.
#[tauri::command]
pub async fn list_recent_files() -> Result<Vec<RecentFileView>, AppCommandError> {
    let paths = tauri::async_runtime::spawn_blocking(|| {
        let folder = recent_files::folder().ok_or(AppError::Unavailable)?;
        recent_files::newest(&folder).map_err(|error| {
            log::warn!("recent files: could not be read: {}", error.kind());
            AppError::Unavailable
        })
    })
    .await
    .map_err(|_| AppError::Unavailable)??;
    let views = paths
        .iter()
        .enumerate()
        .map(|(key, path)| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let folder = path
                .parent()
                .and_then(|parent| parent.file_name())
                .map(|folder| folder.to_string_lossy().into_owned())
                .unwrap_or_default();
            RecentFileView {
                key: key as u32,
                kind: kind_of(&name, false),
                name,
                folder,
            }
        })
        .collect();
    *SHOWN.lock().unwrap_or_else(PoisonError::into_inner) = paths;
    Ok(views)
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

/// Opens the recent file `key` with its program, as a double click would.
#[tauri::command]
pub async fn open_recent_file(key: u32) -> Result<(), AppCommandError> {
    let path = path_of(key)?;
    #[cfg(windows)]
    {
        use crate::windows::jumplist::{Runnable, run};
        let target = path.to_string_lossy().into_owned();
        tauri::async_runtime::spawn_blocking(move || run(&Runnable::Open(target)))
            .await
            .map_err(|_| AppError::LaunchFailed)?
            .map_err(|error| {
                log::warn!("recent files: could not be opened: {error}");
                AppError::LaunchFailed
            })?;
    }
    #[cfg(not(windows))]
    let _ = path;
    Ok(())
}

/// Drags the recent file `key` out of the panel, as the Capturas de tela
/// drags a screenshot: on the window's thread, while the button is held.
#[tauri::command]
pub fn drag_recent_file(key: u32) -> Result<(), AppCommandError> {
    let path = path_of(key)?;
    #[cfg(windows)]
    crate::windows::screenshots::drag(&path).map_err(|error| {
        log::warn!("recent files: the drag failed: {error}");
        AppError::LaunchFailed
    })?;
    #[cfg(not(windows))]
    let _ = path;
    Ok(())
}

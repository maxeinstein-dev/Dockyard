//! The user's skins of the bar (spec 0020, Q14): the `Skins` folder of the
//! app's data folder, listed for the style gallery of Configurações with
//! the folders refused and why, the command that opens it, and the skin the
//! dock wears with the Custom style.
//!
//! Nothing here sends a path to a window, and logs name only the capability
//! and the kind of error: never a folder's name.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::apps::AppCommandError;
use crate::domain::apps::AppError;
use crate::domain::settings::{DockSettings, DockStyle};
use crate::domain::skin::{self, CachedSkin, FolderSkin, Middle, SkinProblem};
use crate::windows::apps;

fn folder(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| skin::root(&dir))
}

/// The folder skin `id`, from the dock's cache: read again only when its
/// files changed.
pub fn named(app: &AppHandle, id: Option<&str>) -> Option<Arc<CachedSkin>> {
    skin::cached(&folder(app)?, id?)
}

/// Whether the dock already said its folder skin can't be used: said once,
/// and again only after one was used.
static WARNED: AtomicBool = AtomicBool::new(false);

/// The folder skin the dock wears with `dock`: `None` for any other style,
/// or when the one named can't be used (the bar is then Nativo's, said once
/// in the log).
pub fn chosen(app: &AppHandle, dock: &DockSettings) -> Option<Arc<CachedSkin>> {
    if dock.style != DockStyle::Custom {
        return None;
    }
    let found = named(app, dock.custom_skin.as_deref());
    if found.is_some() {
        WARNED.store(false, Ordering::Relaxed);
    } else if !WARNED.swap(true, Ordering::Relaxed) {
        log::warn!("skins: the chosen folder skin can't be used; the bar is drawn as Nativo");
    }
    found
}

/// A skin of the folder, for the gallery: what its card's miniature draws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinInfo {
    /// The name of its folder: what `dock.customSkin` holds.
    pub id: String,
    /// The name its `skin.json` gives, or the folder's when that is blank.
    pub name: String,
    /// `data:image/png;base64,…`.
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub left: u32,
    pub right: u32,
    pub repeat: bool,
    pub pixelated: bool,
    pub dark: bool,
}

/// What the skins folder holds: the skins, and the folders refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FolderSkins {
    pub skins: Vec<SkinInfo>,
    pub problems: Vec<SkinProblem>,
}

fn info_of(skin: FolderSkin) -> SkinInfo {
    let meta = &skin.meta;
    let name = meta.name.trim();
    SkinInfo {
        name: if name.is_empty() {
            skin.id.clone()
        } else {
            name.to_owned()
        },
        url: format!("data:image/png;base64,{}", STANDARD.encode(&skin.png)),
        width: meta.width,
        height: meta.height,
        left: meta.slice.left,
        right: meta.slice.right,
        repeat: meta.middle == Middle::Repeat,
        pixelated: meta.pixelated,
        dark: meta.dark.unwrap_or(true),
        id: skin.id,
    }
}

/// Lists the skins of the folder. Reads it again each time: a change there
/// is what the gallery wants to see when it opens.
#[tauri::command]
pub async fn list_folder_skins(app: AppHandle) -> Result<FolderSkins, AppCommandError> {
    let root = folder(&app).ok_or(AppError::Unavailable)?;
    let (skins, problems) = tauri::async_runtime::spawn_blocking(move || skin::read_folder(&root))
        .await
        .map_err(|_| AppCommandError::from(AppError::Unavailable))?;
    Ok(FolderSkins {
        skins: skins.into_iter().map(info_of).collect(),
        problems,
    })
}

/// Opens the skins folder in File Explorer, making it if it is not there.
#[tauri::command]
pub fn open_skins_folder(app: AppHandle) -> Result<(), AppCommandError> {
    let folder = folder(&app).ok_or(AppError::Unavailable)?;
    std::fs::create_dir_all(&folder).map_err(|error| {
        log::warn!("skins: could not make the skins folder: {}", error.kind());
        AppCommandError::from(AppError::Unavailable)
    })?;
    apps::open_folder(&folder).map_err(|error| {
        log::warn!("skins: could not open the skins folder: {error}");
        AppCommandError::from(AppError::Unavailable)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::skin::{SkinMeta, Slice};

    #[test]
    fn a_skin_goes_to_the_gallery_as_a_data_url_with_its_cut() {
        let meta = SkinMeta {
            name: "  ".into(),
            image: "skin.png".into(),
            width: 9,
            height: 4,
            slice: Slice {
                top: 0,
                right: 3,
                bottom: 0,
                left: 2,
            },
            middle: Middle::Repeat,
            pixelated: true,
            dark: None,
            ink: None,
            indicator: None,
            vertical: None,
        };
        let info = info_of(FolderSkin {
            id: "Minha".into(),
            meta,
            png: vec![1, 2, 3],
            vertical_png: None,
        });
        assert_eq!(
            info,
            SkinInfo {
                id: "Minha".into(),
                // A blank name is the folder's.
                name: "Minha".into(),
                url: "data:image/png;base64,AQID".into(),
                width: 9,
                height: 4,
                left: 2,
                right: 3,
                repeat: true,
                pixelated: true,
                dark: true,
            }
        );
        let json = serde_json::to_value(FolderSkins {
            skins: Vec::new(),
            problems: vec![SkinProblem {
                folder: "x".into(),
                kind: skin::ProblemKind::SizeMismatch,
            }],
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"skins": [], "problems": [{"folder": "x", "kind": "sizeMismatch"}]})
        );
    }
}

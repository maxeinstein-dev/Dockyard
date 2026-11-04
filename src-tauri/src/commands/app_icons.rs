//! The icon of one program (spec 0024): what its menu offers ("Original",
//! the packs, "Escolher imagem…") and the choice, kept in the settings
//! (`dock.appIcons`). The dock reads it in `apps::dock_icon_png`.
//!
//! An image the user picks is read once, checked and kept as a PNG in the
//! `AppIcons` folder; nothing here sends a path to a window.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use super::apps::{AppCommandError, target_of};
use super::widgets::lock;
use crate::domain::app_icon::{AppIcon, MAX_CHOSEN_BYTES, choice_key, chosen_png};
use crate::domain::apps::AppError;
use crate::persistence::app_icons;
use crate::persistence::database::Storage;

/// A pack a program can wear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackChoice {
    pub id: String,
    pub name: String,
}

/// What a program's "Ícone" shows: its choice (`None`: the dock's pack) and
/// the packs there are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppIconOptions {
    pub current: Option<AppIcon>,
    pub packs: Vec<PackChoice>,
}

fn folder(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| app_icons::root(&dir))
}

/// The id in the Apps folder of the pinned program `id`.
fn app_id_of(state: &State<'_, Mutex<Storage>>, id: &str) -> Result<String, AppCommandError> {
    target_of(&mut lock(state), id).map_err(AppCommandError::from)
}

/// The icon of the pinned program `id` and the packs it can wear.
#[tauri::command]
pub fn app_icon_options(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<AppIconOptions, AppCommandError> {
    let app_id = app_id_of(&state, &id)?;
    let current = super::settings::current(&app)
        .dock
        .app_icons
        .get(&choice_key(&app_id))
        .cloned();
    let packs = super::icons::usable(&app)
        .into_iter()
        .map(|(id, name)| PackChoice { id, name })
        .collect();
    Ok(AppIconOptions { current, packs })
}

/// Has the program `app_id` wear `icon` (`None`: the dock's pack again),
/// and removes the image it wore before if no program wears it any more.
fn wear(app: &AppHandle, app_id: &str, icon: Option<AppIcon>) -> Result<(), AppCommandError> {
    let key = choice_key(app_id);
    let mut before = None;
    let saved = super::settings::change(app, |settings| {
        let icons = &mut settings.dock.app_icons;
        before = match icon {
            Some(icon) => icons.insert(key, icon),
            None => icons.remove(&key),
        };
    })
    .map_err(|error| {
        log::warn!("icons: a program's icon could not be kept: {error}");
        AppCommandError::from(AppError::WriteFailed)
    })?;
    let left = before.as_ref().and_then(AppIcon::file).filter(|file| {
        !saved
            .dock
            .app_icons
            .values()
            .any(|icon| icon.file() == Some(*file))
    });
    if let (Some(file), Some(folder)) = (left, folder(app)) {
        app_icons::remove(&folder, file);
    }
    Ok(())
}

/// Has the pinned program `id` wear `icon`: its Windows icon or a pack's
/// image; `None` for the dock's pack again. An image of the user's own comes
/// only from `choose_app_icon_image`.
#[tauri::command]
pub fn set_app_icon(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
    icon: Option<AppIcon>,
) -> Result<(), AppCommandError> {
    let app_id = app_id_of(&state, &id)?;
    let known = match &icon {
        None | Some(AppIcon::Original) => true,
        Some(AppIcon::Pack { pack }) => super::icons::usable(&app)
            .iter()
            .any(|(known, _)| known == pack),
        Some(AppIcon::Image { .. }) => false,
    };
    if !known {
        return Err(AppError::InvalidApp.into());
    }
    wear(&app, &app_id, icon)
}

/// "Escolher imagem…": the Windows dialog for a PNG or an ICO, kept as the
/// icon of the pinned program `id`. `false` when the dialog was cancelled or
/// the image refused (too big, not an image, a size the dock does not take:
/// only logged).
#[tauri::command]
pub async fn choose_app_icon_image(
    app: AppHandle,
    state: State<'_, Mutex<Storage>>,
    id: String,
) -> Result<bool, AppCommandError> {
    let app_id = app_id_of(&state, &id)?;
    let handle = app.clone();
    let lookup = app_id.clone();
    let kept = tauri::async_runtime::spawn_blocking(move || {
        let name = crate::windows::apps::name_of(&lookup).unwrap_or_default();
        let text = &crate::i18n::messages().tray;
        let title = text.choose_icon_title.replace("{name}", &name);
        let path = crate::windows::file_dialog::pick_image(&title, text.image_files)?;
        let too_big = std::fs::metadata(&path).map_or(true, |meta| meta.len() > MAX_CHOSEN_BYTES);
        if too_big {
            log::warn!("icons: the image chosen is too big");
            return None;
        }
        let png = match std::fs::read(&path).map(|bytes| chosen_png(&bytes)) {
            Ok(Ok(png)) => png,
            Ok(Err(error)) => {
                log::warn!("icons: the image chosen was refused: {error:?}");
                return None;
            }
            Err(error) => {
                log::warn!(
                    "icons: the image chosen could not be read: {}",
                    error.kind()
                );
                return None;
            }
        };
        let folder = folder(&handle)?;
        app_icons::save(&folder, &png)
            .inspect_err(|error| {
                log::warn!("icons: the image could not be kept: {}", error.kind());
            })
            .ok()
    })
    .await
    .map_err(|_| AppCommandError::from(AppError::Unavailable))?;
    match kept {
        Some(file) => wear(&app, &app_id, Some(AppIcon::Image { file })).map(|()| true),
        None => Ok(false),
    }
}

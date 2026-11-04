//! Exporting and importing the dock (spec 0026): the Windows dialogs, the
//! file read and written by Rust, and the import applied at the next start.
//! The page never names a path: it asks to export, to choose a file (and
//! gets what it holds), to confirm or to cancel.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use tauri::{AppHandle, Manager};

use crate::domain::widgets::UnavailableReason;
use crate::persistence::database::Storage;
use crate::persistence::transfer::{self, Checked, MAX_FILE_BYTES, Summary, TransferError};

/// The file chosen to import, checked, until confirmed or cancelled.
static CHOSEN: Mutex<Option<Checked>> = Mutex::new(None);

fn data_dir(app: &AppHandle) -> Result<PathBuf, TransferError> {
    app.path().app_data_dir().map_err(|_| TransferError::Failed)
}

/// A time for a name: `20261104T180000Z`.
fn stamp() -> String {
    jiff::Timestamp::now()
        .strftime("%Y%m%dT%H%M%SZ")
        .to_string()
}

/// At start, before the settings and the database are read: a pending
/// import takes the place of the data (`transfer::apply_pending`). The
/// settings and the database must share the folder, as they do on Windows.
pub fn apply_pending(app: &AppHandle) {
    let (Ok(data), Ok(config)) = (app.path().app_data_dir(), app.path().app_config_dir()) else {
        return;
    };
    if data != config {
        if data.join(transfer::PENDING_DIR).exists() {
            log::warn!("transfer: the settings and the data are apart; the import waits");
        }
        return;
    }
    match transfer::apply_pending(&data, &stamp()) {
        Some(true) => log::info!("transfer: an import took the place of the data"),
        Some(false) => {
            log::warn!("transfer: the import could not be applied; the data is as it was")
        }
        None => {}
    }
}

/// "Exportar…": the "Salvar como" dialog, then the copy written there.
/// `false` when the dialog was cancelled.
#[tauri::command]
pub async fn export_copy(app: AppHandle) -> Result<bool, TransferError> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let text = &crate::i18n::messages().tray;
        let today = jiff::Timestamp::now().strftime("%Y-%m-%d").to_string();
        let Some(path) = crate::windows::file_dialog::save_file(
            text.export_title,
            text.copy_files,
            "*.dockyard",
            transfer::EXTENSION,
            &format!("Dockyard-{today}.{}", transfer::EXTENSION),
        ) else {
            return Ok(false);
        };
        let settings = serde_json::to_value(super::settings::current(&handle))
            .map_err(|_| TransferError::Failed)?;
        let dir = data_dir(&handle)?;
        let bytes = {
            let state = handle.state::<Mutex<Storage>>();
            let mut storage = state.lock().unwrap_or_else(PoisonError::into_inner);
            let conn = storage.connection().map_err(|_| TransferError::Failed)?;
            transfer::export(
                conn,
                settings,
                &dir,
                env!("CARGO_PKG_VERSION"),
                &jiff::Timestamp::now().to_string(),
            )?
        };
        std::fs::write(&path, bytes).map_err(|error| {
            log::warn!("transfer: the copy could not be written: {}", error.kind());
            TransferError::Failed
        })?;
        log::info!("transfer: a copy was exported");
        Ok(true)
    })
    .await
    .map_err(|_| TransferError::Failed)?
}

/// "Importar…": the "Abrir" dialog, then the file read and checked; what it
/// holds, or `None` when the dialog was cancelled. Nothing changes until
/// `confirm_import`.
#[tauri::command]
pub async fn choose_import(app: AppHandle) -> Result<Option<Summary>, TransferError> {
    tauri::async_runtime::spawn_blocking(move || {
        let text = &crate::i18n::messages().tray;
        let Some(path) = crate::windows::file_dialog::pick_file(
            text.import_title,
            text.copy_files,
            "*.dockyard",
        ) else {
            return Ok(None);
        };
        let size = std::fs::metadata(&path)
            .map_err(|_| TransferError::Failed)?
            .len();
        if size > MAX_FILE_BYTES {
            return Err(TransferError::TooBig);
        }
        let bytes = std::fs::read(&path).map_err(|_| TransferError::Failed)?;
        let checked = transfer::check(&bytes, &data_dir(&app)?).inspect_err(|error| {
            log::warn!("transfer: a copy was refused: {error}");
        })?;
        let summary = checked.summary.clone();
        *CHOSEN.lock().unwrap_or_else(PoisonError::into_inner) = Some(checked);
        Ok(Some(summary))
    })
    .await
    .map_err(|_| TransferError::Failed)?
}

/// "Importar e reiniciar": the file chosen waits for the next start, and
/// the Dockyard starts again (letting go of the taskbar and the database
/// first, as a restart after a failure does).
#[tauri::command]
pub fn confirm_import(app: AppHandle) -> Result<(), TransferError> {
    let chosen = CHOSEN.lock().unwrap_or_else(PoisonError::into_inner).take();
    let Some(checked) = chosen else {
        return Err(TransferError::NotACopy);
    };
    transfer::stage(&checked, &data_dir(&app)?).inspect_err(|error| {
        log::warn!("transfer: the import could not be staged: {error}");
    })?;
    log::info!("transfer: an import waits for the restart");
    // The database closes, so the next start can move it aside.
    if let Some(state) = app.try_state::<Mutex<Storage>>() {
        *state.lock().unwrap_or_else(PoisonError::into_inner) =
            Storage::unavailable(UnavailableReason::OpenFailed);
    }
    #[cfg(windows)]
    {
        crate::windows::appbar::remove();
        crate::windows::taskbar::release();
        crate::windows::window_spaces::release_all();
    }
    tauri_plugin_single_instance::destroy(&app);
    app.restart();
}

/// "Cancelar": the file chosen is forgotten.
#[tauri::command]
pub fn cancel_import() {
    CHOSEN.lock().unwrap_or_else(PoisonError::into_inner).take();
}

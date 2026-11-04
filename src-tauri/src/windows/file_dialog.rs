//! The Windows "Abrir" and "Salvar como" dialogs: an image the user picks
//! (spec 0024: the icon of a program) and the copy of the dock (spec 0026).
//! Blocks the caller until the dialog closes: never the dock's thread.
//!
//! The dialog runs on a thread of its own, made for it: the shell's dialogs
//! need a single-threaded apartment, and a thread of a pool may already be
//! in the multithreaded one, where the dialog does not open.

use std::path::PathBuf;

/// The image file the user picks (PNG or ICO), titled `title`; `None` when
/// the dialog is cancelled or can't open (logged).
pub fn pick_image(title: &str, filter_name: &str) -> Option<PathBuf> {
    pick_file(title, filter_name, "*.png;*.ico")
}

/// The file the user picks among `spec` (`*.png;*.ico`), titled `title`;
/// `None` when cancelled or failed (logged).
#[cfg(windows)]
pub fn pick_file(title: &str, filter_name: &str, spec: &str) -> Option<PathBuf> {
    let (title, filter_name, spec) = (title.to_owned(), filter_name.to_owned(), spec.to_owned());
    on_own_thread(move || show(&title, &filter_name, &spec, None))
}

/// Where the user saves a file of the kind `spec` (`*.dockyard`), offered
/// as `name` with the extension `extension`; `None` when cancelled or
/// failed (logged).
#[cfg(windows)]
pub fn save_file(
    title: &str,
    filter_name: &str,
    spec: &str,
    extension: &str,
    name: &str,
) -> Option<PathBuf> {
    let (title, filter_name, spec) = (title.to_owned(), filter_name.to_owned(), spec.to_owned());
    let save = (extension.to_owned(), name.to_owned());
    on_own_thread(move || show(&title, &filter_name, &spec, Some(save)))
}

#[cfg(windows)]
fn on_own_thread(
    dialog: impl FnOnce() -> ::windows::core::Result<Option<PathBuf>> + Send + 'static,
) -> Option<PathBuf> {
    let picked = std::thread::Builder::new()
        .name("file-dialog".into())
        .spawn(move || {
            use ::windows::Win32::System::Com::{
                COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
            };
            let entered = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if entered.is_err() {
                log::warn!("dialogs: the dialog's thread has no apartment: {entered:?}");
                return None;
            }
            let picked = dialog();
            unsafe { CoUninitialize() };
            match picked {
                Ok(path) => path,
                Err(error) => {
                    log::warn!("dialogs: the file dialog failed: {error}");
                    None
                }
            }
        })
        .ok()?
        .join();
    picked.ok().flatten()
}

/// The dialog itself, on a single-threaded apartment: "Abrir", or "Salvar
/// como" with `save` (the extension and the name offered). The path chosen,
/// `None` when cancelled.
#[cfg(windows)]
fn show(
    title: &str,
    filter_name: &str,
    spec: &str,
    save: Option<(String, String)>,
) -> ::windows::core::Result<Option<PathBuf>> {
    use ::windows::Win32::Foundation::ERROR_CANCELLED;
    use ::windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
    use ::windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use ::windows::Win32::UI::Shell::{
        FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_OVERWRITEPROMPT, FileOpenDialog,
        FileSaveDialog, IFileDialog, IFileOpenDialog, IFileSaveDialog, SIGDN_FILESYSPATH,
    };
    use ::windows::core::{HRESULT, HSTRING, Interface, PCWSTR};

    let dialog: IFileDialog = match &save {
        None => unsafe {
            CoCreateInstance::<_, IFileOpenDialog>(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)
        }?
        .cast()?,
        Some(_) => unsafe {
            CoCreateInstance::<_, IFileSaveDialog>(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
        }?
        .cast()?,
    };
    let name = HSTRING::from(filter_name);
    let spec = HSTRING::from(spec);
    let filters = [COMDLG_FILTERSPEC {
        pszName: PCWSTR(name.as_ptr()),
        pszSpec: PCWSTR(spec.as_ptr()),
    }];
    unsafe {
        dialog.SetFileTypes(&filters)?;
        dialog.SetTitle(&HSTRING::from(title))?;
        let options = dialog.GetOptions()?;
        let wanted = match &save {
            None => FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST,
            Some((extension, offered)) => {
                dialog.SetDefaultExtension(&HSTRING::from(extension.as_str()))?;
                dialog.SetFileName(&HSTRING::from(offered.as_str()))?;
                FOS_FORCEFILESYSTEM | FOS_OVERWRITEPROMPT
            }
        };
        dialog.SetOptions(options | wanted)?;
        if let Err(error) = dialog.Show(None) {
            return if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                Ok(None)
            } else {
                Err(error)
            };
        }
        let item = dialog.GetResult()?;
        let path = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let text = path.to_string().ok();
        CoTaskMemFree(Some(path.0 as *const _));
        Ok(text.map(PathBuf::from))
    }
}

#[cfg(not(windows))]
pub fn pick_file(_title: &str, _filter_name: &str, _spec: &str) -> Option<PathBuf> {
    None
}

#[cfg(not(windows))]
pub fn save_file(
    _title: &str,
    _filter_name: &str,
    _spec: &str,
    _extension: &str,
    _name: &str,
) -> Option<PathBuf> {
    None
}

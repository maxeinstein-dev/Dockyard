//! The Windows "Abrir" dialog, for a file the user picks: an image (spec
//! 0024: the icon of a program) or a program to pin ("Procurar…", F7 of
//! estudo 0002). Blocks the caller until the dialog closes: never the dock's
//! thread.
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

/// The program the user picks to pin (an `.exe` or a shortcut), titled
/// `title`; `None` when the dialog is cancelled or can't open (logged).
pub fn pick_program(title: &str, filter_name: &str) -> Option<PathBuf> {
    pick_file(title, filter_name, "*.exe;*.lnk")
}

/// A file the user picks among those `spec` matches (`*.png;*.ico`), shown
/// as `filter_name`.
#[cfg(windows)]
fn pick_file(title: &str, filter_name: &str, spec: &str) -> Option<PathBuf> {
    let (title, filter_name, spec) = (title.to_owned(), filter_name.to_owned(), spec.to_owned());
    let picked = std::thread::Builder::new()
        .name("pick-file".into())
        .spawn(move || {
            use ::windows::Win32::System::Com::{
                COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
            };
            let entered = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if entered.is_err() {
                log::warn!("dialog: its thread has no apartment: {entered:?}");
                return None;
            }
            let picked = show(&title, &filter_name, &spec);
            unsafe { CoUninitialize() };
            match picked {
                Ok(path) => path,
                Err(error) => {
                    log::warn!("dialog: it failed: {error}");
                    None
                }
            }
        })
        .ok()?
        .join();
    picked.ok().flatten()
}

/// The dialog itself, on a single-threaded apartment: the path chosen,
/// `None` when cancelled.
#[cfg(windows)]
fn show(title: &str, filter_name: &str, spec: &str) -> ::windows::core::Result<Option<PathBuf>> {
    use ::windows::Win32::Foundation::ERROR_CANCELLED;
    use ::windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
    use ::windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use ::windows::Win32::UI::Shell::{
        FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH,
    };
    use ::windows::core::{HRESULT, HSTRING, PCWSTR};

    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }?;
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
        dialog.SetOptions(options | FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST)?;
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
fn pick_file(_title: &str, _filter_name: &str, _spec: &str) -> Option<PathBuf> {
    None
}

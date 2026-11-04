//! The Windows "Abrir" dialog, for an image the user picks (spec 0024: the
//! icon of a program). Runs on the thread that calls it, which it blocks
//! until the dialog closes: never the dock's.

use std::path::PathBuf;

/// The image file the user picks (PNG or ICO), titled `title`; `None` when
/// the dialog is cancelled or can't open.
#[cfg(windows)]
pub fn pick_image(title: &str, filter_name: &str) -> Option<PathBuf> {
    use ::windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree,
    };
    use ::windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use ::windows::Win32::UI::Shell::{
        FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FileOpenDialog, IFileOpenDialog, SIGDN_FILESYSPATH,
    };
    use ::windows::core::{HSTRING, PCWSTR};

    // Already initialized on this thread in another mode: fine either way.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }.ok()?;
    let name = HSTRING::from(filter_name);
    let spec = HSTRING::from("*.png;*.ico");
    let filters = [COMDLG_FILTERSPEC {
        pszName: PCWSTR(name.as_ptr()),
        pszSpec: PCWSTR(spec.as_ptr()),
    }];
    unsafe {
        dialog.SetFileTypes(&filters).ok()?;
        dialog.SetTitle(&HSTRING::from(title)).ok()?;
        let options = dialog.GetOptions().ok()?;
        dialog
            .SetOptions(options | FOS_FORCEFILESYSTEM | FOS_FILEMUSTEXIST)
            .ok()?;
        // Cancelled: an error too.
        dialog.Show(None).ok()?;
        let item = dialog.GetResult().ok()?;
        let path = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let text = path.to_string().ok();
        CoTaskMemFree(Some(path.0 as *const _));
        text.map(PathBuf::from)
    }
}

#[cfg(not(windows))]
pub fn pick_image(_title: &str, _filter_name: &str) -> Option<PathBuf> {
    None
}

//! Folders on the dock (spec 0025), as Windows has them: the newest entries
//! of a pinned folder, where the Downloads are, and the Recycle Bin (full or
//! empty, and emptied with Windows' own question).
//!
//! A folder is read only when its panel opens, off the dock's thread, and
//! nothing of it is kept after: the panel's keys are the only trace.

use std::path::{Path, PathBuf};

use crate::domain::folders::{FolderEntry, MAX_FOLDER_ENTRIES, MAX_LOOKED_AT, newest};

/// The newest entries of the folder `path`, the newest first, and whether
/// the folder had more than were looked at. Hidden and system entries are
/// left out, as File Explorer leaves them out.
pub fn newest_in(path: &Path) -> std::io::Result<(Vec<FolderEntry>, bool)> {
    let mut entries = Vec::new();
    let mut more = false;
    for entry in std::fs::read_dir(path)? {
        if entries.len() == MAX_LOOKED_AT {
            more = true;
            break;
        }
        let Ok(entry) = entry else { continue };
        let Ok(meta) = entry.metadata() else { continue };
        if hidden(&meta) {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_millis() as u64);
        entries.push(FolderEntry {
            name,
            modified,
            folder: meta.is_dir(),
        });
    }
    Ok((newest(entries, MAX_FOLDER_ENTRIES), more))
}

#[cfg(windows)]
fn hidden(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const HIDDEN: u32 = 0x2;
    const SYSTEM: u32 = 0x4;
    meta.file_attributes() & (HIDDEN | SYSTEM) != 0
}

#[cfg(not(windows))]
fn hidden(_meta: &std::fs::Metadata) -> bool {
    false
}

/// The user's Downloads folder.
#[cfg(windows)]
pub fn downloads() -> Option<PathBuf> {
    use ::windows::Win32::System::Com::CoTaskMemFree;
    use ::windows::Win32::UI::Shell::{FOLDERID_Downloads, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
    let text = unsafe { SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None) }.ok()?;
    let path = unsafe { text.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(text.0.cast_const().cast())) };
    path.map(PathBuf::from)
}

#[cfg(not(windows))]
pub fn downloads() -> Option<PathBuf> {
    None
}

/// Whether the Recycle Bin has anything, on any drive; `None` when Windows
/// does not say.
#[cfg(windows)]
pub fn bin_full() -> Option<bool> {
    use ::windows::Win32::UI::Shell::{SHQUERYRBINFO, SHQueryRecycleBinW};
    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        ..Default::default()
    };
    unsafe { SHQueryRecycleBinW(::windows::core::PCWSTR::null(), &mut info) }.ok()?;
    Some(info.i64NumItems > 0)
}

#[cfg(not(windows))]
pub fn bin_full() -> Option<bool> {
    None
}

/// Empties the Recycle Bin after Windows asks the user (its own question,
/// with the count of items). Blocks until answered, on a thread of its own:
/// the question is a window of the shell. `false` when nothing was emptied
/// (the answer was no, or it failed: logged).
#[cfg(windows)]
pub fn empty_bin() -> bool {
    let emptied = std::thread::Builder::new()
        .name("empty-bin".into())
        .spawn(|| {
            use ::windows::Win32::System::Com::{
                COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize,
            };
            use ::windows::Win32::UI::Shell::SHEmptyRecycleBinW;
            let entered = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            let result = unsafe { SHEmptyRecycleBinW(None, ::windows::core::PCWSTR::null(), 0) };
            if entered.is_ok() {
                unsafe { CoUninitialize() };
            }
            match result {
                Ok(()) => true,
                Err(error) => {
                    // "No" to the question comes back as an error too.
                    log::info!("folders: the Recycle Bin was not emptied: {}", error.code());
                    false
                }
            }
        })
        .ok()
        .map(|thread| thread.join().unwrap_or(false));
    emptied.unwrap_or(false)
}

#[cfg(not(windows))]
pub fn empty_bin() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_lists_its_newest_entries_and_its_subfolders() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.txt"), b"x").unwrap();
        std::fs::create_dir(dir.path().join("Fotos")).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(dir.path().join("old.txt"))
            .unwrap()
            .set_modified(old)
            .unwrap();
        std::fs::write(dir.path().join("new.pdf"), b"x").unwrap();

        let (entries, more) = newest_in(dir.path()).unwrap();

        assert!(!more);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries.last().unwrap().name, "old.txt");
        assert!(
            entries
                .iter()
                .any(|entry| entry.name == "Fotos" && entry.folder)
        );
    }

    #[test]
    fn a_folder_that_is_gone_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(newest_in(&dir.path().join("gone")).is_err());
    }
}

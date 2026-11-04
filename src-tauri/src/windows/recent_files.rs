//! The Arquivos (F9 of estudo 0002), as Windows keeps them: the documents
//! opened last, from the links of the Recent folder (`shell:recent`), the
//! newest first.
//!
//! Read only when the panel asks, off the dock's thread. Each link is read,
//! never resolved, and a target on a network share is shown without looking
//! at it: an offline share would hang the listing.

use std::path::{Path, PathBuf};

use crate::domain::folders::{self, FolderEntry, MAX_LOOKED_AT};
use crate::domain::widgets::files::recent;

/// The user's Recent folder.
#[cfg(windows)]
pub fn folder() -> Option<PathBuf> {
    use ::windows::Win32::System::Com::CoTaskMemFree;
    use ::windows::Win32::UI::Shell::{FOLDERID_Recent, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
    let text = unsafe { SHGetKnownFolderPath(&FOLDERID_Recent, KF_FLAG_DEFAULT, None) }.ok()?;
    let path = unsafe { text.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(text.0.cast_const().cast())) };
    path.map(PathBuf::from)
}

#[cfg(not(windows))]
pub fn folder() -> Option<PathBuf> {
    None
}

/// How many of the newest links are read: many point at folders or at files
/// gone since, and each read takes the shell's time.
const LINKS_READ: usize = 80;

/// The files opened last, the newest first, each once: the targets of the
/// newest links of `folder` that are still files.
pub fn newest(folder: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut links = Vec::new();
    for entry in std::fs::read_dir(folder)?.take(MAX_LOOKED_AT) {
        let Ok(entry) = entry else { continue };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".lnk") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_millis() as u64);
        links.push(FolderEntry {
            name,
            modified,
            folder: false,
        });
    }
    let targets = folders::newest(links, LINKS_READ)
        .into_iter()
        .filter_map(|link| target_of(&folder.join(&link.name)))
        .collect();
    Ok(recent(targets, still_a_file)
        .into_iter()
        .map(PathBuf::from)
        .collect())
}

#[cfg(windows)]
fn target_of(link: &Path) -> Option<String> {
    super::jumplist::link_target(link)
}

#[cfg(not(windows))]
fn target_of(_link: &Path) -> Option<String> {
    None
}

/// Whether `target` is still a file: a share (`\\server\…`) is trusted
/// without looking, so an offline one never hangs the panel.
fn still_a_file(target: &str) -> bool {
    target.starts_with(r"\\") || Path::new(target).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_share_is_trusted_and_a_missing_file_is_not() {
        assert!(still_a_file(r"\\servidor\pasta\relatorio.pdf"));
        assert!(!still_a_file(r"C:\nao\existe\nada.txt"));
    }
}

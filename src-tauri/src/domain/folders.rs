//! Folders on the dock (spec 0025): a folder pinned like a program, the
//! Downloads among them, and the Recycle Bin. Pure rules: which pinned item
//! is which, the newest entries a folder's panel lists and the kind of each.
//! The disk and the shell are read in `windows/folders.rs`.

use serde::Serialize;

use super::apps::is_valid_app_id;

/// The id of the Recycle Bin pinned to the dock: the shell's parsing name
/// of the folder, which no program's id can be.
pub const RECYCLE_BIN_ID: &str = "::{645FF040-5081-101B-9F08-00AA002F954E}";

/// The most entries a folder's panel lists: the newest.
pub const MAX_FOLDER_ENTRIES: usize = 24;

/// The most entries of a folder looked at to find the newest: a folder with
/// more lists the newest of these, and the panel says so.
pub const MAX_LOOKED_AT: usize = 5000;

/// What a pinned item opens: a program (the rest of the dock), a folder or
/// the Recycle Bin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Place {
    Program,
    Folder,
    RecycleBin,
}

/// Whether `path` can be a folder pinned to the dock: an absolute path on a
/// drive (`C:\...`, no UNC, no relative part). Looks at the text only;
/// whether the folder exists is checked where it is read.
pub fn is_pinnable_folder(path: &str) -> bool {
    let bytes = path.as_bytes();
    let drive =
        bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\';
    drive
        && is_valid_app_id(path)
        && !path.contains("\\..\\")
        && !path.ends_with("\\..")
        && !path.contains('/')
}

/// What the pinned item of id `app_id` is, from its text: the Recycle Bin's
/// id, a path on a drive that is not a program nor a shortcut (a folder), or
/// a program.
pub fn place_of(app_id: &str) -> Place {
    if app_id.eq_ignore_ascii_case(RECYCLE_BIN_ID) {
        Place::RecycleBin
    } else if is_pinnable_folder(app_id) && !super::apps::is_pinnable_path(app_id) {
        Place::Folder
    } else {
        Place::Program
    }
}

/// What an entry of a folder's panel is, for its glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    Folder,
    Image,
    Video,
    Audio,
    Document,
    Archive,
    Program,
    Other,
}

/// The kind of the file named `name`, by its extension.
pub fn kind_of(name: &str, folder: bool) -> EntryKind {
    if folder {
        return EntryKind::Folder;
    }
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" | "heic" | "tif"
        | "tiff" => EntryKind::Image,
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" => EntryKind::Video,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" => EntryKind::Audio,
        "pdf" | "doc" | "docx" | "odt" | "rtf" | "txt" | "md" | "xls" | "xlsx" | "ods" | "csv"
        | "ppt" | "pptx" | "odp" => EntryKind::Document,
        "zip" | "rar" | "7z" | "tar" | "gz" | "xz" | "bz2" | "iso" => EntryKind::Archive,
        "exe" | "msi" | "msix" | "appx" | "bat" | "cmd" | "lnk" => EntryKind::Program,
        _ => EntryKind::Other,
    }
}

/// One entry of a folder, as read from the disk: its name, when it last
/// changed (in milliseconds since 1970) and whether it is a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEntry {
    pub name: String,
    pub modified: u64,
    pub folder: bool,
}

/// The newest `limit` of `entries`, the newest first; the same time goes by
/// name.
pub fn newest(mut entries: Vec<FolderEntry>, limit: usize) -> Vec<FolderEntry> {
    entries.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries.truncate(limit);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, modified: u64) -> FolderEntry {
        FolderEntry {
            name: name.to_owned(),
            modified,
            folder: false,
        }
    }

    #[test]
    fn a_folder_on_a_drive_can_be_pinned() {
        assert!(is_pinnable_folder(r"C:\Users\Me\Downloads"));
        assert!(is_pinnable_folder(r"D:\"));
        assert!(!is_pinnable_folder(r"\\server\share"));
        assert!(!is_pinnable_folder(r"Users\Me"));
        assert!(!is_pinnable_folder(r"C:\Users\..\Windows"));
        assert!(!is_pinnable_folder(r"C:\Users\.."));
        assert!(!is_pinnable_folder("C:/Users"));
        assert!(!is_pinnable_folder("C:\\Users\\Me\n"));
        assert!(!is_pinnable_folder(""));
    }

    #[test]
    fn the_id_says_what_a_pinned_item_is() {
        assert_eq!(place_of(RECYCLE_BIN_ID), Place::RecycleBin);
        assert_eq!(place_of(&RECYCLE_BIN_ID.to_lowercase()), Place::RecycleBin);
        assert_eq!(place_of(r"C:\Users\Me\Downloads"), Place::Folder);
        assert_eq!(place_of(r"C:\Tools\Tool.exe"), Place::Program);
        assert_eq!(place_of(r"C:\Users\Me\Desktop\Tool.lnk"), Place::Program);
        assert_eq!(
            place_of("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            Place::Program
        );
        assert_eq!(
            place_of(r"{6D809377-6AF0-444B-8957-A3773F02200E}\Microsoft VS Code\Code.exe"),
            Place::Program
        );
    }

    #[test]
    fn the_extension_gives_the_kind() {
        assert_eq!(kind_of("Fotos", true), EntryKind::Folder);
        assert_eq!(kind_of("foto.JPG", false), EntryKind::Image);
        assert_eq!(kind_of("aula.mp4", false), EntryKind::Video);
        assert_eq!(kind_of("nota.pdf", false), EntryKind::Document);
        assert_eq!(kind_of("pacote.7z", false), EntryKind::Archive);
        assert_eq!(kind_of("Setup.exe", false), EntryKind::Program);
        assert_eq!(kind_of("README", false), EntryKind::Other);
        assert_eq!(kind_of("arquivo.desconhecido", false), EntryKind::Other);
    }

    #[test]
    fn the_newest_come_first_and_only_as_many_as_asked() {
        let entries = vec![
            entry("a.txt", 10),
            entry("b.txt", 30),
            entry("C.txt", 20),
            entry("c.txt", 30),
        ];

        let names: Vec<_> = newest(entries, 3)
            .into_iter()
            .map(|entry| entry.name)
            .collect();

        assert_eq!(names, ["b.txt", "c.txt", "C.txt"]);
    }
}

//! A copy of the dock to take elsewhere (spec 0026): one `.dockyard` file
//! with the settings, the database (workspaces, programs, widgets, notes)
//! and the images chosen as icons (spec 0024). No secret goes in: the
//! accounts' tokens stay in the Credential Manager.
//!
//! Importing never touches the data in use. The file is checked (format,
//! size, a database this Dockyard can open, settings it can read), then
//! staged in `Importacao-pendente`; the next start (`apply_pending`), before
//! anything reads the data, moves what was there to
//! `Antes-de-importar-<UTC time>` and puts the staged files in its place.
//! An older database is then migrated as at any start.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::database::{FILE_NAME as DB_FILE, MIGRATIONS, Storage};
use super::settings_file::{self, FILE_NAME as SETTINGS_FILE};
use crate::domain::app_icon::is_image_file;
use crate::domain::widgets::StorageStatus;

/// The version of the file's layout.
pub const FORMAT: u32 = 1;

/// The extension of the file.
pub const EXTENSION: &str = "dockyard";

/// The biggest file read: far above a dock's data.
pub const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// The biggest image chosen as an icon taken along (the chosen ones are
/// kept as PNGs of at most this).
const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;

/// The folder the import waits in, in the data folder.
pub const PENDING_DIR: &str = "Importacao-pendente";

/// The start of the folder what was there before an import goes to.
pub const BEFORE_PREFIX: &str = "Antes-de-importar-";

/// The folder of the images chosen as icons (`persistence/app_icons.rs`).
const ICONS_DIR: &str = "AppIcons";

/// The file, as written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bundle {
    format: u32,
    /// The Dockyard that wrote it.
    app: String,
    /// When, as an RFC 3339 time.
    exported_at: String,
    /// The database's schema version (`user_version`).
    schema: usize,
    settings: serde_json::Value,
    /// The database, a SQLite file, in base64.
    database: String,
    /// The images chosen as icons, by file name, in base64.
    #[serde(default)]
    app_icons: BTreeMap<String, String>,
}

/// What a file holds, for the page to show before importing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub app: String,
    pub exported_at: String,
    pub workspaces: usize,
    pub apps: usize,
    pub widgets: usize,
    pub icons: usize,
}

/// Why a file can't be exported or imported. Names only the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferError {
    #[error("the file is too big")]
    TooBig,
    #[error("the file is not a Dockyard copy")]
    NotACopy,
    #[error("the copy is from a newer Dockyard")]
    Newer,
    #[error("the copy's data is damaged")]
    Damaged,
    #[error("the data could not be read or written")]
    Failed,
}

fn failed<E>(_: E) -> TransferError {
    TransferError::Failed
}

/// A folder of its own in `parent`, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(parent: &Path, name: &str) -> Result<Scratch, TransferError> {
        let dir = parent.join(format!("{name}-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).map_err(failed)?;
        Ok(Scratch(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The file of the dock: `conn` the database in use, `settings` the
/// settings in use, `data_dir` the data folder (for the icons and a
/// scratch copy), `app` this Dockyard's version, `now` an RFC 3339 time.
pub fn export(
    conn: &Connection,
    settings: serde_json::Value,
    data_dir: &Path,
    app: &str,
    now: &str,
) -> Result<Vec<u8>, TransferError> {
    let scratch = Scratch::new(data_dir, "exportando")?;
    let copy = scratch.0.join(DB_FILE);
    // A consistent copy, without the -wal: what a reader would see now.
    conn.execute("VACUUM INTO ?1", [copy.to_string_lossy().as_ref()])
        .map_err(failed)?;
    let database = fs::read(&copy).map_err(failed)?;
    let schema: usize = conn
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(failed)?
        .try_into()
        .map_err(failed)?;
    let mut app_icons = BTreeMap::new();
    if let Ok(entries) = fs::read_dir(data_dir.join(ICONS_DIR)) {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !is_image_file(&name) {
                continue;
            }
            if let Ok(bytes) = fs::read(entry.path())
                && bytes.len() <= MAX_ICON_BYTES
            {
                app_icons.insert(name, STANDARD.encode(bytes));
            }
        }
    }
    let bundle = Bundle {
        format: FORMAT,
        app: app.to_owned(),
        exported_at: now.to_owned(),
        schema,
        settings,
        database: STANDARD.encode(database),
        app_icons,
    };
    serde_json::to_vec_pretty(&bundle).map_err(failed)
}

/// A file checked: what it holds and its parts, ready to stage.
pub struct Checked {
    pub summary: Summary,
    settings: Vec<u8>,
    database: Vec<u8>,
    icons: Vec<(String, Vec<u8>)>,
}

/// Reads and checks a file: its layout, a database this Dockyard opens
/// (migrating an older one, in a scratch folder in `data_dir`) and settings
/// it reads without falling back to the defaults. Nothing of the data in
/// use is touched.
pub fn check(bytes: &[u8], data_dir: &Path) -> Result<Checked, TransferError> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(TransferError::TooBig);
    }
    let bundle: Bundle = serde_json::from_slice(bytes).map_err(|_| TransferError::NotACopy)?;
    if bundle.format != FORMAT {
        return Err(if bundle.format > FORMAT {
            TransferError::Newer
        } else {
            TransferError::NotACopy
        });
    }
    if bundle.schema > MIGRATIONS.len() {
        return Err(TransferError::Newer);
    }
    let database = STANDARD
        .decode(&bundle.database)
        .map_err(|_| TransferError::Damaged)?;
    let settings = serde_json::to_vec_pretty(&bundle.settings).map_err(failed)?;
    let mut icons = Vec::new();
    for (name, data) in &bundle.app_icons {
        let bytes = STANDARD.decode(data).map_err(|_| TransferError::Damaged)?;
        if !is_image_file(name) || bytes.len() > MAX_ICON_BYTES || !bytes.starts_with(b"\x89PNG") {
            return Err(TransferError::Damaged);
        }
        icons.push((name.clone(), bytes));
    }

    let scratch = Scratch::new(data_dir, "conferindo")?;
    fs::write(scratch.0.join(DB_FILE), &database).map_err(failed)?;
    let settings_path = scratch.0.join(SETTINGS_FILE);
    fs::write(&settings_path, &settings).map_err(failed)?;
    if settings_file::load(&settings_path).issue.is_some() {
        return Err(TransferError::Damaged);
    }
    let summary = {
        let mut storage = Storage::open(&scratch.0);
        let widgets = storage.snapshot().map_err(|_| TransferError::Damaged)?;
        if widgets.storage != StorageStatus::Ok {
            return Err(TransferError::Damaged);
        }
        let apps = storage
            .apps_snapshot()
            .map_err(|_| TransferError::Damaged)?;
        let workspaces = storage.workspaces().map_err(|_| TransferError::Damaged)?;
        // `storage` closes here, before the scratch folder goes.
        Summary {
            app: bundle.app.clone(),
            exported_at: bundle.exported_at.clone(),
            workspaces: workspaces.len(),
            apps: apps.apps.len(),
            widgets: widgets.widgets.len(),
            icons: icons.len(),
        }
    };
    Ok(Checked {
        summary,
        settings,
        database,
        icons,
    })
}

/// Puts a checked file in `data_dir/Importacao-pendente`, whole or not at
/// all (written aside, then renamed). An earlier pending import is
/// replaced.
pub fn stage(checked: &Checked, data_dir: &Path) -> Result<(), TransferError> {
    let pending = data_dir.join(PENDING_DIR);
    let scratch = Scratch::new(data_dir, "preparando")?;
    fs::write(scratch.0.join(DB_FILE), &checked.database).map_err(failed)?;
    fs::write(scratch.0.join(SETTINGS_FILE), &checked.settings).map_err(failed)?;
    let icons = scratch.0.join(ICONS_DIR);
    fs::create_dir_all(&icons).map_err(failed)?;
    for (name, bytes) in &checked.icons {
        fs::write(icons.join(name), bytes).map_err(failed)?;
    }
    if pending.exists() {
        fs::remove_dir_all(&pending).map_err(failed)?;
    }
    fs::rename(&scratch.0, &pending).map_err(failed)?;
    Ok(())
}

/// What the data folder holds that an import replaces: the database with
/// its companions, the settings and the icons.
fn replaced() -> [&'static str; 5] {
    [
        DB_FILE,
        "dockmax.db-wal",
        "dockmax.db-shm",
        SETTINGS_FILE,
        ICONS_DIR,
    ]
}

/// Renames `from` to `to`, trying again for up to 3 s: the Dockyard that
/// asked for the import may still be letting go of its files.
fn rename_soon(from: &Path, to: &Path) -> bool {
    for _ in 0..10 {
        if fs::rename(from, to).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    false
}

/// At start, before the settings and the database are read: a pending
/// import takes the place of the data in `data_dir`, what was there kept in
/// `Antes-de-importar-<stamp>`. `Some(true)` when one was applied,
/// `Some(false)` when it failed (what was moved is put back), `None` with
/// nothing pending.
pub fn apply_pending(data_dir: &Path, stamp: &str) -> Option<bool> {
    let pending = data_dir.join(PENDING_DIR);
    if !pending.is_dir() {
        return None;
    }
    let before = data_dir.join(format!("{BEFORE_PREFIX}{stamp}"));
    if fs::create_dir_all(&before).is_err() {
        return Some(false);
    }
    let mut moved_out: Vec<&str> = Vec::new();
    let mut moved_in: Vec<&str> = Vec::new();
    let mut ok = true;
    for name in replaced() {
        let current = data_dir.join(name);
        if current.exists() {
            if !rename_soon(&current, &before.join(name)) {
                ok = false;
                break;
            }
            moved_out.push(name);
        }
    }
    if ok {
        for name in replaced() {
            let staged = pending.join(name);
            if staged.exists() {
                if fs::rename(&staged, data_dir.join(name)).is_err() {
                    ok = false;
                    break;
                }
                moved_in.push(name);
            }
        }
    }
    if !ok {
        // Back as it was: the staged files to the pending folder, the old
        // ones to their place.
        for name in moved_in {
            let _ = fs::rename(data_dir.join(name), pending.join(name));
        }
        for name in moved_out {
            let _ = fs::rename(before.join(name), data_dir.join(name));
        }
        let _ = fs::remove_dir(&before);
        return Some(false);
    }
    let _ = fs::remove_dir_all(&pending);
    Some(true)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::domain::settings::Settings;

    fn settings_json() -> serde_json::Value {
        serde_json::to_value(Settings::default()).unwrap()
    }

    /// A data folder with a database (seeded: three widgets) and settings.
    fn data() -> (tempfile::TempDir, Storage) {
        let dir = tempdir().unwrap();
        let storage = Storage::open(dir.path());
        settings_file::save(&dir.path().join(SETTINGS_FILE), &Settings::default()).unwrap();
        (dir, storage)
    }

    fn exported(storage: &mut Storage, dir: &Path) -> Vec<u8> {
        export(
            storage.connection().unwrap(),
            settings_json(),
            dir,
            "0.9.2",
            "2026-11-04T18:00:00Z",
        )
        .unwrap()
    }

    #[test]
    fn a_copy_goes_and_comes_back_the_same() {
        let (dir, mut storage) = data();
        let icons = dir.path().join(ICONS_DIR);
        fs::create_dir_all(&icons).unwrap();
        let icon = "01a1a796-983a-77be-9580-03fd3f034c24.png";
        fs::write(icons.join(icon), b"\x89PNG rest").unwrap();
        let bytes = exported(&mut storage, dir.path());

        let elsewhere = tempdir().unwrap();
        let checked = check(&bytes, elsewhere.path()).unwrap();
        assert_eq!(checked.summary.widgets, 3);
        assert_eq!(checked.summary.icons, 1);
        assert_eq!(checked.summary.app, "0.9.2");
        stage(&checked, elsewhere.path()).unwrap();
        assert_eq!(apply_pending(elsewhere.path(), "x"), Some(true));

        let again = Storage::open(elsewhere.path());
        let before: Vec<_> = storage.snapshot().unwrap().widgets;
        let after: Vec<_> = again.snapshot().unwrap().widgets;
        assert_eq!(before, after);
        assert_eq!(
            fs::read(elsewhere.path().join(ICONS_DIR).join(icon)).unwrap(),
            b"\x89PNG rest"
        );
        assert!(
            settings_file::load(&elsewhere.path().join(SETTINGS_FILE))
                .issue
                .is_none()
        );
        drop(again);
        // Nothing left behind but the copy of what was there.
        let left: Vec<_> = fs::read_dir(elsewhere.path())
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| {
                name.starts_with("conferindo")
                    || name.starts_with("preparando")
                    || name == PENDING_DIR
            })
            .collect();
        assert!(left.is_empty(), "{left:?}");
    }

    #[test]
    fn what_was_there_is_kept_aside() {
        let (dir, mut storage) = data();
        let bytes = exported(&mut storage, dir.path());
        drop(storage);
        fs::write(dir.path().join(SETTINGS_FILE), b"{\"old\": true}").unwrap();

        let checked = check(&bytes, dir.path()).unwrap();
        stage(&checked, dir.path()).unwrap();
        assert_eq!(apply_pending(dir.path(), "20261104T180000Z"), Some(true));

        let before = dir.path().join(format!("{BEFORE_PREFIX}20261104T180000Z"));
        assert_eq!(
            fs::read(before.join(SETTINGS_FILE)).unwrap(),
            b"{\"old\": true}"
        );
        assert!(before.join(DB_FILE).exists());
        assert_eq!(apply_pending(dir.path(), "again"), None);
    }

    #[test]
    fn a_damaged_or_foreign_file_changes_nothing() {
        let (dir, mut storage) = data();
        let bytes = exported(&mut storage, dir.path());
        let mut bundle: Bundle = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(
            check(b"not json", dir.path()).err(),
            Some(TransferError::NotACopy)
        );

        let mut newer = bundle.clone();
        newer.schema = MIGRATIONS.len() + 1;
        let newer = serde_json::to_vec(&newer).unwrap();
        assert_eq!(check(&newer, dir.path()).err(), Some(TransferError::Newer));

        let mut format = bundle.clone();
        format.format = FORMAT + 1;
        let format = serde_json::to_vec(&format).unwrap();
        assert_eq!(check(&format, dir.path()).err(), Some(TransferError::Newer));

        bundle.database = STANDARD.encode(b"not a database at all, just text");
        let damaged = serde_json::to_vec(&bundle).unwrap();
        assert_eq!(
            check(&damaged, dir.path()).err(),
            Some(TransferError::Damaged)
        );

        assert!(!dir.path().join(PENDING_DIR).exists());
        assert_eq!(apply_pending(dir.path(), "x"), None);
    }

    #[test]
    fn a_copy_of_an_older_schema_is_taken_and_migrated() {
        let dir = tempdir().unwrap();
        // A database at schema 5, before the stacks.
        let old = dir.path().join("old.db");
        {
            let conn = Connection::open(&old).unwrap();
            for sql in &MIGRATIONS[..5] {
                conn.execute_batch(sql).unwrap();
            }
            conn.pragma_update(None, "user_version", 5).unwrap();
        }
        let bundle = Bundle {
            format: FORMAT,
            app: "0.9.0".into(),
            exported_at: "2026-10-01T00:00:00Z".into(),
            schema: 5,
            settings: settings_json(),
            database: STANDARD.encode(fs::read(&old).unwrap()),
            app_icons: BTreeMap::new(),
        };
        let bytes = serde_json::to_vec(&bundle).unwrap();

        let checked = check(&bytes, dir.path()).unwrap();

        assert_eq!(checked.summary.app, "0.9.0");
    }

    #[test]
    fn an_icon_that_is_not_a_chosen_png_is_refused() {
        let (dir, mut storage) = data();
        let bytes = exported(&mut storage, dir.path());
        let mut bundle: Bundle = serde_json::from_slice(&bytes).unwrap();
        bundle
            .app_icons
            .insert("../dockmax.db".into(), STANDARD.encode(b"\x89PNG"));
        let bytes = serde_json::to_vec(&bundle).unwrap();

        assert_eq!(
            check(&bytes, dir.path()).err(),
            Some(TransferError::Damaged)
        );
    }
}

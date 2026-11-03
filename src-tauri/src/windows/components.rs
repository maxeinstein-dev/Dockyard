//! The Edge components WebView2 downloaded into the app's profile before
//! `--disable-component-update` (in `BROWSER_ARGS`) stopped it: video DRM,
//! ad filter lists, speech recognition, hyphenation and their downloads.
//! The app uses none of them, and they took about 42 MB of the user's
//! disk. They are deleted at startup, before any WebView2 runs; once gone,
//! nothing brings them back, and each later startup only finds them
//! missing.

use std::fs;
use std::io;
use std::path::Path;

use tauri::{AppHandle, Manager, Runtime};

/// The folder WebView2 keeps its profile in, in the app's local data
/// folder (Tauri's default).
const PROFILE: &str = "EBWebView";

/// What the component updater left in the profile, by folder.
pub const COMPONENTS: [&str; 6] = [
    "WidevineCdm",
    "Subresource Filter",
    "component_crx_cache",
    "Speech Recognition",
    "hyphen-data",
    "PKIMetadata",
];

/// Deletes the components of the app's WebView2 profile. A failure is
/// logged and leaves the folder for the next startup.
pub fn drop_downloaded<R: Runtime>(app: &AppHandle<R>) {
    let Ok(dir) = app.path().app_local_data_dir() else {
        return;
    };
    let freed = drop_from(&dir.join(PROFILE));
    if freed > 0 {
        log::info!(
            "components: deleted the Edge components of the WebView2 profile ({} MB)",
            freed / (1024 * 1024)
        );
    }
}

/// Deletes each folder of `COMPONENTS` in `profile`; the bytes it freed.
fn drop_from(profile: &Path) -> u64 {
    let mut freed = 0;
    for name in COMPONENTS {
        let folder = profile.join(name);
        if !folder.is_dir() {
            continue;
        }
        let size = size_of(&folder);
        match fs::remove_dir_all(&folder) {
            Ok(()) => freed += size,
            Err(error) => log::warn!(
                "components: a folder could not be deleted ({:?})",
                error.kind()
            ),
        }
    }
    freed
}

/// The bytes of the files under `folder`.
fn size_of(folder: &Path) -> u64 {
    fn walk(folder: &Path) -> io::Result<u64> {
        let mut total = 0;
        for entry in fs::read_dir(folder)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                total += walk(&entry.path())?;
            } else if kind.is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }
    walk(folder).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_components_go_and_the_rest_of_the_profile_stays() {
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path();
        for name in COMPONENTS {
            fs::create_dir_all(profile.join(name).join("1.0")).unwrap();
            fs::write(profile.join(name).join("1.0").join("file"), b"12345").unwrap();
        }
        // The caches make the pages open faster: they stay.
        fs::create_dir_all(profile.join("Default").join("Cache")).unwrap();
        fs::write(profile.join("Default").join("Cache").join("data"), b"x").unwrap();
        fs::write(profile.join("Local State"), b"{}").unwrap();

        assert_eq!(drop_from(profile), 5 * COMPONENTS.len() as u64);
        for name in COMPONENTS {
            assert!(!profile.join(name).exists(), "{name}");
        }
        assert!(profile.join("Default").join("Cache").join("data").exists());
        assert!(profile.join("Local State").exists());
        // The next startup finds nothing to do.
        assert_eq!(drop_from(profile), 0);
    }

    #[test]
    fn a_missing_profile_is_nothing_to_do() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(drop_from(&dir.path().join("EBWebView")), 0);
    }

    #[test]
    fn the_updater_that_downloads_them_stays_off() {
        // Deleting them is only worth it while nothing downloads them again.
        assert!(
            super::super::BROWSER_ARGS
                .split(' ')
                .any(|arg| arg == "--disable-component-update"),
            "{}",
            super::super::BROWSER_ARGS
        );
    }
}

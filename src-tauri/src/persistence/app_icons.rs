//! The images of the user's own chosen for programs (spec 0024), in the
//! `AppIcons` folder of the data: PNGs Dockyard names itself, read by the
//! dock and removed when no program wears them any more.

use std::path::{Path, PathBuf};

use crate::domain::app_icon::is_image_file;
use crate::domain::icon_pack::{MAX_IMAGE_SIDE, is_valid_image_size, png_size};
use crate::domain::widgets::WidgetId;

/// The biggest image kept, in bytes: a PNG of `MAX_IMAGE_SIDE` pixels of a
/// side, with room.
const MAX_KEPT_BYTES: u64 = (MAX_IMAGE_SIDE as u64) * (MAX_IMAGE_SIDE as u64) * 4 + 4096;

/// The folder, in the data folder `data_dir`.
pub fn root(data_dir: &Path) -> PathBuf {
    data_dir.join("AppIcons")
}

/// Keeps `png` under a new name, which it returns.
pub fn save(root: &Path, png: &[u8]) -> std::io::Result<String> {
    std::fs::create_dir_all(root)?;
    let name = format!("{}.png", WidgetId::new().as_str());
    std::fs::write(root.join(&name), png)?;
    Ok(name)
}

/// The image `file`, if it is one Dockyard kept and still a good PNG.
pub fn read(root: &Path, file: &str) -> Option<Vec<u8>> {
    if !is_image_file(file) {
        return None;
    }
    let path = root.join(file);
    if std::fs::metadata(&path).ok()?.len() > MAX_KEPT_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    png_size(&bytes)
        .filter(|size| is_valid_image_size(*size))
        .map(|_| bytes)
}

/// Removes the image `file`; one already gone is fine.
pub fn remove(root: &Path, file: &str) {
    if !is_image_file(file) {
        return;
    }
    match std::fs::remove_file(root.join(file)) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => log::warn!("icons: a chosen image was left behind: {}", error.kind()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn png(side: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, side, side);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[0, 0, 255, 255].repeat((side * side) as usize))
            .unwrap();
        drop(writer);
        bytes
    }

    #[test]
    fn an_image_is_kept_read_and_removed_by_its_own_name() {
        let dir = tempdir().unwrap();
        let root = root(dir.path());
        let image = png(48);

        let name = save(&root, &image).unwrap();

        assert!(is_image_file(&name), "{name}");
        assert_eq!(read(&root, &name), Some(image));
        // Never a path out of the folder.
        assert_eq!(read(&root, "../dockmax.db"), None);
        remove(&root, &name);
        assert_eq!(read(&root, &name), None);
        remove(&root, &name);
    }
}

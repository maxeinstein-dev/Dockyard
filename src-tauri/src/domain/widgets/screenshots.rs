//! The Capturas de tela (F9 of estudo 0002): an item of the dock whose
//! panel shows the newest screenshots of Windows' Screenshots folder, to
//! open or drag out. It has nothing to keep: the folder is Windows' own, so
//! the config is empty and stays `{}`.

use serde::{Deserialize, Serialize};

use super::WidgetError;

/// How many screenshots the panel shows.
pub const MAX_SHOWN: usize = 12;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenshotsConfig {}

impl ScreenshotsConfig {
    /// Always valid: there is nothing in it.
    pub fn validate(&self) -> Result<(), WidgetError> {
        Ok(())
    }
}

/// Whether the file `name` is an image a screenshot is saved as.
pub fn is_image(name: &str) -> bool {
    let Some((_, extension)) = name.rsplit_once('.') else {
        return false;
    };
    ["png", "jpg", "jpeg", "gif", "bmp", "webp"]
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_is_empty_and_reads_from_anything_older() {
        assert_eq!(
            serde_json::to_string(&ScreenshotsConfig::default()).unwrap(),
            "{}"
        );
        let read: ScreenshotsConfig = serde_json::from_str(r#"{"width": 3}"#).unwrap();
        assert_eq!(read, ScreenshotsConfig::default());
        assert_eq!(read.validate(), Ok(()));
    }

    #[test]
    fn only_images_are_screenshots() {
        for name in [
            "Captura de tela 2026-11-04.png",
            "a.JPG",
            "b.jpeg",
            "c.webp",
        ] {
            assert!(is_image(name), "{name}");
        }
        for name in ["notas.txt", "png", "vídeo.mp4", "desktop.ini"] {
            assert!(!is_image(name), "{name}");
        }
    }
}

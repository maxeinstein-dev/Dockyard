//! The skins of the bar (spec 0020 §3): a style whose bar is an image, cut
//! in nine slices. The top and bottom slices are empty: the whole image goes
//! to the bar's height, so only its width changes. The left and right ends
//! keep their drawing; the middle repeats (or stretches) to the bar's width.
//!
//! A skin is `skin.json` and `skin.png` in `src-tauri/skins/<id>/`, made by
//! `src-tauri/skins/fonte/gerar.py` from a drawing in text. They are in the
//! binary; the miniatures of Configurações read the same files.
//!
//! The user's own skins live in the `Skins` folder of the app's data folder
//! (spec 0020, Q14): one subfolder per skin, with the same two files. The
//! folder is the user's and may hold what a download put there, so a skin is
//! taken only after the checks of [`read_folder`]: its name, the size of its
//! files, the PNG header against `skin.json`, and the slices. Nothing here
//! logs a path or a name.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::icon_pack::png_size;
use super::settings::DockStyle;

/// What `skin.json` says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SkinMeta {
    pub name: String,
    pub image: String,
    pub width: u32,
    pub height: u32,
    pub slice: Slice,
    pub middle: Middle,
    /// Scaled up without smoothing (pixel art).
    pub pixelated: bool,
    /// Whether the bar is dark, so an icon pack's dark images go on it;
    /// `None` is dark. Only a folder skin says it: a bundled one's style
    /// says it in [`DockStyle::bar_dark`].
    #[serde(default)]
    pub dark: Option<bool>,
    /// `#rrggbb`: the clock's ink, and the mark under an open item, on a
    /// folder skin; `None` (or anything else) for the dock's own. A bundled
    /// skin's colors are its style's, in `look.rs`.
    #[serde(default)]
    pub ink: Option<String>,
    #[serde(default)]
    pub indicator: Option<String>,
}

/// The nine-slice cut, in pixels of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Slice {
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub left: u32,
}

/// How the middle fills the bar's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Middle {
    Repeat,
    Stretch,
}

/// A skin in the binary.
pub struct Skin {
    pub id: &'static str,
    pub png: &'static [u8],
    json: &'static str,
}

macro_rules! skin {
    ($id:literal) => {
        Skin {
            id: $id,
            png: include_bytes!(concat!("../../skins/", $id, "/skin.png")),
            json: include_str!(concat!("../../skins/", $id, "/skin.json")),
        }
    };
}

/// Every skin in the binary.
pub static SKINS: [Skin; 9] = [
    skin!("classic98"),
    skin!("phosphor"),
    skin!("retro"),
    skin!("tactical"),
    skin!("medieval"),
    skin!("hunter"),
    skin!("cyberpunk"),
    skin!("arcade"),
    skin!("scifi"),
];

impl Skin {
    /// Its `skin.json`, read once.
    pub fn meta(&self) -> &'static SkinMeta {
        static METAS: OnceLock<Vec<SkinMeta>> = OnceLock::new();
        let metas = METAS.get_or_init(|| {
            SKINS
                .iter()
                .map(|skin| serde_json::from_str(skin.json).expect("a bundled skin.json is valid"))
                .collect()
        });
        let index = SKINS
            .iter()
            .position(|skin| skin.id == self.id)
            .expect("a bundled skin");
        &metas[index]
    }
}

/// The skin of `style`'s bar; `None` for a style painted with colors.
pub fn skin_of(style: DockStyle) -> Option<&'static Skin> {
    let id = match style {
        DockStyle::Classic98 => "classic98",
        DockStyle::Phosphor => "phosphor",
        DockStyle::Retro => "retro",
        DockStyle::Tactical => "tactical",
        DockStyle::Medieval => "medieval",
        DockStyle::Hunter => "hunter",
        DockStyle::Cyberpunk => "cyberpunk",
        DockStyle::Arcade => "arcade",
        DockStyle::Scifi => "scifi",
        _ => return None,
    };
    SKINS.iter().find(|skin| skin.id == id)
}

/// A rectangle: `x`, `y`, width and height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A part of the image (`from`, in its pixels) drawn at `to`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub from: Rect,
    pub to: Rect,
}

/// The pieces that draw `meta`'s image over `bar`: the left end, the middle
/// (in tiles when it repeats, the last one cut) and the right end, all at
/// the bar's height. A bar narrower than the two ends shrinks them.
pub fn pieces(meta: &SkinMeta, bar: Rect) -> Vec<Piece> {
    if meta.height == 0 || bar.width <= 0.0 || bar.height <= 0.0 {
        return Vec::new();
    }
    let height = f64::from(meta.height);
    let k = bar.height / height;
    let (left, right) = (f64::from(meta.slice.left), f64::from(meta.slice.right));
    let middle = f64::from(meta.width) - left - right;
    let ends = (left + right) * k;
    let squeeze = if ends > bar.width {
        bar.width / ends
    } else {
        1.0
    };
    let (left_to, right_to) = (left * k * squeeze, right * k * squeeze);
    let at = |x: f64, width: f64| Rect {
        x,
        y: bar.y,
        width,
        height: bar.height,
    };
    let from = |x: f64, width: f64| Rect {
        x,
        y: 0.0,
        width,
        height,
    };
    let mut pieces = vec![Piece {
        from: from(0.0, left),
        to: at(bar.x, left_to),
    }];
    let span = bar.width - left_to - right_to;
    if span > 0.0 && middle > 0.0 {
        match meta.middle {
            Middle::Stretch => pieces.push(Piece {
                from: from(left, middle),
                to: at(bar.x + left_to, span),
            }),
            Middle::Repeat => {
                let tile = middle * k;
                let mut x = 0.0;
                while x < span - 1e-6 {
                    let width = tile.min(span - x);
                    pieces.push(Piece {
                        from: from(left, width / k),
                        to: at(bar.x + left_to + x, width),
                    });
                    x += tile;
                }
            }
        }
    }
    pieces.push(Piece {
        from: from(f64::from(meta.width) - right, right),
        to: at(bar.x + bar.width - right_to, right_to),
    });
    pieces
}

/// The folder of the user's skins, under the app's data folder.
pub const FOLDER: &str = "Skins";

/// The most skin folders looked at; the rest are left out.
const MAX_SKINS: usize = 100;

/// The longest name of a skin folder, in characters.
const MAX_ID_CHARS: usize = 40;

/// The biggest `skin.png` taken: 2 MiB.
pub const MAX_PNG_BYTES: u64 = 2 * 1024 * 1024;

/// The biggest `skin.json` read; a real one is a few hundred bytes.
const MAX_JSON_BYTES: u64 = 64 * 1024;

/// The widest and tallest image taken, in pixels.
const MAX_SIDE: u32 = 4096;

/// The skins folder inside `data_dir`.
pub fn root(data_dir: &Path) -> PathBuf {
    data_dir.join(FOLDER)
}

/// A skin of the folder that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderSkin {
    /// The name of its folder: what `dock.customSkin` holds.
    pub id: String,
    pub meta: SkinMeta,
    pub png: Vec<u8>,
}

/// Why a folder of the skins folder is not a skin. Shown to the user, by the
/// folder's name, in Configurações; carries no path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemKind {
    /// The folder's name is not 1–40 letters, digits, spaces, `_`, `.` or
    /// `-`, or it starts with a dot.
    BadName,
    NoJson,
    /// `skin.json` is too big, or is not what the format says.
    BadJson,
    /// `image` is not exactly `skin.png`.
    BadImageName,
    NoImage,
    /// `skin.png` is over [`MAX_PNG_BYTES`].
    TooBig,
    NotPng,
    /// The PNG is not the size `skin.json` says.
    SizeMismatch,
    /// The size is out of 1–4096 px, or the ends are as wide as the image.
    BadSlice,
}

/// A folder refused, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinProblem {
    pub folder: String,
    pub kind: ProblemKind,
}

/// Whether `id` can name a skin folder (and so `dock.customSkin`): 1–40
/// letters, digits, spaces, `_`, `.` or `-`, not starting with a dot. No
/// separator, so it never leads out of the skins folder.
pub fn is_valid_skin_id(id: &str) -> bool {
    let count = id.chars().count();
    (1..=MAX_ID_CHARS).contains(&count)
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '.' | '-'))
}

/// Why a file could not be read whole.
enum Unread {
    Missing,
    TooBig,
}

/// Reads the file `path`, if it is no bigger than `most` bytes.
fn read_bounded(path: &Path, most: u64) -> Result<Vec<u8>, Unread> {
    let file = fs::File::open(path).map_err(|_| Unread::Missing)?;
    let meta = file.metadata().map_err(|_| Unread::Missing)?;
    if !meta.is_file() {
        return Err(Unread::Missing);
    }
    if meta.len() > most {
        return Err(Unread::TooBig);
    }
    let mut bytes = Vec::new();
    file.take(most + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Unread::Missing)?;
    // It may have grown since its length was read.
    if bytes.len() as u64 > most {
        return Err(Unread::TooBig);
    }
    Ok(bytes)
}

/// The real path of `name` inside `dir` (itself a real path), or `None` if
/// it does not exist or leads outside it (a link or a junction).
fn inside(dir: &Path, name: &str) -> Option<PathBuf> {
    let real = dir.join(name).canonicalize().ok()?;
    real.starts_with(dir).then_some(real)
}

/// The skin in the folder `dir`, named `id` (already checked), or why not.
fn check(id: &str, dir: &Path) -> Result<FolderSkin, ProblemKind> {
    let dir = dir.canonicalize().map_err(|_| ProblemKind::NoJson)?;
    let json = inside(&dir, "skin.json").ok_or(ProblemKind::NoJson)?;
    let bytes = read_bounded(&json, MAX_JSON_BYTES).map_err(|unread| match unread {
        Unread::Missing => ProblemKind::NoJson,
        Unread::TooBig => ProblemKind::BadJson,
    })?;
    let text = String::from_utf8(bytes).map_err(|_| ProblemKind::BadJson)?;
    // A BOM is what some editors put in front of a JSON file.
    let meta: SkinMeta = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|_| ProblemKind::BadJson)?;
    if meta.image != "skin.png" {
        return Err(ProblemKind::BadImageName);
    }
    let sides = 1..=MAX_SIDE;
    if !sides.contains(&meta.width)
        || !sides.contains(&meta.height)
        || meta.slice.left.saturating_add(meta.slice.right) >= meta.width
    {
        return Err(ProblemKind::BadSlice);
    }
    let image = inside(&dir, "skin.png").ok_or(ProblemKind::NoImage)?;
    let png = read_bounded(&image, MAX_PNG_BYTES).map_err(|unread| match unread {
        Unread::Missing => ProblemKind::NoImage,
        Unread::TooBig => ProblemKind::TooBig,
    })?;
    let size = png_size(&png).ok_or(ProblemKind::NotPng)?;
    if size != (meta.width, meta.height) {
        return Err(ProblemKind::SizeMismatch);
    }
    Ok(FolderSkin {
        id: id.to_owned(),
        meta,
        png,
    })
}

/// Every skin of the skins folder `dir`, and the folders refused, both by
/// name. Only its direct subfolders are looked at (a file there is left
/// out); a missing folder has no skins and no problems.
pub fn read_folder(dir: &Path) -> (Vec<FolderSkin>, Vec<SkinProblem>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return (Vec::new(), Vec::new());
    };
    let mut folders: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            )
        })
        .collect();
    folders.sort_by(|a, b| a.0.cmp(&b.0));
    folders.truncate(MAX_SKINS);
    let (mut skins, mut problems) = (Vec::new(), Vec::new());
    for (name, path) in folders {
        let found = if is_valid_skin_id(&name) {
            check(&name, &path)
        } else {
            Err(ProblemKind::BadName)
        };
        match found {
            Ok(skin) => skins.push(skin),
            Err(kind) => problems.push(SkinProblem { folder: name, kind }),
        }
    }
    (skins, problems)
}

/// The skin `id` of the skins folder `dir`, if it passes every check.
pub fn load(dir: &Path, id: &str) -> Option<FolderSkin> {
    if !is_valid_skin_id(id) {
        return None;
    }
    check(id, &dir.join(id)).ok()
}

/// A folder skin as the dock keeps it: `key` changes when either of its
/// files does, so the dock makes its bitmap again for an edited skin.
#[derive(Debug, PartialEq, Eq)]
pub struct CachedSkin {
    pub key: String,
    pub skin: FolderSkin,
}

/// The length and the time of change of `path`, as text.
fn stamp_of(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    Some(format!("{}-{modified}", meta.len()))
}

/// The skin last read, so the dock does not read the PNG again on every
/// refresh: only the two files' lengths and times are looked at.
static CACHE: Mutex<Option<Arc<CachedSkin>>> = Mutex::new(None);

/// Like [`load`], read again only when `skin.json` or `skin.png` changed.
pub fn cached(dir: &Path, id: &str) -> Option<Arc<CachedSkin>> {
    if !is_valid_skin_id(id) {
        return None;
    }
    let folder = dir.join(id);
    let key = format!(
        "folder/{id}/{}/{}",
        stamp_of(&folder.join("skin.json"))?,
        stamp_of(&folder.join("skin.png"))?
    );
    let kept = CACHE.lock().unwrap_or_else(PoisonError::into_inner).clone();
    if let Some(kept) = kept.filter(|kept| kept.key == key) {
        return Some(kept);
    }
    let fresh = Arc::new(CachedSkin {
        key,
        skin: load(dir, id)?,
    });
    *CACHE.lock().unwrap_or_else(PoisonError::into_inner) = Some(fresh.clone());
    Some(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(width: u32, left: u32, right: u32, middle: Middle) -> SkinMeta {
        SkinMeta {
            name: "Teste".into(),
            image: "skin.png".into(),
            width,
            height: 16,
            slice: Slice {
                top: 0,
                right,
                bottom: 0,
                left,
            },
            middle,
            pixelated: true,
            dark: None,
            ink: None,
            indicator: None,
        }
    }

    const BAR: Rect = Rect {
        x: 10.0,
        y: 4.0,
        width: 100.0,
        height: 64.0,
    };

    #[test]
    fn the_ends_keep_their_drawing_at_the_bar_s_height() {
        let pieces = pieces(&meta(5, 2, 2, Middle::Stretch), BAR);
        assert_eq!(pieces.len(), 3);
        // 64 / 16: every pixel of the image is 4 of the bar.
        assert_eq!(
            pieces[0].from,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 2.0,
                height: 16.0
            }
        );
        assert_eq!(
            pieces[0].to,
            Rect {
                x: 10.0,
                y: 4.0,
                width: 8.0,
                height: 64.0
            }
        );
        assert_eq!(
            pieces[2].from,
            Rect {
                x: 3.0,
                y: 0.0,
                width: 2.0,
                height: 16.0
            }
        );
        assert_eq!(
            pieces[2].to,
            Rect {
                x: 102.0,
                y: 4.0,
                width: 8.0,
                height: 64.0
            }
        );
        // The middle stretches between them.
        assert_eq!(
            pieces[1].to,
            Rect {
                x: 18.0,
                y: 4.0,
                width: 84.0,
                height: 64.0
            }
        );
    }

    #[test]
    fn a_repeating_middle_comes_in_tiles_the_last_one_cut() {
        // A 3 px middle is 12 px on the bar; 84 px of middle is 7 tiles.
        let tiled = pieces(&meta(7, 2, 2, Middle::Repeat), BAR);
        let middle = &tiled[1..tiled.len() - 1];
        assert_eq!(middle.len(), 7);
        assert!(
            middle
                .iter()
                .all(|piece| (piece.to.width - 12.0).abs() < 1e-9)
        );
        let wide = Rect { width: 90.0, ..BAR };
        let cut = pieces(&meta(7, 2, 2, Middle::Repeat), wide);
        let last = cut[cut.len() - 2];
        // 74 px of middle: six whole tiles and 2 px, 0.5 px of the image.
        assert!((last.to.width - 2.0).abs() < 1e-9);
        assert!((last.from.width - 0.5).abs() < 1e-9);
        let covered: f64 = cut.iter().map(|piece| piece.to.width).sum();
        assert!((covered - 90.0).abs() < 1e-9);
    }

    #[test]
    fn a_bar_narrower_than_the_ends_shrinks_them() {
        let narrow = Rect { width: 10.0, ..BAR };
        let pieces = pieces(&meta(5, 2, 2, Middle::Repeat), narrow);
        assert_eq!(pieces.len(), 2);
        assert!((pieces[0].to.width - 5.0).abs() < 1e-9);
        assert!((pieces[1].to.x - 15.0).abs() < 1e-9);
    }

    #[test]
    fn every_bundled_skin_is_what_its_png_and_its_styles_say() {
        for skin in &SKINS {
            let meta = skin.meta();
            let size = crate::domain::icon_pack::png_size(skin.png).expect("a PNG");
            assert_eq!(size, (meta.width, meta.height), "{}", skin.id);
            assert_eq!((meta.slice.top, meta.slice.bottom), (0, 0), "{}", skin.id);
            assert!(
                meta.slice.left + meta.slice.right < meta.width,
                "{}",
                skin.id
            );
            assert_eq!(meta.image, "skin.png", "{}", skin.id);
        }
        for (style, id) in [
            (DockStyle::Classic98, "classic98"),
            (DockStyle::Phosphor, "phosphor"),
            (DockStyle::Retro, "retro"),
            (DockStyle::Tactical, "tactical"),
            (DockStyle::Medieval, "medieval"),
            (DockStyle::Hunter, "hunter"),
            (DockStyle::Cyberpunk, "cyberpunk"),
            (DockStyle::Arcade, "arcade"),
            (DockStyle::Scifi, "scifi"),
        ] {
            assert_eq!(skin_of(style).map(|skin| skin.id), Some(id));
        }
        assert!(skin_of(DockStyle::Native).is_none());
        assert!(skin_of(DockStyle::Custom).is_none());
    }

    // ---- The Skins folder ----

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[0; 16]);
        bytes
    }

    fn json(width: u32, height: u32, left: u32, right: u32, image: &str) -> String {
        serde_json::json!({
            "name": "Minha barra", "image": image, "width": width, "height": height,
            "slice": {"top": 0, "right": right, "bottom": 0, "left": left},
            "middle": "stretch", "pixelated": false,
        })
        .to_string()
    }

    /// A folder `name` in `root`, with these files (`None`: left out).
    fn skin_dir(root: &Path, name: &str, json: Option<&str>, png: Option<&[u8]>) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        if let Some(json) = json {
            fs::write(dir.join("skin.json"), json).unwrap();
        }
        if let Some(png) = png {
            fs::write(dir.join("skin.png"), png).unwrap();
        }
    }

    fn problem_of(root: &Path, name: &str) -> Option<ProblemKind> {
        read_folder(root)
            .1
            .into_iter()
            .find(|problem| problem.folder == name)
            .map(|problem| problem.kind)
    }

    #[test]
    fn a_bundled_skin_json_reads_without_the_optional_fields() {
        for skin in &SKINS {
            let meta = skin.meta();
            assert_eq!(
                (&meta.dark, &meta.ink, &meta.indicator),
                (&None, &None, &None),
                "{}",
                skin.id
            );
        }
        let meta: SkinMeta = serde_json::from_str(
            r##"{"name":"X","image":"skin.png","width":8,"height":8,
                "slice":{"top":0,"right":2,"bottom":0,"left":2},"middle":"repeat",
                "pixelated":true,"dark":false,"ink":"#ffffff","indicator":"#ff0000"}"##,
        )
        .unwrap();
        assert_eq!(meta.dark, Some(false));
        assert_eq!(meta.ink.as_deref(), Some("#ffffff"));
        assert_eq!(meta.indicator.as_deref(), Some("#ff0000"));
    }

    #[test]
    fn a_valid_folder_skin_is_read_with_its_image() {
        let root = tempfile::tempdir().unwrap();
        let image = png(12, 6);
        skin_dir(
            root.path(),
            "Minha",
            Some(&json(12, 6, 3, 3, "skin.png")),
            Some(&image),
        );

        let (skins, problems) = read_folder(root.path());

        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(skins.len(), 1);
        assert_eq!(skins[0].id, "Minha");
        assert_eq!(skins[0].meta.name, "Minha barra");
        assert_eq!(skins[0].png, image);
        assert_eq!(load(root.path(), "Minha"), Some(skins[0].clone()));
        assert_eq!(load(root.path(), "Outra"), None);
        assert_eq!(load(root.path(), "../Minha"), None);
    }

    #[test]
    fn a_missing_skins_folder_is_empty_and_has_no_problem() {
        let root = tempfile::tempdir().unwrap();
        let (skins, problems) = read_folder(&root.path().join(FOLDER));
        assert!(skins.is_empty() && problems.is_empty());
    }

    #[test]
    fn the_skins_and_the_problems_come_by_name_and_files_are_left_out() {
        let root = tempfile::tempdir().unwrap();
        let good = json(4, 4, 1, 1, "skin.png");
        for name in ["c", "a", "b"] {
            skin_dir(root.path(), name, Some(&good), Some(&png(4, 4)));
        }
        for name in ["z", "y"] {
            skin_dir(root.path(), name, None, None);
        }
        fs::write(root.path().join("solto.png"), png(4, 4)).unwrap();

        let (skins, problems) = read_folder(root.path());

        let ids: Vec<_> = skins.iter().map(|skin| skin.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
        let folders: Vec<_> = problems.iter().map(|p| p.folder.as_str()).collect();
        assert_eq!(folders, ["y", "z"]);
    }

    #[test]
    fn a_folder_name_must_be_short_and_plain() {
        let longest = "a".repeat(40);
        let too_long = "a".repeat(41);
        for id in ["a", "Minha skin", "pixel_bar-2.0", longest.as_str()] {
            assert!(is_valid_skin_id(id), "{id}");
        }
        for id in [
            "",
            ".oculta",
            "..",
            "a/b",
            r"a\b",
            "á",
            "a:b",
            too_long.as_str(),
        ] {
            assert!(!is_valid_skin_id(id), "{id:?}");
        }
        let root = tempfile::tempdir().unwrap();
        let good = json(4, 4, 1, 1, "skin.png");
        for name in [".oculta", "com#sinal"] {
            skin_dir(root.path(), name, Some(&good), Some(&png(4, 4)));
            assert_eq!(problem_of(root.path(), name), Some(ProblemKind::BadName));
        }
    }

    #[test]
    fn each_problem_of_a_folder_skin_is_named() {
        let root = tempfile::tempdir().unwrap();
        let r = root.path();
        let good = json(8, 4, 2, 2, "skin.png");
        skin_dir(r, "sem-json", None, Some(&png(8, 4)));
        skin_dir(r, "json-ruim", Some("{ not json"), Some(&png(8, 4)));
        skin_dir(
            r,
            "json-sem-campo",
            Some(r#"{"name":"x"}"#),
            Some(&png(8, 4)),
        );
        skin_dir(
            r,
            "json-enorme",
            Some(&" ".repeat(70 * 1024)),
            Some(&png(8, 4)),
        );
        skin_dir(
            r,
            "outra-imagem",
            Some(&json(8, 4, 2, 2, "../x.png")),
            Some(&png(8, 4)),
        );
        skin_dir(r, "sem-png", Some(&good), None);
        let mut big = png(8, 4);
        big.resize(MAX_PNG_BYTES as usize + 1, 0);
        skin_dir(r, "png-enorme", Some(&good), Some(&big));
        skin_dir(
            r,
            "nao-png",
            Some(&good),
            Some(b"GIF89a, long enough to have a header"),
        );
        skin_dir(r, "outro-tamanho", Some(&good), Some(&png(8, 5)));
        skin_dir(
            r,
            "pontas-largas",
            Some(&json(8, 4, 4, 4, "skin.png")),
            Some(&png(8, 4)),
        );
        skin_dir(
            r,
            "sem-largura",
            Some(&json(0, 4, 0, 0, "skin.png")),
            Some(&png(0, 4)),
        );
        skin_dir(
            r,
            "alta-demais",
            Some(&json(8, 5000, 2, 2, "skin.png")),
            Some(&png(8, 5000)),
        );

        for (name, kind) in [
            ("sem-json", ProblemKind::NoJson),
            ("json-ruim", ProblemKind::BadJson),
            ("json-sem-campo", ProblemKind::BadJson),
            ("json-enorme", ProblemKind::BadJson),
            ("outra-imagem", ProblemKind::BadImageName),
            ("sem-png", ProblemKind::NoImage),
            ("png-enorme", ProblemKind::TooBig),
            ("nao-png", ProblemKind::NotPng),
            ("outro-tamanho", ProblemKind::SizeMismatch),
            ("pontas-largas", ProblemKind::BadSlice),
            ("sem-largura", ProblemKind::BadSlice),
            ("alta-demais", ProblemKind::BadSlice),
        ] {
            assert_eq!(problem_of(r, name), Some(kind), "{name}");
        }
        assert!(read_folder(r).0.is_empty());
    }

    #[test]
    fn a_skin_json_with_a_bom_is_read() {
        let root = tempfile::tempdir().unwrap();
        let text = format!("\u{feff}{}", json(4, 4, 1, 1, "skin.png"));
        skin_dir(root.path(), "bom", Some(&text), Some(&png(4, 4)));
        assert_eq!(read_folder(root.path()).0.len(), 1);
    }

    #[test]
    fn the_cached_skin_is_read_again_when_its_files_change() {
        let root = tempfile::tempdir().unwrap();
        skin_dir(
            root.path(),
            "Viva",
            Some(&json(4, 4, 1, 1, "skin.png")),
            Some(&png(4, 4)),
        );

        let first = cached(root.path(), "Viva").unwrap();
        let again = cached(root.path(), "Viva").unwrap();
        assert!(Arc::ptr_eq(&first, &again));

        // A wider image, and so a longer file: another key, read again.
        skin_dir(
            root.path(),
            "Viva",
            Some(&json(6, 4, 1, 1, "skin.png")),
            None,
        );
        let mut longer = png(6, 4);
        longer.push(0);
        fs::write(root.path().join("Viva").join("skin.png"), &longer).unwrap();
        let edited = cached(root.path(), "Viva").unwrap();
        assert_ne!(edited.key, first.key);
        assert_eq!(edited.skin.meta.width, 6);

        assert!(cached(root.path(), "Sumiu").is_none());
        assert!(cached(root.path(), "../Viva").is_none());
    }
}

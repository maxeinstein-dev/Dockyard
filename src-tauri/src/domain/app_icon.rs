//! The icon chosen for one program (spec 0024, B4b of spec 0007): its
//! Windows icon whatever the pack, a pack's image, or an image of the user's
//! own. Kept in the settings by the program's id in the Apps folder; a
//! program with no choice wears the dock's pack, as before.
//!
//! An image of the user's own is a PNG or an ICO read once, when chosen, and
//! kept as a PNG in the `AppIcons` folder of the data, under a name of its
//! own: the dock never reads the user's file again.

use serde::{Deserialize, Serialize};

use super::icon_pack::{
    MAX_IMAGE_SIDE, MIN_IMAGE_SIDE, is_valid_image_size, is_valid_pack_id, png_size,
};

/// The most programs with an icon chosen.
pub const MAX_APP_ICONS: usize = 500;

/// The biggest image file the user may choose, in bytes.
pub const MAX_CHOSEN_BYTES: u64 = 2 * 1024 * 1024;

/// What a program wears instead of the dock's pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AppIcon {
    /// The icon Windows gives, whatever the pack.
    Original,
    /// The image the pack `pack` has for it (its fallback if it has none).
    Pack { pack: String },
    /// An image of the user's own, the file `file` of `AppIcons`.
    Image { file: String },
}

impl AppIcon {
    pub fn is_valid(&self) -> bool {
        match self {
            AppIcon::Original => true,
            AppIcon::Pack { pack } => is_valid_pack_id(pack),
            AppIcon::Image { file } => is_image_file(file),
        }
    }

    /// The image of `AppIcons` it names, if it is one.
    pub fn file(&self) -> Option<&str> {
        match self {
            AppIcon::Image { file } => Some(file),
            _ => None,
        }
    }

    /// A name for it that changes when it does: what the dock keeps its
    /// picture by.
    pub fn source(&self) -> String {
        match self {
            AppIcon::Original => "original".to_owned(),
            AppIcon::Pack { pack } => format!("pack:{pack}"),
            AppIcon::Image { file } => format!("image:{file}"),
        }
    }
}

/// The key a program's choice is kept by: its id, in lowercase (Windows does
/// not tell the case of an id apart).
pub fn choice_key(app_id: &str) -> String {
    app_id.to_lowercase()
}

/// A name of `AppIcons` Dockyard gave: a UUID and `.png`.
pub fn is_image_file(name: &str) -> bool {
    name.len() == 40
        && name.ends_with(".png")
        && name[..36]
            .bytes()
            .enumerate()
            .all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase(),
            })
}

/// Why a chosen image is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageError {
    TooBig,
    NotAnImage,
    BadSize,
}

/// The PNG the dock keeps for a chosen file's `bytes`: a PNG as it is, an
/// ICO's biggest picture (one kept as a PNG inside it, or a 32-bit bitmap
/// turned into one).
pub fn chosen_png(bytes: &[u8]) -> Result<Vec<u8>, ImageError> {
    if bytes.len() as u64 > MAX_CHOSEN_BYTES {
        return Err(ImageError::TooBig);
    }
    let png = match png_size(bytes) {
        Some(_) => bytes.to_vec(),
        None => ico_picture(bytes).ok_or(ImageError::NotAnImage)?,
    };
    let size = png_size(&png).ok_or(ImageError::NotAnImage)?;
    if !is_valid_image_size(size) {
        return Err(ImageError::BadSize);
    }
    Ok(png)
}

/// The biggest picture of an ICO, as a PNG; `None` when it is not an ICO or
/// its biggest picture is a bitmap of another depth than 32 bits.
fn ico_picture(bytes: &[u8]) -> Option<Vec<u8>> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    if u16_at(0)? != 0 || u16_at(2)? != 1 {
        return None;
    }
    let count = usize::from(u16_at(4)?);
    let side = |byte: u8| if byte == 0 { 256 } else { u32::from(byte) };
    let (offset, length) = (0..count)
        .filter_map(|index| {
            let entry = 6 + index * 16;
            let width = side(*bytes.get(entry)?);
            let length = usize::try_from(u32_at(entry + 8)?).ok()?;
            let offset = usize::try_from(u32_at(entry + 12)?).ok()?;
            Some((width, offset, length))
        })
        .max_by_key(|(width, _, _)| *width)
        .map(|(_, offset, length)| (offset, length))?;
    let picture = bytes.get(offset..offset.checked_add(length)?)?;
    if png_size(picture).is_some() {
        return Some(picture.to_vec());
    }
    bitmap_png(picture)
}

/// A 32-bit bitmap of an ICO (a `BITMAPINFOHEADER`, the colors bottom-up,
/// then the 1-bit mask), as a PNG.
fn bitmap_png(dib: &[u8]) -> Option<Vec<u8>> {
    let i32_at = |at: usize| Some(i32::from_le_bytes(dib.get(at..at + 4)?.try_into().ok()?));
    let u16_at = |at: usize| Some(u16::from_le_bytes(dib.get(at..at + 2)?.try_into().ok()?));
    let header = usize::try_from(i32_at(0)?).ok()?;
    let width = u32::try_from(i32_at(4)?).ok()?;
    // The height counts the colors and the mask.
    let height = u32::try_from(i32_at(8)?).ok()? / 2;
    if u16_at(14)? != 32
        || i32_at(16)? != 0
        || !(MIN_IMAGE_SIDE..=MAX_IMAGE_SIDE).contains(&width)
        || !(MIN_IMAGE_SIDE..=MAX_IMAGE_SIDE).contains(&height)
    {
        return None;
    }
    let (w, h) = (width as usize, height as usize);
    let colors = dib.get(header..header + w * h * 4)?;
    let mask_row = w.div_ceil(32) * 4;
    let mask = dib.get(header + w * h * 4..).unwrap_or_default();
    // An old icon leaves the alpha empty and says what is clear in its mask.
    let no_alpha = colors.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 0);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in (0..h).rev() {
        for column in 0..w {
            let at = (row * w + column) * 4;
            let (b, g, r, a) = (colors[at], colors[at + 1], colors[at + 2], colors[at + 3]);
            let alpha = if no_alpha {
                let bit = mask
                    .get(row * mask_row + column / 8)
                    .is_some_and(|byte| byte & (0x80 >> (column % 8)) != 0);
                if bit { 0 } else { 255 }
            } else {
                a
            };
            rgba.extend_from_slice(&[r, g, b, alpha]);
        }
    }
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&rgba).ok()?;
    }
    Some(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "0192f1a2-0b1c-7d2e-8f30-123456789abc.png";

    /// A PNG of `side` × `side`, every pixel red.
    fn png(side: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, side, side);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[255, 0, 0, 255].repeat((side * side) as usize))
            .unwrap();
        drop(writer);
        bytes
    }

    /// An ICO of `pictures` (its side and its bytes).
    fn ico(pictures: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = vec![0, 0, 1, 0];
        bytes.extend((pictures.len() as u16).to_le_bytes());
        let mut offset = 6 + pictures.len() * 16;
        for (side, picture) in pictures {
            bytes.extend([*side, *side, 0, 0, 1, 0, 32, 0]);
            bytes.extend((picture.len() as u32).to_le_bytes());
            bytes.extend((offset as u32).to_le_bytes());
            offset += picture.len();
        }
        for (_, picture) in pictures {
            bytes.extend(picture);
        }
        bytes
    }

    /// A 32-bit bitmap of `side` × `side` as an ICO keeps it, the bottom
    /// row blue and the rest green, with no alpha and a mask that clears
    /// the top-left pixel.
    fn dib(side: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(40i32.to_le_bytes());
        bytes.extend((side as i32).to_le_bytes());
        bytes.extend((side as i32 * 2).to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(32u16.to_le_bytes());
        bytes.extend([0u8; 24]);
        for row in 0..side {
            for _ in 0..side {
                // Bottom-up: the first row is the bottom one.
                bytes.extend(if row == 0 {
                    [255, 0, 0, 0]
                } else {
                    [0, 255, 0, 0]
                });
            }
        }
        let mask_row = (side as usize).div_ceil(32) * 4;
        for row in 0..side {
            let mut line = vec![0u8; mask_row];
            if row == side - 1 {
                line[0] = 0x80;
            }
            bytes.extend(line);
        }
        bytes
    }

    fn pixels(png_bytes: &[u8]) -> (u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buffer).unwrap();
        (info.width, buffer)
    }

    #[test]
    fn the_choices_are_kept_by_kind_and_checked() {
        let pack = AppIcon::Pack {
            pack: "pixel-system".into(),
        };
        assert_eq!(
            serde_json::to_string(&pack).unwrap(),
            r#"{"kind":"pack","pack":"pixel-system"}"#
        );
        let image: AppIcon =
            serde_json::from_str(&format!(r#"{{"kind":"image","file":"{FILE}"}}"#)).unwrap();
        assert!(image.is_valid());
        assert_eq!(image.file(), Some(FILE));
        assert_eq!(image.source(), format!("image:{FILE}"));
        assert!(AppIcon::Original.is_valid());
        assert!(
            !AppIcon::Pack {
                pack: "../x".into()
            }
            .is_valid()
        );
        for name in [
            "..\\x.png",
            "a.png",
            &FILE.to_uppercase(),
            &FILE.replace(".png", ".ico"),
        ] {
            assert!(
                !AppIcon::Image {
                    file: name.to_owned()
                }
                .is_valid(),
                "{name}"
            );
        }
        assert_eq!(
            choice_key("Microsoft.WindowsCalculator_8wekyb3d8bbwe!App"),
            "microsoft.windowscalculator_8wekyb3d8bbwe!app"
        );
    }

    #[test]
    fn a_png_is_kept_as_it_is_within_the_limits() {
        let bytes = png(64);
        assert_eq!(chosen_png(&bytes), Ok(bytes));
        assert_eq!(chosen_png(&png(8)), Err(ImageError::BadSize));
        assert_eq!(chosen_png(b"not an image"), Err(ImageError::NotAnImage));
        let huge = vec![0u8; MAX_CHOSEN_BYTES as usize + 1];
        assert_eq!(chosen_png(&huge), Err(ImageError::TooBig));
    }

    #[test]
    fn an_ico_gives_its_biggest_picture() {
        // A PNG inside: taken as it is, the biggest one (0 means 256).
        let small = png(16);
        let big = png(256);
        let icon = ico(&[(16, small), (0, big.clone())]);
        assert_eq!(chosen_png(&icon), Ok(big));

        // A bitmap: turned the right way up, the mask making a hole.
        let icon = ico(&[(32, dib(32))]);
        let (width, rgba) = pixels(&chosen_png(&icon).unwrap());
        assert_eq!(width, 32);
        assert_eq!(rgba[..4], [0, 255, 0, 0]);
        assert_eq!(rgba[4..8], [0, 255, 0, 255]);
        let last_row = (32 * 31 * 4) as usize;
        assert_eq!(rgba[last_row..last_row + 4], [0, 0, 255, 255]);
    }
}

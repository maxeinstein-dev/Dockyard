//! The Capturas de tela (F9 of estudo 0002), as Windows keeps them: the
//! Screenshots folder (where Print Screen and the Snipping Tool save), its
//! newest images, a small picture of each, and the file itself dragged out
//! of the panel, as File Explorer drags it.
//!
//! The folder is read only when the panel asks, off the dock's thread; the
//! pictures are made with WIC from the image alone, never with the shell's
//! thumbnail handlers, so no third-party extension loads in the app.

use std::path::{Path, PathBuf};

use crate::domain::folders::FolderEntry;
use crate::domain::widgets::screenshots::{MAX_SHOWN, is_image};

/// The user's Screenshots folder (`Imagens\Capturas de Tela`).
#[cfg(windows)]
pub fn folder() -> Option<PathBuf> {
    use ::windows::Win32::System::Com::CoTaskMemFree;
    use ::windows::Win32::UI::Shell::{
        FOLDERID_Screenshots, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    };
    let text =
        unsafe { SHGetKnownFolderPath(&FOLDERID_Screenshots, KF_FLAG_DEFAULT, None) }.ok()?;
    let path = unsafe { text.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(text.0.cast_const().cast())) };
    path.map(PathBuf::from)
}

#[cfg(not(windows))]
pub fn folder() -> Option<PathBuf> {
    None
}

/// The newest images of `folder`, the newest first, at most `MAX_SHOWN`.
pub fn newest(folder: &Path) -> std::io::Result<Vec<FolderEntry>> {
    let (entries, _) = super::folders::newest_in(folder)?;
    Ok(entries
        .into_iter()
        .filter(|entry| !entry.folder && is_image(&entry.name))
        .take(MAX_SHOWN)
        .collect())
}

/// The biggest file read for a picture: a screenshot of several monitors in
/// 4K is about 20 MB as PNG.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// A PNG of `path` that fits in `side` × `side`, keeping its proportions;
/// `None` for a file too big or not an image WIC reads.
#[cfg(windows)]
pub fn thumbnail(path: &Path, side: u32) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Imaging::{
        CLSID_WICImagingFactory, GUID_WICPixelFormat32bppRGBA, IWICImagingFactory,
        WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
        WICDecodeMetadataCacheOnDemand,
    };
    use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};

    if std::fs::metadata(path).ok()?.len() > MAX_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    super::apps::init_com();
    let (width, height, pixels) = unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let stream = wic.CreateStream().ok()?;
        stream.InitializeFromMemory(&bytes).ok()?;
        let decoder = wic
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)
            .ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (mut full_width, mut full_height) = (0, 0);
        frame.GetSize(&mut full_width, &mut full_height).ok()?;
        let (width, height) = fit(full_width, full_height, side)?;
        let scaler = wic.CreateBitmapScaler().ok()?;
        scaler
            .Initialize(&frame, width, height, WICBitmapInterpolationModeFant)
            .ok()?;
        let converter = wic.CreateFormatConverter().ok()?;
        converter
            .Initialize(
                &scaler,
                &GUID_WICPixelFormat32bppRGBA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeCustom,
            )
            .ok()?;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        converter
            .CopyPixels(std::ptr::null(), width * 4, &mut pixels)
            .ok()?;
        (width, height, pixels)
    };
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&pixels).ok()?;
    }
    Some(png)
}

#[cfg(not(windows))]
pub fn thumbnail(_path: &Path, _side: u32) -> Option<Vec<u8>> {
    None
}

/// The size of an image of `width` × `height` that fits in `side` ×
/// `side`, keeping its proportions and never growing it; `None` for an
/// empty image.
pub fn fit(width: u32, height: u32, side: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 || side == 0 {
        return None;
    }
    let scale = (f64::from(side) / f64::from(width.max(height))).min(1.0);
    let scaled = |value: u32| ((f64::from(value) * scale).round() as u32).max(1);
    Some((scaled(width), scaled(height)))
}

/// Drags the file `path` out, as File Explorer drags it: to a folder, a
/// chat or an e-mail. Runs on the window's thread, with the button held;
/// returns once it is dropped or let go.
#[cfg(windows)]
pub fn drag(path: &Path) -> windows::core::Result<()> {
    use windows::Win32::System::Com::IDataObject;
    use windows::Win32::System::Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK};
    use windows::Win32::UI::Shell::{BHID_DataObject, SHDoDragDrop};

    let items = super::open_with::items_of(&[path.to_path_buf()])?;
    let data: IDataObject = unsafe { items.BindToHandler(None, &BHID_DataObject)? };
    unsafe { SHDoDragDrop(None, &data, None, DROPEFFECT_COPY | DROPEFFECT_LINK)? };
    Ok(())
}

#[cfg(not(windows))]
pub fn drag(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_fits_the_side_keeping_its_proportions() {
        assert_eq!(fit(1920, 1080, 160), Some((160, 90)));
        assert_eq!(fit(1080, 1920, 160), Some((90, 160)));
        assert_eq!(fit(100, 50, 160), Some((100, 50)));
        assert_eq!(fit(5000, 2, 160), Some((160, 1)));
        assert_eq!(fit(0, 10, 160), None);
    }
}

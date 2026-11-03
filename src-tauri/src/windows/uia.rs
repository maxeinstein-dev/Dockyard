//! What the UI Automation providers of the native windows (the dock's,
//! the launcher's and the menu's `access.rs`) share: their COM objects stay
//! each in its own module, since each exposes different things; the small
//! pieces they all wrote the same way live here (M12).

use windows::Win32::System::Com::SAFEARRAY;
use windows::Win32::System::Ole::{SafeArrayCreateVector, SafeArrayPutElement};
use windows::Win32::System::Variant::VT_I4;
use windows::Win32::UI::Accessibility::{UiaAppendRuntimeId, UiaRect};
use windows::core::{Error, Result};

/// Nothing: a null out-pointer with success.
pub fn none<T>() -> Result<T> {
    Err(Error::empty())
}

/// A number for the key, the same each time it is asked: an element's
/// part of its runtime id.
pub fn number_of(key: &str) -> i32 {
    // FNV-1a: stable across runs, unlike the standard hasher.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in key.bytes() {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash as i32
}

/// The runtime id of an element of the window: UI Automation adds the
/// window's own (`UiaAppendRuntimeId`) before `number`.
pub fn runtime_id(number: i32) -> Result<*mut SAFEARRAY> {
    unsafe {
        let ids = SafeArrayCreateVector(VT_I4, 0, 2);
        if ids.is_null() {
            return none();
        }
        for (index, value) in [UiaAppendRuntimeId as i32, number].into_iter().enumerate() {
            let at = index as i32;
            SafeArrayPutElement(ids, &at, std::ptr::from_ref(&value).cast())?;
        }
        Ok(ids)
    }
}

/// An element's place on screen, from its left, top, width and height.
pub fn rect((left, top, width, height): (f64, f64, f64, f64)) -> UiaRect {
    UiaRect {
        left,
        top,
        width,
        height,
    }
}

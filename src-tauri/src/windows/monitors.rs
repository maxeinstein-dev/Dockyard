//! The monitors the dock goes on (spec 0010 D4b: one dock per monitor).

use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::core::BOOL;

use super::dock::Rect;

/// `MONITORINFOF_PRIMARY`.
const PRIMARY: u32 = 1;

/// A monitor as the dock sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// What names it across changes of the layout and across starts (spec
    /// 0028 A): the path of the monitor's device, from its EDID, which a new
    /// `\\.\DISPLAYn` number does not change; the device's name when
    /// Windows gives no path.
    pub key: String,
    /// The device's name (`\\.\DISPLAY1`): what the workspaces were kept
    /// by before spec 0028, adopted once.
    pub device: String,
    /// The name Windows shows for the monitor ("DELL U2419H"); the device's
    /// name when there is none.
    pub name: String,
    /// Its handle, as Windows gives it now (an `HMONITOR` as a number).
    pub handle: isize,
    /// Its rectangle and its work area, in physical pixels.
    pub rect: Rect,
    pub work: Rect,
    /// Its scale: 1 at 96 DPI.
    pub scale: f64,
    pub primary: bool,
}

fn rect(r: RECT) -> Rect {
    Rect {
        x: r.left,
        y: r.top,
        width: u32::try_from(r.right - r.left).unwrap_or(0),
        height: u32::try_from(r.bottom - r.top).unwrap_or(0),
    }
}

/// The monitor `handle`; `None` when it can't be read.
fn describe(handle: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let read =
        unsafe { GetMonitorInfoW(handle, std::ptr::from_mut(&mut info).cast::<MONITORINFO>()) };
    if !read.as_bool() {
        return None;
    }
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    let _ = unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
    let length = info
        .szDevice
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(info.szDevice.len());
    let device = String::from_utf16_lossy(&info.szDevice[..length]);
    let (key, name) = identity(&device).unwrap_or_else(|| (device.clone(), device.clone()));
    Some(Monitor {
        key,
        device,
        name,
        handle: handle.0 as isize,
        rect: rect(info.monitorInfo.rcMonitor),
        work: rect(info.monitorInfo.rcWork),
        scale: f64::from(dpi_x.max(1)) / 96.0,
        primary: info.monitorInfo.dwFlags & PRIMARY != 0,
    })
}

/// The path and the shown name of the monitor on the device `device`, as
/// Windows' display configuration gives them; kept for 2 s (the monitors
/// are listed often, and the configuration seldom changes).
fn identity(device: &str) -> Option<(String, String)> {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    type Known = HashMap<String, (String, String)>;
    static KNOWN: Mutex<Option<(Instant, Known)>> = Mutex::new(None);
    let mut known = KNOWN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fresh = known
        .as_ref()
        .is_some_and(|(at, _)| at.elapsed() < Duration::from_secs(2));
    if !fresh {
        *known = Some((Instant::now(), read_identities()));
    }
    known.as_ref()?.1.get(device).cloned()
}

/// Each active monitor's device name, with its device path and its shown
/// name (`QueryDisplayConfig`).
fn read_identities() -> std::collections::HashMap<String, (String, String)> {
    use windows::Win32::Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
        DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME,
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS,
        QueryDisplayConfig,
    };
    let mut found = std::collections::HashMap::new();
    let (mut paths_len, mut modes_len) = (0u32, 0u32);
    if unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut paths_len, &mut modes_len) }
        .is_err()
    {
        return found;
    }
    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); paths_len as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); modes_len as usize];
    let read = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut paths_len,
            paths.as_mut_ptr(),
            &mut modes_len,
            modes.as_mut_ptr(),
            None,
        )
    };
    if read.is_err() {
        return found;
    }
    let text = |units: &[u16]| {
        let end = units
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(units.len());
        String::from_utf16_lossy(&units[..end])
    };
    for path in paths.iter().take(paths_len as usize) {
        let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                adapterId: path.sourceInfo.adapterId,
                id: path.sourceInfo.id,
            },
            ..Default::default()
        };
        let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            ..Default::default()
        };
        let read_source = unsafe { DisplayConfigGetDeviceInfo(&mut source.header) };
        let read_target = unsafe { DisplayConfigGetDeviceInfo(&mut target.header) };
        if read_source != 0 || read_target != 0 {
            continue;
        }
        let device = text(&source.viewGdiDeviceName);
        let path_text = text(&target.monitorDevicePath);
        let name = text(&target.monitorFriendlyDeviceName);
        if device.is_empty() || path_text.is_empty() {
            continue;
        }
        let name = if name.is_empty() {
            device.clone()
        } else {
            name
        };
        found.insert(device, (path_text.to_lowercase(), name));
    }
    found
}

/// Every monitor, the primary first.
pub fn all() -> Vec<Monitor> {
    unsafe extern "system" fn collect(
        handle: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        state: LPARAM,
    ) -> BOOL {
        crate::windows::panics::window_message(|| {
            let found = unsafe { &mut *(state.0 as *mut Vec<HMONITOR>) };
            found.push(handle);
            true.into()
        })
        .unwrap_or_else(|| false.into())
    }
    let mut handles: Vec<HMONITOR> = Vec::new();
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(std::ptr::from_mut(&mut handles) as isize),
        )
    };
    let mut monitors: Vec<Monitor> = handles.into_iter().filter_map(describe).collect();
    monitors.sort_by_key(|monitor| !monitor.primary);
    monitors
}

//! What the dock shows at its end while the Windows taskbar is hidden, as the
//! taskbar does (and the PrimoDock): the battery, the network and the
//! volume. Read when asked; cheap enough to ask every few seconds.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
    GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses, IF_TYPE_IEEE80211, IF_TYPE_SOFTWARE_LOOPBACK,
    IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::System::Com::{CLSCTX_ALL, CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// The battery: how full, whether it is charging, and whether Windows'
/// energy saver is on while it is not (its icon then turns orange, as the
/// taskbar's, which is orange only off the charger).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
    pub saver: bool,
}

/// How the computer is online.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    Wifi,
    Wired,
    Offline,
}

/// The volume of the default output: 0–100, and whether it is muted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume {
    pub percent: u8,
    pub muted: bool,
}

/// Everything at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// `None` on a computer with no battery: nothing is shown for it.
    pub battery: Option<Battery>,
    pub network: Network,
    /// `None` with no output device.
    pub volume: Option<Volume>,
}

pub fn read() -> Status {
    Status {
        battery: battery(),
        network: network(),
        volume: volume(),
    }
}

/// The last reading, and when it was made.
static LAST: Mutex<Option<(Instant, Status)>> = Mutex::new(None);

/// How long a reading serves every dock: each dock asks every 3 s, so with
/// two monitors the second one gets the first one's reading.
const SHARED_FOR: Duration = Duration::from_millis(2_500);

/// What [`read`] gives, read once for all the docks that ask within
/// [`SHARED_FOR`].
pub fn read_shared() -> Status {
    let mut last = LAST.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if let Some((at, status)) = *last
        && still_fresh(at, now)
    {
        return status;
    }
    let status = read();
    *last = Some((now, status));
    status
}

/// Drops the shared reading: the next [`read_shared`] reads again. The power
/// line changed (`WM_POWERBROADCAST`): the battery's icon follows at once,
/// not at the next tick (30 s while the dock saves).
pub fn forget_shared() {
    *LAST.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

/// Whether a reading made `at` still serves `now`.
fn still_fresh(at: Instant, now: Instant) -> bool {
    now.saturating_duration_since(at) < SHARED_FOR
}

/// Whether the computer runs on its battery now (the battery saver, spec
/// 0021): unplugged. A desktop, or a line Windows can't tell, is not.
pub fn on_battery() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.is_ok() && unplugged(&status)
}

/// Whether Windows' energy saver (battery saver) is on now: the dock saves
/// with it too, under "Na bateria". The old battery saver sets
/// `SystemStatusFlag`; Windows 11's energy saver is told only by its own
/// power setting, `GUID_ENERGY_SAVER_STATUS` (achado do autor, 2026-11-05:
/// with it on, the flag stayed 0 and the effective power mode said "better
/// battery", while the setting said 1).
pub fn energy_saver() -> bool {
    let mut status = SYSTEM_POWER_STATUS::default();
    let flag = unsafe { GetSystemPowerStatus(&mut status) }.is_ok() && saver_on(&status);
    flag || setting_saves()
}

/// `GUID_ENERGY_SAVER_STATUS` (winnt.h, Windows 11): 0 off, 1 on, 2 on with
/// the most savings.
const ENERGY_SAVER_STATUS: windows::core::GUID =
    windows::core::GUID::from_u128(0x550e8400_e29b_41d4_a716_446655440000);

/// The energy saver's status as last told, -1 before it is.
static SAVER_STATUS: AtomicI32 = AtomicI32::new(-1);

/// The energy saver is on, by its own setting.
fn setting_saves() -> bool {
    SAVER_STATUS.load(Ordering::Relaxed) > 0
}

/// What [`watch_power_mode`] calls when the status changes.
static ON_CHANGE: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

/// Windows tells the energy saver's status: a `POWERBROADCAST_SETTING`
/// whose data is a `u32`.
unsafe extern "system" fn saver_changed(
    _context: *const core::ffi::c_void,
    _kind: u32,
    setting: *const core::ffi::c_void,
) -> u32 {
    let _ = super::panics::window_message(|| {
        use windows::Win32::System::Power::POWERBROADCAST_SETTING;
        if setting.is_null() {
            return;
        }
        let told = unsafe { &*setting.cast::<POWERBROADCAST_SETTING>() };
        if told.PowerSetting != ENERGY_SAVER_STATUS || told.DataLength < 4 {
            return;
        }
        let data = unsafe { std::ptr::addr_of!(told.Data).cast::<u32>().read_unaligned() };
        let now = i32::try_from(data).unwrap_or(0);
        let before = SAVER_STATUS.swap(now, Ordering::Relaxed);
        if before != now {
            log::info!("power: energy saver status {now}");
            if let Some(changed) = ON_CHANGE.get() {
                changed();
            }
        }
    });
    0
}

/// Listens to Windows' energy saver for the rest of the run (once): Windows
/// tells its status at once and on every change, and `changed` runs then.
pub fn watch_power_mode(changed: impl Fn() + Send + Sync + 'static) {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Power::{
        DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, PowerSettingRegisterNotification,
    };
    use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK;
    if ON_CHANGE.set(Box::new(changed)).is_err() {
        return;
    }
    // Windows keeps the pointer for as long as the registration lasts: the
    // run's.
    let parameters: &'static mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS =
        Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(saver_changed),
            Context: std::ptr::null_mut(),
        }));
    let mut handle: *mut core::ffi::c_void = std::ptr::null_mut();
    let registered = unsafe {
        PowerSettingRegisterNotification(
            &ENERGY_SAVER_STATUS,
            DEVICE_NOTIFY_CALLBACK,
            HANDLE(std::ptr::from_mut(parameters).cast()),
            &mut handle,
        )
    };
    if registered.is_err() {
        // An older Windows: the battery saver's flag still holds.
        log::info!("power: the energy saver's status is not told ({registered:?})");
    }
}

/// `SystemStatusFlag`: 1 while the energy saver is on.
fn saver_on(status: &SYSTEM_POWER_STATUS) -> bool {
    status.SystemStatusFlag == 1
}

/// `ACLineStatus`: 0 offline, 1 online, 255 unknown.
fn unplugged(status: &SYSTEM_POWER_STATUS) -> bool {
    status.ACLineStatus == 0
}

fn battery() -> Option<Battery> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    battery_of(&status).map(|battery| Battery {
        saver: (battery.saver || setting_saves()) && !battery.charging,
        ..battery
    })
}

/// The battery `status` describes; `None` with none (flag 128) or an unknown
/// charge (255).
fn battery_of(status: &SYSTEM_POWER_STATUS) -> Option<Battery> {
    const NO_BATTERY: u8 = 128;
    const UNKNOWN: u8 = 255;
    if status.BatteryFlag & NO_BATTERY != 0
        || status.BatteryFlag == UNKNOWN
        || status.BatteryLifePercent == UNKNOWN
    {
        return None;
    }
    Some(Battery {
        percent: status.BatteryLifePercent.min(100),
        // 8: charging; or on the charger.
        charging: status.BatteryFlag & 8 != 0 || status.ACLineStatus == 1,
        // Orange as the taskbar's: only off the charger.
        saver: saver_on(status) && status.ACLineStatus != 1 && status.BatteryFlag & 8 == 0,
    })
}

/// Online through a Wi-Fi adapter, through another one, or not at all: an
/// adapter counts when it is up and has a gateway.
fn network() -> Network {
    let flags = GAA_FLAG_INCLUDE_GATEWAYS
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size: u32 = 16 * 1024;
    for _ in 0..3 {
        // u64 keeps the buffer aligned for the structure.
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let result = unsafe { GetAdaptersAddresses(0, flags, None, Some(first), &mut size) };
        const BUFFER_OVERFLOW: u32 = 111;
        if result == BUFFER_OVERFLOW {
            continue;
        }
        if result != 0 {
            return Network::Offline;
        }
        let mut found = Network::Offline;
        let mut at = first.cast_const();
        while !at.is_null() {
            let adapter = unsafe { &*at };
            let online = adapter.OperStatus == IfOperStatusUp
                && adapter.IfType != IF_TYPE_SOFTWARE_LOOPBACK
                && !adapter.FirstGatewayAddress.is_null();
            if online {
                if adapter.IfType == IF_TYPE_IEEE80211 {
                    return Network::Wifi;
                }
                found = Network::Wired;
            }
            at = adapter.Next;
        }
        return found;
    }
    Network::Offline
}

/// The default output's volume. Needs COM on the calling thread (the dock's).
fn volume() -> Option<Volume> {
    unsafe {
        let devices: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER).ok()?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let control: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let level = control.GetMasterVolumeLevelScalar().ok()?;
        let muted = control.GetMute().ok()?.as_bool();
        Some(Volume {
            percent: (level.clamp(0.0, 1.0) * 100.0).round() as u8,
            muted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_reading_serves_every_dock_for_a_moment() {
        let at = Instant::now();
        // The second dock asks a moment after the first: the same reading.
        assert!(still_fresh(at, at));
        assert!(still_fresh(at, at + Duration::from_millis(400)));
        // The next round, 3 s later, reads again.
        assert!(!still_fresh(at, at + Duration::from_secs(3)));
    }

    fn power(flag: u8, percent: u8, line: u8) -> SYSTEM_POWER_STATUS {
        SYSTEM_POWER_STATUS {
            ACLineStatus: line,
            BatteryFlag: flag,
            BatteryLifePercent: percent,
            ..Default::default()
        }
    }

    #[test]
    fn a_desktop_has_no_battery_to_show() {
        assert_eq!(battery_of(&power(128, 255, 1)), None);
        assert_eq!(battery_of(&power(255, 255, 255)), None);
    }

    #[test]
    fn only_a_line_off_is_on_the_battery() {
        assert!(unplugged(&power(1, 80, 0)));
        assert!(!unplugged(&power(8, 40, 1)));
        // A desktop, and a line Windows can't tell.
        assert!(!unplugged(&power(128, 255, 1)));
        assert!(!unplugged(&power(255, 255, 255)));
    }

    #[test]
    fn a_laptop_shows_its_charge_and_whether_it_charges() {
        assert_eq!(
            battery_of(&power(1, 80, 0)),
            Some(Battery {
                percent: 80,
                charging: false,
                saver: false
            })
        );
        assert_eq!(
            battery_of(&power(8, 40, 1)),
            Some(Battery {
                percent: 40,
                charging: true,
                saver: false
            })
        );
    }

    #[test]
    fn the_energy_saver_comes_with_the_battery() {
        let on = SYSTEM_POWER_STATUS {
            SystemStatusFlag: 1,
            ..power(1, 30, 0)
        };
        assert!(saver_on(&on));
        assert!(battery_of(&on).is_some_and(|battery| battery.saver));
        assert!(!saver_on(&power(1, 30, 0)));
        // On the charger the icon is not orange, as the taskbar's.
        let charging = SYSTEM_POWER_STATUS {
            SystemStatusFlag: 1,
            ..power(8, 30, 1)
        };
        assert!(battery_of(&charging).is_some_and(|battery| !battery.saver));
    }
}

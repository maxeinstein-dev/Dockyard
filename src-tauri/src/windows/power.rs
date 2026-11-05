//! The battery saver as it stands (spec 0021): the docks decide it, from the
//! setting and the power line, and the threads that poll (the Tocando
//! agora, the Clima, the update) read it to space out their readings.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::domain::power::{self, Poll};
use crate::domain::settings::PowerSaver;

/// Whether the dock saves now; false until a dock first decides.
static SAVING: AtomicBool = AtomicBool::new(false);

/// Whether the dock saves now, as last decided.
pub fn saving() -> bool {
    SAVING.load(Ordering::Relaxed)
}

/// Decides again from `mode` and the power line, read now; the answer.
/// Windows' energy saver counts as the battery: with it on, "Na bateria"
/// saves even on the charger.
/// Cheap (`GetSystemPowerStatus` only): the docks call it on every status
/// tick, on `WM_POWERBROADCAST` and when the settings change.
pub fn decide(mode: PowerSaver) -> bool {
    let now = power::saving(
        mode,
        super::system_status::on_battery() || super::system_status::energy_saver(),
    );
    SAVING.store(now, Ordering::Relaxed);
    now
}

/// How long between two readings of `poll` now.
pub fn every(poll: Poll) -> Duration {
    power::every(poll, saving())
}

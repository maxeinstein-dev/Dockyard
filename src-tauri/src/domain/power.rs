//! The battery saver (spec 0021): when it is on, and how often what is not
//! urgent is read then. `windows::power` reads the power line and keeps the
//! answer for the threads that poll.

use std::time::Duration;

use super::settings::PowerSaver;

/// Whether the dock saves now: always, never, or while the computer runs on
/// its battery (`on_battery`: unplugged; a desktop never is).
pub fn saving(mode: PowerSaver, on_battery: bool) -> bool {
    match mode {
        PowerSaver::Off => false,
        PowerSaver::OnBattery => on_battery,
        PowerSaver::Always => true,
    }
}

/// What is read again on its own, without the user asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// The battery, the network and the volume at the dock's end.
    Status,
    /// The processor, the memory and the network of the system widgets.
    Activity,
    /// What plays now, for the Tocando agora.
    Media,
    /// The Clima's forecast, from Open-Meteo.
    Weather,
    /// A new version, from GitHub.
    Update,
}

/// How long between two readings of `poll`, `saving` or not (the table of
/// spec 0021 §3). Saving, each reading is 3 to 10 times rarer; what the
/// user does (a click, an open panel) still reads at once, and the clock
/// keeps its minute.
pub fn every(poll: Poll, saving: bool) -> Duration {
    let seconds = match (poll, saving) {
        (Poll::Status, false) => 3,
        (Poll::Status, true) => 30,
        (Poll::Activity, false) => 2,
        (Poll::Activity, true) => 6,
        (Poll::Media, false) => 1,
        (Poll::Media, true) => 3,
        (Poll::Weather, false) => 30 * 60,
        (Poll::Weather, true) => 2 * 60 * 60,
        // Every 6 hours, the update's own pace.
        (Poll::Update, false) => super::update::EVERY.as_secs(),
        (Poll::Update, true) => 24 * 60 * 60,
    };
    Duration::from_secs(seconds)
}

/// How often a long wait (the Clima's, the update's) looks at the power
/// again, so plugging the charger back in shortens it within this.
pub const RECHECK: Duration = Duration::from_secs(10 * 60);

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Poll; 5] = [
        Poll::Status,
        Poll::Activity,
        Poll::Media,
        Poll::Weather,
        Poll::Update,
    ];

    #[test]
    fn off_never_saves_always_always_does_and_on_battery_follows_the_line() {
        for on_battery in [false, true] {
            assert!(!saving(PowerSaver::Off, on_battery));
            assert!(saving(PowerSaver::Always, on_battery));
        }
        assert!(saving(PowerSaver::OnBattery, true));
        assert!(!saving(PowerSaver::OnBattery, false));
    }

    #[test]
    fn the_default_saves_only_unplugged() {
        assert!(saving(PowerSaver::default(), true));
        assert!(!saving(PowerSaver::default(), false));
    }

    #[test]
    fn the_intervals_without_saving_are_the_ones_of_before() {
        assert_eq!(every(Poll::Status, false), Duration::from_secs(3));
        assert_eq!(every(Poll::Activity, false), Duration::from_secs(2));
        assert_eq!(every(Poll::Media, false), Duration::from_secs(1));
        assert_eq!(every(Poll::Weather, false), Duration::from_secs(30 * 60));
        assert_eq!(every(Poll::Update, false), Duration::from_secs(6 * 3600));
    }

    #[test]
    fn saving_the_status_is_read_every_30_seconds() {
        assert_eq!(every(Poll::Status, true), Duration::from_secs(30));
    }

    #[test]
    fn saving_every_reading_is_rarer_and_none_stops() {
        for poll in ALL {
            let normal = every(poll, false);
            let saving = every(poll, true);
            assert!(saving >= normal * 3, "{poll:?}: {saving:?} vs {normal:?}");
            assert!(saving <= Duration::from_secs(24 * 60 * 60), "{poll:?}");
        }
    }

    #[test]
    fn a_long_wait_looks_at_the_power_more_often_than_it_lasts() {
        assert!(RECHECK < every(Poll::Weather, true));
        assert!(RECHECK < every(Poll::Update, false));
    }
}

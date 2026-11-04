//! The Relógio mundial (F9 of estudo 0002): the time in a few places at
//! once, on a wide tile, as the Clock is in one. The config holds the zones,
//! in the user's order, the hour cycle and the width: two icons show the
//! first two places, three icons the first three; the panel shows them all.

use serde::{Deserialize, Serialize};

use super::WidgetError;
use super::clock::{HourCycle, is_iana_time_zone};

/// How many icons wide it may be.
pub const WIDTHS: std::ops::RangeInclusive<u8> = 2..=3;

/// How many places it may hold.
pub const MAX_ZONES: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorldClockConfig {
    pub width: u8,
    /// IANA names, such as `Europe/London`, in the order shown.
    pub zones: Vec<String>,
    pub hour_cycle: HourCycle,
}

impl Default for WorldClockConfig {
    fn default() -> Self {
        WorldClockConfig {
            width: 2,
            zones: vec!["Europe/London".to_owned(), "America/New_York".to_owned()],
            hour_cycle: HourCycle::H23,
        }
    }
}

impl WorldClockConfig {
    /// A width of `WIDTHS`, and one to `MAX_ZONES` places, each an IANA
    /// name (as the Clock's) and none twice.
    pub fn validate(&self) -> Result<(), WidgetError> {
        if !WIDTHS.contains(&self.width) {
            return Err(WidgetError::InvalidWidth);
        }
        if self.zones.is_empty() || self.zones.len() > MAX_ZONES {
            return Err(WidgetError::InvalidTimeZone);
        }
        for (at, zone) in self.zones.iter().enumerate() {
            if !is_iana_time_zone(zone) || self.zones[..at].contains(zone) {
                return Err(WidgetError::InvalidTimeZone);
            }
        }
        Ok(())
    }
}

/// A short place name for an IANA zone, as the interface's `zoneCity`: its
/// last part, with spaces (`America/Sao_Paulo` → `Sao Paulo`).
pub fn city(zone: &str) -> String {
    zone.rsplit('/').next().unwrap_or(zone).replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(zones: &[&str]) -> WorldClockConfig {
        WorldClockConfig {
            zones: zones.iter().map(|zone| (*zone).to_owned()).collect(),
            ..WorldClockConfig::default()
        }
    }

    #[test]
    fn the_default_is_two_places_two_icons_wide() {
        let config = WorldClockConfig::default();
        assert_eq!(config.validate(), Ok(()));
        assert_eq!(
            serde_json::to_value(&config).unwrap(),
            serde_json::json!({
                "width": 2,
                "zones": ["Europe/London", "America/New_York"],
                "hourCycle": "h23"
            })
        );
    }

    #[test]
    fn places_are_iana_names_one_to_six_none_twice() {
        assert_eq!(with(&["Asia/Tokyo"]).validate(), Ok(()));
        assert_eq!(with(&[]).validate(), Err(WidgetError::InvalidTimeZone));
        assert_eq!(
            with(&["Asia/Tokyo", "Asia/Tokyo"]).validate(),
            Err(WidgetError::InvalidTimeZone)
        );
        assert_eq!(
            with(&["asia/tokyo"]).validate(),
            Err(WidgetError::InvalidTimeZone)
        );
        assert_eq!(
            with(&["Factory"]).validate(),
            Err(WidgetError::InvalidTimeZone)
        );
        let seven = [
            "Asia/Tokyo",
            "Europe/London",
            "Europe/Paris",
            "America/New_York",
            "America/Sao_Paulo",
            "Australia/Sydney",
            "Asia/Kolkata",
        ];
        assert_eq!(with(&seven[..6]).validate(), Ok(()));
        assert_eq!(with(&seven).validate(), Err(WidgetError::InvalidTimeZone));
    }

    #[test]
    fn the_width_is_two_or_three_icons() {
        for (width, ok) in [(1, false), (2, true), (3, true), (4, false)] {
            let config = WorldClockConfig {
                width,
                ..WorldClockConfig::default()
            };
            assert_eq!(config.validate().is_ok(), ok, "{width}");
        }
    }

    #[test]
    fn a_city_is_the_last_part_with_spaces() {
        assert_eq!(city("America/Sao_Paulo"), "Sao Paulo");
        assert_eq!(city("America/Argentina/Buenos_Aires"), "Buenos Aires");
        assert_eq!(city("UTC"), "UTC");
    }
}

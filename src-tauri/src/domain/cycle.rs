//! The Ciclo (spec 0017 F2b): the menstrual cycle of the author's fiancée,
//! read from her calendar, where she marks each first day with an all-day
//! event (`🌸` by default). From the marks of the last year, the average
//! cycle and period, the day of the cycle, and the next start. Pure, and
//! kept in memory only: nothing of hers goes to the disk.

use serde::Serialize;

const DAY: i64 = 24 * 3600;

/// Without enough marks, a cycle of 28 days and a period of 5.
pub const DEFAULT_CYCLE: f64 = 28.0;
pub const DEFAULT_PERIOD: f64 = 5.0;
/// How many of the last cycles the average takes.
pub const AVERAGE_OF: usize = 6;
/// Gaps outside this many days are not cycles (a mark missed, or two marks
/// of one period) and are left out of the average.
const CYCLE_RANGE: std::ops::RangeInclusive<i64> = 15..=60;
/// A period is 1 to 10 days long.
const PERIOD_RANGE: std::ops::RangeInclusive<i64> = 1..=10;

/// A mark she made: the event's start and end, Unix seconds (all-day: local
/// midnights).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mark {
    pub start: i64,
    pub end: i64,
}

/// A period, as local day numbers (days since 1970-01-01, local).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    pub first: i64,
    /// Days long.
    pub days: i64,
}

/// The local day number of `at`.
pub fn day_of(at: i64, offset: i64) -> i64 {
    (at + offset).div_euclid(DAY)
}

/// The marks as periods, oldest first: marks that start within the same
/// period (the event in two calendars, or one per day) are one period.
pub fn periods(marks: &[Mark], offset: i64) -> Vec<Period> {
    let mut days: Vec<(i64, i64)> = marks
        .iter()
        .map(|mark| {
            let first = day_of(mark.start, offset);
            // An all-day event ends at the next midnight.
            let last = day_of(mark.end - 1, offset).max(first);
            (
                first,
                (last - first + 1).clamp(*PERIOD_RANGE.start(), *PERIOD_RANGE.end()),
            )
        })
        .collect();
    days.sort();
    let mut periods: Vec<Period> = Vec::new();
    for (first, length) in days {
        match periods.last_mut() {
            Some(last) if first < last.first + last.days.max(*CYCLE_RANGE.start() / 2) => {
                // The same period, marked again: the longest stretch counts.
                last.days = last.days.max(first + length - last.first);
            }
            _ => periods.push(Period {
                first,
                days: length,
            }),
        }
    }
    periods
}

/// The averages, from the last [`AVERAGE_OF`] cycles.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Averages {
    pub cycle: f64,
    pub period: f64,
    /// How many cycles the average took (0: the defaults).
    pub cycles: usize,
}

pub fn averages(periods: &[Period]) -> Averages {
    let gaps: Vec<i64> = periods
        .windows(2)
        .map(|pair| pair[1].first - pair[0].first)
        .filter(|gap| CYCLE_RANGE.contains(gap))
        .collect();
    let recent = &gaps[gaps.len().saturating_sub(AVERAGE_OF)..];
    let cycle = if recent.is_empty() {
        DEFAULT_CYCLE
    } else {
        recent.iter().sum::<i64>() as f64 / recent.len() as f64
    };
    // A period marked as one day only says when it started.
    let lengths: Vec<i64> = periods
        .iter()
        .rev()
        .take(AVERAGE_OF)
        .map(|period| period.days)
        .filter(|days| *days > 1)
        .collect();
    let period = if lengths.is_empty() {
        DEFAULT_PERIOD
    } else {
        lengths.iter().sum::<i64>() as f64 / lengths.len() as f64
    };
    Averages {
        cycle,
        period,
        cycles: recent.len(),
    }
}

/// Where the cycle is today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum Phase {
    /// No mark yet.
    Unknown,
    /// The period: its `day` (1 is the first).
    Period { day: i64 },
    /// The days before the next start, within the sensitive window: `in`
    /// days to it.
    Sensitive {
        #[serde(rename = "in")]
        days: i64,
    },
    /// The rest of the cycle: `in` days to the next start (0 or less: due,
    /// not marked yet).
    Between {
        #[serde(rename = "in")]
        days: i64,
    },
}

/// The cycle seen today.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: Phase,
    pub averages: Averages,
    /// Today's day of the cycle (1 on a first day); 0 without marks.
    pub day: i64,
    /// The last first day and the next one expected, local day numbers.
    pub last: Option<i64>,
    pub next: Option<i64>,
    /// How many periods were read.
    pub marks: usize,
}

/// The cycle on `today` (a local day number), with the sensitive window
/// `sensitive` days before the next start.
pub fn status(periods: &[Period], today: i64, sensitive: i64) -> Status {
    let averages = averages(periods);
    // A mark in the future (made ahead) doesn't start the cycle yet.
    let Some(last) = periods.iter().rev().find(|period| period.first <= today) else {
        return Status {
            phase: Phase::Unknown,
            averages,
            day: 0,
            last: None,
            next: None,
            marks: periods.len(),
        };
    };
    let day = today - last.first + 1;
    let next = last.first + averages.cycle.round() as i64;
    let to_next = next - today;
    let length = if last.days > 1 {
        last.days
    } else {
        averages.period.round() as i64
    };
    let phase = if day <= length {
        Phase::Period { day }
    } else if to_next <= sensitive && to_next > 0 {
        Phase::Sensitive { days: to_next }
    } else {
        Phase::Between { days: to_next }
    };
    Status {
        phase,
        averages,
        day,
        last: Some(last.first),
        next: Some(next),
        marks: periods.len(),
    }
}

/// A notice of the Ciclo (F2b-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Notice {
    /// The sensitive window opens today.
    Sensitive,
    /// The expected start is tomorrow.
    Eve,
    /// The expected start is today.
    Day,
    /// One of the author's reminders: its text.
    Reminder { text: String },
}

impl Notice {
    /// What tells this notice from the others of the day, for firing it
    /// once.
    pub fn key(&self) -> String {
        match self {
            Notice::Sensitive => "sensitive".into(),
            Notice::Eve => "eve".into(),
            Notice::Day => "day".into(),
            Notice::Reminder { text } => format!("reminder:{text}"),
        }
    }
}

/// The notices of `today` for a cycle seen as `status`, with the widget's
/// config: none without a prediction.
pub fn notices(
    status: &Status,
    config: &crate::domain::widgets::CycleConfig,
    today: i64,
) -> Vec<Notice> {
    use crate::domain::widgets::cycle::Anchor;
    let Some(next) = status.next.filter(|_| status.phase != Phase::Unknown) else {
        return Vec::new();
    };
    let opens = next - i64::from(config.sensitive);
    let mut notices = Vec::new();
    if config.notices.sensitive && today == opens {
        notices.push(Notice::Sensitive);
    }
    if config.notices.eve && today == next - 1 {
        notices.push(Notice::Eve);
    }
    if config.notices.day && today == next {
        notices.push(Notice::Day);
    }
    for reminder in &config.reminders {
        let anchor = match reminder.anchor {
            Anchor::Sensitive => opens,
            Anchor::Period => next,
        };
        if today == anchor + i64::from(reminder.days) {
            notices.push(Notice::Reminder {
                text: reminder.text.trim().to_owned(),
            });
        }
    }
    notices
}

/// What a day ahead is expected to be (spec 0031 §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum Expected {
    /// Inside the sensitive window before the next start.
    Sensitive,
    /// A day of the period: its day, or `None` while late, when the
    /// coming days may be it.
    Period { day: Option<i64> },
}

/// The days expected from `today` to the end of the next period, in
/// order and each once: the rest of the period going on, the sensitive
/// window and the next period. Late, today and the days after it of the
/// average period, as maybe the period. Empty without marks.
pub fn ahead(periods: &[Period], today: i64, sensitive: i64) -> Vec<(i64, Expected)> {
    let status = status(periods, today, sensitive);
    let (Some(last), Some(next)) = (status.last, status.next) else {
        return Vec::new();
    };
    let period = (status.averages.period.round() as i64).max(1);
    let mut days = Vec::new();
    if next < today {
        days.extend((today..today + period).map(|day| (day, Expected::Period { day: None })));
        return days;
    }
    if let Phase::Period { .. } = status.phase {
        let length = periods
            .iter()
            .find(|each| each.first == last)
            .map_or(
                period,
                |each| if each.days > 1 { each.days } else { period },
            );
        days.extend((today..last + length).map(|day| {
            (
                day,
                Expected::Period {
                    day: Some(day - last + 1),
                },
            )
        }));
    }
    let from = days.last().map_or(today, |(day, _)| day + 1);
    days.extend((from.max(next - sensitive)..next).map(|day| (day, Expected::Sensitive)));
    let from = days.last().map_or(today, |(day, _)| day + 1);
    days.extend((from.max(next)..next + period).map(|day| {
        (
            day,
            Expected::Period {
                day: Some(day - next + 1),
            },
        )
    }));
    days
}

/// How many days past the expected start, with no new mark.
pub fn late_by(status: &Status) -> Option<i64> {
    match status.phase {
        Phase::Between { days } if days < 0 => Some(-days),
        _ => None,
    }
}

/// The first expected day an event from `start` to `end` (exclusive, in
/// seconds) falls on, and what it is. An all-day event runs from local
/// midnight to local midnight, as Google gives it.
pub fn overlaps(
    start: i64,
    end: i64,
    offset: i64,
    ahead: &[(i64, Expected)],
) -> Option<(i64, Expected)> {
    let first = day_of(start, offset);
    let last = day_of(end - 1, offset).max(first);
    ahead
        .iter()
        .find(|(day, _)| (first..=last).contains(day))
        .copied()
}

/// An event that falls on an expected day (spec 0031 §3.4): the event, the
/// first such day, and what that day is expected to be.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coincidence {
    pub event: crate::domain::calendar::Event,
    pub day: i64,
    pub expected: Expected,
}

/// Google's calendars of holidays: their days are not plans.
fn holidays(calendar: &str) -> bool {
    calendar.contains("#holiday@group.v.calendar.google.com")
}

/// The `events` (in order) that fall on the days `ahead`, in the calendars
/// `shows` takes: not the holidays, nor the marks of `symbol`.
pub fn coinciding(
    events: &[crate::domain::calendar::Event],
    ahead: &[(i64, Expected)],
    offset: i64,
    symbol: &str,
    shows: impl Fn(&str) -> bool,
) -> Vec<Coincidence> {
    events
        .iter()
        .filter(|event| {
            shows(&event.calendar) && !holidays(&event.calendar) && !event.title.contains(symbol)
        })
        .filter_map(|event| {
            overlaps(event.start, event.end, offset, ahead).map(|(day, expected)| Coincidence {
                event: event.clone(),
                day,
                expected,
            })
        })
        .collect()
}

/// Whether today is an expected day (the sensitive window, the period, or
/// the days that may be it while late): the Calendário's tile has a dot
/// then (spec 0031 §3.5), whatever was seen.
pub fn in_phase(periods: &[Period], today: i64, sensitive: i64) -> bool {
    ahead(periods, today, sensitive)
        .first()
        .is_some_and(|(day, _)| *day == today)
}

/// The last day `ahead` reaches: how far the events must be read.
pub fn horizon_end(ahead: &[(i64, Expected)]) -> Option<i64> {
    ahead.last().map(|(day, _)| *day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::calendar::Event;

    const BRT: i64 = -3 * 3600;

    /// An all-day mark of `days` days starting on local day `first`.
    fn mark(first: i64, days: i64) -> Mark {
        Mark {
            start: first * DAY - BRT,
            end: (first + days) * DAY - BRT,
        }
    }

    #[test]
    fn marks_become_periods_and_repeats_count_once() {
        let marks = [
            mark(100, 5),
            mark(128, 1),
            mark(100, 1),
            mark(129, 1),
            mark(157, 4),
        ];
        assert_eq!(
            periods(&marks, BRT),
            [
                Period {
                    first: 100,
                    days: 5
                },
                Period {
                    first: 128,
                    days: 2
                },
                Period {
                    first: 157,
                    days: 4
                },
            ]
        );
    }

    #[test]
    fn the_averages_take_the_last_six_cycles_or_the_defaults() {
        assert_eq!(
            averages(&[]),
            Averages {
                cycle: 28.0,
                period: 5.0,
                cycles: 0
            }
        );
        let firsts = [0, 30, 59, 89, 118, 148, 177, 207];
        let periods: Vec<Period> = firsts
            .iter()
            .map(|&first| Period { first, days: 4 })
            .collect();
        let averages = averages(&periods);
        // The last six gaps: 29, 30, 29, 30, 29, 30.
        assert_eq!(averages.cycles, 6);
        assert!((averages.cycle - 29.5).abs() < 1e-9);
        assert_eq!(averages.period, 4.0);
        // A missed mark (a 60+ day gap) is not a cycle.
        let missed = [Period { first: 0, days: 5 }, Period { first: 70, days: 5 }];
        assert_eq!(averages_of(&missed).cycles, 0);
    }

    fn averages_of(periods: &[Period]) -> Averages {
        averages(periods)
    }

    #[test]
    fn the_phase_follows_the_day_of_the_cycle() {
        let periods = [
            Period {
                first: 100,
                days: 5,
            },
            Period {
                first: 128,
                days: 5,
            },
        ];
        // Cycle of 28; period of 5; sensitive window of 5 days.
        let on = |today| status(&periods, today, 5).phase;
        assert_eq!(on(128), Phase::Period { day: 1 });
        assert_eq!(on(132), Phase::Period { day: 5 });
        assert_eq!(on(133), Phase::Between { days: 23 });
        assert_eq!(on(151), Phase::Sensitive { days: 5 });
        assert_eq!(on(155), Phase::Sensitive { days: 1 });
        // Due and not marked: no warning, only the count.
        assert_eq!(on(156), Phase::Between { days: 0 });
        assert_eq!(on(158), Phase::Between { days: -2 });
        let today = status(&periods, 140, 5);
        assert_eq!(today.day, 13);
        assert_eq!(today.next, Some(156));
    }

    #[test]
    fn the_notices_fall_on_their_days() {
        use crate::domain::widgets::CycleConfig;
        use crate::domain::widgets::cycle::{Anchor, Reminder};
        let periods = [
            Period {
                first: 100,
                days: 5,
            },
            Period {
                first: 128,
                days: 5,
            },
        ];
        // Next start: 156; the window of 5 days opens on 151.
        let config = CycleConfig {
            reminders: vec![
                Reminder {
                    text: " Comprar chocolate ".into(),
                    anchor: Anchor::Sensitive,
                    days: -1,
                },
                Reminder {
                    text: "Paciência".into(),
                    anchor: Anchor::Period,
                    days: 1,
                },
            ],
            ..CycleConfig::default()
        };
        let on = |today| notices(&status(&periods, today, 5), &config, today);
        assert_eq!(
            on(150),
            [Notice::Reminder {
                text: "Comprar chocolate".into()
            }]
        );
        assert_eq!(on(151), [Notice::Sensitive]);
        assert_eq!(on(155), [Notice::Eve]);
        assert_eq!(on(156), [Notice::Day]);
        assert!(on(140).is_empty());
        // Turned off: only the reminders.
        let quiet = CycleConfig {
            notices: crate::domain::widgets::cycle::Notices {
                sensitive: false,
                eve: false,
                day: false,
                windows: true,
            },
            ..config.clone()
        };
        assert!(notices(&status(&periods, 151, 5), &quiet, 151).is_empty());
        // Without marks, nothing.
        assert!(notices(&status(&[], 151, 5), &config, 151).is_empty());
    }

    #[test]
    fn without_marks_nothing_is_guessed() {
        let none = status(&[], 100, 5);
        assert_eq!(none.phase, Phase::Unknown);
        // A mark ahead of today doesn't count yet.
        let ahead = status(
            &[Period {
                first: 120,
                days: 5,
            }],
            100,
            5,
        );
        assert_eq!(ahead.phase, Phase::Unknown);
    }

    #[test]
    fn a_one_day_mark_lasts_the_average_period() {
        let periods = [
            Period {
                first: 100,
                days: 6,
            },
            Period {
                first: 128,
                days: 1,
            },
        ];
        // Only the 6-day period says how long they last.
        assert_eq!(status(&periods, 133, 5).phase, Phase::Period { day: 6 });
        assert_eq!(status(&periods, 134, 5).phase, Phase::Between { days: 22 });
    }

    /// Two marks 28 days apart, of 5 days: next start 156.
    fn two_cycles() -> [Period; 2] {
        [
            Period {
                first: 100,
                days: 5,
            },
            Period {
                first: 128,
                days: 5,
            },
        ]
    }

    fn predicted(from: i64, to: i64) -> Vec<(i64, Expected)> {
        (from..=to)
            .map(|day| {
                (
                    day,
                    Expected::Period {
                        day: Some(day - from + 1),
                    },
                )
            })
            .collect()
    }

    fn sensitive_days(from: i64, to: i64) -> Vec<(i64, Expected)> {
        (from..=to).map(|day| (day, Expected::Sensitive)).collect()
    }

    #[test]
    fn ahead_runs_to_the_end_of_the_next_period() {
        let periods = two_cycles();
        // Between: the window (151–155), then the period (156–160).
        let mut between = sensitive_days(151, 155);
        between.extend(predicted(156, 160));
        assert_eq!(ahead(&periods, 140, 5), between);
        // Inside the window: from today on.
        let mut inside = sensitive_days(153, 155);
        inside.extend(predicted(156, 160));
        assert_eq!(ahead(&periods, 153, 5), inside);
        // On the day expected, still the prediction.
        assert_eq!(ahead(&periods, 156, 5), predicted(156, 160));
    }

    #[test]
    fn ahead_keeps_the_rest_of_the_period_going_on() {
        // Day 3 of the period that began on 128: 130–132 are its days 3–5.
        let mut expected: Vec<(i64, Expected)> = (130..=132)
            .map(|day| {
                (
                    day,
                    Expected::Period {
                        day: Some(day - 127),
                    },
                )
            })
            .collect();
        expected.extend(sensitive_days(151, 155));
        expected.extend(predicted(156, 160));
        assert_eq!(ahead(&two_cycles(), 130, 5), expected);
    }

    #[test]
    fn late_the_coming_days_may_be_the_period() {
        let periods = two_cycles();
        // Two days past 156 with no mark: today and the 4 after, day unknown.
        let maybe: Vec<(i64, Expected)> = (158..=162)
            .map(|day| (day, Expected::Period { day: None }))
            .collect();
        assert_eq!(ahead(&periods, 158, 5), maybe);
        assert_eq!(late_by(&status(&periods, 158, 5)), Some(2));
        assert_eq!(late_by(&status(&periods, 156, 5)), None);
        assert_eq!(late_by(&status(&periods, 140, 5)), None);
        assert_eq!(late_by(&status(&[], 140, 5)), None);
    }

    #[test]
    fn ahead_without_marks_is_empty_and_never_repeats_a_day() {
        assert!(ahead(&[], 140, 5).is_empty());
        // Short cycle (15), long period (10) and window (10): the window
        // would start inside the period going on, and starts after it.
        let tight = [
            Period {
                first: 100,
                days: 10,
            },
            Period {
                first: 115,
                days: 10,
            },
        ];
        let days: Vec<i64> = ahead(&tight, 116, 10).iter().map(|(day, _)| *day).collect();
        assert_eq!(days, (116..=139).collect::<Vec<i64>>());
        assert_eq!(ahead(&tight, 116, 10)[9], (125, Expected::Sensitive));
    }

    #[test]
    fn an_event_takes_the_first_expected_day_it_falls_on() {
        let days = ahead(&two_cycles(), 140, 5);
        let hour = 3600;
        let at = |day: i64, hours: i64| day * DAY + hours * hour - BRT;
        // A meeting on 153, 14h to 15h.
        assert_eq!(
            overlaps(at(153, 14), at(153, 15), BRT, &days),
            Some((153, Expected::Sensitive))
        );
        // An all-day trip from 150 to 157: its first expected day is 151.
        let trip = mark(150, 8);
        assert_eq!(
            overlaps(trip.start, trip.end, BRT, &days),
            Some((151, Expected::Sensitive))
        );
        // Past midnight, from 160 22h to 161 2h: the 5th day.
        assert_eq!(
            overlaps(at(160, 22), at(161, 2), BRT, &days),
            Some((160, Expected::Period { day: Some(5) }))
        );
        // The end is exclusive: 150 23h to 151 0h stays on 150.
        assert_eq!(overlaps(at(150, 23), at(151, 0), BRT, &days), None);
        // Nothing expected that day.
        assert_eq!(overlaps(at(141, 9), at(141, 10), BRT, &days), None);
    }

    fn event(id: &str, calendar: &str, title: &str, start: i64, end: i64) -> Event {
        Event {
            id: id.into(),
            calendar: calendar.into(),
            calendar_name: calendar.into(),
            color: None,
            title: title.into(),
            start,
            end,
            all_day: false,
            location: None,
            description: None,
            join: None,
            link: None,
            busy: false,
            key: id.into(),
        }
    }

    #[test]
    fn the_events_on_expected_days_coincide_in_order() {
        let days = ahead(&two_cycles(), 140, 5);
        let at = |day: i64, hours: i64| day * DAY + hours * 3600 - BRT;
        let trip = mark(150, 8);
        let events = [
            event("past", "max", "Dentista", at(139, 9), at(139, 10)),
            event("free", "max", "Reunião", at(141, 9), at(141, 10)),
            event("trip", "noiva", "Viagem", trip.start, trip.end),
            event("hidden", "work", "Daily", at(152, 9), at(152, 10)),
            event(
                "holiday",
                "pt-br.brazilian#holiday@group.v.calendar.google.com",
                "Feriado",
                at(153, 0),
                at(154, 0),
            ),
            event("mark", "noiva", "🌸", at(156, 0), at(157, 0)),
            event("dinner", "max", "Jantar", at(157, 20), at(157, 22)),
        ];
        let shows = |calendar: &str| calendar != "work";
        let found: Vec<(&str, i64, Expected)> = coinciding(&events, &days, BRT, "🌸", shows)
            .into_iter()
            .map(|each| {
                (
                    events
                        .iter()
                        .find(|e| e.id == each.event.id)
                        .unwrap()
                        .id
                        .as_str(),
                    each.day,
                    each.expected,
                )
            })
            .collect();
        assert_eq!(
            found,
            [
                ("trip", 151, Expected::Sensitive),
                ("dinner", 157, Expected::Period { day: Some(2) }),
            ]
        );
    }

    #[test]
    fn the_horizon_ends_with_the_last_expected_day() {
        assert_eq!(horizon_end(&ahead(&two_cycles(), 140, 5)), Some(160));
        assert_eq!(horizon_end(&[]), None);
    }

    #[test]
    fn the_dot_shows_on_every_expected_day() {
        let periods = two_cycles();
        // Between: no dot.
        assert!(!in_phase(&periods, 140, 5));
        // The window, the day due and the period going on.
        assert!(in_phase(&periods, 151, 5));
        assert!(in_phase(&periods, 155, 5));
        assert!(in_phase(&periods, 156, 5));
        assert!(in_phase(&periods, 130, 5));
        assert!(!in_phase(&periods, 133, 5));
        // Late, the days that may be the period.
        assert!(in_phase(&periods, 158, 5));
        assert!(!in_phase(&[], 140, 5));
    }
}

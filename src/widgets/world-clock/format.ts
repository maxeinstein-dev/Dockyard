import type { HourCycle } from "../../core/widgets";
import { activeLocale } from "../../i18n";
import { formatClock, type Instant } from "../clock/format";
import { utcOffsetMinutes, zoneCity } from "../clock/zones";

/* What the Relógio mundial shows of one place (F9 of estudo 0002). Pure,
 * through `Intl` only, as the Clock's. */

/** One place at one instant. */
export interface PlaceReading {
  timeZone: string;
  /** `Sao Paulo`, as the Clock names a zone. */
  city: string;
  /** `HH:mm`, or `h:mm` in 12 h. */
  time: string;
  /** AM/PM in 12 h; null in 24 h. */
  dayPeriod: string | null;
  /** The place's day against the user's: −1 (yesterday there), 0 or +1. */
  day: -1 | 0 | 1;
  /** Minutes the place is ahead of the user (negative: behind). */
  ahead: number;
}

/** The calendar day of `instant` in `timeZone` (the user's without one),
 * as a number that only compares: `20261104`. */
function dayNumber(instant: Instant, timeZone?: string): number {
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(instant);
  const part = (type: string) =>
    Number(parts.find((each) => each.type === type)?.value ?? 0);
  return part("year") * 10_000 + part("month") * 100 + part("day");
}

/** `timeZone` at `instant`, against the user's own time. A zone this
 * WebView doesn't know reads as the user's (the Clock's fallback). */
export function readPlace(
  instant: Instant,
  timeZone: string,
  hourCycle: HourCycle,
  locale: string = activeLocale(),
): PlaceReading {
  const reading = formatClock(
    instant,
    { timeZone, hourCycle, showSeconds: false, iconStyle: "digital" },
    locale,
  );
  const known = !reading.timeZoneFallback;
  const there = known ? dayNumber(instant, timeZone) : dayNumber(instant);
  const here = dayNumber(instant);
  const date = typeof instant === "number" ? new Date(instant) : instant;
  const ahead = known
    ? utcOffsetMinutes(instant, timeZone) + date.getTimezoneOffset()
    : 0;
  return {
    timeZone,
    city: zoneCity(timeZone),
    time: reading.icon,
    dayPeriod: reading.dayPeriod,
    day: there === here ? 0 : there > here ? 1 : -1,
    ahead,
  };
}

/** The difference as people say it: `+3 h`, `−2h30`, with a true minus. */
export function aheadText(minutes: number): string {
  const sign = minutes < 0 ? "−" : "+";
  const whole = Math.abs(minutes);
  const hours = Math.floor(whole / 60);
  const rest = whole % 60;
  return rest === 0
    ? `${sign}${hours} h`
    : `${sign}${hours}h${String(rest).padStart(2, "0")}`;
}

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";
import type { CalendarEvent } from "./calendar";
import type { CalendarConfig } from "./widgets";

/* The Calendário's section Ciclo (spec 0031): where the cycle is, computed
 * in Rust from the marks in her calendar (`domain/cycle.rs`); the marks
 * never reach the page. Must match `CycleView` in `commands/calendar.rs`. */

export type CyclePhase =
  | { phase: "unknown" }
  | { phase: "period"; day: number }
  | { phase: "sensitive"; in: number }
  | { phase: "between"; in: number };

export interface CycleStatus {
  phase: CyclePhase;
  averages: { cycle: number; period: number; cycles: number };
  /** Today's day of the cycle; 0 without marks. */
  day: number;
  /** Local day numbers (days since 1970-01-01). */
  last: number | null;
  next: number | null;
  marks: number;
}

/** A notice of today (F2b-2). Must match `Notice` in `domain/cycle.rs`. */
export type CycleNotice =
  | { kind: "sensitive" }
  | { kind: "eve" }
  | { kind: "day" }
  | { kind: "reminder"; text: string };

/** What a day ahead is expected to be: `day` null while late, when the
 * coming days may be the period. Must match `Expected` in Rust. */
export type Expected =
  { phase: "sensitive" } | { phase: "period"; day: number | null };

/** An event on an expected day: the first such day (a local day number). */
export interface Coincidence {
  event: CalendarEvent;
  day: number;
  expected: Expected;
}

export interface CycleView {
  /** null until the marks were read. */
  status: CycleStatus | null;
  today: number;
  accounts: boolean;
  /** Today's notices and reminders; the Windows notice never says them. */
  notices: CycleNotice[];
  /** Days past the expected start with no new mark. */
  late: number | null;
  /** The events of the shown calendars on the days expected, in order. */
  coinciding: Coincidence[];
}

/** What became of a test notice: `off` when Windows has the notices off,
 * where it would take the notice and show nothing. */
export type TestNotice = "sent" | "off" | "failed";

/** Gives the Ciclo `widget` a notice now, to see it: the badge and the
 * bounce on the dock, and the Windows notice when `windows`. */
export function cycleTestNotice(
  widget: string,
  windows: boolean,
): Promise<TestNotice> {
  return invoke<void>("cycle_test_notice", { widget, windows }).then(
    () => "sent" as const,
    (error: unknown) => {
      if (error === "notificationsOff") return "off" as const;
      log.warn(`cycle: test notice failed: ${errorKind(error)}`);
      return "failed" as const;
    },
  );
}

/** How often an open panel asks again. */
export const CYCLE_EVERY_MS = 60_000;

export function cycleStatus(config: CalendarConfig): Promise<CycleView> {
  return invoke<CycleView>("cycle_status", { config });
}

/** The cycle of the Calendário `config`, asked again every minute while
 * mounted; null, asking nothing, while its cycle is off. */
export function useCycle(config: CalendarConfig): CycleView | null {
  const [view, setView] = useState<CycleView | null>(null);
  const on = config.cycle !== null;
  const key = JSON.stringify(config);
  useEffect(() => {
    if (!on) return;
    let active = true;
    const ask = () => {
      cycleStatus(config).then(
        (next) => {
          if (active) setView(next);
        },
        (error: unknown) => {
          log.warn(`cycle: read failed: ${errorKind(error)}`);
        },
      );
    };
    ask();
    const every = window.setInterval(ask, CYCLE_EVERY_MS);
    return () => {
      active = false;
      window.clearInterval(every);
    };
    // The config is the key: a new object with the same values asks nothing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return on ? view : null;
}

/** `09/11`: the date of a local day number. */
export function dayText(day: number): string {
  const date = new Date(day * 24 * 3600 * 1000);
  const dd = String(date.getUTCDate()).padStart(2, "0");
  const mm = String(date.getUTCMonth() + 1).padStart(2, "0");
  return `${dd}/${mm}`;
}

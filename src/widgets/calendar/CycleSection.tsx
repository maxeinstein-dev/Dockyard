import {
  dayText,
  type Coincidence,
  type CycleNotice,
  type CycleStatus,
  type CycleView,
} from "../../core/cycle";
import type { CalendarEvent } from "../../core/calendar";
import { format, plural, useT, type Messages } from "../../i18n";

/** The sentence of where the cycle is. */
function phaseText(t: Messages, status: CycleStatus, late: number | null) {
  const text = t.widgets.cycle;
  const phase = status.phase;
  if (late !== null) return plural(t.widgets.calendar.cycle.late, late);
  switch (phase.phase) {
    case "unknown":
      return text.unknown;
    case "period":
      return format(text.period, { day: phase.day });
    case "sensitive":
      return plural(text.sensitive, phase.in);
    case "between":
      return phase.in <= 0 ? text.due : plural(text.between, phase.in);
  }
}

/** The line of a notice of today. */
function noticeText(t: Messages, notice: CycleNotice): string {
  return notice.kind === "reminder"
    ? notice.text
    : t.widgets.cycle.notices[notice.kind];
}

/** What an expected day is: TPM, the period's day, or maybe the period. */
function expectedText(t: Messages, each: Coincidence): string {
  const text = t.widgets.calendar.cycle.expected;
  const expected = each.expected;
  if (expected.phase === "sensitive") return text.sensitive;
  return expected.day === null
    ? text.maybe
    : format(text.period, { day: expected.day });
}

/**
 * The Calendário's section Ciclo (spec 0031 §3.4): where the cycle is, the
 * notices of today, the events of the calendars shown that fall on the
 * days expected (each opens its details) and the next start expected.
 */
function CycleSection({
  view,
  symbol,
  onOpen,
}: {
  view: CycleView | null;
  symbol: string;
  onOpen: (event: CalendarEvent, day: number) => void;
}) {
  const t = useT();
  const text = t.widgets.calendar.cycle;
  const cycleText = t.widgets.cycle;
  const status = view?.status ?? null;
  return (
    <section className="cal-day cal-cycle-now" aria-label={text.title}>
      <h3 className="cal-day-name">{text.title}</h3>
      {view && !view.accounts ? (
        <p className="cal-empty">{cycleText.noAccounts}</p>
      ) : status === null ? (
        <p className="cal-empty">{cycleText.loading}</p>
      ) : (
        <>
          <p className="cal-cycle-phase">
            {phaseText(t, status, view?.late ?? null)}
          </p>
          {view && view.notices.length > 0 ? (
            <ul className="cal-cycle-notices" aria-label={cycleText.today}>
              {view.notices.map((notice) => (
                <li
                  key={notice.kind === "reminder" ? notice.text : notice.kind}
                >
                  {noticeText(t, notice)}
                </li>
              ))}
            </ul>
          ) : null}
          {view && view.coinciding.length > 0 ? (
            <ul className="cal-list" aria-label={text.coincide}>
              {view.coinciding.map((each) => (
                <li key={each.event.id + each.event.calendar}>
                  <button
                    type="button"
                    className="cal-coincide"
                    onClick={() => onOpen(each.event, each.day)}
                  >
                    <span className="cal-time">{dayText(each.day)}</span>
                    <span className="cal-title">
                      {each.event.title ||
                        (each.event.busy
                          ? t.widgets.calendar.busy
                          : t.widgets.calendar.noTitle)}
                    </span>
                    <span className="cal-coincide-phase">
                      {expectedText(t, each)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="cal-empty">{text.nothing}</p>
          )}
          {status.next !== null ? (
            <p className="cal-cycle-next">
              {cycleText.next}: {dayText(status.next)}
            </p>
          ) : null}
          <p className="sys-hint">
            {status.averages.cycles === 0
              ? format(cycleText.defaults, { symbol })
              : plural(cycleText.basedOn, status.averages.cycles)}
          </p>
        </>
      )}
    </section>
  );
}

export default CycleSection;

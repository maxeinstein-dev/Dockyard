import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "../system/system.css";
import { aheadText, readPlace } from "./format";

/** The main view of a Relógio mundial panel (F9 of estudo 0002): every
 * place, in the user's order, with its time, its day when it is not the
 * user's, and how far ahead or behind it is. */
function WorldClockPanel({
  instance,
  now,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"world-clock">) {
  const t = useT();
  const text = t.widgets.worldClock;
  const { zones, hourCycle } = instance.config;
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <ul className="sys-list" aria-label={text.listLabel}>
        {zones.map((zone) => {
          const place = readPlace(now, zone, hourCycle);
          const day =
            place.day === 1
              ? text.dayAfter
              : place.day === -1
                ? text.dayBefore
                : null;
          const offset =
            place.ahead === 0
              ? text.sameTime
              : format(text.offset, { offset: aheadText(place.ahead) });
          return (
            <li key={zone}>
              <span className="sys-name">{place.city}</span>
              <span className="sys-figure">
                {place.time}
                {place.dayPeriod ? ` ${place.dayPeriod}` : ""}
              </span>
              <span className="sys-detail">
                {day ? `${day} · ${offset}` : offset}
              </span>
            </li>
          );
        })}
      </ul>
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default WorldClockPanel;

import { useState } from "react";
import {
  HOUR_CYCLES,
  MAX_WORLD_ZONES,
  WORLD_CLOCK_WIDTHS,
  type HourCycle,
} from "../../core/widgets";
import { format, useT } from "../../i18n";
import OptionGroup from "../../panel/OptionGroup";
import PanelFooter from "../../panel/PanelFooter";
import IconButton from "../../panel/parts/IconButton";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import SelectField from "../../panel/parts/SelectField";
import TextField from "../../panel/parts/TextField";
import {
  ianaTimeZones,
  utcOffset,
  zoneCity,
  zonesByOffset,
} from "../clock/zones";
import type { WidgetSettingsProps } from "../contract";
import { WidthGroup } from "../system/parts";
import { useSystemSettings } from "../system/useSettings";
import { findZones, foundName } from "./search";
import "./world-clock.css";

/** The add list's value before a zone is picked. */
const NONE = "";

/** The "Ajustes" view of a Relógio mundial panel (F9 of estudo 0002):
 * label, the places (added from the list of zones, moved and removed),
 * width, the hour's format and "Remover do dock". Each change is saved at
 * once. */
function WorldClockSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"world-clock">) {
  const t = useT();
  const text = t.widgets.worldClock;
  /** When the view opened: the offsets of the list are read at it. */
  const [now] = useState(() => new Date());
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  const { config } = settings.shown;
  const zones = config.zones;
  const full = zones.length >= MAX_WORLD_ZONES;

  function move(at: number, by: -1 | 1) {
    const next = [...zones];
    const [zone] = next.splice(at, 1);
    if (zone === undefined) return;
    next.splice(at + by, 0, zone);
    settings.saveConfig({ zones: next });
  }

  const [query, setQuery] = useState("");
  const found = findZones(
    query,
    ianaTimeZones().filter((zone) => !zones.includes(zone)),
  );

  const addable = zonesByOffset(
    now,
    ianaTimeZones().filter((zone) => !zones.includes(zone)),
  ).map(({ timeZone, offset }) => ({
    value: timeZone,
    label: `${zoneCity(timeZone)} (${offset})`,
  }));

  return (
    <>
      <PanelHeader title={text.settingsTitle} onBack={onBack} focusAction />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds["world-clock"]}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <p className="world-places-title">{text.places}</p>
      <ul className="world-places" aria-label={text.places}>
        {zones.map((zone, at) => {
          const city = zoneCity(zone);
          return (
            <li key={zone}>
              <span className="world-place-name">{city}</span>
              <span className="world-place-offset">{utcOffset(now, zone)}</span>
              <IconButton
                icon="up"
                label={format(text.moveUp, { city })}
                disabled={at === 0}
                onClick={() => move(at, -1)}
              />
              <IconButton
                icon="down"
                label={format(text.moveDown, { city })}
                disabled={at === zones.length - 1}
                onClick={() => move(at, 1)}
              />
              <IconButton
                icon="trash"
                label={format(text.remove, { city })}
                disabled={zones.length <= 1}
                onClick={() =>
                  settings.saveConfig({
                    zones: zones.filter((each) => each !== zone),
                  })
                }
              />
            </li>
          );
        })}
      </ul>
      <p className="sys-hint">{text.placesHint}</p>
      {full ? (
        <p className="sys-hint">
          {format(text.full, { max: MAX_WORLD_ZONES })}
        </p>
      ) : (
        <>
          <TextField
            label={text.search}
            placeholder={text.searchPlaceholder}
            value={query}
            onChange={setQuery}
          />
          {query.trim() === "" ? null : found.length === 0 ? (
            <p className="sys-hint" role="status">
              {text.nothingFound}
            </p>
          ) : (
            <ul className="world-found" aria-label={text.found}>
              {found.map((zone) => (
                <li key={zone}>
                  <button
                    type="button"
                    className="world-found-zone"
                    onClick={() => {
                      settings.saveConfig({ zones: [...zones, zone] });
                      setQuery("");
                    }}
                  >
                    <span className="world-place-name">
                      {foundName(query, zone)}
                    </span>
                    <span className="world-place-offset">
                      {utcOffset(now, zone)}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          <SelectField
            label={text.add}
            value={NONE}
            options={[{ value: NONE, label: text.addPlaceholder }, ...addable]}
            onChange={(zone) => {
              if (zone !== NONE)
                settings.saveConfig({ zones: [...zones, zone] });
            }}
          />
        </>
      )}
      <WidthGroup
        width={config.width}
        widths={WORLD_CLOCK_WIDTHS}
        onSelect={(width) => settings.saveConfig({ width })}
      />
      <OptionGroup<HourCycle>
        label={t.widgets.clock.hourCycle}
        options={HOUR_CYCLES.map((value) => ({
          value,
          label: t.widgets.clock.hourCycles[value],
        }))}
        selected={config.hourCycle}
        onSelect={(hourCycle) => settings.saveConfig({ hourCycle })}
      />
      <RemoveButton
        onClick={() => void settings.remove()}
        disabled={settings.removing}
      />
      <PanelFooter
        alert={settings.error ? t.widgets.errors[settings.error] : null}
      >
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default WorldClockSettings;

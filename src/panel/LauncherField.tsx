import { useState } from "react";
import {
  DEFAULT_HOTKEY,
  clearLauncherHistory,
  hotkeyLabel,
  useLauncherStatus,
} from "../core/launcher";
import { format, useT } from "../i18n";
import HotkeyField from "./HotkeyField";
import InlineNotice from "./parts/InlineNotice";
import ToggleField from "./parts/ToggleField";
import "./parts/parts.css";

type LauncherFieldProps = {
  /** Whether the hotkey opens the launcher (`Settings.launcher`); null
   * while the settings load. */
  on: boolean | null;
  /** What opens it (`Settings.launcherHotkey`); null while the settings
   * load. */
  hotkey?: string | null;
  onChange: (on: boolean) => void;
  /** Shows the history's row under the hotkey; off in the Configurações
   * app, which puts it in Lançador › Avançado (spec 0029 §2.1). */
  withHistory?: boolean;
};

/** Forgetting what the launcher opened: a row with its button. */
export function LauncherHistoryField() {
  const text = useT().settings.launcher;
  const [cleared, setCleared] = useState(false);
  return (
    <div className="toggle-row">
      <div className="toggle-text">
        <span className="toggle-label">{text.history}</span>
        <span className="toggle-hint">{text.historyHint}</span>
      </div>
      <button
        type="button"
        className="secondary-button"
        disabled={cleared}
        onClick={() => {
          void clearLauncherHistory().then(setCleared);
        }}
      >
        {cleared ? text.cleared : text.clear}
      </button>
    </div>
  );
}

/**
 * The launcher's place in Configurações › Lançador (spec 0012; spec 0029): the switch of
 * its hotkey, a notice when another program holds it (the launcher then
 * opens from the dock's menu), the hotkey itself (L3), and forgetting what
 * it opened.
 */
function LauncherField({
  on,
  onChange,
  hotkey = null,
  withHistory = true,
}: LauncherFieldProps) {
  const t = useT();
  const text = t.settings.launcher;
  const status = useLauncherStatus();
  const shown = hotkeyLabel(hotkey ?? DEFAULT_HOTKEY, text.hotkey.space);
  return (
    <>
      <ToggleField
        label={format(text.label, { hotkey: shown })}
        hint={text.hint}
        checked={on ?? true}
        disabled={on === null}
        onChange={onChange}
      />
      {on && status?.hotkeyTaken ? (
        <InlineNotice
          title={format(text.taken, { hotkey: shown })}
          detail={text.takenDetail}
        />
      ) : null}
      {on && hotkey !== null ? <HotkeyField hotkey={hotkey} /> : null}
      {withHistory ? <LauncherHistoryField /> : null}
    </>
  );
}

export default LauncherField;

import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  DEFAULT_DOCK_HOTKEY,
  DEFAULT_HOTKEY,
  HOTKEY_PRESSED_EVENT,
  captureDockHotkey,
  captureLauncherHotkey,
  hotkeyFromEvent,
  hotkeyLabel,
  setDockHotkey,
  setLauncherHotkey,
  type HotkeyErrorCode,
} from "../core/launcher";
import { format, useT } from "../i18n";
import "./parts/parts.css";

type HotkeyFieldProps = {
  /** The hotkey in use (`Settings.launcherHotkey`, or `dockHotkey`). */
  hotkey: string;
  disabled?: boolean;
  /** Whose hotkey: the launcher's (the default) or the dock's (M14). */
  of?: "launcher" | "dock";
};

/** How each hotkey is changed and captured, and what it is unless changed. */
const KINDS = {
  launcher: {
    fallback: DEFAULT_HOTKEY,
    save: setLauncherHotkey,
    capture: captureLauncherHotkey,
  },
  dock: {
    fallback: DEFAULT_DOCK_HOTKEY,
    save: setDockHotkey,
    capture: captureDockHotkey,
  },
} as const;

type State =
  | { kind: "idle" }
  | { kind: "capturing" }
  | { kind: "saving" }
  | { kind: "saved"; hotkey: string }
  | { kind: "refused"; code: HotkeyErrorCode };

/**
 * The launcher's hotkey in Configurações › Lançador (spec 0012 L3; spec 0029): a button
 * that shows it and, pressed, captures the next combination. While it
 * captures, Rust lets go of the current hotkey (or pressing it would open
 * the launcher); Esc or leaving the button cancels. Rust tries the new one
 * for real before saving it, and says why when it can't. The dock's hotkey
 * (M14) works the same way, without the try: Rust logs when another program
 * holds it.
 */
function HotkeyField({
  hotkey,
  disabled = false,
  of = "launcher",
}: HotkeyFieldProps) {
  const t = useT();
  const text =
    of === "dock" ? t.settings.dockHotkey : t.settings.launcher.hotkey;
  const { fallback, save: saveHotkey, capture: captureHotkey } = KINDS[of];
  const hintId = useId();
  const statusId = useId();
  const [state, setState] = useState<State>({ kind: "idle" });
  const capturing = useRef(false);
  const label = (value: string) => hotkeyLabel(value, text.space);

  function stopCapture() {
    if (!capturing.current) return;
    capturing.current = false;
    captureHotkey(false);
  }

  // Leaving Configurações in the middle gives the hotkey back.
  useEffect(
    () => () => {
      if (capturing.current) captureHotkey(false);
    },
    [captureHotkey],
  );

  function start() {
    capturing.current = true;
    captureHotkey(true);
    setState({ kind: "capturing" });
  }

  function cancel() {
    stopCapture();
    setState({ kind: "idle" });
  }

  function save(wanted: string) {
    setState({ kind: "saving" });
    void saveHotkey(wanted).then((code) => {
      // Rust ended the capture: the new hotkey, or the one before it, is
      // held again.
      capturing.current = false;
      setState(
        code === null
          ? { kind: "saved", hotkey: wanted }
          : { kind: "refused", code },
      );
    });
  }

  // Windows turns Alt+Space into the window's menu before the page sees
  // it: while the capture runs, Rust hands it here instead.
  const capturingNow = state.kind === "capturing";
  const saveRef = useRef(save);
  useEffect(() => {
    saveRef.current = save;
  });
  useEffect(() => {
    if (!capturingNow) return;
    let active = true;
    const unlisten = listen<string>(HOTKEY_PRESSED_EVENT, (event) => {
      if (active) saveRef.current(event.payload);
    });
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, [capturingNow]);

  function onKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (state.kind !== "capturing") return;
    const plain =
      !event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;
    if (plain && event.key === "Tab") {
      cancel();
      return;
    }
    // Every other key is the capture's: not the page's, not Windows' (the
    // window's menu on Alt+Space), not Esc closing Configurações.
    event.preventDefault();
    event.stopPropagation();
    if (plain && event.key === "Escape") {
      cancel();
      return;
    }
    const pressed = hotkeyFromEvent(event);
    if (pressed === "modifier") return;
    if (pressed === "unsupported") {
      stopCapture();
      setState({ kind: "refused", code: "unsupportedKey" });
      return;
    }
    save(pressed);
  }

  const status =
    state.kind === "saving"
      ? text.saving
      : state.kind === "saved"
        ? format(text.saved, { hotkey: label(state.hotkey) })
        : null;

  return (
    <div className="hotkey-field">
      <div className="toggle-row">
        <div className="toggle-text">
          <span className="toggle-label">{text.label}</span>
          <span id={hintId} className="toggle-hint">
            {text.hint}
          </span>
        </div>
        <button
          type="button"
          className="secondary-button hotkey-button"
          aria-label={
            state.kind === "capturing"
              ? text.press
              : format(text.change, { hotkey: label(hotkey) })
          }
          aria-describedby={hintId}
          aria-pressed={state.kind === "capturing"}
          disabled={disabled || state.kind === "saving"}
          onClick={() => {
            if (state.kind !== "capturing") start();
          }}
          onKeyDown={onKeyDown}
          onBlur={() => {
            if (state.kind === "capturing") cancel();
          }}
        >
          {state.kind === "capturing" ? text.press : label(hotkey)}
        </button>
      </div>
      <p id={statusId} className="toggle-hint" role="status">
        {status}
      </p>
      {state.kind === "refused" ? (
        <p className="field-error" role="alert">
          {text.errors[state.code]}
        </p>
      ) : null}
      {hotkey !== fallback && state.kind !== "capturing" ? (
        <button
          type="button"
          className="quiet-button"
          disabled={disabled || state.kind === "saving"}
          onClick={() => save(fallback)}
        >
          {text.restore}
        </button>
      ) : null}
    </div>
  );
}

export default HotkeyField;

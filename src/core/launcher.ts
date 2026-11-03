import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { errorKind, log } from "./log";

/* The launcher (spec 0012): a native window Rust opens with its hotkey.
 * Configurações switches it, says when another program holds the hotkey,
 * forgets what it opened, and changes the hotkey (L3). */

/** What Configurações shows about the launcher; must match `LauncherStatus`
 * in `src-tauri/src/commands/launcher.rs`. */
export interface LauncherStatus {
  /** Another program holds the hotkey: the launcher opens only from the
   * menus ("Buscar apps…"). */
  hotkeyTaken: boolean;
}

/** Event the windows receive when the hotkey's state changes. */
export const LAUNCHER_STATUS_EVENT = "launcher-status";

export const LAUNCHER_STATUS_COMMAND = "launcher_status";
export const CLEAR_LAUNCHER_HISTORY_COMMAND = "clear_launcher_history";
export const SET_LAUNCHER_HOTKEY_COMMAND = "set_launcher_hotkey";
export const CAPTURE_LAUNCHER_HOTKEY_COMMAND = "capture_launcher_hotkey";
/** The dock's hotkey (M14), which takes the keyboard to the dock: changed
 * and captured as the launcher's. */
export const SET_DOCK_HOTKEY_COMMAND = "set_dock_hotkey";
export const CAPTURE_DOCK_HOTKEY_COMMAND = "capture_dock_hotkey";

/** Rust's word that Alt+Space was pressed while the hotkey is captured:
 * Windows opens the window's menu with it before the page sees the key. */
export const HOTKEY_PRESSED_EVENT = "launcher-hotkey-pressed";

/** The hotkey unless the user changes it. */
export const DEFAULT_HOTKEY = "Alt+Space";

/** The dock's unless the user changes it: Win+Alt+Z, written in the order
 * Rust keeps (`Ctrl`, `Alt`, `Shift`, `Win`). */
export const DEFAULT_DOCK_HOTKEY = "Alt+Win+Z";

/** Why a new hotkey was refused; must match `HotkeyErrorCode` in
 * `src-tauri/src/commands/launcher.rs`, plus `unsupportedKey` (a key Rust
 * never sees: the field refuses it). */
export type HotkeyErrorCode =
  | "malformed"
  | "noModifier"
  | "takesTyping"
  | "reserved"
  | "taken"
  /** It is the other hotkey's: the dock's for the launcher, the
   * launcher's for the dock. */
  | "clash"
  | "notSaved"
  | "unsupportedKey";

const HOTKEY_ERRORS: ReadonlySet<string> = new Set(
  Object.keys({
    malformed: true,
    noModifier: true,
    takesTyping: true,
    reserved: true,
    taken: true,
    clash: true,
    notSaved: true,
    unsupportedKey: true,
  } satisfies Record<HotkeyErrorCode, true>),
);

/** Runs the command that changes a hotkey. Resolves null when it did, or
 * why not. */
function setHotkey(
  command: string,
  hotkey: string,
): Promise<HotkeyErrorCode | null> {
  return invoke(command, { hotkey }).then(
    () => null,
    (error: unknown) => {
      const code =
        typeof error === "object" && error !== null && "code" in error
          ? String(error.code)
          : "";
      if (HOTKEY_ERRORS.has(code)) return code as HotkeyErrorCode;
      log.warn(`hotkey: the change failed: ${errorKind(error)}`);
      return "notSaved";
    },
  );
}

/** Makes `hotkey` the launcher's. Resolves null when it did, or why not. */
export function setLauncherHotkey(
  hotkey: string,
): Promise<HotkeyErrorCode | null> {
  return setHotkey(SET_LAUNCHER_HOTKEY_COMMAND, hotkey);
}

/** Makes `hotkey` the dock's. Resolves null when it did, or why not. */
export function setDockHotkey(hotkey: string): Promise<HotkeyErrorCode | null> {
  return setHotkey(SET_DOCK_HOTKEY_COMMAND, hotkey);
}

function capture(command: string, on: boolean): void {
  invoke(command, { on }).catch((error: unknown) => {
    log.warn(`hotkey: capture failed: ${errorKind(error)}`);
  });
}

/** Lets go of the hotkeys while a new launcher's one is captured (`on`), or
 * takes them back. Failures are logged; Rust takes them back on its own
 * after a minute. */
export function captureLauncherHotkey(on: boolean): void {
  capture(CAPTURE_LAUNCHER_HOTKEY_COMMAND, on);
}

/** `captureLauncherHotkey`, for a new dock's hotkey. */
export function captureDockHotkey(on: boolean): void {
  capture(CAPTURE_DOCK_HOTKEY_COMMAND, on);
}

/** The key part of a hotkey from Windows' virtual-key code (`keyCode`):
 * 75 is `K`, 55 is `7`, 116 is `F5`, 32 is `Space`; null for the other
 * keys. */
function keyName(keyCode: number): string | null {
  if (keyCode >= 65 && keyCode <= 90) return String.fromCharCode(keyCode);
  if (keyCode >= 48 && keyCode <= 57) return String.fromCharCode(keyCode);
  if (keyCode >= 112 && keyCode <= 135) return `F${keyCode - 111}`;
  return keyCode === 32 ? "Space" : null;
}

/** Shift, Ctrl, Alt and the two Windows keys. */
const MODIFIER_KEYS = new Set([16, 17, 18, 91, 92]);

/** What a key press makes, as Rust writes a hotkey (`Ctrl+Shift+K`):
 * `modifier` while only modifiers are down, `unsupported` for a key a hotkey
 * can't have. By the virtual key (`keyCode`), which is what Windows
 * registers: on any keyboard layout, the hotkey is the key pressed. */
export function hotkeyFromEvent(event: {
  keyCode: number;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}): string | "modifier" | "unsupported" {
  if (MODIFIER_KEYS.has(event.keyCode)) return "modifier";
  const key = keyName(event.keyCode);
  if (key === null) return "unsupported";
  const parts = [
    event.ctrlKey ? "Ctrl" : null,
    event.altKey ? "Alt" : null,
    event.shiftKey ? "Shift" : null,
    event.metaKey ? "Win" : null,
    key,
  ];
  return parts.filter((part) => part !== null).join("+");
}

/** A hotkey as people read it: `Space` in the user's language. */
export function hotkeyLabel(hotkey: string, space: string): string {
  return hotkey
    .split("+")
    .map((part) => (part === "Space" ? space : part))
    .join("+");
}

/** Forgets what the launcher opened. Resolves whether it did; failures are
 * logged by kind. */
export function clearLauncherHistory(): Promise<boolean> {
  return invoke<void>(CLEAR_LAUNCHER_HISTORY_COMMAND).then(
    () => true,
    (error: unknown) => {
      log.warn(`launcher: clearing the history failed: ${errorKind(error)}`);
      return false;
    },
  );
}

/** The launcher's status, kept current by `launcher-status`; null while
 * loading, and for good if it can't be read. */
export function useLauncherStatus(): LauncherStatus | null {
  const [status, setStatus] = useState<LauncherStatus | null>(null);

  useEffect(() => {
    let active = true;
    // A change may arrive before the first read answers: the read is older
    // then, and is dropped.
    let changed = false;

    const unlisten = listen<LauncherStatus>(LAUNCHER_STATUS_EVENT, (event) => {
      if (!active) return;
      changed = true;
      setStatus(event.payload);
    });

    invoke<LauncherStatus>(LAUNCHER_STATUS_COMMAND).then(
      (current) => {
        if (active && !changed) setStatus(current);
      },
      (error: unknown) => {
        if (active)
          log.warn(`launcher: status read failed: ${errorKind(error)}`);
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  return status;
}

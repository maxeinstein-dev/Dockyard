import { invoke } from "@tauri-apps/api/core";

/* The Capturas de tela's commands (F9 of estudo 0002). The page never gets
 * a path: each screenshot comes with a key, valid until the next listing. */

/** One screenshot, newest first in the list. */
export interface Screenshot {
  key: number;
  /** The file's name, for its accessible name and its tip. */
  name: string;
  /** When it was saved, in milliseconds since 1970. */
  modified: number;
  /** A PNG `data:` URL; null when the image could not be read. */
  picture: string | null;
}

export interface ScreenshotsList {
  /** False while Windows has no Screenshots folder (none saved yet). */
  folder: boolean;
  items: Screenshot[];
}

export const SCREENSHOT_COMMANDS = {
  list: "list_screenshots",
  open: "open_screenshot",
  openFolder: "open_screenshots_folder",
  drag: "drag_screenshot",
} as const;

export function listScreenshots(): Promise<ScreenshotsList> {
  return invoke<ScreenshotsList>(SCREENSHOT_COMMANDS.list);
}

export function openScreenshot(key: number): Promise<void> {
  return invoke(SCREENSHOT_COMMANDS.open, { key });
}

export function openScreenshotsFolder(): Promise<void> {
  return invoke(SCREENSHOT_COMMANDS.openFolder);
}

/** Drags the screenshot `key` out, as File Explorer drags a file: called
 * while the button is held; resolves once it is dropped or let go. */
export function dragScreenshot(key: number): Promise<void> {
  return invoke(SCREENSHOT_COMMANDS.drag, { key });
}

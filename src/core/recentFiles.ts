import { invoke } from "@tauri-apps/api/core";
import type { FolderEntryKind } from "./apps";

/* The Arquivos' commands (F9 of estudo 0002). The page never gets a path:
 * each file comes with a key, valid until the next listing. */

/** One file opened lately, newest first in the list. */
export interface RecentFile {
  key: number;
  name: string;
  /** The name of the folder it is in, never its path. */
  folder: string;
  kind: FolderEntryKind;
}

export const RECENT_FILE_COMMANDS = {
  list: "list_recent_files",
  open: "open_recent_file",
  drag: "drag_recent_file",
} as const;

export function listRecentFiles(): Promise<RecentFile[]> {
  return invoke<RecentFile[]>(RECENT_FILE_COMMANDS.list);
}

export function openRecentFile(key: number): Promise<void> {
  return invoke(RECENT_FILE_COMMANDS.open, { key });
}

/** Drags the file `key` out, as File Explorer drags it: called while the
 * button is held; resolves once it is dropped or let go. */
export function dragRecentFile(key: number): Promise<void> {
  return invoke(RECENT_FILE_COMMANDS.drag, { key });
}

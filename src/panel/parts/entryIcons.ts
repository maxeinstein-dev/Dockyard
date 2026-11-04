import type { FolderEntryKind } from "../../core/apps";

/** The glyph of an entry of a pinned folder (spec 0025) and of a recent
 * file (the Arquivos, F9 of estudo 0002). */
export const ENTRY_ICONS = {
  folder: "folder",
  image: "image",
  video: "play",
  audio: "play",
  document: "file",
  archive: "archive",
  program: "launch",
  other: "file",
} as const satisfies Record<FolderEntryKind, string>;

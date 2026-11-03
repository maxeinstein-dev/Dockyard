import { invoke } from "@tauri-apps/api/core";
import type { SkinPaint } from "../dock/paints";
import { errorKind, log } from "./log";

/* The user's skins of the bar, in the `Skins` folder of the app's data
 * folder (spec 0020, Q14). Rust reads the folder and checks every skin; the
 * page only lists them, with their image as a `data:` URL, and opens the
 * folder, never a path. */

export { SKIN_PROBLEM_KINDS, type SkinProblemKind } from "./skinProblems";
import type { SkinProblemKind } from "./skinProblems";

/** A skin of the folder (`SkinInfo` in `src-tauri/src/commands/skins.rs`):
 * what its card's miniature draws. */
export interface FolderSkinInfo {
  /** The name of its folder: what `dock.customSkin` holds. */
  id: string;
  /** The name its `skin.json` gives. */
  name: string;
  /** `data:image/png;base64,…`. */
  url: string;
  width: number;
  height: number;
  left: number;
  right: number;
  repeat: boolean;
  pixelated: boolean;
  dark: boolean;
}

/** A folder refused, by its name, and why. */
export interface SkinProblem {
  folder: string;
  kind: SkinProblemKind;
}

export interface FolderSkins {
  skins: FolderSkinInfo[];
  problems: SkinProblem[];
}

export const SKIN_COMMANDS = {
  list: "list_folder_skins",
  openFolder: "open_skins_folder",
} as const;

/** Reads the skins folder again and lists what is in it. */
export function listFolderSkins(): Promise<FolderSkins> {
  return invoke<FolderSkins>(SKIN_COMMANDS.list).catch((error: unknown) => {
    log.warn(`skins: list failed: ${errorKind(error)}`);
    throw error;
  });
}

/** Opens the skins folder in File Explorer, making it if it is not there. */
export function openSkinsFolder(): Promise<void> {
  return invoke<void>(SKIN_COMMANDS.openFolder).catch((error: unknown) => {
    log.warn(`skins: open folder failed: ${errorKind(error)}`);
    throw error;
  });
}

/** The paint of a folder skin's bar in a miniature. */
export function folderSkinPaint(skin: FolderSkinInfo): SkinPaint {
  return {
    url: skin.url,
    width: skin.width,
    height: skin.height,
    left: skin.left,
    right: skin.right,
    repeat: skin.repeat,
    pixelated: skin.pixelated,
  };
}

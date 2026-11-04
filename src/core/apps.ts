import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";
import type { MoveDirection, StorageStatus, WidgetId } from "./widgets";
import { useTauriSnapshot, type SnapshotSource } from "./snapshot";

/* The programs pinned to the dock (spec 0006). Rust owns the list: the dock
 * only reads it (and the icons) and opens a program; every change goes
 * through the panel. */

/** The most programs the dock holds pinned. */
export const MAX_APPS = 40;

/** One pinned program. */
export interface PinnedApp {
  /** The dock's own id for the item: what the commands take. */
  id: WidgetId;
  /** The program's name in the Windows Apps folder: what opens it. */
  appId: string;
  /** The name shown for it. */
  name: string;
}

/** The pinned programs in dock order, and the state of the storage (the
 * widgets' database holds both). */
export interface AppsSnapshot {
  storage: StorageStatus;
  apps: PinnedApp[];
}

/** One program of the catalog the panel offers. */
export interface CatalogEntry {
  appId: string;
  name: string;
}

/** One entry of the menu of a program: the key names it for
 * `runAppEntry`. */
export interface MenuEntry {
  key: number;
  title: string;
  /** What an entry of a folder is (spec 0025), for its glyph. */
  kind?: FolderEntryKind;
}

/** What an entry of a pinned folder is (spec 0025). */
export type FolderEntryKind =
  | "folder"
  | "image"
  | "video"
  | "audio"
  | "document"
  | "archive"
  | "program"
  | "other";

export type MenuSectionKind =
  "tasks" | "category" | "recent" | "frequent" | "files";

export const MENU_SECTION_KINDS = [
  "tasks",
  "category",
  "recent",
  "frequent",
  "files",
] as const satisfies readonly MenuSectionKind[];

/** What a pinned item opens (spec 0025). */
export type PinnedPlace = "program" | "folder" | "recycleBin";

export interface MenuSection {
  kind: MenuSectionKind;
  /** The name of the program's own category; null for the others. */
  title: string | null;
  entries: MenuEntry[];
}

/** The menu of a pinned program, as the Windows taskbar shows it (its Jump
 * List): `canReveal` says whether its file can be shown in Explorer. */
export interface AppMenu {
  sections: MenuSection[];
  canReveal: boolean;
  /** A program, a folder (its newest entries are the "files" section) or the
   * Recycle Bin. */
  place: PinnedPlace;
  /** A folder with more entries than were looked at. */
  more: boolean;
}

export type AppErrorCode =
  | "limitReached"
  | "notFound"
  | "alreadyPinned"
  | "invalidApp"
  | "launchFailed"
  | "unavailable"
  | "writeFailed"
  | "notReady";

export const APP_ERROR_CODES: readonly AppErrorCode[] = Object.keys({
  limitReached: 0,
  notFound: 0,
  alreadyPinned: 0,
  invalidApp: 0,
  launchFailed: 0,
  unavailable: 0,
  writeFailed: 0,
  notReady: 0,
} satisfies Record<AppErrorCode, 0>) as AppErrorCode[];

const ERROR_CODES: ReadonlySet<string> = new Set(APP_ERROR_CODES);

/** How an app command rejects. Carries no id and no name. */
export interface AppCommandError {
  code: AppErrorCode;
}

export function isAppCommandError(error: unknown): error is AppCommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    ERROR_CODES.has(error.code)
  );
}

export const APP_COMMANDS = {
  list: "list_apps",
  catalog: "list_catalog",
  icon: "app_icon",
  pin: "pin_app",
  browse: "browse_app",
  unpin: "unpin_app",
  move: "move_app",
  place: "place_app",
  launch: "launch_app",
  menu: "app_menu",
  runEntry: "run_app_entry",
  reveal: "reveal_app",
  iconOptions: "app_icon_options",
  setIcon: "set_app_icon",
  chooseIconImage: "choose_app_icon_image",
  emptyRecycleBin: "empty_recycle_bin",
} as const;

/** What a program wears instead of the dock's pack (spec 0024). */
export type AppIconChoice =
  | { kind: "original" }
  | { kind: "pack"; pack: string }
  | { kind: "image"; file: string };

/** A program's "Ícone": its choice (`null`: the dock's pack) and the packs
 * it can wear. */
export interface AppIconOptions {
  current: AppIconChoice | null;
  packs: { id: string; name: string }[];
}

/** Event every window receives with an `AppsSnapshot` after a change. */
export const APPS_CHANGED_EVENT = "apps-changed";

/** The sizes of an icon Rust reads: the panel's list and the dock. */
export const ICON_SIZES = { list: 32, dock: 96 } as const;

/** Runs `command`. A known `{ code }` from Rust passes through; any other
 * failure has no code of its own: it is logged by kind and rejects with
 * `notReady`. */
function run<T>(
  command: string,
  args: Record<string, unknown>,
  logFailure: (error: unknown) => void,
): Promise<T> {
  return invoke<T>(command, args).catch((error: unknown) => {
    if (isAppCommandError(error)) throw error;
    logFailure(error);
    const notReady: AppCommandError = { code: "notReady" };
    throw notReady;
  });
}

/** The pinned programs in dock order. */
export function listApps(): Promise<AppsSnapshot> {
  return run(APP_COMMANDS.list, {}, (error) =>
    log.warn(`apps: list failed: ${errorKind(error)}`),
  );
}

/** Every program of the Windows Apps folder, by name. Takes about a second
 * the first time. */
export function listCatalog(): Promise<CatalogEntry[]> {
  return run(APP_COMMANDS.catalog, {}, (error) =>
    log.warn(`apps: catalog failed: ${errorKind(error)}`),
  );
}

/** `{ workspace }` when one is named: without it, Rust takes the workspace
 * in use (spec 0013). */
function inSpace(workspace: string | undefined): { workspace?: string } {
  return workspace === undefined ? {} : { workspace };
}

/** Pins a program of the catalog at the end of the workspace `workspace`
 * (or the one in use). */
export function pinApp(appId: string, workspace?: string): Promise<PinnedApp> {
  return run(APP_COMMANDS.pin, { appId, ...inSpace(workspace) }, (error) =>
    log.warn(`apps: pin failed: ${errorKind(error)}`),
  );
}

/** "Procurar…" (F7 of estudo 0002): the Windows dialog for an `.exe` or a
 * shortcut, pinned at the end of the workspace `workspace` (or the one in
 * use); null when the dialog was cancelled. */
export function browseApp(workspace?: string): Promise<PinnedApp | null> {
  return run(APP_COMMANDS.browse, inSpace(workspace), (error) =>
    log.warn(`apps: browse failed: ${errorKind(error)}`),
  );
}

/** Takes a program out of the workspace; with no other one showing it, it
 * is unpinned for good. */
export function unpinApp(id: WidgetId, workspace?: string): Promise<void> {
  return run(APP_COMMANDS.unpin, { id, ...inSpace(workspace) }, (error) =>
    log.warn(`apps: unpin failed: ${errorKind(error)}`),
  );
}

export function moveApp(
  id: WidgetId,
  direction: MoveDirection,
  workspace?: string,
): Promise<void> {
  return run(
    APP_COMMANDS.move,
    { id, direction, ...inSpace(workspace) },
    (error) => log.warn(`apps: move failed: ${errorKind(error)}`),
  );
}

/** Puts a pinned program at `index` among the pinned ones (the dock's drag):
 * the others close up around it. */
export function placeApp(id: WidgetId, index: number): Promise<void> {
  return run(APP_COMMANDS.place, { id, index }, (error) =>
    log.warn(`apps: place failed: ${errorKind(error)}`),
  );
}

/** Opens a pinned program. */
export function launchApp(id: WidgetId): Promise<void> {
  return run(APP_COMMANDS.launch, { id }, (error) =>
    log.warn(`apps: launch failed: ${errorKind(error)}`),
  );
}

/** The menu of a pinned program: its categories, recent items and tasks.
 * Reads the program's list from disk. */
export function appMenu(id: WidgetId): Promise<AppMenu> {
  return run(APP_COMMANDS.menu, { id }, (error) =>
    log.warn(`apps: menu failed: ${errorKind(error)}`),
  );
}

/** Runs an entry of the menu `appMenu` last returned for `id`. */
export function runAppEntry(id: WidgetId, key: number): Promise<void> {
  return run(APP_COMMANDS.runEntry, { id, key }, (error) =>
    log.warn(`apps: entry failed: ${errorKind(error)}`),
  );
}

/** Empties the Recycle Bin pinned as `id`, after Windows asks the user
 * (spec 0025): `false` when the answer was no. */
export function emptyRecycleBin(id: WidgetId): Promise<boolean> {
  return run(APP_COMMANDS.emptyRecycleBin, { id }, (error) =>
    log.warn(`apps: empty bin failed: ${errorKind(error)}`),
  );
}

/** Shows the program's file in File Explorer. */
export function revealApp(id: WidgetId): Promise<void> {
  return run(APP_COMMANDS.reveal, { id }, (error) =>
    log.warn(`apps: reveal failed: ${errorKind(error)}`),
  );
}

/** The icon of the pinned program `id` and the packs it can wear. */
export function appIconOptions(id: WidgetId): Promise<AppIconOptions> {
  return run(APP_COMMANDS.iconOptions, { id }, (error) =>
    log.warn(`apps: icon options failed: ${errorKind(error)}`),
  );
}

/** Has the pinned program `id` wear its Windows icon or a pack's image;
 * `null` for the dock's pack again. */
export function setAppIcon(
  id: WidgetId,
  icon: Exclude<AppIconChoice, { kind: "image" }> | null,
): Promise<void> {
  return run(APP_COMMANDS.setIcon, { id, icon }, (error) =>
    log.warn(`apps: set icon failed: ${errorKind(error)}`),
  );
}

/** "Escolher imagem…": Rust opens the Windows dialog and keeps the image;
 * `false` when it was cancelled or refused. */
export function chooseAppIconImage(id: WidgetId): Promise<boolean> {
  return run(APP_COMMANDS.chooseIconImage, { id }, (error) =>
    log.warn(`apps: choose icon failed: ${errorKind(error)}`),
  );
}

/** At most this many icons are read at once: each is a few milliseconds of
 * the shell's time, and the panel's catalog asks for a few hundred. */
const ICON_CONCURRENCY = 4;

/** The most icons kept (M8): a few hundred for the catalog, and room for
 * another pack's. Each is a data URL of a few kilobytes; trying packs used
 * to add a copy of every icon for each. The ones read longest ago go
 * first, and are read again when asked. */
export const MAX_ICONS = 600;

/** The icons read, the most recently asked last (a `Map` keeps the order). */
const icons = new Map<string, Promise<string | null>>();
const waiting: (() => void)[] = [];
let reading = 0;

function pump() {
  while (reading < ICON_CONCURRENCY) {
    const start = waiting.shift();
    if (!start) return;
    reading += 1;
    start();
  }
}

/** The icon of a program as an image URL, or null when Windows has none (or
 * it can't be read). Each program and size is asked once per page; the
 * requests go a few at a time. `variant` names what else the icon depends on
 * (the icon pack in use, spec 0007): the same program in another variant is
 * asked again. */
export function appIcon(
  appId: string,
  size: number = ICON_SIZES.dock,
  variant = "",
): Promise<string | null> {
  const key = `${size} ${variant} ${appId}`;
  let icon = icons.get(key);
  if (icon !== undefined) {
    // Asked again: the last to go.
    icons.delete(key);
    icons.set(key, icon);
  } else {
    icon = new Promise<string | null>((resolve) => {
      waiting.push(() => {
        invoke<string | null>(APP_COMMANDS.icon, { appId, size })
          .then(
            (url) => resolve(typeof url === "string" ? url : null),
            (error: unknown) => {
              log.warn(`apps: icon failed: ${errorKind(error)}`);
              resolve(null);
            },
          )
          .finally(() => {
            reading -= 1;
            pump();
          });
      });
      pump();
    });
    icons.set(key, icon);
    if (icons.size > MAX_ICONS) {
      const oldest = icons.keys().next().value;
      if (oldest !== undefined) icons.delete(oldest);
    }
  }
  return icon;
}

/** Forgets the icons read, for the ones that changed on disk; what is being
 * read still ends. */
export function dropIcons(): void {
  icons.clear();
}

/** Forgets the icons read and what waits to be read (for the tests). */
export function forgetIcons(): void {
  icons.clear();
  waiting.length = 0;
  reading = 0;
}

const APPS_SOURCE: SnapshotSource<AppsSnapshot> = {
  event: APPS_CHANGED_EVENT,
  read: listApps,
  accept: (value) =>
    Array.isArray((value as AppsSnapshot | null)?.apps)
      ? (value as AppsSnapshot)
      : null,
  readFailed: () => log.warn("apps: read failed"),
  listenFailed: (error) => log.warn(`apps: listen failed: ${errorKind(error)}`),
};

/** The pinned programs and the storage state, kept current by
 * `apps-changed`; null while loading, and for good if they can't be read (the
 * dock then has no apps). */
export function useApps(): AppsSnapshot | null {
  return useTauriSnapshot(APPS_SOURCE);
}

/** The icon of a program, as it arrives: null until it is read, and when
 * the program has none. */
export function useAppIcon(
  appId: string,
  size: number = ICON_SIZES.dock,
  variant = "",
): string | null {
  const [icon, setIcon] = useState<{ key: string; url: string | null } | null>(
    null,
  );
  const key = `${size} ${variant} ${appId}`;
  useEffect(() => {
    let active = true;
    void appIcon(appId, size, variant).then((url) => {
      if (active) setIcon({ key, url });
    });
    return () => {
      active = false;
    };
  }, [appId, size, variant, key]);
  return icon !== null && icon.key === key ? icon.url : null;
}

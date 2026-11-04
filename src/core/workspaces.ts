import { invoke } from "@tauri-apps/api/core";
import { errorKind, log } from "./log";
import type { MoveDirection } from "./widgets";
import { useTauriSnapshot, type SnapshotSource } from "./snapshot";

/* The workspaces (spec 0013): each one is a dock of its own, with a name, a
 * color and the programs and widgets it shows, in order. Rust owns them;
 * Configurações edits any of them, whichever the docks show. The programs
 * and widgets of a workspace change through the apps and widgets commands,
 * which take its id (`workspace`). */

/** The most workspaces: Ctrl+Alt+1…9 reaches each one. */
export const MAX_WORKSPACES = 9;

/** The longest name, in characters. */
export const MAX_WORKSPACE_NAME = 32;

/** The colors a workspace can have, in the order new ones take them (keep
 * in step with `PALETTE` in `src-tauri/src/domain/workspaces.rs`). */
export const WORKSPACE_COLORS = [
  "#0A84FF",
  "#FF9F0A",
  "#30D158",
  "#BF5AF2",
  "#FF375F",
  "#FF453A",
  "#FFD60A",
  "#8E8E93",
] as const;

/** How much room a spacer leaves (spec 0022): a quarter or half of an
 * icon. */
export type SpacerSize = "small" | "large";

/** An empty room between two items of a workspace's dock (spec 0022). */
export interface WorkspaceSpacer {
  id: string;
  size: SpacerSize;
}

/** Widgets in one tile of a workspace's dock (spec 0023): their ids in
 * order, and which is on top. */
export interface WorkspaceStack {
  id: string;
  widgets: string[];
  top: number;
}

export interface Workspace {
  id: string;
  name: string;
  /** One of `WORKSPACE_COLORS`. */
  color: string;
  /** Its items in dock order: `app:<id>`, `widget:<id>`, `spacer:<id>`
   * and `stack:<id>`. */
  items: string[];
  /** Its spacers, in dock order: what the `spacer:<id>` keys stand for. */
  spacers: WorkspaceSpacer[];
  /** Its stacks, in dock order: what the `stack:<id>` keys stand for. */
  stacks: WorkspaceStack[];
}

/** Every workspace in order, and the one in use (the dock used last). */
export interface WorkspacesSnapshot {
  workspaces: Workspace[];
  inUse: string | null;
}

export type WorkspaceErrorCode =
  | "notFound"
  | "limitReached"
  | "invalidName"
  | "invalidColor"
  | "lastWorkspace"
  | "unavailable"
  | "writeFailed"
  | "notReady";

export interface WorkspaceCommandError {
  code: WorkspaceErrorCode;
}

const ERROR_CODES: ReadonlySet<string> = new Set<WorkspaceErrorCode>([
  "notFound",
  "limitReached",
  "invalidName",
  "invalidColor",
  "lastWorkspace",
  "unavailable",
  "writeFailed",
  "notReady",
]);

export function isWorkspaceCommandError(
  error: unknown,
): error is WorkspaceCommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    ERROR_CODES.has(error.code)
  );
}

export const WORKSPACE_COMMANDS = {
  list: "list_workspaces",
  create: "create_workspace",
  duplicate: "duplicate_workspace",
  rename: "rename_workspace",
  recolor: "recolor_workspace",
  move: "move_workspace",
  delete: "delete_workspace",
  moveAppWindows: "move_app_windows",
  moveOpenWindows: "move_open_windows",
  removeSpacer: "remove_spacer",
  moveSpacer: "move_spacer",
  moveStack: "move_stack",
  dissolveStack: "dissolve_stack",
} as const;

/** Event every window receives with a `WorkspacesSnapshot` after a change. */
export const WORKSPACES_CHANGED_EVENT = "workspaces-changed";

/** Runs `command`. A known `{ code }` from Rust passes through; any other
 * failure is logged by kind and rejects with `notReady`. */
function run<T>(command: string, args: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args).catch((error: unknown) => {
    if (isWorkspaceCommandError(error)) throw error;
    log.warn(`workspaces: a command failed: ${errorKind(error)}`);
    const notReady: WorkspaceCommandError = { code: "notReady" };
    throw notReady;
  });
}

export function listWorkspaces(): Promise<WorkspacesSnapshot> {
  return run(WORKSPACE_COMMANDS.list, {});
}

/** Makes a workspace at the end, empty, with the next free color. */
export function createWorkspace(name?: string): Promise<Workspace> {
  return run(WORKSPACE_COMMANDS.create, name === undefined ? {} : { name });
}

/** A copy of the workspace `id` right after it (F7 of estudo 0002), named
 * `name`: the same programs and widgets, its own spacers and stacks. */
export function duplicateWorkspace(
  id: string,
  name: string,
): Promise<Workspace> {
  return run(WORKSPACE_COMMANDS.duplicate, { id, name });
}

/** The name of a copy of the workspace `name`: `template` ("{name}
 * (cópia)") with the name cut to fit, and a number when it is taken
 * ("Principal (cópia) 2"). */
export function copyName(
  name: string,
  template: string,
  taken: readonly string[],
): string {
  for (let number = 1; ; number += 1) {
    const suffix = number === 1 ? "" : ` ${number}`;
    const room =
      MAX_WORKSPACE_NAME -
      [...template.replace("{name}", "")].length -
      [...suffix].length;
    const cut = [...name].slice(0, Math.max(room, 1)).join("").trimEnd();
    const made = `${template.replace("{name}", cut)}${suffix}`;
    if (!taken.includes(made)) return made;
  }
}

export function renameWorkspace(id: string, name: string): Promise<void> {
  return run(WORKSPACE_COMMANDS.rename, { id, name });
}

export function recolorWorkspace(id: string, color: string): Promise<void> {
  return run(WORKSPACE_COMMANDS.recolor, { id, color });
}

export function moveWorkspace(
  id: string,
  direction: MoveDirection,
): Promise<void> {
  return run(WORKSPACE_COMMANDS.move, { id, direction });
}

/** Deletes a workspace (never the last): the programs and widgets no other
 * workspace has go with it. */
export function deleteWorkspace(id: string): Promise<void> {
  return run(WORKSPACE_COMMANDS.delete, { id });
}

/** "Mover para <espaço>" (spec 0013 C3): the windows of the pinned program
 * `id` in the workspace in use go to `workspace`. How many moved. */
export function moveAppWindows(id: string, workspace: string): Promise<number> {
  return run(WORKSPACE_COMMANDS.moveAppWindows, { id, workspace });
}

/** The same for the open program `key` that is not pinned. */
export function moveOpenWindows(
  key: string,
  workspace: string,
): Promise<number> {
  return run(WORKSPACE_COMMANDS.moveOpenWindows, { key, workspace });
}

/** Removes the spacer `id` of `workspace` (the one in use without it) for
 * good (spec 0022). */
export function removeSpacer(id: string, workspace?: string): Promise<void> {
  return run(
    WORKSPACE_COMMANDS.removeSpacer,
    workspace === undefined ? { id } : { id, workspace },
  );
}

/** Moves the spacer `id` one place up or down among the widgets and the
 * spacers of `workspace` (Itens do dock lists them together). */
export function moveSpacer(
  id: string,
  direction: MoveDirection,
  workspace?: string,
): Promise<void> {
  return run(
    WORKSPACE_COMMANDS.moveSpacer,
    workspace === undefined ? { id, direction } : { id, direction, workspace },
  );
}

/** Moves the stack `id` one place up or down among the widgets, the
 * spacers and the stacks of `workspace` (spec 0023). */
export function moveStack(
  id: string,
  direction: MoveDirection,
  workspace?: string,
): Promise<void> {
  return run(
    WORKSPACE_COMMANDS.moveStack,
    workspace === undefined ? { id, direction } : { id, direction, workspace },
  );
}

/** Undoes the stack `id` of `workspace`: its widgets loose in its place. */
export function dissolveStack(id: string, workspace?: string): Promise<void> {
  return run(
    WORKSPACE_COMMANDS.dissolveStack,
    workspace === undefined ? { id } : { id, workspace },
  );
}

/** A name the user may give: one line, no spaces at the ends, 1 to
 * `MAX_WORKSPACE_NAME` characters (Rust checks it again). */
export function isValidWorkspaceName(name: string): boolean {
  const length = [...name].length;
  return (
    length >= 1 &&
    length <= MAX_WORKSPACE_NAME &&
    name.trim() === name &&
    !/\p{Cc}/u.test(name)
  );
}

/** Of `list` (programs or widgets, by `id`), those `workspace` shows, in its
 * order, the widgets of a stack in its place (spec 0023); the whole list
 * when there is no workspace (they can't be read). */
export function inWorkspace<T extends { id: string }>(
  list: readonly T[],
  workspace: Workspace | null | undefined,
  kind: "app" | "widget",
): T[] {
  if (!workspace) return [...list];
  const byKey = new Map(list.map((item) => [`${kind}:${item.id}`, item]));
  const stacks = new Map(
    workspace.stacks.map((stack) => [`stack:${stack.id}`, stack]),
  );
  return workspace.items.flatMap((key) => {
    const stack = kind === "widget" ? stacks.get(key) : undefined;
    const keys = stack ? stack.widgets.map((id) => `widget:${id}`) : [key];
    return keys.flatMap((each) => {
      const item = byKey.get(each);
      return item ? [item] : [];
    });
  });
}

const WORKSPACES_SOURCE: SnapshotSource<WorkspacesSnapshot> = {
  event: WORKSPACES_CHANGED_EVENT,
  read: listWorkspaces,
  accept: (value) =>
    Array.isArray((value as WorkspacesSnapshot | null)?.workspaces)
      ? (value as WorkspacesSnapshot)
      : null,
  readFailed: () => log.warn("workspaces: read failed"),
  listenFailed: (error) =>
    log.warn(`workspaces: listen failed: ${errorKind(error)}`),
};

/** The workspaces as they are, following `workspaces-changed`. null while
 * loading, and for good when they can't be read. */
export function useWorkspaces(): WorkspacesSnapshot | null {
  return useTauriSnapshot(WORKSPACES_SOURCE);
}

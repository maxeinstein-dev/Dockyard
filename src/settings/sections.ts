/** The sections of the Configurações app; must match `Section` in
 * `src-tauri/src/domain/launcher_pages.rs`. */
export type Section =
  | "general"
  | "look"
  | "behavior"
  | "workspaces"
  | "apps"
  | "items"
  | "launcher"
  | "accounts"
  | "about";

/** The bands of the side bar (spec 0029 §2): a small title over the
 * sections it holds. */
export type Band = "dock" | "content" | "more";

/** The side bar, top to bottom: each band and its sections (spec 0029 §2). */
export const BANDS: readonly { band: Band; sections: readonly Section[] }[] = [
  { band: "dock", sections: ["general", "look", "behavior"] },
  { band: "content", sections: ["workspaces", "apps", "items"] },
  { band: "more", sections: ["launcher", "accounts", "about"] },
];

/** Every section, in the order of the side bar. */
export const SECTIONS: readonly Section[] = BANDS.flatMap(
  (each) => each.sections,
);

/** Event Rust sends to the open window to show a section (a page the
 * launcher found). */
export const SECTION_EVENT = "settings-section";

export function isSection(value: unknown): value is Section {
  return SECTIONS.some((section) => section === value);
}

declare global {
  interface Window {
    /** The section a window opened by the launcher starts on, set by Rust
     * before the page's scripts run. */
    __DOCKYARD_SECTION__?: string;
  }
}

/** The section the window starts on: the launcher's, else Geral. */
export function startSection(): Section {
  const asked = window.__DOCKYARD_SECTION__;
  return isSection(asked) ? asked : "general";
}

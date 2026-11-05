import { format, type Messages } from "../i18n";
import type { Section } from "./sections";

/** One option the search of Configurações finds (estudo 0002 F6). */
export interface SearchEntry {
  section: Section;
  /** The option's name as its row shows it; the search compares it and,
   * after the jump, looks for it in the section. */
  label: string;
  /** The title of the card the row is in, shown beside the result. */
  group?: string;
}

/**
 * Every option of Configurações, by section, in the order of the window.
 * `search.test.tsx` renders each section and fails when a row is missing
 * here or an entry has no row, so a new field can't be left out by
 * accident.
 */
export function searchIndex(t: Messages): SearchEntry[] {
  const s = t.settings;
  const groups = s.app.groups;
  const look = s.look;
  /** The entries of `section`: its rows, by the group they are in. */
  const inSection =
    (section: Section) =>
    (label: string, group?: string): SearchEntry =>
      group === undefined ? { section, label } : { section, label, group };
  const general = inSection("general");
  const inLook = inSection("look");
  const behavior = inSection("behavior");
  const launcher = inSection("launcher");
  const accounts = inSection("accounts");
  return [
    general(groups.dock, groups.dock),
    general(s.groups.theme, groups.dock),
    general(s.groups.edge, groups.dock),
    general(s.groups.size, groups.dock),
    general(s.groups.opacity, groups.dock),
    general(groups.taskbar, groups.taskbar),
    general(s.groups.taskbar, groups.taskbar),
    general(t.tray.inDock.label, groups.taskbar),
    general(groups.startup, groups.startup),
    general(s.groups.language, groups.startup),
    general(s.launchAtLogin, groups.startup),
    inLook(groups.themes, groups.themes),
    inLook(look.themes, groups.themes),
    inLook(groups.style, groups.style),
    inLook(look.style, groups.style),
    inLook(groups.tweaks, groups.tweaks),
    inLook(look.border, groups.tweaks),
    inLook(look.corners, groups.tweaks),
    inLook(look.reflections, groups.tweaks),
    inLook(look.restore, groups.tweaks),
    inLook(groups.hover, groups.hover),
    inLook(look.hover, groups.hover),
    inLook(look.hoverScale, groups.hover),
    inLook(groups.indicator, groups.indicator),
    inLook(look.indicator, groups.indicator),
    inLook(look.indicatorColor, groups.indicator),
    inLook(s.indicatorFollowsWorkspace, groups.indicator),
    inLook(groups.icons, groups.icons),
    inLook(look.iconShape, groups.icons),
    inLook(look.iconPack, groups.icons),
    inLook(groups.openAnimation, groups.openAnimation),
    inLook(look.openAnimation, groups.openAnimation),
    behavior(groups.showHide, groups.showHide),
    behavior(s.autoHide, groups.showHide),
    behavior(s.autoHideReveal, groups.showHide),
    behavior(s.autoHideDelay, groups.showHide),
    behavior(groups.previews, groups.previews),
    behavior(s.previews, groups.previews),
    behavior(s.previewDelay, groups.previews),
    behavior(s.previewsOnlySeveral, groups.previews),
    behavior(groups.appsInDock, groups.appsInDock),
    behavior(s.showOpenApps, groups.appsInDock),
    behavior(s.showBadges, groups.appsInDock),
    behavior(groups.energy, groups.energy),
    behavior(s.groups.powerSaver, groups.energy),
    behavior(groups.monitors, groups.monitors),
    behavior(s.everyMonitor, groups.monitors),
    behavior(s.switchEveryMonitor, groups.monitors),
    behavior(groups.advanced, groups.advanced),
    behavior(s.groups.middleClick, groups.advanced),
    behavior(s.dockHotkey.label, groups.advanced),
    { section: "workspaces" as const, label: t.workspaces.name },
    { section: "apps" as const, label: t.apps.add },
    { section: "items" as const, label: t.widgets.catalog.title },
    launcher(groups.launcher, groups.launcher),
    // The switch's name carries the hotkey ("Lançador (Alt+Espaço)"): the
    // entry is the name alone, and the row is found by its start.
    launcher(launcherName(s.launcher.label), groups.launcher),
    launcher(s.launcher.hotkey.label, groups.launcher),
    launcher(s.launcher.keywords.label, s.launcher.keywords.label),
    launcher(groups.advanced, groups.advanced),
    launcher(s.launcher.history, groups.advanced),
    accounts(t.accounts.client.title, t.accounts.client.title),
    accounts(t.accounts.client.id, t.accounts.client.title),
    accounts(t.accounts.client.secret, t.accounts.client.title),
    accounts(t.accounts.list.title, t.accounts.list.title),
    // Sobre: the copy of the dock (spec 0026).
    { section: "about" as const, label: s.copy.title, group: s.copy.title },
  ].filter(
    // A card titled like its only row ("Lançador") is found once.
    (entry, index, all) =>
      all.findIndex(
        (other) =>
          other.section === entry.section && other.label === entry.label,
      ) === index,
  );
}

/** "Lançador ({hotkey})" without the hotkey: "Lançador". */
function launcherName(template: string): string {
  return format(template, { hotkey: "" })
    .replace(/\s*\(\s*\)\s*$/, "")
    .trim();
}

/** Lower case, without accents or extra spaces: "Prévia" finds "previa". */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

/** What a search shows: the sections whose name matches, and the options. */
export interface SearchResult {
  sections: Section[];
  entries: SearchEntry[];
}

/**
 * The sections and options that hold every word of `query`, in the order
 * of the window. An option matches by its name, its card's title or its
 * section's name; a section, by its name or its lead. An empty query finds
 * nothing.
 */
export function searchSettings(
  query: string,
  index: readonly SearchEntry[],
  sectionNames: Readonly<Record<Section, string>>,
  sectionLeads: Readonly<Record<Section, string>>,
  sections: readonly Section[],
): SearchResult {
  const words = normalize(query).split(" ").filter(Boolean);
  if (words.length === 0) return { sections: [], entries: [] };
  const holds = (text: string) => {
    const haystack = normalize(text);
    return words.every((word) => haystack.includes(word));
  };
  return {
    sections: sections.filter((section) =>
      holds(`${sectionNames[section]} ${sectionLeads[section]}`),
    ),
    entries: index.filter((entry) =>
      holds(
        `${entry.label} ${entry.group ?? ""} ${sectionNames[entry.section]}`,
      ),
    ),
  };
}

/** The names a row of Configurações shows, the ones the index lists. */
export const ROW_LABELS =
  ".option-label, .toggle-label, .range-label, .field-label, .settings-group-title, .catalog-title, .apps-add-label";

/** The row an option lives in, from its name. */
const ROWS =
  ".option-row, .choice-field, .toggle-row, .range-row, .field, .settings-group, .catalog, .apps-add";

/**
 * The row of the option named `label` inside `root`: the one whose name is
 * `label`, or starts with it and a parenthesis (the launcher's switch);
 * a card's title gives the whole card. Null when the section doesn't show
 * it now (a row that appears only with its switch on).
 */
export function findRow(root: ParentNode, label: string): HTMLElement | null {
  const wanted = normalize(label);
  const names = Array.from(root.querySelectorAll<HTMLElement>(ROW_LABELS));
  const name =
    names.find((each) => normalize(each.textContent ?? "") === wanted) ??
    names.find((each) =>
      normalize(each.textContent ?? "").startsWith(`${wanted} (`),
    );
  if (!name) return null;
  return name.closest<HTMLElement>(ROWS) ?? name;
}

/**
 * The titles of the groups of `section`, in the order of the window: the
 * entries named like their group (spec 0029 §3, the index under the lead).
 */
export function sectionGroups(
  index: readonly SearchEntry[],
  section: Section,
): string[] {
  return index
    .filter((entry) => entry.section === section && entry.label === entry.group)
    .map((entry) => entry.label);
}

/** The first control of a row the Tab key would reach. */
export function firstControl(row: HTMLElement): HTMLElement | null {
  return row.querySelector<HTMLElement>(
    'input:not([disabled]), select:not([disabled]), textarea:not([disabled]), button:not([disabled]):not([tabindex="-1"]), [tabindex="0"]',
  );
}

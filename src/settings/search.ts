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
  const general = (label: string, group: string): SearchEntry => ({
    section: "general",
    label,
    group,
  });
  const inLook = (label: string, group: string): SearchEntry => ({
    section: "look",
    label,
    group,
  });
  return [
    general(groups.dock, groups.dock),
    general(s.groups.theme, groups.dock),
    general(s.groups.edge, groups.dock),
    general(s.groups.size, groups.dock),
    general(s.groups.opacity, groups.dock),
    general(groups.taskbar, groups.taskbar),
    general(s.groups.taskbar, groups.taskbar),
    general(t.tray.inDock.label, groups.taskbar),
    general(groups.behavior, groups.behavior),
    general(s.everyMonitor, groups.behavior),
    general(s.switchEveryMonitor, groups.behavior),
    general(s.indicatorFollowsWorkspace, groups.behavior),
    general(s.autoHide, groups.behavior),
    general(s.previews, groups.behavior),
    general(s.previewDelay, groups.behavior),
    general(s.previewsOnlySeveral, groups.behavior),
    general(s.showOpenApps, groups.behavior),
    general(s.showBadges, groups.behavior),
    general(s.groups.middleClick, groups.behavior),
    general(s.groups.powerSaver, groups.behavior),
    general(s.dockHotkey.label, groups.behavior),
    general(groups.launcher, groups.launcher),
    // The switch's name carries the hotkey ("Lançador (Alt+Espaço)"): the
    // entry is the name alone, and the row is found by its start.
    general(launcherName(s.launcher.label), groups.launcher),
    general(s.launcher.hotkey.label, groups.launcher),
    general(s.launcher.history, groups.launcher),
    general(s.launcher.keywords.label, groups.launcher),
    general(s.groups.language, s.groups.language),
    general(groups.startup, groups.startup),
    general(s.launchAtLogin, groups.startup),
    { section: "workspaces" as const, label: t.workspaces.name },
    // Spec 0028: the monitors, each one's workspace and hold.
    {
      section: "workspaces" as const,
      label: t.workspaces.monitors.title,
      group: t.workspaces.monitors.title,
    },
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
    inLook(groups.icons, groups.icons),
    inLook(look.iconShape, groups.icons),
    inLook(look.iconPack, groups.icons),
    inLook(groups.openAnimation, groups.openAnimation),
    inLook(look.openAnimation, groups.openAnimation),
    { section: "items" as const, label: t.widgets.catalog.title },
    // Sobre: the copy of the dock (spec 0026).
    { section: "about" as const, label: s.copy.title, group: s.copy.title },
  ].filter(
    // A card titled like its only row ("Idioma") is found once.
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
  ".option-label, .toggle-label, .range-label, .field-label, .settings-group-title, .catalog-title";

/** The row an option lives in, from its name. */
const ROWS =
  ".option-row, .choice-field, .toggle-row, .range-row, .field, .settings-group, .catalog";

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

/** The first control of a row the Tab key would reach. */
export function firstControl(row: HTMLElement): HTMLElement | null {
  return row.querySelector<HTMLElement>(
    'input:not([disabled]), select:not([disabled]), textarea:not([disabled]), button:not([disabled]):not([tabindex="-1"]), [tabindex="0"]',
  );
}

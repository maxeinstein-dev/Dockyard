import {
  Fragment,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { flushSync } from "react-dom";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useApps } from "../core/apps";
import { errorKind, log } from "../core/log";
import { useSettings } from "../core/settings";
import { useWidgets } from "../core/widgets";
import { useWorkspaces } from "../core/workspaces";
import { format, useT } from "../i18n";
import AboutPanel from "../panel/AboutPanel";
import AppsPanel from "../panel/AppsPanel";
import AccountsPanel from "./AccountsPanel";
import MonitorsSection from "./MonitorsSection";
import SettingsCopy from "./SettingsCopy";
import WidgetCatalog from "./WidgetCatalog";
import DockItems from "../panel/DockItems";
import LookPanel from "../panel/LookPanel";
import SettingsPanel from "../panel/SettingsPanel";
import WorkspacePicker from "../panel/WorkspacePicker";
import WorkspacesPanel from "../panel/WorkspacesPanel";
import Icon from "../panel/parts/Icon";
import { NamedBySection } from "../panel/parts/focus";
import { openAdvanced } from "../panel/parts/advanced";
import { inWorkspace } from "../core/workspaces";
import {
  BANDS,
  SECTIONS,
  SECTION_EVENT,
  isSection,
  startSection,
  type Section,
} from "./sections";
import {
  findRow,
  firstControl,
  searchIndex,
  searchSettings,
  sectionGroups,
  type SearchEntry,
} from "./search";
import "./SettingsApp.css";
import "./SettingsSections.css";

/** The glaze of each section's plate (spec 0014 R1; DESIGN.md "Tertiary",
 * the static .t-* classes of tokens.css). */
const PLATES: Readonly<Record<Section, string>> = {
  general: "t-settings",
  look: "t-music",
  behavior: "t-activity",
  workspaces: "t-clock",
  apps: "t-files",
  items: "t-notes",
  launcher: "t-launcher",
  accounts: "t-calendar",
  about: "t-about",
};

/** A section's plate: its glaze with its line icon. Decorative. */
function Plate({
  section,
  large = false,
}: {
  section: Section;
  large?: boolean;
}) {
  return (
    <span
      className={`section-plate ${PLATES[section]}${large ? " large" : ""}`}
      aria-hidden="true"
    >
      <Icon name={section} />
    </span>
  );
}

/** The index of a section's groups, under its lead (spec 0029 §3): one
 * button per group, which scrolls to it and focuses its title. Shown only
 * with 3 groups or more, Avançado aside. */
function SectionIndex({
  label,
  groups,
  advanced,
  onJump,
}: {
  label: string;
  groups: readonly string[];
  advanced: string;
  onJump: (group: string) => void;
}) {
  if (groups.filter((group) => group !== advanced).length < 3) return null;
  return (
    <nav className="section-index" aria-label={label}>
      {groups.map((group, at) => (
        <Fragment key={group}>
          {at > 0 ? (
            <span className="section-index-dot" aria-hidden="true">
              ·
            </span>
          ) : null}
          <button
            type="button"
            className="section-index-item"
            onClick={() => onJump(group)}
          >
            {group}
          </button>
        </Fragment>
      ))}
    </nav>
  );
}

/** Where the arrows, Home and End take the focus among `count` sections
 * of the side bar from `at` (spec 0029 §8.4); null for any other key. */
function navStep(key: string, at: number, count: number): number | null {
  const last = count - 1;
  if (key === "ArrowDown") return at === last ? 0 : at + 1;
  if (key === "ArrowUp") return at === 0 ? last : at - 1;
  if (key === "Home") return 0;
  if (key === "End") return last;
  return null;
}

/** The keys of the side bar: the focus moves; Enter or Space opens. */
function moveInNav(event: ReactKeyboardEvent<HTMLButtonElement>) {
  const nav = event.currentTarget.closest("nav");
  if (!nav) return;
  const items = Array.from(
    nav.querySelectorAll<HTMLButtonElement>("[data-nav-section]"),
  );
  const to = navStep(
    event.key,
    items.indexOf(event.currentTarget),
    items.length,
  );
  if (to === null) return;
  event.preventDefault();
  items[to]?.focus();
}

function closeWindow() {
  getCurrentWindow()
    .close()
    .catch((error: unknown) => {
      log.warn(`settings: close failed: ${errorKind(error)}`);
    });
}

/**
 * The Configurações app (spec 0011 H1): an ordinary window, made when the
 * user opens the settings and destroyed when it closes. A side bar of
 * sections and, beside it, the same views the panel had, at a width that fits
 * them. Every choice is saved at once, as in the panel; Esc closes the window,
 * as it closes a panel.
 */
function SettingsApp() {
  const t = useT();
  const text = t.settings.app;
  const settings = useSettings();
  const widgets = useWidgets();
  const apps = useApps();
  const spaces = useWorkspaces();
  const [section, setSection] = useState<Section>(startSection);
  /** The workspace being edited (spec 0013): chosen here, whichever the
   * docks show; at first the one in use. */
  const [chosen, setChosen] = useState<string | null>(null);
  /** What the search of the side bar holds (estudo 0002 F6). */
  const [query, setQuery] = useState("");
  /** The option a result led to, highlighted once its section shows;
   * `jumps` counts the results chosen, so one in the section on the screen
   * is also marked. */
  const target = useRef<SearchEntry | null>(null);
  const [jumps, setJumps] = useState(0);
  const searchField = useRef<HTMLInputElement>(null);
  const main = useRef<HTMLElement>(null);
  /** Counts the sections opened by the keyboard: each one takes the focus
   * to the title, for the screen reader to say where it is (spec 0029
   * §8.5). */
  const [arrivals, setArrivals] = useState(0);
  const index = searchIndex(t);
  const found = searchSettings(
    query,
    index,
    text.sections,
    text.leads,
    SECTIONS,
  );
  const searching = query.trim() !== "";
  const all = spaces?.workspaces ?? [];
  // The programs of the workspace in use, in dock order: the miniatures of
  // the styles show them, as the dock in use does.
  const inUse = all.find((each) => each.id === spaces?.inUse) ?? all[0];
  const pinnedInUse = inWorkspace(apps?.apps ?? [], inUse, "app").map(
    (app) => app.appId,
  );
  const editing =
    all.find((each) => each.id === chosen) ??
    all.find((each) => each.id === spaces?.inUse) ??
    all[0] ??
    null;

  // The launcher found a section with the window already open.
  useEffect(() => {
    let active = true;
    const unlisten = listen<unknown>(SECTION_EVENT, (event) => {
      if (active && isSection(event.payload)) setSection(event.payload);
    });
    return () => {
      active = false;
      void unlisten.then((stop) => stop());
    };
  }, []);

  // The option a result led to: scrolled to, marked for a moment and
  // focused, once its section is on the screen.
  useEffect(() => {
    const wanted = target.current;
    if (wanted === null || wanted.section !== section) return;
    target.current = null;
    const row = main.current ? findRow(main.current, wanted.label) : null;
    if (row === null) return;
    // A row of a closed Avançado: open it, then come back once it shows.
    if (row.closest("[hidden]") !== null && openAdvanced(row)) {
      target.current = wanted;
      setJumps((count) => count + 1);
      return;
    }
    row.scrollIntoView({ block: "center" });
    row.classList.add("search-hit");
    const control = firstControl(row);
    if (control) {
      control.focus({ preventScroll: true });
    } else {
      row.tabIndex = -1;
      row.focus({ preventScroll: true });
    }
    const timer = window.setTimeout(
      () => row.classList.remove("search-hit"),
      1600,
    );
    return () => window.clearTimeout(timer);
  }, [jumps, section]);

  // A section opened by the keyboard: its title takes the focus.
  useEffect(() => {
    if (arrivals === 0) return;
    document.getElementById("panel-title")?.focus();
  }, [arrivals]);

  /** Opens `each`; by the keyboard (a click with no pointer), the focus goes
   * to its title. */
  function open(each: Section, byKeyboard: boolean) {
    setSection(each);
    if (byKeyboard) setArrivals((count) => count + 1);
  }

  /** A group of the index: scrolled to, its title focused; Avançado opens,
   * its button taking the focus. */
  function jump(group: string) {
    const card = main.current ? findRow(main.current, group) : null;
    if (card === null) return;
    flushSync(() => openAdvanced(card));
    card.scrollIntoView({ block: "start" });
    const toggle = card.querySelector<HTMLElement>(".settings-advanced-toggle");
    const title = card.querySelector<HTMLElement>(".settings-group-title");
    if (toggle) {
      toggle.focus({ preventScroll: true });
    } else if (title) {
      title.tabIndex = -1;
      title.focus({ preventScroll: true });
    }
  }

  function go(entry: SearchEntry) {
    target.current = entry;
    setSection(entry.section);
    setJumps((count) => count + 1);
  }

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey && !event.altKey && event.key.toLowerCase() === "f") {
        event.preventDefault();
        searchField.current?.focus();
        searchField.current?.select();
        return;
      }
      if (event.key === "Escape") closeWindow();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="settings-app">
      <nav className="settings-nav" aria-label={text.nav}>
        <div className="settings-brand" aria-hidden="true">
          <span className="settings-brand-name">Dockyard</span>
          <span className="settings-brand-sub">{text.brand}</span>
        </div>
        <input
          ref={searchField}
          type="search"
          className="settings-search"
          placeholder={text.search.label}
          aria-label={text.search.label}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            // Esc empties the search first; on an empty one it closes
            // the window, as anywhere else.
            if (event.key === "Escape" && query !== "") {
              event.stopPropagation();
              event.nativeEvent.stopImmediatePropagation();
              setQuery("");
            }
          }}
        />
        {searching ? (
          <div
            className="settings-results"
            role="group"
            aria-label={text.search.results}
          >
            {found.sections.map((each) => (
              <button
                key={each}
                type="button"
                className="settings-nav-item"
                aria-current={each === section ? "page" : undefined}
                onClick={(event) => open(each, event.detail === 0)}
              >
                <Plate section={each} />
                {text.sections[each]}
              </button>
            ))}
            {found.entries.map((entry) => (
              <button
                key={`${entry.section}:${entry.label}`}
                type="button"
                className="settings-nav-item settings-result"
                onClick={() => go(entry)}
              >
                <span className="settings-result-name">{entry.label}</span>
                {/* Between the two, for the name read aloud. */}{" "}
                <span className="settings-result-where">
                  {entry.group && entry.group !== entry.label
                    ? `${text.sections[entry.section]} › ${entry.group}`
                    : text.sections[entry.section]}
                </span>
              </button>
            ))}
            <p className="settings-results-none" role="status">
              {found.sections.length === 0 && found.entries.length === 0
                ? format(text.search.none, { query: query.trim() })
                : ""}
            </p>
          </div>
        ) : (
          // The bands (spec 0029 §2), each a group named by its title. The
          // sections are one stop of Tab, the one shown; the arrows move
          // among them.
          BANDS.map(({ band, sections }) => (
            <div
              key={band}
              className="settings-band"
              role="group"
              aria-labelledby={`settings-band-${band}`}
            >
              <span
                id={`settings-band-${band}`}
                className="settings-band-title"
              >
                {text.bands[band]}
              </span>
              {sections.map((each) => (
                <button
                  key={each}
                  type="button"
                  className="settings-nav-item"
                  data-nav-section={each}
                  tabIndex={each === section ? 0 : -1}
                  aria-current={each === section ? "page" : undefined}
                  onKeyDown={moveInNav}
                  onClick={(event) => open(each, event.detail === 0)}
                >
                  <Plate section={each} />
                  {text.sections[each]}
                </button>
              ))}
            </div>
          ))
        )}
      </nav>
      <main ref={main} className="settings-main">
        <section
          key={section}
          className="panel settings-card"
          data-section={section}
          aria-labelledby="panel-title"
        >
          <header className="section-header">
            <Plate section={section} large />
            <div className="section-header-text">
              <h1 id="panel-title" tabIndex={-1}>
                {text.sections[section]}
              </h1>
              <p className="section-lead">{text.leads[section]}</p>
            </div>
          </header>
          <SectionIndex
            label={text.contents}
            groups={sectionGroups(index, section)}
            advanced={text.groups.advanced}
            onJump={jump}
          />
          <NamedBySection.Provider value={true}>
            {section === "general" ? (
              <SettingsPanel
                settings={settings}
                widgets={widgets}
                apps={apps}
                withTrayInDock
                withLauncher
                grouped
              />
            ) : null}
            {section === "behavior" || section === "launcher" ? (
              <SettingsPanel
                settings={settings}
                withLauncher
                grouped
                part={section}
              />
            ) : null}
            {section === "workspaces" ? (
              <>
                <WorkspacesPanel
                  snapshot={spaces}
                  selected={editing}
                  onSelect={setChosen}
                  onEditApps={() => setSection("apps")}
                  onEditItems={() => setSection("items")}
                />
                <MonitorsSection snapshot={spaces} />
              </>
            ) : null}
            {section === "look" ? (
              <LookPanel settings={settings} appIds={pinnedInUse} grouped />
            ) : null}
            {section === "apps" ? (
              <>
                <WorkspacePicker
                  workspaces={all}
                  selected={editing}
                  onSelect={setChosen}
                />
                <AppsPanel snapshot={apps} workspace={editing} browse />
              </>
            ) : null}
            {section === "items" ? (
              <>
                <WorkspacePicker
                  workspaces={all}
                  selected={editing}
                  onSelect={setChosen}
                />
                <DockItems snapshot={widgets} workspace={editing} catalog />
                <WidgetCatalog snapshot={widgets} workspace={editing} />
              </>
            ) : null}
            {section === "accounts" ? <AccountsPanel /> : null}
            {section === "about" ? (
              <>
                <AboutPanel />
                <SettingsCopy />
              </>
            ) : null}
          </NamedBySection.Provider>
        </section>
      </main>
    </div>
  );
}

export default SettingsApp;

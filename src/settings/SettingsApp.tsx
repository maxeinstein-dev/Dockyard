import { useEffect, useRef, useState } from "react";
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
import SettingsCopy from "./SettingsCopy";
import WidgetCatalog from "./WidgetCatalog";
import DockItems from "../panel/DockItems";
import LookPanel from "../panel/LookPanel";
import SettingsPanel from "../panel/SettingsPanel";
import WorkspacePicker from "../panel/WorkspacePicker";
import WorkspacesPanel from "../panel/WorkspacesPanel";
import Icon from "../panel/parts/Icon";
import { NamedBySection } from "../panel/parts/focus";
import { inWorkspace } from "../core/workspaces";
import {
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
  type SearchEntry,
} from "./search";
import "./SettingsApp.css";
import "./SettingsSections.css";

/** The glaze of each section's plate (spec 0014 R1; DESIGN.md "Tertiary",
 * the static .t-* classes of tokens.css). */
const PLATES: Readonly<Record<Section, string>> = {
  general: "t-settings",
  workspaces: "t-clock",
  look: "t-music",
  apps: "t-files",
  items: "t-notes",
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
  const found = searchSettings(
    query,
    searchIndex(t),
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
                onClick={() => setSection(each)}
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
          SECTIONS.map((each) => (
            <button
              key={each}
              type="button"
              className="settings-nav-item"
              aria-current={each === section ? "page" : undefined}
              onClick={() => setSection(each)}
            >
              <Plate section={each} />
              {text.sections[each]}
            </button>
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
              <h1 id="panel-title">{text.sections[section]}</h1>
              <p className="section-lead">{text.leads[section]}</p>
            </div>
          </header>
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
            {section === "workspaces" ? (
              <WorkspacesPanel
                snapshot={spaces}
                selected={editing}
                onSelect={setChosen}
                onEditApps={() => setSection("apps")}
                onEditItems={() => setSection("items")}
              />
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

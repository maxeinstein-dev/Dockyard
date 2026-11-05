import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import { en } from "../i18n/en";
import { ptBR } from "../i18n/pt-BR";
import { forgetAdvanced } from "../panel/parts/advanced";
import SettingsApp from "./SettingsApp";
import {
  ROW_LABELS,
  findRow,
  normalize,
  searchIndex,
  searchSettings,
} from "./search";
import { SECTIONS } from "./sections";

const text = ptBR.settings.app;

const { invoke, close } = vi.hoisted(() => ({
  invoke: vi.fn(),
  close: vi.fn(() => Promise.resolve()),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ close }),
}));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const SETTINGS: Settings = {
  version: 1,
  theme: "system",
  launchAtLogin: false,
  launcher: true,
  launcherKeywords: [],
  launcherHotkey: "Alt+Space",
  dockHotkey: "Alt+Win+Z",
  language: "system",
  tourShown: true,
  powerSaver: "onBattery",
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
    // Every switch that shows more rows on, so the guard sees them too
    // (spec 0029 §8.2).
    autoHide: true,
    previews: true,
    hover: "scale",
  },
};

const WORKSPACES = {
  inUse: "a",
  workspaces: [
    {
      id: "a",
      name: "Principal",
      color: "blue",
      items: [],
      stacks: [],
      spacers: [],
    },
  ],
};

async function renderApp() {
  const view = render(<SettingsApp />);
  await act(async () => {});
  return view;
}

const nav = () => screen.getByRole("navigation", { name: text.nav });
const searchBox = () =>
  within(nav()).getByRole("searchbox", { name: text.search.label });

beforeEach(() => {
  delete window.__DOCKYARD_SECTION__;
  forgetAdvanced();
  close.mockClear();
  invoke.mockReset();
  invoke.mockImplementation((command: string) => {
    if (command === "get_settings") return Promise.resolve(SETTINGS);
    if (command === "list_widgets") {
      return Promise.resolve({ storage: { state: "ok" }, widgets: [] });
    }
    if (command === "list_apps") {
      return Promise.resolve({ storage: { state: "ok" }, apps: [] });
    }
    if (command === "list_workspaces") return Promise.resolve(WORKSPACES);
    // Contas with no client: its fields show.
    if (command === "google_status") {
      return Promise.resolve({ clientId: null, accounts: [] });
    }
    return new Promise(() => {});
  });
  Element.prototype.scrollIntoView = vi.fn();
});

afterEach(() => {
  delete window.__DOCKYARD_SECTION__;
});

describe("the index of the search", () => {
  // Guard: every row each section shows is in the index, and every entry
  // of the index is a row of its section. A new field left out, or a
  // renamed one, fails here.
  it.each(SECTIONS)("matches the rows of %s", async (section) => {
    window.__DOCKYARD_SECTION__ = section;
    const { container } = await renderApp();
    const main = container.querySelector(".settings-main");
    expect(main).not.toBeNull();
    const shown = Array.from(main!.querySelectorAll(ROW_LABELS)).map((each) =>
      normalize(each.textContent ?? ""),
    );
    const indexed = searchIndex(ptBR)
      .filter((entry) => entry.section === section)
      .map((entry) => entry);
    for (const entry of indexed) {
      expect(findRow(main!, entry.label), entry.label).not.toBeNull();
    }
    // Every section shows something to find.
    expect(indexed.length, section).toBeGreaterThan(0);
    for (const name of shown) {
      const covered = indexed.some((entry) => {
        const label = normalize(entry.label);
        return name === label || name.startsWith(`${label} (`);
      });
      expect(covered, name).toBe(true);
    }
    cleanup();
  });

  it("has an entry for each name, once per section", () => {
    const index = searchIndex(ptBR);
    const keys = index.map((entry) => `${entry.section}:${entry.label}`);
    expect(new Set(keys).size).toBe(keys.length);
    expect(searchIndex(en)).toHaveLength(index.length);
  });
});

describe("searchSettings", () => {
  const find = (query: string) =>
    searchSettings(
      query,
      searchIndex(ptBR),
      text.sections,
      text.leads,
      SECTIONS,
    );

  it("finds nothing for an empty query", () => {
    expect(find("   ")).toEqual({ sections: [], entries: [] });
  });

  it("ignores accents and case", () => {
    const labels = find("PREVIA").entries.map((entry) => entry.label);
    expect(labels).toContain(ptBR.settings.previews);
    expect(labels).toContain(ptBR.settings.previewDelay);
  });

  it("needs every word, in any field", () => {
    const labels = find("indicador cor").entries.map((entry) => entry.label);
    expect(labels).toEqual([
      ptBR.settings.look.indicatorColor,
      ptBR.settings.indicatorFollowsWorkspace,
    ]);
  });

  it("finds a section by its name or its lead", () => {
    expect(find("contas").sections).toEqual(["accounts"]);
    expect(find("licença").sections).toEqual(["about"]);
  });

  it("finds a widget of the catalog by its name", () => {
    expect(find("claude").entries).toEqual([
      {
        section: "items",
        label: ptBR.widgets.kinds.claude,
        group: ptBR.widgets.catalog.title,
      },
    ]);
  });

  it("finds the launcher's switch by its name without the hotkey", () => {
    const entries = find("lançador").entries;
    expect(entries.map((entry) => entry.label)).toContain("Lançador");
  });
});

describe("the search of Configurações", () => {
  it("swaps the sections for the results while it holds something", async () => {
    await renderApp();
    fireEvent.change(searchBox(), { target: { value: "bateria" } });

    const results = within(nav()).getByRole("group", {
      name: text.search.results,
    });
    expect(
      within(results).getByRole("button", {
        name: `${ptBR.settings.groups.powerSaver} ${text.sections.behavior} › ${text.groups.energy}`,
      }),
    ).toBeTruthy();
    expect(
      within(nav()).queryByRole("button", { name: text.sections.about }),
    ).toBeNull();

    fireEvent.change(searchBox(), { target: { value: "" } });
    expect(
      within(nav()).getByRole("button", { name: text.sections.about }),
    ).toBeTruthy();
  });

  it("says when nothing matches", async () => {
    await renderApp();
    fireEvent.change(searchBox(), { target: { value: "xyzzy" } });
    expect(screen.getByRole("status").textContent).toBe(
      "Nenhum ajuste com “xyzzy”.",
    );
  });

  it("opens Avançado for a row inside it (spec 0029 §8.1)", async () => {
    await renderApp();
    fireEvent.change(searchBox(), { target: { value: "botao do meio" } });
    fireEvent.click(
      within(nav()).getByRole("button", {
        name: new RegExp(`^${ptBR.settings.groups.middleClick}`),
      }),
    );
    await act(async () => {});

    expect(
      screen.getByRole("button", {
        name: text.groups.advanced,
        expanded: true,
      }),
    ).toBeInTheDocument();
    const row = document.querySelector(".search-hit");
    expect(row?.textContent).toContain(ptBR.settings.groups.middleClick);
    expect(row?.contains(document.activeElement)).toBe(true);
  });

  it("goes to the option's section, marks the row and focuses it", async () => {
    await renderApp();
    fireEvent.change(searchBox(), { target: { value: "reflexos" } });
    fireEvent.click(
      within(nav()).getByRole("button", {
        name: new RegExp(`^${ptBR.settings.look.reflections}`),
      }),
    );
    await act(async () => {});

    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe(
      text.sections.look,
    );
    const row = document.querySelector(".search-hit");
    expect(row?.textContent).toContain(ptBR.settings.look.reflections);
    expect(row?.contains(document.activeElement)).toBe(true);
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();
  });

  it("empties with Esc before Esc closes the window", async () => {
    await renderApp();
    fireEvent.change(searchBox(), { target: { value: "tema" } });
    fireEvent.keyDown(searchBox(), { key: "Escape" });
    expect((searchBox() as HTMLInputElement).value).toBe("");
    expect(close).not.toHaveBeenCalled();

    fireEvent.keyDown(searchBox(), { key: "Escape" });
    expect(close).toHaveBeenCalled();
  });

  it("takes the focus with Ctrl+F", async () => {
    await renderApp();
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(document.activeElement).toBe(searchBox());
  });
});

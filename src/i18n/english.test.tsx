import { act, fireEvent, render } from "@testing-library/react";
import {
  afterAll,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import SettingsApp from "../settings/SettingsApp";
import TourApp from "../tour/TourApp";
import { en } from "./en";
import { setLanguage } from "./index";
import { ptBR } from "./pt-BR";

/* Draws Configurações and the tour in English and fails on any Portuguese
 * left on screen: a text written in the JSX instead of read from the
 * messages shows up here, since `en.ts` has no say over it. */

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ close: () => Promise.resolve() }),
}));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

/** Texts that look Portuguese on purpose, each with why. */
const ALLOWED: Readonly<Record<string, string>> = {
  // The language picker names each language in its own tongue.
  Português: "a language's own name",
};

/** Letters English never uses and Portuguese does. */
const PT_LETTERS = /[ãõçáàâêéíóôúÃÕÇÁÀÂÊÉÍÓÔÚ]/;

/** Short Portuguese words with no English homograph, as whole words (so
 * "github.com" or "de-DE" do not count). */
const PT_WORDS =
  /(^|\s)(de|da|dos|das|para|com|uma|um|não|você|seu|sua|pelo|pela|ao|aos|ou|em)(?=$|[\s,.;:!?…])/i;

/** A decimal comma ("1,10×"): a number formatted the PT-BR way by hand.
 * A thousands separator has three digits after it, so it does not count. */
const PT_DECIMAL = /\d,\d{1,2}(?!\d)/;

/** Every string of a messages tree. */
function leaves(tree: unknown): string[] {
  if (typeof tree === "string") return [tree];
  if (tree !== null && typeof tree === "object") {
    return Object.values(tree).flatMap(leaves);
  }
  return [];
}

/** Whole PT-BR messages English does not share (templates aside: their
 * filled form is caught by the letters and words above). */
const PT_ONLY = new Set(
  leaves(ptBR).filter(
    (text) => !text.includes("{") && !new Set(leaves(en)).has(text),
  ),
);

/** Why `text` reads as Portuguese, or null. */
function portuguese(text: string): string | null {
  if (Object.prototype.hasOwnProperty.call(ALLOWED, text)) return null;
  if (PT_ONLY.has(text)) return "a PT-BR message";
  if (PT_LETTERS.test(text)) return "Portuguese letters";
  const word = PT_WORDS.exec(text);
  if (word) return `the word "${word[2] ?? ""}"`;
  if (PT_DECIMAL.test(text)) return "a decimal comma";
  return null;
}

const SHOWN_ATTRIBUTES = [
  "aria-label",
  "aria-valuetext",
  "title",
  "placeholder",
  "alt",
];

/** The texts of the page a person reads or hears: its text nodes and the
 * attributes that name or explain something. */
function shownTexts(root: HTMLElement): string[] {
  const texts: string[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node.nodeValue?.trim();
    if (text && node.parentElement?.closest("style, script") == null) {
      texts.push(text);
    }
  }
  for (const element of root.querySelectorAll("*")) {
    for (const name of SHOWN_ATTRIBUTES) {
      const value = element.getAttribute(name)?.trim();
      if (value) texts.push(value);
    }
  }
  return texts;
}

/** Each Portuguese text on the page, as `where: "text" (why)`. */
function leftovers(where: string): string[] {
  return [...new Set(shownTexts(document.body))].flatMap((text) => {
    const why = portuguese(text);
    return why ? [`${where}: "${text}" (${why})`] : [];
  });
}

const SETTINGS: Settings = {
  version: 1,
  theme: "system",
  launchAtLogin: false,
  launcher: true,
  launcherKeywords: [],
  launcherHotkey: "Alt+Space",
  dockHotkey: "Alt+Win+Z",
  language: "en",
  tourShown: true,
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
  },
};

/** What Rust answers: nothing stored, or a bit of everything (names in
 * English, as a user's would be). */
const WORLDS = {
  empty: {
    get_settings: SETTINGS,
    list_apps: { storage: { state: "ok" }, apps: [] },
    list_widgets: { storage: { state: "ok" }, widgets: [] },
    list_workspaces: { workspaces: [], inUse: null },
    list_catalog: [],
    list_icon_packs: [],
    launcher_status: { hotkeyTaken: false },
    google_status: { clientId: null, accounts: [] },
    update_status: { state: "none" },
  },
  filled: {
    get_settings: SETTINGS,
    list_apps: {
      storage: { state: "ok" },
      apps: [{ id: "a1", appId: "Notepad", name: "Notepad" }],
    },
    list_widgets: {
      storage: { state: "ok" },
      widgets: [
        {
          id: "c1",
          label: null,
          kind: "clock",
          config: {
            timeZone: null,
            hourCycle: "h23",
            showSeconds: false,
            iconStyle: "digital",
          },
        },
        { id: "n1", label: null, kind: "note", config: {} },
      ],
    },
    list_workspaces: {
      workspaces: [
        {
          id: "w1",
          name: "Main",
          color: "#0A84FF",
          items: ["app:a1", "widget:c1", "widget:n1"],
          spacers: [],
        },
        { id: "w2", name: "Work", color: "#FF9F0A", items: [], spacers: [] },
      ],
      inUse: "w1",
    },
    list_catalog: [{ appId: "Calculator", name: "Calculator" }],
    list_icon_packs: [],
    launcher_status: { hotkeyTaken: true },
    google_status: {
      clientId: "1.apps.googleusercontent.com",
      accounts: ["someone@example.com"],
    },
    update_status: { state: "available", version: "9.9.9", notes: null },
  },
} satisfies Record<string, Record<string, unknown>>;

function answer(world: Record<string, unknown>) {
  invoke.mockImplementation((command: string) =>
    Object.prototype.hasOwnProperty.call(world, command)
      ? Promise.resolve(world[command])
      : // Commands the sweep does not need stay pending.
        new Promise(() => {}),
  );
}

const settle = () => act(async () => {});

beforeAll(() => setLanguage("en"));
// The other tests read PT-BR (src/test/setup.ts).
afterAll(() => setLanguage("pt-BR"));

beforeEach(() => {
  delete window.__DOCKYARD_SECTION__;
  invoke.mockReset();
});

describe("the interface in English", () => {
  it("knows Portuguese when it sees it", () => {
    expect(portuguese(ptBR.settings.app.sections.workspaces)).not.toBeNull();
    expect(portuguese("Voltar")).not.toBeNull();
    expect(portuguese("1 de 6")).not.toBeNull();
    expect(portuguese("1,10×")).not.toBeNull();
    expect(portuguese("1,000 apps")).toBeNull();
    expect(portuguese("Português")).toBeNull();
    expect(portuguese(en.settings.app.sections.workspaces)).toBeNull();
    expect(portuguese(en.about.repository)).toBeNull();
  });

  it.each(Object.entries(WORLDS))(
    "shows every section of Configurações with no Portuguese (%s)",
    async (_name, world) => {
      answer(world);
      render(<SettingsApp />);
      await settle();

      const found: string[] = [];
      const sections = [
        ...document.querySelectorAll<HTMLButtonElement>(".settings-nav-item"),
      ];
      expect(sections).toHaveLength(7);
      for (const button of sections) {
        fireEvent.click(button);
        await settle();
        const where = button.textContent ?? "";
        found.push(...leftovers(where));
        // Open what opens in place (a disclosure), and read it too.
        for (const closed of document.querySelectorAll<HTMLElement>(
          ".settings-card [aria-expanded='false']",
        )) {
          fireEvent.click(closed);
          await settle();
        }
        found.push(...leftovers(`${where} (opened)`));
      }

      expect([...new Set(found)]).toEqual([]);
    },
  );

  it("walks the tour with no Portuguese", async () => {
    answer(WORLDS.filled);
    render(<TourApp />);
    await settle();

    const found: string[] = [];
    for (let step = 1; ; step++) {
      found.push(...leftovers(`tour step ${step}`));
      const next = [...document.querySelectorAll("button")].find(
        (button) => button.textContent === en.tour.next,
      );
      if (!next) break;
      fireEvent.click(next);
    }

    expect(found).toEqual([]);
    expect(document.body).toHaveTextContent(en.tour.openSettings);
  });
});

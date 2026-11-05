import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import { ptBR } from "../i18n/pt-BR";
import { forgetAdvanced } from "../panel/parts/advanced";
import SettingsApp from "./SettingsApp";

const text = ptBR.settings.app;

type Listener = (event: { payload: unknown }) => void;

const { invoke, close, listeners } = vi.hoisted(() => ({
  invoke: vi.fn(),
  close: vi.fn(() => Promise.resolve()),
  listeners: new Map<string, Listener>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((event: string, handler: Listener) => {
    listeners.set(event, handler);
    return Promise.resolve(() => {});
  }),
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
  },
};

async function renderApp() {
  const view = render(<SettingsApp />);
  await act(async () => {});
  return view;
}

const nav = () => screen.getByRole("navigation", { name: text.nav });
const sectionButton = (name: string) =>
  within(nav()).getByRole("button", { name });
/** Opens the Avançado of the section shown. */
const openAdvancedGroup = () =>
  fireEvent.click(
    screen.getByRole("button", { name: text.groups.advanced, expanded: false }),
  );

describe("SettingsApp", () => {
  beforeEach(() => {
    delete window.__DOCKYARD_SECTION__;
    forgetAdvanced();
    listeners.clear();
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
      return new Promise(() => {});
    });
  });

  it("lists its sections in the side bar, Geral first and chosen", async () => {
    await renderApp();

    expect(
      within(nav())
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual([
      "Geral",
      "Aparência",
      "Comportamento",
      "Espaços",
      "Apps",
      "Widgets",
      "Lançador",
      "Contas",
      "Sobre",
    ]);
    expect(sectionButton("Geral")).toHaveAttribute("aria-current", "page");
    expect(
      screen.getByRole("group", { name: ptBR.settings.groups.theme }),
    ).toBeInTheDocument();
  });

  it("heads each section with its plate, title and line", async () => {
    await renderApp();

    for (const button of within(nav()).getAllByRole("button")) {
      // The plate is decorative: the button keeps the section's name.
      expect(button.querySelector(".section-plate")).toHaveAttribute(
        "aria-hidden",
        "true",
      );
    }
    expect(document.querySelector(".section-header")).toHaveTextContent(
      text.leads.general,
    );
    expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
    expect(
      screen.getByRole("heading", { level: 1, name: "Geral" }),
    ).toBeInTheDocument();
  });

  it("shows the position and the taskbar as cards with a picture", async () => {
    await renderApp();

    const edges = screen.getByRole("radiogroup", {
      name: ptBR.settings.groups.edge,
    });
    expect(
      within(edges).getByRole("radio", { name: "Embaixo" }),
    ).toHaveAttribute("aria-checked", "true");
    expect(
      within(edges)
        .getByRole("radio", { name: "Em cima" })
        .querySelector(".thumb-screen"),
    ).toHaveAttribute("data-edge", "top");

    const taskbar = within(
      screen.getByRole("radiogroup", { name: ptBR.settings.groups.taskbar }),
    ).getByRole("radio", { name: "Visível" });
    expect(taskbar).toHaveAccessibleDescription(
      ptBR.settings.taskbarHints.visible,
    );
    expect(taskbar).toHaveAttribute("aria-checked", "false");
    fireEvent.click(taskbar);
    await act(async () => {});
    const saved = invoke.mock.calls.find(
      ([command]) => command === "update_settings",
    )?.[1] as { settings: Settings } | undefined;
    expect(saved?.settings.dock.taskbar).toBe("visible");
  });

  it("shows the look's choices as cards, the chosen one moving", async () => {
    await renderApp();
    fireEvent.click(sectionButton("Aparência"));
    await act(async () => {});
    openAdvancedGroup();

    const look = ptBR.settings.look;
    for (const [group, chosen] of [
      [look.hover, look.hoverOptions.scale],
      [look.indicator, look.indicatorOptions.dot],
      [look.iconShape, look.iconShapeOptions.plate],
      [look.openAnimation, look.openAnimationOptions.bounce],
    ] as const) {
      const card = within(
        screen.getByRole("radiogroup", { name: group }),
      ).getByRole("radio", { name: chosen });
      expect(card, group).toHaveAttribute("aria-checked", "true");
      expect(card.querySelector(".thumb"), group).not.toBeNull();
    }
    const bounce = within(
      screen.getByRole("radiogroup", { name: look.openAnimation }),
    ).getByRole("radio", { name: look.openAnimationOptions.bounce });
    expect(bounce.querySelector(".thumb")).toHaveAttribute("data-chosen");
    const shake = within(
      screen.getByRole("radiogroup", { name: look.openAnimation }),
    ).getByRole("radio", { name: look.openAnimationOptions.shake });
    expect(shake.querySelector(".thumb")).not.toHaveAttribute("data-chosen");
  });

  it("puts Geral in titled groups (spec 0029 §2)", async () => {
    await renderApp();

    expect(
      screen
        .getAllByRole("heading", { level: 2 })
        .map((heading) => heading.textContent),
    ).toEqual([text.groups.dock, text.groups.taskbar, text.groups.startup]);
    const startup = within(
      screen.getByRole("region", { name: text.groups.startup }),
    );
    expect(
      startup.getByRole("switch", { name: ptBR.settings.launchAtLogin }),
    ).toBeInTheDocument();
    expect(
      startup.getByRole("group", { name: ptBR.settings.groups.language }),
    ).toBeInTheDocument();
  });

  it("splits the old Geral into Comportamento and Lançador", async () => {
    await renderApp();
    fireEvent.click(sectionButton("Comportamento"));
    await act(async () => {});

    expect(
      screen
        .getAllByRole("heading", { level: 2 })
        .map((heading) => heading.textContent),
    ).toEqual([
      text.groups.showHide,
      text.groups.previews,
      text.groups.appsInDock,
      text.groups.energy,
      text.groups.monitors,
      text.groups.advanced,
    ]);
    // The dock's hotkey (M14), in Avançado.
    openAdvancedGroup();
    expect(
      within(
        screen.getByRole("region", { name: text.groups.advanced }),
      ).getByRole("button", {
        name: "Alterar o atalho do dock, hoje Alt+Win+Z",
      }),
    ).toBeInTheDocument();

    fireEvent.click(sectionButton("Lançador"));
    await act(async () => {});
    expect(
      screen
        .getAllByRole("heading", { level: 2 })
        .map((heading) => heading.textContent),
    ).toEqual([
      text.groups.launcher,
      ptBR.settings.launcher.keywords.label,
      text.groups.advanced,
    ]);
    openAdvancedGroup();
    expect(
      within(
        screen.getByRole("region", { name: text.groups.advanced }),
      ).getByRole("button", { name: ptBR.settings.launcher.clear }),
    ).toBeInTheDocument();
  });

  it("keeps Avançado closed at first, saying what it holds (spec 0029 §4)", async () => {
    await renderApp();
    fireEvent.click(sectionButton("Comportamento"));
    await act(async () => {});

    const toggle = screen.getByRole("button", {
      name: text.groups.advanced,
      expanded: false,
    });
    expect(toggle).toHaveAccessibleDescription(text.advancedSummary.behavior);
    expect(
      screen.queryByRole("group", {
        name: ptBR.settings.groups.middleClick,
      }),
    ).toBeNull();

    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.getByRole("group", { name: ptBR.settings.groups.middleClick }),
    ).toBeInTheDocument();

    // Open while the window lives: away and back, it stays open.
    fireEvent.click(sectionButton("Geral"));
    fireEvent.click(sectionButton("Comportamento"));
    await act(async () => {});
    expect(
      screen.getByRole("button", {
        name: text.groups.advanced,
        expanded: true,
      }),
    ).toBeInTheDocument();
  });

  it("opens Avançado from the index, its button taking the focus", async () => {
    Element.prototype.scrollIntoView = vi.fn();
    await renderApp();
    fireEvent.click(sectionButton("Aparência"));
    await act(async () => {});

    const index = screen.getByRole("navigation", { name: text.contents });
    fireEvent.click(
      within(index).getByRole("button", { name: text.groups.advanced }),
    );
    const toggle = screen.getByRole("button", {
      name: text.groups.advanced,
      expanded: true,
    });
    expect(document.activeElement).toBe(toggle);
    expect(
      screen.getByRole("radiogroup", {
        name: ptBR.settings.look.openAnimation,
      }),
    ).toBeInTheDocument();
  });

  it("leaves the folders in Avançado, their problems under the styles", async () => {
    await renderApp();
    fireEvent.click(sectionButton("Aparência"));
    await act(async () => {});

    const look = ptBR.settings.look;
    expect(
      screen.queryByRole("button", { name: look.openSkinsFolder }),
    ).toBeNull();
    expect(screen.queryByRole("button", { name: look.reloadPacks })).toBeNull();
    openAdvancedGroup();
    const advanced = within(
      screen.getByRole("region", { name: text.groups.advanced }),
    );
    expect(
      advanced.getByRole("button", { name: look.openSkinsFolder }),
    ).toBeInTheDocument();
    expect(
      advanced.getByRole("button", { name: look.openPacksFolder }),
    ).toBeInTheDocument();
    expect(
      advanced.getByRole("button", { name: look.reloadPacks }),
    ).toBeInTheDocument();
  });

  it("puts the indicator in the workspace color beside its color", async () => {
    await renderApp();
    fireEvent.click(sectionButton("Aparência"));
    await act(async () => {});

    expect(
      within(
        screen.getByRole("region", { name: text.groups.indicator }),
      ).getByRole("switch", { name: ptBR.settings.indicatorFollowsWorkspace }),
    ).toBeInTheDocument();
  });

  it("names its bands, a group each (spec 0029 §8.4)", async () => {
    await renderApp();

    for (const [band, first] of [
      [text.bands.dock, "Geral"],
      [text.bands.content, "Espaços"],
      [text.bands.more, "Lançador"],
    ] as const) {
      const group = within(nav()).getByRole("group", { name: band });
      expect(within(group).getAllByRole("button")[0]).toHaveTextContent(first);
    }
  });

  it("moves among the sections with the arrows, one stop of Tab", async () => {
    await renderApp();

    const buttons = within(nav()).getAllByRole("button");
    expect(buttons.filter((button) => button.tabIndex === 0)).toEqual([
      sectionButton("Geral"),
    ]);
    sectionButton("Geral").focus();
    fireEvent.keyDown(document.activeElement!, { key: "ArrowDown" });
    expect(document.activeElement).toBe(sectionButton("Aparência"));
    fireEvent.keyDown(document.activeElement!, { key: "End" });
    expect(document.activeElement).toBe(sectionButton("Sobre"));
    fireEvent.keyDown(document.activeElement!, { key: "ArrowDown" });
    expect(document.activeElement).toBe(sectionButton("Geral"));
    fireEvent.keyDown(document.activeElement!, { key: "ArrowUp" });
    expect(document.activeElement).toBe(sectionButton("Sobre"));
    fireEvent.keyDown(document.activeElement!, { key: "Home" });
    expect(document.activeElement).toBe(sectionButton("Geral"));
    // Moving is not opening.
    expect(sectionButton("Geral")).toHaveAttribute("aria-current", "page");
  });

  it("takes the focus to the title of a section opened by the keyboard", async () => {
    await renderApp();

    // A click with no pointer: Enter or Space.
    fireEvent.click(sectionButton("Contas"), { detail: 0 });
    await act(async () => {});
    expect(document.activeElement).toBe(
      screen.getByRole("heading", { level: 1, name: "Contas" }),
    );
    expect(sectionButton("Contas").tabIndex).toBe(0);

    fireEvent.click(sectionButton("Sobre"), { detail: 1 });
    await act(async () => {});
    expect(document.activeElement).not.toBe(
      screen.getByRole("heading", { level: 1, name: "Sobre" }),
    );
  });

  it("indexes the groups of a section with 3 or more (spec 0029 §3)", async () => {
    Element.prototype.scrollIntoView = vi.fn();
    await renderApp();
    fireEvent.click(sectionButton("Comportamento"));
    await act(async () => {});

    const index = screen.getByRole("navigation", { name: text.contents });
    expect(
      within(index)
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual([
      text.groups.showHide,
      text.groups.previews,
      text.groups.appsInDock,
      text.groups.energy,
      text.groups.monitors,
      text.groups.advanced,
    ]);
    fireEvent.click(
      within(index).getByRole("button", { name: text.groups.energy }),
    );
    expect(document.activeElement).toBe(
      screen.getByRole("heading", { level: 2, name: text.groups.energy }),
    );
    expect(Element.prototype.scrollIntoView).toHaveBeenCalled();

    // Lançador has two groups besides Avançado: no index.
    fireEvent.click(sectionButton("Lançador"));
    await act(async () => {});
    expect(
      screen.queryByRole("navigation", { name: text.contents }),
    ).toBeNull();
  });

  it("shows the styles with the programs of the workspace in use", async () => {
    invoke.mockImplementation((command: string, args?: { appId?: string }) => {
      if (command === "get_settings") return Promise.resolve(SETTINGS);
      if (command === "list_apps") {
        return Promise.resolve({
          storage: { state: "ok" },
          apps: [
            { id: "a1", appId: "Edge", name: "Edge" },
            { id: "a2", appId: "Code", name: "Code" },
          ],
        });
      }
      if (command === "list_workspaces") {
        return Promise.resolve({
          workspaces: [
            {
              id: "w1",
              name: "Principal",
              color: "#0A84FF",
              items: [],
              spacers: [],
              stacks: [],
            },
            {
              id: "w2",
              name: "Trabalho",
              color: "#FF9F0A",
              items: ["app:a2"],
              spacers: [],
              stacks: [],
            },
          ],
          inUse: "w2",
        });
      }
      if (command === "app_icon") {
        return Promise.resolve(`data:image/png;base64,${args?.appId ?? ""}`);
      }
      return new Promise(() => {});
    });
    await renderApp();

    fireEvent.click(sectionButton("Aparência"));
    await act(async () => {});

    const card = within(
      screen.getByRole("radiogroup", { name: ptBR.settings.look.style }),
    ).getByRole("radio", { name: "Neon" });
    // Only the program of Trabalho, the workspace in use.
    expect(
      [...card.querySelectorAll("img")].map((img) => img.getAttribute("src")),
    ).toEqual(["data:image/png;base64,Code"]);
  });

  it("has no rows that open subviews: the sections are in the side bar", async () => {
    await renderApp();

    expect(
      screen.queryByRole("button", { name: /Aparência do dock.*Nativo/ }),
    ).toBeNull();
  });

  it("shows the look of the dock with no Voltar, at a width that fits it", async () => {
    await renderApp();

    fireEvent.click(sectionButton("Aparência"));

    expect(sectionButton("Aparência")).toHaveAttribute("aria-current", "page");
    expect(sectionButton("Geral")).not.toHaveAttribute("aria-current");
    expect(
      screen.getByRole("heading", { level: 1, name: text.sections.look }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Voltar" })).toBeNull();
    expect(document.querySelector(".settings-card")).toHaveAttribute(
      "data-section",
      "look",
    );
  });

  it.each([
    ["Apps", text.sections.apps],
    ["Widgets", ptBR.settings.app.sections.items],
    ["Sobre", ptBR.panel.titles.about],
  ])("shows %s under its own title", async (section, title) => {
    await renderApp();

    fireEvent.click(sectionButton(section));

    expect(
      screen.getByRole("heading", { level: 1, name: title }),
    ).toBeInTheDocument();
  });

  it("starts on the section the launcher asked for", async () => {
    window.__DOCKYARD_SECTION__ = "items";
    await renderApp();

    expect(sectionButton("Widgets")).toHaveAttribute("aria-current", "page");
  });

  it("starts on Geral when asked for a section it does not have", async () => {
    window.__DOCKYARD_SECTION__ = "developer";
    await renderApp();

    expect(sectionButton("Geral")).toHaveAttribute("aria-current", "page");
  });

  it("shows the section the launcher found while open", async () => {
    await renderApp();

    act(() => listeners.get("settings-section")?.({ payload: "about" }));
    expect(sectionButton("Sobre")).toHaveAttribute("aria-current", "page");
    act(() => listeners.get("settings-section")?.({ payload: "nope" }));
    expect(sectionButton("Sobre")).toHaveAttribute("aria-current", "page");
  });

  it("closes its window with Esc", async () => {
    await renderApp();

    fireEvent.keyDown(window, { key: "Escape" });

    expect(close).toHaveBeenCalledTimes(1);
  });
});

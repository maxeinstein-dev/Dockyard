import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppsSnapshot, PinnedApp } from "../core/apps";
import type { OpenNow, OpenOther } from "../core/openApps";
import {
  appPanel,
  otherPanel,
  widgetPanel,
  type PanelId,
} from "../core/panels";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import type {
  CreatableKind,
  WidgetInstance,
  WidgetsSnapshot,
} from "../core/widgets";
import { WIDGET_FIXTURES } from "../core/widgets.fixtures";
import type { WorkspacesSnapshot } from "../core/workspaces";
import type {
  WidgetPanelProps,
  WidgetSettingsProps,
} from "../widgets/contract";
import PanelContent from "./PanelContent";
import {
  APPS_VIEW,
  BACK_FROM_APPS_VIEW,
  BACK_FROM_LOOK_VIEW,
  BACK_VIEW,
  LOOK_VIEW,
  MAIN_VIEW,
  SUB_VIEW,
  type PanelView,
} from "./view";

/* PanelContent's routing alone: which view each panel and view shows, and
 * what it hands it. Every view is a stand-in that shows its name and keeps
 * the props it got. */

const { invoke, got, standIn } = vi.hoisted(() => {
  /** The last props each stand-in got, by its name. */
  const got = new Map<string, Record<string, unknown>>();
  function standIn(name: string) {
    return function StandIn(props: Record<string, unknown>) {
      got.set(name, props);
      return <p data-testid="view">{name}</p>;
    };
  }
  return { invoke: vi.fn(), got, standIn };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("./AboutPanel", () => ({ default: standIn("AboutPanel") }));
vi.mock("./AppMenuPanel", () => ({ default: standIn("AppMenuPanel") }));
vi.mock("./AppsPanel", () => ({ default: standIn("AppsPanel") }));
vi.mock("./OpenMenuPanel", () => ({ default: standIn("OpenMenuPanel") }));
vi.mock("./DockItems", () => ({ default: standIn("DockItems") }));
vi.mock("./SettingsPanel", () => ({ default: standIn("SettingsPanel") }));
vi.mock("./TrayPanel", () => ({ default: standIn("TrayPanel") }));
vi.mock("./LookPanel", () => ({ default: standIn("LookPanel") }));
vi.mock("../widgets/registry", async (importOriginal) => {
  const real = await importOriginal<typeof import("../widgets/registry")>();
  return {
    ...real,
    definitionOf: (instance: WidgetInstance<CreatableKind>) => ({
      ...real.definitionOf(instance),
      Panel: standIn("WidgetPanel"),
      Settings: standIn("WidgetSettings"),
    }),
  };
});

const SETTINGS: Settings = {
  version: 1,
  theme: "system",
  launchAtLogin: false,
  launcher: true,
  launcherKeywords: [],
  launcherHotkey: "Alt+Space",
  language: "system",
  tourShown: true,
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
    trayInDock: ["volume", "wifi"],
  },
};

const CLOCK = WIDGET_FIXTURES["clock-sample"];
const WIDGETS: WidgetsSnapshot = {
  storage: { state: "ok" },
  widgets: [CLOCK],
};
const APP: PinnedApp = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f60a0",
  appId: "Microsoft.WindowsNotepad_8wekyb3d8bbwe!App",
  name: "Bloco de Notas",
};
const APPS: AppsSnapshot = { storage: { state: "ok" }, apps: [APP] };
const OTHER: OpenOther = {
  key: "k1",
  appId: "chrome",
  name: "Chrome",
  count: 2,
  active: false,
  badge: false,
};
const OPEN: OpenNow = {
  apps: new Map([[APP.id, { id: APP.id, count: 3, active: false }]]),
  others: [OTHER],
  loaded: true,
};
const WORKSPACES: WorkspacesSnapshot = { workspaces: [], inUse: null };

type Props = Parameters<typeof PanelContent>[0];

function renderContent(panelId: PanelId, overrides: Partial<Props> = {}) {
  const onView = vi.fn<(view: PanelView) => void>();
  const onRemoved = vi.fn();
  const props: Props = {
    panelId,
    settings: SETTINGS,
    widgets: WIDGETS,
    apps: APPS,
    app: null,
    open: OPEN,
    other: null,
    workspaces: WORKSPACES,
    instance: null,
    view: MAIN_VIEW,
    onView,
    onRemoved,
    ...overrides,
  };
  const view = render(<PanelContent {...props} />);
  return { ...view, onView, onRemoved };
}

/** The name of the view shown, or null for none. */
const shown = () => screen.queryByTestId("view")?.textContent ?? null;

/** The props the stand-in `name` got last. */
function propsOf<P>(name: string): P {
  const props = got.get(name);
  if (!props) throw new Error(`${name} was not rendered`);
  return props as P;
}

/** Calls the prop `key` of the stand-in `name`. */
function call(name: string, key: string) {
  const fn = propsOf<Record<string, unknown>>(name)[key];
  if (typeof fn !== "function") throw new Error(`${name}.${key} missing`);
  return act(async () => {
    await fn();
  });
}

beforeEach(() => {
  got.clear();
  invoke.mockReset();
  invoke.mockResolvedValue(undefined);
});

describe("PanelContent", () => {
  describe("a pinned program's menu", () => {
    it("shows the program's menu with its open windows and the workspaces", async () => {
      const { onRemoved } = renderContent(appPanel(APP.id), { app: APP });

      expect(shown()).toBe("AppMenuPanel");
      expect(propsOf("AppMenuPanel")).toMatchObject({
        app: APP,
        openCount: 3,
        workspaces: WORKSPACES,
      });
      await call("AppMenuPanel", "onDone");
      expect(onRemoved).toHaveBeenCalledTimes(1);
    });

    it("counts no windows for a program that is not open", () => {
      renderContent(appPanel(APP.id), {
        app: APP,
        open: { apps: new Map(), others: [], loaded: true },
      });

      expect(propsOf("AppMenuPanel")).toMatchObject({ openCount: 0 });
    });

    it("shows nothing once the program left the list", () => {
      const { container } = renderContent(appPanel(APP.id), { app: null });

      expect(container).toBeEmptyDOMElement();
    });
  });

  describe("an open program's menu", () => {
    it("shows the program's menu with the workspaces", async () => {
      const { onRemoved } = renderContent(otherPanel("k1"), { other: OTHER });

      expect(shown()).toBe("OpenMenuPanel");
      expect(propsOf("OpenMenuPanel")).toMatchObject({
        other: OTHER,
        workspaces: WORKSPACES,
      });
      await call("OpenMenuPanel", "onDone");
      expect(onRemoved).toHaveBeenCalledTimes(1);
    });

    it("shows nothing once the program closed", () => {
      const { container } = renderContent(otherPanel("k1"), { other: null });

      expect(container).toBeEmptyDOMElement();
    });
  });

  describe("a widget", () => {
    it("shows its main view, with a time and not coming back", async () => {
      const { onView } = renderContent(widgetPanel(CLOCK.id), {
        instance: CLOCK,
      });

      expect(shown()).toBe("WidgetPanel");
      const props = propsOf<WidgetPanelProps<"clock">>("WidgetPanel");
      expect(props.instance).toBe(CLOCK);
      expect(props.now).toBeInstanceOf(Date);
      expect(props.returnedFromSettings).toBe(false);

      await call("WidgetPanel", "onAdjust");
      expect(onView).toHaveBeenCalledExactlyOnceWith(SUB_VIEW);
    });

    it("tells the main view it was reached by Voltar", () => {
      renderContent(widgetPanel(CLOCK.id), {
        instance: CLOCK,
        view: BACK_VIEW,
      });

      expect(
        propsOf<WidgetPanelProps<"clock">>("WidgetPanel").returnedFromSettings,
      ).toBe(true);
    });

    it("saves a change of its main view through update_widget, tagged with its kind", async () => {
      renderContent(widgetPanel(CLOCK.id), { instance: CLOCK });

      const { onUpdate } = propsOf<WidgetPanelProps<"clock">>("WidgetPanel");
      await act(() => onUpdate?.({ label: "Lisboa", config: CLOCK.config }));
      expect(invoke).toHaveBeenCalledExactlyOnceWith("update_widget", {
        id: CLOCK.id,
        label: "Lisboa",
        config: { kind: "clock", config: CLOCK.config },
      });
    });

    it("shows its Ajustes in the subview, Voltar going back", async () => {
      const { onView } = renderContent(widgetPanel(CLOCK.id), {
        instance: CLOCK,
        view: SUB_VIEW,
      });

      expect(shown()).toBe("WidgetSettings");
      expect(
        propsOf<WidgetSettingsProps<"clock">>("WidgetSettings").instance,
      ).toBe(CLOCK);
      await call("WidgetSettings", "onBack");
      expect(onView).toHaveBeenCalledExactlyOnceWith(BACK_VIEW);
    });

    it("saves from Ajustes through update_widget", async () => {
      renderContent(widgetPanel(CLOCK.id), {
        instance: CLOCK,
        view: SUB_VIEW,
      });

      const { onUpdate } =
        propsOf<WidgetSettingsProps<"clock">>("WidgetSettings");
      await act(() => onUpdate({ label: null, config: CLOCK.config }));
      expect(invoke).toHaveBeenCalledExactlyOnceWith("update_widget", {
        id: CLOCK.id,
        label: null,
        config: { kind: "clock", config: CLOCK.config },
      });
    });

    it("removes from Ajustes, then has the host close the panel", async () => {
      const { onRemoved } = renderContent(widgetPanel(CLOCK.id), {
        instance: CLOCK,
        view: SUB_VIEW,
      });

      await call("WidgetSettings", "onRemove");
      expect(invoke).toHaveBeenCalledExactlyOnceWith("remove_widget", {
        id: CLOCK.id,
      });
      expect(onRemoved).toHaveBeenCalledTimes(1);
    });

    it("keeps the panel open when the removal is refused", async () => {
      invoke.mockRejectedValue({ code: "notFound" });
      const { onRemoved } = renderContent(widgetPanel(CLOCK.id), {
        instance: CLOCK,
        view: SUB_VIEW,
      });

      const { onRemove } =
        propsOf<WidgetSettingsProps<"clock">>("WidgetSettings");
      await act(async () => {
        await expect(onRemove()).rejects.toEqual({ code: "notFound" });
      });
      expect(onRemoved).not.toHaveBeenCalled();
    });

    it("shows nothing once the instance left the list", () => {
      const { container } = renderContent(widgetPanel(CLOCK.id), {
        instance: null,
        view: SUB_VIEW,
      });

      expect(container).toBeEmptyDOMElement();
    });
  });

  describe("Configurações", () => {
    it("shows the main view with what it lists, focusing no row", () => {
      renderContent("settings");

      expect(shown()).toBe("SettingsPanel");
      expect(propsOf("SettingsPanel")).toMatchObject({
        settings: SETTINGS,
        widgets: WIDGETS,
        apps: APPS,
        focusItemsRow: false,
        focusLookRow: false,
        focusAppsRow: false,
      });
    });

    it.each([
      ["onOpenItems", SUB_VIEW],
      ["onOpenApps", APPS_VIEW],
      ["onOpenLook", LOOK_VIEW],
    ] as const)("opens a subview with %s", async (key, view) => {
      const { onView } = renderContent("settings");

      await call("SettingsPanel", key);
      expect(onView).toHaveBeenCalledExactlyOnceWith(view);
    });

    it.each([
      ["Itens do dock", BACK_VIEW, "focusItemsRow"],
      ["Aparência do dock", BACK_FROM_LOOK_VIEW, "focusLookRow"],
      ["Apps do dock", BACK_FROM_APPS_VIEW, "focusAppsRow"],
    ] as const)(
      "focuses only the row of %s on coming back from it",
      (_name, view, row) => {
        renderContent("settings", { view });

        const props = propsOf<Record<string, unknown>>("SettingsPanel");
        for (const key of ["focusItemsRow", "focusLookRow", "focusAppsRow"]) {
          expect(props[key]).toBe(key === row);
        }
      },
    );

    it.each([
      ["Itens do dock", SUB_VIEW, "DockItems", "snapshot", WIDGETS, BACK_VIEW],
      [
        "Apps do dock",
        APPS_VIEW,
        "AppsPanel",
        "snapshot",
        APPS,
        BACK_FROM_APPS_VIEW,
      ],
      [
        "Aparência do dock",
        LOOK_VIEW,
        "LookPanel",
        "settings",
        SETTINGS,
        BACK_FROM_LOOK_VIEW,
      ],
    ] as const)(
      "shows %s, Voltar going back to its row",
      async (_name, view, name, key, value, back) => {
        const { onView } = renderContent("settings", { view });

        expect(shown()).toBe(name);
        expect(propsOf<Record<string, unknown>>(name)[key]).toBe(value);
        await call(name, "onBack");
        expect(onView).toHaveBeenCalledExactlyOnceWith(back);
      },
    );

    it("passes null while the data can't be read", () => {
      renderContent("settings", { settings: null, widgets: null, apps: null });

      expect(propsOf("SettingsPanel")).toMatchObject({
        settings: null,
        widgets: null,
        apps: null,
      });
    });
  });

  it("shows Sobre", () => {
    renderContent("about");

    expect(shown()).toBe("AboutPanel");
  });

  describe("the tray", () => {
    it("shows the icons the dock keeps", () => {
      renderContent("tray");

      expect(shown()).toBe("TrayPanel");
      expect(propsOf("TrayPanel")).toEqual({ inDock: ["volume", "wifi"] });
    });

    it("keeps none while the settings can't be read", () => {
      renderContent("tray", { settings: null });

      expect(propsOf("TrayPanel")).toEqual({ inDock: [] });
    });
  });

  it("ignores the subview of a panel that has none", () => {
    renderContent("about", { view: SUB_VIEW });
    expect(shown()).toBe("AboutPanel");
  });

  it("does not take one panel's data for another's", () => {
    // A system panel with an app and an instance around still shows itself.
    renderContent("about", { app: APP, instance: CLOCK, other: OTHER });
    expect(shown()).toBe("AboutPanel");
    expect(got.size).toBe(1);
  });
});

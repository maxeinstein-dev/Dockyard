import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ScreenshotsList } from "../../core/screenshots";
import type { WidgetInstance } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import ScreenshotsPanel from "./ScreenshotsPanel";

const text = ptBR.widgets.screenshots;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const INSTANCE: WidgetInstance<"screenshots"> = {
  id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f60a6",
  label: null,
  kind: "screenshots",
  config: {},
};

const TWO: ScreenshotsList = {
  folder: true,
  items: [
    {
      key: 0,
      name: "Captura de tela 2026-11-04 151200.png",
      modified: 2,
      picture: "data:image/png;base64,AA==",
    },
    {
      key: 1,
      name: "Captura de tela 2026-11-03 090000.png",
      modified: 1,
      picture: null,
    },
  ],
};

async function panel(list: ScreenshotsList | Error) {
  invoke.mockImplementation((command: string) => {
    if (command === "list_screenshots") {
      return list instanceof Error
        ? Promise.reject(list)
        : Promise.resolve(list);
    }
    return Promise.resolve();
  });
  render(
    <ScreenshotsPanel
      instance={INSTANCE}
      now={new Date(2026, 10, 4)}
      onAdjust={() => undefined}
      returnedFromSettings={false}
    />,
  );
  await act(async () => {});
}

const sent = (name: string) =>
  invoke.mock.calls
    .filter(([command]) => command === name)
    .map(([, args]) => args as unknown);

const shot = (name: string) =>
  screen.getByRole("button", { name: `Abrir ${name}` });

beforeEach(() => {
  invoke.mockReset();
});

describe("ScreenshotsPanel", () => {
  it("shows the newest first, each named by its file", async () => {
    await panel(TWO);
    const items = within(
      screen.getByRole("list", { name: text.listLabel }),
    ).getAllByRole("button");
    expect(items.map((item) => item.getAttribute("aria-label"))).toEqual([
      "Abrir Captura de tela 2026-11-04 151200.png",
      "Abrir Captura de tela 2026-11-03 090000.png",
    ]);
    expect(items[0]?.querySelector("img")).not.toBeNull();
    expect(items[1]?.querySelector("img")).toBeNull();
  });

  it("opens one on a click, by its key", async () => {
    await panel(TWO);
    fireEvent.click(shot("Captura de tela 2026-11-03 090000.png"));
    expect(sent("open_screenshot")).toEqual([{ key: 1 }]);
  });

  it("drags one once the pointer moves with the button held, and opens nothing", async () => {
    await panel(TWO);
    const first = shot("Captura de tela 2026-11-04 151200.png");
    fireEvent.pointerDown(first, {
      button: 0,
      buttons: 1,
      clientX: 10,
      clientY: 10,
    });
    fireEvent.pointerMove(first, { buttons: 1, clientX: 12, clientY: 11 });
    expect(sent("drag_screenshot")).toEqual([]);
    fireEvent.pointerMove(first, { buttons: 1, clientX: 20, clientY: 10 });
    fireEvent.pointerMove(first, { buttons: 1, clientX: 30, clientY: 10 });
    expect(sent("drag_screenshot")).toEqual([{ key: 0 }]);
    fireEvent.click(first);
    expect(sent("open_screenshot")).toEqual([]);
  });

  it("opens the folder", async () => {
    await panel(TWO);
    fireEvent.click(screen.getByRole("button", { name: text.openFolder }));
    expect(sent("open_screenshots_folder")).toEqual([undefined]);
  });

  it("says when there is none yet, or no folder, or it failed", async () => {
    await panel({ folder: true, items: [] });
    expect(screen.getByRole("status")).toHaveTextContent(text.empty);
  });

  it("says when Windows has no folder yet, without the folder button", async () => {
    await panel({ folder: false, items: [] });
    expect(screen.getByRole("status")).toHaveTextContent(text.noFolder);
    expect(screen.queryByRole("button", { name: text.openFolder })).toBeNull();
  });

  it("says when the folder could not be read", async () => {
    await panel(new Error("x"));
    expect(screen.getByRole("status")).toHaveTextContent(text.failed);
  });
});

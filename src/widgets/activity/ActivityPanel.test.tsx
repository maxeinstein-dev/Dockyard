import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  READ_SYSTEM_ACTIVITY_COMMAND,
  SYSTEM_ACTIVITY_EVERY_MS,
  type SystemReading,
} from "../../core/systemActivity";
import type { WidgetInstance } from "../../core/widgets";
import { WIDGET_FIXTURES } from "../../core/widgets.fixtures";
import ActivityPanel from "./ActivityPanel";

const { invoke, warn } = vi.hoisted(() => ({
  invoke: vi.fn(),
  warn: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../core/log")>()),
  log: { error: vi.fn(), warn, info: vi.fn() },
}));

const GB = 1024 ** 3;

const READING: SystemReading = {
  cpu: 42.4,
  // 6 GB used of 8 GB: 75%.
  memory: { total: 8 * GB, available: 2 * GB },
  down: 0,
  up: 0,
  cpuHistory: [10, 30, 42.4],
  downHistory: [],
  upHistory: [],
};

const ACTIVITY = WIDGET_FIXTURES["activity-default"];

function renderPanel(
  instance: WidgetInstance<"activity"> = ACTIVITY,
  returnedFromSettings = false,
) {
  const onAdjust = vi.fn();
  const view = render(
    <ActivityPanel
      instance={instance}
      now={new Date(0)}
      onAdjust={onAdjust}
      returnedFromSettings={returnedFromSettings}
    />,
  );
  return { ...view, onAdjust };
}

/** Lets the pending reading settle. */
const settle = () => act(async () => {});

/** Moves to the next reading, 2 s later. */
const nextReading = () =>
  act(async () => {
    await vi.advanceTimersByTimeAsync(SYSTEM_ACTIVITY_EVERY_MS);
  });

const rows = () => screen.getAllByRole("listitem");
const fill = (row: HTMLElement) =>
  row.querySelector<HTMLElement>(".sys-fill")?.style.getPropertyValue("--f");

describe("ActivityPanel", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    invoke.mockReset();
    warn.mockClear();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("is titled with the kind's name, or the label", async () => {
    invoke.mockResolvedValue(READING);
    renderPanel();
    await settle();
    expect(
      screen.getByRole("heading", { name: "Atividade" }),
    ).toBeInTheDocument();

    renderPanel({ ...ACTIVITY, label: "Notebook" });
    await settle();
    expect(
      screen.getByRole("heading", { name: "Notebook" }),
    ).toBeInTheDocument();
  });

  it("opens Ajustes, which takes the focus only on coming back", async () => {
    invoke.mockResolvedValue(READING);
    const { onAdjust, unmount } = renderPanel();
    await settle();
    const adjust = screen.getByRole("button", { name: "Ajustes" });
    expect(adjust).not.toHaveFocus();
    fireEvent.click(adjust);
    expect(onAdjust).toHaveBeenCalledTimes(1);
    unmount();

    renderPanel(ACTIVITY, true);
    await settle();
    expect(screen.getByRole("button", { name: "Ajustes" })).toHaveFocus();
  });

  it("says it is reading until the first answer", async () => {
    invoke.mockReturnValue(new Promise(() => {}));
    renderPanel();
    await settle();

    const [cpu, memory] = rows();
    expect(cpu).toHaveTextContent("ProcessadorLendo…");
    expect(memory).toHaveTextContent("MemóriaLendo…");
    // No size yet, and empty bars and graph.
    expect(memory?.querySelector(".sys-detail")).toBeNull();
    expect(fill(cpu as HTMLElement)).toBe("0.0000");
    expect(cpu?.querySelector("polyline")).toBeNull();
  });

  it("shows the processor and the memory of the reading", async () => {
    invoke.mockResolvedValue(READING);
    renderPanel();
    await settle();

    expect(invoke).toHaveBeenCalledWith(READ_SYSTEM_ACTIVITY_COMMAND);
    const [cpu, memory] = rows() as [HTMLElement, HTMLElement];
    expect(cpu.querySelector(".sys-figure")).toHaveTextContent("42%");
    expect(fill(cpu)).toBe("0.4240");
    expect(cpu.querySelector("polyline")).not.toBeNull();

    expect(memory.querySelector(".sys-figure")).toHaveTextContent("75%");
    expect(memory.querySelector(".sys-detail")).toHaveTextContent(
      "6,0 GB de 8,0 GB",
    );
    expect(fill(memory)).toBe("0.7500");
    expect(screen.getByText("Atualiza a cada 2 segundos")).toBeInTheDocument();
  });

  it("keeps the processor as Lendo… until it has two readings", async () => {
    invoke.mockResolvedValue({ ...READING, cpu: null, cpuHistory: [] });
    renderPanel();
    await settle();

    const [cpu, memory] = rows();
    expect(cpu).toHaveTextContent("Lendo…");
    expect(memory).toHaveTextContent("75%");
  });

  it("reads again every 2 seconds while open, and stops when closed", async () => {
    invoke.mockResolvedValueOnce(READING);
    const { unmount } = renderPanel();
    await settle();
    expect(invoke).toHaveBeenCalledTimes(1);

    invoke.mockResolvedValueOnce({ ...READING, cpu: 99.6 });
    await nextReading();
    expect(invoke).toHaveBeenCalledTimes(2);
    expect(rows()[0]).toHaveTextContent("100%");

    unmount();
    await nextReading();
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("goes back to Lendo… when a reading fails, logging only once", async () => {
    invoke.mockResolvedValueOnce(READING);
    renderPanel();
    await settle();
    expect(rows()[0]).toHaveTextContent("42%");

    invoke.mockRejectedValue(new Error("sampler"));
    await nextReading();
    expect(rows()[0]).toHaveTextContent("Lendo…");
    await nextReading();
    expect(warn).toHaveBeenCalledExactlyOnceWith("system: read failed: Error");
  });

  it("hides its bars and graph from assistive technology", async () => {
    invoke.mockResolvedValue(READING);
    renderPanel();
    await settle();

    for (const row of rows()) {
      expect(row.querySelector(".sys-bar")).toHaveAttribute(
        "aria-hidden",
        "true",
      );
    }
    expect(rows()[0]?.querySelector("svg")).toHaveAttribute(
      "aria-hidden",
      "true",
    );
  });

  it("shows no memory share with no total", async () => {
    invoke.mockResolvedValue({
      ...READING,
      memory: { total: 0, available: 0 },
    });
    renderPanel();
    await settle();

    const memory = rows()[1];
    expect(memory?.querySelector(".sys-figure")).toHaveTextContent("Lendo…");
    expect(fill(memory as HTMLElement)).toBe("0.0000");
  });
});

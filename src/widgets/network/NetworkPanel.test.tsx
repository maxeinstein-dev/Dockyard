import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  SYSTEM_ACTIVITY_EVERY_MS,
  type SystemReading,
} from "../../core/systemActivity";
import type { WidgetInstance } from "../../core/widgets";
import { WIDGET_FIXTURES } from "../../core/widgets.fixtures";
import NetworkPanel from "./NetworkPanel";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const KB = 1024;

const READING: SystemReading = {
  cpu: 10,
  memory: null,
  down: 340 * KB,
  up: 1.2 * KB * KB,
  cpuHistory: [],
  downHistory: [0, 128 * KB],
  upHistory: [0, 64 * KB],
};

const NETWORK = WIDGET_FIXTURES["network-default"];

function renderPanel(
  instance: WidgetInstance<"network"> = NETWORK,
  returnedFromSettings = false,
) {
  const onAdjust = vi.fn();
  const view = render(
    <NetworkPanel
      instance={instance}
      now={new Date(0)}
      onAdjust={onAdjust}
      returnedFromSettings={returnedFromSettings}
    />,
  );
  return { ...view, onAdjust };
}

const settle = () => act(async () => {});

const rows = () => screen.getAllByRole("listitem") as HTMLElement[];
/** The y of the newest point of a row's line, or null with no line. */
const lastY = (row: HTMLElement | undefined) => {
  const points = row?.querySelector("polyline")?.getAttribute("points");
  if (!points) return null;
  return Number(points.split(" ").pop()?.split(",")[1]);
};

describe("NetworkPanel", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    invoke.mockReset();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("is titled with the kind's name, or the label, and opens Ajustes", async () => {
    invoke.mockResolvedValue(READING);
    const { onAdjust, unmount } = renderPanel();
    await settle();
    expect(
      screen.getByRole("heading", { name: "Atividade de rede" }),
    ).toBeInTheDocument();
    const adjust = screen.getByRole("button", { name: "Ajustes" });
    expect(adjust).not.toHaveFocus();
    fireEvent.click(adjust);
    expect(onAdjust).toHaveBeenCalledTimes(1);
    unmount();

    renderPanel({ ...NETWORK, label: "Wi-Fi" }, true);
    await settle();
    expect(screen.getByRole("heading", { name: "Wi-Fi" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ajustes" })).toHaveFocus();
  });

  it("says it is reading until the first answer", async () => {
    invoke.mockReturnValue(new Promise(() => {}));
    renderPanel();
    await settle();

    const [down, up] = rows();
    expect(down).toHaveTextContent("DownloadLendo…");
    expect(up).toHaveTextContent("UploadLendo…");
    expect(lastY(down)).toBeNull();
  });

  it("shows the rates down and up, the dock's way", async () => {
    invoke.mockResolvedValue(READING);
    renderPanel();
    await settle();

    const [down, up] = rows();
    expect(down?.querySelector(".sys-figure")).toHaveTextContent("340 KB/s");
    expect(up?.querySelector(".sys-figure")).toHaveTextContent("1,2 MB/s");
    expect(
      screen.getByText("Placas de rede ligadas · a cada 2 segundos"),
    ).toBeInTheDocument();
  });

  it("shows a zero rate as 0 KB/s, not as reading", async () => {
    invoke.mockResolvedValue({ ...READING, down: 0, up: 0 });
    renderPanel();
    await settle();

    expect(rows()[0]).toHaveTextContent("0 KB/s");
    expect(rows()[1]).toHaveTextContent("0 KB/s");
  });

  it("draws both lines on one scale, up to the busier one", async () => {
    invoke.mockResolvedValue(READING);
    renderPanel();
    await settle();

    const [down, up] = rows();
    // The peak of both is the download's 128 KB: at the top (y 2 of 28).
    expect(lastY(down)).toBe(2);
    // Half of it, half the height.
    expect(lastY(up)).toBe(15);
  });

  it("draws a quiet network near the floor, not stretched to the top", async () => {
    invoke.mockResolvedValue({
      ...READING,
      downHistory: [0, 1 * KB],
      upHistory: [0, 0],
    });
    renderPanel();
    await settle();

    // 1 KB against the 64 KB floor: 28 - 26/64.
    expect(lastY(rows()[0])).toBe(27.59);
    expect(lastY(rows()[1])).toBe(28);
  });

  it("follows each reading, 2 seconds apart, until closed", async () => {
    invoke.mockResolvedValueOnce(READING);
    const { unmount } = renderPanel();
    await settle();

    invoke.mockResolvedValueOnce({ ...READING, down: 5 * KB * KB });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SYSTEM_ACTIVITY_EVERY_MS);
    });
    expect(rows()[0]).toHaveTextContent("5,0 MB/s");
    expect(invoke).toHaveBeenCalledTimes(2);

    unmount();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SYSTEM_ACTIVITY_EVERY_MS * 3);
    });
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("hides its graphs from assistive technology", async () => {
    invoke.mockResolvedValue(READING);
    renderPanel();
    await settle();

    for (const row of rows()) {
      expect(row.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
    }
  });
});

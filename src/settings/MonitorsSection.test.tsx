import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { WorkspacesSnapshot } from "../core/workspaces";
import { format } from "../i18n";
import { ptBR } from "../i18n/pt-BR";
import MonitorsSection from "./MonitorsSection";

const text = ptBR.workspaces.monitors;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const SNAPSHOT: WorkspacesSnapshot = {
  workspaces: [
    { id: "w1", name: "Trabalho", color: "#0A84FF", items: [] },
    { id: "w2", name: "Pessoal", color: "#FF9F0A", items: [] },
  ],
  inUse: "w1",
} as unknown as WorkspacesSnapshot;

const MONITORS = [
  {
    key: "path-a",
    name: "DELL U2419H",
    primary: true,
    workspace: "w1",
    fixed: false,
  },
  {
    key: "path-b",
    name: "LG 24",
    primary: false,
    workspace: "w2",
    fixed: true,
  },
];

const flush = () => act(() => Promise.resolve());
const sent = (name: string) =>
  invoke.mock.calls
    .filter(([command]) => command === name)
    .map(([, args]) => args as unknown);

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation((command: string) =>
    Promise.resolve(command === "list_monitors" ? MONITORS : undefined),
  );
});

describe("MonitorsSection (spec 0028 E)", () => {
  it("lists each monitor with its workspace and its hold", async () => {
    render(<MonitorsSection snapshot={SNAPSHOT} />);
    await flush();

    const main = format(text.primary, { name: "DELL U2419H" });
    expect(
      screen.getByLabelText(format(text.workspace, { name: main })),
    ).toHaveValue("w1");
    expect(
      screen.getByRole("switch", {
        name: format(text.fixed, { name: "LG 24" }),
      }),
    ).toHaveAttribute("aria-checked", "true");
  });

  it("moves a monitor to a workspace and holds or releases it", async () => {
    render(<MonitorsSection snapshot={SNAPSHOT} />);
    await flush();

    fireEvent.change(
      screen.getByLabelText(format(text.workspace, { name: "LG 24" })),
      {
        target: { value: "w1" },
      },
    );
    fireEvent.click(
      screen.getByRole("switch", {
        name: format(text.fixed, { name: "LG 24" }),
      }),
    );
    await flush();

    expect(sent("set_monitor_workspace")).toEqual([
      { monitor: "path-b", workspace: "w1" },
    ]);
    expect(sent("set_monitor_fixed")).toEqual([
      { monitor: "path-b", fixed: false },
    ]);
  });
});

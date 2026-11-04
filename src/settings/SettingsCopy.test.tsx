import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ptBR } from "../i18n/pt-BR";
import SettingsCopy from "./SettingsCopy";

const text = ptBR.settings.copy;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

/** Answers each command from `answers`; an `Error` rejects with its text,
 * as Rust's error codes come. */
function answer(answers: Record<string, unknown>) {
  invoke.mockImplementation((command: string) => {
    const value = answers[command];
    if (value instanceof Error) return Promise.reject(value.message);
    return Promise.resolve(value);
  });
}

const flush = () => act(() => Promise.resolve());
const button = (name: string) => screen.getByRole("button", { name });
const sent = (name: string) =>
  invoke.mock.calls.filter(([command]) => command === name).length;

const SUMMARY = {
  app: "0.9.2",
  exportedAt: "2026-11-04T18:00:00Z",
  workspaces: 2,
  apps: 18,
  widgets: 10,
  icons: 1,
};

beforeEach(() => {
  invoke.mockReset();
});

describe("SettingsCopy (spec 0026)", () => {
  it("exports, and says so", async () => {
    answer({ export_copy: true });
    render(<SettingsCopy />);

    fireEvent.click(button(text.export));
    await flush();

    expect(sent("export_copy")).toBe(1);
    expect(screen.getByRole("status")).toHaveTextContent(text.exported);
  });

  it("says nothing when the dialog was cancelled", async () => {
    answer({ export_copy: false, choose_import: null });
    render(<SettingsCopy />);

    fireEvent.click(button(text.export));
    await flush();
    fireEvent.click(button(text.import));
    await flush();

    expect(screen.getByRole("status")).toBeEmptyDOMElement();
    expect(button(text.import)).toBeEnabled();
  });

  it("shows what a copy holds before importing it", async () => {
    answer({ choose_import: SUMMARY, confirm_import: undefined });
    render(<SettingsCopy />);

    fireEvent.click(button(text.import));
    await flush();

    expect(
      screen.getByText(/Espaços: 2 · Apps: 18 · Widgets: 10/),
    ).toBeInTheDocument();
    expect(screen.getByText(text.replaces)).toBeInTheDocument();
    expect(sent("confirm_import")).toBe(0);
    fireEvent.click(button(text.confirm));
    await flush();

    expect(sent("confirm_import")).toBe(1);
    expect(screen.getByRole("status")).toHaveTextContent(text.restarting);
  });

  it("cancels, back to the two buttons", async () => {
    answer({ choose_import: SUMMARY, cancel_import: undefined });
    render(<SettingsCopy />);

    fireEvent.click(button(text.import));
    await flush();
    fireEvent.click(button(text.cancel));
    await flush();

    expect(sent("cancel_import")).toBe(1);
    expect(button(text.export)).toBeInTheDocument();
  });

  it("says why a file was refused", async () => {
    answer({ choose_import: new Error("newer") });
    render(<SettingsCopy />);

    fireEvent.click(button(text.import));
    await flush();

    expect(screen.getByRole("status")).toHaveTextContent(text.errors.newer);
  });
});

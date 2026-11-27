import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CycleConfig } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import tokens from "../../themes/tokens.css?raw";
import CycleFields from "./CycleFields";
import css from "./cycle.css?raw";

const text = ptBR.widgets.cycle;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => undefined),
}));

const CONFIG: CycleConfig = {
  symbol: "🌸",
  calendar: null,
  sensitive: 5,
  notices: { sensitive: true, eve: true, day: true, windows: true },
  reminders: [{ text: "Paciência", anchor: "period", days: 1 }],
};

type OnChange = (config: CycleConfig) => void;

/** The Ciclo's fields, as the Calendário's settings show them (spec 0031):
 * each change gives the whole cycle back, and the settings show it. */
function show(config = CONFIG) {
  const onChange = vi.fn<OnChange>();
  function Settings() {
    const [shown, setShown] = useState(config);
    return (
      <CycleFields
        widget="c"
        config={shown}
        invalid={false}
        onChange={(next) => {
          setShown(next);
          onChange(next);
        }}
      />
    );
  }
  render(<Settings />);
  return { onChange };
}

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(() => Promise.resolve(null));
});

describe("CycleFields", () => {
  it("turns the Windows notice on", () => {
    const { onChange } = show({
      ...CONFIG,
      notices: { ...CONFIG.notices, windows: false },
    });
    // The test shows the notice on the dock, Windows or not.
    fireEvent.click(screen.getByRole("button", { name: text.testNotice }));
    expect(invoke).toHaveBeenCalledWith("cycle_test_notice", {
      widget: "c",
      windows: false,
    });
    fireEvent.click(
      screen.getByRole("switch", { name: text.noticeToggles.windows }),
    );
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        notices: { sensitive: true, eve: true, day: true, windows: true },
      }) as CycleConfig,
    );
  });

  it("says when Windows has the notices off", async () => {
    invoke.mockImplementation(() => Promise.reject("notificationsOff"));
    show();
    fireEvent.click(screen.getByRole("button", { name: text.testNotice }));
    expect(await screen.findByRole("status")).toHaveTextContent(
      text.testResult.off,
    );
  });

  it("turns a Windows notice off", () => {
    const { onChange } = show();
    fireEvent.click(
      screen.getByRole("switch", { name: text.noticeToggles.eve }),
    );
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        notices: { sensitive: true, eve: false, day: true, windows: true },
      }) as CycleConfig,
    );
  });

  it("lists, adds and removes reminders", () => {
    const { onChange } = show();
    expect(screen.getByText("Paciência")).toBeInTheDocument();
    expect(screen.getByText("1 dia depois da menstruação")).toBeInTheDocument();

    const add = screen.getByRole("button", { name: text.addReminder });
    expect(add).toBeDisabled();
    fireEvent.change(screen.getByLabelText(text.reminderText), {
      target: { value: " Comprar chocolate " },
    });
    fireEvent.change(screen.getByLabelText(text.reminderDaysLabel), {
      target: { value: "-2" },
    });
    fireEvent.click(add);
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        reminders: [
          CONFIG.reminders[0],
          { text: "Comprar chocolate", anchor: "sensitive", days: -2 },
        ],
      }) as CycleConfig,
    );

    fireEvent.click(
      screen.getByRole("button", { name: "Remover o lembrete Paciência" }),
    );
    // The one just added stays.
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        reminders: [
          { text: "Comprar chocolate", anchor: "sensitive", days: -2 },
        ],
      }) as CycleConfig,
    );
  });

  it("refuses a reminder longer than Rust takes", () => {
    show();
    fireEvent.change(screen.getByLabelText(text.reminderText), {
      target: { value: "x".repeat(121) },
    });
    expect(
      screen.getByRole("button", { name: text.addReminder }),
    ).toBeDisabled();
  });

  it("sends a test notice and says so", async () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: text.testNotice }));
    expect(invoke).toHaveBeenCalledWith("cycle_test_notice", {
      widget: "c",
      windows: true,
    });
    expect(await screen.findByRole("status")).toHaveTextContent(
      text.testResult.sent,
    );
  });
});

describe("cycle.css", () => {
  it("uses only tokens that tokens.css defines", () => {
    const defined = new Set(
      [...tokens.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    );
    const used = [...css.matchAll(/var\((--[\w-]+)/g)].map((match) => match[1]);
    expect(used.length).toBeGreaterThan(0);
    expect(used.filter((token) => !defined.has(token))).toEqual([]);
  });
});

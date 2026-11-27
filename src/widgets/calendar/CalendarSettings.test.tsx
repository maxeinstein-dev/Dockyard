import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CalendarConfig, WidgetInstance } from "../../core/widgets";
import { DEFAULT_CYCLE } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import CalendarSettings from "./CalendarSettings";
import css from "./calendar.css?raw";

const text = ptBR.widgets.calendar;
const cycleText = ptBR.widgets.cycle;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => undefined),
}));

const CONFIG: CalendarConfig = {
  width: 2,
  hidden: [],
  tasks: true,
  taskOrder: [],
  cardPeriod: "auto",
  cardTasks: true,
  cycle: null,
};

const calendar: WidgetInstance<"calendar"> = {
  id: "k",
  label: null,
  kind: "calendar",
  config: CONFIG,
};

type OnUpdate = (update: WidgetUpdate<"calendar">) => Promise<void>;

function show(config: CalendarConfig = CONFIG) {
  const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
  render(
    <CalendarSettings
      instance={{ ...calendar, config }}
      onUpdate={onUpdate}
      onRemove={() => Promise.resolve()}
      onBack={() => undefined}
    />,
  );
  return onUpdate;
}

function lastConfig(onUpdate: ReturnType<typeof show>): CalendarConfig {
  const last = onUpdate.mock.lastCall;
  if (!last) throw new Error("nothing was saved");
  return last[0].config;
}

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(() => Promise.resolve(null));
});

describe("CalendarSettings › Ciclo (spec 0031)", () => {
  it("starts off, with none of the Ciclo's settings", () => {
    show();
    const toggle = screen.getByRole("switch", { name: text.cycle.show });
    expect(toggle).toHaveAttribute("aria-checked", "false");
    expect(screen.queryByLabelText(cycleText.symbol)).toBeNull();
  });

  it("turned on, takes the Ciclo's defaults and keeps the rest", () => {
    const onUpdate = show({ ...CONFIG, width: 1 });
    fireEvent.click(screen.getByRole("switch", { name: text.cycle.show }));
    expect(lastConfig(onUpdate)).toEqual({
      ...CONFIG,
      width: 1,
      cycle: DEFAULT_CYCLE,
    });
  });

  it("on, shows the Ciclo's settings and saves them into the cycle", () => {
    const onUpdate = show({ ...CONFIG, cycle: DEFAULT_CYCLE });
    expect(screen.getByLabelText(cycleText.symbol)).toHaveValue("🌸");
    fireEvent.click(screen.getByRole("button", { name: "7" }));
    expect(lastConfig(onUpdate).cycle).toEqual({
      ...DEFAULT_CYCLE,
      sensitive: 7,
    });
    fireEvent.click(
      screen.getByRole("switch", { name: cycleText.noticeToggles.eve }),
    );
    expect(lastConfig(onUpdate).cycle?.notices.eve).toBe(
      !DEFAULT_CYCLE.notices.eve,
    );
  });

  it("turned off, keeps nothing of the cycle", () => {
    const onUpdate = show({
      ...CONFIG,
      cycle: { ...DEFAULT_CYCLE, sensitive: 7 },
    });
    fireEvent.click(screen.getByRole("switch", { name: text.cycle.show }));
    expect(lastConfig(onUpdate).cycle).toBeNull();
  });
});

describe("CalendarSettings scroll", () => {
  it("scrolls the fields, keeping the header, Remover and the footer in view", () => {
    // With the Ciclo's settings the fields outgrow the screen.
    show({ ...CONFIG, cycle: DEFAULT_CYCLE });
    const scroll = document.querySelector(".cal-scroll");
    expect(scroll).not.toBeNull();
    const fields = within(scroll as HTMLElement);
    expect(fields.getByLabelText(cycleText.symbol)).toBeVisible();
    expect(fields.getByText(cycleText.remindersTitle)).toBeVisible();
    expect(
      fields.queryByRole("button", { name: ptBR.widgets.common.remove }),
    ).toBeNull();
    expect(fields.queryByRole("heading")).toBeNull();
  });

  it("scrolls above 400 px", () => {
    expect(css).toMatch(/\.cal-scroll\s*\{[^}]*max-height:\s*400px/);
    expect(css).toMatch(/\.cal-scroll\s*\{[^}]*overflow-y:\s*auto/);
  });
});

import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { WidgetInstance, WorldClockConfig } from "../../core/widgets";
import { ptBR } from "../../i18n/pt-BR";
import type { WidgetUpdate } from "../contract";
import WorldClockPanel from "./WorldClockPanel";
import WorldClockSettings from "./WorldClockSettings";
import { aheadText, readPlace } from "./format";

const text = ptBR.widgets.worldClock;

type OnUpdate = (update: WidgetUpdate<"world-clock">) => Promise<void>;

/* The tests run in whatever zone the machine has: they compare places with
 * each other, never with the user's own time. */

/** 15 Jan 2026, 23:30 UTC: already the 16th in Tokyo, still the 15th in
 * Honolulu. */
const LATE = Date.UTC(2026, 0, 15, 23, 30);

function worldClock(
  config: Partial<WorldClockConfig> = {},
): WidgetInstance<"world-clock"> {
  return {
    id: "019a1b2c-3d4e-7f60-8a1b-2c3d4e5f60a4",
    label: null,
    kind: "world-clock",
    config: {
      width: 2,
      zones: ["Asia/Tokyo", "Pacific/Honolulu"],
      hourCycle: "h23",
      ...config,
    },
  };
}

describe("readPlace", () => {
  it("gives the place's time in its zone", () => {
    expect(readPlace(LATE, "Asia/Tokyo", "h23", "pt-BR").time).toBe("08:30");
    expect(readPlace(LATE, "Pacific/Honolulu", "h23", "pt-BR").time).toBe(
      "13:30",
    );
    const twelve = readPlace(LATE, "Pacific/Honolulu", "h12", "en-US");
    expect(twelve.time).toBe("1:30");
    expect(twelve.dayPeriod).toBe("PM");
  });

  it("names the place as the Clock does", () => {
    expect(readPlace(LATE, "America/Sao_Paulo", "h23").city).toBe("Sao Paulo");
  });

  it("tells the day and the difference against each other", () => {
    const tokyo = readPlace(LATE, "Asia/Tokyo", "h23");
    const honolulu = readPlace(LATE, "Pacific/Honolulu", "h23");
    expect(tokyo.day - honolulu.day).toBe(1);
    expect(tokyo.ahead - honolulu.ahead).toBe(19 * 60);
  });

  it("writes the difference as people say it", () => {
    expect(aheadText(180)).toBe("+3 h");
    expect(aheadText(-150)).toBe("−2h30");
    expect(aheadText(330)).toBe("+5h30");
  });
});

describe("WorldClockPanel", () => {
  it("lists every place, in the user's order, with its time", () => {
    render(
      <WorldClockPanel
        instance={worldClock({
          zones: ["Pacific/Honolulu", "Asia/Tokyo", "Europe/London"],
        })}
        now={new Date(LATE)}
        onAdjust={() => undefined}
        returnedFromSettings={false}
      />,
    );
    const rows = within(
      screen.getByRole("list", { name: text.listLabel }),
    ).getAllByRole("listitem");
    expect(rows.map((row) => row.firstChild?.textContent)).toEqual([
      "Honolulu",
      "Tokyo",
      "London",
    ]);
    expect(rows[1]?.textContent).toContain("08:30");
    expect(rows[2]?.textContent).toContain("23:30");
  });
});

describe("WorldClockSettings", () => {
  function settings(instance = worldClock()) {
    const onUpdate = vi.fn<OnUpdate>(() => Promise.resolve());
    render(
      <WorldClockSettings
        instance={instance}
        onUpdate={onUpdate}
        onRemove={() => Promise.resolve()}
        onBack={() => undefined}
      />,
    );
    return onUpdate;
  }

  const zonesSent = (onUpdate: ReturnType<typeof settings>) =>
    onUpdate.mock.calls.map(([update]) => update.config.zones);

  it("moves and removes a place", () => {
    const onUpdate = settings();
    fireEvent.click(screen.getByRole("button", { name: "Descer Tokyo" }));
    fireEvent.click(screen.getByRole("button", { name: "Remover Honolulu" }));
    expect(zonesSent(onUpdate)).toEqual([
      ["Pacific/Honolulu", "Asia/Tokyo"],
      ["Asia/Tokyo"],
    ]);
  });

  it("keeps the last place and the ends where they are", () => {
    settings(worldClock({ zones: ["Asia/Tokyo"] }));
    for (const name of ["Subir Tokyo", "Descer Tokyo", "Remover Tokyo"]) {
      expect(
        (screen.getByRole("button", { name }) as HTMLButtonElement).disabled,
      ).toBe(true);
    }
  });

  it("adds a place from the zones not in it yet", () => {
    const onUpdate = settings();
    const list = screen.getByLabelText(text.add) as HTMLSelectElement;
    const values = Array.from(list.options).map((option) => option.value);
    expect(values).not.toContain("Asia/Tokyo");
    fireEvent.change(list, { target: { value: "Europe/London" } });
    expect(zonesSent(onUpdate)).toEqual([
      ["Asia/Tokyo", "Pacific/Honolulu", "Europe/London"],
    ]);
  });

  it("says when it is full instead of offering more", () => {
    settings(
      worldClock({
        zones: [
          "Asia/Tokyo",
          "Pacific/Honolulu",
          "Europe/London",
          "Europe/Paris",
          "America/New_York",
          "America/Sao_Paulo",
        ],
      }),
    );
    expect(screen.queryByLabelText(text.add)).toBeNull();
    expect(screen.getByText("Até 6 cidades.")).toBeInTheDocument();
  });

  it("saves the width and the hour's format", () => {
    const onUpdate = settings();
    fireEvent.click(screen.getByRole("button", { name: "3 ícones" }));
    fireEvent.click(
      screen.getByRole("button", { name: ptBR.widgets.clock.hourCycles.h12 }),
    );
    const configs = onUpdate.mock.calls.map(([update]) => update.config);
    expect(configs[0]?.width).toBe(3);
    expect(configs[1]?.hourCycle).toBe("h12");
  });
});

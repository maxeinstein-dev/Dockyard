import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ptBR } from "../i18n/pt-BR";
import type { GalleryDock } from "./StyleGallery";
import ThemeGallery from "./ThemeGallery";

/* M8: while "Manter este tema?" counts down, only the countdown changes:
 * the miniatures of the dock, the costly part of the gallery, are not drawn
 * again each second. */

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const { drawn } = vi.hoisted(() => ({ drawn: vi.fn() }));
vi.mock("./StyleGallery", async (original) => {
  const real = await original<typeof import("./StyleGallery")>();
  return {
    ...real,
    MiniDock: (props: Parameters<typeof real.MiniDock>[0]) => {
      drawn(props.of);
      return real.MiniDock(props);
    },
  };
});

const DOCK: GalleryDock = {
  style: "native",
  tweaks: {},
  indicator: "dot",
  indicatorColor: null,
  iconPack: null,
};
const APPS: readonly string[] = [];

describe("ThemeGallery's countdown", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("does not draw the miniatures again each second", () => {
    vi.useFakeTimers();
    invoke.mockResolvedValue(undefined);
    render(<ThemeGallery dock={DOCK} appIds={APPS} onApply={vi.fn()} />);
    const group = screen.getByRole("group", {
      name: ptBR.settings.look.themes,
    });
    fireEvent.click(within(group).getByRole("button", { name: "Neon" }));
    drawn.mockClear();

    // One second at a time: each tick renders before the next is set.
    for (let second = 0; second < 3; second++) {
      act(() => {
        vi.advanceTimersByTime(1000);
      });
    }

    expect(screen.getByText("Volta ao anterior em 12 s")).toBeInTheDocument();
    expect(drawn).not.toHaveBeenCalled();
  });
});

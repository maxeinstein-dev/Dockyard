import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { RecentFile } from "../../core/recentFiles";
import { WIDGET_FIXTURES } from "../../core/widgets.fixtures";
import { ptBR } from "../../i18n/pt-BR";
import FilesPanel from "./FilesPanel";

const text = ptBR.widgets.files;

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const FILES: RecentFile[] = [
  { key: 0, name: "relatorio.pdf", folder: "Documentos", kind: "document" },
  { key: 1, name: "foto.png", folder: "Imagens", kind: "image" },
];

async function panel(files: RecentFile[] | Error) {
  invoke.mockImplementation((command: string) => {
    if (command === "list_recent_files") {
      return files instanceof Error
        ? Promise.reject(files)
        : Promise.resolve(files);
    }
    return Promise.resolve();
  });
  render(
    <FilesPanel
      instance={WIDGET_FIXTURES["files-default"]}
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

beforeEach(() => {
  invoke.mockReset();
});

describe("FilesPanel", () => {
  it("lists the files opened last, each with its folder, never its path", async () => {
    await panel(FILES);
    const rows = within(
      screen.getByRole("list", { name: text.listLabel }),
    ).getAllByRole("button");
    expect(rows.map((row) => row.getAttribute("aria-label"))).toEqual([
      "Abrir relatorio.pdf, em Documentos",
      "Abrir foto.png, em Imagens",
    ]);
  });

  it("opens one on a click, by its key", async () => {
    await panel(FILES);
    fireEvent.click(
      screen.getByRole("button", { name: "Abrir foto.png, em Imagens" }),
    );
    expect(sent("open_recent_file")).toEqual([{ key: 1 }]);
  });

  it("drags one once the pointer moves with the button held, and opens nothing", async () => {
    await panel(FILES);
    const first = screen.getByRole("button", {
      name: "Abrir relatorio.pdf, em Documentos",
    });
    fireEvent.pointerDown(first, {
      button: 0,
      buttons: 1,
      clientX: 10,
      clientY: 10,
    });
    fireEvent.pointerMove(first, { buttons: 1, clientX: 10, clientY: 20 });
    fireEvent.click(first);
    expect(sent("drag_recent_file")).toEqual([{ key: 0 }]);
    expect(sent("open_recent_file")).toEqual([]);
  });

  it("says when there is none yet", async () => {
    await panel([]);
    expect(screen.getByRole("status")).toHaveTextContent(text.empty);
  });

  it("says when they could not be read", async () => {
    await panel(new Error("x"));
    expect(screen.getByRole("status")).toHaveTextContent(text.failed);
  });
});

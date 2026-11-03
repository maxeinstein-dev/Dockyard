import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type DockSettings } from "../core/settings";
import { forgetIcons } from "../core/apps";
import StyleGallery from "./StyleGallery";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));

const DOCK: DockSettings = {
  edge: "bottom",
  iconSize: 48,
  opacity: "translucent",
  ...DEFAULT_LOOK,
};

const gallery = () => screen.getByRole("radiogroup", { name: "Tema" });
const card = (name: string) => within(gallery()).getByRole("radio", { name });
const bar = (name: string) =>
  card(name).querySelector<HTMLElement>(".mini-bar") as HTMLElement;

describe("StyleGallery", () => {
  beforeEach(() => {
    forgetIcons();
    invoke.mockReset();
    invoke.mockImplementation((_command: string, args: { appId: string }) =>
      Promise.resolve(`data:image/png;base64,${args.appId}`),
    );
  });

  it("draws every style with the user's programs, the first five", async () => {
    render(
      <StyleGallery
        label="Tema"
        dock={{ ...DOCK, style: "neon" }}
        appIds={["a", "b", "c", "d", "e", "f"]}
        onSelect={vi.fn()}
      />,
    );
    await act(async () => {});

    expect(within(gallery()).getAllByRole("radio")).toHaveLength(17);
    expect(card("Neon")).toHaveAttribute("aria-checked", "true");
    expect(card("Papel")).toHaveAttribute("aria-checked", "false");
    const images = [...card("Papel").querySelectorAll("img")];
    // Decorative: the card's name says which style it is.
    expect(images.map((img) => img.getAttribute("alt"))).toEqual([
      "",
      "",
      "",
      "",
      "",
    ]);
    expect(images.map((img) => img.getAttribute("src"))).toEqual(
      ["a", "b", "c", "d", "e"].map((id) => `data:image/png;base64,${id}`),
    );
  });

  it("shows widget tiles when no program is pinned", () => {
    render(
      <StyleGallery label="Tema" dock={DOCK} appIds={[]} onSelect={vi.fn()} />,
    );

    expect(card("Rocket").querySelectorAll(".mini-tile")).toHaveLength(3);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("paints each style through custom properties, with the user's changes", () => {
    render(
      <StyleGallery
        label="Tema"
        dock={{
          ...DOCK,
          indicator: "dash",
          tweaks: { midnight: { corners: 100, reflections: true } },
        }}
        appIds={[]}
        onSelect={vi.fn()}
      />,
    );

    // Meia-noite: a pill (14 px is half the bar), with reflections.
    expect(bar("Meia-noite").style.getPropertyValue("--mini-radius")).toBe(
      "14px",
    );
    expect(bar("Meia-noite")).toHaveAttribute("data-reflections");
    expect(bar("Meia-noite").style.getPropertyValue("--mini-fill")).toBe(
      "rgb(8 10 16 / 0.93)",
    );
    // Rocket keeps its own 25 %, and its glow.
    expect(bar("Rocket").style.getPropertyValue("--mini-radius")).toBe("3.5px");
    expect(bar("Rocket").style.getPropertyValue("--mini-glow")).not.toBe("");
    // Nativo: the theme's own bar.
    expect(bar("Nativo").style.getPropertyValue("--mini-fill")).toBe("");
    expect(bar("Neon")).toHaveAttribute("data-outline");
    expect(card("Papel").querySelector(".mini-indicator")).toHaveAttribute(
      "data-shape",
      "dash",
    );
    expect(card("Nativo").getAttribute("style")).toBeNull();
  });

  it("colors the mark as the user chose, else as the style does", () => {
    const { rerender } = render(
      <StyleGallery label="Tema" dock={DOCK} appIds={[]} onSelect={vi.fn()} />,
    );
    expect(bar("Neon").style.getPropertyValue("--mini-indicator")).toBe(
      "rgb(0 229 255 / 1)",
    );
    expect(bar("Nativo").style.getPropertyValue("--mini-indicator")).toBe("");

    rerender(
      <StyleGallery
        label="Tema"
        dock={{ ...DOCK, indicatorColor: "#FF453A" }}
        appIds={[]}
        onSelect={vi.fn()}
      />,
    );
    expect(bar("Neon").style.getPropertyValue("--mini-indicator")).toBe(
      "#FF453A",
    );
  });

  it("draws a skin's bar with its image", () => {
    render(
      <StyleGallery label="Tema" dock={DOCK} appIds={[]} onSelect={vi.fn()} />,
    );

    const bar = card("Clássico 98").querySelector<HTMLElement>(".mini-bar");
    expect(bar).toHaveAttribute("data-skin");
    expect(bar?.style.getPropertyValue("--mini-skin")).toContain("skin.png");
    // 2 px ends of a 16 px image, on a 28 px bar.
    expect(bar?.style.getPropertyValue("--mini-skin-width")).toBe(
      "0 3.5px 0 3.5px",
    );
    const plain = card("Nativo").querySelector<HTMLElement>(".mini-bar");
    expect(plain).not.toHaveAttribute("data-skin");
  });

  it("chooses with a click, and moves with the arrows without choosing", () => {
    const onSelect = vi.fn();
    render(
      <StyleGallery label="Tema" dock={DOCK} appIds={[]} onSelect={onSelect} />,
    );

    expect(card("Nativo")).toHaveAttribute("tabindex", "0");
    expect(card("Neon")).toHaveAttribute("tabindex", "-1");
    card("Nativo").focus();
    fireEvent.keyDown(card("Nativo"), { key: "ArrowLeft" });
    expect(card("Espacial")).toHaveFocus();
    fireEvent.keyDown(card("Espacial"), { key: "Home" });
    expect(card("Nativo")).toHaveFocus();
    expect(onSelect).not.toHaveBeenCalled();

    fireEvent.click(card("Papel"));
    expect(onSelect).toHaveBeenCalledWith("paper");
    fireEvent.click(card("Nativo"));
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  const SKIN = {
    id: "Minha",
    name: "Minha barra",
    url: "data:image/png;base64,AAAA",
    width: 12,
    height: 8,
    left: 4,
    right: 2,
    repeat: true,
    pixelated: false,
    dark: true,
  };

  it("puts the skins of the Skins folder after the styles, with their image", () => {
    render(
      <StyleGallery
        label="Tema"
        dock={DOCK}
        appIds={[]}
        onSelect={vi.fn()}
        folderSkins={[SKIN]}
        onSelectFolderSkin={vi.fn()}
      />,
    );

    const cards = within(gallery()).getAllByRole("radio");
    expect(cards).toHaveLength(18);
    expect(cards[16]).toHaveTextContent("Espacial");
    expect(cards[17]).toHaveTextContent("Minha barra");
    const skin = bar("Minha barra");
    expect(skin).toHaveAttribute("data-skin");
    expect(skin.style.getPropertyValue("--mini-skin")).toBe(
      'url("data:image/png;base64,AAAA")',
    );
    // 4 and 2 px ends of an 8 px image, on a 28 px bar.
    expect(skin.style.getPropertyValue("--mini-skin-width")).toBe(
      "0 7px 0 14px",
    );
  });

  it("chooses a folder skin by its folder, and shows it chosen", () => {
    const onSelect = vi.fn();
    const onSelectFolderSkin = vi.fn();
    const { rerender } = render(
      <StyleGallery
        label="Tema"
        dock={DOCK}
        appIds={[]}
        onSelect={onSelect}
        folderSkins={[SKIN]}
        onSelectFolderSkin={onSelectFolderSkin}
      />,
    );

    fireEvent.click(card("Minha barra"));
    expect(onSelectFolderSkin).toHaveBeenCalledWith("Minha");
    expect(onSelect).not.toHaveBeenCalled();

    rerender(
      <StyleGallery
        label="Tema"
        dock={{ ...DOCK, style: "custom", customSkin: "Minha" }}
        appIds={[]}
        onSelect={onSelect}
        folderSkins={[SKIN]}
        onSelectFolderSkin={onSelectFolderSkin}
      />,
    );
    expect(card("Minha barra")).toHaveAttribute("aria-checked", "true");
    expect(card("Nativo")).toHaveAttribute("aria-checked", "false");
  });

  it("keeps a chosen folder skin that is gone, as unavailable", () => {
    render(
      <StyleGallery
        label="Tema"
        dock={{ ...DOCK, style: "custom", customSkin: "Sumiu" }}
        appIds={[]}
        onSelect={vi.fn()}
        folderSkins={[]}
      />,
    );

    const cards = within(gallery()).getAllByRole("radio");
    const gone = cards[cards.length - 1];
    expect(gone).toHaveTextContent("Sumiu");
    expect(gone).toHaveTextContent("Indisponível");
    expect(gone).toHaveAttribute("aria-checked", "true");
    expect(gone?.querySelector(".mini-bar")).not.toHaveAttribute("data-skin");
  });

  it("can't choose while the settings can't be read", () => {
    render(
      <StyleGallery
        label="Tema"
        dock={null}
        appIds={[]}
        disabled
        onSelect={vi.fn()}
      />,
    );

    for (const each of within(gallery()).getAllByRole("radio")) {
      expect(each).toBeDisabled();
      expect(each).toHaveAttribute("aria-checked", "false");
    }
  });
});

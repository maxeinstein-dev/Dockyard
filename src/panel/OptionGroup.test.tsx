import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import OptionGroup, { type Option } from "./OptionGroup";

type Size = "small" | "medium" | "large";

const OPTIONS: readonly Option<Size>[] = [
  { value: "small", label: "Pequeno" },
  { value: "medium", label: "Médio" },
  { value: "large", label: "Grande" },
];

function renderGroup(selected: Size | null = "medium") {
  const onSelect = vi.fn<(value: Size) => void>();
  const view = render(
    <>
      <button type="button">Antes</button>
      <OptionGroup
        label="Tamanho"
        options={OPTIONS}
        selected={selected}
        onSelect={onSelect}
      />
      <button type="button">Depois</button>
    </>,
  );
  return { ...view, onSelect };
}

const group = () => screen.getByRole("group", { name: "Tamanho" });
const option = (name: string) => within(group()).getByRole("button", { name });
const tabStops = () =>
  within(group())
    .getAllByRole("button")
    .filter((button) => button.tabIndex === 0)
    .map((button) => button.textContent);

describe("OptionGroup", () => {
  it("is a group named by its visible label, one toggle button per option", () => {
    renderGroup();

    const label = screen.getByText("Tamanho");
    expect(group()).toHaveAttribute("aria-labelledby", label.id);
    const buttons = within(group()).getAllByRole("button");
    expect(buttons.map((button) => button.textContent)).toEqual([
      "Pequeno",
      "Médio",
      "Grande",
    ]);
    for (const button of buttons) {
      expect(button).toHaveAttribute("type", "button");
    }
  });

  it("presses only the chosen option", () => {
    renderGroup("large");

    expect(option("Pequeno")).toHaveAttribute("aria-pressed", "false");
    expect(option("Médio")).toHaveAttribute("aria-pressed", "false");
    expect(option("Grande")).toHaveAttribute("aria-pressed", "true");
  });

  it("presses none when the value matches no option", () => {
    renderGroup(null);

    for (const button of within(group()).getAllByRole("button")) {
      expect(button).toHaveAttribute("aria-pressed", "false");
    }
    // The group still has its one Tab stop, on the first option.
    expect(tabStops()).toEqual(["Pequeno"]);
  });

  it("has one Tab stop, on the chosen option", () => {
    renderGroup("large");

    expect(tabStops()).toEqual(["Grande"]);
  });

  it("chooses another option on click", () => {
    const { onSelect } = renderGroup("medium");

    fireEvent.click(option("Grande"));
    expect(onSelect).toHaveBeenCalledExactlyOnceWith("large");
  });

  it("does not choose the chosen option again", () => {
    const { onSelect } = renderGroup("medium");

    fireEvent.click(option("Médio"));
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("moves the focus with the arrows, wrapping, without choosing", () => {
    const { onSelect } = renderGroup("medium");
    option("Médio").focus();

    fireEvent.keyDown(option("Médio"), { key: "ArrowRight" });
    expect(option("Grande")).toHaveFocus();
    fireEvent.keyDown(option("Grande"), { key: "ArrowDown" });
    expect(option("Pequeno")).toHaveFocus();
    fireEvent.keyDown(option("Pequeno"), { key: "ArrowLeft" });
    expect(option("Grande")).toHaveFocus();
    fireEvent.keyDown(option("Grande"), { key: "ArrowUp" });
    expect(option("Médio")).toHaveFocus();

    expect(onSelect).not.toHaveBeenCalled();
    // The pressed option did not move with the focus.
    expect(option("Médio")).toHaveAttribute("aria-pressed", "true");
  });

  it("jumps to the ends with Home and End", () => {
    renderGroup("medium");
    option("Médio").focus();

    fireEvent.keyDown(option("Médio"), { key: "End" });
    expect(option("Grande")).toHaveFocus();
    fireEvent.keyDown(option("Grande"), { key: "Home" });
    expect(option("Pequeno")).toHaveFocus();
  });

  it("keeps the Tab stop on the focused option and stops the page from scrolling", () => {
    renderGroup("small");
    option("Pequeno").focus();

    const arrow = fireEvent.keyDown(option("Pequeno"), { key: "ArrowRight" });
    expect(arrow).toBe(false);
    expect(tabStops()).toEqual(["Médio"]);
  });

  it("lets other keys through untouched", () => {
    renderGroup("small");
    option("Pequeno").focus();

    const tab = fireEvent.keyDown(option("Pequeno"), { key: "Tab" });
    expect(tab).toBe(true);
    expect(option("Pequeno")).toHaveFocus();
  });

  it("puts the Tab stop back on the chosen option once the focus leaves the group", () => {
    renderGroup("small");
    option("Pequeno").focus();
    fireEvent.keyDown(option("Pequeno"), { key: "End" });
    expect(tabStops()).toEqual(["Grande"]);

    // Moving inside the group keeps the roving stop.
    fireEvent.blur(option("Grande"), { relatedTarget: option("Médio") });
    fireEvent.focus(option("Médio"));
    expect(tabStops()).toEqual(["Médio"]);

    const after = screen.getByRole("button", { name: "Depois" });
    fireEvent.blur(option("Médio"), { relatedTarget: after });
    expect(tabStops()).toEqual(["Pequeno"]);
  });

  it("follows a new chosen value from its owner", () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <OptionGroup
        label="Tamanho"
        options={OPTIONS}
        selected="small"
        onSelect={onSelect}
      />,
    );
    rerender(
      <OptionGroup
        label="Tamanho"
        options={OPTIONS}
        selected="large"
        onSelect={onSelect}
      />,
    );

    expect(option("Grande")).toHaveAttribute("aria-pressed", "true");
    expect(option("Pequeno")).toHaveAttribute("aria-pressed", "false");
    expect(tabStops()).toEqual(["Grande"]);
  });
});

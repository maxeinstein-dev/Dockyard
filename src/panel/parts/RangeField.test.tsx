import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import RangeField, { type RangeFieldProps } from "./RangeField";

const percent = (value: number) => `${value}%`;

function renderRange(props: Partial<RangeFieldProps> = {}) {
  const onCommit = vi.fn<(value: number) => void>();
  const all: RangeFieldProps = {
    label: "Opacidade",
    value: 40,
    min: 0,
    max: 100,
    step: 5,
    display: percent,
    onCommit,
    ...props,
  };
  const view = render(<RangeField {...all} />);
  const rerender = (next: Partial<RangeFieldProps>) =>
    view.rerender(<RangeField {...all} {...next} />);
  return { ...view, rerender, onCommit };
}

const slider = () =>
  screen.getByRole<HTMLInputElement>("slider", { name: "Opacidade" });
/** The figure at the right of the label (hidden: the slider reads it). */
const figure = (container: HTMLElement) =>
  container.querySelector(".range-value");

describe("RangeField", () => {
  it("is a slider named by its label, with its bounds, step and value", () => {
    const { container } = renderRange();

    const input = slider();
    expect(input).toHaveAttribute("type", "range");
    expect(input).toHaveAttribute("min", "0");
    expect(input).toHaveAttribute("max", "100");
    expect(input).toHaveAttribute("step", "5");
    expect(input.value).toBe("40");
    expect(input).toHaveAttribute("aria-valuetext", "40%");
    expect(figure(container)).toHaveTextContent("40%");
    expect(figure(container)).toHaveAttribute("aria-hidden", "true");
  });

  it("steps by 1 when no step is given", () => {
    renderRange({ step: undefined });

    expect(slider()).toHaveAttribute("step", "1");
  });

  it("is described by its hint, and by nothing without one", () => {
    const { rerender } = renderRange({ hint: "Quanto o fundo deixa ver." });

    expect(slider()).toHaveAccessibleDescription("Quanto o fundo deixa ver.");

    rerender({ hint: undefined });
    expect(slider()).not.toHaveAttribute("aria-describedby");
    expect(screen.queryByText("Quanto o fundo deixa ver.")).toBeNull();
  });

  it("follows a drag on the screen only, and commits once on release", () => {
    const { container, onCommit } = renderRange();

    fireEvent.change(slider(), { target: { value: "55" } });
    fireEvent.change(slider(), { target: { value: "70" } });
    expect(onCommit).not.toHaveBeenCalled();
    expect(slider()).toHaveAttribute("aria-valuetext", "70%");
    expect(figure(container)).toHaveTextContent("70%");

    fireEvent.pointerUp(slider());
    expect(onCommit).toHaveBeenCalledExactlyOnceWith(70);
  });

  it("commits on key release and on blur", () => {
    const { onCommit } = renderRange();

    fireEvent.change(slider(), { target: { value: "45" } });
    fireEvent.keyUp(slider(), { key: "ArrowRight" });
    expect(onCommit).toHaveBeenLastCalledWith(45);

    fireEvent.change(slider(), { target: { value: "50" } });
    fireEvent.blur(slider());
    expect(onCommit).toHaveBeenLastCalledWith(50);
    expect(onCommit).toHaveBeenCalledTimes(2);
  });

  it("does not commit with no change, nor a drag back to the value", () => {
    const { onCommit } = renderRange();

    fireEvent.pointerUp(slider());
    fireEvent.keyUp(slider(), { key: "Tab" });
    fireEvent.blur(slider());
    expect(onCommit).not.toHaveBeenCalled();

    fireEvent.change(slider(), { target: { value: "60" } });
    fireEvent.change(slider(), { target: { value: "40" } });
    fireEvent.pointerUp(slider());
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("commits the same draft only once", () => {
    const { onCommit } = renderRange();

    fireEvent.change(slider(), { target: { value: "80" } });
    fireEvent.pointerUp(slider());
    fireEvent.keyUp(slider(), { key: "ArrowRight" });
    fireEvent.blur(slider());
    expect(onCommit).toHaveBeenCalledExactlyOnceWith(80);
  });

  it("shows the owner's value again after a commit", () => {
    const { container, rerender } = renderRange();

    fireEvent.change(slider(), { target: { value: "90" } });
    fireEvent.pointerUp(slider());
    // The owner refused or has not saved yet: its value shows.
    expect(slider().value).toBe("40");
    expect(figure(container)).toHaveTextContent("40%");

    rerender({ value: 90 });
    expect(slider().value).toBe("90");
    expect(slider()).toHaveAttribute("aria-valuetext", "90%");
  });

  it("reaches both ends", () => {
    const { onCommit } = renderRange();

    fireEvent.change(slider(), { target: { value: "100" } });
    fireEvent.pointerUp(slider());
    fireEvent.change(slider(), { target: { value: "0" } });
    fireEvent.pointerUp(slider());
    expect(onCommit.mock.calls).toEqual([[100], [0]]);
  });

  it("writes the value with the given display", () => {
    renderRange({
      value: 1.1,
      min: 1,
      max: 1.5,
      step: 0.05,
      display: (value) => `${value.toFixed(2).replace(".", ",")}×`,
    });

    expect(slider()).toHaveAttribute("aria-valuetext", "1,10×");
  });

  it("can be disabled", () => {
    renderRange({ disabled: true });

    expect(slider()).toBeDisabled();
  });
});

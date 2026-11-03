import { render } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import {
  HoverThumb,
  IndicatorThumb,
  OpenThumb,
  ScreenThumb,
  ShapeThumb,
} from "./ChoiceThumbs";

/* The pictures of the visual choices: decorative, and drawn by CSS from
 * their `data-*` attributes (ChoiceThumbs.css), so those are the contract. */

/** Renders `node` and returns its root `.thumb`. */
function thumb(node: ReactElement): HTMLElement {
  const { container } = render(node);
  const root = container.querySelector<HTMLElement>(".thumb");
  if (!root) throw new Error("no .thumb rendered");
  return root;
}

describe("ChoiceThumbs", () => {
  it("hides every picture from assistive technology", () => {
    const pictures = [
      <ScreenThumb key="screen" />,
      <HoverThumb key="hover" effect="scale" chosen />,
      <IndicatorThumb key="indicator" shape="dot" />,
      <ShapeThumb key="shape" shape="plate" />,
      <OpenThumb key="open" animation="bounce" chosen={false} />,
    ];
    for (const picture of pictures) {
      const root = thumb(picture);
      expect(root).toHaveAttribute("aria-hidden", "true");
      // Nothing inside has a role or a name of its own.
      expect(root.querySelector("[role], [aria-label], img")).toBeNull();
    }
  });

  describe("ScreenThumb", () => {
    it("defaults to the dock at the bottom over a visible taskbar", () => {
      const root = thumb(<ScreenThumb />);
      expect(root).toHaveClass("thumb-screen");
      expect(root).toHaveAttribute("data-edge", "bottom");
      expect(root).toHaveAttribute("data-taskbar", "visible");
    });

    it("draws the edge and the taskbar mode it is given", () => {
      const root = thumb(<ScreenThumb edge="top" taskbar="hidden" />);
      expect(root).toHaveAttribute("data-edge", "top");
      expect(root).toHaveAttribute("data-taskbar", "hidden");
      expect(root.querySelector(".thumb-taskbar")).not.toBeNull();
    });

    it("puts three small tiles on the dock", () => {
      const root = thumb(<ScreenThumb />);
      const tiles = root.querySelectorAll(".thumb-dock .mini-tile");
      expect(tiles).toHaveLength(3);
      for (const tile of tiles) expect(tile).toHaveClass("small");
    });
  });

  describe("HoverThumb", () => {
    it("marks the middle of three tiles and carries the effect", () => {
      const root = thumb(<HoverThumb effect="magnify" chosen={false} />);
      expect(root).toHaveAttribute("data-hover", "magnify");
      const slots = [...root.querySelectorAll(".thumb-slot")];
      expect(slots).toHaveLength(3);
      expect(slots.map((slot) => slot.hasAttribute("data-middle"))).toEqual([
        false,
        true,
        false,
      ]);
      expect(root.querySelector(".thumb-pointer")).not.toBeNull();
    });

    it("moves only when chosen", () => {
      expect(thumb(<HoverThumb effect="scale" chosen />)).toHaveAttribute(
        "data-chosen",
        "",
      );
      expect(
        thumb(<HoverThumb effect="scale" chosen={false} />),
      ).not.toHaveAttribute("data-chosen");
    });
  });

  describe("IndicatorThumb", () => {
    it.each(["dot", "dash"] as const)(
      "draws a %s under each of the three tiles",
      (shape) => {
        const root = thumb(<IndicatorThumb shape={shape} />);
        expect(root).toHaveAttribute("data-indicator", shape);
        const marks = [...root.querySelectorAll(".thumb-mark")];
        expect(marks.map((mark) => mark.getAttribute("data-at"))).toEqual([
          "0",
          "1",
          "2",
        ]);
        for (const mark of marks) {
          expect(mark).toHaveAttribute("data-shape", shape);
        }
      },
    );
  });

  describe("ShapeThumb", () => {
    it.each(["plate", "uniform", "original"] as const)(
      "shows a round, a square and a wide icon evened out as %s",
      (shape) => {
        const root = thumb(<ShapeThumb shape={shape} />);
        expect(root).toHaveAttribute("data-shape", shape);
        const forms = [...root.querySelectorAll(".thumb-icon")].map((icon) =>
          icon.getAttribute("data-form"),
        );
        expect(forms).toEqual(["round", "square", "wide"]);
      },
    );
  });

  describe("OpenThumb", () => {
    it.each(["bounce", "pulse", "shake"] as const)(
      "carries the %s animation, moving only when chosen",
      (animation) => {
        const chosen = thumb(<OpenThumb animation={animation} chosen />);
        expect(chosen).toHaveAttribute("data-open", animation);
        expect(chosen).toHaveAttribute("data-chosen", "");
        expect(chosen.querySelectorAll(".thumb-slot")).toHaveLength(3);

        const idle = thumb(<OpenThumb animation={animation} chosen={false} />);
        expect(idle).not.toHaveAttribute("data-chosen");
      },
    );
  });
});

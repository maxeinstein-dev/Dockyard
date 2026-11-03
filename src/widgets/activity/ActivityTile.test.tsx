import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import ActivityTile from "./ActivityTile";

describe("ActivityTile", () => {
  it("is a decorative meter glyph, outside the accessibility tree", () => {
    const { container } = render(<ActivityTile />);

    const tile = container.firstElementChild;
    expect(tile).toHaveClass("sys-tile");
    expect(tile).toHaveAttribute("aria-hidden", "true");
    expect(tile?.querySelector("svg")).toHaveAttribute("viewBox", "0 0 24 24");
    // Three bars of different heights: the meter, never a reading.
    expect(tile?.querySelector("path")).toHaveAttribute(
      "d",
      "M7 19V11M12 19V5M17 19v-5",
    );
    expect(container).not.toHaveTextContent(/\d/);
  });
});

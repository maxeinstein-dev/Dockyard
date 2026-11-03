import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import NetworkTile from "./NetworkTile";

describe("NetworkTile", () => {
  it("is a decorative pair of arrows, outside the accessibility tree", () => {
    const { container } = render(<NetworkTile />);

    const tile = container.firstElementChild;
    expect(tile).toHaveClass("sys-tile");
    expect(tile).toHaveAttribute("aria-hidden", "true");
    expect(tile?.querySelector("svg")).toHaveAttribute("viewBox", "0 0 24 24");
    // Down on the left, up on the right: never a reading.
    expect(tile?.querySelector("path")).toHaveAttribute(
      "d",
      "M8.5 4.5v15M4.5 15.5l4 4 4-4M15.5 19.5v-15M11.5 8.5l4-4 4 4",
    );
    expect(container).not.toHaveTextContent(/\d/);
  });
});

import { describe, expect, it } from "vitest";
import { blockInk } from "./calendar";

describe("blockInk", () => {
  it("puts black on a light calendar color and white on a dark one", () => {
    // Google's "Mirtilo claro" and a deep violet, as the dock's tests.
    expect(blockInk("#9fc6e7")).toBe("#000000");
    expect(blockInk("#4a1d8c")).toBe("#ffffff");
    expect(blockInk("#2F5BD3")).toBe("#ffffff");
  });

  it("leaves an event without a color to the accent's ink", () => {
    expect(blockInk(null)).toBeNull();
    expect(blockInk("red")).toBeNull();
    expect(blockInk("#fff")).toBeNull();
  });
});

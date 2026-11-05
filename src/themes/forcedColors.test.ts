import { describe, expect, it } from "vitest";

/* M6: in Windows' contrast mode the shadows and tints are gone, and the
 * focus line is the only sign of focus. A rule that takes the outline off a
 * focused control must give one back in the file's `forced-colors` block,
 * or someone on the keyboard does not see where they are. */

const sheets = import.meta.glob("../**/*.css", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** Focused, but not a control: the outline stays off on purpose. */
const NOT_CONTROLS: Record<string, string> = {
  // The tour's title takes the focus from the code, so a screen reader
  // reads it; the keyboard never lands on it.
  ".tour-title:focus": "../tour/tour.css",
  // The same in Configurações (spec 0029 §3, §8.5): the section's title
  // when opened by the keyboard, a group's from the index.
  ".section-header h1:focus": "../settings/SettingsApp.css",
  ".settings-group-title:focus": "../settings/SettingsApp.css",
};

/** The inside of every `@media (forced-colors: active)` block of `css`. */
function forcedBlocks(css: string): string {
  const blocks: string[] = [];
  const opening = /@media\s*\(forced-colors:\s*active\)\s*\{/g;
  for (let found = opening.exec(css); found; found = opening.exec(css)) {
    let depth = 1;
    let at = opening.lastIndex;
    while (depth > 0 && at < css.length) {
      if (css[at] === "{") depth++;
      if (css[at] === "}") depth--;
      at++;
    }
    blocks.push(css.slice(opening.lastIndex, at - 1));
  }
  return blocks.join("\n");
}

/** The rules of `css` as [selectors, declarations], comments dropped. */
function rules(css: string): [string[], string][] {
  const plain = css.replace(/\/\*[\s\S]*?\*\//g, "");
  return [...plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(
    ([, selectors = "", body = ""]) => [
      selectors
        .split(",")
        .map((selector) => selector.trim())
        .filter(Boolean),
      body,
    ],
  );
}

/** The focus selectors of `css` that take the outline off. */
function outlinesTakenOff(css: string): string[] {
  return rules(css)
    .filter(([, body]) => /outline:\s*(none|0)\b/.test(body))
    .flatMap(([selectors]) => selectors)
    .filter((selector) => selector.includes(":focus"));
}

/** The focus selectors that draw an outline in the contrast mode. */
function outlinesGivenBack(css: string): Set<string> {
  return new Set(
    rules(forcedBlocks(css))
      .filter(([, body]) => /outline:\s*(?!none|0\b)/.test(body))
      .flatMap(([selectors]) => selectors),
  );
}

describe("Windows' contrast mode", () => {
  it("finds the style sheets", () => {
    expect(Object.keys(sheets).length).toBeGreaterThan(20);
  });

  it("shows the focus of every control that takes its outline off", () => {
    const missing = Object.entries(sheets).flatMap(([path, css]) => {
      const given = outlinesGivenBack(css);
      return outlinesTakenOff(css)
        .filter((selector) => NOT_CONTROLS[selector] !== path)
        .filter((selector) => !given.has(selector))
        .map((selector) => `${path}: ${selector}`);
    });
    expect(missing).toEqual([]);
  });

  it("marks the chosen card with the selection's color, not a shadow", () => {
    const css = sheets["../panel/parts/ChoiceCards.css"];
    expect(css).toBeDefined();
    const forced = forcedBlocks(css ?? "");
    expect(forced).toMatch(/\[aria-checked="true"\]::after/);
    expect(forced).toMatch(/border:\s*3px solid Highlight/);
  });
});

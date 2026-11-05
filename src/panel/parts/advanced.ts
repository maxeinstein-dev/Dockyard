/** The Avançado open in this window, by its `id`: kept while the window
 * lives, closed again in the next one (spec 0029 §4). */
export const opened = new Set<string>();

/** Closes every Avançado again, as a new window does: for the tests. */
export function forgetAdvanced() {
  opened.clear();
}

/** Opens the Avançado `inside` sits in, when it is closed: the search and
 * the index lead into it. True when it was closed; it shows on the next
 * render. */
export function openAdvanced(inside: Element): boolean {
  const toggle = inside
    .closest(".settings-advanced")
    ?.querySelector<HTMLButtonElement>(
      ".settings-advanced-toggle[aria-expanded='false']",
    );
  toggle?.click();
  return toggle !== undefined && toggle !== null;
}

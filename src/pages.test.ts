import { describe, expect, it } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import process from "node:process";

/* Every page names its icon, the Dockyard logo in `public/`. Without one,
 * WebView2 asks for `/favicon.ico`, which the app does not have, and each
 * page load wrote "asset not found: favicon.ico" to the log. */

// Vitest runs from the project folder, where the pages are.
const root = (path: string) => resolve(process.cwd(), path);
const PAGES = ["panel.html", "settings.html", "tour.html", "e2e.html"];

describe("the pages' icon", () => {
  it.each(PAGES)("%s links the Dockyard logo", (page) => {
    const html = readFileSync(root(page), "utf8");
    const link = /<link rel="icon" type="image\/png" href="\/([^"]+)" \/>/.exec(
      html,
    );
    expect(link, page).not.toBeNull();
    expect(existsSync(root(`public/${link?.[1] ?? ""}`)), page).toBe(true);
  });
});

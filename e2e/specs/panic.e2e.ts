import { browser } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import { dockPanic, dockView } from "../app.ts";

/* M4: a panic in a window message of the dock does not take the app down.
 * The test build makes the dock panic in its next paint; the window
 * procedure catches it, and the dock is made again from scratch. Nothing is
 * saved by it: the dock shows the same items as before. */

describe("A panic in the dock", () => {
  it("is caught, and the dock comes back with its items", async () => {
    const clock = t.widgets.kinds.clock;
    await browser.waitUntil(
      async () => (await dockView()).items.some((item) => item.name === clock),
      { timeoutMsg: "the dock never showed the Relógio" },
    );
    const before = (await dockView()).items.map((item) => item.name);

    await dockPanic();

    // Made again: the app still answers, and the new dock has the items.
    await browser.waitUntil(
      async () => {
        const view = await dockView().catch(() => null);
        return (
          view !== null &&
          view.items.map((item) => item.name).join("|") === before.join("|")
        );
      },
      { timeoutMsg: "the dock did not come back after the panic" },
    );
  });
});

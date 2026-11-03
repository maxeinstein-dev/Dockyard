import { browser } from "@wdio/globals";
import { ptBR as t } from "../../src/i18n/pt-BR.ts";
import { dockPanic, dockView, restartApp, serverUp } from "../app.ts";

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

  it("closes the app when it keeps panicking", async () => {
    // One panic after another, each one let finish: the dock comes back
    // (made again, at most three times a minute) or the app closes. The
    // test above may have spent one of the three.
    let panics = 0;
    let closed = false;
    while (!closed && panics < 5) {
      await dockPanic().catch(() => undefined);
      panics += 1;
      await browser.waitUntil(
        async () => {
          if (!(await serverUp())) {
            closed = true;
            return true;
          }
          return (await dockView().catch(() => null)) !== null;
        },
        {
          timeout: 30_000,
          timeoutMsg: "the dock neither came back nor closed the app",
        },
      );
    }
    // Closed by the fourth panic within the minute, not made again forever.
    if (!closed)
      throw new Error(`the app was still open after ${panics} panics`);
    if (panics > 4)
      throw new Error(`the app closed only after ${panics} panics`);

    // Up again for the specs after this one.
    await restartApp();
  });
});

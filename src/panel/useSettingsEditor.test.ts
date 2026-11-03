import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_LOOK, type Settings } from "../core/settings";
import { useSettingsEditor } from "./useSettingsEditor";

const { invoke, log } = vi.hoisted(() => ({
  invoke: vi.fn(),
  log: { error: vi.fn(), warn: vi.fn(), info: vi.fn() },
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("../core/log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../core/log")>()),
  log,
}));

const SETTINGS: Settings = {
  version: 1,
  theme: "dark",
  launchAtLogin: false,
  launcher: true,
  launcherKeywords: [],
  launcherHotkey: "Alt+Space",
  language: "system",
  tourShown: true,
  dock: {
    edge: "bottom",
    iconSize: 48,
    opacity: "translucent",
    ...DEFAULT_LOOK,
  },
};

/** A promise settled by the test. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** Lets settled promises run their callbacks. */
const flush = () => act(() => Promise.resolve());

/** The hook over `settings`, which the test changes as `settings-changed`
 * would. */
function editor(settings: Settings | null) {
  return renderHook(
    ({ settings }: { settings: Settings | null }) =>
      useSettingsEditor(settings),
    { initialProps: { settings } },
  );
}

/** Each `update_settings` call answers when the test settles it. */
function answers() {
  const calls: ReturnType<typeof deferred<Settings>>[] = [];
  invoke.mockImplementation(() => {
    const call = deferred<Settings>();
    calls.push(call);
    return call.promise;
  });
  return calls;
}

const light: (current: Settings) => Settings = (current) => ({
  ...current,
  theme: "light",
});

describe("useSettingsEditor", () => {
  beforeEach(() => {
    invoke.mockReset();
    log.warn.mockReset();
  });

  it("shows the settings in use while nothing is changing", () => {
    const { result } = editor(SETTINGS);
    expect(result.current.shown).toBe(SETTINGS);
    expect(result.current.error).toBeNull();
  });

  it("sends the full settings and shows the change at once", () => {
    answers();
    const { result } = editor(SETTINGS);

    act(() => result.current.change(light));

    const sent = { ...SETTINGS, theme: "light" };
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("update_settings", { settings: sent });
    expect(result.current.shown).toEqual(sent);
  });

  it("builds a second change on the first one still in flight", () => {
    answers();
    const { result } = editor(SETTINGS);

    act(() => result.current.change(light));
    act(() =>
      result.current.change((current) => ({ ...current, launcher: false })),
    );

    // The second save keeps the first change instead of starting over from
    // the confirmed settings.
    expect(invoke).toHaveBeenLastCalledWith("update_settings", {
      settings: { ...SETTINGS, theme: "light", launcher: false },
    });
  });

  it("keeps the change shown once saved, until the new settings arrive", async () => {
    const calls = answers();
    const { result, rerender } = editor(SETTINGS);
    act(() => result.current.change(light));
    const sent = { ...SETTINGS, theme: "light" as const };

    calls[0]?.resolve(sent);
    await flush();
    // Saved, but `settings-changed` has not come yet.
    expect(result.current.shown).toEqual(sent);

    const confirmed = { ...sent };
    rerender({ settings: confirmed });
    expect(result.current.shown).toBe(confirmed);
  });

  it("keeps the change shown when the new settings arrive before the save answers", async () => {
    const calls = answers();
    const { result, rerender } = editor(SETTINGS);
    act(() => result.current.change(light));
    const sent = { ...SETTINGS, theme: "light" as const };

    // `settings-changed` first: still waiting for the command's answer.
    const confirmed = { ...sent };
    rerender({ settings: confirmed });
    expect(result.current.shown).toEqual(sent);
    expect(result.current.shown).not.toBe(confirmed);

    calls[0]?.resolve(sent);
    await flush();
    expect(result.current.shown).toBe(confirmed);
  });

  it("goes back to the settings in use with the reason when Rust refuses", async () => {
    const calls = answers();
    const { result } = editor(SETTINGS);
    act(() => result.current.change(light));

    calls[0]?.reject({ code: "lookOutOfRange" });
    await flush();

    expect(result.current.shown).toBe(SETTINGS);
    expect(result.current.error).toBe("lookOutOfRange");
  });

  it("says notReady when the save fails for another reason", async () => {
    const calls = answers();
    const { result } = editor(SETTINGS);
    act(() => result.current.change(light));

    calls[0]?.reject(new Error("ipc down"));
    await flush();

    expect(result.current.shown).toBe(SETTINGS);
    expect(result.current.error).toBe("notReady");
  });

  it("refuses a change while the settings can't be read", () => {
    const next = vi.fn(light);
    const { result } = editor(null);

    act(() => result.current.change(next));

    expect(result.current.error).toBe("notReady");
    expect(result.current.shown).toBeNull();
    expect(next).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("clears the last refusal at the next change", async () => {
    const calls = answers();
    const { result } = editor(SETTINGS);
    act(() => result.current.change(light));
    calls[0]?.reject({ code: "iconSizeOutOfRange" });
    await flush();
    expect(result.current.error).toBe("iconSizeOutOfRange");

    act(() => result.current.change(light));
    expect(result.current.error).toBeNull();
  });

  it("ignores an older change refused after a newer one was made", async () => {
    const calls = answers();
    const { result } = editor(SETTINGS);
    act(() => result.current.change(light));
    act(() =>
      result.current.change((current) => ({ ...current, launcher: false })),
    );

    calls[0]?.reject({ code: "lookOutOfRange" });
    await flush();

    // The newer change is still the one shown, without the older's reason.
    expect(result.current.shown).toEqual({
      ...SETTINGS,
      theme: "light",
      launcher: false,
    });
    expect(result.current.error).toBeNull();
  });

  it("does not take an older change's save for the newer one", async () => {
    const calls = answers();
    const { result, rerender } = editor(SETTINGS);
    act(() => result.current.change(light));
    act(() =>
      result.current.change((current) => ({ ...current, launcher: false })),
    );
    const latest = { ...SETTINGS, theme: "light" as const, launcher: false };

    calls[0]?.resolve({ ...SETTINGS, theme: "light" });
    await flush();
    // The older change's settings arrive: the newer one is not saved yet.
    rerender({ settings: { ...SETTINGS, theme: "light" } });
    expect(result.current.shown).toEqual(latest);

    calls[1]?.resolve(latest);
    await flush();
    const confirmed = { ...latest };
    rerender({ settings: confirmed });
    expect(result.current.shown).toBe(confirmed);
  });
});

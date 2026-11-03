import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activeLocale, resolveLocale, setLanguage, useT } from ".";
import { en } from "./en";
import { ptBR } from "./pt-BR";

/** Makes the WebView report `language` as the system's. */
function systemLanguage(language: string) {
  vi.spyOn(navigator, "language", "get").mockReturnValue(language);
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  // The other tests read the PT-BR texts (see `src/test/setup.ts`).
  setLanguage("pt-BR");
});

describe("resolveLocale", () => {
  it("keeps a chosen language whatever the system's", () => {
    expect(resolveLocale("pt-BR", "en-US")).toBe("pt-BR");
    expect(resolveLocale("en", "pt-BR")).toBe("en");
  });

  it.each(["pt-BR", "pt-PT", "pt", "PT-br"])(
    "follows a Portuguese system (%s) with PT-BR",
    (system) => {
      expect(resolveLocale("system", system)).toBe("pt-BR");
    },
  );

  it.each(["en-US", "en-GB", "es-ES", "fr", "ja-JP"])(
    "falls back to English on any other system (%s)",
    (system) => {
      expect(resolveLocale("system", system)).toBe("en");
    },
  );

  it("reads the system language from the WebView by default", () => {
    systemLanguage("pt-PT");
    expect(resolveLocale("system")).toBe("pt-BR");
    systemLanguage("de-DE");
    expect(resolveLocale("system")).toBe("en");
  });

  it("uses PT-BR when there is no navigator to ask", () => {
    vi.stubGlobal("navigator", undefined);
    expect(resolveLocale("system")).toBe("pt-BR");
  });
});

describe("the starting locale", () => {
  it.each([
    ["pt-BR", "pt-BR"],
    ["en-US", "en"],
  ] as const)(
    "is the system's (%s) until the settings arrive",
    async (system, locale) => {
      systemLanguage(system);
      vi.resetModules();
      const fresh = await import(".");
      expect(fresh.activeLocale()).toBe(locale);
    },
  );
});

describe("setLanguage", () => {
  it("switches every useT to the new locale", () => {
    const { result } = renderHook(() => useT());
    expect(result.current).toBe(ptBR);

    act(() => setLanguage("en"));
    expect(result.current).toBe(en);
    expect(activeLocale()).toBe("en");

    act(() => setLanguage("pt-BR"));
    expect(result.current).toBe(ptBR);
  });

  it("follows the system for Sistema", () => {
    const { result } = renderHook(() => useT());

    systemLanguage("en-US");
    act(() => setLanguage("system"));
    expect(result.current).toBe(en);

    systemLanguage("pt-BR");
    act(() => setLanguage("system"));
    expect(result.current).toBe(ptBR);
  });

  it("marks the document with the locale", () => {
    setLanguage("en");
    expect(document.documentElement.lang).toBe("en");
    setLanguage("pt-BR");
    expect(document.documentElement.lang).toBe("pt-BR");
  });

  it("marks the document even when the locale is already in use", () => {
    document.documentElement.lang = "";
    setLanguage("pt-BR");
    expect(document.documentElement.lang).toBe("pt-BR");
  });
});

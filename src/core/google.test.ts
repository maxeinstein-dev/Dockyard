import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  GOOGLE_ACCOUNTS_EVENT,
  GOOGLE_COMMANDS,
  cancelGoogleSignIn,
  connectGoogleAccount,
  disconnectGoogleAccount,
  googleCalendars,
  googleStatus,
  setGoogleClient,
  useGoogleStatus,
  type GoogleStatus,
} from "./google";

type Handler = () => void;

const { invoke, listen, handlers, unlisten, warn } = vi.hoisted(() => {
  const handlers = new Map<string, Handler>();
  const unlisten = vi.fn();
  return {
    handlers,
    unlisten,
    invoke: vi.fn(),
    listen: vi.fn((event: string, handler: Handler) => {
      handlers.set(event, handler);
      return Promise.resolve(unlisten);
    }),
    warn: vi.fn(),
  };
});
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));
vi.mock("./log", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./log")>()),
  log: { error: vi.fn(), warn, info: vi.fn() },
}));

const STATUS: GoogleStatus = {
  clientId: "123.apps.googleusercontent.com",
  accounts: ["ana@example.com"],
};

beforeEach(() => {
  invoke.mockReset();
  listen.mockClear();
  unlisten.mockClear();
  warn.mockClear();
  handlers.clear();
});

describe("google commands", () => {
  it("match the names of the Rust commands", () => {
    expect(GOOGLE_COMMANDS).toEqual({
      status: "google_status",
      setClient: "set_google_client",
      connect: "connect_google_account",
      cancel: "cancel_google_sign_in",
      calendars: "google_calendars",
      disconnect: "disconnect_google_account",
    });
    expect(GOOGLE_ACCOUNTS_EVENT).toBe("google-accounts-changed");
  });

  it("send each command its arguments and resolve with its answer", async () => {
    invoke.mockResolvedValueOnce(STATUS);
    await expect(googleStatus()).resolves.toEqual(STATUS);
    expect(invoke).toHaveBeenLastCalledWith("google_status", {});

    invoke.mockResolvedValueOnce(undefined);
    await setGoogleClient("id-1", "segredo");
    expect(invoke).toHaveBeenLastCalledWith("set_google_client", {
      id: "id-1",
      secret: "segredo",
    });

    invoke.mockResolvedValueOnce("ana@example.com");
    await expect(connectGoogleAccount()).resolves.toBe("ana@example.com");
    expect(invoke).toHaveBeenLastCalledWith("connect_google_account", {});

    invoke.mockResolvedValueOnce(undefined);
    await cancelGoogleSignIn();
    expect(invoke).toHaveBeenLastCalledWith("cancel_google_sign_in", {});

    const calendars = [
      {
        id: "primary",
        name: "Ana",
        color: "#3366cc",
        primary: true,
        access: "owner",
      },
    ];
    invoke.mockResolvedValueOnce(calendars);
    await expect(googleCalendars("ana@example.com")).resolves.toEqual(
      calendars,
    );
    expect(invoke).toHaveBeenLastCalledWith("google_calendars", {
      email: "ana@example.com",
    });

    invoke.mockResolvedValueOnce(undefined);
    await disconnectGoogleAccount("ana@example.com");
    expect(invoke).toHaveBeenLastCalledWith("disconnect_google_account", {
      email: "ana@example.com",
    });
    expect(warn).not.toHaveBeenCalled();
  });

  it.each([
    "noClient",
    "invalidClientId",
    "invalidClientSecret",
    "refused",
    "timedOut",
    "cancelled",
    "offline",
    "signInAgain",
    "notConnected",
    "unexpected",
    "vault",
    "notAllowed",
  ])("pass the code %s through as it came, without logging", async (code) => {
    invoke.mockRejectedValueOnce(code);

    await expect(connectGoogleAccount()).rejects.toBe(code);
    expect(warn).not.toHaveBeenCalled();
  });

  it.each([
    ["an unknown code", "somethingElse"],
    ["an Error", new Error("ipc down: ana@example.com")],
    ["an object", { code: "noClient" }],
    ["nothing", undefined],
    // `notReady` is TS's own: Rust never sends it.
    ["the TS-only code", "notReady"],
  ])("turn %s into notReady and log only its kind", async (_name, error) => {
    invoke.mockRejectedValueOnce(error);

    await expect(googleStatus()).rejects.toBe("notReady");
    expect(warn).toHaveBeenCalledTimes(1);
    const message = String(warn.mock.calls[0]?.[0]);
    expect(message).toMatch(/^google: a command failed: /);
    // Never the error's text: it may carry an e-mail.
    expect(message).not.toContain("ana@example.com");
  });
});

describe("useGoogleStatus", () => {
  it("is null while loading, then the status", async () => {
    let answer!: (status: GoogleStatus) => void;
    invoke.mockReturnValueOnce(
      new Promise<GoogleStatus>((resolve) => {
        answer = resolve;
      }),
    );
    const { result } = renderHook(() => useGoogleStatus());

    expect(result.current.status).toBeNull();
    await act(async () => answer(STATUS));
    expect(result.current.status).toEqual(STATUS);
  });

  it("reads again when an account comes or goes", async () => {
    invoke.mockResolvedValueOnce(STATUS);
    const { result } = renderHook(() => useGoogleStatus());
    await act(async () => {});
    expect(listen).toHaveBeenCalledWith(
      GOOGLE_ACCOUNTS_EVENT,
      expect.any(Function),
    );

    const next = { ...STATUS, accounts: [...STATUS.accounts, "bia@x.com"] };
    invoke.mockResolvedValueOnce(next);
    const handler = handlers.get(GOOGLE_ACCOUNTS_EVENT);
    if (!handler) throw new Error("no listener");
    await act(async () => handler());
    expect(result.current.status).toEqual(next);
    expect(invoke).toHaveBeenCalledTimes(2);
  });

  it("reads again on refresh", async () => {
    invoke.mockResolvedValueOnce({ clientId: null, accounts: [] });
    const { result } = renderHook(() => useGoogleStatus());
    await act(async () => {});
    expect(result.current.status).toEqual({ clientId: null, accounts: [] });

    invoke.mockResolvedValueOnce(STATUS);
    await act(async () => result.current.refresh());
    expect(result.current.status).toEqual(STATUS);
  });

  it("goes back to null when a reading fails", async () => {
    invoke.mockResolvedValueOnce(STATUS);
    const { result } = renderHook(() => useGoogleStatus());
    await act(async () => {});
    expect(result.current.status).toEqual(STATUS);

    invoke.mockRejectedValueOnce("unexpected");
    await act(async () => result.current.refresh());
    expect(result.current.status).toBeNull();
  });

  it("stops listening when unmounted", async () => {
    invoke.mockResolvedValue(STATUS);
    const { unmount } = renderHook(() => useGoogleStatus());
    await act(async () => {});
    expect(unlisten).not.toHaveBeenCalled();

    unmount();
    await act(async () => {});
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});

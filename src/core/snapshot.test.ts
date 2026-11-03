import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriSnapshot, type SnapshotSource } from "./snapshot";

type Handler = (event: { payload: unknown }) => void;

const { listen, handlers, stopped } = vi.hoisted(() => {
  const handlers = new Map<string, Handler>();
  const stopped = { count: 0 };
  return {
    handlers,
    stopped,
    listen: vi.fn((event: string, handler: Handler) => {
      handlers.set(event, handler);
      return Promise.resolve(() => {
        stopped.count += 1;
      });
    }),
  };
});
vi.mock("@tauri-apps/api/event", () => ({ listen }));

const EVENT = "numbers-changed";

/** A source of numbers: anything else Rust sends is not one. */
function source(read: () => Promise<unknown>) {
  return {
    event: EVENT,
    read,
    accept: (value: unknown) => (typeof value === "number" ? value : null),
    readFailed: vi.fn(),
    listenFailed: vi.fn(),
  } satisfies SnapshotSource<number>;
}

/** Sends `payload` as Rust's event. */
function send(payload: unknown) {
  act(() => handlers.get(EVENT)?.({ payload }));
}

beforeEach(() => {
  handlers.clear();
  listen.mockClear();
  stopped.count = 0;
});

describe("useTauriSnapshot", () => {
  it("is null while loading, then what was read", async () => {
    const numbers = source(() => Promise.resolve(1));
    const { result } = renderHook(() => useTauriSnapshot(numbers));
    expect(result.current).toBeNull();
    await waitFor(() => expect(result.current).toBe(1));
  });

  it("keeps what each event brings", async () => {
    const numbers = source(() => Promise.resolve(1));
    const { result } = renderHook(() => useTauriSnapshot(numbers));
    await waitFor(() => expect(result.current).toBe(1));
    send(2);
    expect(result.current).toBe(2);
  });

  it("drops a read older than an event", async () => {
    let answer: (value: unknown) => void = () => {};
    const numbers = source(() => new Promise((resolve) => (answer = resolve)));
    const { result } = renderHook(() => useTauriSnapshot(numbers));
    await waitFor(() => expect(handlers.has(EVENT)).toBe(true));
    send(2);
    await act(async () => answer(1));
    expect(result.current).toBe(2);
  });

  it("drops what is not a value, read or sent", async () => {
    const numbers = source(() => Promise.resolve("broken"));
    const { result } = renderHook(() => useTauriSnapshot(numbers));
    await act(async () => {});
    expect(result.current).toBeNull();
    send(3);
    send({ not: "a number" });
    expect(result.current).toBe(3);
  });

  it("says when the read fails, and stays null", async () => {
    const failure = new TypeError("C:\\Users\\someone");
    const numbers = source(() => Promise.reject(failure));
    const { result } = renderHook(() => useTauriSnapshot(numbers));
    await waitFor(() =>
      expect(numbers.readFailed).toHaveBeenCalledWith(failure),
    );
    expect(result.current).toBeNull();
  });

  it("says when it can't listen, and still shows what it read", async () => {
    const failure = new Error("no event system");
    listen.mockImplementationOnce(() => Promise.reject(failure));
    const numbers = source(() => Promise.resolve(1));
    const { result } = renderHook(() => useTauriSnapshot(numbers));
    await waitFor(() => expect(result.current).toBe(1));
    expect(numbers.listenFailed).toHaveBeenCalledWith(failure);
  });

  it("stops listening when the window lets go, and ignores what comes late", async () => {
    let answer: (value: unknown) => void = () => {};
    const numbers = source(() => new Promise((resolve) => (answer = resolve)));
    const { result, unmount } = renderHook(() => useTauriSnapshot(numbers));
    await waitFor(() => expect(handlers.has(EVENT)).toBe(true));
    unmount();
    await act(async () => answer(1));
    expect(stopped.count).toBe(1);
    expect(result.current).toBeNull();
    expect(numbers.readFailed).not.toHaveBeenCalled();
  });
});

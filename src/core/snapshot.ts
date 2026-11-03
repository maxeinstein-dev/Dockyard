import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

/* Most of what the windows show lives in Rust, which sends each new value
 * as an event: the pinned programs, the settings, the widgets... A window
 * reads the value once and then keeps the one each event brings. The rules
 * are the same for all of them, so they are written once here (M12). */

/** Something Rust keeps and announces. A module-level constant, so the hook
 * that follows it starts once. */
export interface SnapshotSource<T> {
  /** The event Rust sends with each new value. */
  event: string;
  /** Reads the value now. */
  read: () => Promise<unknown>;
  /** The value as the window keeps it, or null when what came is not one
   * (it is dropped). */
  accept: (value: unknown) => T | null;
  /** Says, in the log, that the read failed. Each source writes its own
   * line: a log line is a literal, with only the error's kind in it. */
  readFailed: (error: unknown) => void;
  /** Says, in the log, that the event can't be listened to. */
  listenFailed: (error: unknown) => void;
}

/**
 * The latest value of `source`: null until it is read, and for good if it
 * can't be. A change may arrive before the first read answers; the read is
 * older then, so it is dropped. A failure to listen is logged, and the
 * value read stays.
 */
export function useTauriSnapshot<T>(source: SnapshotSource<T>): T | null {
  const [value, setValue] = useState<T | null>(null);

  useEffect(() => {
    let active = true;
    let changed = false;

    const unlisten = listen<unknown>(source.event, (event) => {
      if (!active) return;
      const next = source.accept(event.payload);
      if (next === null) return;
      changed = true;
      setValue(next);
    }).catch((error: unknown) => {
      source.listenFailed(error);
      return undefined;
    });

    source.read().then(
      (current) => {
        if (!active || changed) return;
        const next = source.accept(current);
        if (next !== null) setValue(next);
      },
      (error: unknown) => {
        if (active) source.readFailed(error);
      },
    );

    return () => {
      active = false;
      void unlisten.then((stop) => stop?.());
    };
  }, [source]);

  return value;
}

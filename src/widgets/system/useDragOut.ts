import { useRef, type PointerEvent } from "react";

/** How far, in pixels, the pointer moves with the button held before a
 * press becomes a drag (Windows' own default). */
export const DRAG_DISTANCE = 4;

/**
 * A list of files the user drags out of a panel (F9 of estudo 0002: the
 * Capturas de tela and the Arquivos): once the pointer moves
 * `DRAG_DISTANCE` with the left button held, `drag(key)` asks Rust for the
 * native drag of that file; the click that follows a drag opens nothing.
 * `drag` and `open` log their own failures.
 */
export function useDragOut(
  drag: (key: number) => void,
  open: (key: number) => void,
) {
  /** Where the button went down, until it moves far enough to drag. */
  const pressed = useRef<{ key: number; x: number; y: number } | null>(null);
  /** A drag happened: the click that follows it opens nothing. */
  const dragged = useRef(false);

  return (key: number) => ({
    onPointerDown: (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) return;
      dragged.current = false;
      pressed.current = { key, x: event.clientX, y: event.clientY };
    },
    onPointerMove: (event: PointerEvent<HTMLElement>) => {
      const start = pressed.current;
      if (start === null || (event.buttons & 1) === 0) return;
      const far =
        Math.abs(event.clientX - start.x) >= DRAG_DISTANCE ||
        Math.abs(event.clientY - start.y) >= DRAG_DISTANCE;
      if (!far) return;
      pressed.current = null;
      dragged.current = true;
      drag(start.key);
    },
    onPointerUp: () => {
      pressed.current = null;
    },
    onDragStart: (event: { preventDefault: () => void }) =>
      event.preventDefault(),
    onClick: () => {
      if (dragged.current) {
        dragged.current = false;
        return;
      }
      open(key);
    },
  });
}

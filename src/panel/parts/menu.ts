import type { KeyboardEvent, MouseEvent } from "react";

/** The menu items of `root` that can be chosen, in order. */
export function menuItems(root: HTMLElement | null): HTMLButtonElement[] {
  return Array.from(
    root?.querySelectorAll<HTMLButtonElement>(
      'button:is([role="menuitem"], [role="menuitemradio"]):not(:disabled)',
    ) ?? [],
  );
}

/** The arrow keys, Home and End move the focus between the items of the menu
 * in `root`, without choosing. */
export function moveInMenu(
  root: HTMLElement | null,
  event: KeyboardEvent<HTMLElement>,
) {
  const buttons = menuItems(root);
  const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
  const target = {
    ArrowDown: buttons[Math.min(at + 1, buttons.length - 1)],
    ArrowUp: buttons[Math.max(at - 1, 0)],
    Home: buttons[0],
    End: buttons[buttons.length - 1],
  }[event.key];
  if (target) {
    event.preventDefault();
    target.focus();
  }
}

/** The pointer over an item of a menu takes the focus to it, as the
 * Dockyard's native menus do: one item lit at a time, and the arrow keys go
 * on from where the pointer is (achado do autor, 2026-11-05). */
export function followPointer(event: MouseEvent<HTMLElement>) {
  const item = (event.target as HTMLElement).closest<HTMLButtonElement>(
    'button:is([role="menuitem"], [role="menuitemradio"]):not(:disabled)',
  );
  if (item !== null && item !== document.activeElement) {
    item.focus({ preventScroll: true });
  }
}

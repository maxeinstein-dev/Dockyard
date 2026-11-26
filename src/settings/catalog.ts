import type { CreatableKind } from "../core/widgets";

export type Category =
  "system" | "time" | "media" | "productivity" | "personal";

/** The catalog's shelves, in order, with their kinds. */
export const SHELVES: readonly {
  category: Category;
  kinds: CreatableKind[];
}[] = [
  {
    category: "system",
    kinds: ["launcher", "keyboard", "activity", "network"],
  },
  {
    category: "time",
    kinds: [
      "calendar",
      "clock",
      "world-clock",
      "countdown",
      "pomodoro",
      "time-progress",
    ],
  },
  {
    category: "productivity",
    kinds: ["todo", "note", "files", "water", "claude", "weather"],
  },
  { category: "media", kinds: ["media", "screenshots"] },
  { category: "personal", kinds: ["cycle"] },
];

/** Every kind of the catalog, in the order of its shelves: the search of
 * Configurações finds each one by its name ("Claude" finds Uso do Claude). */
export const CATALOG_KINDS: readonly CreatableKind[] = SHELVES.flatMap(
  (shelf) => shelf.kinds,
);

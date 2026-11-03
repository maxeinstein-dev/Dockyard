/* Why a folder of the `Skins` folder is not a skin (spec 0020, Q14). Apart
 * from `skins.ts` so the texts can name them without the miniatures' paints
 * and their images (the e2e suite's typecheck reads the texts). */

/** Why a folder of the skins folder is not a skin; must match `ProblemKind`
 * in `src-tauri/src/domain/skin.rs`. */
export type SkinProblemKind =
  | "badName"
  | "noJson"
  | "badJson"
  | "badImageName"
  | "noImage"
  | "tooBig"
  | "notPng"
  | "sizeMismatch"
  | "badSlice";

export const SKIN_PROBLEM_KINDS = Object.keys({
  badName: 0,
  noJson: 0,
  badJson: 0,
  badImageName: 0,
  noImage: 0,
  tooBig: 0,
  notPng: 0,
  sizeMismatch: 0,
  badSlice: 0,
} satisfies Record<SkinProblemKind, 0>) as SkinProblemKind[];

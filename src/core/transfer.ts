import { invoke } from "@tauri-apps/api/core";

/* Exporting and importing the dock (spec 0026). Rust shows the Windows
 * dialogs and reads and writes the files: the page never names a path. */

/** What a copy holds: `Summary` in `src-tauri/src/persistence/transfer.rs`. */
export interface CopySummary {
  app: string;
  /** RFC 3339. */
  exportedAt: string;
  workspaces: number;
  apps: number;
  widgets: number;
  icons: number;
}

/** Why a copy was refused: `TransferError` in Rust. */
export type TransferErrorCode =
  "tooBig" | "notACopy" | "newer" | "damaged" | "failed";

export const TRANSFER_ERROR_CODES = [
  "tooBig",
  "notACopy",
  "newer",
  "damaged",
  "failed",
] as const satisfies readonly TransferErrorCode[];

export function isTransferError(value: unknown): value is TransferErrorCode {
  return TRANSFER_ERROR_CODES.some((code) => code === value);
}

/** "Exportar…": `false` when the dialog was cancelled. */
export function exportCopy(): Promise<boolean> {
  return invoke("export_copy");
}

/** "Importar…": what the file chosen holds, `null` when cancelled. Nothing
 * changes until `confirmImport`. */
export function chooseImport(): Promise<CopySummary | null> {
  return invoke("choose_import");
}

/** Imports the file chosen: the Dockyard starts again with it. */
export function confirmImport(): Promise<void> {
  return invoke("confirm_import");
}

export function cancelImport(): Promise<void> {
  return invoke("cancel_import");
}

import { useState } from "react";
import { errorKind, log } from "../core/log";
import {
  cancelImport,
  chooseImport,
  confirmImport,
  exportCopy,
  isTransferError,
  type CopySummary,
  type TransferErrorCode,
} from "../core/transfer";
import { activeLocale, format, useT } from "../i18n";
import SettingsGroup from "../panel/parts/SettingsGroup";
import "./SettingsCopy.css";

/** What the last action left to say. */
type Said =
  | { kind: "none" }
  | { kind: "busy" }
  | { kind: "exported" }
  | { kind: "chosen"; summary: CopySummary }
  | { kind: "restarting" }
  | { kind: "error"; code: TransferErrorCode };

/** The day of an RFC 3339 time, as the locale writes it. */
function day(when: string, language: string): string {
  const date = new Date(when);
  return Number.isNaN(date.getTime())
    ? when
    : date.toLocaleString(language, { dateStyle: "short", timeStyle: "short" });
}

/**
 * Configurações › Sobre: "Cópia das configurações" (spec 0026). Exportar
 * writes a `.dockyard` file; Importar reads one, says what it holds and,
 * once confirmed, starts the Dockyard again with it. What was there is
 * kept aside by Rust.
 */
function SettingsCopy() {
  const t = useT();
  const text = t.settings.copy;
  const [said, setSaid] = useState<Said>({ kind: "none" });

  function fail(error: unknown) {
    log.warn(`transfer: failed: ${errorKind(error)}`);
    setSaid({
      kind: "error",
      code: isTransferError(error) ? error : "failed",
    });
  }

  const exporting = () => {
    setSaid({ kind: "busy" });
    exportCopy().then(
      (written) => setSaid(written ? { kind: "exported" } : { kind: "none" }),
      fail,
    );
  };

  const importing = () => {
    setSaid({ kind: "busy" });
    chooseImport().then(
      (summary) =>
        setSaid(summary ? { kind: "chosen", summary } : { kind: "none" }),
      fail,
    );
  };

  const confirm = () => {
    setSaid({ kind: "restarting" });
    confirmImport().catch(fail);
  };

  const cancel = () => {
    setSaid({ kind: "none" });
    cancelImport().catch(() => {});
  };

  const busy = said.kind === "busy" || said.kind === "restarting";
  let line: string | null = null;
  if (said.kind === "exported") line = text.exported;
  if (said.kind === "restarting") line = text.restarting;
  if (said.kind === "error") line = text.errors[said.code];

  return (
    <SettingsGroup title={text.title} grouped>
      <p className="copy-text">{text.lead}</p>
      {said.kind === "chosen" ? (
        <div className="copy-chosen">
          <p className="copy-summary">
            {format(text.summary, {
              date: day(said.summary.exportedAt, activeLocale()),
              version: said.summary.app,
              workspaces: said.summary.workspaces,
              apps: said.summary.apps,
              widgets: said.summary.widgets,
              icons: said.summary.icons,
            })}
          </p>
          <p className="copy-text">{text.replaces}</p>
          <div className="copy-actions">
            <button type="button" className="quiet-button" onClick={cancel}>
              {text.cancel}
            </button>
            <button
              type="button"
              className="secondary-button"
              onClick={confirm}
            >
              {text.confirm}
            </button>
          </div>
        </div>
      ) : (
        <div className="copy-actions">
          <button
            type="button"
            className="secondary-button"
            onClick={exporting}
            disabled={busy}
          >
            {text.export}
          </button>
          <button
            type="button"
            className="secondary-button"
            onClick={importing}
            disabled={busy}
          >
            {text.import}
          </button>
        </div>
      )}
      <p
        className={
          said.kind === "error" ? "copy-text copy-error live" : "copy-text live"
        }
        role="status"
      >
        {line}
      </p>
    </SettingsGroup>
  );
}

export default SettingsCopy;

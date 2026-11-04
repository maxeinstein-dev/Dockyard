import { useEffect, useState } from "react";
import { errorKind, log } from "../../core/log";
import {
  dragRecentFile,
  listRecentFiles,
  openRecentFile,
  type RecentFile,
} from "../../core/recentFiles";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import Icon from "../../panel/parts/Icon";
import { ENTRY_ICONS } from "../../panel/parts/entryIcons";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "../system/system.css";
import { useDragOut } from "../system/useDragOut";
import "./files.css";

type Read = RecentFile[] | "loading" | "failed";

/** The main view of an Arquivos panel (F9 of estudo 0002): the files opened
 * last, in any program, newest first, each with the folder it is in. A
 * click opens one with its program; dragging one hands the file itself to
 * where it is dropped, as the Capturas de tela does. */
function FilesPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"files">) {
  const t = useT();
  const text = t.widgets.files;
  const [read, setRead] = useState<Read>("loading");
  const handlers = useDragOut(
    (key) => {
      dragRecentFile(key).catch((error: unknown) => {
        log.warn(`recent files: drag failed: ${errorKind(error)}`);
      });
    },
    (key) => {
      openRecentFile(key).catch((error: unknown) => {
        log.warn(`recent files: open failed: ${errorKind(error)}`);
      });
    },
  );

  useEffect(() => {
    let active = true;
    listRecentFiles().then(
      (files) => {
        if (active) setRead(files);
      },
      (error: unknown) => {
        log.warn(`recent files: list failed: ${errorKind(error)}`);
        if (active) setRead("failed");
      },
    );
    return () => {
      active = false;
    };
  }, []);

  const files = typeof read === "object" ? read : null;
  const status =
    read === "loading"
      ? text.loading
      : read === "failed"
        ? text.failed
        : files && files.length === 0
          ? text.empty
          : null;

  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <p className="sys-hint" role="status">
        {status}
      </p>
      {files && files.length > 0 ? (
        <>
          <ul className="recent-list" aria-label={text.listLabel}>
            {files.map((file) => (
              <li key={file.key}>
                <button
                  type="button"
                  className="recent-item"
                  aria-label={format(text.open, {
                    name: file.name,
                    folder: file.folder,
                  })}
                  {...handlers(file.key)}
                >
                  <Icon name={ENTRY_ICONS[file.kind]} />
                  <span className="recent-name">{file.name}</span>
                  <span className="recent-folder">{file.folder}</span>
                </button>
              </li>
            ))}
          </ul>
          <p className="sys-hint">{text.hint}</p>
        </>
      ) : null}
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default FilesPanel;

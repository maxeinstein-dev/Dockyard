import { useEffect, useRef, useState, type PointerEvent } from "react";
import { errorKind, log } from "../../core/log";
import {
  dragScreenshot,
  listScreenshots,
  openScreenshot,
  openScreenshotsFolder,
  type ScreenshotsList,
} from "../../core/screenshots";
import { format, useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "../system/system.css";
import "./screenshots.css";

/** How far, in pixels, the pointer moves with the button held before a
 * press becomes a drag (Windows' own default). */
const DRAG_DISTANCE = 4;

type Read = ScreenshotsList | "loading" | "failed";

/** The main view of a Capturas de tela panel (F9 of estudo 0002): the
 * newest screenshots as pictures, newest first. A click opens one with its
 * program; dragging one hands the file itself to where it is dropped (a
 * folder, a chat, an e-mail), as File Explorer does. */
function ScreenshotsPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"screenshots">) {
  const t = useT();
  const text = t.widgets.screenshots;
  const [read, setRead] = useState<Read>("loading");
  /** Where the button went down, until it moves far enough to drag. */
  const pressed = useRef<{ key: number; x: number; y: number } | null>(null);
  /** A drag happened: the click that follows it opens nothing. */
  const dragged = useRef(false);

  useEffect(() => {
    let active = true;
    listScreenshots().then(
      (list) => {
        if (active) setRead(list);
      },
      (error: unknown) => {
        log.warn(`screenshots: list failed: ${errorKind(error)}`);
        if (active) setRead("failed");
      },
    );
    return () => {
      active = false;
    };
  }, []);

  function onPointerDown(key: number, event: PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) return;
    dragged.current = false;
    pressed.current = { key, x: event.clientX, y: event.clientY };
  }

  function onPointerMove(event: PointerEvent<HTMLButtonElement>) {
    const start = pressed.current;
    if (start === null || (event.buttons & 1) === 0) return;
    const far =
      Math.abs(event.clientX - start.x) >= DRAG_DISTANCE ||
      Math.abs(event.clientY - start.y) >= DRAG_DISTANCE;
    if (!far) return;
    pressed.current = null;
    dragged.current = true;
    dragScreenshot(start.key).catch((error: unknown) => {
      log.warn(`screenshots: drag failed: ${errorKind(error)}`);
    });
  }

  function open(key: number) {
    if (dragged.current) {
      dragged.current = false;
      return;
    }
    openScreenshot(key).catch((error: unknown) => {
      log.warn(`screenshots: open failed: ${errorKind(error)}`);
    });
  }

  function openFolder() {
    openScreenshotsFolder().catch((error: unknown) => {
      log.warn(`screenshots: folder failed: ${errorKind(error)}`);
    });
  }

  const list = typeof read === "object" ? read : null;
  const status =
    read === "loading"
      ? text.loading
      : read === "failed"
        ? text.failed
        : list && !list.folder
          ? text.noFolder
          : list && list.items.length === 0
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
      {list && list.items.length > 0 ? (
        <>
          <ul className="shots-grid" aria-label={text.listLabel}>
            {list.items.map((shot) => (
              <li key={shot.key}>
                <button
                  type="button"
                  className="shots-item"
                  aria-label={format(text.open, { name: shot.name })}
                  title={shot.name}
                  onPointerDown={(event) => onPointerDown(shot.key, event)}
                  onPointerMove={onPointerMove}
                  onPointerUp={() => {
                    pressed.current = null;
                  }}
                  onDragStart={(event) => event.preventDefault()}
                  onClick={() => open(shot.key)}
                >
                  {shot.picture ? (
                    <img src={shot.picture} alt="" draggable={false} />
                  ) : (
                    <span className="shots-missing" aria-hidden="true" />
                  )}
                </button>
              </li>
            ))}
          </ul>
          <p className="sys-hint">{text.hint}</p>
        </>
      ) : null}
      {list?.folder ? (
        <button type="button" className="secondary-button" onClick={openFolder}>
          {text.openFolder}
        </button>
      ) : null}
      <PanelFooter>{text.footer}</PanelFooter>
    </>
  );
}

export default ScreenshotsPanel;

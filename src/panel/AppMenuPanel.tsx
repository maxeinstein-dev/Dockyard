import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { closeApp } from "../core/openApps";
import { moveInMenu } from "./parts/menu";
import {
  appIconOptions,
  appMenu,
  chooseAppIconImage,
  isAppCommandError,
  launchApp,
  revealApp,
  runAppEntry,
  setAppIcon,
  unpinApp,
  type AppErrorCode,
  type AppIconOptions,
  type AppMenu,
  type PinnedApp,
} from "../core/apps";
import { moveAppWindows, type WorkspacesSnapshot } from "../core/workspaces";
import { format, useT } from "../i18n";
import PanelFooter from "./PanelFooter";
import MoveToWorkspace from "./parts/MoveToWorkspace";
import Icon from "./parts/Icon";
import PanelHeader from "./parts/PanelHeader";
import "./AppMenuPanel.css";

/** Props of the menu of a pinned program (spec 0006 §6): what the Windows
 * taskbar shows on right click, its Jump List, and a few actions of the
 * dock. */
export interface AppMenuPanelProps {
  app: PinnedApp;
  /** How many windows the program has open (spec 0009 D3): with any, the menu
   * offers to close them. */
  openCount?: number;
  /** The workspaces: with two or more, the menu offers to move the open
   * windows to another (spec 0013 C3). */
  workspaces?: WorkspacesSnapshot | null;
  /** An entry or an action was done: the host closes the panel. */
  onDone: () => void;
}

/** What `appMenu` answered for one program: a newer program makes it stale
 * and the menu shows as loading again. */
interface Read {
  id: string;
  menu: AppMenu | "failed";
}

/**
 * The menu of a pinned program, opened from its icon with a right click: its
 * own categories (or its recent items) and its tasks, as the taskbar shows
 * them, then Abrir, Mostrar no Explorador de Arquivos and Remover do dock.
 * Choosing an entry runs it by its key (Rust remembers what each key runs);
 * the page never holds a path.
 */
function AppMenuPanel({
  app,
  openCount = 0,
  workspaces = null,
  onDone,
}: AppMenuPanelProps) {
  const t = useT();
  const text = t.apps.menu;
  const headings = useId();
  const [read, setRead] = useState<Read | null>(null);
  const [error, setError] = useState<AppErrorCode | null>(null);
  const [icons, setIcons] = useState<{
    id: string;
    options: AppIconOptions;
  } | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const iconHeading = useId();

  const menu = read !== null && read.id === app.id ? read.menu : null;

  useEffect(() => {
    let active = true;
    appMenu(app.id).then(
      (answer) => {
        if (active) setRead({ id: app.id, menu: answer });
      },
      () => {
        if (active) setRead({ id: app.id, menu: "failed" });
      },
    );
    return () => {
      active = false;
    };
  }, [app.id]);

  // What "Ícone" offers (spec 0024): without it, the section is left out.
  useEffect(() => {
    let active = true;
    appIconOptions(app.id).then(
      (options) => {
        // An answer without the packs (an older Rust) leaves it out too.
        if (active && Array.isArray(options?.packs)) {
          setIcons({ id: app.id, options });
        }
      },
      () => {},
    );
    return () => {
      active = false;
    };
  }, [app.id]);

  // The first item takes the focus once the menu is there, so the arrow keys
  // work at once; before that, the first action.
  useEffect(() => {
    if (menu === null) return;
    root.current
      ?.querySelector<HTMLButtonElement>('button[role="menuitem"]')
      ?.focus();
  }, [menu]);

  function fail(reason: unknown) {
    setError(isAppCommandError(reason) ? reason.code : "notReady");
  }

  function done(action: Promise<unknown>) {
    setError(null);
    action.then(onDone, fail);
  }

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) =>
    moveInMenu(root.current, event);

  const sections = menu !== null && menu !== "failed" ? menu.sections : [];
  const canReveal = menu !== null && menu !== "failed" && menu.canReveal;

  return (
    <>
      <PanelHeader title={app.name} />
      <div
        ref={root}
        className="app-menu"
        role="menu"
        aria-label={format(text.label, { name: app.name })}
        onKeyDown={onKeyDown}
      >
        {menu === null ? (
          <p className="app-menu-note" role="status">
            {text.loading}
          </p>
        ) : null}
        {menu === "failed" ? (
          <p className="app-menu-note" role="alert">
            {text.failed}
          </p>
        ) : null}
        {sections.map((section, index) => {
          const heading = `${headings}-${index}`;
          return (
            <div
              key={index}
              className="app-menu-section"
              role="group"
              aria-labelledby={heading}
            >
              <h2 id={heading} className="app-menu-heading">
                {section.title ?? text.sections[section.kind]}
              </h2>
              {section.entries.map((entry) => (
                <button
                  key={entry.key}
                  type="button"
                  role="menuitem"
                  className="app-menu-item"
                  onClick={() => done(runAppEntry(app.id, entry.key))}
                >
                  {entry.title}
                </button>
              ))}
            </div>
          );
        })}
        <div className="app-menu-section" role="group">
          <button
            type="button"
            role="menuitem"
            className="app-menu-item"
            onClick={() => done(launchApp(app.id))}
          >
            <Icon name="launch" />
            {text.open}
          </button>
          {canReveal ? (
            <button
              type="button"
              role="menuitem"
              className="app-menu-item"
              onClick={() => done(revealApp(app.id))}
            >
              <Icon name="folder" />
              {text.reveal}
            </button>
          ) : null}
          {openCount > 0 ? (
            <button
              type="button"
              role="menuitem"
              className="app-menu-item"
              onClick={() => done(closeApp(app.id))}
            >
              <Icon name="close" />
              {openCount > 1 ? text.closeWindows : text.closeWindow}
            </button>
          ) : null}
          <button
            type="button"
            role="menuitem"
            className="app-menu-item"
            onClick={() => done(unpinApp(app.id))}
          >
            <Icon name="unpin" />
            {text.remove}
          </button>
        </div>
        {icons !== null && icons.id === app.id ? (
          <div
            className="app-menu-section"
            role="group"
            aria-labelledby={iconHeading}
          >
            <h2 id={iconHeading} className="app-menu-heading">
              {text.icon.title}
            </h2>
            {(
              [
                [null, text.icon.dock],
                [{ kind: "original" }, text.icon.original],
                ...icons.options.packs.map(
                  (pack) =>
                    [{ kind: "pack", pack: pack.id }, pack.name] as const,
                ),
              ] as const
            ).map(([choice, label]) => {
              const current = icons.options.current;
              const checked =
                choice === null
                  ? current === null
                  : current !== null &&
                    current.kind === choice.kind &&
                    (choice.kind !== "pack" ||
                      (current.kind === "pack" &&
                        current.pack === choice.pack));
              return (
                <button
                  key={choice === null ? "dock" : `${choice.kind}:${label}`}
                  type="button"
                  role="menuitemradio"
                  aria-checked={checked}
                  className="app-menu-item"
                  onClick={() => done(setAppIcon(app.id, choice))}
                >
                  {label}
                </button>
              );
            })}
            <button
              type="button"
              role="menuitemradio"
              aria-checked={icons.options.current?.kind === "image"}
              className="app-menu-item"
              onClick={() => {
                // The dialog takes the focus and the panel closes; Rust
                // keeps the image on its own.
                chooseAppIconImage(app.id).catch(() => {});
                onDone();
              }}
            >
              {text.icon.image}
            </button>
          </div>
        ) : null}
        {openCount > 0 ? (
          <MoveToWorkspace
            workspaces={workspaces}
            onMove={(workspace) => done(moveAppWindows(app.id, workspace))}
          />
        ) : null}
      </div>
      <PanelFooter alert={error && t.apps.errors[error]}>
        {text.footer}
      </PanelFooter>
    </>
  );
}

export default AppMenuPanel;

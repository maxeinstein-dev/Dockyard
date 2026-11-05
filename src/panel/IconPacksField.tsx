import { useCallback, useEffect, useState } from "react";
import {
  listIconPacks,
  openIconPacksFolder,
  reloadIconPacks,
  type IconPackInfo,
} from "../core/iconPacks";
import { format, useT } from "../i18n";
import ChoiceCards from "./parts/ChoiceCards";
import { MINI_ITEMS, MiniDock, type GalleryDock } from "./StyleGallery";
import { usePackPreview } from "./usePackPreview";

/** The value of the "Original" option: no id is the empty string. */
const ORIGINAL = "";

/** The dock with the pinned programs in the icons of `pack`: the picture of
 * its card, decorative. */
function PackThumb({
  pack,
  dock,
  appIds,
  generation,
}: {
  pack: string | null;
  dock: GalleryDock | null;
  appIds: readonly string[];
  generation: number;
}) {
  const icons = usePackPreview(pack, appIds, generation, dock?.style ?? null);
  return (
    <MiniDock
      of={dock?.style ?? "native"}
      dock={dock}
      appIds={appIds}
      icons={icons ?? appIds.slice(0, MINI_ITEMS).map(() => null)}
    />
  );
}

/** The icon pack of the dock, chosen among the packs of the packs folder
 * (spec 0007) as a grid of cards like the dock's styles (spec 0014 R1):
 * Original, or one that is there, each with the dock drawn in its icons; a
 * chosen pack that is gone stays as unavailable. The packs are read when the
 * view opens and again with "Recarregar pacotes". */
function IconPacksField({
  selected,
  dock,
  appIds,
  disabled,
  onSelect,
  withActions = true,
  reloads = 0,
}: {
  selected: string | null;
  /** The dock's settings, for the style of the miniatures. */
  dock: GalleryDock | null;
  /** The programs pinned in the workspace in use, in dock order. */
  appIds: readonly string[];
  disabled?: boolean;
  onSelect: (id: string | null) => void;
  /** Shows the folder's buttons under the cards; off in the Configurações
   * app, which puts them in Aparência › Avançado (`PacksFolderRow`, spec
   * 0029 §4). */
  withActions?: boolean;
  /** Counts the reloads asked from outside (`PacksFolderRow`): each new
   * count reads the packs again. */
  reloads?: number;
}) {
  const text = useT().settings.look;
  const [packs, setPacks] = useState<IconPackInfo[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [generation, setGeneration] = useState(0);

  const load = useCallback((read: () => Promise<IconPackInfo[]>) => {
    let active = true;
    read().then(
      (found) => {
        if (!active) return;
        setPacks(found);
        setFailed(false);
        setGeneration((n) => n + 1);
      },
      () => {
        if (active) setFailed(true);
      },
    );
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => load(listIconPacks), [load]);
  useEffect(() => {
    if (reloads > 0) return load(reloadIconPacks);
  }, [load, reloads]);

  const usable = (packs ?? []).filter((pack) => pack.error === null);
  const broken = (packs ?? []).filter((pack) => pack.error !== null);

  const choices = [
    { value: ORIGINAL, label: text.iconPackOriginal },
    ...usable.map((pack) => ({
      value: pack.id,
      label: pack.name,
      hint:
        pack.total > 0
          ? format(text.iconPackCovers, {
              covers: pack.covers,
              total: pack.total,
            })
          : format(text.iconPackCount, { count: pack.iconCount }),
    })),
  ];
  if (selected !== null && !usable.some((pack) => pack.id === selected)) {
    choices.push({
      value: selected,
      label: selected,
      // While the packs are being read the chosen one is not "gone" yet.
      hint: packs === null ? text.iconPackLoading : text.iconPackMissing,
    });
  }

  return (
    <div className="pack-field">
      <ChoiceCards
        label={text.iconPack}
        choices={choices}
        selected={selected ?? ORIGINAL}
        disabled={disabled}
        onSelect={(value) => onSelect(value === ORIGINAL ? null : value)}
        thumb={(value) => (
          <PackThumb
            pack={value === ORIGINAL ? null : value}
            dock={dock}
            appIds={appIds}
            generation={generation}
          />
        )}
        wide
      />
      <p className="look-hint">
        {failed ? text.iconPackFailed : text.iconPackHint}
      </p>
      {broken.length > 0 ? (
        <ul className="pack-problems">
          {broken.map((pack) => (
            <li key={pack.id}>
              {format(text.packProblem, {
                id: pack.id,
                reason: pack.error ? text.packErrors[pack.error] : "",
              })}
            </li>
          ))}
        </ul>
      ) : null}
      {withActions ? (
        <PacksFolderActions onReload={() => load(reloadIconPacks)} />
      ) : null}
    </div>
  );
}

/** The buttons of the packs folder: open it, read it again. */
function PacksFolderActions({ onReload }: { onReload: () => void }) {
  const text = useT().settings.look;
  return (
    <div className="pack-actions">
      <button
        type="button"
        className="secondary-button"
        onClick={() => {
          void openIconPacksFolder().catch(() => {});
        }}
      >
        {text.openPacksFolder}
      </button>
      <button type="button" className="secondary-button" onClick={onReload}>
        {text.reloadPacks}
      </button>
    </div>
  );
}

/** The packs folder as a row of Aparência › Avançado (spec 0029 §4): its
 * name, what to do with it, and its buttons. */
export function PacksFolderRow({ onReload }: { onReload: () => void }) {
  const text = useT().settings.look;
  return (
    <div className="toggle-row folder-row">
      <div className="toggle-text">
        <span className="toggle-label">{text.packsFolder}</span>
        <span className="toggle-hint">{text.packsFolderHint}</span>
      </div>
      <PacksFolderActions onReload={onReload} />
    </div>
  );
}

export default IconPacksField;

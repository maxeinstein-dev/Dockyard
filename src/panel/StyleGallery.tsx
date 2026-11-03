import { ICON_SIZES, useAppIcon } from "../core/apps";
import {
  DOCK_STYLES,
  type DockSettings,
  type DockStyle,
} from "../core/settings";
import { folderSkinPaint, type FolderSkinInfo } from "../core/skins";
import { styleLookOf } from "../dock/look";
import {
  STYLE_PAINTS,
  css,
  fillCss,
  glowCss,
  outlineCss,
  shelfCss,
  type StylePaint,
} from "../dock/paints";
import { useT } from "../i18n";
import { MiniTile } from "./DockItems";
import ChoiceCards from "./parts/ChoiceCards";
import "./StyleGallery.css";

/** How many items a miniature shows. */
export const MINI_ITEMS = 5;

/** The bar of a miniature is this much of the real dock's (a 48 px tile
 * becomes 18 px): the glows shrink with it. */
const MINI_SCALE = 18 / 48;

/** Half the height of the miniature's bar, in px (StyleGallery.css): its
 * corners at 100 % are a pill. */
const MINI_HALF_HEIGHT = 14;

export type GalleryDock = Pick<
  DockSettings,
  | "style"
  | "tweaks"
  | "indicator"
  | "indicatorColor"
  | "iconPack"
  | "customSkin"
>;

/** The value of a folder skin's card: its folder's name after a prefix no
 * style's name has. */
const FOLDER_PREFIX = "folder:";

type StyleGalleryProps = {
  /** The dock's settings shown, or null while they can't be read. */
  dock: GalleryDock | null;
  /** The programs pinned in the workspace in use, in dock order: the
   * miniatures show the first ones. Without any, widget tiles. */
  appIds: readonly string[];
  label: string;
  disabled?: boolean;
  onSelect: (style: DockStyle) => void;
  /** The skins of the `Skins` folder (spec 0020, Q14), each a card after
   * the styles; null while the folder is being read. */
  folderSkins?: readonly FolderSkinInfo[] | null;
  /** A folder skin chosen, by the name of its folder. */
  onSelectFolderSkin?: (id: string) => void;
};

/** A program's icon in a miniature, the letter-less plate while it loads. */
function MiniApp({ appId, variant }: { appId: string; variant: string }) {
  const icon = useAppIcon(appId, ICON_SIZES.list, variant);
  return icon === null ? (
    <span className="mini-app" />
  ) : (
    <img className="mini-app" src={icon} alt="" draggable={false} />
  );
}

/** The dock in `style`, with the user's changes to that style, as a
 * picture: decorative, the card's name says which style it is. `icons`,
 * when given, are the pictures of the programs (an icon pack's), in place of
 * the ones the dock shows now. `paint`, when given, is the bar's in place of
 * the style's (a folder skin's). */
export function MiniDock({
  of: style,
  dock,
  appIds,
  icons,
  paint = STYLE_PAINTS[style],
}: {
  of: DockStyle;
  dock: GalleryDock | null;
  appIds: readonly string[];
  icons?: readonly (string | null)[];
  paint?: StylePaint;
}) {
  const look = styleLookOf(style, dock?.tweaks[style]);
  const outline = look.border ? outlineCss(paint) : null;
  const indicator =
    dock?.indicatorColor ??
    (paint.indicator === null ? null : css(paint.indicator));
  const shown = appIds.slice(0, MINI_ITEMS);

  return (
    <span className="mini-wall" aria-hidden="true">
      <span
        className="mini-bar"
        data-border={look.border && outline === null ? "" : undefined}
        data-outline={outline === null ? undefined : ""}
        data-flat={paint.flat ? "" : undefined}
        data-reflections={look.reflections ? "" : undefined}
        data-skin={paint.skin ? "" : undefined}
        // The style's paint, through the CSSOM (no inline style).
        ref={(bar) => {
          if (!bar) return;
          const set = (name: string, value: string | null) => {
            if (value === null) bar.style.removeProperty(name);
            else bar.style.setProperty(name, value);
          };
          set("--mini-radius", `${(look.corners / 100) * MINI_HALF_HEIGHT}px`);
          set("--mini-fill", fillCss(paint));
          set("--mini-glow", glowCss(paint, MINI_SCALE));
          set("--mini-outline", outline);
          set(
            "--mini-hi",
            paint.hi === "none" ? "transparent" : paint.hi && css(paint.hi),
          );
          set("--mini-indicator", indicator);
          set("--mini-shelf", paint.shelf ? shelfCss() : null);
          // A skin: its image at the bar's height, the ends as they are.
          const skin = paint.skin;
          const k = skin ? (2 * MINI_HALF_HEIGHT) / skin.height : 0;
          set("--mini-skin", skin && `url("${skin.url}")`);
          set(
            "--mini-skin-slice",
            skin && `0 ${skin.right} 0 ${skin.left} fill`,
          );
          set(
            "--mini-skin-width",
            skin && `0 ${skin.right * k}px 0 ${skin.left * k}px`,
          );
          set(
            "--mini-skin-repeat",
            skin && (skin.repeat ? "repeat" : "stretch"),
          );
        }}
      >
        {shown.length > 0
          ? shown.map((appId, index) =>
              icons === undefined ? (
                <MiniApp
                  key={appId}
                  appId={appId}
                  variant={dock?.iconPack ?? ""}
                />
              ) : icons[index] ? (
                <img
                  key={appId}
                  className="mini-app"
                  src={icons[index]}
                  alt=""
                  draggable={false}
                />
              ) : (
                <span key={appId} className="mini-app" />
              ),
            )
          : (["clock", "note", "time-progress"] as const).map((kind) => (
              <MiniTile key={kind} kind={kind} small />
            ))}
        <span
          className="mini-indicator"
          data-shape={dock?.indicator ?? "dot"}
        />
      </span>
    </span>
  );
}

/**
 * The style of the dock as a grid of cards (spec 0014 R1), each with the
 * dock drawn in that style (`ChoiceCards`).
 */
function StyleGallery({
  dock,
  appIds,
  label,
  disabled = false,
  onSelect,
  folderSkins = [],
  onSelectFolderSkin,
}: StyleGalleryProps) {
  const t = useT();
  const text = t.settings.look;
  const skins = new Map((folderSkins ?? []).map((skin) => [skin.id, skin]));
  const chosenSkin =
    dock?.style === "custom" ? (dock.customSkin ?? null) : null;
  const choices: { value: string; label: string; hint?: string }[] = [
    ...DOCK_STYLES.map((style) => ({
      value: style as string,
      label: text.styles[style],
    })),
    ...[...skins.values()].map((skin) => ({
      value: FOLDER_PREFIX + skin.id,
      label: skin.name,
    })),
  ];
  // A skin chosen that is not in the folder (any more) stays, as
  // unavailable: the dock wears Nativo's bar meanwhile.
  if (chosenSkin !== null && !skins.has(chosenSkin)) {
    choices.push({
      value: FOLDER_PREFIX + chosenSkin,
      label: chosenSkin,
      hint: folderSkins === null ? undefined : text.skinMissing,
    });
  }
  const selected =
    dock === null
      ? null
      : dock.style === "custom"
        ? FOLDER_PREFIX + (chosenSkin ?? "")
        : dock.style;
  return (
    <ChoiceCards
      label={label}
      choices={choices}
      selected={selected}
      disabled={disabled}
      onSelect={(value) => {
        if (value.startsWith(FOLDER_PREFIX)) {
          onSelectFolderSkin?.(value.slice(FOLDER_PREFIX.length));
        } else {
          onSelect(value as DockStyle);
        }
      }}
      thumb={(value) => {
        if (!value.startsWith(FOLDER_PREFIX)) {
          return (
            <MiniDock of={value as DockStyle} dock={dock} appIds={appIds} />
          );
        }
        const skin = skins.get(value.slice(FOLDER_PREFIX.length));
        return (
          <MiniDock
            of="custom"
            dock={dock}
            appIds={appIds}
            paint={
              skin
                ? { ...STYLE_PAINTS.custom, skin: folderSkinPaint(skin) }
                : STYLE_PAINTS.custom
            }
          />
        );
      }}
      wide
    />
  );
}

export default StyleGallery;

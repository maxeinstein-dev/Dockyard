import {
  LANGUAGES,
  MAX_AUTO_HIDE_DELAY,
  MAX_PREVIEW_DELAY,
  MAX_AUTO_HIDE_REVEAL,
  POWER_SAVERS,
  SIZE_PRESETS,
  type DockEdge,
  type DockOpacity,
  type MiddleClick,
  type Settings,
  type SizePreset,
  type TaskbarMode,
  type Theme,
} from "../core/settings";
import { MAX_APPS, type AppsSnapshot } from "../core/apps";
import { MAX_WIDGETS, type WidgetsSnapshot } from "../core/widgets";
import { activeLocale, format, useT, type Messages } from "../i18n";
import OptionGroup, { type Option } from "./OptionGroup";
import PanelFooter from "./PanelFooter";
import NavRow from "./parts/NavRow";
import RangeField from "./parts/RangeField";
import ChoiceCards from "./parts/ChoiceCards";
import SettingsGroup, { AdvancedGroup } from "./parts/SettingsGroup";
import { ScreenThumb } from "./ChoiceThumbs";
import ToggleField from "./parts/ToggleField";
import HotkeyField from "./HotkeyField";
import LauncherField, { LauncherHistoryField } from "./LauncherField";
import TrayInDockField from "./TrayInDockField";
import WebKeywordsField from "./WebKeywordsField";
import { useSettingsEditor } from "./useSettingsEditor";

const THEMES: readonly Theme[] = ["system", "light", "dark"];
/** The edges offered: the sides (spec 0028) come with step 8, once the
 * column is whole. */
const EDGES: readonly DockEdge[] = ["bottom", "top"];
const SIZES: readonly SizePreset[] = ["small", "medium", "large"];
const OPACITIES: readonly DockOpacity[] = ["translucent", "solid"];
const TASKBARS: readonly TaskbarMode[] = ["visible", "autohide", "hidden"];
const MIDDLE_CLICKS: readonly MiddleClick[] = ["newInstance", "none"];

/** Milliseconds as seconds, the Brazilian way: `0,3 s`. */
function seconds(ms: number): string {
  return `${(ms / 1000).toLocaleString(activeLocale(), { maximumFractionDigits: 2 })} s`;
}

/** The preset whose icon size is `iconSize`, or null when none is (a size
 * set by hand in the file): then no size option shows as chosen. */
function sizePresetOf(iconSize: number): SizePreset | null {
  return SIZES.find((preset) => SIZE_PRESETS[preset] === iconSize) ?? null;
}

function options<V extends string>(
  values: readonly V[],
  labels: Readonly<Record<V, string>>,
): Option<V>[] {
  return values.map((value) => ({ value, label: labels[value] }));
}

/** The value of the "Itens do dock" row: `3 de 12`, or `Indisponível`
 * while the widgets can't be used or read. */
function itemsValue(t: Messages, widgets: WidgetsSnapshot | null): string {
  if (widgets === null || widgets.storage.state === "unavailable") {
    return t.widgets.items.unavailable;
  }
  return format(t.widgets.items.count, {
    count: widgets.widgets.length,
    max: MAX_WIDGETS,
  });
}

/** The value of the "Apps do dock" row: `3 de 40`, or `Indisponível`
 * while the apps can't be used or read. */
function appsValue(t: Messages, apps: AppsSnapshot | null): string {
  if (apps === null || apps.storage.state === "unavailable") {
    return t.apps.unavailable;
  }
  return format(t.apps.count, { count: apps.apps.length, max: MAX_APPS });
}

type SettingsPanelProps = {
  /** The settings in use (`useSettings`), or null while they can't be
   * read. The only source of the confirmed values. */
  settings: Settings | null;
  /** The widgets and the storage state (`useWidgets`), for the count and
   * the warning dot of the "Itens do dock" row; null (the default) while
   * loading or unreadable: the row says `Indisponível`, unmarked. */
  widgets?: WidgetsSnapshot | null;
  /** Opens Configurações › Itens do dock (`DockItems`): the panel host
   * swaps this view for the subview. Without it there is no row. */
  onOpenItems?: () => void;
  /** The pinned programs (`useApps`), for the count of the "Apps do dock"
   * row; null (the default) while loading or unreadable. */
  apps?: AppsSnapshot | null;
  /** Opens Configurações › Apps do dock (`AppsPanel`). Without it there
   * is no row. */
  onOpenApps?: () => void;
  /** Opens Configurações › Aparência do dock (`LookPanel`). Without it
   * there is no row. */
  onOpenLook?: () => void;
  /** Focuses the "Itens do dock" row on mount: set it when coming back
   * from the subview (its Voltar). */
  focusItemsRow?: boolean;
  /** The same for the "Aparência do dock" row. */
  focusLookRow?: boolean;
  /** The same for the "Apps do dock" row. */
  focusAppsRow?: boolean;
  /** Shows which tray icons the dock shows itself (`TrayInDockField`), with
   * the taskbar hidden: the Configurações app's, which reads the tray. */
  withTrayInDock?: boolean;
  /** Shows the launcher's switch and history (`LauncherField`): the
   * Configurações app's, which may ask about the launcher. */
  withLauncher?: boolean;
  /** The rows in titled cards, as the Configurações app shows them (spec
   * 0014 R1), split among its sections (spec 0029 §2.1): `part` says which
   * section's. */
  grouped?: boolean;
  part?: SettingsPart;
};

/** The sections of the Configurações app made of these rows (spec 0029). */
export type SettingsPart = "general" | "behavior" | "launcher";

/**
 * The Settings panel: one option group per setting, applied and saved on
 * every choice (spec 0002 §2.3 item 4, §2.4 item 3). A choice shows at once
 * while Rust saves it; if Rust refuses, the translated reason shows in the
 * footer and the options go back to the confirmed settings.
 */
function SettingsPanel({
  settings,
  widgets = null,
  apps = null,
  onOpenItems,
  onOpenApps,
  onOpenLook,
  focusItemsRow = false,
  focusLookRow = false,
  focusAppsRow = false,
  withTrayInDock = false,
  withLauncher = false,
  grouped = false,
  part = "general",
}: SettingsPanelProps) {
  const t = useT();
  const { shown, change, error } = useSettingsEditor(settings);
  const theme = shown?.theme ?? null;
  const edge = shown?.dock.edge ?? null;
  const size = shown ? sizePresetOf(shown.dock.iconSize) : null;
  const opacity = shown?.dock.opacity ?? null;
  const taskbar = shown?.dock.taskbar ?? null;

  const groups = t.settings.app.groups;

  /** A switch of `shown.dock`, `fallback` while there are no settings. */
  const dockToggle = (
    key:
      | "everyMonitor"
      | "switchEveryMonitor"
      | "indicatorFollowsWorkspace"
      | "autoHide"
      | "previews"
      | "showOpenApps"
      | "showBadges",
    label: string,
    hint: string,
    fallback: boolean,
  ) => (
    <ToggleField
      label={label}
      hint={hint}
      checked={shown?.dock[key] ?? fallback}
      disabled={shown === null}
      onChange={(value) =>
        change((s) => ({ ...s, dock: { ...s.dock, [key]: value } }))
      }
    />
  );

  // Each field once; the two layouts below put them in their own order.
  const themeField = (
    <OptionGroup
      label={t.settings.groups.theme}
      options={options(THEMES, t.settings.theme)}
      selected={theme}
      onSelect={(value) => change((s) => ({ ...s, theme: value }))}
    />
  );
  const languageField = (
    <OptionGroup
      label={t.settings.groups.language}
      options={options(LANGUAGES, t.settings.language)}
      selected={shown?.language ?? null}
      onSelect={(value) => change((s) => ({ ...s, language: value }))}
    />
  );
  const setEdge = (value: DockEdge) =>
    change((s) => ({ ...s, dock: { ...s.dock, edge: value } }));
  const sizeField = (
    <OptionGroup
      label={t.settings.groups.size}
      options={options(SIZES, t.settings.size)}
      selected={size}
      onSelect={(value) =>
        change((s) => ({
          ...s,
          dock: { ...s.dock, iconSize: SIZE_PRESETS[value] },
        }))
      }
    />
  );
  const opacityField = (
    <OptionGroup
      label={t.settings.groups.opacity}
      options={options(OPACITIES, t.settings.opacity)}
      selected={opacity}
      onSelect={(value) =>
        change((s) => ({ ...s, dock: { ...s.dock, opacity: value } }))
      }
    />
  );
  const setTaskbar = (value: TaskbarMode) =>
    change((s) => ({ ...s, dock: { ...s.dock, taskbar: value } }));
  // The dock holds the tray only with the taskbar hidden altogether.
  const trayInDockField =
    withTrayInDock && shown !== null && taskbar === "hidden" ? (
      <TrayInDockField
        inDock={shown.dock.trayInDock ?? []}
        onChange={(trayInDock) =>
          change((s) => ({ ...s, dock: { ...s.dock, trayInDock } }))
        }
      />
    ) : null;
  const monitorFields = (
    <>
      {dockToggle(
        "everyMonitor",
        t.settings.everyMonitor,
        t.settings.everyMonitorHint,
        true,
      )}
      {dockToggle(
        "switchEveryMonitor",
        t.settings.switchEveryMonitor,
        t.settings.switchEveryMonitorHint,
        false,
      )}
      {dockToggle(
        "indicatorFollowsWorkspace",
        t.settings.indicatorFollowsWorkspace,
        t.settings.indicatorFollowsWorkspaceHint,
        false,
      )}
    </>
  );
  const autoHideFields = (
    <>
      {dockToggle(
        "autoHide",
        t.settings.autoHide,
        t.settings.autoHideHint,
        false,
      )}
      {shown?.dock.autoHide ? (
        <>
          <RangeField
            label={t.settings.autoHideReveal}
            hint={t.settings.autoHideRevealHint}
            value={shown.dock.autoHideReveal}
            min={0}
            max={MAX_AUTO_HIDE_REVEAL}
            step={50}
            display={seconds}
            onCommit={(autoHideReveal) =>
              change((s) => ({ ...s, dock: { ...s.dock, autoHideReveal } }))
            }
          />
          <RangeField
            label={t.settings.autoHideDelay}
            hint={t.settings.autoHideDelayHint}
            value={shown.dock.autoHideDelay}
            min={0}
            max={MAX_AUTO_HIDE_DELAY}
            step={100}
            display={seconds}
            onCommit={(autoHideDelay) =>
              change((s) => ({ ...s, dock: { ...s.dock, autoHideDelay } }))
            }
          />
        </>
      ) : null}
    </>
  );
  const previewFields = (
    <>
      {dockToggle(
        "previews",
        t.settings.previews,
        t.settings.previewsHint,
        true,
      )}
      {shown?.dock.previews ? (
        <>
          <RangeField
            label={t.settings.previewDelay}
            value={shown.dock.previewDelay}
            min={0}
            max={MAX_PREVIEW_DELAY}
            step={50}
            display={seconds}
            onCommit={(previewDelay) =>
              change((s) => ({ ...s, dock: { ...s.dock, previewDelay } }))
            }
          />
          <ToggleField
            label={t.settings.previewsOnlySeveral}
            checked={shown.dock.previewsOnlySeveral}
            onChange={(previewsOnlySeveral) =>
              change((s) => ({
                ...s,
                dock: { ...s.dock, previewsOnlySeveral },
              }))
            }
          />
        </>
      ) : null}
    </>
  );
  const showOpenAppsField = dockToggle(
    "showOpenApps",
    t.settings.showOpenApps,
    t.settings.showOpenAppsHint,
    true,
  );
  const showBadgesField = dockToggle(
    "showBadges",
    t.settings.showBadges,
    t.settings.showBadgesHint,
    true,
  );
  const middleClickField = (
    <OptionGroup
      label={t.settings.groups.middleClick}
      options={options(MIDDLE_CLICKS, t.settings.middleClick)}
      selected={shown?.dock.middleClick ?? null}
      onSelect={(value) =>
        change((s) => ({ ...s, dock: { ...s.dock, middleClick: value } }))
      }
    />
  );
  const powerSaverField = (
    <OptionGroup
      label={t.settings.groups.powerSaver}
      options={options(POWER_SAVERS, t.settings.powerSaver)}
      selected={shown?.powerSaver ?? null}
      onSelect={(value) => change((s) => ({ ...s, powerSaver: value }))}
      hint={t.settings.powerSaverHint}
    />
  );
  const dockHotkeyField =
    withLauncher && shown !== null ? (
      <HotkeyField of="dock" hotkey={shown.dockHotkey} />
    ) : null;
  const launcherField = withLauncher ? (
    <LauncherField
      on={shown?.launcher ?? null}
      hotkey={shown?.launcherHotkey ?? null}
      onChange={(launcher) => change((s) => ({ ...s, launcher }))}
    />
  ) : null;
  const webKeywordsField =
    withLauncher && shown !== null ? (
      <WebKeywordsField
        keywords={shown.launcherKeywords}
        onChange={(launcherKeywords) =>
          change((s) => ({ ...s, launcherKeywords }))
        }
      />
    ) : null;
  const launchAtLoginField = (
    <ToggleField
      label={t.settings.launchAtLogin}
      checked={shown?.launchAtLogin ?? false}
      disabled={shown === null}
      onChange={(launchAtLogin) => change((s) => ({ ...s, launchAtLogin }))}
    />
  );
  // The rows to the subviews and the footer, last in both layouts.
  const ending = (
    <>
      {onOpenLook ? (
        <NavRow
          label={t.settings.look.title}
          value={
            shown === null
              ? ""
              : shown.dock.style === "custom"
                ? // A skin of the `Skins` folder, by the name of its folder.
                  (shown.dock.customSkin ?? t.settings.look.styles.custom)
                : t.settings.look.styles[shown.dock.style]
          }
          onClick={onOpenLook}
          focusOnMount={focusLookRow}
        />
      ) : null}
      {onOpenApps ? (
        <NavRow
          label={t.apps.title}
          value={appsValue(t, apps)}
          warning={apps !== null && apps.storage.state !== "ok"}
          onClick={onOpenApps}
          focusOnMount={focusAppsRow}
        />
      ) : null}
      {onOpenItems ? (
        <NavRow
          label={t.widgets.items.title}
          value={itemsValue(t, widgets)}
          // The notice lives in the subview (spec 0003 §2.4 item 7).
          warning={widgets !== null && widgets.storage.state !== "ok"}
          onClick={onOpenItems}
          focusOnMount={focusItemsRow}
        />
      ) : null}
      <PanelFooter alert={error && t.settings.errors[error]}>
        {t.settings.footer}
      </PanelFooter>
    </>
  );

  if (grouped && part === "behavior") {
    return (
      <>
        <SettingsGroup title={groups.showHide} grouped>
          {autoHideFields}
        </SettingsGroup>
        <SettingsGroup title={groups.previews} grouped>
          {previewFields}
        </SettingsGroup>
        <SettingsGroup title={groups.appsInDock} grouped>
          {showOpenAppsField}
          {showBadgesField}
        </SettingsGroup>
        <SettingsGroup title={groups.energy} grouped>
          {powerSaverField}
        </SettingsGroup>
        {/* Until the Monitores section (F13) takes them. */}
        <SettingsGroup title={groups.monitors} grouped>
          {dockToggle(
            "everyMonitor",
            t.settings.everyMonitor,
            t.settings.everyMonitorHint,
            true,
          )}
          {dockToggle(
            "switchEveryMonitor",
            t.settings.switchEveryMonitor,
            t.settings.switchEveryMonitorHint,
            false,
          )}
        </SettingsGroup>
        <AdvancedGroup
          id="behavior"
          title={groups.advanced}
          summary={t.settings.app.advancedSummary.behavior}
        >
          {middleClickField}
          {dockHotkeyField}
        </AdvancedGroup>
        {ending}
      </>
    );
  }

  if (grouped && part === "launcher") {
    return (
      <>
        <SettingsGroup title={groups.launcher} grouped>
          <LauncherField
            on={shown?.launcher ?? null}
            hotkey={shown?.launcherHotkey ?? null}
            onChange={(launcher) => change((s) => ({ ...s, launcher }))}
            withHistory={false}
          />
        </SettingsGroup>
        <SettingsGroup title={t.settings.launcher.keywords.label} grouped>
          {webKeywordsField}
        </SettingsGroup>
        <AdvancedGroup
          id="launcher"
          title={groups.advanced}
          summary={t.settings.app.advancedSummary.launcher}
        >
          <LauncherHistoryField />
        </AdvancedGroup>
        {ending}
      </>
    );
  }

  if (grouped) {
    return (
      <>
        <SettingsGroup title={groups.dock} grouped>
          {themeField}
          <ChoiceCards
            label={t.settings.groups.edge}
            choices={options(EDGES, t.settings.edge)}
            selected={edge}
            disabled={shown === null}
            onSelect={setEdge}
            thumb={(value) => <ScreenThumb edge={value} />}
          />
          {sizeField}
          {opacityField}
        </SettingsGroup>
        <SettingsGroup title={groups.taskbar} grouped>
          <ChoiceCards
            label={t.settings.groups.taskbar}
            choices={TASKBARS.map((value) => ({
              value,
              label: t.settings.taskbar[value],
              hint: t.settings.taskbarHints[value],
            }))}
            selected={taskbar}
            disabled={shown === null}
            onSelect={setTaskbar}
            thumb={(value) => (
              <ScreenThumb edge={edge ?? "bottom"} taskbar={value} />
            )}
          />
          {trayInDockField}
        </SettingsGroup>
        <SettingsGroup title={groups.startup} grouped>
          {languageField}
          {launchAtLoginField}
        </SettingsGroup>
        {ending}
      </>
    );
  }

  return (
    <>
      {themeField}
      {languageField}
      <OptionGroup
        label={t.settings.groups.edge}
        options={options(EDGES, t.settings.edge)}
        selected={edge}
        onSelect={setEdge}
      />
      {sizeField}
      {opacityField}
      <OptionGroup
        label={t.settings.groups.taskbar}
        options={options(TASKBARS, t.settings.taskbar)}
        selected={taskbar}
        onSelect={setTaskbar}
      />
      {trayInDockField}
      {monitorFields}
      {autoHideFields}
      {previewFields}
      {showOpenAppsField}
      {showBadgesField}
      {launchAtLoginField}
      {launcherField}
      {webKeywordsField}
      {middleClickField}
      {powerSaverField}
      {dockHotkeyField}
      {ending}
    </>
  );
}

export default SettingsPanel;

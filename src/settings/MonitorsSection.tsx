import { useEffect, useState } from "react";
import type { Settings } from "../core/settings";
import {
  listMonitors,
  setMonitorFixed,
  setMonitorWorkspace,
  type MonitorView,
  type WorkspacesSnapshot,
} from "../core/workspaces";
import { format, useT } from "../i18n";
import SelectField from "../panel/parts/SelectField";
import SettingsGroup from "../panel/parts/SettingsGroup";
import ToggleField from "../panel/parts/ToggleField";
import { useSettingsEditor } from "../panel/useSettingsEditor";
import "./MonitorsSection.css";

/**
 * Configurações › Espaços › Monitores (spec 0030 E): each monitor with the
 * workspace it shows and whether it is held on it. Read again whenever the
 * workspaces change (`snapshot`), a switch from the dock included.
 */
function MonitorsSection({
  snapshot,
  settings,
}: {
  snapshot: WorkspacesSnapshot | null;
  /** The settings in use, for "Trocar de espaço em todos os monitores" at
   * the top of the group (spec 0029 §2.1); without them, no switch. */
  settings?: Settings | null;
}) {
  const t = useT();
  const text = t.workspaces.monitors;
  const { shown, change } = useSettingsEditor(settings ?? null);
  const [monitors, setMonitors] = useState<MonitorView[] | null>(null);

  useEffect(() => {
    let active = true;
    listMonitors().then(
      (found) => {
        if (active && Array.isArray(found)) setMonitors(found);
      },
      () => {},
    );
    return () => {
      active = false;
    };
  }, [snapshot]);

  const options = (snapshot?.workspaces ?? []).map((workspace) => ({
    value: workspace.id,
    label: workspace.name,
  }));

  return (
    <SettingsGroup title={text.title} grouped>
      {settings !== undefined ? (
        <ToggleField
          label={t.settings.switchEveryMonitor}
          hint={t.settings.switchEveryMonitorHint}
          checked={shown?.dock.switchEveryMonitor ?? false}
          disabled={shown === null}
          onChange={(switchEveryMonitor) =>
            change((s) => ({ ...s, dock: { ...s.dock, switchEveryMonitor } }))
          }
        />
      ) : null}
      {monitors === null ? (
        <p className="monitors-note">{text.loading}</p>
      ) : (
        monitors.map((monitor) => {
          const name = monitor.primary
            ? format(text.primary, { name: monitor.name })
            : monitor.name;
          return (
            <div key={monitor.key} className="monitors-row">
              <SelectField
                label={format(text.workspace, { name })}
                value={monitor.workspace ?? ""}
                options={options}
                onChange={(workspace) => {
                  setMonitorWorkspace(monitor.key, workspace).catch(() => {});
                }}
              />
              <ToggleField
                label={format(text.fixed, { name })}
                hint={text.fixedHint}
                checked={monitor.fixed}
                onChange={(fixed) => {
                  setMonitorFixed(monitor.key, fixed).catch(() => {});
                }}
              />
            </div>
          );
        })
      )}
    </SettingsGroup>
  );
}

export default MonitorsSection;

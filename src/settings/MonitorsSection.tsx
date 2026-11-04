import { useEffect, useState } from "react";
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
import "./MonitorsSection.css";

/**
 * Configurações › Espaços › Monitores (spec 0028 E): each monitor with the
 * workspace it shows and whether it is held on it. Read again whenever the
 * workspaces change (`snapshot`), a switch from the dock included.
 */
function MonitorsSection({
  snapshot,
}: {
  snapshot: WorkspacesSnapshot | null;
}) {
  const t = useT();
  const text = t.workspaces.monitors;
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

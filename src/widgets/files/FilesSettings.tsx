import { useT } from "../../i18n";
import PanelFooter from "../../panel/PanelFooter";
import PanelHeader from "../../panel/parts/PanelHeader";
import RemoveButton from "../../panel/parts/RemoveButton";
import TextField from "../../panel/parts/TextField";
import type { WidgetSettingsProps } from "../contract";
import { useSystemSettings } from "../system/useSettings";

/** The "Ajustes" view of an Arquivos panel: label and "Remover do
 * dock". The list is Windows' own. */
function FilesSettings({
  instance,
  onUpdate,
  onRemove,
  onBack,
}: WidgetSettingsProps<"files">) {
  const t = useT();
  const settings = useSystemSettings({ instance, onUpdate, onRemove });
  return (
    <>
      <PanelHeader
        title={t.widgets.files.settingsTitle}
        onBack={onBack}
        focusAction
      />
      <TextField
        label={t.widgets.common.label}
        placeholder={t.widgets.kinds.files}
        value={settings.labelDraft}
        invalid={settings.error === "invalidLabel"}
        onChange={settings.setLabelDraft}
        onCommit={settings.commitLabel}
      />
      <p className="sys-hint">{t.widgets.files.hint}</p>
      <RemoveButton
        onClick={() => void settings.remove()}
        disabled={settings.removing}
      />
      <PanelFooter
        alert={settings.error ? t.widgets.errors[settings.error] : null}
      >
        {t.widgets.common.footer}
      </PanelFooter>
    </>
  );
}

export default FilesSettings;

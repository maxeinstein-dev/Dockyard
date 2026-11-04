import { useT } from "../../i18n";
import PanelHeader from "../../panel/parts/PanelHeader";
import type { WidgetPanelProps } from "../contract";
import { widgetName } from "../names";
import "../system/system.css";

/** The main view of a Layout do teclado panel (F9 of estudo 0002). A click
 * on the dock switches the layout, never opens this panel: it only says how
 * the item works,
 * should it be opened some other way. */
function KeyboardPanel({
  instance,
  onAdjust,
  returnedFromSettings,
}: WidgetPanelProps<"keyboard">) {
  const t = useT();
  return (
    <>
      <PanelHeader
        title={widgetName(t, instance)}
        onAdjust={onAdjust}
        focusAction={returnedFromSettings}
      />
      <p className="sys-hint">{t.widgets.keyboard.hint}</p>
    </>
  );
}

export default KeyboardPanel;

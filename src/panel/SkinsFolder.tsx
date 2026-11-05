import { openSkinsFolder } from "../core/skins";
import { format, useT } from "../i18n";
import type { SkinsFolderState } from "./useSkinsFolder";

/** The button that opens the `Skins` folder. */
function OpenSkinsFolder() {
  const text = useT().settings.look;
  return (
    <div className="pack-actions">
      <button
        type="button"
        className="secondary-button"
        onClick={() => {
          void openSkinsFolder().catch(() => {});
        }}
      >
        {text.openSkinsFolder}
      </button>
    </div>
  );
}

/** The `Skins` folder as a row of Aparência › Avançado (spec 0029 §4): what
 * it is for, and its button. */
export function SkinsFolderRow() {
  const text = useT().settings.look;
  return (
    <div className="toggle-row folder-row">
      <div className="toggle-text">
        <span className="toggle-label">{text.skinsFolder}</span>
        <span className="toggle-hint">{text.skinsHint}</span>
      </div>
      <OpenSkinsFolder />
    </div>
  );
}

/** Under the styles: what the `Skins` folder is for, each folder refused
 * and why, and the button that opens it. In the Configurações app
 * (`withFolder` off) only what went wrong stays here; the rest is in
 * Avançado, and errors are never hidden (spec 0029 §4). */
function SkinsFolderNotice({
  problems,
  failed,
  withFolder = true,
}: Pick<SkinsFolderState, "problems" | "failed"> & { withFolder?: boolean }) {
  const text = useT().settings.look;
  if (!withFolder && !failed && problems.length === 0) return null;
  return (
    <div className="pack-field">
      {failed || withFolder ? (
        <p className="look-hint">
          {failed ? text.skinsFailed : text.skinsHint}
        </p>
      ) : null}
      {problems.length > 0 ? (
        <ul className="pack-problems">
          {problems.map((problem) => (
            <li key={problem.folder}>
              {format(text.skinProblem, {
                folder: problem.folder,
                reason: text.skinProblems[problem.kind],
              })}
            </li>
          ))}
        </ul>
      ) : null}
      {withFolder ? <OpenSkinsFolder /> : null}
    </div>
  );
}

export default SkinsFolderNotice;

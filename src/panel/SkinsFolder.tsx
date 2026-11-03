import { openSkinsFolder } from "../core/skins";
import { format, useT } from "../i18n";
import type { SkinsFolderState } from "./useSkinsFolder";

/** Under the styles: what the `Skins` folder is for, each folder refused
 * and why, and the button that opens it. */
function SkinsFolderNotice({
  problems,
  failed,
}: Pick<SkinsFolderState, "problems" | "failed">) {
  const text = useT().settings.look;
  return (
    <div className="pack-field">
      <p className="look-hint">{failed ? text.skinsFailed : text.skinsHint}</p>
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
    </div>
  );
}

export default SkinsFolderNotice;

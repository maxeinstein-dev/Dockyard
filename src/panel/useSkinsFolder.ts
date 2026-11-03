import { useEffect, useState } from "react";
import {
  listFolderSkins,
  type FolderSkinInfo,
  type SkinProblem,
} from "../core/skins";

/** The `Skins` folder as last read (spec 0020, Q14): its skins (null while
 * it is being read), the folders refused, and whether it could not be read
 * at all. */
export interface SkinsFolderState {
  skins: FolderSkinInfo[] | null;
  problems: SkinProblem[];
  failed: boolean;
}

/** Reads the `Skins` folder when the view opens: a skin put there shows
 * without restarting the app. */
export function useSkinsFolder(): SkinsFolderState {
  const [state, setState] = useState<SkinsFolderState>({
    skins: null,
    problems: [],
    failed: false,
  });
  useEffect(() => {
    let active = true;
    listFolderSkins().then(
      (found) => {
        if (active) {
          setState({
            skins: found.skins,
            problems: found.problems,
            failed: false,
          });
        }
      },
      () => {
        if (active) setState({ skins: [], problems: [], failed: true });
      },
    );
    return () => {
      active = false;
    };
  }, []);
  return state;
}

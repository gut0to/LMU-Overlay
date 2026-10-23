import { useEffect } from "react";
import { createDraft } from "./drafts.ts";
import type { DraftStore } from "./draft-storage.ts";

/** Recovery choices retain ownership of the stored draft until resolved. */
export function useDraftPersistence(
  config: object | null,
  path: string,
  baseline: string,
  enabled: boolean,
  store: DraftStore,
  report: (error: string | null) => void,
) {
  useEffect(() => {
    if (!enabled || !config) return;
    if (JSON.stringify(config) === baseline) {
      report(store.clear());
      return;
    }
    try {
      report(store.write(createDraft(path, baseline, config)));
    } catch (error) {
      report(String(error));
    }
  }, [config, path, baseline, enabled, store, report]);
}

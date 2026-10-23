import { DRAFT_STORAGE_KEY } from "./drafts.ts";

type DraftStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

/** Browser storage may fail at access time or during any operation. */
export class DraftStore {
  constructor(private readonly storage: () => DraftStorage) {}

  read(): { raw: string | null; error: string | null } {
    try { return { raw: this.storage().getItem(DRAFT_STORAGE_KEY), error: null }; }
    catch { return { raw: null, error: "Local recovery is unavailable. Save or export your changes before closing Settings." }; }
  }

  write(raw: string): string | null {
    try { this.storage().setItem(DRAFT_STORAGE_KEY, raw); return null; }
    catch { return "Could not keep a local recovery draft. Save or export your changes before closing Settings."; }
  }

  clear(): string | null {
    try { this.storage().removeItem(DRAFT_STORAGE_KEY); return null; }
    catch { return "Could not clear the local recovery draft. It may be offered again when Settings reopens."; }
  }
}

import { isImportScope, type ImportScope } from "./import-scopes.ts";
import type { OverlayConfig } from "./types";

export type ScopedImportRequest = { base: OverlayConfig; text: string; scope: ImportScope };
export function importRequestKey(base: OverlayConfig | null, text: string, scope: ImportScope): string {
  return base ? JSON.stringify({ base, text, scope }) : "";
}
export function readImportRequest(key: string): ScopedImportRequest {
  const value = JSON.parse(key);
  if (!value || typeof value.text !== "string" || !isImportScope(value.scope) || !value.base || typeof value.base !== "object" || Array.isArray(value.base)) throw new Error("Import review is no longer valid. Preview it again.");
  return { base: value.base, text: value.text, scope: value.scope };
}

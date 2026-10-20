/** A completed write may normalize its own request, never overwrite newer edits. */
export function acceptSavedConfig<T>(current: T | null, requested: T, saved: T): T | null {
  return JSON.stringify(current) === JSON.stringify(requested) ? saved : current;
}

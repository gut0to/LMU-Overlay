/** Disk persistence and live reload are independent outcomes. */
export function saveFeedback(success: string, reloadWarning?: string | null) {
  return { message: reloadWarning || success, liveApplied: !reloadWarning };
}

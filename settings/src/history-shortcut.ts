type ShortcutEvent = { key: string; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean; repeat: boolean; isComposing: boolean };
export function historyShortcut(event: ShortcutEvent, editingText: boolean): "undo" | "redo" | null {
  if (editingText || event.altKey || event.repeat || event.isComposing || !(event.ctrlKey || event.metaKey)) return null;
  const key = event.key.toLowerCase();
  if (key === "z") return event.shiftKey ? "redo" : "undo";
  if (key === "y" && event.ctrlKey && !event.shiftKey) return "redo";
  return null;
}

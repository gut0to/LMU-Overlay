export type KeyCapture = {
  key: string; ctrlKey: boolean; shiftKey: boolean; altKey: boolean;
  metaKey: boolean; repeat: boolean; isComposing: boolean;
};

/** Only function keys supported by the host can be persisted as bindings. */
export function captureHotkey(event: KeyCapture): string | null {
  if (event.repeat || event.metaKey || event.isComposing || !/^F([1-9]|1[0-2])$/.test(event.key)) return null;
  return [event.ctrlKey && "Ctrl", event.shiftKey && "Shift", event.altKey && "Alt", event.key].filter(Boolean).join("+");
}

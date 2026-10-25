type LayoutKey = {
  key: string;
  shiftKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  isComposing?: boolean;
};

/** Arrow keys belong to layout editing only while the preview owns focus. */
export function layoutNudge(
  event: LayoutKey,
): { dx: number; dy: number } | null {
  if (event.ctrlKey || event.altKey || event.metaKey || event.isComposing)
    return null;
  const step = event.shiftKey ? 10 : 1;
  switch (event.key) {
    case "ArrowLeft":
      return { dx: -step, dy: 0 };
    case "ArrowRight":
      return { dx: step, dy: 0 };
    case "ArrowUp":
      return { dx: 0, dy: -step };
    case "ArrowDown":
      return { dx: 0, dy: step };
    default:
      return null;
  }
}

export type HostState = {
  state: "checking" | "running" | "stopped" | "unreachable";
  visible: boolean;
  editing: boolean;
  error: string | null;
};
export const initialHostState: HostState = {
  state: "checking",
  visible: false,
  editing: false,
  error: null,
};

export function projectHostState(value: unknown): HostState {
  const unavailable = (message: string): HostState => ({
    state: "unreachable",
    visible: false,
    editing: false,
    error: message,
  });
  if (!value || typeof value !== "object")
    return unavailable(
      "Invalid overlay host state. Refresh or restart the overlay.",
    );
  const raw = value as Record<string, unknown>;
  if (raw.state === "stopped")
    return { state: "stopped", visible: false, editing: false, error: null };
  if (raw.state === "unreachable")
    return unavailable(
      typeof raw.error === "string" && raw.error.trim()
        ? raw.error
        : "Overlay host is not responding. Restart it and retry.",
    );
  if (
    raw.state !== "running" ||
    typeof raw.visible !== "boolean" ||
    typeof raw.editing !== "boolean"
  )
    return unavailable(
      "Invalid overlay host state. Refresh or restart the overlay.",
    );
  return {
    state: "running",
    visible: raw.visible,
    editing: raw.editing,
    error: null,
  };
}

import { useState } from "react";
import { displayWorkspace, matchesDisplay, type Display } from "./displays";
import type { WorkspaceSize } from "./workspace";

export function DisplayWorkspacePicker(props: { displays: Display[]; workspace: WorkspaceSize; onChange: (workspace: WorkspaceSize) => void; loading: boolean; error: string | null; onRefresh: () => void }) {
  const [workArea, setWorkArea] = useState(false);
  const selected = props.displays.find((display) => matchesDisplay(props.workspace, display, workArea));
  return <div className="displayWorkspacePicker">
    <label className="referenceField">Connected display<select value={selected?.id ?? ""} onChange={(event) => {
      const display = props.displays.find((display) => display.id === event.target.value);
      if (display) props.onChange(displayWorkspace(display, workArea));
    }}>
      <option value="">Custom workspace</option>
      {props.displays.map((display) => <option key={display.id} value={display.id}>{display.name}{display.primary ? " (Primary)" : ""} — {display.bounds.width} × {display.bounds.height}</option>)}
    </select></label>
    <label className="toggle"><input type="checkbox" checked={workArea} onChange={(event) => {
      setWorkArea(event.target.checked);
      if (selected) props.onChange(displayWorkspace(selected, event.target.checked));
    }} /><span>Use work area, excluding taskbars</span></label>
    <button className="secondaryButton" disabled={props.loading} onClick={props.onRefresh}>{props.loading ? "Detecting displays…" : "Refresh connected displays"}</button>
    {props.error && <p className="inlineError" role="status">{props.error}</p>}
    {!props.loading && !props.error && props.displays.length === 0 && <p className="previewHint">No displays were reported. Choose a workspace preset or enter the desktop size manually.</p>}
    {selected && <p className="previewHint">Screen origin: {props.workspace.originX}, {props.workspace.originY}. Choosing a display changes only the editor workspace.</p>}
  </div>;
}

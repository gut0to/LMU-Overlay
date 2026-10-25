import { useState } from "react";
import { alignWidgetToReference, type WidgetAlignment } from "./layout-tools";
import type { LayoutSelection, WidgetLayout, WindowConfig } from "./types";

type LayoutEntry = { key: LayoutSelection; label: string; layout: WidgetLayout };
type WidgetToolsProps = {
  selected: LayoutSelection;
  entries: LayoutEntry[];
  window: WindowConfig;
  locked: boolean;
  onReplace: (layout: WidgetLayout) => void;
};

export function WidgetTools(props: WidgetToolsProps) {
  const [referenceKey, setReferenceKey] = useState("");
  const selected = props.entries.find((entry) => entry.key === props.selected);
  const references = props.entries.filter((entry) => entry.key !== props.selected);
  const reference = references.find((entry) => entry.key === referenceKey);
  const alignments: Array<[WidgetAlignment, string]> = [["left", "Left edges"], ["center-x", "Horizontal centers"], ["right", "Right edges"], ["top", "Top edges"], ["center-y", "Vertical centers"], ["bottom", "Bottom edges"]];
  if (!selected) return null;
  return <fieldset className="widgetTools" disabled={props.locked || selected.layout.locked}>
    <legend>Align with another widget</legend>
    <label className="referenceField">Reference widget
      <select value={reference?.key ?? ""} onChange={(event) => setReferenceKey(event.target.value)}>
        <option value="">Choose a visible widget</option>
        {references.map((entry) => <option key={entry.key} value={entry.key}>{entry.label}</option>)}
      </select>
    </label>
    <div className="buttonRow">{alignments.map(([alignment, label]) => <button key={alignment} className="secondaryButton" disabled={!reference} onClick={() => {
      if (reference) props.onReplace(alignWidgetToReference(props.window, selected.layout, reference.layout, alignment));
    }}>{label}</button>)}</div>
    {references.length === 0 && <p className="previewHint">Enable a second widget on this surface to align their edges or centers.</p>}
  </fieldset>;
}

import {
  copyWidgetLayout,
  pasteWidgetLayout,
  type LayoutClipboard,
  type LayoutPaste,
} from "./layout-clipboard";
import { useState } from "react";
import {
  alignWidgetToReference,
  nudgeWidget,
  moveWidgetInStack,
  type WidgetAlignment,
} from "./layout-tools";
import type { LayoutSelection, WidgetLayout, WindowConfig } from "./types";

type LayoutEntry = {
  key: LayoutSelection;
  label: string;
  layout: WidgetLayout;
};
type WidgetToolsProps = {
  selected: LayoutSelection;
  entries: LayoutEntry[];
  window: WindowConfig;
  locked: boolean;
  onReplace: (layout: WidgetLayout) => void;
  clipboard: LayoutClipboard | null;
  onCopy: (clipboard: LayoutClipboard) => void;
};

export function WidgetTools(props: WidgetToolsProps) {
  const [nudgeStep, setNudgeStep] = useState(1);
  const [referenceKey, setReferenceKey] = useState("");
  const selected = props.entries.find((entry) => entry.key === props.selected);
  const references = props.entries.filter(
    (entry) => entry.key !== props.selected,
  );
  const reference = references.find((entry) => entry.key === referenceKey);
  const alignments: Array<[WidgetAlignment, string]> = [
    ["left", "Left edges"],
    ["center-x", "Horizontal centers"],
    ["right", "Right edges"],
    ["top", "Top edges"],
    ["center-y", "Vertical centers"],
    ["bottom", "Bottom edges"],
  ];
  if (!selected) return null;
  return (
    <details className="widgetToolsGroup">
      <summary>Arrange and copy layout</summary>
      <button
        className="secondaryButton"
        onClick={() =>
          props.onCopy(copyWidgetLayout(selected.label, selected.layout))
        }
      >
        Copy layout
      </button>
      <fieldset
        className="widgetTools"
        disabled={props.locked || selected.layout.locked}
      >
        <legend>Widget placement</legend>
        <div className="buttonRow">
          {(
            [
              ["position", "Paste position"],
              ["size", "Paste size"],
              ["geometry", "Paste full geometry"],
            ] as Array<[LayoutPaste, string]>
          ).map(([mode, label]) => (
            <button
              key={mode}
              className="secondaryButton"
              disabled={!props.clipboard}
              onClick={() => {
                if (props.clipboard)
                  props.onReplace(
                    pasteWidgetLayout(
                      props.window,
                      selected.layout,
                      props.clipboard,
                      mode,
                    ),
                  );
              }}
            >
              {label}
            </button>
          ))}
        </div>
        {props.clipboard && (
          <p className="previewHint" role="status">
            Layout copied from {props.clipboard.source}. Choose a widget on any
            surface to paste.
          </p>
        )}
        <label className="referenceField">
          Nudge distance
          <select
            value={nudgeStep}
            onChange={(event) => setNudgeStep(Number(event.target.value))}
          >
            <option value={1}>1 pixel</option>
            <option value={10}>10 pixels</option>
          </select>
        </label>
        <div
          className="buttonRow"
          role="group"
          aria-label="Move selected widget"
        >
          {[
            ["Left", -1, 0],
            ["Right", 1, 0],
            ["Up", 0, -1],
            ["Down", 0, 1],
          ].map(([label, dx, dy]) => (
            <button
              key={label}
              className="secondaryButton"
              onClick={() =>
                props.onReplace(
                  nudgeWidget(
                    props.window,
                    selected.layout,
                    Number(dx) * nudgeStep,
                    Number(dy) * nudgeStep,
                  ),
                )
              }
            >
              {label}
            </button>
          ))}
        </div>
        <p className="previewHint">
          Arrow keys move a focused preview widget by 1 pixel; Shift moves 10
          pixels. Precision moves ignore snapping.
        </p>
        <div className="buttonRow">
          <button
            className="secondaryButton"
            onClick={() =>
              props.onReplace(
                moveWidgetInStack(
                  selected.layout,
                  references.map((entry) => entry.layout),
                  "front",
                ),
              )
            }
          >
            Bring to front
          </button>
          <button
            className="secondaryButton"
            onClick={() =>
              props.onReplace(
                moveWidgetInStack(
                  selected.layout,
                  references.map((entry) => entry.layout),
                  "back",
                ),
              )
            }
          >
            Send to back
          </button>
        </div>
        <label className="referenceField">
          Reference widget
          <select
            value={reference?.key ?? ""}
            onChange={(event) => setReferenceKey(event.target.value)}
          >
            <option value="">Choose a visible widget</option>
            {references.map((entry) => (
              <option key={entry.key} value={entry.key}>
                {entry.label}
              </option>
            ))}
          </select>
        </label>
        <div className="buttonRow">
          {alignments.map(([alignment, label]) => (
            <button
              key={alignment}
              className="secondaryButton"
              disabled={!reference}
              onClick={() => {
                if (reference)
                  props.onReplace(
                    alignWidgetToReference(
                      props.window,
                      selected.layout,
                      reference.layout,
                      alignment,
                    ),
                  );
              }}
            >
              {label}
            </button>
          ))}
        </div>
        <button
          className="secondaryButton"
          disabled={!reference}
          onClick={() => {
            if (reference)
              props.onReplace(
                pasteWidgetLayout(
                  props.window,
                  selected.layout,
                  copyWidgetLayout(reference.label, reference.layout),
                  "size",
                ),
              );
          }}
        >
          Match reference size
        </button>
        {references.length === 0 && (
          <p className="previewHint">
            Enable a second widget on this surface to align their edges or
            centers.
          </p>
        )}
      </fieldset>
    </details>
  );
}

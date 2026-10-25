import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { useConfigHistory } from "../src/useConfigHistory.ts";
import { updateOverlayLayoutSelection } from "../src/layout-state.ts";
import {
  copyWidgetLayout,
  pasteWidgetLayout,
} from "../src/layout-clipboard.ts";
import { nudgeWidget } from "../src/layout-tools.ts";

test("held nudges and cross-surface pastes remain independently undoable in React", () => {
  let current;
  function Editor() {
    current = useConfigHistory();
    return null;
  }
  const layout = {
    x: 10,
    y: 20,
    width: 100,
    height: 40,
    scale: 1,
    locked: false,
    opacity: 1,
    z_index: 1,
  };
  const window = { width: 800, height: 600 };
  const original = {
    layout: { telemetry: layout },
    overlays: [
      { id: "main", layout_overrides: {} },
      { id: "second", layout_overrides: {} },
    ],
  };
  let renderer;
  try {
    act(() => {
      renderer = create(React.createElement(Editor));
    });
    act(() => current.replaceConfig(original));
    act(() => current.beginEdit());
    for (let i = 1; i <= 5; i++)
      act(() =>
        current.setConfig((config) =>
          updateOverlayLayoutSelection(
            config,
            "main",
            "telemetry",
            nudgeWidget(window, layout, i, 0),
          ),
        ),
      );
    act(() => current.endEdit());
    assert.equal(current.config.overlays[0].layout_overrides.telemetry.x, 15);
    assert.equal(current.config.layout.telemetry.x, 10);
    const copied = copyWidgetLayout(
      "Main",
      current.config.overlays[0].layout_overrides.telemetry,
    );
    act(() =>
      current.setConfig((config) =>
        updateOverlayLayoutSelection(
          config,
          "second",
          "telemetry",
          pasteWidgetLayout(window, layout, copied, "position"),
        ),
      ),
    );
    assert.equal(current.config.overlays[1].layout_overrides.telemetry.x, 15);
    act(() => current.undo());
    assert.deepEqual(current.config.overlays[1].layout_overrides, {});
    act(() => current.undo());
    assert.deepEqual(current.config, original);
    act(() => current.redo());
    assert.equal(current.config.overlays[0].layout_overrides.telemetry.x, 15);
  } finally {
    if (renderer) act(() => renderer.unmount());
  }
});

import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { useConfigHistory } from "../src/useConfigHistory.ts";

function renderHistory() {
  let current;
  function Editor() {
    current = useConfigHistory();
    return React.createElement("output", null, current.config?.x ?? "empty");
  }
  let renderer;
  act(() => { renderer = create(React.createElement(React.StrictMode, null, React.createElement(Editor))); });
  return { get current() { return current; }, renderer };
}

test("real React history actions remain stable and resolve functional edits in order", () => {
  const editor = renderHistory();
  try {
    const actions = { setConfig: editor.current.setConfig, undo: editor.current.undo, endEdit: editor.current.endEdit };
    act(() => editor.current.replaceConfig({ x: 0 }));
    act(() => {
      editor.current.setConfig((value) => ({ x: value.x + 1 }));
      editor.current.setConfig((value) => ({ x: value.x + 1 }));
    });
    assert.deepEqual(editor.current.config, { x: 2 });
    assert.equal(editor.current.setConfig, actions.setConfig);
    assert.equal(editor.current.undo, actions.undo);
    assert.equal(editor.current.endEdit, actions.endEdit);
    act(() => editor.current.undo());
    assert.deepEqual(editor.current.config, { x: 1 });
    act(() => editor.current.redo());
    assert.deepEqual(editor.current.config, { x: 2 });
  } finally { act(() => editor.renderer.unmount()); }
});

test("real React gesture grouping and file replacement preserve history ownership", () => {
  const editor = renderHistory();
  try {
    act(() => editor.current.replaceConfig({ x: 0 }));
    act(() => editor.current.beginEdit());
    assert.equal(editor.current.editingGesture, true);
    act(() => {
      editor.current.setConfig({ x: 10 });
      editor.current.setConfig({ x: 20 });
      editor.current.endEdit();
    });
    act(() => editor.current.undo());
    assert.deepEqual(editor.current.config, { x: 0 });
    act(() => editor.current.acknowledgeConfig((value) => value));
    assert.equal(editor.current.canRedo, true);
    act(() => editor.current.replaceConfig({ x: 500 }));
    assert.equal(editor.current.canUndo, false);
    assert.equal(editor.current.canRedo, false);
    assert.equal(editor.current.editingGesture, false);
  } finally { act(() => editor.renderer.unmount()); }
});

import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { useAutosave } from "../src/useAutosave.ts";

test("autosave debounces current edits and respects gestures, manual mode, failures and cancel", (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const writes = [];
  const save = async (value) => {
    writes.push(value);
  };
  let cancel, renderer;
  function Editor({ x, enabled = true, failed = null }) {
    cancel = useAutosave({ x }, '{"x":0}', failed, enabled, save, 100);
    return null;
  }
  try {
    act(() => {
      renderer = create(React.createElement(Editor, { x: 0 }));
    });
    act(() => t.mock.timers.tick(100));
    assert.equal(writes.length, 0);
    act(() => renderer.update(React.createElement(Editor, { x: 1 })));
    act(() => t.mock.timers.tick(50));
    act(() => renderer.update(React.createElement(Editor, { x: 2 })));
    act(() => t.mock.timers.tick(99));
    assert.equal(writes.length, 0);
    act(() => t.mock.timers.tick(1));
    assert.deepEqual(writes, [{ x: 2 }]);
    act(() =>
      renderer.update(React.createElement(Editor, { x: 3, enabled: false })),
    );
    act(() => t.mock.timers.tick(1000));
    assert.equal(writes.length, 1);
    act(() =>
      renderer.update(React.createElement(Editor, { x: 3, failed: '{"x":3}' })),
    );
    act(() => t.mock.timers.tick(1000));
    assert.equal(writes.length, 1);
    act(() => renderer.update(React.createElement(Editor, { x: 4 })));
    act(() => cancel());
    act(() => t.mock.timers.tick(1000));
    assert.equal(writes.length, 1);
    act(() => renderer.update(React.createElement(Editor, { x: 5 })));
    act(() => renderer.unmount());
    renderer = null;
    act(() => t.mock.timers.tick(1000));
    assert.equal(writes.length, 1);
  } finally {
    if (renderer) act(() => renderer.unmount());
  }
});

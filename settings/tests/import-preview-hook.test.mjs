import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { useImportPreview } from "../src/useImportPreview.ts";

test("import review ignores obsolete responses and invalidates when text changes", async () => {
  const requests = [];
  const validate = (text) => new Promise((resolve, reject) => requests.push({ text, resolve, reject }));
  let current;
  function Editor({ text }) { current = useImportPreview(text, validate); return null; }
  let renderer;
  try {
    act(() => { renderer = create(React.createElement(Editor, { text: "first" })); });
    let first;
    act(() => { first = current.review(); });
    assert.equal(current.pending, true);
    act(() => renderer.update(React.createElement(Editor, { text: "second" })));
    assert.equal(current.pending, false);
    let second;
    act(() => { second = current.review(); });
    await act(async () => { requests[1].resolve({ width: 800 }); await second; });
    await act(async () => { requests[0].resolve({ width: 400 }); await first; });
    assert.deepEqual(current.config, { width: 800 });
    act(() => renderer.update(React.createElement(Editor, { text: "edited" })));
    assert.equal(current.config, null);
    let failure;
    act(() => { failure = current.review(); });
    await act(async () => { requests[2].reject(new Error("invalid TOML")); await failure; });
    assert.match(current.error, /invalid TOML/);
    act(() => current.clear());
    assert.equal(current.error, null);
  } finally { if (renderer) act(() => renderer.unmount()); }
});

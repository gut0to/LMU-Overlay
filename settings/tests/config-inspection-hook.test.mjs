import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { useConfigInspection } from "../src/useConfigInspection.ts";

test("saved-file review ignores old editor snapshots and never replaces current editor state", async () => {
  const requests = [];
  const load = () =>
    new Promise((resolve, reject) => requests.push({ resolve, reject }));
  let current, renderer;
  function Editor({ keyValue }) {
    current = useConfigInspection(keyValue, load);
    return null;
  }
  try {
    act(() => {
      renderer = create(React.createElement(Editor, { keyValue: "editor A" }));
    });
    let old;
    act(() => {
      old = current.inspect();
    });
    act(() =>
      renderer.update(React.createElement(Editor, { keyValue: "editor B" })),
    );
    assert.equal(current.pending, false);
    let fresh;
    act(() => {
      fresh = current.inspect();
    });
    const result = {
      path: "saved.toml",
      revision: "18446744073709551615",
      config: { x: 10 },
      text: "# saved",
    };
    await act(async () => {
      requests[1].resolve(result);
      await fresh;
    });
    await act(async () => {
      requests[0].reject(new Error("obsolete failure"));
      await old;
    });
    assert.deepEqual(current.result, result);
    assert.equal(current.error, null);
    act(() =>
      renderer.update(React.createElement(Editor, { keyValue: "editor C" })),
    );
    assert.equal(current.result, null);
    let obsolete;
    act(() => {
      obsolete = current.inspect();
    });
    act(() =>
      renderer.update(React.createElement(Editor, { keyValue: "editor D" })),
    );
    act(() =>
      renderer.update(React.createElement(Editor, { keyValue: "editor C" })),
    );
    await act(async () => {
      requests[2].resolve(result);
      await obsolete;
    });
    assert.equal(
      current.result,
      null,
      "returning to an earlier editor state must not revive an obsolete read",
    );
    act(() => current.clear());
    assert.equal(current.pending, false);
  } finally {
    if (renderer) act(() => renderer.unmount());
  }
});

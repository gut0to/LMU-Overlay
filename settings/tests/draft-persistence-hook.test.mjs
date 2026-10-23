import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { useDraftPersistence } from "../src/useDraftPersistence.ts";
import { DraftStore } from "../src/draft-storage.ts";
import { findRecovery, parseDraft } from "../src/drafts.ts";

test("real React recovery persistence retains pending choices and follows save acknowledgements", () => {
  let raw = null;
  let error = null;
  const store = new DraftStore(() => ({
    getItem: () => raw,
    setItem: (_, value) => {
      raw = value;
    },
    removeItem: () => {
      raw = null;
    },
  }));
  const report = (value) => {
    error = value;
  };
  const path = "C:/settings.toml";
  const saved = { x: 0 };
  function Editor({
    config,
    baseline = JSON.stringify(saved),
    enabled = true,
  }) {
    useDraftPersistence(config, path, baseline, enabled, store, report);
    return null;
  }
  let renderer;
  try {
    act(() => {
      renderer = create(React.createElement(Editor, { config: saved }));
    });
    act(() =>
      renderer.update(React.createElement(Editor, { config: { x: 10 } })),
    );
    const retained = raw;
    assert.deepEqual(findRecovery(raw, path, saved).draft.config, { x: 10 });
    act(() =>
      renderer.update(
        React.createElement(Editor, { config: saved, enabled: false }),
      ),
    );
    assert.equal(
      raw,
      retained,
      "an unresolved recovery choice must survive the clean loaded editor",
    );
    act(() =>
      renderer.update(
        React.createElement(Editor, {
          config: { x: 20 },
          baseline: JSON.stringify({ x: 10 }),
        }),
      ),
    );
    assert.equal(parseDraft(raw).base, JSON.stringify({ x: 10 }));
    assert.deepEqual(parseDraft(raw).config, { x: 20 });
    act(() =>
      renderer.update(
        React.createElement(Editor, {
          config: { x: 20 },
          baseline: JSON.stringify({ x: 20 }),
        }),
      ),
    );
    assert.equal(raw, null);
    assert.equal(error, null);
  } finally {
    if (renderer) act(() => renderer.unmount());
  }
});

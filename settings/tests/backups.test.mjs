import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { parseBackups, backupTime } from "../src/backups.ts";
import { useConfigBackups } from "../src/useConfigBackups.ts";

test("backup metadata rejects unsafe paths, duplicates and coerced sizes", () => {
  const item = { id: "123-1-1.toml", modified_ms: 1, size_bytes: 20 };
  assert.deepEqual(parseBackups([item, item, { ...item, id: "../1.toml" }, { ...item, id: "124.toml", size_bytes: "20" }]), [item]);
  assert.deepEqual(parseBackups(null), []);
  assert.equal(backupTime(0), "Unknown time");
  assert.equal(backupTime(Number.MAX_SAFE_INTEGER), "Unknown time");
});

test("backup hook ignores results from an earlier configuration owner", async () => {
  const requests = [];
  const load = () => new Promise((resolve, reject) => requests.push({ resolve, reject }));
  let current, renderer;
  function Editor({ owner }) { current = useConfigBackups(owner, load); return null; }
  try {
    act(() => { renderer = create(React.createElement(Editor, { owner: "first" })); });
    act(() => renderer.update(React.createElement(Editor, { owner: "second" })));
    await act(async () => { requests[1].resolve([{ id: "222.toml", modified_ms: 1, size_bytes: 10 }]); await Promise.resolve(); });
    await act(async () => { requests[0].reject(new Error("stale failure")); await Promise.resolve(); });
    assert.equal(current.items[0].id, "222.toml");
    assert.equal(current.error, null);
    assert.equal(current.loading, false);
  } finally { if (renderer) act(() => renderer.unmount()); }
});

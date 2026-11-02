import assert from "node:assert/strict";
import test from "node:test";
import { compareConfiguration, overlappingPaths } from "../src/config-conflicts.ts";

test("three-way review separates local and external edits from conflicting final values", () => {
  const baseline = { window: { x: 0, y: 0 }, style: { opacity: 255 } };
  const editor = { window: { x: 10, y: 20 }, style: { opacity: 255 } };
  const saved = { window: { x: 30, y: 20 }, style: { opacity: 100 } };
  const result = compareConfiguration(baseline, editor, saved);
  assert.equal(result.localCount, 2);
  assert.equal(result.externalCount, 3);
  assert.deepEqual(result.conflicts, [{ path: "window.x", editor: 10, saved: 30 }]);
  assert.equal(result.differences.total, 2);
  assert.equal(overlappingPaths("window.x", "window.xyz"), false);
  assert.equal(overlappingPaths("overlays[0]", "overlays[0].name"), true);
});

test("parent deletions conflict with nested edits and retain the values at the common path", () => {
  const baseline = { group: { a: 1, b: 2 } };
  const editor = {};
  const saved = { group: { a: 3 } };
  const result = compareConfiguration(baseline, editor, saved);
  assert.deepEqual(result.conflicts, [{ path: "group", editor: undefined, saved: { a: 3 } }]);
  assert.equal(compareConfiguration(baseline, {}, {}).conflictCount, 0);
  assert.equal(compareConfiguration({ a: 0, b: 0 }, { a: 1, b: 1 }, { a: 2, b: 2 }, 1).conflicts.length, 1);
});

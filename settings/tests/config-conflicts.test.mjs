import assert from "node:assert/strict";
import test from "node:test";
import {
  compareConfiguration,
  overlappingPaths,
} from "../src/config-conflicts.ts";

test("large conflict reviews retain full counts and bound displayed results", () => {
  const baseline = Object.fromEntries(
    Array.from({ length: 2000 }, (_, i) => [`field${i}`, 0]),
  );
  const editor = Object.fromEntries(
    Object.keys(baseline).map((key) => [key, 1]),
  );
  const saved = Object.fromEntries(
    Object.keys(baseline).map((key) => [key, 2]),
  );
  const result = compareConfiguration(baseline, editor, saved);
  assert.equal(result.localCount, 2000);
  assert.equal(result.externalCount, 2000);
  assert.equal(result.conflictCount, 2000);
  assert.equal(result.conflicts.length, 50);
  assert.equal(result.differences.items.length, 50);
  assert.equal(
    compareConfiguration({ group: { a: 1 } }, { group: 0 }, { group: { a: 2 } })
      .conflictCount,
    1,
  );
  assert.equal(
    compareConfiguration({ group: { a: 1 } }, { group: { a: 2 } }, { group: 0 })
      .conflictCount,
    1,
  );
});

test("three-way review separates local and external edits from conflicting final values", () => {
  const baseline = { window: { x: 0, y: 0 }, style: { opacity: 255 } };
  const editor = { window: { x: 10, y: 20 }, style: { opacity: 255 } };
  const saved = { window: { x: 30, y: 20 }, style: { opacity: 100 } };
  const result = compareConfiguration(baseline, editor, saved);
  assert.equal(result.localCount, 2);
  assert.equal(result.externalCount, 3);
  assert.deepEqual(result.conflicts, [
    { path: "window.x", editor: 10, saved: 30 },
  ]);
  assert.equal(result.differences.total, 2);
  assert.equal(overlappingPaths("window.x", "window.xyz"), false);
  assert.equal(overlappingPaths("overlays[0]", "overlays[0].name"), true);
});

test("parent deletions conflict with nested edits and retain the values at the common path", () => {
  const baseline = { group: { a: 1, b: 2 } };
  const editor = {};
  const saved = { group: { a: 3 } };
  const result = compareConfiguration(baseline, editor, saved);
  assert.deepEqual(result.conflicts, [
    { path: "group", editor: undefined, saved: { a: 3 } },
  ]);
  assert.equal(compareConfiguration(baseline, {}, {}).conflictCount, 0);
  assert.equal(
    compareConfiguration({ a: 0, b: 0 }, { a: 1, b: 1 }, { a: 2, b: 2 }, 1)
      .conflicts.length,
    1,
  );
});

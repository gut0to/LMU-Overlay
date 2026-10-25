import assert from "node:assert/strict";
import test from "node:test";
import {
  configurationChanges,
  formatChangeValue,
} from "../src/config-changes.ts";

test("import changes identify nested fields, additions, removals and surface membership", () => {
  const before = {
    window: { x: 0 },
    overlays: [{ name: "Main", widgets: ["inputs"] }],
    removed: true,
  };
  const after = {
    window: { x: -1920 },
    overlays: [{ name: "Main", widgets: ["inputs", "fuel"] }],
    added: false,
  };
  const result = configurationChanges(before, after);
  assert.equal(result.total, 4);
  assert.deepEqual(
    result.items.map((item) => item.path),
    ["added", "overlays[0].widgets[1]", "removed", "window.x"],
  );
  assert.deepEqual(result.items[1], {
    path: "overlays[0].widgets[1]",
    before: undefined,
    after: "fuel",
  });
  assert.equal(configurationChanges({ b: 2, a: 1 }, { a: 1, b: 2 }).total, 0);
});

test("large reviews retain an exact total with a bounded visible list", () => {
  const result = configurationChanges(
    { a: 0, b: 0, c: 0 },
    { a: 1, b: 1, c: 1 },
    1,
  );
  assert.equal(result.total, 3);
  assert.equal(result.items.length, 1);
  assert.equal(configurationChanges(0, 1, 0).items.length, 0);
  assert.equal(formatChangeValue(undefined), "Not set");
  assert.equal(formatChangeValue(""), '""');
  assert.equal(formatChangeValue(false), "false");
  assert.equal(formatChangeValue(null), "null");
  assert.equal(formatChangeValue("x".repeat(200)).length, 158);
});

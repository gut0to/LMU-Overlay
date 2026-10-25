import assert from "node:assert/strict";
import test from "node:test";
import { layoutNudge } from "../src/layout-shortcuts.ts";

test("arrows nudge one pixel and Shift accelerates to ten pixels", () => {
  const event = {
    key: "ArrowLeft",
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
  };
  assert.deepEqual(layoutNudge(event), { dx: -1, dy: 0 });
  assert.deepEqual(
    layoutNudge({ ...event, shiftKey: true, key: "ArrowDown" }),
    { dx: 0, dy: 10 },
  );
  for (const modifier of ["ctrlKey", "altKey", "metaKey", "isComposing"])
    assert.equal(layoutNudge({ ...event, [modifier]: true }), null);
  assert.equal(layoutNudge({ ...event, key: "Tab" }), null);
  assert.equal(layoutNudge({ ...event, key: "z" }), null);
});

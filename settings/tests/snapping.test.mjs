import assert from "node:assert/strict";
import test from "node:test";
import { snapToWidgetLayouts } from "../src/snapping.ts";
const widget = { x: 100, y: 100, width: 50, height: 20, scale: 1, opacity: 1, z_index: 0, locked: false };
test("snapping selects the closest target instead of cascading through neighbors", () => {
  const first = { ...widget, x: 104, y: 200 };
  const second = { ...widget, x: 109, y: 300 };
  assert.equal(snapToWidgetLayouts(widget, [first, second], 10).x, 104);
  assert.equal(snapToWidgetLayouts(widget, [second, first], 10).x, 104);
});

test("zero snap distance only permits exact alignments", async () => {
  const target = { ...widget, x: 101, y: 101 };
  assert.deepEqual(snapToWidgetLayouts(widget, [target], 0), widget);
  assert.deepEqual(snapToWidgetLayouts(widget, [], 10), widget);
  assert.deepEqual(snapToWidgetLayouts(widget, [target], -1), widget);
});

test("equidistant alignments are stable across neighbor ordering", async () => {
  const neighbors = [96, 104, 200].map((x) => ({ ...widget, x, y: x }));
  const expected = snapToWidgetLayouts(widget, neighbors, 10);
  for (const order of [neighbors.toReversed(), [neighbors[1], neighbors[2], neighbors[0]], [neighbors[2], neighbors[0], neighbors[1]]]) {
    assert.deepEqual(snapToWidgetLayouts(widget, order, 10), expected);
  }
  assert.equal(expected.x, 96);
});

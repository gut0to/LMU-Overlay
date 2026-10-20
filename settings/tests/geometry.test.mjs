import assert from "node:assert/strict";
import test from "node:test";
import { normalizeWidgetLayoutForWindow, scaledDimension } from "../src/geometry.ts";
const screen = { width: 1920, height: 1080 };
const widget = { x: 100, y: 100, width: 320, height: 80, scale: 1, opacity: 1, z_index: 0, locked: false };
test("normalization preserves valid geometry without mutating it", () => {
  assert.deepEqual(normalizeWidgetLayoutForWindow(screen, widget), widget);
  assert.equal(scaledDimension(100, 1.5), 150);
  assert.notEqual(normalizeWidgetLayoutForWindow(screen, widget), widget);
});

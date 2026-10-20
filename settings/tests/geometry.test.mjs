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

test("small windows constrain visual scale as well as size", async () => {
  const result = normalizeWidgetLayoutForWindow({ width: 48, height: 48 }, { ...widget, scale: 2 });
  assert.ok(scaledDimension(result.width, result.scale) <= 48);
  assert.equal(result.x, 0);
});

test("malformed and extreme widget values normalize consistently", async () => {
  for (const value of [NaN, Infinity, -Infinity, -10000, 0, 10000]) {
    const input = { ...widget, x: value, y: value, width: value, height: value, scale: value, opacity: value, z_index: value };
    const result = normalizeWidgetLayoutForWindow(screen, input);
    assert.ok(Object.values(result).every((item) => typeof item !== "number" || Number.isFinite(item)));
    assert.ok(result.x >= 0 && result.y >= 0);
    assert.ok(result.x + scaledDimension(result.width, result.scale) <= screen.width);
    assert.ok(result.y + scaledDimension(result.height, result.scale) <= screen.height);
    assert.deepEqual(normalizeWidgetLayoutForWindow(screen, result), result);
  }
});

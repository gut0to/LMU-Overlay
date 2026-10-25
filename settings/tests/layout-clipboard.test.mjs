import assert from "node:assert/strict";
import test from "node:test";
import { copyWidgetLayout, pasteWidgetLayout } from "../src/layout-clipboard.ts";

const window = { width: 800, height: 600 };
const source = { x: 50, y: 100, width: 100, height: 40, scale: 2, locked: true, opacity: 0.5, z_index: 99 };
const target = { ...source, x: 0, y: 0, scale: 1, locked: false, opacity: 1, z_index: 3 };

test("layout clipboard owns geometry and excludes locks appearance and stacking", () => {
  const copied = copyWidgetLayout("Main / telemetry", source);
  assert.deepEqual(Object.keys(copied.geometry), ["x", "y", "width", "height", "scale"]);
  const pasted = pasteWidgetLayout(window, target, copied, "geometry");
  assert.equal(pasted.x, 50);
  assert.equal(pasted.scale, 2);
  assert.equal(pasted.locked, false);
  assert.equal(pasted.opacity, 1);
  assert.equal(pasted.z_index, 3);
  const locked = { ...target, locked: true };
  assert.equal(pasteWidgetLayout(window, locked, copied, "geometry"), locked);
});

test("size-only paste matches visual size and position-only paste leaves size unchanged", () => {
  const copied = copyWidgetLayout("Source", source);
  const sized = pasteWidgetLayout(window, target, copied, "size");
  assert.equal(sized.width, 200);
  assert.equal(sized.height, 80);
  assert.equal(sized.scale, 1);
  assert.equal(sized.x, 0);
  const positioned = pasteWidgetLayout(window, target, copied, "position");
  assert.equal(positioned.x, 50);
  assert.equal(positioned.width, 100);
  assert.equal(positioned.scale, 1);
});

import assert from "node:assert/strict";
import test from "node:test";
import { alignWidgetToReference, nudgeWidget, moveWidgetInStack, fitWindowToWidgets } from "../src/layout-tools.ts";

export const window = { x: 0, y: 0, width: 800, height: 600 };
export const widget = { x: 10, y: 20, width: 100, height: 40, scale: 2, opacity: 1, z_index: 1, locked: false };
const reference = { ...widget, x: 300, y: 200, width: 200, height: 80, scale: 1.5 };

test("reference alignment uses scaled centers and edges on both axes", () => {
  assert.equal(alignWidgetToReference(window, widget, reference, "left").x, 300);
  assert.equal(alignWidgetToReference(window, widget, reference, "center-x").x, 350);
  assert.equal(alignWidgetToReference(window, widget, reference, "right").x, 400);
  assert.equal(alignWidgetToReference(window, widget, reference, "top").y, 200);
  assert.equal(alignWidgetToReference(window, widget, reference, "center-y").y, 220);
  assert.equal(alignWidgetToReference(window, widget, reference, "bottom").y, 240);
  assert.equal(widget.x, 10);
});

test("alignment respects locks and keeps visible bounds inside the target surface", () => {
  const locked = { ...widget, locked: true };
  assert.equal(alignWidgetToReference(window, locked, reference, "left"), locked);
  assert.equal(alignWidgetToReference(window, widget, { ...reference, x: 790 }, "right").x, 600);
});

test("nudges are exact pixels without grid snapping and clamp at all surface edges", () => {
  assert.equal(nudgeWidget(window, widget, 1, 0).x, 11);
  assert.equal(nudgeWidget(window, widget, -100, -100).x, 0);
  assert.equal(nudgeWidget(window, widget, 9999, 9999).y, 520);
  assert.equal(nudgeWidget(window, widget, NaN, 0), widget);
  const locked = { ...widget, locked: true };
  assert.equal(nudgeWidget(window, locked, 10, 10), locked);
});

test("stacking moves past visible neighbors and stays within native order limits", () => {
  assert.equal(moveWidgetInStack(widget, [{ ...widget, z_index: 100 }], "front").z_index, 101);
  assert.equal(moveWidgetInStack(widget, [{ ...widget, z_index: -100 }], "back").z_index, -101);
  assert.equal(moveWidgetInStack({ ...widget, z_index: 1000 }, [], "front").z_index, 1000);
  const locked = { ...widget, locked: true };
  assert.equal(moveWidgetInStack(locked, [], "front"), locked);
});

test("fit window accounts for scaled extents while preserving monitor position and widgets", () => {
  const source = { ...window, x: -1920, y: 120 };
  const fitted = fitWindowToWidgets(source, [{ ...widget, x: 300, y: 200 }]);
  assert.equal(fitted.width, 514);
  assert.equal(fitted.height, 294);
  assert.equal(fitted.x, -1920);
  assert.equal(fitted.y, 120);
  assert.equal(fitWindowToWidgets(source, []), source);
  assert.equal(fitWindowToWidgets(source, [{ ...widget, x: 9999 }]).width, 1200);
});

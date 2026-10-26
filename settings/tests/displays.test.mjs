import assert from "node:assert/strict";
import test from "node:test";
import { parseDisplays, displayWorkspace, matchesDisplay } from "../src/displays.ts";

const display = { id: "DISPLAY2", name: "Left monitor", primary: false, bounds: { x: -1920, y: -120, width: 1920, height: 1080 }, work_area: { x: -1920, y: -120, width: 1920, height: 1040 } };
test("detected monitor and work-area workspaces preserve extended desktop coordinates", () => {
  const workspace = displayWorkspace(display, true);
  assert.deepEqual(workspace, { width: 1920, height: 1040, originX: -1920, originY: -120 });
  assert.equal(matchesDisplay(workspace, display, true), true);
  assert.equal(matchesDisplay(workspace, display, false), false);
  assert.equal(displayWorkspace({ ...display, bounds: { ...display.bounds, width: 1e8 } }, false).width, 16384);
});

test("invalid native display lists do not coerce dimensions or duplicate monitor ids", () => {
  assert.deepEqual(parseDisplays(null), []);
  assert.deepEqual(parseDisplays([display, display, { ...display, id: "bad", bounds: { ...display.bounds, width: "1920" } }, { ...display, id: "zero", work_area: { ...display.work_area, height: 0 } }]), [display]);
});

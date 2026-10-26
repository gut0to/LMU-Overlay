import assert from "node:assert/strict";
import test from "node:test";
import { fitWindowToWorkspace } from "../src/display-placement.ts";

test("fit a selected surface onto a left monitor without moving its widgets or changing sample settings", () => {
  const window = { x: 4000, y: 2000, width: 800, height: 600, refresh_hz: 60, sample_ms: 10, history_samples: 240 };
  const result = fitWindowToWorkspace(window, { originX: -1920, originY: -120, width: 1920, height: 1040 });
  assert.equal(result.x, -800);
  assert.equal(result.y, 320);
  assert.equal(result.sample_ms, 10);
  assert.equal(window.x, 4000);
  const small = fitWindowToWorkspace(window, { originX: 100, originY: 200, width: 320, height: 240 });
  assert.deepEqual([small.x, small.y, small.width, small.height], [100, 200, 320, 240]);
});

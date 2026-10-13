import assert from "node:assert/strict";
import test from "node:test";
import { restoreWorkspace } from "../src/workspace.ts";

const defaults = { width: 1920, height: 1080, originX: 0, originY: 0 };

test("missing and malformed preferences recover default workspace", () => {
  for (const saved of [null, "", "{", "null", "[]", "true", '"screen"']) {
    assert.deepEqual(restoreWorkspace(saved, defaults), defaults);
  }
});

test("partial preferences preserve defaults and negative monitor origins", () => {
  assert.deepEqual(restoreWorkspace('{"originX":-1920}', defaults), {
    ...defaults, originX: -1920,
  });
});

test("dimensions stay within supported bounds", () => {
  assert.deepEqual(restoreWorkspace('{"width":1,"height":99999}', defaults), {
    ...defaults, width: 320, height: 8640,
  });
  assert.deepEqual(restoreWorkspace('{"width":99999,"height":-1}', defaults), {
    ...defaults, width: 16384, height: 240,
  });
});

test("invalid JSON field types never coerce into display coordinates", () => {
  for (const value of [null, true, false, "1440", [], {}]) {
    const saved = JSON.stringify({ width: value, height: value, originX: value, originY: value });
    assert.deepEqual(restoreWorkspace(saved, defaults), defaults);
  }
});

test("overflowed numeric literals recover defaults", () => {
  assert.deepEqual(restoreWorkspace('{"width":1e999,"originY":-1e999}', defaults), defaults);
});

test("fractional coordinates normalize to pixels and do not mutate defaults", () => {
  assert.deepEqual(restoreWorkspace('{"width":2560.4,"height":1440.6,"originX":-1919.8,"originY":20.2}', defaults), {
    width: 2560, height: 1441, originX: -1920, originY: 20,
  });
  assert.notEqual(restoreWorkspace(null, defaults), defaults);
  assert.deepEqual(defaults, { width: 1920, height: 1080, originX: 0, originY: 0 });
});

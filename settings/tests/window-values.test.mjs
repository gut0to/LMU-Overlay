import assert from "node:assert/strict";
import test from "node:test";
import { normalizeWindowField } from "../src/window-values.ts";
test("window values remain finite when editor input is malformed", () => {
  for (const key of ["x", "y", "width", "height", "refresh_hz", "sample_ms", "history_samples"]) {
    for (const value of [NaN, Infinity, -Infinity]) assert.ok(Number.isFinite(normalizeWindowField(key, value)));
  }
  assert.equal(normalizeWindowField("x", -1920), -1920);
});

test("window limits preserve negative monitor origins and integer dimensions", async () => {
  assert.equal(normalizeWindowField("x", -1e20), -2147483648);
  assert.equal(normalizeWindowField("y", 1e20), 2147483647);
  for (const [key, min, max] of [["width",280,1200],["height",140,800],["refresh_hz",15,144],["sample_ms",5,250],["history_samples",16,900]]) {
    assert.equal(normalizeWindowField(key, -1e20), min);
    assert.equal(normalizeWindowField(key, 1e20), max);
    assert.equal(normalizeWindowField(key, min + 0.6), min + 1);
  }
});

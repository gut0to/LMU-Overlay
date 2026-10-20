import assert from "node:assert/strict";
import test from "node:test";
import { normalizeWindowField } from "../src/window-values.ts";
test("window values remain finite when editor input is malformed", () => {
  for (const key of ["x", "y", "width", "height", "refresh_hz", "sample_ms", "history_samples"]) {
    for (const value of [NaN, Infinity, -Infinity]) assert.ok(Number.isFinite(normalizeWindowField(key, value)));
  }
  assert.equal(normalizeWindowField("x", -1920), -1920);
});

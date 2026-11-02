import assert from "node:assert/strict";
import test from "node:test";
import { importRequestKey, readImportRequest } from "../src/import-request.ts";

test("review ownership includes the current editor base, TOML and selected scope", () => {
  const base = { window: { x: 0 } };
  const key = importRequestKey(base, "config_version = 7", "appearance");
  assert.deepEqual(readImportRequest(key), { base, text: "config_version = 7", scope: "appearance" });
  assert.notEqual(importRequestKey({ window: { x: 10 } }, "config_version = 7", "appearance"), key);
  assert.notEqual(importRequestKey(base, "config_version = 6", "appearance"), key);
  assert.notEqual(importRequestKey(base, "config_version = 7", "full"), key);
  assert.equal(importRequestKey(null, "text", "full"), "");
  assert.throws(() => readImportRequest('{"base":{},"text":"text","scope":"unknown"}'), /Preview it again/);
});

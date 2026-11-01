import assert from "node:assert/strict";
import test from "node:test";
import { restoreSavePreferences } from "../src/save-preferences.ts";

test("manual saving survives reload without coercing malformed preference values", () => {
  assert.deepEqual(restoreSavePreferences('{"version":1,"autosave":false}'), {
    version: 1,
    autosave: false,
  });
  for (const raw of [
    null,
    "{",
    "[]",
    "null",
    '{"version":2,"autosave":false}',
    '{"version":1,"autosave":"false"}',
    '{"version":1,"autosave":0}',
  ]) {
    assert.deepEqual(restoreSavePreferences(raw), {
      version: 1,
      autosave: true,
    });
  }
});

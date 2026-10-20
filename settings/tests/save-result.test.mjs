import assert from "node:assert/strict";
import test from "node:test";
import { acceptSavedConfig } from "../src/save-result.ts";
test("save acknowledgements preserve edits made during the write", () => {
  const requested = { width: 300 };
  const edited = { width: 500 };
  assert.equal(acceptSavedConfig(edited, requested, { width: 320 }), edited);
  assert.deepEqual(acceptSavedConfig(requested, requested, { width: 320 }), { width: 320 });
});

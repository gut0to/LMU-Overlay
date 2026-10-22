import assert from "node:assert/strict";
import test from "node:test";
import { saveFeedback } from "../src/save-feedback.ts";
test("saved settings retain reload warnings without claiming live success", () => {
  assert.deepEqual(saveFeedback("Saved", "Configuration saved; overlay reload failed"), {
    message: "Configuration saved; overlay reload failed", liveApplied: false,
  });
});

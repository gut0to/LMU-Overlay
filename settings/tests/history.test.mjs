import assert from "node:assert/strict";
import test from "node:test";
import { createHistory, editHistory, undoHistory } from "../src/history.ts";
test("undo restores the previous configuration snapshot", () => {
  let state = createHistory({ x: 0 });
  state = editHistory(state, { x: 20 });
  state = editHistory(state, { x: 40 });
  assert.deepEqual(undoHistory(state).present, { x: 20 });
  assert.deepEqual(state.present, { x: 40 });
});

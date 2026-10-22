import assert from "node:assert/strict";
import test from "node:test";
import { createHistory, editHistory, undoHistory, redoHistory, beginHistory, endHistory } from "../src/history.ts";
test("undo restores the previous configuration snapshot", () => {
  let state = createHistory({ x: 0 });
  state = editHistory(state, { x: 20 });
  state = editHistory(state, { x: 40 });
  assert.deepEqual(undoHistory(state).present, { x: 20 });
  assert.deepEqual(state.present, { x: 40 });
});

test("redo restores undone edits and new edits replace the old branch", () => {
  let state = editHistory(createHistory({ x: 0 }), { x: 10 });
  state = undoHistory(state);
  assert.deepEqual(redoHistory(state).present, { x: 10 });
  state = editHistory(state, { x: 30 });
  assert.equal(redoHistory(state), state);
  assert.deepEqual(state.future, []);
});

test("a pointer gesture groups all intermediate positions into one undo", () => {
  let state = beginHistory(createHistory({ x: 0 }));
  for (let x = 1; x <= 100; x++) state = editHistory(state, { x });
  state = endHistory(state);
  assert.equal(state.past.length, 1);
  assert.deepEqual(undoHistory(state).present, { x: 0 });
  assert.deepEqual(redoHistory(undoHistory(state)).present, { x: 100 });
});

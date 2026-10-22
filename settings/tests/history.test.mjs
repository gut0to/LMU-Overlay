import assert from "node:assert/strict";
import test from "node:test";
import { acceptSavedConfig } from "../src/save-result.ts";
import { createHistory, editHistory, undoHistory, redoHistory, beginHistory, endHistory, acknowledgeHistory, historyReducer } from "../src/history.ts";
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

test("save acknowledgements do not add undo entries or erase redo", () => {
  let state = undoHistory(editHistory(createHistory({ width: 300 }), { width: 500 }));
  state = acknowledgeHistory(state, { width: 320 });
  assert.equal(state.past.length, 0);
  assert.deepEqual(redoHistory(state).present, { width: 500 });
});

test("a slow save acknowledgement preserves an undo performed during the write", () => {
  const request = { x: 50 };
  let state = editHistory(createHistory({ x: 0 }), request);
  state = undoHistory(state);
  state = acknowledgeHistory(state, acceptSavedConfig(state.present, request, { x: 48 }));
  assert.deepEqual(state.present, { x: 0 });
  assert.deepEqual(redoHistory(state).present, request);
});

test("replacing the configuration clears stale edits and redo branches", () => {
  let state = undoHistory(editHistory(createHistory({ x: 0 }), { x: 20 }));
  state = historyReducer(state, { kind: "replace", value: { x: 500 } });
  assert.deepEqual(state.present, { x: 500 });
  assert.equal(undoHistory(state), state);
  assert.equal(redoHistory(state), state);
});

test("history reducers preserve immutable snapshots and ignore no-op edits", () => {
  const original = Object.freeze({ x: 0 });
  const state = createHistory(original);
  Object.freeze(state.past); Object.freeze(state.future); Object.freeze(state);
  const edited = historyReducer(state, { kind: "edit", update: (current) => ({ ...current, x: current.x + 1 }) });
  assert.deepEqual(edited.present, { x: 1 });
  assert.deepEqual(state.present, { x: 0 });
  assert.equal(editHistory(edited, { x: 1 }), edited);
  assert.deepEqual(historyReducer(state, { kind: "edit", update: (current) => ({ ...current, x: current.x + 1 }) }), edited);
});

import assert from "node:assert/strict";
import test from "node:test";
import { historyShortcut } from "../src/history-shortcut.ts";
const event = (key, extra = {}) => ({ key, ctrlKey: true, metaKey: false, shiftKey: false, altKey: false, repeat: false, isComposing: false, ...extra });
test("standard history shortcuts preserve native text editing", () => {
  assert.equal(historyShortcut(event("z"), false), "undo");
  assert.equal(historyShortcut(event("z", { shiftKey: true }), false), "redo");
  assert.equal(historyShortcut(event("y"), false), "redo");
  assert.equal(historyShortcut(event("z"), true), null);
});

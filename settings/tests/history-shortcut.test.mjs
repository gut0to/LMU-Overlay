import assert from "node:assert/strict";
import test from "node:test";
import { historyShortcut } from "../src/history-shortcut.ts";
const event = (key, extra = {}) => ({
  key,
  ctrlKey: true,
  metaKey: false,
  shiftKey: false,
  altKey: false,
  repeat: false,
  isComposing: false,
  ...extra,
});
test("standard history shortcuts preserve native text editing", () => {
  assert.equal(historyShortcut(event("z"), false), "undo");
  assert.equal(historyShortcut(event("z", { shiftKey: true }), false), "redo");
  assert.equal(historyShortcut(event("y"), false), "redo");
  assert.equal(historyShortcut(event("z"), true), null);
});

test("history shortcuts ignore repeats, composition and unrelated chords", () => {
  for (const flag of ["repeat", "isComposing", "altKey"])
    assert.equal(historyShortcut(event("z", { [flag]: true }), false), null);
  assert.equal(historyShortcut(event("z", { ctrlKey: false }), false), null);
  assert.equal(
    historyShortcut(event("z", { ctrlKey: false, metaKey: true }), false),
    "undo",
  );
  assert.equal(historyShortcut(event("a"), false), null);
});

import assert from "node:assert/strict";
import test from "node:test";
import { captureHotkey, normalizeHotkey } from "../src/hotkeys.ts";
const event = (key, extra = {}) => ({ key, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, repeat: false, isComposing: false, ...extra });
test("captures only host-supported function keys", () => {
  assert.equal(captureHotkey(event("F12", { ctrlKey: true, altKey: true })), "Ctrl+Alt+F12");
  for (const key of ["a", "Enter", " ", "F0", "F13", "Dead", "Tab"]) assert.equal(captureHotkey(event(key)), null);
  for (const flag of ["repeat", "metaKey", "isComposing"]) assert.equal(captureHotkey(event("F9", { [flag]: true })), null);
});

test("equivalent modifier orders and aliases conflict", () => {
  assert.equal(normalizeHotkey("shift + control + f9"), normalizeHotkey("Ctrl+Shift+F9"));
});

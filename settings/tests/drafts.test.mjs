import assert from "node:assert/strict";
import test from "node:test";
import {
  createDraft,
  parseDraft,
  findRecovery,
  DRAFT_RETENTION_MS,
  MAX_DRAFT_LENGTH,
} from "../src/drafts.ts";

const now = 1_800_000_000_000;
const path = "C:\\AppData\\HashOverlay\\overlay.toml";
const saved = { window: { x: 0 } };
const edited = { window: { x: 42 } };
const raw = () => createDraft(path, JSON.stringify(saved), edited, now);

test("recovery snapshots keep their file and baseline and do not alias later edits", () => {
  const draft = raw();
  assert.deepEqual(parseDraft(draft, now).config, edited);
  assert.equal(
    findRecovery(draft, path.toLowerCase().replaceAll("\\", "/"), saved, now)
      .conflict,
    false,
  );
  assert.equal(
    findRecovery(draft, path, { window: { x: 99 } }, now).conflict,
    true,
  );
  assert.equal(findRecovery(draft, path, edited, now), null);
  assert.equal(findRecovery(draft, "C:/other.toml", saved, now), null);
});

test("recovery refuses corrupt, future, expired and unsupported envelopes", () => {
  for (const value of [
    null,
    "{",
    "[]",
    "null",
    JSON.stringify({ ...JSON.parse(raw()), version: 2 }),
    JSON.stringify({ ...JSON.parse(raw()), savedAt: "yesterday" }),
    JSON.stringify({ ...JSON.parse(raw()), config: [] }),
  ]) {
    assert.equal(parseDraft(value, now), null);
  }
  assert.equal(parseDraft(raw(), now + DRAFT_RETENTION_MS + 1), null);
  assert.equal(parseDraft(raw(), now - 5 * 60 * 1000 - 1), null);
  assert.ok(parseDraft(raw(), now + DRAFT_RETENTION_MS));
});

test("draft size is bounded before storing and before parsing", () => {
  assert.throws(
    () => createDraft(path, "{}", { text: "x".repeat(MAX_DRAFT_LENGTH) }, now),
    /storage limit/,
  );
  assert.equal(parseDraft(" ".repeat(MAX_DRAFT_LENGTH + 1), now), null);
});

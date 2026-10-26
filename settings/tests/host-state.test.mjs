import assert from "node:assert/strict";
import test from "node:test";
import { projectHostState } from "../src/host-state.ts";

test("failed and stopped snapshots clear stale live flags without claiming readiness", () => {
  assert.deepEqual(
    projectHostState({ state: "stopped", visible: true, editing: true }),
    { state: "stopped", visible: false, editing: false, error: null },
  );
  const failed = projectHostState({
    state: "unreachable",
    visible: true,
    editing: true,
    error: "deadline exceeded",
  });
  assert.equal(failed.visible, false);
  assert.equal(failed.editing, false);
  assert.equal(failed.error, "deadline exceeded");
  assert.equal(
    projectHostState({ state: "running", visible: false, editing: true }).state,
    "running",
  );
});

test("invalid native responses never coerce strings into live host flags", () => {
  for (const value of [
    null,
    false,
    {},
    { state: "running", visible: "false", editing: false },
    { state: "future" },
  ]) {
    assert.equal(projectHostState(value).state, "unreachable");
  }
});

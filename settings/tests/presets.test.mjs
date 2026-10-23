import assert from "node:assert/strict";
import test from "node:test";
import { currentProfile, profilesMatch } from "../src/presets.ts";

test("preset identity ignores object insertion order, including nested widget maps", () => {
  assert.equal(profilesMatch({ style: { a: 1, b: 2 }, extra_widgets: {} }, { extra_widgets: {}, style: { b: 2, a: 1 } }), true);
  assert.equal(profilesMatch({ style: { a: 1 } }, { style: { a: 2 } }), false);
});

test("captured profiles own their nested values", () => {
  const config = { performance: { mode: "normal" }, timing: { reference_mode: "personal_best", mini_sectors: 20 }, style: { opacity: 1 }, units: {}, coaching: {}, layout: { telemetry: { x: 10 } }, extra_widgets: {}, widgets: { pedals: true } };
  const profile = currentProfile(config);
  config.layout.telemetry.x = 99;
  profile.style.opacity = 0.5;
  assert.equal(profile.layout.telemetry.x, 10);
  assert.equal(config.style.opacity, 1);
  assert.equal(profile.pedals, true);
});

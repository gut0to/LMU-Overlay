import assert from "node:assert/strict";
import test from "node:test";
import { currentProfile, profilesMatch, addCustomPreset, CUSTOM_PRESET_LIMIT, uniquePresetName, duplicateCustomPreset } from "../src/presets.ts";

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

test("creating beyond the preset limit leaves existing profiles untouched", () => {
  const config = { presets: { custom: Array.from({ length: CUSTOM_PRESET_LIMIT }, (_, i) => ({ name: `Preset ${i}` })) } };
  assert.equal(addCustomPreset(config), config);
});

test("new names fill gaps and avoid case-insensitive collisions", () => {
  assert.equal(uniquePresetName([{ name: "CUSTOM 1" }, { name: "Custom 3" }], "Custom"), "Custom 2");
});

test("duplicate inserts an independent profile immediately after its source", () => {
  const config = { presets: { custom: [{ name: "Race", profile: { style: { opacity: 1 } } }, { name: "Practice", profile: {} }] } };
  const result = duplicateCustomPreset(config, 0);
  assert.deepEqual(result.presets.custom.map((p) => p.name), ["Race", "Race copy 1", "Practice"]);
  result.presets.custom[1].profile.style.opacity = 0.4;
  assert.equal(config.presets.custom[0].profile.style.opacity, 1);
  assert.equal(duplicateCustomPreset(config, -1), config);
});

import assert from "node:assert/strict";
import test from "node:test";
import {
  setCatalogWidgets,
  catalogWidgetEnabled,
  preserveOtherSurfaceWidgets,
  synchronizeWidgetMembership,
} from "../src/widget-membership.ts";

function fixture() {
  return {
    widgets: { pedals: true, speed_gear_rpm: true },
    extra_widgets: { fuel: { enabled: true, layout: { x: 5 } } },
    overlays: [
      { id: "main", widgets: ["inputs", "telemetry", "fuel"] },
      { id: "second", widgets: ["inputs", "fuel"] },
    ],
  };
}

test("bulk disable hides only the active surface and retains shared render flags", () => {
  const original = fixture();
  const next = setCatalogWidgets(
    original,
    "main",
    ["inputs", "telemetry", "fuel"],
    false,
  );
  assert.deepEqual(next.overlays[0].widgets, []);
  assert.deepEqual(next.overlays[1], original.overlays[1]);
  assert.equal(next.widgets.pedals, true);
  assert.equal(next.widgets.speed_gear_rpm, false);
  assert.equal(next.extra_widgets.fuel.enabled, true);
  assert.equal(catalogWidgetEnabled(next, "main", "fuel"), false);
  assert.equal(catalogWidgetEnabled(next, "second", "fuel"), true);
  assert.deepEqual(original, fixture());
});

test("bulk enable deduplicates membership and ignores unknown widgets and missing surfaces", () => {
  const config = fixture();
  config.overlays[0].widgets = [];
  const next = setCatalogWidgets(
    config,
    "main",
    ["inputs", "inputs", "fuel", "unknown"],
    true,
  );
  assert.deepEqual(next.overlays[0].widgets, ["inputs", "fuel"]);
  assert.equal(catalogWidgetEnabled(next, "main", "inputs"), true);
  assert.equal(setCatalogWidgets(config, "missing", ["inputs"], false), config);
});

test("applying a surface preset preserves shared widgets and removes unshared ones", () => {
  const original = fixture();
  original.widgets.steering = true;
  const next = {
    ...original,
    widgets: { pedals: false, speed_gear_rpm: false, steering: false },
    extra_widgets: {},
    overlays: [{ id: "main", widgets: [] }, original.overlays[1]],
  };
  const result = preserveOtherSurfaceWidgets(original, next, "main");
  assert.equal(result.widgets.pedals, true);
  assert.equal(result.widgets.steering, true);
  assert.equal(result.widgets.speed_gear_rpm, false);
  assert.equal(result.extra_widgets.fuel.enabled, true);
  assert.equal(catalogWidgetEnabled(result, "main", "fuel"), false);
  assert.equal(catalogWidgetEnabled(result, "second", "fuel"), true);
  assert.deepEqual(next.extra_widgets, {});
});

test("membership normalization preserves steering-only and title-only surface choices", () => {
  const config = fixture();
  config.widgets = {
    pedals: false,
    steering: true,
    title: true,
    speed_gear_rpm: false,
  };
  const next = synchronizeWidgetMembership(config);
  assert.equal(next.widgets.pedals, false);
  assert.equal(next.widgets.speed_gear_rpm, false);
  assert.equal(next.widgets.steering, true);
  assert.equal(next.widgets.title, true);
});

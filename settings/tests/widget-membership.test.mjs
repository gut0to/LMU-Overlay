import assert from "node:assert/strict";
import test from "node:test";
import { setCatalogWidgets, catalogWidgetEnabled } from "../src/widget-membership.ts";

function fixture() {
  return { widgets: { pedals: true, speed_gear_rpm: true }, extra_widgets: { fuel: { enabled: true, layout: { x: 5 } } }, overlays: [
    { id: "main", widgets: ["inputs", "telemetry", "fuel"] },
    { id: "second", widgets: ["inputs", "fuel"] },
  ] };
}

test("bulk disable hides only the active surface and retains shared render flags", () => {
  const original = fixture();
  const next = setCatalogWidgets(original, "main", ["inputs", "telemetry", "fuel"], false);
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
  const next = setCatalogWidgets(config, "main", ["inputs", "inputs", "fuel", "unknown"], true);
  assert.deepEqual(next.overlays[0].widgets, ["inputs", "fuel"]);
  assert.equal(catalogWidgetEnabled(next, "main", "inputs"), true);
  assert.equal(setCatalogWidgets(config, "missing", ["inputs"], false), config);
});

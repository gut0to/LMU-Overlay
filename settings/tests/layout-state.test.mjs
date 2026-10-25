import assert from "node:assert/strict";
import test from "node:test";
import { updateOverlayLayoutSelection } from "../src/layout-state.ts";

test("editing one surface preserves shared templates and another surface's inherited layouts", () => {
  const config = {
    layout: { telemetry: { x: 10 } },
    extra_widgets: { fuel: { layout: { x: 20 } } },
    overlays: [
      { id: "main", layout_overrides: {} },
      { id: "second", layout_overrides: {} },
    ],
  };
  const layout = { x: 99 };
  const next = updateOverlayLayoutSelection(
    config,
    "main",
    "telemetry",
    layout,
  );
  layout.x = 200;
  assert.equal(next.overlays[0].layout_overrides.telemetry.x, 99);
  assert.equal(next.layout.telemetry.x, 10);
  assert.equal(next.overlays[1], config.overlays[1]);
  const extra = updateOverlayLayoutSelection(next, "second", "extra:fuel", {
    x: 30,
  });
  assert.equal(extra.extra_widgets.fuel.layout.x, 20);
  assert.equal(extra.overlays[1].layout_overrides.fuel.x, 30);
  assert.equal(
    updateOverlayLayoutSelection(config, "missing", "telemetry", { x: 0 }),
    config,
  );
});

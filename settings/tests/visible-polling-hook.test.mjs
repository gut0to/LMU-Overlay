import assert from "node:assert/strict";
import test from "node:test";
import React from "react";
import { act, create } from "react-test-renderer";
import { useVisiblePolling } from "../src/useVisiblePolling.ts";

test("polling is sequential, pauses when hidden and stops after unmount", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const previous = globalThis.document;
  const document = new EventTarget();
  document.visibilityState = "visible";
  globalThis.document = document;
  const requests = [];
  const poll = () => new Promise((resolve) => requests.push(resolve));
  function Editor() {
    useVisiblePolling(poll, 100);
    return null;
  }
  let renderer;
  try {
    act(() => {
      renderer = create(React.createElement(Editor));
    });
    assert.equal(requests.length, 1);
    act(() => t.mock.timers.tick(1000));
    assert.equal(requests.length, 1);
    await act(async () => {
      requests[0]();
      await Promise.resolve();
    });
    act(() => t.mock.timers.tick(100));
    assert.equal(requests.length, 2);
    document.visibilityState = "hidden";
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    await act(async () => {
      requests[1]();
      await Promise.resolve();
    });
    act(() => t.mock.timers.tick(1000));
    assert.equal(requests.length, 2);
    document.visibilityState = "visible";
    act(() => document.dispatchEvent(new Event("visibilitychange")));
    assert.equal(requests.length, 3);
    act(() => renderer.unmount());
    renderer = null;
    await act(async () => {
      requests[2]();
      await Promise.resolve();
    });
    act(() => t.mock.timers.tick(1000));
    assert.equal(requests.length, 3);
  } finally {
    if (renderer) act(() => renderer.unmount());
    globalThis.document = previous;
  }
});

import assert from "node:assert/strict";
import test from "node:test";
import { StatusRequest } from "../src/status-request.ts";
test("overlapping status checks share the actual result", async () => {
  const request = new StatusRequest();
  let resolve;
  let calls = 0;
  const first = request.run(() => { calls++; return new Promise((done) => { resolve = done; }); });
  const second = request.run(async () => false);
  assert.equal(first, second);
  await Promise.resolve();
  resolve(true);
  assert.equal(await second, true);
  assert.equal(calls, 1);
  assert.equal(await request.run(async () => false), false);
});
test("failed status checks release the shared request", async () => {
  const request = new StatusRequest();
  await assert.rejects(request.run(async () => { throw Error("offline"); }), /offline/);
  assert.equal(await request.run(async () => true), true);
});

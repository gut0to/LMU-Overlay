import assert from "node:assert/strict";
import test from "node:test";
import { ConfigTasks } from "../src/config-tasks.ts";

test("operations execute in the order submitted", async () => {
  const tasks = new ConfigTasks();
  const order = [];
  await Promise.all([1, 2, 3].map((value) => tasks.run(async () => { order.push(value); return value; })));
  assert.deepEqual(order, [1, 2, 3]);
});

test("rejected operations do not block later saves", async () => {
  const tasks = new ConfigTasks();
  const failure = tasks.run(async () => { throw new Error("disk full"); });
  const recovery = tasks.run(async () => "saved");
  await assert.rejects(failure, /disk full/);
  assert.equal(await recovery, "saved");
});

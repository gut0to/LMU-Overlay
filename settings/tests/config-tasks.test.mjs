import assert from "node:assert/strict";
import test from "node:test";
import { ConfigTasks } from "../src/config-tasks.ts";

test("operations execute in the order submitted", async () => {
  const tasks = new ConfigTasks();
  const order = [];
  await Promise.all([1, 2, 3].map((value) => tasks.run(async () => { order.push(value); return value; })));
  assert.deepEqual(order, [1, 2, 3]);
});

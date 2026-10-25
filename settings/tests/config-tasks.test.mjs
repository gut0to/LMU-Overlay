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

test("waiting writes use the revision produced by the previous operation", async () => {
  const tasks = new ConfigTasks();
  let revision = 1;
  const observed = [];
  let release;
  const gate = new Promise((resolve) => { release = resolve; });
  const first = tasks.run(async () => { observed.push(revision); await gate; revision++; });
  const second = tasks.run(async () => { observed.push(revision); revision++; });
  await Promise.resolve();
  assert.deepEqual(observed, [1]);
  release();
  await Promise.all([first, second]);
  assert.deepEqual(observed, [1, 2]);
});

test("queued writes retain all digits in opaque revision tokens", async () => {
  const tasks = new ConfigTasks();
  let revision = "18446744073709551614";
  const observed = [];
  await Promise.all([
    tasks.run(async () => { observed.push(revision); revision = "18446744073709551615"; }),
    tasks.run(async () => { observed.push(revision); }),
  ]);
  assert.deepEqual(observed, ["18446744073709551614", "18446744073709551615"]);
});

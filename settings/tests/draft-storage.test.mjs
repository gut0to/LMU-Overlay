import assert from "node:assert/strict";
import test from "node:test";
import { DraftStore } from "../src/draft-storage.ts";
import { DRAFT_STORAGE_KEY } from "../src/drafts.ts";

test("recovery storage uses one stable key and can clear persisted edits", () => {
  const data = new Map();
  const store = new DraftStore(() => ({
    getItem: (k) => data.get(k) ?? null,
    setItem: (k, v) => data.set(k, v),
    removeItem: (k) => data.delete(k),
  }));
  assert.deepEqual(store.read(), { raw: null, error: null });
  assert.equal(store.write("draft"), null);
  assert.equal(data.get(DRAFT_STORAGE_KEY), "draft");
  assert.equal(store.read().raw, "draft");
  assert.equal(store.clear(), null);
  assert.equal(store.read().raw, null);
});

test("blocked storage access and operation failures provide actionable errors", () => {
  const denied = () => {
    throw new Error("denied");
  };
  for (const store of [
    new DraftStore(denied),
    new DraftStore(() => ({
      getItem: denied,
      setItem: denied,
      removeItem: denied,
    })),
  ]) {
    assert.match(store.read().error, /Save or export/);
    assert.match(store.write("draft"), /Save or export/);
    assert.match(store.clear(), /offered again/);
  }
});

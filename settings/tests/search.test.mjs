import assert from "node:assert/strict";
import test from "node:test";
import { matchesSearch } from "../src/search.ts";
test("all search terms match in any order", () => {
  assert.equal(matchesSearch("Fuel usage per lap", "lap fuel"), true);
  assert.equal(matchesSearch("Fuel usage per lap", "fuel tyres"), false);
  assert.equal(matchesSearch("Fuel", "   "), true);
});

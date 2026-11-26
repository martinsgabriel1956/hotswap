import { test } from "node:test";
import assert from "node:assert/strict";
import { isNewer } from "./update.ts";

test("compara número a número, com ou sem o v da tag", () => {
  assert.equal(isNewer("v0.6.0", "0.5.1"), true);
  assert.equal(isNewer("0.10.0", "0.9.3"), true);
  assert.equal(isNewer("v1.0.0", "0.99.99"), true);
  assert.equal(isNewer("v0.5.1", "0.5.1"), false);
  assert.equal(isNewer("v0.5.0", "0.5.1"), false);
  assert.equal(isNewer("v0.5.1.1", "0.5.1"), true);
});

test("pré-release e tag estranha não avisam", () => {
  assert.equal(isNewer("v0.6.0-beta.1", "0.5.1"), false);
  assert.equal(isNewer("nightly", "0.5.1"), false);
  assert.equal(isNewer("", "0.5.1"), false);
});

test("instalada em pré-release: a final da mesma versão não é mais nova pelo número, e a seguinte é", () => {
  assert.equal(isNewer("v0.6.0", "0.6.0-beta.1"), false);
  assert.equal(isNewer("v0.6.1", "0.6.0-beta.1"), true);
});

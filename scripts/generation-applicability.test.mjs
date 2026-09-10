// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { reviewApplicability, checkApplicabilityReport } from "./generation-applicability.mjs";

function fixture() {
  const areas = ["base-allocation-tailored", "ego-negative", "random-artifact", "fixed-artifact-reward", "use-save"];
  return {
    playableBuildIds: ["build.warrior"], playableRaceIds: ["race.human"],
    builds: [{ id: "build.warrior", classId: "class.warrior" }], classes: [{ id: "class.warrior" }],
    applicability: {
      schemaVersion: 1, sourceRef: "master", sourceCommit: "a".repeat(40), entrySource: "creation.ts",
      conditionScopes: [{ id: "shared", file: "src/object2.c", start: 1, end: 2, status: "implemented", entry: "natural generation" }],
      sharedReviews: areas.map(id => ({ id, conditionIds: ["shared"], entry: "natural generation", reason: "Reviewed shared source path", implementation: ["generator.rs"], tests: ["generation_tests.rs"] })),
      builds: [{ buildId: "build.warrior", areas: areas.map(review => ({ review, status: "no-special-difference", reason: "Default source branch applies" })), gapIds: [] }],
      deferred: [{ status: "deferred", conditionIds: ["shared"], reason: "Scroll entry unavailable", prerequisite: "Actual scroll entry" }], gaps: [],
    },
  };
}
function report(expected) {
  return {
    sourceRef: expected.sourceRef, sourceCommit: expected.sourceCommit, runtimeParityComplete: false,
    currentPlayableSharedGenerationComplete: expected.reviews.every(b => b.complete),
    buildApplicability: { ...structuredClone(expected), sourceConditions: [{ scope: "shared", source: "src/object2.c:1" }] },
  };
}
function addGap(input) {
  input.applicability.builds[0].gapIds.push("consumer");
  input.applicability.gaps.push({ id: "consumer", status: "pending", buildIds: ["build.warrior"], areas: ["random-artifact"], reason: "Consumer evidence pending", requiredEvidence: "Real generated item and save continuation" });
}

test("reviewed builds pass; unavailable systems do not imply a reachable gap", () => {
  const expected = reviewApplicability(fixture());
  assert.equal(expected.reviews[0].complete, true);
  checkApplicabilityReport(report(expected), expected);
});

test("new class and new realm Build both require their own review, identifying Build and Class", () => {
  for (const classId of ["class.new", "class.warrior"]) {
    const input = fixture();
    input.playableBuildIds.push("build.new-realm");
    input.builds.push({ id: "build.new-realm", classId, firstRealmId: "death" });
    if (classId === "class.new") input.classes.push({ id: classId });
    assert.throws(() => reviewApplicability(input), /missing applicability review: Build build.new-realm, Class class\./);
    const record = structuredClone(input.applicability.builds[0]);
    record.buildId = "build.new-realm";
    input.applicability.builds.push(record);
    assert.equal(reviewApplicability(input).resolvedBuilds[1].firstRealmId, "death");
  }
});

test("missing definitions, duplicate reviews, dangling conditions and unavailable playable identities fail", () => {
  for (const [mutate, message] of [
    [i => i.builds.pop(), /missing formal Build build.warrior/],
    [i => i.classes.pop(), /missing formal Class class.warrior/],
    [i => i.applicability.builds.push(i.applicability.builds[0]), /duplicate build.warrior/],
    [i => i.applicability.sharedReviews[0].conditionIds.push("missing"), /unknown missing/],
    [i => i.applicability.conditionScopes[0].unavailableClassIds = ["class.warrior"], /unavailable identity is playable: class.warrior/],
    [i => i.applicability.conditionScopes[0].unavailableRaceIds = ["race.human"], /unavailable identity is playable: race.human/],
    [i => Object.assign(i.applicability.conditionScopes[0], { status: "deferred-unavailable-build", prerequisite: "Real class entry" }), /unavailable build requires explicit Class\/Race IDs/],
  ]) {
    const input = fixture(); mutate(input);
    assert.throws(() => reviewApplicability(input), message);
  }
});

test("a label without reasons, entry or implementation/test evidence does not close review", () => {
  for (const [mutate, message] of [
    [i => i.applicability.builds[0].areas[0].reason = " ", /reason: required text/],
    [i => i.applicability.sharedReviews[0].entry = "", /entry: required text/],
    [i => i.applicability.sharedReviews[0].implementation = [], /implementation references required/],
    [i => i.applicability.sharedReviews[0].tests = [], /tests references required/],
    [i => i.applicability.builds[0].areas.pop(), /five generation areas required/],
  ]) {
    const input = fixture(); mutate(input);
    assert.throws(() => reviewApplicability(input), message);
  }
});

test("reachable deferred work requires a linked gap and cannot claim build or overall completion", () => {
  const input = fixture();
  Object.assign(input.applicability.builds[0].areas[2], { status: "deferred", prerequisite: "Implement actual consumer" });
  assert.throws(() => reviewApplicability(input), /reachable deferred area needs a gap/);
  addGap(input);
  const expected = reviewApplicability(input);
  assert.equal(expected.reviews[0].complete, false);
  checkApplicabilityReport(report(expected), expected);
  const stale = report(expected);
  stale.currentPlayableSharedGenerationComplete = true;
  assert.throws(() => checkApplicabilityReport(stale, expected), /completion contradicts/);
  const staleBuild = report(expected);
  staleBuild.buildApplicability.reviews[0].complete = true;
  assert.throws(() => checkApplicabilityReport(staleBuild, expected), /stale applicability report: reviews/);
  input.applicability.builds[0].complete = true;
  assert.throws(() => reviewApplicability(input), /completion contradicts gaps/);
});

test("gaps must reference the same actual builds and areas in both directions", () => {
  for (const [mutate, message] of [
    [i => i.applicability.builds[0].gapIds.pop(), /missing gap reference consumer/],
    [i => i.applicability.gaps[0].buildIds = ["build.missing"], /unknown build.missing/],
    [i => i.applicability.gaps[0].areas = ["missing"], /unknown missing/],
    [i => i.applicability.gaps[0].requiredEvidence = "", /requiredEvidence: required text/],
  ]) {
    const input = fixture(); addGap(input); mutate(input);
    assert.throws(() => reviewApplicability(input), message);
  }
});

test("changed review, formal realm, class or report source cannot pass a stale report", () => {
  for (const [mutate, message] of [
    [i => i.applicability.builds[0].areas[0].reason += " revised", /stale applicability report: reviews/],
    [i => i.builds[0].firstRealmId = "death", /stale applicability report: resolvedBuilds/],
    [i => { i.classes.push({ id: "class.new" }); i.builds[0].classId = "class.new"; }, /stale applicability report: playableClasses/],
    [i => i.applicability.sourceCommit = "b".repeat(40), /stale applicability report: sourceCommit/],
  ]) {
    const input = fixture(); const old = report(reviewApplicability(input)); mutate(input);
    assert.throws(() => checkApplicabilityReport(old, reviewApplicability(input)), message);
  }
});

test("real CLI checks committed data without PATH tools or file writes", () => {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const files = ["design/generation-build-applicability.json", "design/ego-contract-audit.json"];
  const before = files.map(f => readFileSync(path.join(root, f)));
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => key.toLowerCase() !== "path"));
  const output = execFileSync(process.execPath, [path.join(root, "scripts/audit-egos.mjs"), "--check-applicability"], { cwd: root, env: { ...env, PATH: "" }, encoding: "utf8" });
  assert.match(output, /Applicability check passed/);
  files.forEach((f, i) => assert.deepEqual(readFileSync(path.join(root, f)), before[i], `${f} changed`));
});

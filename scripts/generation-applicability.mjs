// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";

const AREAS = ["base-allocation-tailored", "ego-negative", "random-artifact", "fixed-artifact-reward", "use-save"];
const INPUT = "design/generation-build-applicability.json";
const REPORT = "design/ego-contract-audit.json";
const text = (value, label) => assert.ok(typeof value === "string" && value.trim(), `${label}: required text`);
function unique(records, key, label) {
  const result = new Map();
  for (const record of records) {
    text(record[key], `${label}.${key}`);
    assert.ok(!result.has(record[key]), `${label}: duplicate ${record[key]}`);
    result.set(record[key], record);
  }
  return result;
}
function references(ids, records, label) {
  assert.ok(Array.isArray(ids), `${label}: expected ID array`);
  assert.equal(new Set(ids).size, ids.length, `${label}: duplicate reference`);
  for (const id of ids) assert.ok(records.has(id), `${label}: unknown ${id}`);
}

// Pure validation is shared by the read-only CI command, source audit and tests.
export function reviewApplicability({ applicability: a, builds, classes, playableBuildIds, playableRaceIds }) {
  assert.equal(a.schemaVersion, 1, "applicability schemaVersion");
  assert.equal(a.sourceRef, "master", "applicability sourceRef");
  assert.match(a.sourceCommit, /^[a-f0-9]{40}$/, "applicability sourceCommit");
  text(a.entrySource, "entrySource");
  const formalBuilds = unique(builds, "id", "formal builds");
  const formalClasses = unique(classes, "id", "formal classes");
  const records = unique(a.builds, "buildId", "build reviews");
  const scopes = unique(a.conditionScopes, "id", "condition scopes");
  const shared = unique(a.sharedReviews, "id", "shared reviews");
  const gaps = unique(a.gaps, "id", "gaps");
  assert.deepEqual([...shared.keys()].sort(), [...AREAS].sort(), "five generation review areas required");
  assert.equal(new Set(playableBuildIds).size, playableBuildIds.length, "duplicate creation Build ID");
  const resolvedBuilds = playableBuildIds.map(id => {
    const build = formalBuilds.get(id);
    assert.ok(build, `missing formal Build ${id}`);
    assert.ok(formalClasses.has(build.classId), `${id}: missing formal Class ${build.classId}`);
    assert.ok(records.has(id), `missing applicability review: Build ${id}, Class ${build.classId}`);
    return { buildId: id, classId: build.classId, firstRealmId: build.firstRealmId ?? null, secondRealmId: build.secondRealmId ?? null };
  });
  for (const id of records.keys()) assert.ok(playableBuildIds.includes(id), `review has no creation entry: ${id}`);
  const playableClasses = [...new Set(resolvedBuilds.map(b => b.classId))].sort();
  for (const scope of scopes.values()) {
    text(scope.file, `${scope.id}.file`);
    assert.ok(Number.isInteger(scope.start) && scope.start > 0 && Number.isInteger(scope.end) && scope.end >= scope.start, `${scope.id}: invalid source range`);
    text(scope.entry, `${scope.id}.entry`);
    text(scope.status, `${scope.id}.status`);
    if (scope.status.startsWith("deferred")) text(scope.prerequisite, `${scope.id}.prerequisite`);
    if (scope.status === "deferred-unavailable-build") {
      assert.ok(scope.unavailableClassIds?.length || scope.unavailableRaceIds?.length, `${scope.id}: unavailable build requires explicit Class/Race IDs`);
    }
    // These fields encode unavailable identities; prose is not parsed as policy.
    for (const [field, available] of [["unavailableClassIds", playableClasses], ["unavailableRaceIds", playableRaceIds]]) {
      if (scope[field] === undefined) continue;
      assert.ok(Array.isArray(scope[field]), `${scope.id}.${field}: expected ID array`);
      for (const id of scope[field]) {
        text(id, `${scope.id}.${field}`);
        assert.ok(!available.includes(id), `${scope.id}: unavailable identity is playable: ${id}`);
      }
    }
  }
  for (const review of shared.values()) {
    references(review.conditionIds, scopes, review.id);
    assert.ok(review.conditionIds.length, `${review.id}: source conditions required`);
    for (const key of ["entry", "reason"]) text(review[key], `${review.id}.${key}`);
    for (const key of ["implementation", "tests"]) {
      assert.ok(Array.isArray(review[key]) && review[key].length, `${review.id}: ${key} references required`);
      for (const ref of review[key]) text(ref, `${review.id}.${key}`);
    }
  }
  for (const deferred of a.deferred) {
    assert.equal(deferred.status, "deferred", "unavailable dependency status");
    references(deferred.conditionIds, scopes, "deferred conditions");
    text(deferred.reason, "deferred reason");
    text(deferred.prerequisite, "deferred prerequisite");
  }
  for (const gap of gaps.values()) {
    assert.equal(gap.status, "pending", `${gap.id}: resolve evidence and remove closed gaps`);
    references(gap.buildIds, records, gap.id);
    references(gap.areas, shared, gap.id);
    assert.ok(gap.buildIds.length && gap.areas.length, `${gap.id}: affected builds and areas required`);
    text(gap.reason, `${gap.id}.reason`);
    text(gap.requiredEvidence, `${gap.id}.requiredEvidence`);
    for (const id of gap.buildIds) assert.ok(records.get(id).gapIds.includes(gap.id), `${id}: missing gap reference ${gap.id}`);
  }
  const reviews = playableBuildIds.map(id => {
    const build = records.get(id);
    const areas = unique(build.areas, "review", id);
    assert.deepEqual([...areas.keys()].sort(), [...AREAS].sort(), `${id}: five generation areas required`);
    references(build.gapIds, gaps, id);
    for (const gapId of build.gapIds) assert.ok(gaps.get(gapId).buildIds.includes(id), `${id}: unrelated gap ${gapId}`);
    for (const area of areas.values()) {
      assert.ok(["implemented", "no-special-difference", "deferred"].includes(area.status), `${id}/${area.review}: unknown status ${area.status}`);
      text(area.reason, `${id}/${area.review}.reason`);
      if (area.status === "deferred") {
        text(area.prerequisite, `${id}/${area.review}.prerequisite`);
        assert.ok(build.gapIds.some(g => gaps.get(g).areas.includes(area.review)), `${id}/${area.review}: reachable deferred area needs a gap`);
      }
    }
    const complete = build.gapIds.length === 0;
    if (Object.hasOwn(build, "complete")) assert.equal(build.complete, complete, `${id}: completion contradicts gaps`);
    return { ...build, complete };
  });
  return {
    entrySource: a.entrySource, reviewInput: INPUT, sourceRef: a.sourceRef, sourceCommit: a.sourceCommit,
    playableBuilds: playableBuildIds, playableRaces: playableRaceIds, playableClasses, resolvedBuilds,
    reviews, sharedReviews: a.sharedReviews, deferred: a.deferred, gaps: a.gaps, conditionScopes: a.conditionScopes,
  };
}

export function checkApplicabilityReport(report, expected) {
  const hint = "regenerate design/ego-contract-audit.json with the authoritative source audit";
  for (const [key, value] of Object.entries(expected)) assert.deepEqual(report.buildApplicability?.[key], value, `stale applicability report: ${key}; ${hint}`);
  assert.equal(report.sourceCommit, expected.sourceCommit, `stale source commit; ${hint}`);
  assert.equal(report.sourceRef, expected.sourceRef, `stale source ref; ${hint}`);
  assert.equal(report.currentPlayableSharedGenerationComplete, expected.reviews.every(b => b.complete), "report completion contradicts current build gaps");
  assert.equal(report.runtimeParityComplete, false, "full-source parity remains incomplete");
  for (const condition of report.buildApplicability.sourceConditions) {
    const scope = expected.conditionScopes.find(s => s.id === condition.scope);
    const [file, line] = condition.source.split(":");
    assert.ok(scope && scope.file === file && Number(line) >= scope.start && Number(line) <= scope.end, `stale source condition ${condition.source}; ${hint}`);
  }
}

export async function loadApplicability(root) {
  const json = async file => JSON.parse(await readFile(path.join(root, file), "utf8"));
  async function definitions(folder) {
    const dir = `packs/rfb-demo-original/${folder}`;
    return Promise.all((await readdir(path.join(root, dir))).filter(n => n.endsWith(".json")).map(n => json(`${dir}/${n}`)));
  }
  const [applicability, builds, classes, entry] = await Promise.all([
    json(INPUT), definitions("builds"), definitions("classes"), import(pathToFileURL(path.join(root, "web/src/character-creation.ts"))),
  ]);
  const expected = reviewApplicability({ applicability, builds, classes, playableBuildIds: entry.PLAYTEST_BUILD_IDS, playableRaceIds: entry.PLAYTEST_RACE_IDS });
  const refs = [...applicability.sharedReviews, ...applicability.builds.flatMap(b => b.areas)].flatMap(r => [...(r.implementation ?? []), ...(r.tests ?? [])]);
  await Promise.all([...new Set(refs)].map(async ref => {
    const [file, symbol] = ref.split("#");
    const target = path.resolve(root, file);
    await stat(target);
    if (symbol) assert.ok((await readFile(target, "utf8")).includes(symbol), `missing evidence symbol: ${ref}`);
  }));
  return expected;
}

export async function checkApplicability(root) {
  const expected = await loadApplicability(root);
  checkApplicabilityReport(JSON.parse(await readFile(path.join(root, REPORT), "utf8")), expected);
  return expected;
}

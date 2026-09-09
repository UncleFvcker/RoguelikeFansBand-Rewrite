// SPDX-License-Identifier: MPL-2.0

import assert from "node:assert/strict";
import { mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";

// Optional architecture experiment; independent of the player scenario.
export async function runRendererProfile(driver, artifactDirectory) {
  const renderProfilePath = path.join(artifactDirectory, "render-profile.json");
  await rm(renderProfilePath, { force: true });
  await driver.waitFor(
    `return document.documentElement.dataset.appMode === "title"`,
    "title session shell",
    60_000,
  );
  await driver.execute(`
    localStorage.setItem("rfb.renderer-profile-enabled", "1");
    setTimeout(() => window.location.reload(), 250);
    return true;
  `);
  try {
    await driver.waitFor(
      `return performance.getEntriesByType("navigation")[0]?.type === "reload" && typeof window.__rfbRunRendererProfile === "function"`,
      "renderer profile hook",
      60_000,
    );
    await driver.execute(`
      window.__rfbRunRendererProfile().catch(() => undefined);
      return true;
    `);
    await driver.waitFor(
      `return document.documentElement.dataset.rendererProfileState === "complete" || document.documentElement.dataset.rendererProfileState === "error"`,
      "large-map renderer profile",
      120_000,
    );
    const profileState = await driver.execute(`
      return {
        state: document.documentElement.dataset.rendererProfileState,
        error: window.__rfbRendererProfileError,
        report: window.__rfbRendererProfileResult,
      };
    `);
    assert.equal(profileState.state, "complete", profileState.error);
    const profile = profileState.report;
    assert.equal(profile.schemaVersion, 1);
    assert.equal(profile.scenarioId, "rfb-render-profile-large-original-v1");
    assert.equal(profile.rendererBackend, "pixi-layered-chunks-v3");
    assert.equal(profile.dynamicViewMode, "visible-chunk-reuse-v1");
    assert.equal(profile.width, 192);
    assert.equal(profile.height, 64);
    assert.equal(profile.cellCount, 12_288);
    assert.equal(profile.dynamicUpdateCellCount, 256);
    assert.equal(profile.terrainUpdateCellCount, 96);
    assert.equal(profile.estimatedFullMapDynamicDisplayObjectCount, 86_016);
    assert.equal(profile.recommendation, "retain-visible-chunk-dynamic-views");
    assert.deepEqual(profile.runs.map((run) => run.chunkSize), [8, 16, 32]);
    assert.deepEqual(
      profile.runs.map((run) => run.diagnostics.terrainChunkCount),
      [192, 48, 12],
    );
    assert.deepEqual(
      profile.runs.map((run) => run.diagnostics.activeDynamicChunkCount),
      [16, 4, 4],
    );
    assert.deepEqual(
      profile.runs.map((run) => run.diagnostics.cellViewCount),
      [1024, 1024, 4096],
    );
    assert.deepEqual(
      profile.runs.map((run) => run.diagnostics.dynamicDisplayObjectCount),
      [7168, 7168, 28_672],
    );
    for (const run of profile.runs) {
      assert.ok(run.diagnostics.visibleChunkCount > 0);
      assert.ok(run.diagnostics.visibleChunkCount < run.diagnostics.terrainChunkCount);
      assert.equal(
        run.diagnostics.activeDynamicChunkCount,
        run.diagnostics.visibleChunkCount,
      );
      assert.equal(run.diagnostics.pooledDynamicChunkCount, 0);
      assert.ok(
        run.diagnostics.dynamicDisplayObjectCount <
          profile.estimatedFullMapDynamicDisplayObjectCount,
      );
      assert.equal(
        run.diagnostics.lastRebuiltTerrainChunks,
        run.diagnostics.terrainChunkCount,
      );
      assert.ok(
        run.diagnostics.totalRebuiltTerrainChunks >=
          run.diagnostics.terrainChunkCount * 2,
      );
      assert.ok(run.canvasPixelWidth >= 420);
      assert.ok(run.canvasPixelHeight >= 420);
      assert.equal(run.frameTiming.sampleCount, 45);
      for (const timing of [
        run.initializeMs,
        run.initialCameraMs,
        run.initialSnapshotMs,
        run.cameraSweepMs,
        run.dynamicUpdateMs,
        run.terrainUpdateMs,
        run.tilesetSwitchMs,
        run.frameTiming.medianMs,
        run.frameTiming.p95Ms,
        run.frameTiming.maxMs,
      ]) {
        assert.ok(Number.isFinite(timing) && timing >= 0);
      }
    }
    await mkdir(artifactDirectory, { recursive: true });
    await writeFile(renderProfilePath, `${JSON.stringify(profile, null, 2)}\n`);
  } finally {
    await driver.execute(`
      localStorage.removeItem("rfb.renderer-profile-enabled");
      delete window.__rfbRunRendererProfile;
      return true;
    `);
  }
}

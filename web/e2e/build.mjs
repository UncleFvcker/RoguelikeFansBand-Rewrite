// SPDX-License-Identifier: MPL-2.0

import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const webDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = path.resolve(webDirectory, "..");
const tauriCli = path.join(
  webDirectory,
  "node_modules",
  "@tauri-apps",
  "cli",
  "tauri.js",
);

const child = spawn(
  process.execPath,
  [tauriCli, "build", "--debug", "--no-bundle", "--features", "webdriver"],
  {
    cwd: webDirectory,
    env: {
      ...process.env,
      CARGO_TARGET_DIR: path.join(repositoryDirectory, "target", "e2e"),
    },
    stdio: "inherit",
  },
);

child.once("error", (error) => {
  process.stderr.write(`${String(error)}\n`);
  process.exitCode = 1;
});
child.once("exit", (code, signal) => {
  if (signal) {
    process.stderr.write(`Tauri E2E build terminated by ${signal}.\n`);
    process.exitCode = 1;
    return;
  }
  process.exitCode = code ?? 1;
});

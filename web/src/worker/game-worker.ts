/// <reference lib="webworker" />
// SPDX-License-Identifier: MPL-2.0

import { decode, encode } from "@msgpack/msgpack";
import initWasm, { WasmGame } from "../wasm/rfb_wasm";

import type { GameSnapshot, GameUpdate } from "../protocol";
import type { WorkerRequest, WorkerResponse } from "../worker-messages";

const worker = self as DedicatedWorkerGlobalScope;
let game: WasmGame | undefined;
let wasmReady: Promise<unknown> | undefined;

worker.addEventListener("message", (event: MessageEvent<WorkerRequest>) => {
  void handleRequest(event.data);
});

async function handleRequest(request: WorkerRequest): Promise<void> {
  try {
    switch (request.type) {
      case "initialize": {
        await ensureWasm();
        game?.free();
        game = new WasmGame(request.seed, request.createdAt);
        const snapshot = decode(game.snapshot()) as GameSnapshot;
        post({ requestId: request.requestId, ok: true, type: "snapshot", payload: snapshot });
        break;
      }
      case "command": {
        const core = requireGame();
        const command = encode({
          commandSeq: request.commandSeq,
          expectedRevision: request.expectedRevision,
          command: request.command,
        });
        const update = decode(core.dispatch(command)) as GameUpdate;
        post({ requestId: request.requestId, ok: true, type: "update", payload: update });
        break;
      }
      case "save": {
        const bytes = new Uint8Array(requireGame().save(request.savedAt));
        const copy = bytes.slice().buffer;
        post({ requestId: request.requestId, ok: true, type: "save", payload: copy }, [copy]);
        break;
      }
      case "load": {
        const snapshot = decode(requireGame().load(new Uint8Array(request.data))) as GameSnapshot;
        post({ requestId: request.requestId, ok: true, type: "snapshot", payload: snapshot });
        break;
      }
    }
  } catch (error) {
    post({
      requestId: request.requestId,
      ok: false,
      type: "error",
      message: error instanceof Error ? error.message : String(error),
    });
  }
}

function ensureWasm(): Promise<unknown> {
  wasmReady ??= initWasm();
  return wasmReady;
}

function requireGame(): WasmGame {
  if (!game) {
    throw new Error("Rust core has not been initialized");
  }
  return game;
}

function post(response: WorkerResponse, transfer: Transferable[] = []): void {
  worker.postMessage(response, transfer);
}


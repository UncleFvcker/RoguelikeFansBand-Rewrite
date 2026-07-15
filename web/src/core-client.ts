// SPDX-License-Identifier: MPL-2.0

import type { GameCommand, GameSnapshot, GameUpdate } from "./protocol";
import type { WorkerRequest, WorkerResponse } from "./worker-messages";

interface PendingRequest {
  resolve: (response: WorkerResponse) => void;
  reject: (error: Error) => void;
}

type WithoutRequestId<T> = T extends WorkerRequest ? Omit<T, "requestId"> : never;
type WorkerRequestPayload = WithoutRequestId<WorkerRequest>;

export class CoreClient {
  readonly #worker = new Worker(new URL("./worker/game-worker.ts", import.meta.url), {
    type: "module",
  });
  readonly #pending = new Map<number, PendingRequest>();
  #requestId = 0;
  #revision = 0;
  #commandSeq = 0;

  constructor() {
    this.#worker.addEventListener("message", (event: MessageEvent<WorkerResponse>) => {
      const response = event.data;
      const pending = this.#pending.get(response.requestId);
      if (!pending) return;
      this.#pending.delete(response.requestId);
      if (response.ok) {
        pending.resolve(response);
      } else {
        pending.reject(new Error(response.message));
      }
    });
    this.#worker.addEventListener("error", (event) => {
      const error = new Error(event.message || "Web Worker crashed");
      for (const pending of this.#pending.values()) pending.reject(error);
      this.#pending.clear();
    });
  }

  async initialize(seed: string): Promise<GameSnapshot> {
    const response = await this.#send({
      type: "initialize",
      seed,
      createdAt: new Date().toISOString(),
    });
    if (response.type !== "snapshot") throw new Error("Unexpected initialize response");
    this.#syncSnapshot(response.payload);
    return response.payload;
  }

  async dispatch(command: GameCommand): Promise<GameUpdate> {
    const nextSeq = this.#commandSeq + 1;
    const response = await this.#send({
      type: "command",
      commandSeq: nextSeq,
      expectedRevision: this.#revision,
      command,
    });
    if (response.type !== "update") throw new Error("Unexpected command response");
    this.#revision = response.payload.revision;
    this.#commandSeq = response.payload.commandSeq;
    return response.payload;
  }

  async save(): Promise<Uint8Array> {
    const response = await this.#send({ type: "save", savedAt: new Date().toISOString() });
    if (response.type !== "save") throw new Error("Unexpected save response");
    return new Uint8Array(response.payload);
  }

  async load(data: Uint8Array): Promise<GameSnapshot> {
    const copy = data.slice().buffer;
    const response = await this.#send({ type: "load", data: copy }, [copy]);
    if (response.type !== "snapshot") throw new Error("Unexpected load response");
    this.#syncSnapshot(response.payload);
    return response.payload;
  }

  dispose(): void {
    this.#worker.terminate();
    this.#pending.clear();
  }

  #syncSnapshot(snapshot: GameSnapshot): void {
    this.#revision = snapshot.revision;
    this.#commandSeq = snapshot.lastCommandSeq;
  }

  #send(
    request: WorkerRequestPayload,
    transfer: Transferable[] = [],
  ): Promise<WorkerResponse> {
    const requestId = ++this.#requestId;
    return new Promise((resolve, reject) => {
      this.#pending.set(requestId, { resolve, reject });
      this.#worker.postMessage({ ...request, requestId } as WorkerRequest, transfer);
    });
  }
}

// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type {
  GameCommand,
  GameEventDto,
  GameSnapshot,
  GameUpdate,
  TaskServiceDto,
  TaskStatusDto,
  TaskStatusKindDto,
} from "./protocol";

export type TaskServiceAction = "accept" | "claim";

interface TaskServiceDom {
  readonly dialog: HTMLDialogElement;
  readonly title: HTMLElement;
  readonly description: HTMLElement;
  readonly owner: HTMLElement;
  readonly close: HTMLButtonElement;
  readonly list: HTMLUListElement;
  readonly feedback: HTMLElement;
}

export class TaskServicePanel {
  readonly #state: AppState;
  readonly #localization: Localization;
  readonly #dispatch: (command: GameCommand) => Promise<void>;
  readonly #formatEvent: (event: GameEventDto) => string;
  readonly #beforeOpen: () => void;
  readonly #dom: TaskServiceDom;
  #service: TaskServiceDto | undefined;
  #dismissedServiceId: string | undefined;
  #feedback: GameEventDto | undefined;
  #installed = false;

  constructor(options: {
    document: Document;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
    formatEvent: (event: GameEventDto) => string;
    beforeOpen: () => void;
  }) {
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#formatEvent = options.formatEvent;
    this.#beforeOpen = options.beforeOpen;
    this.#dom = createTaskServiceDom(options.document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.dialog.addEventListener("close", this.#closed);
    this.#dom.list.addEventListener("click", this.#performAction);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.close.removeEventListener("click", this.#close);
    this.#dom.dialog.removeEventListener("close", this.#closed);
    this.#dom.list.removeEventListener("click", this.#performAction);
  }

  render(state: GameSnapshot | GameUpdate): void {
    const event = lastTaskServiceEvent(state);
    if (event) this.#feedback = event;
    const service = state.taskServices.find((candidate) => candidate.playerAtEntrance);
    if (!service) {
      this.reset();
      return;
    }
    const changed = this.#service?.id !== service.id;
    this.#service = service;
    if (changed) this.#feedback = undefined;
    this.#renderPanel();
    if (!this.#dom.dialog.open && this.#dismissedServiceId !== service.id) {
      this.#beforeOpen();
      this.#dom.dialog.showModal();
      this.#dom.list.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    }
  }

  localize(): void {
    if (this.#service) this.#renderPanel();
  }

  updateActions(): void {
    if (this.#service) this.#renderTasks();
  }

  reset(): void {
    this.#service = undefined;
    this.#dismissedServiceId = undefined;
    this.#feedback = undefined;
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  readonly #close = (): void => {
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  };

  readonly #closed = (): void => {
    if (this.#service?.playerAtEntrance) this.#dismissedServiceId = this.#service.id;
  };

  readonly #performAction = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const button = target.closest<HTMLButtonElement>("[data-task-action]");
    const service = this.#service;
    const taskId = button?.dataset.taskId;
    const action = button?.dataset.taskAction as TaskServiceAction | undefined;
    if (!button || button.disabled || !service || !taskId || !action || this.#state.busy) return;
    this.#feedback = undefined;
    const command: GameCommand = action === "accept"
      ? { type: "accept-task", facilityId: service.id, taskId }
      : { type: "claim-task-reward", facilityId: service.id, taskId };
    void this.#dispatch(command);
  };

  #renderPanel(): void {
    const service = this.#service;
    if (!service) return;
    this.#dom.title.textContent = this.#localization.format(service.nameKey);
    this.#dom.description.textContent = this.#localization.format(service.descriptionKey);
    this.#dom.owner.textContent = this.#localization.format("task-service-owner", {
      owner: this.#localization.format(service.ownerNameKey),
    });
    this.#renderTasks();
    this.#dom.feedback.textContent = this.#feedback ? this.#formatEvent(this.#feedback) : "";
    this.#dom.feedback.dataset.kind = this.#feedback?.kind.endsWith("unavailable")
      ? "error"
      : this.#feedback
        ? "success"
        : "none";
  }

  #renderTasks(): void {
    const tasks = this.#service?.tasks ?? [];
    this.#dom.list.replaceChildren();
    if (tasks.length === 0) {
      const empty = this.#dom.list.ownerDocument.createElement("li");
      empty.className = "task-service-empty";
      empty.textContent = this.#localization.format("task-service-empty");
      this.#dom.list.append(empty);
      return;
    }
    for (const task of tasks) this.#dom.list.append(this.#taskRow(task));
  }

  #taskRow(task: TaskStatusDto): HTMLLIElement {
    const document = this.#dom.list.ownerDocument;
    const row = document.createElement("li");
    row.className = "task-service-row";
    const copy = document.createElement("div");
    copy.className = "task-service-copy";
    const heading = document.createElement("div");
    heading.className = "task-service-heading";
    const name = document.createElement("h3");
    name.textContent = this.#localization.format(task.nameKey);
    const status = document.createElement("span");
    status.className = "task-service-status";
    status.dataset.status = task.status;
    status.textContent = this.#localization.format(`task-status-${task.status}`);
    heading.append(name, status);
    copy.append(heading);
    if (task.descriptionKey) {
      const description = document.createElement("p");
      description.textContent = this.#localization.format(task.descriptionKey);
      copy.append(description);
    }
    if (task.status === "taken" || task.status === "active") {
      const progress = document.createElement("p");
      progress.className = "task-service-progress";
      progress.textContent = this.#localization.format("task-service-progress", {
        current: task.current,
        required: task.required,
      });
      copy.append(progress);
    }
    row.append(copy);
    const action = taskActionForStatus(task.status);
    if (action) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "primary-button task-service-action";
      button.dataset.taskAction = action;
      button.dataset.taskId = task.taskId;
      button.disabled = this.#state.busy;
      button.textContent = this.#localization.format(
        action === "accept" ? "action-task-accept" : "action-task-claim",
      );
      row.append(button);
    }
    return row;
  }
}

export function taskActionForStatus(
  status: TaskStatusKindDto,
): TaskServiceAction | undefined {
  if (status === "available") return "accept";
  if (status === "reward-available") return "claim";
  return undefined;
}

function lastTaskServiceEvent(state: GameSnapshot | GameUpdate): GameEventDto | undefined {
  if (!("events" in state)) return undefined;
  for (let index = state.events.length - 1; index >= 0; index -= 1) {
    const event = state.events[index];
    if (
      event?.kind === "task.accepted" ||
      event?.kind === "task.accept-unavailable" ||
      event?.kind === "task.rewarded" ||
      event?.kind === "task.reward-claim-unavailable"
    ) {
      return event;
    }
  }
  return undefined;
}

function createTaskServiceDom(document: Document): TaskServiceDom {
  const element = <T extends HTMLElement>(id: string): T => {
    const found = document.getElementById(id);
    if (!found) throw new Error(`Missing element #${id}`);
    return found as T;
  };
  return {
    dialog: element("task-service-dialog"),
    title: element("task-service-title"),
    description: element("task-service-description"),
    owner: element("task-service-owner"),
    close: element("task-service-close"),
    list: element("task-service-list"),
    feedback: element("task-service-feedback"),
  };
}

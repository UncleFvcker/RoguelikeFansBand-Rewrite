// SPDX-License-Identifier: MPL-2.0

import type { AppState } from "./app-state";
import type { Localization } from "./localization";
import type {
  GameCommand,
  CasinoActionDto,
  CasinoGameDto,
  GameEventDto,
  GameSnapshot,
  GameUpdate,
  BountyOfficeActionDto,
  BountyMissionStatusDto,
  FacilityMembershipDto,
  FacilityServiceKindDto,
  ItemIdentificationDto,
  ResearchMonsterDto,
  TaskServiceDto,
  TaskStatusDto,
  TaskStatusKindDto,
} from "./protocol";

export type TaskServiceAction = "accept" | "claim";

export function filterResearchMonsters(monsters: readonly ResearchMonsterDto[], name: string, glyph: string,
  group: string, displayName: (monster: ResearchMonsterDto) => string): ResearchMonsterDto[] {
  const query = name.trim().toLocaleLowerCase();
  return monsters.filter((monster) => (!query || displayName(monster).toLocaleLowerCase().includes(query))
    && (!glyph || monster.glyph === glyph)
    && (group === "all" || (group === "unique" ? monster.unique : !monster.unique)));
}

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
  readonly #visibleItemName: (displayNameKey: string, kindId: string, artifactName?: string | null) => string;
  readonly #contentName: (id: string) => string;
  readonly #statusName: (id: string) => string;
  readonly #beforeOpen: () => void;
  readonly #dom: TaskServiceDom;
  #service: TaskServiceDto | undefined;
  #dismissedServiceId: string | undefined;
  #feedback: GameEventDto | undefined;
  #overviewVisible = false;
  #installed = false;
  #monsterKindId = "";
  #monsterName = "";
  #monsterGlyph = "";
  #monsterGroup = "all";
  #teleportDungeonId = "";
  #teleportDepth = "";

  constructor(options: {
    document: Document;
    state: AppState;
    localization: Localization;
    dispatch: (command: GameCommand) => Promise<void>;
    formatEvent: (event: GameEventDto) => string;
    visibleItemName: (displayNameKey: string, kindId: string, artifactName?: string | null) => string;
    contentName: (id: string) => string;
    statusName: (id: string) => string;
    beforeOpen: () => void;
  }) {
    this.#state = options.state;
    this.#localization = options.localization;
    this.#dispatch = options.dispatch;
    this.#formatEvent = options.formatEvent;
    this.#visibleItemName = options.visibleItemName;
    this.#contentName = options.contentName;
    this.#statusName = options.statusName;
    this.#beforeOpen = options.beforeOpen;
    this.#dom = createTaskServiceDom(options.document);
  }

  install(): void {
    if (this.#installed) return;
    this.#installed = true;
    this.#dom.close.addEventListener("click", this.#close);
    this.#dom.dialog.addEventListener("close", this.#closed);
    this.#dom.dialog.addEventListener("cancel", this.#cancel);
    this.#dom.list.addEventListener("click", this.#performAction);
  }

  dispose(): void {
    if (!this.#installed) return;
    this.#installed = false;
    this.#dom.close.removeEventListener("click", this.#close);
    this.#dom.dialog.removeEventListener("close", this.#closed);
    this.#dom.dialog.removeEventListener("cancel", this.#cancel);
    this.#dom.list.removeEventListener("click", this.#performAction);
  }

  render(state: GameSnapshot | GameUpdate): void {
    const event = lastTaskServiceEvent(state);
    if (event) {
      this.#feedback = event;
      this.#overviewVisible = false;
    }
    const service = state.taskServices.find((candidate) => candidate.playerAtEntrance);
    if (!service) {
      this.reset();
      return;
    }
    const changed = this.#service?.id !== service.id;
    this.#service = service;
    if (changed) {
      this.#teleportDungeonId = "";
      this.#teleportDepth = "";
      this.#feedback = undefined;
      this.#overviewVisible = false;
    }
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
    this.#overviewVisible = false;
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  }

  readonly #cancel = (event: Event): void => {
    const session = this.#service?.casino?.session;
    if (session && (session.round.type !== "finished" || this.#state.busy)) event.preventDefault();
  };

  readonly #close = (): void => {
    const session = this.#service?.casino?.session;
    if (session && (session.round.type !== "finished" || this.#state.busy)) return;
    if (this.#dom.dialog.open) this.#dom.dialog.close();
  };

  readonly #closed = (): void => {
    if (this.#service?.casino?.session?.round.type === "finished") {
      void this.#dispatch({ type: "casino", facilityId: this.#service.id, action: { type: "leave" } });
    }
    if (this.#service?.playerAtEntrance) this.#dismissedServiceId = this.#service.id;
  };

  readonly #performAction = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const facilityButton = target.closest<HTMLButtonElement>("[data-facility-action]");
    const service = this.#service;
    if (facilityButton && service && !facilityButton.disabled && !this.#state.busy) {
      this.#feedback = undefined;
      const action = facilityButton.dataset.facilityAction;
      this.#overviewVisible = false;
      if (action === "identify" || action === "research") {
        const select = this.#dom.list.querySelector<HTMLSelectElement>(
          `[data-facility-item-action="${action}"]`,
        );
        if (select?.value) {
          void this.#dispatch({
            type: action === "identify" ? "identify-at-facility" : "research-item-at-facility",
            facilityId: service.id,
            itemId: select.value,
          });
        }
      } else if (action === "research-monster") {
        if (this.#monsterKindId) void this.#dispatch({
          type: "research-monster-at-facility", facilityId: service.id, actorKindId: this.#monsterKindId,
        });
      } else if (action === "teleport-level") {
        if (this.#teleportDungeonId && this.#teleportDepth) void this.#dispatch({
          type: "teleport-to-dungeon-level-at-facility", facilityId: service.id,
          dungeonId: this.#teleportDungeonId, depth: Number(this.#teleportDepth),
        });
      } else if (action === "identify-all") {
        void this.#dispatch({
          type: "identify-all-at-facility",
          facilityId: service.id,
        });
      } else if (action === "stay") {
        void this.#dispatch({ type: "stay-at-inn", facilityId: service.id });
      } else if (action === "overview") {
        this.#overviewVisible = true;
        this.#renderPanel();
      } else if (action === "service") {
        const facilityService = facilityButton.dataset.facilityService as
          | FacilityServiceKindDto
          | undefined;
        if (facilityService) {
          const select = this.#dom.list.querySelector<HTMLSelectElement>(
            `select[data-facility-service="${facilityService}"]`,
          );
          const option = select?.selectedOptions[0];
          void this.#dispatch({
            type: "use-facility-service",
            facilityId: service.id,
            service: facilityService,
            itemId: option?.dataset.itemId,
            enchantmentSteps: option ? Number(option.dataset.steps) : undefined,
          });
        }
      } else if (action === "rename") {
        const input = this.#dom.list.querySelector<HTMLInputElement>("[data-player-name]");
        if (input) {
          void this.#dispatch({
            type: "rename-at-facility",
            facilityId: service.id,
            name: input.value,
          });
        }
      }
      return;
    }
    const bountyButton = target.closest<HTMLButtonElement>("[data-bounty-action]");
    if (bountyButton && service && !bountyButton.disabled && !this.#state.busy) {
      const action = bountyButton.dataset.bountyAction as BountyOfficeActionDto | undefined;
      if (action) {
        this.#feedback = undefined;
        void this.#dispatch({
          type: "use-bounty-office",
          facilityId: service.id,
          action,
          itemId: bountyButton.dataset.itemId,
        });
      }
      return;
    }
    const button = target.closest<HTMLButtonElement>("[data-task-action]");
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
      membership: this.#localization.format(facilityMembershipKey(service.membership)),
    });
    this.#renderTasks();
    this.#dom.feedback.textContent = this.#feedback
      ? this.#formatEvent(this.#feedback)
      : this.#overviewVisible && service.overviewMessageKey
        ? this.#localization.format(service.overviewMessageKey)
        : "";
    this.#dom.feedback.dataset.kind = this.#feedback?.kind.endsWith("unavailable")
      ? "error"
      : this.#feedback || this.#overviewVisible
        ? "success"
        : "none";
  }

  #renderTasks(): void {
    const tasks = this.#service?.tasks ?? [];
    this.#dom.list.replaceChildren();
    this.#renderFacilityActions();
    this.#renderCasino();
    if (tasks.length === 0 && this.#dom.list.childElementCount === 0) {
      const empty = this.#dom.list.ownerDocument.createElement("li");
      empty.className = "task-service-empty";
      empty.textContent = this.#localization.format("task-service-empty");
      this.#dom.list.append(empty);
      return;
    }
    for (const task of tasks) this.#dom.list.append(this.#taskRow(task));
  }

  #renderCasino(): void {
    const service = this.#service;
    const casino = service?.casino;
    const session = casino?.session;
    this.#dom.close.disabled = this.#state.busy || !!(session && session.round.type !== "finished");
    if (!casino || !service) return;
    const document = this.#dom.list.ownerDocument;
    const row = document.createElement("li");
    row.className = "task-service-row";
    const send = (action: CasinoActionDto): void => {
      if (!this.#state.busy) void this.#dispatch({ type: "casino", facilityId: service.id, action });
    };
    const text = (value: string): void => {
      const p = document.createElement("p"); p.textContent = value; row.append(p);
    };
    const button = (key: string, act: () => void): HTMLButtonElement => {
      const b = document.createElement("button"); b.type = "button";
      b.textContent = this.#localization.format(key); b.disabled = this.#state.busy;
      b.addEventListener("click", act); row.append(b); return b;
    };
    const label = (key: string, control: HTMLElement): void => {
      const l = document.createElement("label"); l.textContent = this.#localization.format(key);
      l.append(control); row.append(l);
    };
    const wheel = document.createElement("select");
    for (let n = 0; n < 10; n++) {
      const option = document.createElement("option"); option.value = String(n); option.textContent = String(n); wheel.append(option);
    }
    wheel.disabled = this.#state.busy;
    const cards = (values: readonly number[], replace: boolean): (() => number) => {
      const checks: HTMLInputElement[] = [];
      values.forEach((card) => {
        const l = document.createElement("label");
        l.textContent = card === 52 ? "JOKER" : `${["♣", "♦", "♥", "♠"][Math.floor(card / 13)]}${["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"][card % 13]}`;
        if (replace) {
          const check = document.createElement("input"); check.type = "checkbox"; check.disabled = this.#state.busy;
          l.append(check, this.#localization.format("casino-replace")); checks.push(check);
        }
        row.append(l);
      });
      return () => checks.reduce((mask, check, i) => mask | (check.checked ? 1 << i : 0), 0);
    };
    if (!session) {
      const game = document.createElement("select");
      for (const kind of ["in-between", "craps", "roulette", "dice-slots", "poker"] satisfies CasinoGameDto[]) {
        const option = document.createElement("option"); option.value = kind; option.textContent = this.#localization.format(`casino-${kind}`); game.append(option);
      }
      game.disabled = this.#state.busy; label("casino-game", game);
      const wager = document.createElement("input"); wager.type = "number"; wager.min = "1";
      wager.max = String(casino.maximumWager); wager.step = "1"; wager.value = "1"; wager.disabled = this.#state.busy;
      const wagerLabel = document.createElement("label"); wagerLabel.textContent = this.#localization.format("casino-wager", { maximum: casino.maximumWager });
      wagerLabel.append(wager); row.append(wagerLabel);
      label("casino-choice", wheel);
      const updateWheel = (): void => { wheel.parentElement!.hidden = game.value !== "roulette"; };
      game.addEventListener("change", updateWheel); updateWheel();
      const start = button("casino-start", () => {
        if (wager.checkValidity()) send({ type: "start", game: game.value as CasinoGameDto, wager: Number(wager.value),
          rouletteChoice: game.value === "roulette" ? Number(wheel.value) : undefined });
      });
      start.disabled ||= casino.maximumWager < 1;
    } else {
      text(this.#localization.format(`casino-${session.game}`));
      text(this.#localization.format("casino-session", { wager: session.wager, gold: session.startingGold }));
      const round = session.round;
      if (round.type === "poker") {
        const mask = cards(round.cards, true); button("casino-draw", () => send({ type: "draw", replaceMask: mask() }));
      } else if (round.type === "craps") {
        text(this.#localization.format("casino-point", { point: round.point })); text(round.dice.join(" · "));
        button("casino-roll", () => send({ type: "roll" }));
      } else {
        if (session.game === "poker") cards(round.values, false);
        else if (session.game === "dice-slots") text(round.values.map((v) => ["🍋", "🍊", "⚔", "🛡", "🟣", "🍒"][v - 1]).join(" · "));
        else text(round.values.join(" · "));
        text(this.#localization.format(round.resultKey));
        text(this.#localization.format("casino-return", { payout: round.payout, odds: round.odds }));
        if (session.game === "roulette") label("casino-choice", wheel);
        const again = button("casino-again", () => send({ type: "again", rouletteChoice: session.game === "roulette" ? Number(wheel.value) : undefined }));
        again.disabled ||= casino.maximumWager < session.wager;
        button("casino-leave", () => send({ type: "leave" }));
      }
    }
    const rules = document.createElement("details"); const title = document.createElement("summary");
    title.textContent = this.#localization.format("casino-rules"); const body = document.createElement("p");
    body.textContent = this.#localization.format("casino-rules-text"); rules.append(title, body); row.append(rules);
    this.#dom.list.append(row);
  }

  #renderMonsterResearch(): void {
    const service = this.#service;
    if (service?.researchMonsterCost == null) return;
    const monsters = service.researchMonsters ?? [];
    const document = this.#dom.list.ownerDocument;
    const row = document.createElement("li");
    row.className = "task-service-row monster-research-row";
    const name = document.createElement("input");
    name.type = "search";
    name.value = this.#monsterName;
    name.placeholder = this.#localization.format("monster-research-name");
    name.setAttribute("aria-label", name.placeholder);
    const glyph = document.createElement("input");
    glyph.value = this.#monsterGlyph;
    glyph.maxLength = 1;
    glyph.placeholder = this.#localization.format("monster-research-glyph");
    glyph.setAttribute("aria-label", glyph.placeholder);
    const group = document.createElement("select");
    group.setAttribute("aria-label", this.#localization.format("monster-research-group"));
    for (const value of ["all", "unique", "nonunique"]) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = this.#localization.format(`monster-research-${value}`);
      group.append(option);
    }
    group.value = this.#monsterGroup;
    const select = document.createElement("select");
    select.setAttribute("aria-label", this.#localization.format("monster-research-target"));
    const button = document.createElement("button");
    button.type = "button";
    button.className = "primary-button task-service-action";
    button.dataset.facilityAction = "research-monster";
    button.textContent = this.#localization.format("action-monster-research", { cost: service.researchMonsterCost });
    const detail = document.createElement("div");
    const renderDetail = (): void => {
      this.#monsterKindId = select.value;
      const monster = monsters.find((entry) => entry.kindId === select.value);
      const knowledge = monster?.knowledge;
      detail.replaceChildren();
      button.disabled = this.#state.busy || !monster;
      const line = (key: string, value: string | number): void => {
        const p = document.createElement("p");
        p.textContent = `${this.#localization.format(key)}: ${value}`;
        detail.append(p);
      };
      if (!knowledge) {
        detail.textContent = this.#localization.format(monster ? "monster-research-unseen" : "monster-research-empty");
        return;
      }
      const description = document.createElement("p");
      description.textContent = this.#localization.format(knowledge.descriptionKey);
      detail.append(description);
      line("monster-research-base-hp", knowledge.maxHp);
      line("monster-probe-speed", knowledge.speed);
      line("monster-probe-armor-class", knowledge.armorClass);
      line("monster-probe-resistances", knowledge.resistances.filter((r) => r.level !== "normal")
        .map((r) => `${this.#localization.format(`damage-type-${r.damageType}-name`)}: ${this.#localization.format(`resistance-level-${r.level}`)}`).join(", ") || "—");
      line("monster-probe-status-immunities", knowledge.statusImmunities.map(this.#statusName).join(", ") || "—");
      line("monster-probe-melee", knowledge.meleeRoutine.blows
        .map((b) => `${this.#contentName(b.methodId)} ${b.damage.dice}d${b.damage.sides} (${b.toHit >= 0 ? "+" : ""}${b.toHit})`).join(", ") || "—");
      line("monster-probe-abilities", knowledge.abilityIds.map(this.#contentName).join(", ") || "—");
    };
    const filter = (): void => {
      this.#monsterName = name.value;
      this.#monsterGlyph = glyph.value;
      this.#monsterGroup = group.value;
      const matches = filterResearchMonsters(monsters, name.value, glyph.value, group.value,
        (entry) => this.#localization.format(entry.nameKey));
      select.replaceChildren();
      for (const monster of matches) {
        const option = document.createElement("option");
        option.value = monster.kindId;
        option.textContent = `${monster.glyph} ${this.#localization.format(monster.nameKey)} (${monster.level})`;
        select.append(option);
      }
      if (matches.some((entry) => entry.kindId === this.#monsterKindId)) select.value = this.#monsterKindId;
      renderDetail();
    };
    name.addEventListener("input", filter);
    glyph.addEventListener("input", filter);
    group.addEventListener("change", filter);
    select.addEventListener("change", renderDetail);
    filter();
    row.append(name, glyph, group, select, button, detail);
    this.#dom.list.append(row);
  }

  #renderTeleportLevel(): void {
    const service = this.#service;
    if (service?.teleportLevelCost == null) return;
    const dungeons = service.teleportDungeons ?? [];
    const document = this.#dom.list.ownerDocument;
    const row = document.createElement("li");
    row.className = "task-service-row";
    const dungeon = document.createElement("select");
    dungeon.setAttribute("aria-label", this.#localization.format("teleport-level-dungeon"));
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = this.#localization.format(dungeons.length ? "teleport-level-dungeon" : "teleport-level-empty");
    dungeon.append(placeholder);
    for (const entry of dungeons) {
      const option = document.createElement("option");
      option.value = entry.dungeonId;
      option.textContent = this.#localization.format("teleport-level-dungeon-option", {
        name: this.#localization.format(entry.nameKey), depth: entry.recallDepth,
      });
      dungeon.append(option);
    }
    dungeon.value = this.#teleportDungeonId;
    dungeon.disabled = this.#state.busy || !dungeons.length;
    const depth = document.createElement("select");
    depth.setAttribute("aria-label", this.#localization.format("teleport-level-depth"));
    const button = document.createElement("button");
    button.type = "button";
    button.className = "primary-button task-service-action";
    button.dataset.facilityAction = "teleport-level";
    button.textContent = this.#localization.format("action-teleport-level", { cost: service.teleportLevelCost });
    const update = (): void => {
      this.#teleportDepth = depth.value;
      button.disabled = this.#state.busy || !depth.value;
    };
    const selectDungeon = (): void => {
      this.#teleportDungeonId = dungeon.value;
      depth.replaceChildren();
      const entry = dungeons.find((entry) => entry.dungeonId === dungeon.value);
      for (const value of entry?.depths ?? []) {
        const option = document.createElement("option");
        option.value = String(value);
        option.textContent = this.#localization.format("teleport-level-depth-option", { depth: value });
        depth.append(option);
      }
      if (entry?.depths.includes(Number(this.#teleportDepth))) depth.value = this.#teleportDepth;
      depth.disabled = this.#state.busy || !entry;
      update();
    };
    dungeon.addEventListener("change", selectDungeon);
    depth.addEventListener("change", update);
    selectDungeon();
    row.append(dungeon, depth, button);
    this.#dom.list.append(row);
  }

  #renderFacilityActions(): void {
    const service = this.#service;
    if (!service) return;
    const document = this.#dom.list.ownerDocument;
    this.#renderBountyOffice();
    this.#renderMonsterResearch();
    this.#renderTeleportLevel();
    const renderItemAction = (
      action: "identify" | "research",
      cost: number | null | undefined,
      full: boolean,
    ): void => {
      if (cost === undefined || cost === null) return;
      const row = document.createElement("li");
      row.className = "task-service-row";
      const select = document.createElement("select");
      select.dataset.facilityItemAction = action;
      const items = [...this.#state.inventory, ...this.#state.equipment]
        .filter((item) => facilityIdentificationCandidate(item.identification, full));
      for (const item of items) {
        const option = document.createElement("option");
        option.value = item.id;
        option.textContent = this.#visibleItemName(item.displayNameKey, item.kindId, item.artifactName);
        select.append(option);
      }
      const button = document.createElement("button");
      button.type = "button";
      button.className = "primary-button task-service-action";
      button.dataset.facilityAction = action;
      button.disabled = this.#state.busy || items.length === 0;
      button.textContent = this.#localization.format(
        action === "identify" ? "action-facility-identify" : "action-facility-research",
        {
          cost,
        },
      );
      row.append(select, button);
      this.#dom.list.append(row);
    };
    renderItemAction("identify", service.identifyItemCost, false);
    renderItemAction("research", service.researchItemCost, true);
    if (service.identifyAllItemsCost !== undefined && service.identifyAllItemsCost !== null) {
      const row = document.createElement("li");
      row.className = "task-service-row";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "primary-button task-service-action";
      button.dataset.facilityAction = "identify-all";
      button.disabled = this.#state.busy || ![...this.#state.inventory, ...this.#state.equipment]
        .some((item) => facilityIdentificationCandidate(item.identification, false));
      button.textContent = this.#localization.format("action-facility-identify-all", {
        cost: service.identifyAllItemsCost,
      });
      row.append(button);
      this.#dom.list.append(row);
    }
    if (service.innStayCost !== undefined && service.innStayCost !== null) {
      const row = document.createElement("li");
      row.className = "task-service-row";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "primary-button task-service-action";
      button.dataset.facilityAction = "stay";
      button.disabled = this.#state.busy;
      button.textContent = this.#localization.format("action-inn-stay", {
        cost: service.innStayCost,
      });
      row.append(button);
      this.#dom.list.append(row);
    }
    if (service.overviewMessageKey) {
      const row = document.createElement("li");
      row.className = "task-service-row";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "task-service-action";
      button.dataset.facilityAction = "overview";
      button.disabled = this.#state.busy;
      button.textContent = this.#localization.format("action-facility-overview");
      row.append(button);
      this.#dom.list.append(row);
    }
    const carriedItems = new Map(
      [...this.#state.inventory, ...this.#state.equipment].map((item) => [item.id, item]),
    );
    for (const facilityService of service.serviceActions ?? []) {
      const row = document.createElement("li");
      row.className = "task-service-row";
      if (facilityServiceUsesItem(facilityService.kind)) {
        const select = document.createElement("select");
        select.dataset.facilityService = facilityService.kind;
        for (const target of facilityService.targets ?? []) {
          const item = carriedItems.get(target.itemId);
          if (!item) continue;
          for (const choice of target.choices) {
            const option = document.createElement("option");
            option.value = `${target.itemId}:${choice.steps}`;
            option.dataset.itemId = target.itemId;
            option.dataset.steps = String(choice.steps);
            option.textContent = this.#localization.format("facility-enchantment-choice", {
              target: this.#visibleItemName(item.displayNameKey, item.kindId, item.artifactName),
              steps: choice.steps,
              hit: choice.result.toHit,
              damage: choice.result.toDamage,
              armor: choice.result.toArmor,
              cost: choice.cost,
            });
            select.append(option);
          }
        }
        select.disabled = this.#state.busy;
        row.append(select);
      }
      const button = document.createElement("button");
      button.type = "button";
      button.className = "primary-button task-service-action";
      button.dataset.facilityAction = "service";
      button.dataset.facilityService = facilityService.kind;
      button.disabled = this.#state.busy
        || facilityServiceUsesItem(facilityService.kind)
          && (facilityService.targets?.length ?? 0) === 0;
      button.textContent = this.#localization.format(
        facilityServiceActionKey(facilityService.kind),
        facilityServiceUsesItem(facilityService.kind) ? undefined : { cost: facilityService.cost },
      );
      row.append(button);
      this.#dom.list.append(row);
    }
    if (service.legalNameChangeCost !== undefined && service.legalNameChangeCost !== null) {
      const row = document.createElement("li");
      row.className = "task-service-row";
      const input = document.createElement("input");
      input.type = "text";
      input.maxLength = 32;
      input.value = this.#state.status?.player.name ?? "";
      input.dataset.playerName = "true";
      input.setAttribute("aria-label", this.#localization.format("facility-rename-label"));
      const button = document.createElement("button");
      button.type = "button";
      button.className = "primary-button task-service-action";
      button.dataset.facilityAction = "rename";
      button.disabled = this.#state.busy;
      button.textContent = this.#localization.format("action-facility-rename", {
        cost: service.legalNameChangeCost,
      });
      row.append(input, button);
      this.#dom.list.append(row);
    }
  }

  #renderBountyOffice(): void {
    const bounty = this.#service?.bountyOffice;
    if (!bounty) return;
    const document = this.#dom.list.ownerDocument;
    const daily = document.createElement("li");
    daily.className = "task-service-row";
    daily.textContent = this.#localization.format("bounty-daily-target", {
      actor: this.#localization.format(bounty.dailyTarget.actorNameKey),
      corpse: bounty.dailyTarget.corpseReward,
      skeleton: bounty.dailyTarget.skeletonReward,
    });
    this.#dom.list.append(daily);

    const wanted = document.createElement("li");
    wanted.className = "task-service-row task-service-copy";
    for (const [index, target] of bounty.wantedTargets.entries()) {
      const line = document.createElement("p");
      line.textContent = this.#localization.format("bounty-wanted-target", {
        index: index + 1,
        status: this.#localization.format(
          target.completed ? "bounty-wanted-completed" : "bounty-wanted-open",
        ),
        actor: this.#localization.format(target.actorNameKey),
        reward: this.#localization.format(target.rewardNameKey),
      });
      wanted.append(line);
    }
    this.#dom.list.append(wanted);

    for (const turnIn of bounty.turnIns) {
      const row = document.createElement("li");
      row.className = "task-service-row";
      const button = document.createElement("button");
      button.type = "button";
      button.className = "primary-button task-service-action";
      button.dataset.bountyAction = "turn-in";
      button.dataset.itemId = turnIn.itemId;
      button.disabled = this.#state.busy;
      button.textContent = turnIn.reward.kind === "gold"
        ? this.#localization.format("action-bounty-turn-in-gold", {
            actor: this.#localization.format(turnIn.actorNameKey),
            gold: turnIn.reward.amount,
          })
        : this.#localization.format("action-bounty-turn-in-item", {
            actor: this.#localization.format(turnIn.actorNameKey),
            item: this.#localization.format(turnIn.reward.itemNameKey),
          });
      row.append(button);
      this.#dom.list.append(row);
    }

    const missionRow = document.createElement("li");
    missionRow.className = "task-service-row";
    const missionButton = document.createElement("button");
    missionButton.type = "button";
    missionButton.className = "primary-button task-service-action";
    missionButton.disabled = this.#state.busy;
    if (!bounty.mission) {
      missionButton.dataset.bountyAction = bountyMissionAction(undefined);
      missionButton.textContent = this.#localization.format("action-bounty-request-mission");
    } else if (bounty.mission.status === "active") {
      const progress = document.createElement("p");
      progress.textContent = this.#localization.format("bounty-mission-progress", {
        floor: this.#localization.format(bounty.mission.floorNameKey),
        depth: bounty.mission.depth,
        actor: this.#localization.format(bounty.mission.actorNameKey),
        remaining: bounty.mission.remaining,
        total: bounty.mission.total,
      });
      missionRow.append(progress);
      missionButton.dataset.bountyAction = bountyMissionAction(bounty.mission.status);
      missionButton.textContent = this.#localization.format("action-bounty-abandon-mission");
    } else {
      const complete = document.createElement("p");
      complete.textContent = this.#localization.format("bounty-mission-complete", {
        actor: this.#localization.format(bounty.mission.actorNameKey),
        item: this.#localization.format(bounty.mission.rewardNameKey),
      });
      missionRow.append(complete);
      missionButton.dataset.bountyAction = bountyMissionAction(bounty.mission.status);
      missionButton.textContent = this.#localization.format("action-bounty-claim-mission");
    }
    missionRow.append(missionButton);
    this.#dom.list.append(missionRow);
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
        taskActionLabelKey(action, task.hasItemReward),
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

export function taskActionLabelKey(
  action: TaskServiceAction,
  hasItemReward: boolean,
): "action-task-accept" | "action-task-claim" | "action-task-conclude" {
  if (action === "accept") return "action-task-accept";
  return hasItemReward ? "action-task-claim" : "action-task-conclude";
}

export function bountyMissionAction(
  status: BountyMissionStatusDto | undefined,
): BountyOfficeActionDto {
  if (status === "active") return "abandon-mission";
  if (status === "reward-available") return "claim-mission-reward";
  return "request-mission";
}

function lastTaskServiceEvent(state: GameSnapshot | GameUpdate): GameEventDto | undefined {
  if (!("events" in state)) return undefined;
  for (let index = state.events.length - 1; index >= 0; index -= 1) {
    const event = state.events[index];
    if (
      event?.kind === "task.accepted" ||
      event?.kind === "task.accept-unavailable" ||
      event?.kind === "task.rewarded" ||
      event?.kind === "task.reward-claim-unavailable" ||
      event?.kind === "facility.identify-unavailable" ||
      event?.kind === "facility.identified" ||
      event?.kind === "facility.monster-researched" ||
      event?.kind === "facility.monster-research-unavailable" ||
      event?.kind === "facility.identify-all-unavailable" ||
      event?.kind === "facility.identified-all" ||
      event?.kind === "facility.service-unavailable" ||
      event?.kind === "facility.healed" ||
      event?.kind === "facility.vitality-restored" ||
      event?.kind === "facility.mutation-cured" ||
      event?.kind === "facility.item-enchanted" ||
      event?.kind === "facility.armor-assessed" ||
      event?.kind === "facility.recall-started" ||
      event?.kind === "facility.rename-unavailable" ||
      event?.kind === "facility.renamed" ||
      event?.kind === "inn.stay" ||
      event?.kind === "inn.stay-unavailable" ||
      event?.kind.startsWith("facility.casino-") ||
      event?.kind.startsWith("bounty.")
    ) {
      return event;
    }
  }
  return undefined;
}

export function facilityMembershipKey(membership: FacilityMembershipDto) {
  return `facility-membership-${membership}` as const;
}

export function facilityServiceUsesItem(service: FacilityServiceKindDto): boolean {
  return service.startsWith("enchant-");
}

export function facilityServiceActionKey(service: FacilityServiceKindDto) {
  return `action-facility-${service}` as const;
}

export function facilityIdentificationCandidate(
  identification: ItemIdentificationDto,
  full: boolean,
): boolean {
  return full ? identification !== "identified" : identification === "unexamined";
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

// SPDX-License-Identifier: MPL-2.0
import type { AppState } from "./app-state";
import type { Localization, MessageKey } from "./localization";
import type { GameCommand, SummonCommandModeDto } from "./protocol";

type PetMenuOptions = {
  document: Document;
  state: AppState;
  localization: Localization;
  dispatch: (command: GameCommand) => Promise<void>;
  contentName: (id: string | undefined) => string;
  confirm: (message: string) => boolean;
  startRiding: () => void;
};

export class PetMenu {
  #dialog: HTMLDialogElement | undefined;
  readonly #options: PetMenuOptions;

  constructor(options: PetMenuOptions) { this.#options = options; }

  close(): void { this.#dialog?.close(); }

  open(): void {
    const { document, state, localization, dispatch, contentName, confirm, startRiding } = this.#options;
    if (state.busy || state.commandBlocked || state.worldMap || state.targeting || state.terrainInteractionMode
      || document.querySelector("dialog[open]")) return;
    const player = state.status?.player;
    if (!player) return;
    const dialog = document.createElement("dialog");
    dialog.className = "item-target-dialog pet-menu-dialog";
    this.#dialog = dialog;
    const title = document.createElement("h2");
    title.textContent = localization.format("pet-menu-open");
    dialog.setAttribute("aria-label", title.textContent);
    const header = document.createElement("header");
    header.append(title); dialog.append(header);
    const summary = document.createElement("p");
    summary.className = "pet-menu-summary";
    summary.textContent = localization.format("summon-command-status", {
      mode: localization.format(`summon-command-mode-${player.summonCommand?.mode ?? "follow"}` as MessageKey),
      count: player.petUpkeep?.controlledPets ?? 0, upkeep: player.petUpkeep?.upkeepPercent ?? 0,
    });
    dialog.append(summary);
    const body = document.createElement("div"); body.className = "pet-menu-content"; dialog.append(body);
    const section = (key: MessageKey) => {
      const node = document.createElement("section"), heading = document.createElement("h3");
      heading.textContent = localization.format(key); node.append(heading); body.append(node); return node;
    };
    let group: HTMLElement = section("panel-summon-command-title");
    const button = (key: MessageKey, run: () => void, disabled = false) => {
      const element = document.createElement("button");
      element.type = "button";
      element.textContent = localization.format(key);
      element.disabled = disabled;
      element.addEventListener("click", run);
      group.append(element);
      return element;
    };
    const send = (command: GameCommand) => { dialog.close(); void dispatch(command); };
    if (player.ridingActorId) {
      button(player.summonCommand?.ridingTwoHands ? "pet-menu-reins" : "pet-menu-two-hands",
        () => send({ type: "set-pet-option", option: "riding-two-hands", enabled: !player.summonCommand?.ridingTwoHands }))
        .setAttribute("aria-pressed", String(player.summonCommand?.ridingTwoHands ?? false));
      const control = document.createElement("p");
      control.textContent = localization.format(player.ridingWithoutReins ? "pet-menu-without-reins" : "pet-menu-with-reins");
      group.append(control);
    }
    const modes: SummonCommandModeDto[] = ["stay-close", "follow", "attack", "give-space", "keep-distance", "guard"];
    for (const mode of modes) {
      button(`action-summon-command-${mode}`, () => send({ type: "set-summon-command", mode }))
        .setAttribute("aria-pressed", String((player.summonCommand?.mode ?? "follow") === mode));
    }
    group = section("pet-menu-behavior-title");
    for (const [option, enabled] of [
      ["highlight-map", player.summonCommand?.highlightMap ?? false],
      ["highlight-lists", player.summonCommand?.highlightLists ?? true],
      ["open-doors", player.summonCommand?.openDoors ?? false],
      ["pickup-items", player.summonCommand?.pickupItems ?? false],
      ["no-breeding", player.summonCommand?.noBreeding ?? false],
      ["attack-spells", player.summonCommand?.attackSpells ?? true],
      ["summon-spells", player.summonCommand?.summonSpells ?? true],
      ["teleport", player.summonCommand?.teleport ?? true],
      ["allow-player-damage", player.summonCommand?.allowPlayerDamage ?? false],
    ] as const) {
      button(`pet-menu-${option}-${enabled ? "on" : "off"}`,
        () => send({ type: "set-pet-option", option, enabled: !enabled }))
        .setAttribute("aria-pressed", String(enabled));
    }
    const select = (key: MessageKey, entries: Array<{ id: string; name: string }>) => {
      const label = document.createElement("label");
      const text = document.createElement("span");
      text.textContent = localization.format(key);
      const input = document.createElement("select");
      for (const entry of entries) {
        const option = document.createElement("option");
        option.value = entry.id; option.textContent = entry.name; input.append(option);
      }
      input.disabled = entries.length === 0;
      label.append(text, input); group.append(label); return input;
    };
    group = section("pet-menu-management-title");
    const enemies = state.status?.entities.filter(entity => entity.faction === "hostile") ?? [];
    const target = select("pet-menu-target", enemies.map(entity => ({ id: entity.id,
      name: `${entity.customName ?? contentName(entity.kindId)} (${entity.position.x}, ${entity.position.y})` })));
    const current = document.createElement("p");
    const namedTarget = enemies.find(entity => entity.id === player.summonCommand?.targetActorId);
    current.textContent = localization.format("pet-menu-current-target", { target: namedTarget ? namedTarget.customName ?? contentName(namedTarget.kindId)
      : localization.format(player.summonCommand?.targetActorId ? "pet-menu-target-unseen" : "pet-menu-target-none") });
    group.append(current);
    button("pet-menu-set-target", () => send({ type: "set-pet-target", actorId: target.value }), !enemies.length);
    button("pet-menu-clear-target", () => send({ type: "set-pet-target", actorId: null }), !player.summonCommand?.targetActorId);
    const pets = player.pets ?? [];
    const pet = select("pet-menu-select-pet", pets.map((entry, index) => ({ id: entry.actorId,
      name: `${index + 1}. ${entry.customName ?? localization.format(entry.nameKey as MessageKey)}${entry.riding ? ` (${localization.format("pet-riding")})` : ""}` })));
    const nameLabel = document.createElement("label");
    const nameText = document.createElement("span");
    nameText.textContent = localization.format("pet-menu-name");
    const nameInput = document.createElement("input");
    nameInput.type = "text";
    nameLabel.append(nameText, nameInput); group.append(nameLabel);
    const rename = button("pet-menu-name-apply", () => send({ type: "set-pet-name", actorId: pet.value, name: nameInput.value || null }));
    const clearName = button("pet-menu-name-clear", () => send({ type: "set-pet-name", actorId: pet.value, name: null }));
    nameInput.addEventListener("keydown", event => {
      if (event.key === "Enter" && !event.isComposing) { event.preventDefault(); rename.click(); }
    });
    const selectPet = () => {
      const selected = pets.find(entry => entry.actorId === pet.value);
      nameInput.value = selected?.customName ?? "";
      nameInput.disabled = rename.disabled = !selected?.canName;
      clearName.disabled = !selected?.canName || !selected?.customName;
      pet.className = selected?.highlight ? "pet-highlight" : "";
    };
    pet.addEventListener("change", selectPet); selectPet();
    button("pet-menu-dismiss-one", () => {
      const selected = pets.find(entry => entry.actorId === pet.value);
      if (selected && confirm(localization.format("pet-menu-dismiss-confirm", { pet: selected.customName ?? localization.format(selected.nameKey as MessageKey) })))
        send({ type: "dismiss-pet", actorId: selected.actorId });
    }, !pets.length);
    button("action-dismiss-pets", () => {
      if (confirm(localization.format("pet-menu-dismiss-all-confirm"))) send({ type: "dismiss-pets" });
    }, !pets.length);
    button(player.ridingActorId ? "pet-menu-dismount" : "pet-menu-ride", () => { dialog.close(); startRiding(); }, !pets.length);
    group = header;
    const close = button("action-dialog-close", () => dialog.close());
    dialog.addEventListener("close", () => { dialog.remove(); if (this.#dialog === dialog) this.#dialog = undefined; }, { once: true });
    document.body.append(dialog); dialog.showModal(); close.focus();
  }
}

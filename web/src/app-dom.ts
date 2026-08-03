// SPDX-License-Identifier: MPL-2.0

import type { SummonCommandModeDto } from "./protocol";

type DocumentLookup = Pick<Document, "getElementById">;

export interface AppDom {
  readonly mapHost: HTMLElement;
  readonly targetCursor: HTMLElement;
  readonly traverseStairs: HTMLButtonElement;
  readonly targetModeToggle: HTMLButtonElement;
  readonly lookModeToggle: HTMLButtonElement;
  readonly targetModeStatus: HTMLElement;
  readonly connectionStatus: HTMLElement;
  readonly journeyPanel: HTMLElement;
  readonly journeyDungeonName: HTMLElement;
  readonly journeyDepth: HTMLElement;
  readonly journeyBoss: HTMLElement;
  readonly onboardingKind: HTMLElement;
  readonly onboardingTitle: HTMLElement;
  readonly onboardingDetail: HTMLElement;
  readonly onboardingControl: HTMLElement;
  readonly onboardingProgress: HTMLElement;
  readonly onboardingHideOptional: HTMLInputElement;
  readonly onboardingReset: HTMLButtonElement;
  readonly journeyResult: HTMLElement;
  readonly resultKind: HTMLElement;
  readonly resultTitle: HTMLElement;
  readonly resultDetail: HTMLElement;
  readonly resultBuild: HTMLElement;
  readonly resultSeed: HTMLElement;
  readonly resultTurn: HTMLElement;
  readonly resultScore: HTMLElement;
  readonly resultDungeons: HTMLElement;
  readonly resultTasks: HTMLElement;
  readonly resultContinue: HTMLButtonElement;
  readonly resultRestart: HTMLButtonElement;
  readonly resultNewGame: HTMLButtonElement;
  readonly resultLoad: HTMLButtonElement;
  readonly resultMenu: HTMLButtonElement;
  readonly resultExit: HTMLButtonElement;
  readonly resultError: HTMLElement;
  readonly messageList: HTMLOListElement;
  readonly turnValue: HTMLElement;
  readonly hpValue: HTMLElement;
  readonly healthMeter: HTMLElement;
  readonly healthMeterFill: HTMLElement;
  readonly goldValue: HTMLElement;
  readonly nutritionValue: HTMLElement;
  readonly attackValue: HTMLElement;
  readonly defenseValue: HTMLElement;
  readonly effectsValue: HTMLElement;
  readonly positionValue: HTMLElement;
  readonly hashValue: HTMLElement;
  readonly progressionLevelValue: HTMLElement;
  readonly progressionExperienceValue: HTMLElement;
  readonly progressionCapValue: HTMLElement;
  readonly progressionPointsValue: HTMLElement;
  readonly progressionBuildValue: HTMLElement;
  readonly progressionRaceValue: HTMLElement;
  readonly progressionClassValue: HTMLElement;
  readonly progressionPersonalityValue: HTMLElement;
  readonly progressionMultipliersValue: HTMLElement;
  readonly attributeList: HTMLUListElement;
  readonly skillList: HTMLUListElement;
  readonly resourceList: HTMLUListElement;
  readonly abilityList: HTMLUListElement;
  readonly resourceRest: HTMLButtonElement;
  readonly nearbyCurrent: HTMLElement;
  readonly nearbyList: HTMLUListElement;
  readonly summonCommandStatus: HTMLElement;
  readonly summonCommandButtons: Readonly<Record<SummonCommandModeDto, HTMLButtonElement>>;
  readonly taskLogList: HTMLUListElement;
  readonly campaignStatusValue: HTMLElement;
  readonly campaignScoreValue: HTMLElement;
  readonly campaignDungeonsValue: HTMLElement;
  readonly campaignTasksValue: HTMLElement;
  readonly campaignRetire: HTMLButtonElement;
  readonly inventoryCount: HTMLElement;
  readonly inventorySelectionCount: HTMLElement;
  readonly inventoryUse: HTMLButtonElement;
  readonly inventoryAppraise: HTMLButtonElement;
  readonly inventoryEquip: HTMLButtonElement;
  readonly inventoryDrop: HTMLButtonElement;
  readonly inventoryDropQuantity: HTMLInputElement;
  readonly inventoryList: HTMLUListElement;
  readonly equipmentList: HTMLUListElement;
  readonly nativeSaveName: HTMLInputElement;
  readonly nativeSaveCreate: HTMLButtonElement;
  readonly nativeSaveRefresh: HTMLButtonElement;
  readonly nativeSaveList: HTMLUListElement;
  readonly replayButton: HTMLButtonElement;
  readonly saveButton: HTMLButtonElement;
  readonly loadInput: HTMLInputElement;
  readonly clearMessages: HTMLButtonElement;
  readonly inputPresetSelect: HTMLSelectElement;
  readonly tilesetPresetSelect: HTMLSelectElement;
  readonly cameraModeSelect: HTMLSelectElement;
  readonly zoomSelect: HTMLSelectElement;
  readonly controlsHelp: HTMLElement;
  readonly languageSelect: HTMLSelectElement;
}

export function createAppDom(document: DocumentLookup): Readonly<AppDom> {
  return Object.freeze({
    mapHost: element<HTMLElement>(document, "map-host"),
    targetCursor: element<HTMLElement>(document, "target-cursor"),
    traverseStairs: element<HTMLButtonElement>(document, "traverse-stairs"),
    targetModeToggle: element<HTMLButtonElement>(document, "target-mode-toggle"),
    lookModeToggle: element<HTMLButtonElement>(document, "look-mode-toggle"),
    targetModeStatus: element<HTMLElement>(document, "target-mode-status"),
    connectionStatus: element<HTMLElement>(document, "connection-status"),
    journeyPanel: element<HTMLElement>(document, "journey-panel"),
    journeyDungeonName: element<HTMLElement>(document, "journey-dungeon-name"),
    journeyDepth: element<HTMLElement>(document, "journey-depth"),
    journeyBoss: element<HTMLElement>(document, "journey-boss"),
    onboardingKind: element<HTMLElement>(document, "onboarding-kind"),
    onboardingTitle: element<HTMLElement>(document, "onboarding-title"),
    onboardingDetail: element<HTMLElement>(document, "onboarding-detail"),
    onboardingControl: element<HTMLElement>(document, "onboarding-control"),
    onboardingProgress: element<HTMLElement>(document, "onboarding-progress"),
    onboardingHideOptional: element<HTMLInputElement>(document, "onboarding-hide-optional"),
    onboardingReset: element<HTMLButtonElement>(document, "onboarding-reset"),
    journeyResult: element<HTMLElement>(document, "journey-result"),
    resultKind: element<HTMLElement>(document, "result-kind"),
    resultTitle: element<HTMLElement>(document, "result-title"),
    resultDetail: element<HTMLElement>(document, "result-detail"),
    resultBuild: element<HTMLElement>(document, "result-build"),
    resultSeed: element<HTMLElement>(document, "result-seed"),
    resultTurn: element<HTMLElement>(document, "result-turn"),
    resultScore: element<HTMLElement>(document, "result-score"),
    resultDungeons: element<HTMLElement>(document, "result-dungeons"),
    resultTasks: element<HTMLElement>(document, "result-tasks"),
    resultContinue: element<HTMLButtonElement>(document, "result-continue"),
    resultRestart: element<HTMLButtonElement>(document, "result-restart"),
    resultNewGame: element<HTMLButtonElement>(document, "result-new-game"),
    resultLoad: element<HTMLButtonElement>(document, "result-load"),
    resultMenu: element<HTMLButtonElement>(document, "result-menu"),
    resultExit: element<HTMLButtonElement>(document, "result-exit"),
    resultError: element<HTMLElement>(document, "result-error"),
    messageList: element<HTMLOListElement>(document, "message-list"),
    turnValue: element<HTMLElement>(document, "turn-value"),
    hpValue: element<HTMLElement>(document, "hp-value"),
    healthMeter: element<HTMLElement>(document, "health-meter"),
    healthMeterFill: element<HTMLElement>(document, "health-meter-fill"),
    goldValue: element<HTMLElement>(document, "gold-value"),
    nutritionValue: element<HTMLElement>(document, "nutrition-value"),
    attackValue: element<HTMLElement>(document, "attack-value"),
    defenseValue: element<HTMLElement>(document, "defense-value"),
    effectsValue: element<HTMLElement>(document, "effects-value"),
    positionValue: element<HTMLElement>(document, "position-value"),
    hashValue: element<HTMLElement>(document, "hash-value"),
    progressionLevelValue: element<HTMLElement>(document, "progression-level-value"),
    progressionExperienceValue: element<HTMLElement>(document, "progression-experience-value"),
    progressionCapValue: element<HTMLElement>(document, "progression-cap-value"),
    progressionPointsValue: element<HTMLElement>(document, "progression-points-value"),
    progressionBuildValue: element<HTMLElement>(document, "progression-build-value"),
    progressionRaceValue: element<HTMLElement>(document, "progression-race-value"),
    progressionClassValue: element<HTMLElement>(document, "progression-class-value"),
    progressionPersonalityValue: element<HTMLElement>(
      document,
      "progression-personality-value",
    ),
    progressionMultipliersValue: element<HTMLElement>(
      document,
      "progression-multipliers-value",
    ),
    attributeList: element<HTMLUListElement>(document, "attribute-list"),
    skillList: element<HTMLUListElement>(document, "skill-list"),
    resourceList: element<HTMLUListElement>(document, "resource-list"),
    abilityList: element<HTMLUListElement>(document, "ability-list"),
    resourceRest: element<HTMLButtonElement>(document, "resource-rest"),
    nearbyCurrent: element<HTMLElement>(document, "nearby-current"),
    nearbyList: element<HTMLUListElement>(document, "nearby-list"),
    summonCommandStatus: element<HTMLElement>(document, "summon-command-status"),
    summonCommandButtons: Object.freeze({
      follow: element<HTMLButtonElement>(document, "summon-command-follow"),
      attack: element<HTMLButtonElement>(document, "summon-command-attack"),
      "keep-distance": element<HTMLButtonElement>(document, "summon-command-keep-distance"),
      guard: element<HTMLButtonElement>(document, "summon-command-guard"),
    }),
    taskLogList: element<HTMLUListElement>(document, "task-log-list"),
    campaignStatusValue: element<HTMLElement>(document, "campaign-status-value"),
    campaignScoreValue: element<HTMLElement>(document, "campaign-score-value"),
    campaignDungeonsValue: element<HTMLElement>(document, "campaign-dungeons-value"),
    campaignTasksValue: element<HTMLElement>(document, "campaign-tasks-value"),
    campaignRetire: element<HTMLButtonElement>(document, "campaign-retire"),
    inventoryCount: element<HTMLElement>(document, "inventory-count"),
    inventorySelectionCount: element<HTMLElement>(document, "inventory-selection-count"),
    inventoryUse: element<HTMLButtonElement>(document, "inventory-use"),
    inventoryAppraise: element<HTMLButtonElement>(document, "inventory-appraise"),
    inventoryEquip: element<HTMLButtonElement>(document, "inventory-equip"),
    inventoryDrop: element<HTMLButtonElement>(document, "inventory-drop"),
    inventoryDropQuantity: element<HTMLInputElement>(document, "inventory-drop-quantity"),
    inventoryList: element<HTMLUListElement>(document, "inventory-list"),
    equipmentList: element<HTMLUListElement>(document, "equipment-list"),
    nativeSaveName: element<HTMLInputElement>(document, "native-save-name"),
    nativeSaveCreate: element<HTMLButtonElement>(document, "native-save-create"),
    nativeSaveRefresh: element<HTMLButtonElement>(document, "native-save-refresh"),
    nativeSaveList: element<HTMLUListElement>(document, "native-save-list"),
    replayButton: element<HTMLButtonElement>(document, "replay-button"),
    saveButton: element<HTMLButtonElement>(document, "save-button"),
    loadInput: element<HTMLInputElement>(document, "load-input"),
    clearMessages: element<HTMLButtonElement>(document, "clear-messages"),
    inputPresetSelect: element<HTMLSelectElement>(document, "input-preset"),
    tilesetPresetSelect: element<HTMLSelectElement>(document, "tileset-preset"),
    cameraModeSelect: element<HTMLSelectElement>(document, "camera-mode"),
    zoomSelect: element<HTMLSelectElement>(document, "zoom-level"),
    controlsHelp: element<HTMLElement>(document, "controls-help"),
    languageSelect: element<HTMLSelectElement>(document, "language-select"),
  });
}

function element<T extends HTMLElement>(document: DocumentLookup, id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing element #${id}`);
  return found as T;
}

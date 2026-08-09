// SPDX-License-Identifier: MPL-2.0

import type { Localization, MessageKey } from "./localization";
import type {
  DamageResolutionDto,
  DamageTypeDto,
  EquipmentItemDto,
  GameEventDto,
  GameSnapshot,
  GameUpdate,
  InventoryItemDto,
} from "./protocol";

export interface PresentationState {
  currentInventory: readonly InventoryItemDto[];
  currentEquipment: readonly EquipmentItemDto[];
  currentStatus: GameSnapshot | GameUpdate | undefined;
  currentWorldId?: string;
}

export function createPresentationFormatter(
  localization: Localization,
  getState: () => PresentationState,
  helpers: {
    formatAttributeValueArgument(value: string | undefined): string;
    formatTenthsPoundArgument(value: string | undefined): string;
    itemCurseSeverityName(value: string | undefined): string;
  },
) {
  const {
    formatAttributeValueArgument,
    formatTenthsPoundArgument,
    itemCurseSeverityName,
  } = helpers;

  function formatEvent(event: GameEventDto): string {
    switch (event.messageKey) {
      case "mutation-gained":
        return localization.format("message-mutation-gained", {
          mutation: event.args.name ?? event.args.target ?? "?",
        });
      case "mutation-lost":
        return localization.format("message-mutation-lost", {
          mutation: event.args.name ?? event.args.target ?? "?",
        });
      case "ability-studied":
        return localization.format("message-ability-studied", {
          ability: contentName(event.args.target),
        });
      case "ability-forgotten":
        return localization.format("message-ability-forgotten", {
          ability: contentName(event.args.target),
        });
      case "ability-forget-unavailable":
        return localization.format("message-ability-forget-unavailable", {
          ability: contentName(event.args.target),
          reason: abilityUnavailableReason(event.args.reason),
        });
      case "ability-study-unavailable":
        return localization.format("message-ability-study-unavailable", {
          ability: contentName(event.args.target),
          reason: abilityUnavailableReason(event.args.reason),
        });
      case "ability-cast-unavailable":
        return localization.format("message-ability-cast-unavailable", {
          ability: contentName(event.args.target),
          reason: abilityUnavailableReason(event.args.reason),
        });
      case "ability-cast-success":
      case "ability-cast-failure": {
        const resolution =
          event.outcome?.type === "ability-cast" ? event.outcome.resolution : undefined;
        return localization.format(
          event.messageKey === "ability-cast-success"
            ? "message-ability-cast-success"
            : "message-ability-cast-failure",
          {
            ability: contentName(event.args.target),
            roll: resolution?.percentileRoll ?? "?",
            failure: resolution?.failurePercent ?? "?",
            cost: resolution?.resourceCost ?? "?",
          },
        );
      }
      case "ability-target-unavailable":
        return localization.format("message-ability-target-unavailable", {
          ability: contentName(event.args.target),
        });
      case "ability-landed":
        return localization.format("message-ability-landed", {
          ability: contentName(event.args.target),
        });
      case "ability-area-damage":
        return localization.format("message-ability-area-damage", {
          ability: contentName(event.args.target),
          radius: event.args.radius ?? "?",
          targets: event.args.targets ?? "0",
        });
      case "ability-beam-damage":
        return localization.format("message-ability-beam-damage", {
          ability: contentName(event.args.target),
          targets: event.args.targets ?? "0",
        });
      case "ability-cone-damage":
        return localization.format("message-ability-cone-damage", {
          ability: contentName(event.args.target),
          radius: event.args.radius ?? "?",
          targets: event.args.targets ?? "0",
        });
      case "ability-teleport": {
        const resolution =
          event.outcome?.type === "ability-teleport" ? event.outcome.resolution : undefined;
        return localization.format("message-ability-teleport", {
          ability: contentName(event.args.target),
          fromX: resolution?.from.x ?? event.args.fromX ?? "?",
          fromY: resolution?.from.y ?? event.args.fromY ?? "?",
          toX: resolution?.to.x ?? event.args.toX ?? "?",
          toY: resolution?.to.y ?? event.args.toY ?? "?",
        });
      }
      case "ability-summon":
        return localization.format("message-ability-summon", {
          ability: contentName(event.args.target),
          actor: contentName(event.args.actor),
          count: event.args.count ?? "0",
        });
      case "ability-detect":
        return localization.format("message-ability-detect", {
          ability: contentName(event.args.target),
          category: event.args.category ?? "?",
          count: event.args.count ?? "0",
        });
      case "ability-terrain-transform":
        return localization.format("message-ability-terrain-transform", {
          ability: contentName(event.args.target),
          terrain: contentName(event.args.terrain),
          count: event.args.count ?? "0",
        });
      case "ability-effects":
        return localization.format("message-ability-effects", {
          ability: contentName(event.args.target),
          count: event.args.count ?? "0",
        });
      case "monster-ability-decision": {
        const resolution =
          event.outcome?.type === "monster-ability-decision"
            ? event.outcome.resolution
            : undefined;
        const selectedAbilityId = resolution?.selectedAbilityId;
        return localization.format(
          selectedAbilityId
            ? "message-monster-ability-decision-cast"
            : "message-monster-ability-decision-fallback",
          {
            source: contentName(event.args.source),
            ability: selectedAbilityId ? contentName(selectedAbilityId) : "",
            roll: resolution?.frequencyRoll ?? event.args.roll ?? "?",
            frequency: resolution?.frequencyPercent ?? event.args.frequency ?? "?",
          },
        );
      }
      case "monster-ability-cast": {
        const resolution =
          event.outcome?.type === "monster-ability-cast" ? event.outcome.resolution : undefined;
        if (resolution?.summon) {
          const summonedKinds = resolution.summon.summonedKindIds ?? [];
          const actor =
            summonedKinds.length > 0
              ? [...new Set(summonedKinds)].map(contentName).join("、")
              : contentName(resolution.summon.actorKindId);
          return localization.format("message-monster-ability-summon", {
            source: contentName(event.args.source),
            ability: contentName(event.args.target),
            actor,
            count: resolution.summon.entityIds.length,
          });
        }
        const targets = resolution?.targets ?? [];
        const effectCount =
          targets.length > 0
            ? targets.reduce((count, target) => count + target.effects.length, 0)
            : (resolution?.effects.length ?? Number(event.args.count ?? 0));
        return localization.format("message-monster-ability-cast", {
          source: contentName(event.args.source),
          ability: contentName(event.args.target),
          count: effectCount,
          targetCount: targets.length || 1,
        });
      }
      case "summon-expired":
        return localization.format("message-summon-expired", {
          actor: contentName(event.args.actor),
        });
      case "summon-command-changed":
        return localization.format("message-summon-command-changed", {
          mode: localization.format(
            `summon-command-mode-${event.args.mode ?? "follow"}` as MessageKey,
          ),
          count: event.args.count ?? "0",
        });
      case "summon-followed-floor":
        return localization.format("message-summon-followed-floor", {
          actor: contentName(event.args.actor),
        });
      case "summon-could-not-follow":
        return localization.format("message-summon-could-not-follow", {
          actor: contentName(event.args.actor),
        });
      case "ability-hit":
        return localization.format("message-ability-hit", {
          ability: contentName(event.args.source),
          target: contentName(event.args.target),
          damage: event.args.damage ?? "?",
        });
      case "ability-slay":
        return localization.format("message-ability-slay", {
          ability: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "mutation-aura-hit":
      case "mutation-aura-slay": {
        const resolution =
          event.outcome?.type === "damage" || event.outcome?.type === "death"
            ? event.outcome.resolution
            : undefined;
        return localization.format(
          event.messageKey === "mutation-aura-slay"
            ? "message-mutation-aura-slay"
            : "message-mutation-aura-hit",
          {
            type: resolution ? damageTypeName(resolution.damageType) : "?",
            target: contentName(event.args.target),
            damage: event.args.damage ?? "?",
          },
        );
      }
      case "combat-bolt-reflected":
        return localization.format("message-combat-bolt-reflected", {
          reflector: contentName(event.args.reflector),
          source: contentName(event.args.source),
        });
      case "combat-bolt-reflected-hit":
      case "combat-bolt-reflected-slay":
        return localization.format(
          event.messageKey === "combat-bolt-reflected-slay"
            ? "message-combat-bolt-reflected-slay"
            : "message-combat-bolt-reflected-hit",
          {
            reflector: contentName(event.args.reflector),
            source: contentName(event.args.source),
            target: contentName(event.args.target),
            damage: event.args.damage ?? "?",
          },
        );
      case "ability-healed":
        return localization.format("message-ability-healed", {
          ability: contentName(event.args.source),
          amount: event.args.amount ?? "?",
        });
      case "resource-recovered":
        return localization.format("message-resource-recovered", {
          resource: contentName(event.args.target),
          amount: event.args.amount ?? "?",
        });
      case "resource-gained":
        return localization.format("message-resource-gained", {
          resource: contentName(event.args.target),
          amount: event.args.amount ?? "?",
        });
      case "monster-blinked":
        return localization.format("message-monster-blinked", {
          source: contentName(event.args.source),
        });
      case "monster-blinked-target":
        return localization.format("message-monster-blinked-target", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "monster-teleported":
        return localization.format("message-monster-teleported", {
          source: contentName(event.args.source),
        });
      case "monster-dragged-target":
        return localization.format("message-monster-dragged-target", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "monster-banished-target":
        return localization.format("message-monster-banished-target", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "rest-completed":
      case "rest-interrupted":
        return localization.format(
          event.messageKey === "rest-completed"
            ? "message-rest-completed"
            : "message-rest-interrupted",
          {
            turns: event.args.turns ?? "0",
            reason: restStopReason(event.args.reason),
          },
        );
      case "game-wait":
        return localization.format("message-game-wait");
      case "game-move-blocked":
        return localization.format("message-move-blocked");
      case "wilderness-ambushed":
        return localization.format("message-wilderness-ambushed");
      case "floor-transition":
        return localization.format("message-floor-transition", {
          from: floorName(event.args.from),
          to: floorName(event.args.to),
        });
      case "floor-transition-unavailable":
        return localization.format("message-floor-transition-unavailable");
      case "floor-expedition-ended":
        return localization.format("message-floor-expedition-ended");
      case "dungeon-entrance-guardian-defeated":
        return localization.format("message-dungeon-entrance-guardian-defeated", {
          dungeon: event.args.dungeon ?? "?",
        });
      case "campaign-victorious":
        return localization.format("message-campaign-victorious", {
          score: event.args.score ?? "?",
        });
      case "campaign-retired":
        return localization.format("message-campaign-retired", {
          score: event.args.score ?? "?",
        });
      case "campaign-retire-unavailable":
        return localization.format("message-campaign-retire-unavailable");
      case "player-experience-gained":
        return localization.format("message-player-experience-gained", {
          amount: event.args.amount ?? "?",
          total: event.args.total ?? "?",
        });
      case "player-level-gained":
        return localization.format("message-player-level-gained", {
          level: event.args.level ?? "?",
          maxHp: event.args.maxHp ?? "?",
          pending: event.args.pendingAttributeIncreases ?? "?",
        });
      case "player-level-cap-unlocked":
        return localization.format("message-player-level-cap-unlocked", {
          levelCap: event.args.levelCap ?? "?",
          attributeCap: event.args.attributeIndexCap ?? "?",
        });
      case "player-attribute-increased":
        return localization.format("message-player-attribute-increased", {
          attribute: localization.format(
            `attribute-${event.args.attribute ?? "unknown"}` as MessageKey,
          ),
          natural: formatAttributeValueArgument(event.args.natural),
          effective: formatAttributeValueArgument(event.args.effective),
          pending: event.args.pendingAttributeIncreases ?? "?",
        });
      case "player-attribute-increase-unavailable":
        return localization.format("message-player-attribute-increase-unavailable", {
          attribute: localization.format(
            `attribute-${event.args.attribute ?? "unknown"}` as MessageKey,
          ),
        });
      case "floor-one-shot-closed":
        return localization.format("message-floor-one-shot-closed");
      case "task-completed":
        return localization.format("message-task-completed");
      case "task-reward-available":
        return localization.format("message-task-reward-available");
      case "task-accepted":
        return localization.format("message-task-accepted", {
          task: contentName(event.args.task),
        });
      case "task-accept-unavailable":
        return localization.format("message-task-accept-unavailable");
      case "task-failed":
        return localization.format("message-task-failed");
      case "task-abandoned":
        return localization.format("message-task-abandoned");
      case "task-abandon-unavailable":
        return localization.format("message-task-abandon-unavailable");
      case "task-paused":
        return localization.format("message-task-paused");
      case "task-resumed":
        return localization.format("message-task-resumed");
      case "task-rewarded":
        return localization.format("message-task-rewarded", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
        });
      case "task-reward-claim-unavailable":
        return localization.format("message-task-reward-claim-unavailable");
      case "door-opened":
        return localization.format("message-door-opened");
      case "door-open-unavailable":
        return localization.format("message-door-open-unavailable");
      case "door-closed":
        return localization.format("message-door-closed");
      case "door-close-unavailable":
        return localization.format("message-door-close-unavailable");
      case "terrain-trap-triggered":
        return localization.format("message-terrain-trap-triggered", {
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "wilderness-interesting-discovery":
        return localization.format("message-wilderness-interesting-discovery");
      case "wilderness-terrain-damaged":
        return localization.format("message-wilderness-terrain-damaged", {
          terrain: contentName(event.args.terrain),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "terrain-trap-disarmed":
        return localization.format("message-terrain-trap-disarmed");
      case "terrain-trap-disarm-failed":
        return localization.format("message-terrain-trap-disarm-failed");
      case "terrain-trap-disarm-unavailable":
        return localization.format("message-terrain-trap-disarm-unavailable");
      case "terrain-dug":
        return localization.format("message-terrain-dug");
      case "terrain-dig-failed":
        return localization.format("message-terrain-dig-failed");
      case "terrain-dig-unavailable":
        return localization.format("message-terrain-dig-unavailable");
      case "combat-player-hit":
        return formatPlayerDamageEvent(event);
      case "combat-player-slay":
        return localization.format("message-combat-slay", {
          target: contentName(event.args.target),
        });
      case "combat-player-miss":
        return localization.format("message-combat-player-miss", {
          target: contentName(event.args.target),
        });
      case "combat-monster-miss":
        return localization.format("message-combat-monster-miss", {
          source: contentName(event.args.source),
        });
      case "combat-monster-hit":
        return formatMonsterDamageEvent(event);
      case "combat-monster-self-destructed":
        return localization.format("message-combat-monster-self-destructed", {
          source: contentName(event.args.source),
        });
      case "combat-monster-death-explosion-hit":
        return localization.format("message-combat-monster-death-explosion-hit", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "combat-monster-death-explosion-slew":
        return localization.format("message-combat-monster-death-explosion-slew", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "monster-terrain-destroyed":
        return localization.format("message-monster-terrain-destroyed", {
          source: contentName(event.args.source),
          terrain: contentName(event.args.terrain),
        });
      case "monster-warding-glyph-held":
      case "monster-warding-glyph-broken":
        return localization.format(`message-${event.messageKey}`, {
          source: contentName(event.args.source),
        });
      case "monster-item-destroyed":
        return localization.format("message-monster-item-destroyed", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
          quantity: event.args.quantity ?? "?",
        });
      case "monster-item-picked-up":
        return localization.format("message-monster-item-picked-up", {
          source: contentName(event.args.source),
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
        });
      case "monster-gold-theft-prevented":
        return localization.format("message-monster-gold-theft-prevented", {
          source: contentName(event.args.source),
        });
      case "monster-item-theft-prevented":
        return localization.format("message-monster-item-theft-prevented", {
          source: contentName(event.args.source),
        });
      case "monster-gold-stolen":
        return localization.format("message-monster-gold-stolen", {
          source: contentName(event.args.source),
          amount: event.args.amount ?? "?",
        });
      case "monster-item-stolen":
        return localization.format("message-monster-item-stolen", {
          source: contentName(event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "monster-food-eaten":
        return localization.format("message-monster-food-eaten", {
          source: contentName(event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "monster-light-eaten":
        return localization.format("message-monster-light-eaten", {
          source: contentName(event.args.source),
          target: visibleItemNameForKind(event.args.target),
          amount: event.args.amount ?? "?",
        });
      case "combat-monster-entity-miss":
        return localization.format("message-combat-monster-entity-miss", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "combat-monster-entity-hit":
        return localization.format("message-combat-monster-entity-hit", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "combat-monster-entity-slew":
        return localization.format("message-combat-monster-entity-slew", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "combat-monster-fled":
        return localization.format("message-combat-monster-fled", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "combat-monster-kept-distance":
        return localization.format("message-combat-monster-kept-distance", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "combat-summon-miss":
        return localization.format("message-combat-summon-miss", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "combat-summon-hit":
        return localization.format("message-combat-summon-hit", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
          damage: damageResolution(event)?.finalDamage ?? event.args.damage ?? "?",
        });
      case "combat-summon-slay":
        return localization.format("message-combat-summon-slay", {
          source: contentName(event.args.source),
          target: contentName(event.args.target),
        });
      case "combat-player-death":
        return localization.format("message-combat-player-death", {
          source: contentName(event.args.source),
        });
      case "projectile-unavailable":
        return localization.format("message-projectile-unavailable");
      case "projectile-ammo-unavailable":
        return localization.format("message-projectile-ammo-unavailable", {
          target: visibleItemNameForKind(event.args.target),
        });
      case "projectile-target-unavailable":
        return localization.format("message-projectile-target-unavailable");
      case "projectile-landed":
        return localization.format("message-projectile-landed");
      case "projectile-miss":
        return localization.format("message-projectile-miss", {
          target: contentName(event.args.target),
        });
      case "projectile-hit":
        return localization.format("message-projectile-hit", {
          target: contentName(event.args.target),
          damage: event.args.damage ?? "?",
        });
      case "projectile-slay":
        return localization.format("message-projectile-slay", {
          target: contentName(event.args.target),
        });
      case "projectile-ammo-recovered":
        return localization.format("message-projectile-ammo-recovered", {
          target: visibleItemNameForKind(event.args.target),
        });
      case "projectile-ammo-broken":
        return localization.format("message-projectile-ammo-broken", {
          target: visibleItemNameForKind(event.args.target),
        });
      case "status-player-damage":
        return localization.format("message-status-player-damage", {
          status: statusName(event.args.status),
          damage: event.args.damage ?? "?",
        });
      case "status-entity-damage":
        return localization.format("message-status-entity-damage", {
          target: contentName(event.args.target),
          status: statusName(event.args.status),
          damage: event.args.damage ?? "?",
        });
      case "status-player-expired":
        return localization.format("message-status-player-expired", {
          status: statusName(event.args.status),
        });
      case "status-entity-expired":
        return localization.format("message-status-entity-expired", {
          target: contentName(event.args.target),
          status: statusName(event.args.status),
        });
      case "status-player-death":
        return localization.format("message-status-player-death", {
          status: statusName(event.args.status),
        });
      case "hunger-state-changed":
        return localization.format("message-hunger-state-changed", {
          state: localization.format(`nutrition-state-${event.args.to ?? "normal"}`),
        });
      case "hunger-fainted":
        return localization.format("message-hunger-fainted", {
          duration: event.args.duration ?? "?",
        });
      case "hunger-starvation-damage":
        return localization.format("message-hunger-starvation-damage", {
          damage: event.args.damage ?? "?",
        });
      case "hunger-starvation-death":
        return localization.format("message-hunger-starvation-death");
      case "status-entity-death":
        return localization.format("message-status-entity-death", {
          target: contentName(event.args.target),
          status: statusName(event.args.status),
        });
      case "status-fear-blocked":
        return localization.format("message-status-fear-blocked", {
          status: statusName(event.args.status),
        });
      case "status-confused-move":
        return localization.format("message-status-confused-move");
      case "status-paralyzed":
        return localization.format("message-status-paralyzed");
      case "item-pickup-success":
        return localization.format("message-item-pickup-success", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
        });
      case "gold-pickup-success":
        return localization.format("message-gold-pickup-success", {
          amount: event.args.amount ?? "?",
          balance: event.args.balance ?? "?",
        });
      case "shop-purchase-success":
        return localization.format("shop-purchase-success", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
          totalPrice: event.args.totalPrice ?? "?",
          balance: event.args.balance ?? "?",
        });
      case "shop-sale-success":
        return localization.format("shop-sale-success", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
          totalPrice: event.args.totalPrice ?? "?",
          balance: event.args.balance ?? "?",
        });
      case "shop-transaction-unavailable":
        return localization.format("shop-transaction-unavailable", {
          reason: shopTransactionReason(event.args.reason),
        });
      case "home-deposit-success":
        return localization.format("home-deposit-success", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
        });
      case "home-withdraw-success":
        return localization.format("home-withdraw-success", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
        });
      case "home-transfer-unavailable":
        return localization.format("home-transfer-unavailable", {
          reason: shopTransactionReason(event.args.reason),
        });
      case "item-pickup-inventory-full":
        return localization.format("message-item-pickup-inventory-full", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
          usedSlots: event.args.usedSlots ?? "?",
          requiredSlots: event.args.requiredSlots ?? "?",
          capacity: event.args.capacity ?? "?",
        });
      case "item-pickup-none":
        return localization.format("message-item-pickup-none");
      case "item-equip-success":
        return localization.format("message-item-equip-success", {
          target: visibleItemNameForKind(event.args.target),
          slot: equipmentSlotName(event.args.slot),
        });
      case "item-equip-swap":
        return localization.format("message-item-equip-swap", {
          target: visibleItemNameForKind(event.args.target),
          replaced: visibleItemNameForKind(event.args.replaced),
          slot: equipmentSlotName(event.args.slot),
        });
      case "item-equip-unavailable":
        return localization.format("message-item-equip-unavailable");
      case "item-appraise-success":
        return localization.format("message-item-appraise-success", {
          target: visibleItemNameForKind(event.args.target),
          quality: itemQualityName(event.args.quality),
        });
      case "item-appraise-unavailable":
        return localization.format("message-item-appraise-unavailable");
      case "item-destroy-success":
        return localization.format("message-item-destroy-success", {
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
          ruleLine: Number(event.args.ruleLine ?? 0),
        });
      case "item-destroy-unavailable":
        return localization.format("message-item-destroy-unavailable", {
          reason: itemDestroyReason(event.args.reason),
          ruleLine: Number(event.args.ruleLine ?? 0),
        });
      case "item-inscribe-success":
        return localization.format("message-item-inscribe-success", {
          target: visibleItemNameForKind(event.args.target),
          inscription: event.args.inscription ?? "",
          ruleLine: Number(event.args.ruleLine ?? 0),
        });
      case "item-inscribe-cleared":
        return localization.format("message-item-inscribe-cleared", {
          target: visibleItemNameForKind(event.args.target),
          ruleLine: Number(event.args.ruleLine ?? 0),
        });
      case "item-inscribe-unavailable":
        return localization.format("message-item-inscribe-unavailable", {
          reason: itemDestroyReason(event.args.reason),
        });
      case "item-property-discovered":
        return localization.format("message-item-property-discovered", {
          target: visibleItemNameForKind(event.args.target),
          property: itemPropertyName(event.args.propertyNameKey),
        });
      case "loot-drop":
        return localization.format("message-loot-drop", {
          source: contentName(event.args.source),
          target: visibleItemNameForKind(event.args.target),
          quantity: event.args.quantity ?? "?",
        });
      case "gold-drop":
        return localization.format("message-gold-drop", {
          source: contentName(event.args.source),
          amount: event.args.amount ?? "?",
        });
      case "item-unequip-success":
        return localization.format("message-item-unequip-success", {
          target: visibleItemNameForKind(event.args.target),
          slot: equipmentSlotName(event.args.slot),
        });
      case "item-unequip-none":
        return localization.format("message-item-unequip-none", {
          slot: equipmentSlotName(event.args.slot),
        });
      case "item-unequip-cursed":
        return localization.format("message-item-unequip-cursed", {
          target: visibleItemNameForKind(event.args.target),
          slot: equipmentSlotName(event.args.slot),
          severity: itemCurseSeverityName(event.args.severity),
        });
      case "item-drop-success":
        return localization.format("message-item-drop-success", {
          stacks: event.args.stacks ?? "?",
          quantity: event.args.quantity ?? "?",
        });
      case "item-use-heal":
        return localization.format("message-item-use-heal", {
          target: visibleItemName(event.args.nameKey, event.args.target),
          amount: event.args.amount ?? "?",
        });
      case "item-use-food":
        return localization.format("message-item-use-food", {
          target: visibleItemName(event.args.nameKey, event.args.target),
          amount: event.args.amount ?? "?",
          nutrition: event.args.nutrition ?? "?",
        });
      case "item-use-hunger-satisfied":
      case "item-use-hunger-no-effect":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          target: visibleItemName(event.args.nameKey, event.args.target),
          nutrition: event.args.nutrition ?? "?",
        });
      case "item-experience-lost":
        return localization.format("message-item-experience-lost", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          amount: event.args.amount ?? "?",
          remaining: event.args.remaining ?? "?",
        });
      case "item-use-no-effect":
        return localization.format("message-item-use-no-effect", {
          target: visibleItemName(event.args.nameKey, event.args.target),
        });
      case "item-use-status-removed":
        return localization.format("message-item-use-status-removed", {
          target: visibleItemName(event.args.nameKey, event.args.target),
          status: statusName(event.args.status),
        });
      case "item-use-status-no-effect":
        return localization.format("message-item-use-status-no-effect", {
          target: visibleItemName(event.args.nameKey, event.args.target),
          status: statusName(event.args.status),
        });
      case "item-use-status-reduced":
        return localization.format("message-item-use-status-reduced", {
          target: visibleItemName(event.args.nameKey, event.args.target),
          status: statusName(event.args.status),
          before: event.args.before ?? "?",
          after: event.args.after ?? "?",
        });
      case "item-use-status-applied":
      case "item-use-status-resisted":
      case "item-use-status-no-new-effect":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          status: statusName(event.args.status),
          duration: event.args.duration ?? "?",
        });
      case "item-use-blessed":
        return localization.format("message-item-use-blessed", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-slowness-applied":
        return localization.format("message-item-use-slowness-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-slowness-no-effect":
        return localization.format("message-item-use-slowness-no-effect", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-speed":
        return localization.format("message-item-use-speed", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-heroism-applied":
        return localization.format("message-item-use-heroism-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-heroism-no-new-effect":
        return localization.format("message-item-use-heroism-no-new-effect", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-berserk-strength-applied":
        return localization.format("message-item-use-berserk-strength-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-berserk-strength-no-new-effect":
        return localization.format(
          "message-item-use-berserk-strength-no-new-effect",
          {
            source: visibleItemName(event.args.nameKey, event.args.source),
          },
        );
      case "item-use-poetic-inspiration-applied":
        return localization.format("message-item-use-poetic-inspiration-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-poetic-inspiration-no-new-effect":
        return localization.format(
          "message-item-use-poetic-inspiration-no-new-effect",
          {
            source: visibleItemName(event.args.nameKey, event.args.source),
          },
        );
      case "item-use-stone-skin-applied":
        return localization.format("message-item-use-stone-skin-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-stone-skin-no-new-effect":
        return localization.format("message-item-use-stone-skin-no-new-effect", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-restore-life-levels":
        return localization.format("message-item-use-restore-life-levels", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-restore-life-levels-no-effect":
        return localization.format("message-item-use-restore-life-levels-no-effect", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-restoration":
        return localization.format("message-item-use-restoration", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-restoration-no-effect":
        return localization.format("message-item-use-restoration-no-effect", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-attribute-drained":
      case "item-use-attribute-drain-no-effect":
      case "item-use-attribute-sustained":
      case "item-use-attribute-restored":
      case "item-use-attribute-restore-no-effect":
      case "item-use-attribute-increased":
      case "item-use-attribute-increase-no-effect":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          attribute: localization.format(
            `attribute-${event.args.attribute ?? "unknown"}` as MessageKey,
          ),
          before: event.args.before ?? "?",
          after: event.args.after ?? "?",
          maximum: event.args.maximum ?? "?",
        });
      case "item-use-poison-applied":
        return localization.format("message-item-use-poison-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-poison-resisted":
        return localization.format("message-item-use-poison-resisted", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-blindness-applied":
        return localization.format("message-item-use-blindness-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-blindness-no-new-effect":
        return localization.format("message-item-use-blindness-no-new-effect", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-blindness-resisted":
        return localization.format("message-item-use-blindness-resisted", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-thermal-resistance-applied":
        return localization.format("message-item-use-thermal-resistance-applied", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-thermal-resistance-no-effect":
        return localization.format("message-item-use-thermal-resistance-no-effect", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-basic-resistance":
        return localization.format("message-item-use-basic-resistance", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-life-loss":
      case "item-use-life-loss-death":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          amount: event.args.amount ?? "?",
        });
      case "item-use-detonation":
      case "item-use-detonation-death":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "item-use-vengeance":
        return localization.format("message-item-use-vengeance", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "item-use-protection-from-evil":
        return localization.format("message-item-use-protection-from-evil", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          duration: event.args.duration ?? "?",
        });
      case "combat-monster-repelled":
        return localization.format("message-combat-monster-repelled", {
          source: contentName(event.args.source),
        });
      case "item-use-confusing-strike-prepared":
        return localization.format("message-item-use-confusing-strike-prepared", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "combat-confusing-strike-immune":
      case "combat-confusing-strike-resisted":
      case "combat-confusing-strike-applied":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          target: contentName(event.args.target),
          duration: event.args.duration ?? "?",
        });
      case "combat-vengeance-hit":
        return localization.format("message-combat-vengeance-hit", {
          target: contentName(event.args.target),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "combat-vengeance-slay":
        return localization.format("message-combat-vengeance-slay", {
          target: contentName(event.args.target),
        });
      case "item-use-aggravate":
        return localization.format("message-item-use-aggravate", {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-mass-genocide":
        return localization.format("message-item-use-mass-genocide", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          removed: event.args.removed ?? "0",
          resisted: event.args.resisted ?? "0",
          fatigue: event.args.fatigue ?? "0",
        });
      case "item-use-genocide":
        return localization.format("message-item-use-genocide", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          glyph: event.args.glyph ?? "?",
          removed: event.args.removed ?? "0",
          resisted: event.args.resisted ?? "0",
          fatigue: event.args.fatigue ?? "0",
        });
      case "item-use-create-adjacent-terrain":
      case "item-use-create-adjacent-terrain-no-effect":
        return localization.format(`message-${event.messageKey}`, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          count: event.args.count ?? "0",
        });
      case "item-use-create-current-terrain":
      case "item-use-create-current-terrain-no-effect":
        return localization.format(`message-${event.messageKey}`, {
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-floor-light":
      case "item-use-floor-darkness":
      case "item-use-floor-glow-no-effect":
        return localization.format(`message-${event.messageKey}`, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          count: event.args.count ?? "0",
        });
      case "item-use-area-destruction":
      case "item-use-area-destruction-protected":
        return localization.format(`message-${event.messageKey}`, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          count: event.args.count ?? "0",
          entities: event.args.entities ?? "0",
          items: event.args.items ?? "0",
          gold: event.args.gold ?? "0",
        });
      case "item-use-elemental-blast":
        return localization.format("message-item-use-elemental-blast", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          count: event.args.count ?? "0",
        });
      case "item-use-elemental-blast-hit":
        return localization.format("message-item-use-elemental-blast-hit", {
          source: visibleItemNameForKind(event.args.source),
          target: contentName(event.args.target),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "item-use-elemental-blast-slay":
        return localization.format("message-item-use-elemental-blast-slay", {
          source: visibleItemNameForKind(event.args.source),
          target: contentName(event.args.target),
        });
      case "item-use-elemental-backlash":
      case "item-use-elemental-backlash-death":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          source: visibleItemNameForKind(event.args.source),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "item-use-destroy-adjacent-traps-doors":
      case "item-use-destroy-adjacent-traps-doors-no-effect":
        return localization.format(
          `message-${event.messageKey}` as MessageKey,
          {
            source: visibleItemName(event.args.nameKey, event.args.source),
            count: event.args.count ?? "0",
          },
        );
      case "item-use-resource-restored":
        return localization.format("message-item-use-resource-restored", {
          target: visibleItemName(event.args.nameKey, event.args.target),
          resource: contentName(event.args.resource),
          amount: event.args.amount ?? "?",
        });
      case "item-use-resource-no-effect":
        return localization.format("message-item-use-resource-no-effect", {
          target: visibleItemName(event.args.nameKey, event.args.target),
          resource: contentName(event.args.resource),
        });
      case "item-use-resource-drained":
      case "item-use-resource-drain-no-effect":
        return localization.format(`message-${event.messageKey}` as MessageKey, {
          source: visibleItemName(event.args.nameKey, event.args.source),
          resource: contentName(event.args.resource),
          amount: event.args.amount ?? "?",
        });
      case "item-use-identified":
        return localization.format("message-item-use-identified", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-use-fully-identified":
        return localization.format("message-item-use-fully-identified", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-use-inventory-identified":
        return localization.format("message-item-use-inventory-identified", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          count: Number(event.args.count ?? 0),
        });
      case "item-auto-identified":
        return localization.format("message-item-auto-identified", {
          count: Number(event.args.count ?? 0),
        });
      case "item-use-self-knowledge":
        return localization.format("message-item-use-self-knowledge", {
          ...event.args,
          source: visibleItemName(event.args.nameKey, event.args.source),
        });
      case "item-use-acquirement":
        return localization.format("message-item-use-acquirement", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          count: Number(event.args.count ?? 0),
        });
      case "item-use-mundanity":
        return localization.format("message-item-use-mundanity", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-use-crafting":
        return localization.format("message-item-use-crafting", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          target: visibleItemNameForKind(event.args.target),
          affix: contentName(event.args.affix),
        });
      case "item-use-rumour":
        return localization.format("message-item-use-rumour", {
          source: visibleItemName(event.args.nameKey, event.args.source),
          rumour: localization.format(event.args.rumourKey ?? ""),
        });
      case "item-use-enchanted":
        return localization.format("message-item-use-enchanted", {
          source: visibleItemNameForKind(event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-use-enchantment-failed":
        return localization.format("message-item-use-enchantment-failed", {
          source: visibleItemNameForKind(event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-use-cursed":
        return localization.format("message-item-use-cursed", {
          source: visibleItemNameForKind(event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-use-curse-resisted":
        return localization.format("message-item-use-curse-resisted", {
          source: visibleItemNameForKind(event.args.source),
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-use-curse-no-target":
        return localization.format("message-item-use-curse-no-target", {
          source: visibleItemNameForKind(event.args.source),
        });
      case "item-use-curses-removed":
        return localization.format("message-item-use-curses-removed", {
          source: visibleItemNameForKind(event.args.source),
          count: event.args.count ?? "0",
        });
      case "item-use-curse-removal-no-effect":
        return localization.format("message-item-use-curse-removal-no-effect", {
          source: visibleItemNameForKind(event.args.source),
        });
      case "item-use-unavailable":
        return localization.format("message-item-use-unavailable");
      case "device-energy-recovered":
        return localization.format("message-device-energy-recovered", {
          target: visibleItemNameForKind(event.args.target),
          amount: event.args.amount ?? "?",
          current: event.args.current ?? "?",
          maximum: event.args.maximum ?? "?",
        });
      case "device-recharge-unavailable":
        return localization.format("message-device-recharge-unavailable");
      case "device-recharge-success": {
        const success = localization.format("message-device-recharge-success", {
          target: visibleItemNameForKind(event.args.target),
          source:
            event.args.sourceType === "item"
              ? visibleItemNameForKind(event.args.source)
              : contentName(event.args.source),
          amount: event.args.attempted ?? "?",
          current: event.args.after ?? "?",
        });
        return event.args.sourceDestroyed === "true"
          ? `${success} ${localization.format("message-device-recharge-source-destroyed")}`
          : success;
      }
      case "device-recharge-failure": {
        const failure = localization.format("message-device-recharge-failure", {
          target: visibleItemNameForKind(event.args.target),
          source:
            event.args.sourceType === "item"
              ? visibleItemNameForKind(event.args.source)
              : contentName(event.args.source),
        });
        return event.args.sourceDestroyed === "true"
          ? `${failure} ${localization.format("message-device-recharge-source-destroyed")}`
          : failure;
      }
      case "light-refuel-unavailable":
        return localization.format("message-light-refuel-unavailable");
      case "light-refueled":
        return localization.format("message-light-refueled", {
          target: visibleItemNameForKind(event.args.target),
          source: visibleItemNameForKind(event.args.source),
          amount: event.args.amount ?? "?",
          current: event.args.current ?? "?",
          maximum: event.args.maximum ?? "?",
        });
      case "light-extinguished":
        return localization.format("message-light-extinguished", {
          target: visibleItemNameForKind(event.args.target),
        });
      case "item-activation-landed":
        return localization.format("message-item-activation-landed", {
          source: visibleItemNameForKind(event.args.source),
        });
      case "item-activation-hit":
        return localization.format("message-item-activation-hit", {
          source: visibleItemNameForKind(event.args.source),
          target: contentName(event.args.target),
          damage: damageResolution(event)?.finalDamage ?? "?",
        });
      case "item-activation-slay":
        return localization.format("message-item-activation-slay", {
          source: visibleItemNameForKind(event.args.source),
          target: contentName(event.args.target),
        });
      case "item-activation-detected":
        return localization.format("message-item-activation-detected", {
          source: visibleItemNameForKind(event.args.source),
          count: event.args.count ?? "0",
        });
      case "item-use-detected":
        return localization.format("message-item-use-detected", {
          source: visibleItemNameForKind(event.args.source),
          count: event.args.count ?? "0",
        });
      case "item-use-teleported":
      case "item-activation-teleported":
        return localization.format(
          event.messageKey === "item-activation-teleported"
            ? "message-item-activation-teleported"
            : "message-item-use-teleported",
          {
            source: visibleItemNameForKind(event.args.source),
            fromX: event.args.fromX ?? "?",
            fromY: event.args.fromY ?? "?",
            toX: event.args.toX ?? "?",
            toY: event.args.toY ?? "?",
          },
        );
      case "item-use-teleported-level":
        return localization.format("message-item-use-teleported-level", {
          source: visibleItemNameForKind(event.args.source),
          from: floorName(event.args.from),
          to: floorName(event.args.to),
        });
      case "item-recall-started":
        return localization.format("message-item-recall-started", {
          source: visibleItemNameForKind(event.args.source),
          floor: floorName(event.args.floor),
          turns: event.args.turns ?? "?",
        });
      case "item-recall-cancelled":
        return localization.format("message-item-recall-cancelled", {
          source: visibleItemNameForKind(event.args.source),
        });
      case "item-recall-reset":
        return localization.format("message-item-recall-reset", {
          source: visibleItemNameForKind(event.args.source),
          floor: floorName(event.args.floor),
        });
      case "item-recall-triggered":
        return localization.format("message-item-recall-triggered", {
          from: floorName(event.args.from),
          to: floorName(event.args.to),
        });
      case "item-thrown":
        return localization.format("message-item-thrown", {
          target: visibleItemNameForKind(event.args.target),
        });
      case "throw-miss":
        return localization.format("message-throw-miss", {
          source: visibleItemNameForKind(event.args.source),
          target: contentName(event.args.target),
        });
      case "throw-hit":
        return localization.format("message-throw-hit", {
          source: visibleItemNameForKind(event.args.source),
          target: contentName(event.args.target),
          damage: event.args.damage ?? "?",
        });
      case "throw-slay":
        return localization.format("message-throw-slay", {
          source: visibleItemNameForKind(event.args.source),
          target: contentName(event.args.target),
        });
      case "item-throw-unavailable":
        return localization.format("message-item-throw-unavailable");
      case "item-drop-none":
        return localization.format("message-item-drop-none");
      default:
        return localization.format("message-unknown-event", { key: event.messageKey });
    }
  }

  function formatPlayerDamageEvent(event: GameEventDto): string {
    const target = contentName(event.args.target);
    const resolution = damageResolution(event);
    if (!resolution) {
      return localization.format("message-combat-hit", {
        target,
        damage: event.args.damage ?? "?",
      });
    }
    const args = {
      target,
      damage: resolution.finalDamage,
      type: damageTypeName(resolution.damageType),
      adjustment: Math.abs(resolution.resistanceAdjustment),
    };
    if (resolution.resistance === "immune") {
      return localization.format("message-combat-hit-immune", args);
    }
    if (resolution.resistanceAdjustment > 0) {
      return localization.format("message-combat-hit-resisted", args);
    }
    if (resolution.resistanceAdjustment < 0) {
      return localization.format("message-combat-hit-amplified", args);
    }
    return localization.format("message-combat-hit", args);
  }

  function formatMonsterDamageEvent(event: GameEventDto): string {
    const source = contentName(event.args.source);
    const resolution = damageResolution(event);
    if (!resolution) {
      return localization.format("message-combat-monster-hit", {
        source,
        damage: event.args.damage ?? "?",
      });
    }
    const args = {
      source,
      damage: resolution.finalDamage,
      type: damageTypeName(resolution.damageType),
      adjustment: Math.abs(resolution.resistanceAdjustment),
    };
    if (resolution.resistance === "immune") {
      return localization.format("message-combat-monster-hit-immune", args);
    }
    if (resolution.resistanceAdjustment > 0) {
      return localization.format("message-combat-monster-hit-resisted", args);
    }
    if (resolution.resistanceAdjustment < 0) {
      return localization.format("message-combat-monster-hit-amplified", args);
    }
    return localization.format("message-combat-monster-hit", args);
  }

  function damageResolution(event: GameEventDto): DamageResolutionDto | undefined {
    const outcome = event.outcome;
    return outcome?.type === "damage" || outcome?.type === "death"
      ? outcome.resolution
      : undefined;
  }

  function damageTypeName(damageType: DamageTypeDto): string {
    const keys: Record<DamageTypeDto, MessageKey> = {
      physical: "damage-type-physical-name",
      acid: "damage-type-acid-name",
      electricity: "damage-type-electricity-name",
      fire: "damage-type-fire-name",
      cold: "damage-type-cold-name",
      poison: "damage-type-poison-name",
      light: "damage-type-light-name",
      dark: "damage-type-dark-name",
      blindness: "damage-type-blindness-name",
      fear: "damage-type-fear-name",
      confusion: "damage-type-confusion-name",
      nether: "damage-type-nether-name",
      nexus: "damage-type-nexus-name",
      sound: "damage-type-sound-name",
      shards: "damage-type-shards-name",
      chaos: "damage-type-chaos-name",
      disenchant: "damage-type-disenchant-name",
      time: "damage-type-time-name",
      mana: "damage-type-mana-name",
      gravity: "damage-type-gravity-name",
      inertia: "damage-type-inertia-name",
      plasma: "damage-type-plasma-name",
      force: "damage-type-force-name",
      nuke: "damage-type-nuke-name",
      disintegrate: "damage-type-disintegrate-name",
      storm: "damage-type-storm-name",
      "holy-fire": "damage-type-holy-fire-name",
      "hell-fire": "damage-type-hell-fire-name",
      ice: "damage-type-ice-name",
      water: "damage-type-water-name",
      psi: "damage-type-psi-name",
      curse: "damage-type-curse-name",
    };
    return localization.format(keys[damageType]);
  }

  function floorName(id: string | undefined): string {
    if (id === "demo.floor.surface") {
      return localization.format(
        getState().currentWorldId === "demo.world.warrens-journey"
          ? "floor-demo-surface-name"
          : "world-demo-original-lab-name",
      );
    }
    if (id === "demo.floor.echo-depth-1") {
      return localization.format("floor-demo-echo-depth-1-name");
    }
    if (/^demo\.floor\.warrens-depth-\d+$/.test(id ?? "")) {
      return localization.format("floor-demo-warrens-depth-name");
    }
    return id ?? "?";
  }

  function contentName(id: string | undefined): string {
    if (id === "demo.resource.mana") {
      return localization.format("resource-demo-mana-name");
    }
    if (id === "demo.ability.resonant-bolt") {
      return localization.format("ability-demo-resonant-bolt-name");
    }
    if (id === "demo.ability.harmonic-spark") {
      return localization.format("ability-demo-harmonic-spark-name");
    }
    if (id === "demo.ability.echo-burst") {
      return localization.format("ability-demo-echo-burst-name");
    }
    if (id === "demo.ability.echo-companion") {
      return localization.format("ability-demo-echo-companion-name");
    }
    if (id === "demo.ability.echo-pulse") {
      return localization.format("ability-demo-echo-pulse-name");
    }
    if (id === "demo.ability.echo-sight") {
      return localization.format("ability-demo-echo-sight-name");
    }
    if (id === "demo.ability.echo-delving") {
      return localization.format("ability-demo-echo-delving-name");
    }
    if (id === "demo.ability.echo-rampart") {
      return localization.format("ability-demo-echo-rampart-name");
    }
    if (id === "demo.ability.echo-binding") {
      return localization.format("ability-demo-echo-binding-name");
    }
    if (id === "demo.ability.echo-quickening") {
      return localization.format("ability-demo-echo-quickening-name");
    }
    if (id === "demo.ability.mending-echo") {
      return localization.format("ability-demo-mending-echo-name");
    }
    if (id === "demo.item.echo-primer") {
      return localization.format("item-demo-echo-primer-name");
    }
    if (id === "demo.item.stillwater-notes") {
      return localization.format("item-demo-stillwater-notes-name");
    }
    if (id === "demo.item.luminous-shard") {
      return localization.format("item-demo-luminous-shard-name");
    }
    if (id === "demo.item.echo-charm") {
      return localization.format("item-demo-echo-charm-name");
    }
    if (id === "demo.item.echo-blade") {
      return localization.format("item-demo-echo-blade-name");
    }
    if (id === "demo.item.resonance-sling") {
      return localization.format("item-demo-resonance-sling-name");
    }
    if (id === "demo.item.resonance-pellet") {
      return localization.format("item-demo-resonance-pellet-name");
    }
    if (id === "demo.actor.ember-mote") {
      return localization.format("actor-demo-ember-mote-name");
    }
    if (id === "demo.actor.acid-seep") {
      return localization.format("actor-demo-acid-seep-name");
    }
    if (id === "demo.actor.storm-spark") {
      return localization.format("actor-demo-storm-spark-name");
    }
    if (id === "demo.actor.frost-wisp") {
      return localization.format("actor-demo-frost-wisp-name");
    }
    if (id === "demo.actor.venom-spore") {
      return localization.format("actor-demo-venom-spore-name");
    }
    if (id === "demo.actor.echo-hound") {
      return localization.format("actor-demo-echo-hound-name");
    }
    if (id === "demo.actor.echo-cantor") {
      return localization.format("actor-demo-echo-cantor-name");
    }
    if (id === "demo.terrain.floor") {
      return localization.format("terrain-demo-floor-name");
    }
    if (id === "demo.terrain.wall") {
      return localization.format("terrain-demo-wall-name");
    }
    if (id === "demo.terrain.stairs-down") {
      return localization.format("terrain-demo-stairs-down-name");
    }
    if (id === "demo.terrain.stairs-up") {
      return localization.format("terrain-demo-stairs-up-name");
    }
    if (id === "demo.terrain.surface-grass") {
      return localization.format("terrain-demo-surface-grass-name");
    }
    if (id === "demo.terrain.surface-path") {
      return localization.format("terrain-demo-surface-path-name");
    }
    if (id === "demo.terrain.surface-rock") {
      return localization.format("terrain-demo-surface-rock-name");
    }
    if (id === "demo.terrain.surface-tree") {
      return localization.format("terrain-demo-surface-tree-name");
    }
    if (id === "demo.terrain.echo-rubble") {
      return localization.format("terrain-demo-echo-rubble-name");
    }
    if (id) {
      const [namespace, kind, ...nameParts] = id.split(".");
      const derivedNameKey = `${kind}-${namespace}-${nameParts.join("-")}-name`;
      if (
        localization.hasMessage(localization.locale, derivedNameKey) ||
        localization.hasMessage("en-US", derivedNameKey)
      ) {
        return localization.format(derivedNameKey);
      }
    }
    return localization.format(
      id?.startsWith("demo.item.") ? "item-unknown-name" : "actor-unknown-name",
    );
  }

  function abilityUnavailableReason(reason: string | undefined): string {
    return localization.format(`ability-unavailable-${reason ?? "unknown"}` as MessageKey);
  }

  function restStopReason(reason: string | undefined): string {
    return localization.format(`rest-stop-${reason ?? "unknown"}` as MessageKey);
  }

  function shopTransactionReason(reason: string | undefined): string {
    const key = `shop-transaction-reason-${reason ?? "unknown"}`;
    return localization.hasMessage(localization.locale, key) || localization.hasMessage("en-US", key)
      ? localization.format(key)
      : localization.format("shop-transaction-reason-unknown");
  }

  function itemDestroyReason(reason: string | undefined): string {
    const key = `item-destroy-reason-${reason ?? "unknown"}`;
    return localization.hasMessage(localization.locale, key) || localization.hasMessage("en-US", key)
      ? localization.format(key)
      : localization.format("item-destroy-reason-unknown");
  }

  function visibleItemName(
    displayNameKey: string | undefined,
    fallbackKindId: string | undefined,
  ): string {
    if (displayNameKey && localization.hasMessage(localization.locale, displayNameKey)) {
      return localization.format(displayNameKey);
    }
    return contentName(fallbackKindId);
  }

  function visibleItemNameForKind(kindId: string | undefined): string {
    if (!kindId) return localization.format("item-unknown-name");
    const { currentInventory, currentEquipment, currentStatus } = getState();
    const projected =
      currentInventory.find((item) => item.kindId === kindId) ??
      currentEquipment.find((item) => item.kindId === kindId) ??
      currentStatus?.items.find((item) => item.kindId === kindId);
    if (projected) return visibleItemName(projected.displayNameKey, kindId);
    if (kindId === "demo.item.luminous-shard") {
      return localization.format("item-demo-unfamiliar-shard-name");
    }
    return contentName(kindId);
  }

  function itemPropertyName(nameKey: string | undefined): string {
    if (nameKey === "affix-demo-harmonic-edge-name") {
      return localization.format(nameKey);
    }
    return localization.format("item-unknown-name");
  }

  function itemQualityName(quality: string | undefined): string {
    switch (quality) {
      case "ordinary":
        return localization.format("item-quality-ordinary");
      case "fine":
        return localization.format("item-quality-fine");
      case "exceptional":
        return localization.format("item-quality-exceptional");
      default:
        return "?";
    }
  }

  const EQUIPMENT_SLOT_TYPE_KEYS: Record<string, MessageKey> = {
    charm: "equipment-slot-charm",
    weapon: "equipment-slot-weapon",
    launcher: "equipment-slot-launcher",
    body: "equipment-slot-body",
    head: "equipment-slot-head",
    shield: "equipment-slot-shield",
    cloak: "equipment-slot-cloak",
    gloves: "equipment-slot-gloves",
    boots: "equipment-slot-boots",
    ring: "equipment-slot-ring",
    amulet: "equipment-slot-amulet",
    light: "equipment-slot-light",
    container: "equipment-slot-container",
    tool: "equipment-slot-tool",
  };

  function equipmentSlotName(slotType: string | undefined): string {
    const key = slotType ? EQUIPMENT_SLOT_TYPE_KEYS[slotType] : undefined;
    if (key) return localization.format(key);
    return localization.format("equipment-slot-unknown", { slot: slotType ?? "?" });
  }

  function statusName(statusId: string | undefined): string {
    if (statusId === "rfb.status.bleeding") {
      return localization.format("status-bleeding-name");
    }
    if (statusId === "rfb.status.poison") {
      return localization.format("status-poison-name");
    }
    if (statusId === "rfb.status.haste") {
      return localization.format("status-haste-name");
    }
    if (statusId === "rfb.status.slow") {
      return localization.format("status-slow-name");
    }
    if (statusId === "rfb.status.stun") {
      return localization.format("status-stun-name");
    }
    if (statusId === "rfb.status.fear") {
      return localization.format("status-fear-name");
    }
    if (statusId === "rfb.status.confusion") {
      return localization.format("status-confusion-name");
    }
    if (statusId === "rfb.status.hallucination") {
      return localization.format("status-hallucination-name");
    }
    if (statusId === "rfb.status.blindness") {
      return localization.format("status-blindness-name");
    }
    if (statusId === "rfb.status.paralysis") {
      return localization.format("status-paralysis-name");
    }
    if (statusId === "rfb.status.blessed") {
      return localization.format("status-blessed-name");
    }
    if (statusId === "rfb.status.vengeance") {
      return localization.format("status-vengeance-name");
    }
    if (statusId === "rfb.status.protection-from-evil") {
      return localization.format("status-protection-from-evil-name");
    }
    if (statusId === "rfb.status.sight") {
      return localization.format("status-sight-name");
    }
    if (statusId === "rfb.status.poison-resistance") {
      return localization.format("status-poison-resistance-name");
    }
    if (statusId === "rfb.status.invulnerability") {
      return localization.format("status-invulnerability-name");
    }
    if (statusId === "rfb.status.giant-strength") {
      return localization.format("status-giant-strength-name");
    }
    if (statusId === "rfb.status.understanding") {
      return localization.format("status-understanding-name");
    }
    if (statusId === "rfb.status.inventory-protection") {
      return localization.format("status-inventory-protection-name");
    }
    return localization.format("status-unknown-name");
  }

  return {
    formatEvent,
    damageTypeName,
    floorName,
    contentName,
    visibleItemName,
    visibleItemNameForKind,
    itemPropertyName,
    itemQualityName,
    equipmentSlotName,
    statusName,
  };
}

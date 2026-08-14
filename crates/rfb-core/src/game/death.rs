// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::MeleeBlowEffectDefinition;
use rfb_protocol::{ItemEnchantmentsDto, ItemQualityDto, MonsterPackRoleDto, Position};

use crate::{
    effect::{
        DamageOutcome, DamagePacket, STATUS_BLEEDING, STATUS_PARALYSIS, STATUS_SLOW, STATUS_STUN,
        resolve_damage,
    },
    error::CoreError,
    event::DomainEvent,
    resistance::{DamageType, ResistanceLevel},
    state::{Actor, GoldPile, ItemInstance, ItemLocation, SummonIdentity},
};

use super::{
    ActorDeathRecord, CurseEquippedItemRequest, EquippedItemCurseTarget, FatalityPolicy, Game,
    INITIAL_MONSTER_ENERGY_NEED, commit_damage_application, initial_item_curse,
    initial_item_runtime_state, plan_damage_application, rfb_area_damage,
    spawn_actor_from_definition,
};
use crate::save::initial_item_fuel;

const VARIANT_MAINTAINER_KIND_ID: &str = "demo.actor.the-variant-maintainer";
const SOFTWARE_BUG_KIND_ID: &str = "demo.actor.software-bug";
const SOFTWARE_BUG_DEATH_SUMMON_SOURCE_ID: &str =
    "rfb-legacy.ability.summon-software-bug-l14-1d3-1";
const SOFTWARE_BUG_DEATH_SUMMON_COUNT: usize = 4;
const LEGACY_SUMMON_DURATION_TURNS: u16 = 10_000;

struct CarriedDrop {
    item_id: String,
    kind_id: String,
    quantity: u32,
}

struct ActorDeathPlan {
    actor: Actor,
    corpse: Option<ItemInstance>,
    generated_loot: Vec<ItemInstance>,
    generated_gold: Vec<GoldPile>,
    carried: Vec<CarriedDrop>,
    has_drops: bool,
    dissolved_pack_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct BombDamage {
    damage: DamageOutcome,
    shard_damage: i32,
    sound_damage: i32,
    shards_resisted: bool,
    sound_resisted: bool,
}

fn resistance_prevents_status(resistance: ResistanceLevel) -> bool {
    matches!(
        resistance,
        ResistanceLevel::Resistant | ResistanceLevel::Strong | ResistanceLevel::Immune
    )
}

fn rfb_bomb_damage(
    raw_damage: i32,
    distance: u32,
    shard_resistance: ResistanceLevel,
    sound_resistance: ResistanceLevel,
) -> BombDamage {
    let raw_damage = raw_damage.max(0);
    let mut sound_raw = raw_damage.saturating_mul(2).saturating_add(2) / 3;
    let mut shard_raw = raw_damage.saturating_sub(sound_raw);
    for _ in 0..distance {
        shard_raw = shard_raw.saturating_sub(shard_raw / 5);
    }
    sound_raw = sound_raw.saturating_add(i32::try_from(distance).unwrap_or(i32::MAX))
        / i32::try_from(distance.saturating_add(1)).unwrap_or(i32::MAX);

    let shards = resolve_damage(
        DamagePacket::new(shard_raw, DamageType::Shards),
        shard_resistance,
    );
    let sound = resolve_damage(
        DamagePacket::new(sound_raw, DamageType::Sound),
        sound_resistance,
    );
    let prepared_raw = shard_raw.saturating_add(sound_raw);
    let applied = shards.applied.saturating_add(sound.applied);
    BombDamage {
        damage: DamageOutcome {
            raw: prepared_raw,
            armor_reduction: 0,
            requested: prepared_raw,
            applied,
            resistance_delta: prepared_raw.saturating_sub(applied),
            damage_type: DamageType::Shards,
            resistance: ResistanceLevel::Normal,
        },
        shard_damage: shards.applied,
        sound_damage: sound.applied,
        shards_resisted: resistance_prevents_status(shard_resistance),
        sound_resisted: resistance_prevents_status(sound_resistance),
    }
}

impl Game {
    fn summon_variant_maintainer_software_bugs(
        &mut self,
        actor: &Actor,
        changed: &mut BTreeSet<Position>,
    ) {
        if actor.kind_id != VARIANT_MAINTAINER_KIND_ID {
            return;
        }
        let definition = self
            .content
            .actor(SOFTWARE_BUG_KIND_ID)
            .expect("Variant Maintainer requires the Software bug actor")
            .clone();
        let positions = self
            .open_positions_around_for_actor_kind(actor.position, 2, SOFTWARE_BUG_KIND_ID)
            .into_iter()
            .take(SOFTWARE_BUG_DEATH_SUMMON_COUNT)
            .collect::<Vec<_>>();
        for (ordinal, position) in positions.into_iter().enumerate() {
            let id = self.summon_entity_id(SOFTWARE_BUG_DEATH_SUMMON_SOURCE_ID, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.summon = Some(SummonIdentity {
                owner_id: actor.id.clone(),
                source_ability_id: SOFTWARE_BUG_DEATH_SUMMON_SOURCE_ID.to_owned(),
                remaining_turns: LEGACY_SUMMON_DURATION_TURNS,
            });
            changed.insert(position);
            self.entities.push(entity);
        }
    }

    fn apply_death_explosion_slow(&mut self, actor: &Actor, cells: &[(u32, Position)]) {
        if cells
            .iter()
            .any(|(_, position)| *position == self.player.position)
            && !self.player_is_dead()
            && !self.player_status_immunities().contains(STATUS_PARALYSIS)
        {
            self.minor_slow = self.minor_slow.saturating_add(1).min(10);
        }

        let target_ids = cells
            .iter()
            .filter_map(|(_, position)| {
                self.entities
                    .iter()
                    .find(|entity| {
                        entity.id != actor.id && entity.hp > 0 && entity.position == *position
                    })
                    .map(|entity| entity.id.clone())
            })
            .collect::<Vec<_>>();
        for target_id in target_ids {
            let target_index = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id)
                .expect("death explosion slow target must remain available");
            let inertia_resistance = self.entities[target_index]
                .resistances
                .level(DamageType::Inertia);
            let (level, unique) = self
                .actor_runtime_definition(&self.entities[target_index])
                .map_or((1, false), |definition| {
                    (
                        definition.level.max(1),
                        definition
                            .tags
                            .iter()
                            .any(|tag| matches!(tag.as_str(), "unique" | "unique2")),
                    )
                });
            if resistance_prevents_status(inertia_resistance) || unique {
                continue;
            }
            let level_roll = self.rng.bounded(u64::from(level)) + 1;
            let power_roll = self.rng.bounded(62) + 1;
            if level_roll > power_roll {
                continue;
            }
            self.apply_actor_melee_status(target_index, STATUS_SLOW, 25, &actor.kind_id);
        }
    }

    fn apply_amberite_blood_curse(&mut self, actor: &Actor) {
        let Some(level) = self.content.actor(&actor.kind_id).and_then(|definition| {
            definition
                .tags
                .iter()
                .any(|tag| tag == "amberite")
                .then(|| u16::try_from(definition.level).unwrap_or(u16::MAX))
        }) else {
            return;
        };
        if self.player_is_dead() || self.rng.bounded(2) != 0 {
            return;
        }

        self.curse_equipped_item(
            CurseEquippedItemRequest::new(EquippedItemCurseTarget::Any).with_heavy_chance(50),
        );
        let curse_count = self.rng.bounded(3) + 2;
        for _ in 0..curse_count {
            self.apply_nonlethal_ty_curse(level, &actor.kind_id);
        }
    }

    fn actor_death_explosion(
        &mut self,
        actor: &Actor,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(blow) = self
            .content
            .actor(&actor.kind_id)
            .and_then(|definition| definition.melee_routine.as_ref())
            .and_then(|routine| routine.blows.iter().find(|blow| blow.self_destructs))
            .cloned()
        else {
            return Ok(());
        };
        let source_is_player_aligned = self.actor_is_player_aligned(actor);

        let cells = self.area_damage_cells(actor.position, 3);
        changed.extend(cells.iter().map(|(_, position)| *position));
        for effect in &blow.effects {
            if matches!(effect, MeleeBlowEffectDefinition::Slow { .. }) {
                self.apply_death_explosion_slow(actor, &cells);
                continue;
            }
            let (damage_dice, damage_sides, damage_type, bomb) = match effect {
                MeleeBlowEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_type,
                    ..
                } => (
                    *damage_dice,
                    *damage_sides,
                    DamageType::from(*damage_type),
                    false,
                ),
                MeleeBlowEffectDefinition::Poison {
                    damage_dice,
                    damage_sides,
                    ..
                } => (*damage_dice, *damage_sides, DamageType::Poison, false),
                MeleeBlowEffectDefinition::Bomb {
                    damage_dice,
                    damage_sides,
                    ..
                } => (*damage_dice, *damage_sides, DamageType::Shards, true),
                MeleeBlowEffectDefinition::Disease { .. }
                | MeleeBlowEffectDefinition::Shatter { .. }
                | MeleeBlowEffectDefinition::DrainAttributes { .. }
                | MeleeBlowEffectDefinition::DrainResource { .. }
                | MeleeBlowEffectDefinition::DrainCharges { .. }
                | MeleeBlowEffectDefinition::DrainExperience { .. }
                | MeleeBlowEffectDefinition::Unlife { .. }
                | MeleeBlowEffectDefinition::Bleeding { .. }
                | MeleeBlowEffectDefinition::Blind { .. }
                | MeleeBlowEffectDefinition::Confusion { .. }
                | MeleeBlowEffectDefinition::Paralysis { .. }
                | MeleeBlowEffectDefinition::Amnesia { .. }
                | MeleeBlowEffectDefinition::Time { .. }
                | MeleeBlowEffectDefinition::Slow { .. }
                | MeleeBlowEffectDefinition::Inertia { .. }
                | MeleeBlowEffectDefinition::PolymorphPlayer { .. }
                | MeleeBlowEffectDefinition::Stun { .. }
                | MeleeBlowEffectDefinition::Terrify { .. }
                | MeleeBlowEffectDefinition::Disenchant { .. }
                | MeleeBlowEffectDefinition::EatGold { .. }
                | MeleeBlowEffectDefinition::EatItem { .. }
                | MeleeBlowEffectDefinition::EatFood { .. }
                | MeleeBlowEffectDefinition::EatLight { .. } => {
                    unreachable!("validated death explosions only contain projected effects")
                }
            };
            let raw_damage = self.roll_damage(damage_dice, damage_sides);
            for (distance, position) in &cells {
                let prepared_damage = rfb_area_damage(raw_damage, *distance);
                if self.player.position == *position && !self.player_is_dead() {
                    let bomb_damage = bomb.then(|| {
                        rfb_bomb_damage(
                            raw_damage,
                            *distance,
                            self.effective_player_resistances()
                                .level(DamageType::Shards),
                            self.effective_player_resistances().level(DamageType::Sound),
                        )
                    });
                    let damage = self.reduce_player_damage(bomb_damage.map_or_else(
                        || {
                            resolve_damage(
                                DamagePacket::new(prepared_damage, damage_type),
                                self.effective_player_resistances().level(damage_type),
                            )
                        },
                        |damage| damage.damage,
                    ));
                    let application =
                        self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
                    let damage = application.damage;
                    if !application.fatal
                        && let Some(bomb_damage) = bomb_damage
                    {
                        if !bomb_damage.shards_resisted {
                            self.apply_player_melee_status(
                                STATUS_BLEEDING,
                                bomb_damage.shard_damage,
                                &actor.kind_id,
                            );
                        }
                        if !bomb_damage.sound_resisted && bomb_damage.sound_damage > 0 {
                            let maximum = if bomb_damage.sound_damage > 90 {
                                35
                            } else {
                                bomb_damage.sound_damage / 3 + 5
                            };
                            let duration = i32::try_from(self.rng.bounded(maximum as u64) + 1)
                                .expect("bomb stun duration must fit i32");
                            self.apply_player_melee_status(STATUS_STUN, duration, &actor.kind_id);
                        }
                    }
                    events.push(DomainEvent::MonsterDeathExplosionHit {
                        source_kind_id: actor.kind_id.clone(),
                        target_kind_id: self.player.kind_id.clone(),
                        damage,
                    });
                    if application.fatal {
                        events.push(DomainEvent::PlayerDied {
                            source_kind_id: actor.kind_id.clone(),
                            method_id: Some(blow.method_id.clone()),
                            damage,
                        });
                    }
                }

                let Some(target_id) = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.id != actor.id && entity.hp > 0 && entity.position == *position
                    })
                    .map(|entity| entity.id.clone())
                else {
                    continue;
                };
                let target_index = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == target_id)
                    .expect("death explosion target must remain available");
                let target_is_player_aligned = self.entity_is_player_aligned(target_index);
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let bomb_damage = bomb.then(|| {
                    rfb_bomb_damage(
                        raw_damage,
                        *distance,
                        self.entities[target_index]
                            .resistances
                            .level(DamageType::Shards),
                        self.entities[target_index]
                            .resistances
                            .level(DamageType::Sound),
                    )
                });
                let damage = bomb_damage.map_or_else(
                    || {
                        resolve_damage(
                            DamagePacket::new(prepared_damage, damage_type),
                            self.entities[target_index].resistances.level(damage_type),
                        )
                    },
                    |damage| damage.damage,
                );
                let application = plan_damage_application(
                    &self.entities[target_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[target_index], &application);
                self.wake_entity_after_damage(target_index, damage.applied, events);
                if !application.fatal
                    && let Some(bomb_damage) = bomb_damage
                    && !bomb_damage.sound_resisted
                    && bomb_damage.sound_damage > 0
                {
                    let maximum = if bomb_damage.sound_damage > 90 {
                        35
                    } else {
                        bomb_damage.sound_damage / 3 + 5
                    };
                    let duration = i32::try_from(self.rng.bounded(maximum as u64) + 1)
                        .expect("bomb stun duration must fit i32");
                    self.apply_actor_melee_status(
                        target_index,
                        STATUS_STUN,
                        duration,
                        &actor.kind_id,
                    );
                }
                if application.fatal {
                    let death_event = DomainEvent::MonsterDeathExplosionSlew {
                        source_kind_id: actor.kind_id.clone(),
                        target_kind_id,
                        damage,
                    };
                    if target_is_player_aligned {
                        self.resolve_actor_death_without_rewards(
                            target_index,
                            Some(death_event),
                            events,
                            changed,
                            removed_entities,
                        )?;
                    } else {
                        self.resolve_actor_death_with_credit(
                            target_index,
                            death_event,
                            source_is_player_aligned,
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                } else {
                    events.push(DomainEvent::MonsterDeathExplosionHit {
                        source_kind_id: actor.kind_id.clone(),
                        target_kind_id,
                        damage,
                    });
                }
            }
        }
        Ok(())
    }

    fn plan_actor_death(&mut self, index: usize) -> Result<ActorDeathPlan, CoreError> {
        let actor = self.entities[index].clone();
        let actor_definition = self
            .content
            .actor(&actor.kind_id)
            .expect("living actor definition must remain available")
            .clone();
        let (generated_loot, generated_gold) = self.generate_death_loot(&actor)?;
        let corpse_kind_id = if let Some(kind_id) = actor_definition.corpse_item_kind_id {
            Some(kind_id)
        } else if let Some(remains) = actor_definition.remains {
            if self.rng.bounded(u64::from(remains.chance_denominator)) != 0 {
                None
            } else {
                match (remains.corpse_item_kind_id, remains.skeleton_item_kind_id) {
                    (Some(kind_id), None) | (None, Some(kind_id)) => Some(kind_id),
                    (Some(corpse_kind_id), Some(skeleton_kind_id)) => {
                        let total =
                            u64::from(remains.corpse_weight) + u64::from(remains.skeleton_weight);
                        if self.rng.bounded(total) < u64::from(remains.corpse_weight) {
                            Some(corpse_kind_id)
                        } else {
                            Some(skeleton_kind_id)
                        }
                    }
                    (None, None) => unreachable!("validated remains must define an item kind"),
                }
            }
        } else {
            None
        };
        let corpse = if let Some(kind_id) = corpse_kind_id {
            let (activation, charges) =
                initial_item_runtime_state(&self.content, &mut self.rng, &kind_id, 1);
            Some(ItemInstance {
                id: self.allocate_item_instance_id()?,
                activation,
                charges,
                fuel: initial_item_fuel(&self.content, &kind_id),
                device_recovery_progress: 0,
                captured_actor: None,
                curse: initial_item_curse(&self.content, &kind_id),
                permanent_destruction_immunities: Default::default(),
                kind_id,
                quantity: 1,
                inscription: None,
                origin_actor_kind_id: Some(actor.kind_id.clone()),
                origin_kind: None,
                damage_dice_override: None,
                discount_percent: 0,
                quality: ItemQualityDto::Ordinary,
                affix_ids: Vec::new(),
                rolled_affixes: Vec::new(),
                enchantments: ItemEnchantmentsDto::default(),
                location: ItemLocation::Ground(actor.position),
            })
        } else {
            None
        };
        let mut carried = self
            .items
            .iter()
            .filter_map(|item| match &item.location {
                ItemLocation::CarriedBy { actor_id } if actor_id == &actor.id => {
                    Some(CarriedDrop {
                        item_id: item.id.clone(),
                        kind_id: item.kind_id.clone(),
                        quantity: item.quantity,
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        carried.sort_by(|left, right| left.item_id.cmp(&right.item_id));
        let has_drops = !carried.is_empty()
            || !generated_loot.is_empty()
            || !generated_gold.is_empty()
            || corpse.is_some();
        let dissolved_pack_id = actor
            .pack
            .as_ref()
            .and_then(|pack| (pack.role == MonsterPackRoleDto::Leader).then(|| pack.id.clone()));

        Ok(ActorDeathPlan {
            actor,
            corpse,
            generated_loot,
            generated_gold,
            carried,
            has_drops,
            dissolved_pack_id,
        })
    }

    pub(super) fn resolve_actor_death(
        &mut self,
        index: usize,
        death_event: DomainEvent,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.resolve_actor_death_with_credit(
            index,
            death_event,
            true,
            events,
            changed,
            removed_entities,
        )
    }

    pub(super) fn resolve_actor_death_without_rewards(
        &mut self,
        index: usize,
        death_event: Option<DomainEvent>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let dying_actor = self.entities[index].clone();
        if self.riding_actor_id.as_deref() == Some(dying_actor.id.as_str()) {
            self.riding_actor_id = None;
        }
        self.clear_riding_bond_for(&dying_actor.id);
        let carried_item_ids = self
            .items
            .iter()
            .filter_map(|item| match &item.location {
                ItemLocation::CarriedBy { actor_id } if actor_id == &dying_actor.id => {
                    Some(item.id.clone())
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        self.entities[index].hp = self.entities[index].hp.min(0);
        if let Some(death_event) = death_event {
            events.push(death_event);
        }
        self.apply_amberite_blood_curse(&dying_actor);
        self.actor_death_explosion(&dying_actor, events, changed, removed_entities)?;
        self.summon_variant_maintainer_software_bugs(&dying_actor, changed);
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == dying_actor.id)
            .ok_or_else(|| {
                CoreError::Invariant(format!(
                    "dying actor {} disappeared during death explosion",
                    dying_actor.id
                ))
            })?;
        self.entities.remove(index);
        self.record_banor_rupart_group_defeat(&dying_actor.kind_id);
        removed_entities.push(dying_actor.id);
        self.items
            .retain(|item| !carried_item_ids.contains(item.id.as_str()));
        changed.insert(dying_actor.position);
        Ok(())
    }

    pub(super) fn resolve_actor_death_without_credit(
        &mut self,
        index: usize,
        death_event: DomainEvent,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.resolve_actor_death_with_credit(
            index,
            death_event,
            false,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_actor_death_with_credit(
        &mut self,
        index: usize,
        death_event: DomainEvent,
        credit_player: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let dying_actor = self.entities[index].clone();
        if self.riding_actor_id.as_deref() == Some(dying_actor.id.as_str()) {
            self.riding_actor_id = None;
        }
        self.clear_riding_bond_for(&dying_actor.id);
        self.entities[index].hp = self.entities[index].hp.min(0);
        events.push(death_event.clone());
        self.apply_infernal_deal(&dying_actor);
        self.apply_amberite_blood_curse(&dying_actor);
        self.actor_death_explosion(&dying_actor, events, changed, removed_entities)?;
        self.summon_variant_maintainer_software_bugs(&dying_actor, changed);
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == dying_actor.id)
            .ok_or_else(|| {
                CoreError::Invariant(format!(
                    "dying actor {} disappeared during death explosion",
                    dying_actor.id
                ))
            })?;
        let plan = self.plan_actor_death(index)?;
        let ActorDeathPlan {
            actor,
            corpse,
            generated_loot,
            generated_gold,
            carried,
            has_drops,
            dissolved_pack_id,
        } = plan;

        let removed = self.entities.remove(index);
        debug_assert_eq!(removed.id, actor.id);
        if let Some(pack_id) = dissolved_pack_id {
            for entity in &mut self.entities {
                if entity.pack.as_ref().is_some_and(|pack| pack.id == pack_id) {
                    entity.pack = None;
                }
            }
        }
        removed_entities.push(removed.id.clone());
        let removed_definition = self
            .content
            .actor(&removed.kind_id)
            .expect("removed actor definition must remain available");
        let removed_experience_value = removed_definition.experience_value;
        if removed_definition
            .finite_lifetime_instance_limit()
            .is_some()
            && !removed_definition.tags.iter().any(|tag| tag == "guardian")
            && !self.actor_is_dead_unique_resurrection(&removed)
        {
            let defeated = self
                .defeated_limited_actor_counts
                .entry(removed.kind_id.clone())
                .or_default();
            *defeated = defeated.saturating_add(1);
        }
        self.record_banor_rupart_group_defeat(&removed.kind_id);
        let experience_value = self.player_kill_experience_reward(removed_experience_value);
        if credit_player {
            self.apply_player_experience(experience_value, events);
            self.reward_player_kill_riding_bond(&removed, events);
        }
        self.command_actor_deaths.push(ActorDeathRecord {
            actor_id: removed.id.clone(),
            actor_kind_id: removed.kind_id.clone(),
            position: removed.position,
            credit_player,
        });
        let defeated_guardian = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .and_then(|floor| {
                floor.guardian.as_ref().and_then(|guardian| {
                    (guardian.instance_id == removed.id).then(|| {
                        (
                            floor
                                .dungeon_id
                                .clone()
                                .expect("guardian floor must have a dungeon ID"),
                            floor.id.clone(),
                            guardian.actor_kind_id.clone(),
                        )
                    })
                })
            });
        if let Some((dungeon_id, floor_id, target_kind_id)) = defeated_guardian {
            let state = self
                .dungeon_states
                .get_mut(&dungeon_id)
                .expect("guardian dungeon state must remain available");
            let first_defeat = !state.guardian_defeated;
            if first_defeat {
                state.guardian_defeated = true;
                events.push(DomainEvent::DungeonGuardianDefeated {
                    dungeon_id: dungeon_id.clone(),
                    floor_id,
                    target_kind_id,
                });
                let mirror_ids = self
                    .content
                    .world(&self.world_id)
                    .expect("active world must remain available")
                    .procedural_floors
                    .iter()
                    .filter(|floor| {
                        floor.dungeon_id.as_deref() == Some(dungeon_id.as_str())
                            && floor.final_floor
                    })
                    .filter_map(|floor| {
                        floor
                            .guardian
                            .as_ref()
                            .map(|guardian| guardian.instance_id.as_str())
                    })
                    .collect::<BTreeSet<_>>();
                for floor in self.stored_floors.values_mut() {
                    floor
                        .entities
                        .retain(|entity| !mirror_ids.contains(entity.id.as_str()));
                    floor.items.retain(|item| {
                        !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if mirror_ids.contains(actor_id.as_str()))
                    });
                }
            }
        }
        let defeated_entrance_guardian = self.content.world(&self.world_id).and_then(|world| {
            world.dungeons.iter().find_map(|dungeon| {
                dungeon.entrance_guardian.as_ref().and_then(|guardian| {
                    (self.current_floor_id == world.initial_floor_id
                        && guardian.instance_id == removed.id)
                        .then(|| (dungeon.id.clone(), guardian.actor_kind_id.clone()))
                })
            })
        });
        if let Some((dungeon_id, target_kind_id)) = defeated_entrance_guardian {
            let state = self
                .dungeon_states
                .get_mut(&dungeon_id)
                .expect("entrance guardian dungeon state must remain available");
            if !state.entrance_guardian_defeated {
                state.entrance_guardian_defeated = true;
                events.push(DomainEvent::DungeonEntranceGuardianDefeated {
                    dungeon_id,
                    target_kind_id,
                });
            }
        }

        for CarriedDrop {
            item_id,
            kind_id,
            quantity,
        } in carried
        {
            let item = self
                .items
                .iter_mut()
                .find(|item| item.id == item_id)
                .expect("carried item collected from authoritative item set");
            item.location = ItemLocation::Ground(removed.position);
            events.push(DomainEvent::LootDropped {
                source_kind_id: removed.kind_id.clone(),
                target_kind_id: kind_id,
                quantity,
            });
        }
        for item in generated_loot {
            events.push(DomainEvent::LootDropped {
                source_kind_id: removed.kind_id.clone(),
                target_kind_id: item.kind_id.clone(),
                quantity: item.quantity,
            });
            self.items.push(item);
        }
        for gold in generated_gold {
            events.push(DomainEvent::GoldDropped {
                source_kind_id: removed.kind_id.clone(),
                amount: gold.amount,
            });
            self.gold_piles.push(gold);
        }
        if let Some(corpse) = corpse {
            self.items.push(corpse);
        }
        if has_drops {
            changed.insert(removed.position);
        }
        Ok(())
    }
}

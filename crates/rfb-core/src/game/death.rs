// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::MeleeBlowEffectDefinition;
use rfb_protocol::{ItemEnchantmentsDto, ItemQualityDto, MonsterPackRoleDto, Position};

use crate::{
    effect::{DamagePacket, resolve_damage},
    error::CoreError,
    event::DomainEvent,
    resistance::DamageType,
    state::{Actor, GoldPile, ItemInstance, ItemLocation},
};

use super::{
    ActorDeathRecord, FatalityPolicy, Game, commit_damage_application, initial_item_curse,
    initial_item_runtime_state, plan_damage_application, rfb_area_damage,
};
use crate::save::initial_item_fuel;

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

impl Game {
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
            let (damage_dice, damage_sides, damage_type) = match effect {
                MeleeBlowEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_type,
                    ..
                } => (*damage_dice, *damage_sides, DamageType::from(*damage_type)),
                MeleeBlowEffectDefinition::Poison {
                    damage_dice,
                    damage_sides,
                    ..
                } => (*damage_dice, *damage_sides, DamageType::Poison),
                MeleeBlowEffectDefinition::Disease { .. }
                | MeleeBlowEffectDefinition::DrainAttributes { .. }
                | MeleeBlowEffectDefinition::DrainResource { .. }
                | MeleeBlowEffectDefinition::Bleeding { .. }
                | MeleeBlowEffectDefinition::Blind { .. }
                | MeleeBlowEffectDefinition::Confusion { .. }
                | MeleeBlowEffectDefinition::Paralysis { .. }
                | MeleeBlowEffectDefinition::Slow { .. }
                | MeleeBlowEffectDefinition::Stun { .. }
                | MeleeBlowEffectDefinition::Terrify { .. }
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
                    let damage = self.reduce_player_damage(resolve_damage(
                        DamagePacket::new(prepared_damage, damage_type),
                        self.effective_player_resistances().level(damage_type),
                    ));
                    let application =
                        plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
                    commit_damage_application(&mut self.player, &application);
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
                let damage = resolve_damage(
                    DamagePacket::new(prepared_damage, damage_type),
                    self.entities[target_index].resistances.level(damage_type),
                );
                let application = plan_damage_application(
                    &self.entities[target_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[target_index], &application);
                self.wake_entity_after_damage(target_index, damage.applied, events);
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
                curse: initial_item_curse(&self.content, &kind_id),
                kind_id,
                quantity: 1,
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
        self.actor_death_explosion(&dying_actor, events, changed, removed_entities)?;
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
        removed_entities.push(dying_actor.id);
        self.items
            .retain(|item| !carried_item_ids.contains(item.id.as_str()));
        changed.insert(dying_actor.position);
        Ok(())
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
        self.entities[index].hp = self.entities[index].hp.min(0);
        events.push(death_event.clone());
        self.actor_death_explosion(&dying_actor, events, changed, removed_entities)?;
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
        if removed_definition.tags.iter().any(|tag| tag == "unique")
            && !removed_definition.tags.iter().any(|tag| tag == "guardian")
        {
            self.defeated_unique_actor_kind_ids
                .insert(removed.kind_id.clone());
        }
        let experience_value = removed_definition.experience_value;
        if credit_player {
            self.apply_player_experience(experience_value, events);
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

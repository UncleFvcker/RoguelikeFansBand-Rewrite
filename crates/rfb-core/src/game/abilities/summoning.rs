// SPDX-License-Identifier: MPL-2.0

use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::ability_scaling::spell_power_value;
use crate::game::{
    CategorySummonSpec, Game, actor_answers_summons, chebyshev_distance,
    spawn_actor_from_definition,
};
use crate::scheduler::INITIAL_MONSTER_ENERGY_NEED;
use crate::state::{ItemLocation, SummonIdentity};
use rfb_content::{AbilityDefinition, AbilityEffectDefinition, ActorRole};
use rfb_protocol::{
    AbilityEffectResolutionDto, AbilityEffectsResolutionDto, AbilitySummonResolutionDto, Position,
    VirtueKindDto,
};
use std::collections::BTreeSet;

const DEATH_RAISE_DEAD_ABILITY_ID: &str = "demo.ability.death-raise-dead";

impl Game {
    pub(in crate::game) fn resolve_player_summon_effect(
        &mut self,
        ability: &AbilityDefinition,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Summon {
            actor_kind_id,
            count,
            duration_turns,
            hostile,
            ..
        } = &ability.effect
        else {
            unreachable!("summon executor requires a fixed summon effect");
        };
        debug_assert!(positions.len() <= usize::from(*count));
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated summon actor must remain available")
            .clone();
        let mut entity_ids = Vec::with_capacity(positions.len());
        for (ordinal, position) in positions.iter().copied().enumerate() {
            let id = self.summon_entity_id(&ability.id, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            self.maybe_initialize_chameleon_form(&mut entity);
            if !hostile {
                entity.summon = Some(SummonIdentity {
                    owner_id: self.player.id.clone(),
                    source_ability_id: ability.id.clone(),
                    remaining_turns: *duration_turns,
                });
            }
            changed.insert(position);
            entity_ids.push(id);
            self.entities.push(entity);
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution: AbilitySummonResolutionDto {
                owner_id: self.player.id.clone(),
                actor_kind_id: actor_kind_id.clone(),
                entity_ids,
                positions,
                duration_turns: *duration_turns,
                hostile: *hostile,
                group: false,
                summoned_kind_ids: Vec::new(),
            },
        });
    }

    pub(in crate::game) fn resolve_player_category_summon_effect(
        &mut self,
        ability: &AbilityDefinition,
        friendly_candidate_kind_ids: Vec<String>,
        hostile_candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::SummonCategory {
            category,
            upgraded_category,
            upgrade_at_level,
            count_dice,
            count_sides,
            count_bonus,
            maximum_count,
            hostile_chance_percent,
            friendly_group_chance_percent,
            hostile_group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            duration_turns,
            ..
        } = &ability.effect
        else {
            unreachable!("category summon executor requires a category summon effect");
        };
        let hostile = match *hostile_chance_percent {
            0 => false,
            100 => true,
            chance => self.rng.bounded(100) < u64::from(chance),
        };
        let group_chance = if hostile {
            *hostile_group_chance_percent
        } else {
            *friendly_group_chance_percent
        };
        let candidates = if hostile {
            hostile_candidate_kind_ids
        } else {
            friendly_candidate_kind_ids
        };
        let selected_category = upgraded_category
            .as_deref()
            .zip(*upgrade_at_level)
            .filter(|(_, level)| self.progress.level >= *level)
            .map_or(category.as_str(), |(category, _)| category);
        let owner_id = self.player.id.clone();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: &ability.id,
                owner_id: &owner_id,
                category: selected_category,
                count_dice: *count_dice,
                count_sides: *count_sides,
                count_bonus: *count_bonus,
                maximum_count: *maximum_count,
                hostile,
                group_chance_percent: group_chance,
                group_count_dice: *group_count_dice,
                group_count_sides: *group_count_sides,
                group_count_bonus: *group_count_bonus,
                duration_turns: *duration_turns,
            },
            candidates,
            positions,
            changed,
        );
        if ability.id == DEATH_RAISE_DEAD_ABILITY_ID && !resolution.entity_ids.is_empty() {
            self.add_virtue(VirtueKindDto::Unlife, 1);
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution,
        });
    }

    pub(in crate::game) fn resolve_player_nature_gate_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::NatureGate {
            animal_category,
            hound_category,
            hydra_category,
            ent_actor_kind_id,
            radius,
            duration_turns,
        } = &ability.effect
        else {
            unreachable!("nature gate executor requires a nature gate effect");
        };
        let level = self.progress.level;
        let selection = if level < 30 {
            Some((animal_category.as_str(), 1, None))
        } else if level < 47 {
            match self.rng.bounded(3) {
                0 => Some((hound_category.as_str(), 1, None)),
                1 => Some((hydra_category.as_str(), 1, None)),
                _ => Some((
                    animal_category.as_str(),
                    1 + level.saturating_sub(15) / 10,
                    None,
                )),
            }
        } else if self.rng.bounded(5) == 0 {
            Some(("ent", 1, Some(ent_actor_kind_id.as_str())))
        } else {
            None
        };
        let owner_id = self.player.id.clone();
        let Some((category, count, fixed_actor_kind_id)) = selection else {
            events.push(DomainEvent::AbilitySummoned {
                ability_id: ability.id.clone(),
                resolution: AbilitySummonResolutionDto {
                    owner_id,
                    actor_kind_id: "nature-gate".to_owned(),
                    entity_ids: Vec::new(),
                    positions: Vec::new(),
                    duration_turns: *duration_turns,
                    hostile: false,
                    group: false,
                    summoned_kind_ids: Vec::new(),
                },
            });
            return;
        };
        let maximum_level_base = spell_power_value(u64::from(level), ability.spell_power_bonus);
        let maximum_level_sides = spell_power_value(
            u64::from(level.saturating_mul(2) / 3),
            ability.spell_power_bonus,
        )
        .max(1);
        let maximum_level = maximum_level_base
            .saturating_add(self.rng.bounded(maximum_level_sides) + 1)
            .min(u64::from(u16::MAX)) as u16;
        let candidates = fixed_actor_kind_id.map_or_else(
            || self.summon_category_candidate_kind_ids(category, None, maximum_level, false),
            |kind_id| {
                self.content
                    .actor(kind_id)
                    .filter(|definition| {
                        definition.role == ActorRole::Monster
                            && definition.level <= u32::from(maximum_level)
                            && actor_answers_summons(definition)
                    })
                    .map(|definition| vec![definition.id.clone()])
                    .unwrap_or_default()
            },
        );
        let positions = self
            .open_positions_around_for_actor_kinds(self.player.position, *radius, &candidates)
            .into_iter()
            .take(usize::from(count))
            .collect::<Vec<_>>();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: &ability.id,
                owner_id: &owner_id,
                category,
                count_dice: 0,
                count_sides: 0,
                count_bonus: u8::try_from(count).expect("nature gate count must fit u8"),
                maximum_count: Some(
                    u8::try_from(count).expect("nature gate maximum count must fit u8"),
                ),
                hostile: false,
                group_chance_percent: 0,
                group_count_dice: 0,
                group_count_sides: 0,
                group_count_bonus: 0,
                duration_turns: *duration_turns,
            },
            candidates,
            positions,
            changed,
        );
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution,
        });
    }

    pub(in crate::game) fn resolve_player_demon_summoning_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::DemonSummoning = &ability.effect else {
            unreachable!("demon summoning executor requires a demon summoning effect");
        };
        let hostile = self.rng.bounded(3) == 0;
        let level = self.progress.level;
        let maximum_level_roll = self.rng.bounded(u64::from(level / 2)).saturating_add(1);
        let maximum_level = spell_power_value(
            u64::from(level.saturating_mul(2) / 3).saturating_add(maximum_level_roll),
            ability.spell_power_bonus,
        )
        .min(u64::from(u16::MAX)) as u16;
        self.resolve_player_group_summoning_effect(
            ability,
            "demon",
            maximum_level,
            hostile,
            events,
            changed,
        );
    }

    pub(in crate::game) fn resolve_player_angel_summoning_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::AngelSummoning = &ability.effect else {
            unreachable!("angel summoning executor requires an angel summoning effect");
        };
        let hostile = self.rng.bounded(3) == 0;
        let maximum_level = self.progress.level.saturating_mul(3) / 2;
        self.resolve_player_group_summoning_effect(
            ability,
            "angel",
            maximum_level,
            hostile,
            events,
            changed,
        );
    }

    fn resolve_player_group_summoning_effect(
        &mut self,
        ability: &AbilityDefinition,
        category: &str,
        maximum_level: u16,
        hostile: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let level = self.progress.level;
        let candidates =
            self.summon_category_candidate_kind_ids(category, None, maximum_level, false);
        let owner_id = self.player.id.clone();
        let Some(kind_id) = (!candidates.is_empty()).then(|| {
            let choice = usize::try_from(self.rng.bounded(candidates.len() as u64))
                .expect("bounded group summon candidate index must fit usize");
            candidates[choice].clone()
        }) else {
            events.push(DomainEvent::AbilitySummoned {
                ability_id: ability.id.clone(),
                resolution: AbilitySummonResolutionDto {
                    owner_id,
                    actor_kind_id: category.to_owned(),
                    entity_ids: Vec::new(),
                    positions: Vec::new(),
                    duration_turns: 0,
                    hostile,
                    group: false,
                    summoned_kind_ids: Vec::new(),
                },
            });
            return;
        };
        let definition = self
            .content
            .actor(&kind_id)
            .expect("selected group summon candidate must remain available")
            .clone();
        let group = (hostile || level >= 50)
            && definition
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.friends.is_some());
        let count = if group {
            let depth = self.floor_depth(&self.current_floor_id);
            self.original_friend_total(&definition, depth)
        } else {
            1
        };
        let positions = self
            .open_positions_around_for_actor_kind(self.player.position, 2, &kind_id)
            .into_iter()
            .take(usize::from(count))
            .collect::<Vec<_>>();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: &ability.id,
                owner_id: &owner_id,
                category,
                count_dice: 0,
                count_sides: 0,
                count_bonus: 1,
                maximum_count: None,
                hostile,
                group_chance_percent: u8::from(group).saturating_mul(100),
                group_count_dice: 0,
                group_count_sides: 0,
                group_count_bonus: u8::try_from(count).unwrap_or(u8::MAX),
                duration_turns: 0,
            },
            vec![kind_id],
            positions,
            changed,
        );
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution,
        });
    }

    pub(in crate::game) fn resolve_player_greater_demon_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        candidates: Vec<String>,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::SummonGreaterDemon { .. } = ability.effect else {
            unreachable!("greater-demon executor requires a greater-demon effect");
        };
        let owner_id = self.player.id.clone();
        let selected = (!candidates.is_empty()).then(|| {
            let choice = usize::try_from(self.rng.bounded(candidates.len() as u64))
                .expect("bounded greater-demon candidate index must fit usize");
            candidates[choice].clone()
        });
        let resolution = if let Some(kind_id) = selected {
            let definition = self
                .content
                .actor(&kind_id)
                .expect("selected greater demon must remain available")
                .clone();
            let group = definition
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.friends.is_some());
            let count = if group {
                let depth = self.floor_depth(&self.current_floor_id);
                self.original_friend_total(&definition, depth)
            } else {
                1
            };
            self.resolve_category_summon(
                CategorySummonSpec {
                    source_id: &ability.id,
                    owner_id: &owner_id,
                    category: "greater-demon",
                    count_dice: 0,
                    count_sides: 0,
                    count_bonus: 1,
                    maximum_count: None,
                    hostile: false,
                    group_chance_percent: u8::from(group).saturating_mul(100),
                    group_count_dice: 0,
                    group_count_sides: 0,
                    group_count_bonus: u8::try_from(count).unwrap_or(u8::MAX),
                    duration_turns: 0,
                },
                vec![kind_id],
                positions,
                changed,
            )
        } else {
            AbilitySummonResolutionDto {
                owner_id,
                actor_kind_id: "greater-demon".to_owned(),
                entity_ids: Vec::new(),
                positions: Vec::new(),
                duration_turns: 0,
                hostile: false,
                group: false,
                summoned_kind_ids: Vec::new(),
            }
        };
        let consumed = !resolution.entity_ids.is_empty();
        if consumed && let Some(index) = self.items.iter().position(|item| item.id == item_id) {
            if self.items[index].quantity > 1 {
                self.items[index].quantity -= 1;
            } else {
                self.items.remove(index);
                self.item_property_knowledge.remove(item_id);
            }
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution,
        });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::SacrificeCorpse {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    consumed,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn animate_dead_candidates(
        &self,
        origin: Position,
        actor_kind_id: &str,
        corpse_item_kind_id: &str,
        radius: u8,
        count: u8,
    ) -> Vec<(String, Position)> {
        let mut corpses = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position)
                    if item.kind_id == corpse_item_kind_id
                        && chebyshev_distance(origin, position) <= u32::from(radius)
                        && self.actor_kind_can_enter_position(actor_kind_id, position) =>
                {
                    Some((
                        chebyshev_distance(origin, position),
                        position.y,
                        position.x,
                        item.id.clone(),
                        position,
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        corpses.sort();
        corpses.truncate(usize::from(count));
        corpses
            .into_iter()
            .map(|(_, _, _, item_id, position)| (item_id, position))
            .collect()
    }

    pub(in crate::game) fn resolve_player_animate_dead_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            radius,
            count,
            failure_chance_percent,
        } = &ability.effect
        else {
            unreachable!("animate dead executor requires an animate dead effect");
        };
        let origin = self.player.position;
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated animated actor must remain available")
            .clone();
        let corpses = self.animate_dead_candidates(
            origin,
            actor_kind_id,
            corpse_item_kind_id,
            *radius,
            *count,
        );
        let consumed_corpse_item_ids = corpses
            .iter()
            .map(|corpse| corpse.0.clone())
            .collect::<Vec<_>>();
        self.items
            .retain(|item| !consumed_corpse_item_ids.contains(&item.id));
        for item_id in &consumed_corpse_item_ids {
            self.item_property_knowledge.remove(item_id);
        }
        let mut entity_ids = Vec::with_capacity(corpses.len());
        let mut positions = Vec::with_capacity(corpses.len());
        for (ordinal, (_, position)) in corpses.into_iter().enumerate() {
            changed.insert(position);
            if *failure_chance_percent > 0
                && self.rng.bounded(100) < u64::from(*failure_chance_percent)
            {
                continue;
            }
            let id = self.summon_entity_id(&ability.id, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.controller_id = Some(self.player.id.clone());
            self.entities.push(entity);
            entity_ids.push(id);
            positions.push(position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::AnimateDead {
                    effect_index: 0,
                    actor_kind_id: actor_kind_id.clone(),
                    consumed_corpse_item_ids,
                    entity_ids,
                    positions,
                }],
            },
            trace: None,
        });
        Ok(())
    }
}

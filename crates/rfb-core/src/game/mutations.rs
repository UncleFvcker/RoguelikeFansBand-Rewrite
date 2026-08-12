// SPDX-License-Identifier: MPL-2.0

use rfb_content::{MutationDefinition, MutationRatingDefinition};

use super::damage::{FatalityPolicy, commit_damage_application, plan_damage_application};
use super::hunger::NUTRITION_WEAK;
use super::*;

const GOOD_LUCK_MUTATION_ID: &str = "rfb.mutation.good-luck";
const BAD_LUCK_MUTATION_ID: &str = "rfb.mutation.bad-luck";
const EASY_TIRING_MUTATION_ID: &str = "rfb.mutation.easy-tiring";
const IMPOTENCE_MUTATION_ID: &str = "rfb.mutation.impotence";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LuckBias {
    Bad,
    Neutral,
    Good,
}

impl LuckBias {
    pub(super) const fn attribute_increase_threshold(self, value: u16) -> u64 {
        if value == 17 {
            58
        } else {
            match self {
                Self::Bad => 80,
                Self::Neutral => 75,
                Self::Good => 70,
            }
        }
    }
}

fn scale_by_ratio(value: u64, ratio: rfb_content::MutationRatioDefinition) -> u64 {
    value
        .saturating_mul(u64::from(ratio.numerator))
        .saturating_div(u64::from(ratio.denominator))
}

fn impotence_extra_effect(effect: &ItemUseEffectDefinition) -> bool {
    match effect {
        ItemUseEffectDefinition::ApplySpeed { .. } => true,
        ItemUseEffectDefinition::Sequence { effects } => effects.iter().any(impotence_extra_effect),
        _ => false,
    }
}

#[derive(Clone, Copy)]
enum RandomMutationOperation {
    Gain,
    Lose,
}

impl Game {
    pub(super) fn process_periodic_mutations(
        &mut self,
        local_floor_active: bool,
        resting: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if !local_floor_active {
            return Ok(());
        }
        self.process_unwell_sneeze(events, changed, removed_entities)?;
        if self.player_is_dead() {
            return Ok(());
        }
        self.process_periodic_mutations_after(None, resting, events, changed, removed_entities)
    }

    fn process_periodic_mutations_after(
        &mut self,
        source_index: Option<u16>,
        resting: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut mutations = self
            .content
            .mutations()
            .filter(|definition| {
                definition.periodic_effect.is_some()
                    && source_index.is_none_or(|index| definition.source_index > index)
            })
            .cloned()
            .collect::<Vec<_>>();
        mutations.sort_by_key(|definition| definition.source_index);

        for mutation in mutations {
            if !self.progress.active_mutation_ids.contains(&mutation.id) {
                continue;
            }
            let effect = mutation
                .periodic_effect
                .as_ref()
                .expect("filtered periodic mutation must retain its effect");
            self.resolve_periodic_mutation_effect(
                &mutation,
                effect,
                events,
                changed,
                removed_entities,
            )?;
            if self.pending_mutation_direction.is_some() {
                self.pending_mutation_direction = Some(PendingMutationDirectionDto {
                    mutation_id: mutation.id.clone(),
                    resting,
                });
            }
            if self.player_is_dead() || self.pending_mutation_direction.is_some() {
                break;
            }
        }
        Ok(())
    }

    fn resolve_periodic_mutation_effect(
        &mut self,
        mutation: &MutationDefinition,
        effect: &MutationPeriodicEffectDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        match effect {
            MutationPeriodicEffectDefinition::ApplyStatus {
                trigger_one_in,
                status_kind_id,
                intensity,
                duration_ticks,
                duration_dice,
                duration_sides,
                stacking,
            } => {
                if self.rng.bounded(u64::from(*trigger_one_in)) != 0 {
                    return Ok(());
                }
                let duration = (0..*duration_dice).fold(*duration_ticks, |total, _| {
                    total.saturating_add(
                        u32::try_from(self.rng.bounded(u64::from(*duration_sides)) + 1)
                            .expect("periodic status duration must fit u32"),
                    )
                });
                let stacking = match stacking {
                    AbilityStatusStackingDefinition::Replace => StatusStacking::Replace,
                    AbilityStatusStackingDefinition::Extend => StatusStacking::Extend,
                    AbilityStatusStackingDefinition::KeepStrongest => StatusStacking::KeepStrongest,
                };
                apply_status(
                    &mut self.player.statuses,
                    StatusApplication {
                        status: StatusInstance {
                            kind_id: status_kind_id.clone(),
                            intensity: *intensity,
                            remaining_ticks: duration,
                            source_id: Some(mutation.id.clone()),
                            granted_resistances: BTreeMap::new(),
                            granted_brands: BTreeSet::new(),
                            granted_modifiers: StatModifiersDto::default(),
                            granted_equipment_bonuses: EquipmentBonusesDto::default(),
                            granted_status_immunities: BTreeSet::new(),
                            granted_race_id: None,
                            grants_wall_passage: false,
                            incoming_damage_percent: 100,
                        },
                        stacking,
                    },
                );
                self.record_periodic_mutation(mutation, events);
            }
            MutationPeriodicEffectDefinition::BerserkRage => {
                if !self.player_has_status_kind(STATUS_BERSERK) && self.rng.bounded(3_000) == 0 {
                    self.apply_periodic_berserk();
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Cowardice => {
                if !self.periodic_resistance_save(DamageType::Fear, STATUS_FEAR)
                    && self.rng.bounded(3_000) == 12
                {
                    self.apply_simple_periodic_status(
                        &mutation.id,
                        STATUS_FEAR,
                        50,
                        StatusStacking::Extend,
                        100,
                    );
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Alcohol => {
                if self.rng.bounded(6_400) == 320
                    && self.resolve_periodic_alcohol(&mutation.id, events, changed)
                {
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Hallucination => {
                if !self.periodic_resistance_save(DamageType::Chaos, STATUS_HALLUCINATION)
                    && self.rng.bounded(6_400) == 41
                    && !self.periodic_resistance_save(DamageType::Chaos, STATUS_HALLUCINATION)
                {
                    let duration = u32::try_from(self.rng.bounded(50) + 20).unwrap_or(u32::MAX);
                    self.apply_simple_periodic_status(
                        &mutation.id,
                        STATUS_HALLUCINATION,
                        duration,
                        StatusStacking::Extend,
                        100,
                    );
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::ProduceMana => {
                if self.rng.bounded(9_000) == 0 {
                    self.pending_mutation_direction = Some(PendingMutationDirectionDto {
                        mutation_id: mutation.id.clone(),
                        resting: false,
                    });
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::SpeedFlux => {
                if self.rng.bounded(6_000) == 0 {
                    self.resolve_periodic_speed_flux(&mutation.id);
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Invulnerability => {
                if self.rng.bounded(5_000) == 0 {
                    let duration = u32::try_from(self.rng.bounded(8) + 9).unwrap_or(u32::MAX);
                    self.apply_simple_periodic_status(
                        &mutation.id,
                        STATUS_INVULNERABILITY,
                        duration,
                        StatusStacking::Extend,
                        0,
                    );
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::SpToHp => {
                if self.rng.bounded(2_000) == 0 && self.resolve_periodic_sp_to_hp() {
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::HpToSp => {
                if self.rng.bounded(4_000) == 0 && self.resolve_periodic_hp_to_sp() {
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Hypochondria => {
                if self.rng.bounded(1_815) == 0 {
                    if self.rng.bounded(2) == 0 {
                        self.apply_simple_periodic_status(
                            &mutation.id,
                            STATUS_FEAR,
                            50,
                            StatusStacking::Extend,
                            100,
                        );
                    } else if self
                        .player
                        .statuses
                        .iter()
                        .find(|status| status.kind_id == STATUS_UNWELL)
                        .is_none_or(|status| status.remaining_ticks <= 40)
                    {
                        self.apply_simple_periodic_status(
                            &mutation.id,
                            STATUS_UNWELL,
                            50,
                            StatusStacking::Replace,
                            100,
                        );
                    }
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::RandomTeleport => {
                if !self.periodic_resistance_save(DamageType::Nexus, "")
                    && self.rng.bounded(5_000) == 87
                {
                    let candidates = self.random_teleport_candidates(40);
                    if !candidates.is_empty() {
                        let index = usize::try_from(self.rng.bounded(candidates.len() as u64))
                            .expect("bounded teleport candidate index must fit usize");
                        events.extend(self.relocate_player(candidates[index], changed));
                    }
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::RandomBanish => {
                if self.rng.bounded(9_000) == 0 {
                    let actor_ids = self.item_visible_actor_ids();
                    let _ = self.banish_visible_actors(100, actor_ids, changed);
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::ShadowWalk => {
                if self.rng.bounded(12_000) == 0 {
                    self.reality_change_ticks = if self.reality_change_ticks == 0 {
                        u8::try_from(self.rng.bounded(21) + 15).unwrap_or(35)
                    } else {
                        0
                    };
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Fumbling => {
                if self.rng.bounded(10_000) == 0 {
                    let damage = resolve_damage(
                        DamagePacket::new(
                            i32::try_from(self.rng.bounded(25) + 1).unwrap_or(25),
                            DamageType::Physical,
                        ),
                        ResistanceLevel::Normal,
                    );
                    let application =
                        plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
                    commit_damage_application(&mut self.player, &application);
                    let dropped_item_kind_id = self.drop_random_equipped_melee_weapon(changed);
                    events.push(DomainEvent::MutationFumbled {
                        damage,
                        dropped_item_kind_id,
                    });
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Flatulence => {
                if self.rng.bounded(3_000) == 12 {
                    self.resolve_player_area_damage_with_base(
                        &mutation.id,
                        vec![self.player.position],
                        false,
                        DamageType::Poison,
                        3,
                        None,
                        i32::from(self.progress.level),
                        false,
                        events,
                        changed,
                        removed_entities,
                    )?;
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::AttractDemon => {
                if self.rng.bounded(6_666) == 665
                    && self.resolve_periodic_attraction(mutation, "demon", 6, events, changed)
                {
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::EatLight => {
                if self.rng.bounded(3_000) == 0 {
                    self.resolve_periodic_eat_light(mutation, events, changed, removed_entities)?;
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::AttractAnimal => {
                if self.rng.bounded(7_000) == 0
                    && self.resolve_periodic_attraction(mutation, "animal", 3, events, changed)
                {
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::RawChaos => {
                if self.rng.bounded(8_000) == 0 {
                    self.resolve_player_area_damage_with_base(
                        &mutation.id,
                        vec![self.player.position],
                        false,
                        DamageType::Chaos,
                        8,
                        None,
                        i32::from(self.progress.level),
                        false,
                        events,
                        changed,
                        removed_entities,
                    )?;
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::AttractDragon => {
                if self.rng.bounded(3_000) == 0
                    && self.resolve_periodic_attraction(mutation, "dragon", 5, events, changed)
                {
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Normality => {
                if self.rng.bounded(5_000) == 0 {
                    let previous_max_hp = self.effective_player_max_hp();
                    let previous_resource_maxima = self.player_resource_maxima();
                    if self.lose_random_mutation_without_refresh(events).is_some() {
                        self.refresh_after_attribute_change(
                            previous_max_hp,
                            &previous_resource_maxima,
                        );
                        self.record_periodic_mutation(mutation, events);
                    }
                }
            }
            MutationPeriodicEffectDefinition::Wraithform => {
                if self.rng.bounded(3_000) == 0 {
                    let half_level = u32::from(self.progress.level / 2);
                    let duration = if half_level == 0 {
                        1
                    } else {
                        u32::try_from(self.rng.bounded(u64::from(half_level)) + 1)
                            .unwrap_or(half_level)
                            .saturating_add(half_level)
                    };
                    apply_status(
                        &mut self.player.statuses,
                        StatusApplication {
                            status: StatusInstance {
                                kind_id: STATUS_WRAITHFORM.to_owned(),
                                intensity: 1,
                                remaining_ticks: duration,
                                source_id: Some(mutation.id.clone()),
                                granted_resistances: BTreeMap::new(),
                                granted_brands: BTreeSet::new(),
                                granted_modifiers: StatModifiersDto::default(),
                                granted_equipment_bonuses: EquipmentBonusesDto::default(),
                                granted_status_immunities: BTreeSet::new(),
                                granted_race_id: None,
                                grants_wall_passage: true,
                                incoming_damage_percent: 50,
                            },
                            stacking: StatusStacking::Replace,
                        },
                    );
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::PolymorphWounds => {
                if self.rng.bounded(3_000) == 0 {
                    let maximum_hp = self.effective_player_max_hp();
                    self.resolve_polymorph_wounds(&mutation.id, maximum_hp);
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Wasting => {
                if self.rng.bounded(3_000) == 0 {
                    const ATTRIBUTES: [AttributeKind; 6] = [
                        AttributeKind::Strength,
                        AttributeKind::Intelligence,
                        AttributeKind::Wisdom,
                        AttributeKind::Dexterity,
                        AttributeKind::Constitution,
                        AttributeKind::Charisma,
                    ];
                    let attribute = ATTRIBUTES[usize::try_from(self.rng.bounded(6))
                        .expect("wasting attribute index must fit usize")];
                    if !self
                        .player_equipment_passives()
                        .contains(&attribute_sustain_passive(attribute))
                    {
                        let amount = u8::try_from(self.rng.bounded(6) + 7)
                            .expect("wasting drain amount must fit u8");
                        let permanent = self.rng.bounded(6) == 0;
                        let previous_max_hp = self.effective_player_max_hp();
                        let previous_resource_maxima = self.player_resource_maxima();
                        let outcome = if permanent {
                            apply_permanent_attribute_drain(
                                &mut self.progress,
                                attribute,
                                amount,
                                &mut self.rng,
                            )
                        } else {
                            apply_attribute_drain_with_amount(
                                &mut self.progress,
                                attribute,
                                amount,
                                &mut self.rng,
                            )
                        };
                        if outcome.changed {
                            self.refresh_after_attribute_change(
                                previous_max_hp,
                                &previous_resource_maxima,
                            );
                        }
                        self.record_periodic_mutation(mutation, events);
                    }
                }
            }
            MutationPeriodicEffectDefinition::RandomTelepathy => {
                if self.rng.bounded(3_000) == 0 {
                    if let Some(index) = self
                        .player
                        .statuses
                        .iter()
                        .position(|status| status.kind_id == STATUS_TELEPATHY)
                    {
                        self.player.statuses.remove(index);
                    } else {
                        self.apply_simple_periodic_status(
                            &mutation.id,
                            STATUS_TELEPATHY,
                            u32::from(self.progress.level),
                            StatusStacking::Replace,
                            100,
                        );
                    }
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Nausea => {
                if self.rng.bounded(9_000) == 0 {
                    let before = self.nutrition_state();
                    self.nutrition = NUTRITION_WEAK;
                    let after = self.nutrition_state();
                    if before != after {
                        events.push(DomainEvent::NutritionStateChanged {
                            from: before,
                            to: after,
                            nutrition: self.nutrition,
                        });
                    }
                    self.record_periodic_mutation(mutation, events);
                }
            }
            MutationPeriodicEffectDefinition::Warning => {
                if self.rng.bounded(1_000) == 0 {
                    let player_level = u32::from(self.progress.level);
                    let danger_amount = self.entities.iter().fold(0_u32, |total, actor| {
                        if actor.hp <= 0 {
                            return total;
                        }
                        self.actor_runtime_definition(actor)
                            .map_or(total, |definition| {
                                if definition.level < player_level {
                                    total
                                } else {
                                    total.saturating_add(definition.level - player_level + 1)
                                }
                            })
                    });
                    events.push(DomainEvent::MutationWarning { danger_amount });
                    self.record_periodic_mutation(mutation, events);
                }
            }
        }
        Ok(())
    }

    pub(super) fn resolve_polymorph_wounds(&mut self, source_id: &str, maximum_hp: i32) {
        let healing = self.roll_damage(self.progress.level.max(1), 5);
        self.player.hp = self.player.hp.saturating_add(healing).min(maximum_hp);
        if self.rng.bounded(5) == 0 {
            self.player.hp = self.player.hp.saturating_sub(healing / 2);
            apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_BLEEDING.to_owned(),
                        intensity: 1,
                        remaining_ticks: u32::try_from(healing.max(1)).unwrap_or(u32::MAX),
                        source_id: Some(source_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Replace,
                },
            );
        } else if let Some(bleeding) = self
            .player
            .statuses
            .iter_mut()
            .find(|status| status.kind_id == STATUS_BLEEDING)
        {
            bleeding.remaining_ticks = bleeding
                .remaining_ticks
                .saturating_sub(u32::try_from(healing / 2).unwrap_or(u32::MAX));
        }
        self.player
            .statuses
            .retain(|status| status.remaining_ticks > 0);
    }

    fn resolve_periodic_attraction(
        &mut self,
        mutation: &MutationDefinition,
        category: &str,
        friendly_one_in: u64,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let friendly = self.rng.bounded(friendly_one_in) == 0;
        let depth = self.floor_depth(&self.current_floor_id).max(1);
        let candidates = self.summon_category_candidate_kind_ids(category, None, depth, !friendly);
        if candidates.is_empty() {
            return false;
        }
        let choice = usize::try_from(self.rng.bounded(candidates.len() as u64))
            .expect("bounded attraction candidate index must fit usize");
        let kind_id = candidates[choice].clone();
        let definition = self
            .content
            .actor(&kind_id)
            .expect("selected attraction candidate must remain available")
            .clone();
        let count = if definition
            .allocation
            .as_ref()
            .is_some_and(|allocation| allocation.friends.is_some())
        {
            self.original_friend_total(&definition, depth)
        } else {
            1
        };
        let positions = self
            .open_positions_around_for_actor_kind(self.player.position, 2, &kind_id)
            .into_iter()
            .take(usize::from(count))
            .collect::<Vec<_>>();
        if positions.is_empty() {
            return false;
        }
        let owner_id = self.player.id.clone();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: &mutation.id,
                owner_id: &owner_id,
                category,
                count_dice: 0,
                count_sides: 0,
                count_bonus: 1,
                maximum_count: None,
                hostile: !friendly,
                group_chance_percent: u8::from(count > 1).saturating_mul(100),
                group_count_dice: 0,
                group_count_sides: 0,
                group_count_bonus: u8::try_from(count).unwrap_or(u8::MAX),
                duration_turns: 0,
            },
            vec![kind_id],
            positions,
            changed,
        );
        let summoned = !resolution.entity_ids.is_empty();
        events.push(DomainEvent::AbilitySummoned {
            ability_id: mutation.id.clone(),
            resolution,
        });
        summoned
    }

    fn resolve_periodic_eat_light(
        &mut self,
        mutation: &MutationDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut healing: i32 = self
            .index(self.player.position)
            .filter(|index| self.glow[*index])
            .map_or(0, |_| 10);
        let light_index = self.items.iter().position(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "light")
                && item.fuel.is_some_and(|fuel| fuel.current > 0)
                && self
                    .content
                    .item(&item.kind_id)
                    .is_some_and(|definition| !definition.tags.iter().any(|tag| tag == "artifact"))
        });
        if let Some(index) = light_index {
            let item = &mut self.items[index];
            let fuel = item
                .fuel
                .as_mut()
                .expect("selected equipped light must retain fuel");
            healing = healing.saturating_add(i32::from(fuel.current / 20));
            fuel.current /= 2;
            if fuel.current == 0 {
                events.push(DomainEvent::LightExtinguished {
                    target_item_id: item.id.clone(),
                    target_kind_id: item.kind_id.clone(),
                });
            }
        }
        if healing > 0 {
            let maximum = self.effective_player_max_hp();
            apply_healing(
                &mut self.player.hp,
                maximum,
                HealingRequest::amount(healing),
            );
        }
        self.resolve_player_area_damage_with_base(
            &mutation.id,
            vec![self.player.position],
            false,
            DamageType::Dark,
            10,
            None,
            50,
            false,
            events,
            changed,
            removed_entities,
        )?;
        changed.extend(self.extinguish_area(self.player.position, 10));
        Ok(())
    }

    fn drop_random_equipped_melee_weapon(
        &mut self,
        changed: &mut BTreeSet<Position>,
    ) -> Option<String> {
        let candidates =
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    matches!(item.location, ItemLocation::Equipped { .. })
                        && item.curse.is_none()
                        && self.content.item(&item.kind_id).is_some_and(|definition| {
                            definition.tags.iter().any(|tag| tag == "weapon")
                        })
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let choice = usize::try_from(self.rng.bounded(candidates.len() as u64))
            .expect("bounded equipment candidate index must fit usize");
        let item = &mut self.items[candidates[choice]];
        let kind_id = item.kind_id.clone();
        item.location = ItemLocation::Ground(self.player.position);
        self.clamp_player_hp_to_effective_max();
        self.refresh_player_resource_maxima();
        changed.insert(self.player.position);
        Some(kind_id)
    }

    fn apply_simple_periodic_status(
        &mut self,
        source_id: &str,
        status_kind_id: &str,
        duration: u32,
        stacking: StatusStacking,
        incoming_damage_percent: u8,
    ) {
        apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: status_kind_id.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_id.to_owned()),
                    granted_resistances: BTreeMap::new(),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent,
                },
                stacking,
            },
        );
    }

    fn apply_periodic_berserk(&mut self) {
        let Some(mut ability) = self
            .content
            .ability("rfb.ability.mutation.berserk")
            .cloned()
        else {
            return;
        };
        Self::apply_player_level_scaling(&mut ability, self.progress.level);
        let AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            duration_dice,
            duration_sides,
            stacking,
            resistance_type,
            power,
            granted_resistances,
            granted_brands,
            granted_modifiers,
            granted_equipment_bonuses,
            granted_status_immunities,
            granted_race_id,
            grants_wall_passage,
            incoming_damage_percent,
        } = &ability.effect
        else {
            unreachable!("the validated berserk mutation ability must apply a status");
        };
        let _ = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            status_kind_id,
            *intensity,
            *duration_ticks,
            *duration_dice,
            *duration_sides,
            *stacking,
            *resistance_type,
            *power,
            granted_resistances,
            granted_brands,
            granted_modifiers,
            granted_equipment_bonuses,
            granted_status_immunities,
            granted_race_id.as_deref(),
            *grants_wall_passage,
            *incoming_damage_percent,
            None,
            None,
            &mut self.rng,
        );
    }

    fn periodic_resistance_save(&mut self, damage_type: DamageType, status_kind_id: &str) -> bool {
        let percent = if self.player_status_immunities().contains(status_kind_id) {
            100
        } else {
            self.effective_player_resistances()
                .level(damage_type)
                .reduction_percent()
                .max(0)
        };
        self.rng.bounded(33) < u64::try_from(percent).unwrap_or(0)
    }

    fn resolve_periodic_alcohol(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        if !self.periodic_resistance_save(DamageType::Confusion, STATUS_CONFUSION) {
            let _ = self.periodic_resistance_save(DamageType::Chaos, STATUS_HALLUCINATION);
        }
        let mut applied = false;
        if !self.periodic_resistance_save(DamageType::Confusion, STATUS_CONFUSION) {
            let duration = u32::try_from(self.rng.bounded(20) + 15).unwrap_or(u32::MAX);
            self.apply_simple_periodic_status(
                mutation_id,
                STATUS_CONFUSION,
                duration,
                StatusStacking::Extend,
                100,
            );
            applied = true;
        }
        if !self.periodic_resistance_save(DamageType::Chaos, STATUS_HALLUCINATION) {
            if self.rng.bounded(20) == 0 {
                let _lose_all_information = self.rng.bounded(3) == 0;
                self.clear_current_floor_memory(changed);
                let candidates = self.random_teleport_candidates(100);
                if !candidates.is_empty() {
                    let index = usize::try_from(self.rng.bounded(candidates.len() as u64))
                        .expect("bounded teleport candidate index must fit usize");
                    events.extend(self.relocate_player(candidates[index], changed));
                }
                self.clear_current_floor_memory(changed);
                applied = true;
            } else if self.rng.bounded(3) == 0 {
                let duration = u32::try_from(self.rng.bounded(15) + 15).unwrap_or(u32::MAX);
                self.apply_simple_periodic_status(
                    mutation_id,
                    STATUS_HALLUCINATION,
                    duration,
                    StatusStacking::Extend,
                    100,
                );
                applied = true;
            }
        }
        applied
    }

    fn resolve_periodic_speed_flux(&mut self, mutation_id: &str) {
        if self.rng.bounded(2) == 0 {
            if self.player_has_status_kind(STATUS_HASTE) {
                self.player
                    .statuses
                    .retain(|status| status.kind_id != STATUS_HASTE);
            } else if !self.player_has_status_kind(STATUS_SLOW) && self.rng.bounded(2) == 0 {
                let duration = u32::try_from(self.rng.bounded(30) + 11).unwrap_or(u32::MAX);
                self.apply_simple_periodic_status(
                    mutation_id,
                    STATUS_SLOW,
                    duration,
                    StatusStacking::Replace,
                    100,
                );
            } else {
                self.minor_slow = self.minor_slow.saturating_add(10).min(10);
            }
        } else if self.player_has_status_kind(STATUS_SLOW) || self.minor_slow > 0 {
            self.player
                .statuses
                .retain(|status| status.kind_id != STATUS_SLOW);
            self.minor_slow = self.minor_slow.saturating_sub(10);
        } else {
            let duration = u32::try_from(self.rng.bounded(30) + 11).unwrap_or(u32::MAX);
            self.apply_simple_periodic_status(
                mutation_id,
                STATUS_HASTE,
                duration,
                StatusStacking::Replace,
                100,
            );
        }
    }

    fn resolve_periodic_sp_to_hp(&mut self) -> bool {
        let Some(resource_id) = self
            .casting_profile()
            .map(|profile| profile.resource_id.clone())
        else {
            return false;
        };
        let maximum_hp = self.effective_player_max_hp();
        let wounds = maximum_hp.saturating_sub(self.player.hp);
        let Some(pool) = self.resources.get_mut(&resource_id) else {
            return false;
        };
        let amount = pool.current.min(u32::try_from(wounds.max(0)).unwrap_or(0));
        if amount == 0 {
            return false;
        }
        pool.current -= amount;
        self.player.hp = self
            .player
            .hp
            .saturating_add(i32::try_from(amount).unwrap_or(i32::MAX))
            .min(maximum_hp);
        true
    }

    fn resolve_periodic_hp_to_sp(&mut self) -> bool {
        let Some(resource_id) = self
            .casting_profile()
            .map(|profile| profile.resource_id.clone())
        else {
            return false;
        };
        let Some(pool) = self.resources.get_mut(&resource_id) else {
            return false;
        };
        let amount = pool
            .maximum
            .saturating_sub(pool.current)
            .min(u32::try_from(self.player.hp.max(0)).unwrap_or(0));
        if amount == 0 {
            return false;
        }
        pool.current = pool.current.saturating_add(amount).min(pool.maximum);
        self.player.hp = self
            .player
            .hp
            .saturating_sub(i32::try_from(amount).unwrap_or(i32::MAX));
        true
    }

    fn process_unwell_sneeze(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let active = self
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_UNWELL && status.remaining_ticks <= 55);
        if !active || self.rng.bounded(100) != 0 {
            return Ok(());
        }
        let count = u8::try_from(self.rng.bounded(3) + 1).unwrap_or(3);
        let Some(target) = self.random_sneeze_target() else {
            return Ok(());
        };
        let range = self.width.max(self.height);
        let Some(path) = self.targeted_projectile_path_through_target(target, range) else {
            return Ok(());
        };
        let damage = i32::from(self.progress.level / 2);
        for _ in 0..count {
            self.resolve_player_beam_damage_with_base(
                STATUS_UNWELL,
                path.clone(),
                DamageType::Cold,
                damage,
                false,
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    fn random_sneeze_target(&mut self) -> Option<Position> {
        for _ in 0..1_000 {
            let target = Position {
                x: self.player.position.x + i32::try_from(self.rng.bounded(9)).unwrap_or(0) - 4,
                y: self.player.position.y + i32::try_from(self.rng.bounded(9)).unwrap_or(0) - 4,
            };
            if target == self.player.position || rfb_distance(self.player.position, target) > 4 {
                continue;
            }
            let Some(path) = self.untargeted_projectile_path(target, 4) else {
                continue;
            };
            if self
                .trace_projectile_path_with_actor_policy(path, false)
                .0
                .landing
                == target
            {
                return Some(target);
            }
        }
        None
    }

    pub(super) fn resolve_pending_mutation_direction(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<PendingMutationDirectionDto, CoreError> {
        let pending = self
            .pending_mutation_direction
            .take()
            .ok_or(CoreError::MutationDirectionUnavailable)?;
        let path = self
            .projectile_path(
                &TargetSelection::Direction { direction },
                self.width.max(self.height),
            )
            .expect("directional mutation must produce a projectile path");
        self.resolve_player_area_damage_with_base(
            &pending.mutation_id,
            path,
            true,
            DamageType::Mana,
            3,
            None,
            i32::from(self.progress.level).saturating_mul(2),
            false,
            events,
            changed,
            removed_entities,
        )?;
        Ok(pending)
    }

    pub(super) fn resume_periodic_mutations(
        &mut self,
        pending: &PendingMutationDirectionDto,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let source_index = self
            .content
            .mutation(&pending.mutation_id)
            .map(|mutation| mutation.source_index)
            .ok_or(CoreError::MutationDirectionUnavailable)?;
        self.process_periodic_mutations_after(
            Some(source_index),
            pending.resting,
            events,
            changed,
            removed_entities,
        )
    }

    pub(super) fn advance_reality_change(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        if self.reality_change_ticks == 0 {
            return Ok(false);
        }
        self.reality_change_ticks -= 1;
        if self.reality_change_ticks > 0 {
            return Ok(false);
        }
        let regenerated = self.regenerate_current_procedural_dungeon(changed, removed_entities)?;
        events.push(DomainEvent::RealityChangeResolved { regenerated });
        Ok(regenerated)
    }

    fn record_periodic_mutation(
        &self,
        mutation: &MutationDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        events.push(DomainEvent::MutationPeriodicTriggered {
            mutation_id: mutation.id.clone(),
            name: mutation.name.clone(),
        });
    }

    pub(super) fn mutation_activation_for_ability(
        &self,
        ability_id: &str,
    ) -> Option<&MutationActivationDefinition> {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .filter_map(|mutation| mutation.activation.as_ref())
            .find(|activation| activation.ability_id == ability_id)
    }

    pub(super) fn gain_mutation(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        if !self.gain_mutation_without_refresh(mutation_id, events) {
            return false;
        }
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        true
    }

    pub(super) fn gain_mutation_without_refresh(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let Some(definition) = self.content.mutation(mutation_id).cloned() else {
            return false;
        };
        if self.progress.active_mutation_ids.contains(mutation_id) {
            return false;
        }
        for removed_id in &definition.removes_on_gain {
            if self.progress.active_mutation_ids.contains(removed_id)
                && !self.progress.locked_mutation_ids.contains(removed_id)
            {
                self.progress.active_mutation_ids.remove(removed_id);
                let removed = self
                    .content
                    .mutation(removed_id)
                    .expect("validated mutation removal must remain available");
                events.push(DomainEvent::MutationLost {
                    mutation_id: removed.id.clone(),
                    name: removed.name.clone(),
                });
            }
        }
        self.progress
            .active_mutation_ids
            .insert(definition.id.clone());
        events.push(DomainEvent::MutationGained {
            mutation_id: definition.id,
            name: definition.name,
        });
        if definition.auto_identify_items {
            let count = self.identify_carried_items();
            if count > 0 {
                events.push(DomainEvent::ItemAutoIdentified { count });
            }
        }
        true
    }

    fn mutation_can_be_gained(&self, definition: &MutationDefinition) -> bool {
        if self.progress.active_mutation_ids.contains(&definition.id)
            || (definition.id == chaos_patron::CHAOS_GIFT_MUTATION_ID
                && self
                    .progress
                    .active_mutation_ids
                    .contains(chaos_patron::PURPLE_GIFT_MUTATION_ID))
        {
            return false;
        }
        !definition
            .removes_on_gain
            .iter()
            .any(|mutation_id| self.progress.locked_mutation_ids.contains(mutation_id))
    }

    pub(super) fn lose_mutation(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        if !self.lose_mutation_without_refresh(mutation_id, events) {
            return false;
        }
        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        true
    }

    pub(super) fn lose_mutation_without_refresh(
        &mut self,
        mutation_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let Some(definition) = self.content.mutation(mutation_id).cloned() else {
            return false;
        };
        if !self.progress.active_mutation_ids.contains(mutation_id)
            || self.progress.locked_mutation_ids.contains(mutation_id)
        {
            return false;
        }
        self.progress.active_mutation_ids.remove(mutation_id);
        events.push(DomainEvent::MutationLost {
            mutation_id: definition.id,
            name: definition.name,
        });
        true
    }

    pub(super) fn lose_all_unlocked_mutations(&mut self, events: &mut Vec<DomainEvent>) -> usize {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let removed = self.remove_all_unlocked_mutations_without_refresh();
        if removed.is_empty() {
            return 0;
        }

        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        for (mutation_id, name) in &removed {
            events.push(DomainEvent::MutationLost {
                mutation_id: mutation_id.clone(),
                name: name.clone(),
            });
        }
        removed.len()
    }

    pub(super) fn remove_all_unlocked_mutations_without_refresh(
        &mut self,
    ) -> Vec<(String, String)> {
        let mut removed = self
            .content
            .mutations()
            .filter(|definition| {
                self.progress.active_mutation_ids.contains(&definition.id)
                    && !self.progress.locked_mutation_ids.contains(&definition.id)
            })
            .map(|definition| {
                (
                    definition.source_index,
                    definition.id.clone(),
                    definition.name.clone(),
                )
            })
            .collect::<Vec<_>>();
        removed.sort_by_key(|(source_index, _, _)| *source_index);
        for (_, mutation_id, _) in &removed {
            self.progress.active_mutation_ids.remove(mutation_id);
        }
        removed
            .into_iter()
            .map(|(_, mutation_id, name)| (mutation_id, name))
            .collect()
    }

    pub(super) fn gain_random_mutation(&mut self, events: &mut Vec<DomainEvent>) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Gain)?;
        let gained = self.gain_mutation(&mutation_id, events);
        debug_assert!(gained, "selected mutation must remain gainable");
        gained.then_some(mutation_id)
    }

    pub(super) fn gain_random_mutation_without_refresh(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Gain)?;
        let gained = self.gain_mutation_without_refresh(&mutation_id, events);
        debug_assert!(gained, "selected mutation must remain gainable");
        gained.then_some(mutation_id)
    }

    pub(super) fn lose_random_mutation(&mut self, events: &mut Vec<DomainEvent>) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Lose)?;
        let lost = self.lose_mutation(&mutation_id, events);
        debug_assert!(lost, "selected mutation must remain removable");
        lost.then_some(mutation_id)
    }

    pub(super) fn lose_random_mutation_without_refresh(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> Option<String> {
        let mutation_id = self.select_random_mutation(RandomMutationOperation::Lose)?;
        let lost = self.lose_mutation_without_refresh(&mutation_id, events);
        debug_assert!(lost, "selected mutation must remain removable");
        lost.then_some(mutation_id)
    }

    pub(super) fn resolve_polymorph_mutations(&mut self, events: &mut Vec<DomainEvent>) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut count = self
            .progress
            .active_mutation_ids
            .iter()
            .filter(|id| !self.progress.locked_mutation_ids.contains(*id))
            .count();

        if count > 1 && self.rng.bounded(23) == 0 {
            let removed = self.remove_all_unlocked_mutations_without_refresh();
            if removed.is_empty() {
                return false;
            }
            events.push(DomainEvent::MutationAllCured);
            for (mutation_id, name) in removed {
                events.push(DomainEvent::MutationLost { mutation_id, name });
            }
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            return true;
        }

        let mut changed = false;
        loop {
            let can_gain = !self
                .random_mutation_candidates(RandomMutationOperation::Gain)
                .is_empty();
            let can_lose = !self
                .random_mutation_candidates(RandomMutationOperation::Lose)
                .is_empty();
            if !can_gain && !can_lose {
                break;
            }

            let changed_this_round = if self.rng.bounded(2) == 0 {
                let gained = self.gain_random_mutation_without_refresh(events).is_some();
                count = count.saturating_add(usize::from(gained));
                gained
            } else {
                let loss_allowed = if count > 5 {
                    true
                } else {
                    self.rng
                        .bounded(u64::try_from(6 - count).expect("loss bound must fit u64"))
                        == 0
                };
                let lost =
                    loss_allowed && self.lose_random_mutation_without_refresh(events).is_some();
                count = count.saturating_sub(usize::from(lost));
                lost
            };
            if changed_this_round {
                changed = true;
                if self.rng.bounded(2) != 0 {
                    break;
                }
            }
        }

        if changed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
        changed
    }

    fn select_random_mutation(&mut self, operation: RandomMutationOperation) -> Option<String> {
        let candidates = self.random_mutation_candidates(operation);
        let total = candidates.iter().map(|(_, _, weight)| *weight).sum::<u64>();
        if total == 0 {
            return None;
        }
        let roll = self.rng.bounded(total);
        let mut cumulative = 0_u64;
        candidates.into_iter().find_map(|(_, mutation_id, weight)| {
            cumulative = cumulative.saturating_add(weight);
            (roll < cumulative).then_some(mutation_id)
        })
    }

    fn random_mutation_candidates(
        &self,
        operation: RandomMutationOperation,
    ) -> Vec<(u16, String, u64)> {
        let mut candidates = self
            .content
            .mutations()
            .filter_map(|definition| {
                if !definition.random_selection_enabled {
                    return None;
                }
                let eligible = match operation {
                    RandomMutationOperation::Gain => self.mutation_can_be_gained(definition),
                    RandomMutationOperation::Lose => {
                        self.progress.active_mutation_ids.contains(&definition.id)
                            && !self.progress.locked_mutation_ids.contains(&definition.id)
                    }
                };
                let weight =
                    eligible.then(|| self.mutation_random_weight(definition, operation))?;
                (weight > 0).then(|| (definition.source_index, definition.id.clone(), weight))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(source_index, _, _)| *source_index);
        candidates
    }

    fn mutation_random_weight(
        &self,
        definition: &MutationDefinition,
        operation: RandomMutationOperation,
    ) -> u64 {
        let base = u64::from(definition.random_weight);
        if base == 0 {
            return 0;
        }
        let luck = self.player_luck_bias();
        let positive = matches!(
            definition.rating,
            MutationRatingDefinition::Good | MutationRatingDefinition::Great
        );
        let negative = matches!(
            definition.rating,
            MutationRatingDefinition::Awful | MutationRatingDefinition::Bad
        );
        let reduced = match operation {
            RandomMutationOperation::Gain => {
                (luck == LuckBias::Good && negative) || (luck == LuckBias::Bad && positive)
            }
            RandomMutationOperation::Lose => {
                (luck == LuckBias::Good && positive) || (luck == LuckBias::Bad && negative)
            }
        };
        if reduced { 1 } else { base }
    }

    pub(super) fn player_luck_bias(&self) -> LuckBias {
        match (
            self.progress
                .active_mutation_ids
                .contains(GOOD_LUCK_MUTATION_ID),
            self.progress
                .active_mutation_ids
                .contains(BAD_LUCK_MUTATION_ID),
        ) {
            (true, false) => LuckBias::Good,
            (false, true) => LuckBias::Bad,
            _ => LuckBias::Neutral,
        }
    }

    pub(super) fn apply_easy_tiring_fatigue(&mut self, energy: i32) {
        if energy < 1
            || !self
                .progress
                .active_mutation_ids
                .contains(EASY_TIRING_MUTATION_ID)
            || self.rng.bounded(u64::from(16 - self.minor_slow)) != 0
        {
            return;
        }
        let energy = u16::try_from(energy).unwrap_or(u16::MAX);
        if self.minor_slow_energy >= energy {
            self.minor_slow_energy -= energy;
        } else if self.minor_slow < 10 {
            self.minor_slow += 1;
            self.minor_slow_energy = self
                .minor_slow_energy
                .saturating_add(100_u16.saturating_sub(energy));
        } else {
            self.minor_slow_energy = 0;
        }
    }

    pub(super) fn process_minor_slow_recovery(&mut self) {
        if self.minor_slow == 0 {
            self.minor_slow_energy = 0;
            return;
        }
        let mut regeneration = self.player_regeneration_rate_percent() / 100;
        if regeneration == 0 && self.rng.bounded(3) == 0 {
            regeneration = 1;
        }
        let recovered = regeneration
            .saturating_mul(u64::from(self.minor_slow) * 2 + 2)
            .saturating_div(3);
        self.minor_slow_energy = self
            .minor_slow_energy
            .saturating_add(u16::try_from(recovered).unwrap_or(u16::MAX));
        if self.minor_slow_energy >= 100 {
            self.minor_slow_energy -= 100;
            self.minor_slow -= 1;
        }
    }

    pub(super) fn apply_impotence_device_skill_modifier(
        &self,
        ability: &DerivedStat,
        item: &ItemInstance,
        definition: &rfb_content::ItemDefinition,
        effect: &ItemUseEffectDefinition,
    ) -> DerivedStat {
        if !self
            .progress
            .active_mutation_ids
            .contains(IMPOTENCE_MUTATION_ID)
            || !definition
                .tags
                .iter()
                .any(|tag| tag == "staff" || tag == "rod")
        {
            return ability.clone();
        }
        let extra = impotence_extra_effect(effect)
            || definition
                .tags
                .iter()
                .any(|tag| tag == "fireball" || tag == "quickness")
            || item.affix_ids.iter().any(|affix_id| {
                self.content.affix(affix_id).is_some_and(|affix| {
                    affix
                        .tags
                        .iter()
                        .any(|tag| tag == "fireball" || tag == "quickness")
                })
            });
        ability.with_modifier(
            StatLayer::Status,
            IMPOTENCE_MUTATION_ID,
            if extra { -30 } else { -10 },
            StatBounds::NON_NEGATIVE,
        )
    }

    pub(super) fn mutation_regeneration_percent(&self) -> u64 {
        let unlocked = self
            .progress
            .active_mutation_ids
            .len()
            .saturating_sub(self.progress.locked_mutation_ids.len());
        100_u64
            .saturating_sub(
                u64::try_from(unlocked)
                    .unwrap_or(u64::MAX)
                    .saturating_mul(10),
            )
            .max(10)
    }

    pub(super) fn player_mutation_action_energy_cost(&self, action: &GameAction, cost: i32) -> i32 {
        let mut mutations = self
            .content
            .mutations()
            .filter(|definition| self.progress.active_mutation_ids.contains(&definition.id))
            .collect::<Vec<_>>();
        let walking = matches!(
            action,
            GameAction::Move { .. } | GameAction::TravelWorld { .. }
        );
        let scroll_use = match action {
            GameAction::UseItem { item_id, .. } => self
                .items
                .iter()
                .find(|item| item.id == *item_id)
                .and_then(|item| self.content.item(&item.kind_id))
                .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "scroll")),
            _ => false,
        };
        if walking {
            // RFB applies Limp before Fleet of Foot; descending source order
            // preserves that integer-rounding order without hard-coded IDs.
            mutations.sort_by_key(|definition| std::cmp::Reverse(definition.source_index));
        }
        let scaled = mutations.into_iter().fold(
            u64::try_from(cost.max(0)).unwrap_or(0),
            |value, mutation| {
                if walking {
                    mutation
                        .movement_energy_multiplier
                        .map_or(value, |ratio| scale_by_ratio(value, ratio))
                } else if scroll_use {
                    mutation
                        .scroll_energy_multiplier
                        .map_or(value, |ratio| scale_by_ratio(value, ratio))
                } else {
                    value
                }
            },
        );
        i32::try_from(scaled).unwrap_or(i32::MAX)
    }

    pub(super) fn player_kill_experience_reward(&self, amount: u64) -> u64 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(amount, |value, mutation| {
                value
                    .saturating_mul(
                        100_u64.saturating_add(u64::from(mutation.kill_experience_bonus_percent)),
                    )
                    .saturating_div(100)
            })
    }

    pub(super) fn player_relative_experience_reward(&self, amount: u64) -> u64 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .filter_map(|mutation| mutation.relative_experience_multiplier)
            .fold(amount, scale_by_ratio)
    }

    pub(super) fn player_spell_failure_modifier_percent(&self) -> i32 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0_i32, |total, mutation| {
                total.saturating_add(mutation.spell_failure_modifier_percent)
            })
    }

    pub(super) fn player_auto_identifies_items(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.auto_identify_items && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }

    pub(super) fn player_has_black_market_standard_prices(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.black_market_standard_prices
                && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }

    pub(super) fn player_resists_dispel(&mut self) -> bool {
        let chance = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0_u8, |total, mutation| {
                total.saturating_add(mutation.dispel_resistance_percent)
            })
            .min(100);
        chance > 0 && self.rng.bounded(100) < u64::from(chance)
    }

    pub(super) fn player_has_resource_drain_immunity(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.resource_drain_immunity
                && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }
}

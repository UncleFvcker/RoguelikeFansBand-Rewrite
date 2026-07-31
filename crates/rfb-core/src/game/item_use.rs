// SPDX-License-Identifier: MPL-2.0

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ItemUsePlan {
    SelfTarget,
    GlyphGenocide {
        glyph: String,
    },
    CreateAdjacentTerrain {
        replacements: Vec<(Position, String)>,
    },
    DestroyAdjacentTrapsAndDoors {
        replacements: Vec<(Position, String)>,
    },
    VisibleActors {
        actor_ids: Vec<String>,
    },
    Projectile {
        path: Vec<Position>,
    },
    Detect,
    SummonCategory {
        category: String,
        candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
    },
    Item {
        item_id: String,
    },
    RandomTeleport {
        candidates: Vec<Position>,
    },
    TeleportLevel {
        upward_targets: Vec<FloorTransitionTarget>,
        downward_targets: Vec<FloorTransitionTarget>,
    },
    Recall(RecallUseAction),
    ResetRecall(floor::RecallDestination),
}

pub(super) struct SettledItemUse {
    pub(super) kind_id: String,
    pub(super) profile_id: Option<String>,
    pub(super) effect: ItemUseEffectDefinition,
    pub(super) plan: ItemUsePlan,
}

impl Game {
    pub(super) fn item_attribute_kind(attribute: &ItemAttributeDefinition) -> AttributeKind {
        match attribute {
            ItemAttributeDefinition::Strength => AttributeKind::Strength,
            ItemAttributeDefinition::Intelligence => AttributeKind::Intelligence,
            ItemAttributeDefinition::Wisdom => AttributeKind::Wisdom,
            ItemAttributeDefinition::Dexterity => AttributeKind::Dexterity,
            ItemAttributeDefinition::Constitution => AttributeKind::Constitution,
            ItemAttributeDefinition::Charisma => AttributeKind::Charisma,
        }
    }

    pub(super) fn resolve_item_drain_attribute(
        &mut self,
        source_kind_id: &str,
        attribute: AttributeKind,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        if self
            .player_equipment_passives()
            .contains(&attribute_sustain_passive(attribute))
        {
            let value = self.progress.attributes.value(attribute);
            self.mark_item_aware(source_kind_id);
            events.push(DomainEvent::ItemAttributeChanged {
                source_kind_id: source_kind_id.to_owned(),
                display_name_key: self.item_display_name_key(source_kind_id),
                attribute,
                change: ItemAttributeChange::Sustained,
                before: value,
                after: value,
                maximum: self.progress.maximum_attributes.value(attribute),
                noticed: true,
            });
            return true;
        }
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let before = self.progress.attributes.value(attribute);
        let noticed = self.progress.drain_attribute(attribute, &mut self.rng);
        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemAttributeChanged {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            attribute,
            change: ItemAttributeChange::Drained,
            before,
            after: self.progress.attributes.value(attribute),
            maximum: self.progress.maximum_attributes.value(attribute),
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_restore_attribute(
        &mut self,
        source_kind_id: &str,
        attribute: AttributeKind,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let before = self.progress.attributes.value(attribute);
        let noticed = self.progress.restore_attribute(attribute);
        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemAttributeChanged {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            attribute,
            change: ItemAttributeChange::Restored,
            before,
            after: self.progress.attributes.value(attribute),
            maximum: self.progress.maximum_attributes.value(attribute),
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_increase_attributes(
        &mut self,
        source_kind_id: &str,
        attributes: &[AttributeKind],
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let victorious = self.victory_level_cap_unlocked();
        let mut noticed = false;
        let mut resolutions = Vec::with_capacity(attributes.len());

        for &attribute in attributes {
            let before = self.progress.attributes.value(attribute);
            let maximum_before = self.progress.maximum_attributes.value(attribute);
            let changed =
                self.progress
                    .increase_attribute_permanently(attribute, victorious, &mut self.rng);
            let after = self.progress.attributes.value(attribute);
            let maximum = self.progress.maximum_attributes.value(attribute);
            let change = if maximum > maximum_before {
                ItemAttributeChange::Increased
            } else if after > before {
                ItemAttributeChange::Restored
            } else {
                ItemAttributeChange::Increased
            };
            resolutions.push((attribute, change, before, after, maximum, changed));
            noticed = changed || noticed;
        }

        if noticed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
            self.mark_item_aware(source_kind_id);
        }
        let display_name_key = self.item_display_name_key(source_kind_id);
        for (attribute, change, before, after, maximum, changed) in resolutions {
            events.push(DomainEvent::ItemAttributeChanged {
                source_kind_id: source_kind_id.to_owned(),
                display_name_key: display_name_key.clone(),
                attribute,
                change,
                before,
                after,
                maximum,
                noticed: changed,
            });
        }
        noticed
    }

    pub(super) fn use_inventory_item(
        &mut self,
        item_id: &str,
        target: Option<&TargetSelection>,
        target_glyph: Option<&str>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some((index, definition)) = self.inventory_item_use_context(item_id)? else {
            events.push(DomainEvent::ItemUseUnavailable);
            return Ok(());
        };
        let kind_id = self.items[index].kind_id.clone();
        let activation = self.items[index].activation.clone();
        let (profile_id, difficulty, cost, effect, plan) =
            if let Some(activation) = activation.as_ref() {
                let profile = definition
                    .device_generation
                    .as_ref()
                    .and_then(|generation| {
                        generation
                            .activations
                            .iter()
                            .find(|candidate| candidate.id == activation.profile_id)
                    })
                    .expect("validated dynamic item activation profile must remain available");
                let Some(plan) = self.item_use_plan(
                    item_id,
                    &profile.effect,
                    Some(&profile.target),
                    target,
                    target_glyph,
                ) else {
                    events.push(DomainEvent::ItemUseUnavailable);
                    return Ok(());
                };
                (
                    Some(activation.profile_id.clone()),
                    Some(activation.device_check_difficulty),
                    Some(activation.cost),
                    profile.effect.clone(),
                    plan,
                )
            } else if let Some(action) = definition.use_action {
                let Some(plan) =
                    self.item_use_plan(item_id, &action.effect, None, target, target_glyph)
                else {
                    events.push(DomainEvent::ItemUseUnavailable);
                    return Ok(());
                };
                (
                    None,
                    action.device_check_difficulty,
                    action.charges.map(|charges| charges.cost),
                    action.effect,
                    plan,
                )
            } else {
                events.push(DomainEvent::ItemUseUnavailable);
                return Ok(());
            };
        if cost.is_some_and(|cost| {
            self.items[index]
                .charges
                .is_none_or(|state| state.current < cost)
        }) {
            events.push(DomainEvent::ItemUseUnavailable);
            return Ok(());
        }

        self.mark_item_tried(&kind_id);
        if let Some(difficulty) = difficulty {
            let ability = self.player_derived_stats().device_skill;
            let mut difficulty_pipeline = DerivedStatsPipeline::new();
            difficulty_pipeline.add(
                StatKind::ActionDifficulty,
                StatLayer::Environment,
                &kind_id,
                difficulty,
            );
            let check = resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::UseDevice,
                    actor_id: self.player.id.clone(),
                    target_id: Some(item_id.to_owned()),
                    ability,
                    difficulty: difficulty_pipeline
                        .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                },
            );
            let succeeded = check.succeeded();
            let skill_id = self
                .content
                .skill_by_kind(SkillKind::Device)
                .expect("validated device skill must remain available")
                .id
                .clone();
            events.push(DomainEvent::DeviceSkillChecked {
                source_kind_id: kind_id.clone(),
                succeeded,
                resolution: check.to_dto(skill_id),
            });
            if !succeeded {
                return Ok(());
            }
        }

        if let Some(cost) = cost {
            self.items[index]
                .charges
                .as_mut()
                .expect("validated charged item must carry charge state")
                .current -= cost;
        } else if self.items[index].quantity == 1 {
            let removed = self.items.remove(index);
            self.item_property_knowledge.remove(&removed.id);
        } else {
            self.items[index].quantity -= 1;
        }
        self.resolve_inventory_item_effect(
            SettledItemUse {
                kind_id,
                profile_id,
                effect,
                plan,
            },
            events,
            changed,
            removed_entities,
        )
    }

    pub(super) fn item_use_plan(
        &self,
        source_item_id: &str,
        effect: &ItemUseEffectDefinition,
        target_definition: Option<&AbilityTargetDefinition>,
        target: Option<&TargetSelection>,
        target_glyph: Option<&str>,
    ) -> Option<ItemUsePlan> {
        if target_glyph.is_some() && !matches!(effect, ItemUseEffectDefinition::Genocide { .. }) {
            return None;
        }
        let self_target = target.is_none_or(|target| matches!(target, TargetSelection::SelfTarget));
        match effect {
            ItemUseEffectDefinition::Heal { .. }
            | ItemUseEffectDefinition::HealDice { .. }
            | ItemUseEffectDefinition::Bless { .. }
            | ItemUseEffectDefinition::ApplySlowness { .. }
            | ItemUseEffectDefinition::ApplySpeed { .. }
            | ItemUseEffectDefinition::ApplyHeroism { .. }
            | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
            | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
            | ItemUseEffectDefinition::ApplyStoneSkin { .. }
            | ItemUseEffectDefinition::RestoreLifeLevels { .. }
            | ItemUseEffectDefinition::RestoreAllAttributes
            | ItemUseEffectDefinition::RestoreAllVitality { .. }
            | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
            | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
            | ItemUseEffectDefinition::DrainAttribute { .. }
            | ItemUseEffectDefinition::RestoreAttribute { .. }
            | ItemUseEffectDefinition::IncreaseAttribute { .. }
            | ItemUseEffectDefinition::AugmentAttributes
            | ItemUseEffectDefinition::ApplyThermalResistance { .. }
            | ItemUseEffectDefinition::ApplyBasicResistance { .. }
            | ItemUseEffectDefinition::ApplyPoison { .. }
            | ItemUseEffectDefinition::ApplyBlindness { .. }
            | ItemUseEffectDefinition::ApplyDetonation { .. }
            | ItemUseEffectDefinition::SelfLifeLoss { .. }
            | ItemUseEffectDefinition::Vengeance { .. }
            | ItemUseEffectDefinition::ProtectionFromEvil
            | ItemUseEffectDefinition::PrepareConfusingStrike
            | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
            | ItemUseEffectDefinition::SelfCenteredElementalBlast { .. }
            | ItemUseEffectDefinition::AggravateMonsters
            | ItemUseEffectDefinition::MassGenocide { .. }
            | ItemUseEffectDefinition::RemoveStatus { .. }
            | ItemUseEffectDefinition::RestoreResource { .. }
            | ItemUseEffectDefinition::RestoreResourceDice { .. }
            | ItemUseEffectDefinition::RestoreResourceFull { .. }
            | ItemUseEffectDefinition::Sequence { .. }
            | ItemUseEffectDefinition::CurseEquippedItem { .. }
            | ItemUseEffectDefinition::RemoveEquippedCurses { .. } => {
                self_target.then_some(ItemUsePlan::SelfTarget)
            }
            ItemUseEffectDefinition::Genocide { .. } => {
                if target.is_some() {
                    return None;
                }
                let glyph = target_glyph?;
                let mut characters = glyph.chars();
                let character = characters.next()?;
                (!character.is_control() && characters.next().is_none()).then(|| {
                    ItemUsePlan::GlyphGenocide {
                        glyph: glyph.to_owned(),
                    }
                })
            }
            ItemUseEffectDefinition::RechargeFromDevice { .. } => None,
            ItemUseEffectDefinition::CreateAdjacentTerrain {
                source_terrain_ids,
                target_terrain_id,
            } => self_target.then(|| ItemUsePlan::CreateAdjacentTerrain {
                replacements: self
                    .adjacent_terrain_creation_replacements(source_terrain_ids, target_terrain_id),
            }),
            ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors => {
                self_target.then(|| ItemUsePlan::DestroyAdjacentTrapsAndDoors {
                    replacements: self.adjacent_trap_door_replacements(),
                })
            }
            ItemUseEffectDefinition::Damage { .. } => {
                let path = target_definition.and_then(|definition| {
                    target.and_then(|target| self.item_effect_path(definition, target))
                })?;
                Some(ItemUsePlan::Projectile { path })
            }
            ItemUseEffectDefinition::DispelCategory { .. }
            | ItemUseEffectDefinition::BanishVisible { .. } => {
                self_target.then(|| ItemUsePlan::VisibleActors {
                    actor_ids: self.item_visible_actor_ids(),
                })
            }
            ItemUseEffectDefinition::Detect { .. } => self_target.then_some(ItemUsePlan::Detect),
            effect @ ItemUseEffectDefinition::SummonCategory { .. } => {
                self_target.then(|| self.item_category_summon_plan(effect))
            }
            ItemUseEffectDefinition::IdentifyItem { .. } => {
                let TargetSelection::Item {
                    item_id: target_item_id,
                } = target?
                else {
                    return None;
                };
                self.item_is_valid_identify_target(source_item_id, target_item_id)
                    .then(|| ItemUsePlan::Item {
                        item_id: target_item_id.clone(),
                    })
            }
            effect @ ItemUseEffectDefinition::EnchantItem { .. } => {
                let TargetSelection::Item {
                    item_id: target_item_id,
                } = target?
                else {
                    return None;
                };
                self.item_is_valid_enchant_target(source_item_id, target_item_id, effect)
                    .then(|| ItemUsePlan::Item {
                        item_id: target_item_id.clone(),
                    })
            }
            ItemUseEffectDefinition::RandomTeleport { maximum_distance } => {
                if !self_target {
                    return None;
                }
                let candidates = self.random_teleport_candidates(*maximum_distance);
                (!candidates.is_empty()).then_some(ItemUsePlan::RandomTeleport { candidates })
            }
            ItemUseEffectDefinition::TeleportLevel => {
                if !self_target {
                    return None;
                }
                let (upward_targets, downward_targets) = self.teleport_level_targets();
                (!upward_targets.is_empty() || !downward_targets.is_empty()).then_some(
                    ItemUsePlan::TeleportLevel {
                        upward_targets,
                        downward_targets,
                    },
                )
            }
            ItemUseEffectDefinition::Recall { .. } => self_target
                .then(|| self.recall_use_plan())
                .flatten()
                .map(ItemUsePlan::Recall),
            ItemUseEffectDefinition::ResetRecall => self_target
                .then(|| self.recall_reset_plan())
                .flatten()
                .map(ItemUsePlan::ResetRecall),
        }
    }
}

impl Game {
    pub(super) fn resolve_item_restorative_resource_effect(
        &mut self,
        source_kind_id: &str,
        effect: &ItemUseEffectDefinition,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        match effect {
            ItemUseEffectDefinition::Heal { amount } => {
                let amount = i32::try_from(*amount).expect("validated healing amount must fit i32");
                self.resolve_item_healing(source_kind_id, amount, events)
            }
            ItemUseEffectDefinition::HealDice { dice, sides } => {
                let amount = self.roll_damage(*dice, *sides);
                self.resolve_item_healing(source_kind_id, amount, events)
            }
            ItemUseEffectDefinition::RestoreResource {
                resource_id,
                amount,
            } => self.resolve_item_resource_restoration(
                source_kind_id,
                resource_id,
                *amount,
                false,
                events,
            ),
            ItemUseEffectDefinition::RestoreResourceDice {
                resource_id,
                dice,
                sides,
                bonus,
            } => {
                let rolled = u32::try_from(self.roll_damage(*dice, *sides))
                    .expect("validated resource restoration roll must fit u32")
                    .saturating_add(*bonus);
                self.resolve_item_resource_restoration(
                    source_kind_id,
                    resource_id,
                    rolled,
                    false,
                    events,
                )
            }
            ItemUseEffectDefinition::RestoreResourceFull { resource_id } => {
                self.resolve_item_resource_restoration(source_kind_id, resource_id, 0, true, events)
            }
            _ => {
                unreachable!("restorative resource executor requires healing or resource recovery")
            }
        }
    }

    pub(super) fn resolve_item_vitality_restoration_effect(
        &mut self,
        source_kind_id: &str,
        effect: &ItemUseEffectDefinition,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        match effect {
            ItemUseEffectDefinition::RestoreLifeLevels { life_force_amount } => {
                self.resolve_item_restore_life_levels(source_kind_id, *life_force_amount, events)
            }
            ItemUseEffectDefinition::RestoreAllAttributes => {
                let noticed = self.restore_all_player_attributes();
                if noticed {
                    self.mark_item_aware(source_kind_id);
                }
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed,
                });
                noticed
            }
            ItemUseEffectDefinition::RestoreAllVitality { life_force_amount } => {
                let attributes_restored = self.restore_all_player_attributes();
                let vitality_restored =
                    self.restore_player_experience_and_life_force(*life_force_amount, events);
                let noticed = attributes_restored || vitality_restored;
                if noticed {
                    self.mark_item_aware(source_kind_id);
                }
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed,
                });
                noticed
            }
            ItemUseEffectDefinition::ApplyRestorativeFeast {
                healing_dice,
                healing_sides,
            } => {
                if let Some(index) = self
                    .player
                    .statuses
                    .iter()
                    .position(|status| status.kind_id == STATUS_POISON)
                {
                    let before = self.player.statuses[index].remaining_ticks;
                    let reduction = (before / 5).max(100);
                    let after = before.saturating_sub(reduction);
                    if after == 0 {
                        self.player.statuses.remove(index);
                    } else {
                        self.player.statuses[index].remaining_ticks = after;
                    }
                }
                let healing = self.roll_damage(*healing_dice, *healing_sides);
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::Heal { amount: healing },
                );
                self.restore_all_player_attributes();
                self.restore_player_experience_and_life_force(0, events);
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed: true,
                });
                true
            }
            ItemUseEffectDefinition::ApplyLifeRestoration {
                healing_amount,
                life_force_amount,
            } => {
                self.restore_player_experience_and_life_force(*life_force_amount, events);
                self.player.statuses.retain(|status| {
                    !matches!(
                        status.kind_id.as_str(),
                        STATUS_POISON
                            | STATUS_BLINDNESS
                            | STATUS_CONFUSION
                            | STATUS_STUN
                            | STATUS_BLEEDING
                            | STATUS_SLOW
                            | "rfb.status.berserk"
                    )
                });
                self.restore_all_player_attributes();
                let amount = i32::try_from(*healing_amount)
                    .expect("validated life restoration amount must fit i32");
                let max_hp = self.effective_player_max_hp();
                let player = &mut self.player;
                apply_effect(
                    &mut EffectTarget {
                        hp: &mut player.hp,
                        max_hp,
                        resistances: &player.resistances,
                        statuses: &mut player.statuses,
                    },
                    EffectSpec::Heal { amount },
                );
                self.mark_item_aware(source_kind_id);
                events.push(DomainEvent::ItemRestorationResolved {
                    source_kind_id: source_kind_id.to_owned(),
                    display_name_key: self.item_display_name_key(source_kind_id),
                    noticed: true,
                });
                true
            }
            _ => unreachable!("vitality restoration executor requires a restoration effect"),
        }
    }

    fn resolve_item_restore_life_levels(
        &mut self,
        source_kind_id: &str,
        life_force_amount: u16,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let noticed = self.restore_player_experience_and_life_force(life_force_amount, events);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemRestoreLifeLevelsResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            noticed,
        });
        noticed
    }

    fn restore_all_player_attributes(&mut self) -> bool {
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut restored = false;
        for attribute in [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ] {
            restored = self.progress.restore_attribute(attribute) || restored;
        }
        if restored {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
        restored
    }

    fn restore_player_experience_and_life_force(
        &mut self,
        life_force_amount: u16,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let experience_before = self.progress.experience;
        let life_force_before = self.progress.life_force;
        self.progress.experience = self.progress.maximum_experience;
        self.apply_player_experience(0, events);
        self.progress.life_force = self
            .progress
            .life_force
            .saturating_add(life_force_amount)
            .min(1_000);
        self.progress.experience != experience_before
            || self.progress.life_force != life_force_before
    }

    pub(super) fn resolve_item_healing(
        &mut self,
        source_kind_id: &str,
        amount: i32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let max_hp = self.effective_player_max_hp();
        let player = &mut self.player;
        let outcome = apply_effect(
            &mut EffectTarget {
                hp: &mut player.hp,
                max_hp,
                resistances: &player.resistances,
                statuses: &mut player.statuses,
            },
            EffectSpec::Heal { amount },
        );
        let EffectOutcome::Healed { requested, applied } = outcome else {
            unreachable!("healing effects must produce healing outcomes");
        };
        if applied > 0 {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemUsed {
            display_name_key: self.item_display_name_key(source_kind_id),
            source_kind_id: source_kind_id.to_owned(),
            requested,
            applied,
        });
        applied > 0
    }

    fn resolve_item_resource_restoration(
        &mut self,
        source_kind_id: &str,
        resource_id: &str,
        requested: u32,
        full: bool,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let (before, after) = if let Some(pool) = self.resources.get_mut(resource_id) {
            let before = pool.current;
            pool.current = if full {
                pool.maximum
            } else {
                pool.current.saturating_add(requested).min(pool.maximum)
            };
            (before, pool.current)
        } else {
            (0, 0)
        };
        let recovered = after.saturating_sub(before);
        if recovered > 0 {
            self.resources_touched.insert(resource_id.to_owned());
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemResourceRestored {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            resolution: ResourceRecoveryResolutionDto {
                resource_id: resource_id.to_owned(),
                before,
                after,
                recovered,
            },
        });
        recovered > 0
    }

    pub(super) fn resolve_item_confusing_strike_preparation(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        self.confusing_strike_ready = true;
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemConfusingStrikePrepared {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
        });
        true
    }

    pub(super) fn resolve_item_status_removal(
        &mut self,
        source_kind_id: &str,
        status_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let max_hp = self.effective_player_max_hp();
        let player = &mut self.player;
        let outcome = apply_effect(
            &mut EffectTarget {
                hp: &mut player.hp,
                max_hp,
                resistances: &player.resistances,
                statuses: &mut player.statuses,
            },
            EffectSpec::RemoveStatus {
                kind_id: status_kind_id.to_owned(),
            },
        );
        let EffectOutcome::StatusRemoved { removed, .. } = outcome else {
            unreachable!("status removal must produce a status outcome");
        };
        if removed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemStatusRemoved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            status_kind_id: status_kind_id.to_owned(),
            removed,
        });
        removed
    }

    pub(super) fn resolve_item_protection_from_evil(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let duration = u32::from(self.progress.level)
            .saturating_mul(3)
            .saturating_add(
                u32::try_from(self.roll_damage(1, 25))
                    .expect("protection from evil duration must fit u32"),
            );
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            STATUS_PROTECTION_FROM_EVIL,
            1,
            duration,
            0,
            1,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemProtectionFromEvil {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    pub(super) fn resolve_item_blessing(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.blessed",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense: 5,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 10,
                ranged_skill: 10,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let duration = match &resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                ..
            } => *applied_duration_ticks,
            _ => unreachable!("blessing must produce a status application resolution"),
        };
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemBlessed {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }

    pub(super) fn resolve_item_slowness(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated slowness die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated slowness duration must fit u32")
            .saturating_add(duration_bonus);
        let change = if self.player_status_immunities().contains(STATUS_SLOW) {
            StatusChange::Unchanged
        } else {
            apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_SLOW.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::KeepStrongest,
                },
            )
        };
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemSlownessResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_speed(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let already_hasted = self
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_HASTE);
        let duration = if already_hasted {
            5
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated speed die sides must fit u16");
            u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated speed duration must fit u32")
                .saturating_add(duration_bonus)
        };
        let change = apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_HASTE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::new(),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        );
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemSpeedResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
        noticed
    }

    pub(super) fn resolve_item_heroism(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.hero",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                max_hp: 10,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 12,
                ranged_skill: 12,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::from([STATUS_FEAR.to_owned()]),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("heroism must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemHeroismResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_berserk_strength(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.berserk",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense: -10,
                max_hp: 30,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 12,
                melee_damage: 3 + i32::from(self.progress.level / 5),
                ranged_skill: -12,
                throwing_skill: -20,
                device_skill: -20,
                saving_throw_skill: -30,
                stealth_skill: -7,
                search_skill: -15,
                perception_skill: -15,
                digging_skill: 30,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::from([STATUS_FEAR.to_owned()]),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, status_noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("berserk strength must produce a status application resolution"),
        };
        if status_noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemBerserkStrengthResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed: status_noticed,
        });
        let healed = self.resolve_item_healing(source_kind_id, 30, events);
        status_noticed || healed
    }

    pub(super) fn resolve_item_poetic_inspiration(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.poetic-inspiration",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::Extend,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                wisdom: 5,
                charisma: 5,
                ..StatModifiers::default()
            },
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("poetic inspiration must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemPoeticInspirationResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_stone_skin(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let defense = 10 + 40 * i32::from(self.progress.level) / 50;
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            "rfb.status.stone-skin",
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers {
                defense,
                ..StatModifiers::default()
            },
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let (duration, noticed) = match resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                change,
                ..
            } => (
                applied_duration_ticks,
                matches!(change, AbilityStatusChangeDto::Added),
            ),
            _ => unreachable!("stone skin must produce a status application resolution"),
        };
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemStoneSkinResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_thermal_resistance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated thermal die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated thermal duration must fit u32")
            .saturating_add(duration_bonus);
        let change = apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_THERMAL_RESISTANCE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::from([
                        (DamageType::Fire, ResistanceLevel::Resistant),
                        (DamageType::Cold, ResistanceLevel::Resistant),
                    ]),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        );
        let noticed = matches!(change, StatusChange::Added);
        if noticed {
            self.mark_item_aware(source_kind_id);
        }
        events.push(DomainEvent::ItemThermalResistanceResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_basic_resistance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let duration_sides =
            u16::try_from(duration_sides).expect("validated resistance die sides must fit u16");
        let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
            .expect("validated resistance duration must fit u32")
            .saturating_add(duration_bonus);
        apply_status(
            &mut self.player.statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_BASIC_RESISTANCE.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: Some(source_kind_id.to_owned()),
                    granted_resistances: BTreeMap::from([
                        (DamageType::Acid, ResistanceLevel::Resistant),
                        (DamageType::Electricity, ResistanceLevel::Resistant),
                        (DamageType::Fire, ResistanceLevel::Resistant),
                        (DamageType::Cold, ResistanceLevel::Resistant),
                        (DamageType::Poison, ResistanceLevel::Resistant),
                    ]),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::KeepStrongest,
            },
        );
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemBasicResistanceApplied {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
    }

    pub(super) fn resolve_item_poison(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resistance = self
            .effective_player_resistances()
            .level(DamageType::Poison);
        let resistance_threshold = u64::try_from(resistance.reduction_percent().max(0))
            .expect("threshold is non-negative");
        let resisted = self.rng.bounded(55) < resistance_threshold;
        let duration = if resisted {
            None
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated poison die sides must fit u16");
            let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated poison duration must fit u32")
                .saturating_add(duration_bonus);
            apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_POISON.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Extend,
                },
            );
            self.mark_item_aware(source_kind_id);
            Some(duration)
        };
        events.push(DomainEvent::ItemPoisonResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
        });
        !resisted
    }

    pub(super) fn resolve_item_blindness(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let resistance_threshold = if self.player_status_immunities().contains(STATUS_BLINDNESS) {
            55
        } else {
            0
        };
        let resisted = self.rng.bounded(55) < resistance_threshold;
        let (duration, noticed) = if resisted {
            (None, false)
        } else {
            let duration_sides =
                u16::try_from(duration_sides).expect("validated blindness die sides must fit u16");
            let duration = u32::try_from(self.roll_damage(duration_dice, duration_sides))
                .expect("validated blindness duration must fit u32")
                .saturating_add(duration_bonus);
            let change = apply_status(
                &mut self.player.statuses,
                StatusApplication {
                    status: StatusInstance {
                        kind_id: STATUS_BLINDNESS.to_owned(),
                        intensity: 1,
                        remaining_ticks: duration,
                        source_id: Some(source_kind_id.to_owned()),
                        granted_resistances: BTreeMap::new(),
                        granted_brands: BTreeSet::new(),
                        granted_modifiers: StatModifiersDto::default(),
                        granted_equipment_bonuses: EquipmentBonusesDto::default(),
                        granted_status_immunities: BTreeSet::new(),
                        granted_race_id: None,
                        grants_wall_passage: false,
                        incoming_damage_percent: 100,
                    },
                    stacking: StatusStacking::Extend,
                },
            );
            let noticed = matches!(change, StatusChange::Added);
            if noticed {
                self.mark_item_aware(source_kind_id);
            }
            (Some(duration), noticed)
        };
        events.push(DomainEvent::ItemBlindnessResolved {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            noticed,
        });
        noticed
    }

    pub(super) fn resolve_item_vengeance(
        &mut self,
        source_kind_id: &str,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        events: &mut Vec<DomainEvent>,
    ) {
        let resolution = apply_ability_status_effect(
            &mut self.player,
            source_kind_id,
            0,
            STATUS_VENGEANCE,
            1,
            duration_bonus,
            duration_dice,
            duration_sides,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let duration = match &resolution {
            AbilityEffectResolutionDto::ApplyStatus {
                applied_duration_ticks,
                ..
            } => *applied_duration_ticks,
            _ => unreachable!("vengeance must produce a status application resolution"),
        };
        self.mark_item_aware(source_kind_id);
        events.push(DomainEvent::ItemVengeanceActivated {
            source_kind_id: source_kind_id.to_owned(),
            display_name_key: self.item_display_name_key(source_kind_id),
            duration,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![resolution],
            },
        });
    }
}

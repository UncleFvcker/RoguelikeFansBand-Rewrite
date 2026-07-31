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

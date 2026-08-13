// SPDX-License-Identifier: MPL-2.0

use super::*;

const SURFACE_GRASS_TERRAIN_ID: &str = "demo.terrain.surface-grass";

fn projectile_destroys_tree(damage_type: DamageType) -> bool {
    matches!(
        damage_type,
        DamageType::Acid
            | DamageType::Electricity
            | DamageType::Fire
            | DamageType::Cold
            | DamageType::Poison
            | DamageType::Sound
            | DamageType::Shards
            | DamageType::Chaos
            | DamageType::Disenchant
            | DamageType::Time
            | DamageType::Mana
            | DamageType::Gravity
            | DamageType::Plasma
            | DamageType::Force
            | DamageType::Nuke
            | DamageType::Ice
            | DamageType::Meteor
            | DamageType::Rocket
            | DamageType::Disintegrate
    )
}

pub(super) fn ground_item_damage_type_for_ability_effect(
    effect: &AbilityEffectDefinition,
) -> Option<DamageType> {
    match effect {
        AbilityEffectDefinition::Damage { damage_type, .. }
        | AbilityEffectDefinition::AreaDamage { damage_type, .. }
        | AbilityEffectDefinition::BeamDamage { damage_type, .. }
        | AbilityEffectDefinition::BoltOrBeamDamage { damage_type, .. }
        | AbilityEffectDefinition::BoltOrAreaDamage { damage_type, .. }
        | AbilityEffectDefinition::ConeDamage { damage_type, .. } => {
            Some(DamageType::from(*damage_type))
        }
        AbilityEffectDefinition::Malediction { .. } => Some(DamageType::HellFire),
        _ => None,
    }
}

impl Game {
    pub(super) fn resolve_projectile_terrain_effects(
        &mut self,
        affected_positions: &[Position],
        damage_type: DamageType,
        changed: &mut BTreeSet<Position>,
    ) {
        if damage_type == DamageType::Disintegrate {
            for position in affected_positions.iter().copied().collect::<BTreeSet<_>>() {
                let Some(index) = self.index(position) else {
                    continue;
                };
                let Some(target_id) = self
                    .content
                    .terrain(&self.terrain[index])
                    .and_then(|terrain| terrain.monster_destroy_to_terrain_id.clone())
                else {
                    continue;
                };
                if self.terrain[index] != target_id {
                    self.terrain[index] = target_id;
                    changed.insert(position);
                }
            }
        }
        if !projectile_destroys_tree(damage_type)
            || self.content.terrain(SURFACE_GRASS_TERRAIN_ID).is_none()
        {
            return;
        }
        for position in affected_positions.iter().copied().collect::<BTreeSet<_>>() {
            let Some(index) = self.index(position) else {
                continue;
            };
            let destroys_tree = self
                .content
                .terrain(&self.terrain[index])
                .is_some_and(|terrain| {
                    terrain.tags.iter().any(|tag| tag == "tree")
                        && !terrain.tags.iter().any(|tag| tag == "permanent")
                });
            if destroys_tree {
                self.terrain[index] = SURFACE_GRASS_TERRAIN_ID.to_owned();
                changed.insert(position);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_ground_item_shatter_effect(
        &mut self,
        source_kind_id: &str,
        center: Position,
        shatter: &ItemShatterEffectDefinition,
        award_player_kills: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let effects = match &shatter.effect {
            ItemUseEffectDefinition::Sequence { effects } => effects.as_slice(),
            effect => std::slice::from_ref(effect),
        };
        for (effect_index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(effect_index).expect("validated shatter effect index must fit u8");
            let cells = self.area_damage_cells(center, shatter.radius);
            let affected_positions = cells
                .iter()
                .map(|(_, position)| *position)
                .collect::<Vec<_>>();
            let trace = ProjectileTrace {
                origin: center,
                impact: center,
                landing: center,
                traversed: vec![center],
            };
            match effect {
                ItemUseEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                } => {
                    let damage_type = DamageType::from(*damage_type);
                    let raw_damage = self
                        .roll_damage(*damage_dice, *damage_sides)
                        .saturating_add(i32::from(*damage_bonus))
                        .max(0);
                    self.resolve_ground_item_projectile_effects(
                        source_kind_id,
                        &affected_positions,
                        damage_type,
                        award_player_kills,
                        events,
                        changed,
                        removed_entities,
                    );
                    if let Some((distance, _)) = cells
                        .iter()
                        .find(|(_, position)| *position == self.player.position)
                        && !self.player_is_dead()
                    {
                        let prepared = rfb_area_damage(raw_damage, *distance);
                        let target = self.player_derived_stats();
                        let resistance = self.effective_player_resistances().level(damage_type);
                        let damage = self.reduce_player_damage(resolve_armored_damage(
                            prepared,
                            damage_type,
                            target.armor_class.value,
                            resistance,
                        ));
                        let application = plan_damage_application(
                            &self.player,
                            damage,
                            FatalityPolicy::BelowZero,
                        );
                        commit_damage_application(&mut self.player, &application);
                        changed.insert(self.player.position);
                        events.push(DomainEvent::AbilityHit {
                            ability_id: source_kind_id.to_owned(),
                            target_kind_id: self.player.kind_id.clone(),
                            damage,
                            trace: trace.clone(),
                        });
                        if application.fatal {
                            events.push(DomainEvent::PlayerDied {
                                source_kind_id: source_kind_id.to_owned(),
                                method_id: Some("potion-shatter".to_owned()),
                                damage,
                            });
                        }
                    }
                    let targets = cells
                        .iter()
                        .flat_map(|(distance, position)| {
                            self.entities
                                .iter()
                                .filter(move |entity| entity.hp > 0 && entity.position == *position)
                                .map(move |entity| (entity.id.clone(), *distance))
                        })
                        .collect::<Vec<_>>();
                    for (entity_id, distance) in targets {
                        let Some(index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == entity_id && entity.hp > 0)
                        else {
                            continue;
                        };
                        let prepared = rfb_area_damage(raw_damage, distance);
                        let result = if award_player_kills {
                            self.resolve_ability_damage_to_entity(
                                index,
                                source_kind_id,
                                damage_type,
                                prepared,
                                trace.clone(),
                                events,
                                changed,
                                removed_entities,
                            )
                        } else {
                            self.resolve_ability_damage_to_entity_without_rewards(
                                index,
                                source_kind_id,
                                damage_type,
                                prepared,
                                trace.clone(),
                                events,
                                changed,
                                removed_entities,
                            )
                        };
                        result.expect("validated shatter damage transaction must resolve");
                    }
                }
                ItemUseEffectDefinition::Heal { amount } => {
                    let amount = i32::try_from(*amount)
                        .expect("validated shatter healing amount must fit i32");
                    self.resolve_ground_item_shatter_healing(
                        source_kind_id,
                        effect_index,
                        &cells,
                        amount,
                        &trace,
                        events,
                        changed,
                    );
                }
                ItemUseEffectDefinition::HealDice { dice, sides } => {
                    let amount = self.roll_damage(*dice, *sides);
                    self.resolve_ground_item_shatter_healing(
                        source_kind_id,
                        effect_index,
                        &cells,
                        amount,
                        &trace,
                        events,
                        changed,
                    );
                }
                _ => unreachable!("validated shatter programs contain only area effects"),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_ground_item_shatter_healing(
        &mut self,
        source_kind_id: &str,
        effect_index: u8,
        cells: &[(u32, Position)],
        amount: i32,
        trace: &ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if let Some((distance, _)) = cells
            .iter()
            .find(|(_, position)| *position == self.player.position)
            .filter(|_| !self.player_is_dead())
        {
            let amount = rfb_area_damage(amount, *distance);
            let maximum = self.effective_player_max_hp();
            let outcome =
                apply_healing(&mut self.player.hp, maximum, HealingRequest::amount(amount));
            changed.insert(self.player.position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: source_kind_id.to_owned(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(self.player.id.clone()),
                    target_kind_id: Some(self.player.kind_id.clone()),
                    effects: vec![AbilityEffectResolutionDto::Heal {
                        effect_index,
                        resolution: HealingResolutionDto {
                            requested: outcome.requested,
                            applied: outcome.applied,
                        },
                    }],
                },
                trace: Some(trace.clone()),
            });
        }
        let target_ids = cells
            .iter()
            .flat_map(|(distance, position)| {
                self.entities
                    .iter()
                    .filter(move |entity| entity.hp > 0 && entity.position == *position)
                    .map(move |entity| (entity.id.clone(), *distance))
            })
            .collect::<Vec<_>>();
        for (entity_id, distance) in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let amount = rfb_area_damage(amount, distance);
            let maximum = self.entities[index].max_hp;
            let outcome = apply_healing(
                &mut self.entities[index].hp,
                maximum,
                HealingRequest::amount(amount),
            );
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: source_kind_id.to_owned(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(self.entities[index].id.clone()),
                    target_kind_id: Some(self.entities[index].kind_id.clone()),
                    effects: vec![AbilityEffectResolutionDto::Heal {
                        effect_index,
                        resolution: HealingResolutionDto {
                            requested: outcome.requested,
                            applied: outcome.applied,
                        },
                    }],
                },
                trace: Some(trace.clone()),
            });
        }
    }

    fn item_has_elemental_destruction_immunity(
        &self,
        item: &ItemInstance,
        element: ItemDestructionElement,
    ) -> bool {
        let Some(definition) = self.content.item(&item.kind_id) else {
            return true;
        };
        definition
            .elemental_destruction_immunities
            .contains(&element)
            || item.affix_ids.iter().any(|affix_id| {
                self.content
                    .affix(affix_id)
                    .is_some_and(|affix| affix.elemental_destruction_immunities.contains(&element))
            })
            || item.rolled_affixes.iter().any(|rolled| {
                self.content
                    .affix(&rolled.affix_id)
                    .is_some_and(|affix| affix.elemental_destruction_immunities.contains(&element))
            })
    }

    fn item_has_elemental_destruction_vulnerability(
        &self,
        item: &ItemInstance,
        element: ItemDestructionElement,
    ) -> bool {
        let Some(definition) = self.content.item(&item.kind_id) else {
            return false;
        };
        definition
            .elemental_destruction_vulnerabilities
            .contains(&element)
            || item.affix_ids.iter().any(|affix_id| {
                self.content.affix(affix_id).is_some_and(|affix| {
                    affix
                        .elemental_destruction_vulnerabilities
                        .contains(&element)
                })
            })
            || item.rolled_affixes.iter().any(|rolled| {
                self.content.affix(&rolled.affix_id).is_some_and(|affix| {
                    affix
                        .elemental_destruction_vulnerabilities
                        .contains(&element)
                })
            })
    }

    fn item_has_chaos_protection(&self, item: &ItemInstance) -> bool {
        let Some(definition) = self.content.item(&item.kind_id) else {
            return true;
        };
        definition.resistances.contains_key(&ActorDamageType::Chaos)
            || item.affix_ids.iter().any(|affix_id| {
                self.content
                    .affix(affix_id)
                    .is_some_and(|affix| affix.resistances.contains_key(&ActorDamageType::Chaos))
            })
            || item.rolled_affixes.iter().any(|rolled| {
                self.content
                    .affix(&rolled.affix_id)
                    .is_some_and(|affix| affix.resistances.contains_key(&ActorDamageType::Chaos))
            })
    }

    fn item_is_projectile_destruction_protected(&self, item: &ItemInstance) -> bool {
        let Some(definition) = self.content.item(&item.kind_id) else {
            return true;
        };
        definition.resists_projection_destruction
            || item.affix_ids.iter().any(|affix_id| {
                self.content
                    .affix(affix_id)
                    .is_some_and(|affix| affix.resists_projection_destruction)
            })
            || item.rolled_affixes.iter().any(|rolled| {
                self.content
                    .affix(&rolled.affix_id)
                    .is_some_and(|affix| affix.resists_projection_destruction)
            })
    }

    fn element_destroys_item(
        &self,
        item: &ItemInstance,
        element: ItemDestructionElement,
        respect_immunity: bool,
    ) -> bool {
        self.item_has_elemental_destruction_vulnerability(item, element)
            && (!respect_immunity || !self.item_has_elemental_destruction_immunity(item, element))
    }

    fn projectile_destroys_ground_item(
        &self,
        item: &ItemInstance,
        damage_type: DamageType,
    ) -> bool {
        if self.item_is_projectile_destruction_protected(item) {
            return false;
        }
        match damage_type {
            DamageType::Acid => {
                self.element_destroys_item(item, ItemDestructionElement::Acid, true)
            }
            DamageType::Electricity => {
                self.element_destroys_item(item, ItemDestructionElement::Electricity, true)
            }
            DamageType::Fire => {
                self.element_destroys_item(item, ItemDestructionElement::Fire, true)
            }
            DamageType::Cold => {
                self.element_destroys_item(item, ItemDestructionElement::Cold, true)
            }
            DamageType::Plasma => {
                if self.item_has_elemental_destruction_vulnerability(
                    item,
                    ItemDestructionElement::Electricity,
                ) {
                    self.element_destroys_item(item, ItemDestructionElement::Electricity, true)
                } else {
                    self.element_destroys_item(item, ItemDestructionElement::Fire, true)
                }
            }
            DamageType::Meteor => {
                self.element_destroys_item(item, ItemDestructionElement::Fire, true)
                    || self.element_destroys_item(item, ItemDestructionElement::Cold, true)
            }
            DamageType::Ice | DamageType::Shards | DamageType::Sound | DamageType::Force => {
                self.element_destroys_item(item, ItemDestructionElement::Cold, false)
            }
            DamageType::Mana | DamageType::Disintegrate => true,
            DamageType::Chaos => !self.item_has_chaos_protection(item),
            DamageType::HolyFire | DamageType::HellFire => item.curse.is_some(),
            _ => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_ground_item_projectile_effects(
        &mut self,
        ability_id: &str,
        affected_positions: &[Position],
        damage_type: DamageType,
        award_player_kills: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let affected = affected_positions.iter().copied().collect::<BTreeSet<_>>();
        let mut candidates = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position) if affected.contains(&position) => Some((
                    position.y,
                    position.x,
                    item.id.clone(),
                    item.kind_id.clone(),
                    item.quantity,
                    position,
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        candidates.sort_unstable();

        for (_, _, item_id, target_kind_id, quantity, position) in candidates {
            let Some(index) = self.items.iter().position(|item| item.id == item_id) else {
                continue;
            };
            if !self.projectile_destroys_ground_item(&self.items[index], damage_type) {
                continue;
            }
            let shatter = self
                .content
                .item(&target_kind_id)
                .and_then(|definition| definition.shatter_effect.clone());
            self.force_open_capture_ball(&item_id, position, false, events, changed);
            let Some(index) = self.items.iter().position(|item| item.id == item_id) else {
                continue;
            };
            self.items.remove(index);
            self.item_property_knowledge.remove(&item_id);
            changed.insert(position);
            events.push(DomainEvent::GroundItemDestroyedByAbility {
                ability_id: ability_id.to_owned(),
                item_id,
                target_kind_id: target_kind_id.clone(),
                quantity,
                position,
            });
            if let Some(shatter) = shatter {
                self.resolve_ground_item_shatter_effect(
                    &target_kind_id,
                    position,
                    &shatter,
                    award_player_kills,
                    events,
                    changed,
                    removed_entities,
                );
            }
        }
    }
}

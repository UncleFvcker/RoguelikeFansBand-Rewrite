// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(in crate::game) struct ActorDerivedStats {
    pub(in crate::game) max_hp: DerivedStat,
    pub(in crate::game) attack: DerivedStat,
    pub(in crate::game) defense: DerivedStat,
    pub(in crate::game) speed: DerivedStat,
    pub(in crate::game) melee_skill: DerivedStat,
    pub(in crate::game) armor_class: DerivedStat,
    pub(in crate::game) melee_attacks: DerivedStat,
    pub(in crate::game) melee_damage_bonus: DerivedStat,
    pub(in crate::game) ranged_skill: DerivedStat,
    pub(in crate::game) throwing_skill: DerivedStat,
    pub(in crate::game) door_skill: DerivedStat,
    pub(in crate::game) bash_power: DerivedStat,
    pub(in crate::game) search_skill: DerivedStat,
    pub(in crate::game) device_skill: DerivedStat,
    pub(in crate::game) saving_throw_skill: DerivedStat,
    pub(in crate::game) stealth_skill: DerivedStat,
    pub(in crate::game) perception_skill: DerivedStat,
    pub(in crate::game) disarm_skill: DerivedStat,
    pub(in crate::game) dig_skill: DerivedStat,
}

#[derive(Clone)]
pub(in crate::game) struct ResolvedAttackProfile {
    pub(in crate::game) attacks: u16,
    pub(in crate::game) melee_skill: DerivedStat,
    pub(in crate::game) to_hit: i32,
    pub(in crate::game) to_damage: i32,
    pub(in crate::game) damage_dice: u16,
    pub(in crate::game) damage_sides: u16,
    pub(in crate::game) damage_type: DamageType,
    pub(in crate::game) source_item_id: Option<String>,
    pub(in crate::game) source_mutation_id: Option<String>,
    pub(in crate::game) attack_name: Option<String>,
    pub(in crate::game) critical_weight_tenths_pound: Option<u16>,
}

pub(in crate::game) struct ResolvedMeleeBlow {
    pub(in crate::game) method_id: Option<String>,
    pub(in crate::game) to_hit: i32,
    pub(in crate::game) self_destructs: bool,
    pub(in crate::game) effects: Vec<MeleeBlowEffectDefinition>,
}

pub(in crate::game) struct ResolvedProjectileProfile {
    pub(in crate::game) range: u16,
    pub(in crate::game) to_hit: i32,
    pub(in crate::game) to_damage: i32,
    pub(in crate::game) ammunition_to_hit: i32,
    pub(in crate::game) ammunition_to_damage: i32,
    pub(in crate::game) launcher_to_damage: i32,
    pub(in crate::game) damage_multiplier_percent: u16,
    pub(in crate::game) damage_dice: u16,
    pub(in crate::game) damage_sides: u16,
    pub(in crate::game) damage_type: DamageType,
    pub(in crate::game) ammo_item_id: Option<String>,
    pub(in crate::game) ammo_kind_id: String,
    pub(in crate::game) ammo_break_chance_percent: u8,
    pub(in crate::game) source_item_id: String,
}

#[derive(Clone)]
pub(in crate::game) struct ResolvedThrowProfile {
    pub(in crate::game) to_hit: i32,
    pub(in crate::game) to_damage: i32,
    pub(in crate::game) damage_dice: u16,
    pub(in crate::game) damage_sides: u16,
    pub(in crate::game) damage_type: DamageType,
}

impl ResolvedProjectileProfile {
    pub(in crate::game) fn to_dto(&self) -> ProjectileProfileDto {
        ProjectileProfileDto {
            range: self.range,
            to_hit: self.to_hit,
            to_damage: self.to_damage,
            damage: DamageDiceDto {
                dice: self.damage_dice,
                sides: self.damage_sides,
                damage_type: self.damage_type.into(),
            },
            ammo_kind_id: self.ammo_kind_id.clone(),
            target_spec: projectile_target_spec(self.range),
            source_item_id: self.source_item_id.clone(),
        }
    }
}

pub(in crate::game) fn resolved_melee_blows(
    definition: &rfb_content::ActorDefinition,
) -> Vec<ResolvedMeleeBlow> {
    definition.melee_routine.as_ref().map_or_else(
        || {
            vec![ResolvedMeleeBlow {
                method_id: None,
                to_hit: 0,
                self_destructs: false,
                effects: vec![MeleeBlowEffectDefinition::Damage {
                    chance_percent: None,
                    damage_dice: definition.damage_dice,
                    damage_sides: definition.damage_sides,
                    damage_type: definition.damage_type,
                    armor_mitigated: true,
                    vampiric: false,
                }],
            }]
        },
        |routine| {
            routine
                .blows
                .iter()
                .map(|blow| ResolvedMeleeBlow {
                    method_id: Some(blow.method_id.clone()),
                    to_hit: blow.to_hit,
                    self_destructs: blow.self_destructs,
                    effects: blow.effects.clone(),
                })
                .collect()
        },
    )
}

pub(in crate::game) fn actor_melee_routine_dto(
    definition: &rfb_content::ActorDefinition,
) -> MeleeRoutineDto {
    MeleeRoutineDto {
        blows: resolved_melee_blows(definition)
            .into_iter()
            .map(|blow| MeleeBlowDto {
                method_id: blow
                    .method_id
                    .unwrap_or_else(|| "rfb.blow.innate".to_owned()),
                to_hit: blow.to_hit,
                damage: projected_blow_damage(&blow.effects),
            })
            .collect(),
    }
}

fn projected_blow_damage(effects: &[MeleeBlowEffectDefinition]) -> DamageDiceDto {
    let (dice, sides, damage_type) = effects
        .iter()
        .find_map(|effect| match effect {
            MeleeBlowEffectDefinition::Damage {
                damage_dice,
                damage_sides,
                damage_type,
                ..
            } => Some((*damage_dice, *damage_sides, DamageType::from(*damage_type))),
            MeleeBlowEffectDefinition::Poison {
                damage_dice,
                damage_sides,
                ..
            } => Some((*damage_dice, *damage_sides, DamageType::Poison)),
            MeleeBlowEffectDefinition::Disease {
                damage_dice,
                damage_sides,
                ..
            } if *damage_dice > 0 && *damage_sides > 0 => {
                Some((*damage_dice, *damage_sides, DamageType::Physical))
            }
            MeleeBlowEffectDefinition::Disease { .. } => None,
            MeleeBlowEffectDefinition::DrainAttributes { .. }
            | MeleeBlowEffectDefinition::DrainResource { .. }
            | MeleeBlowEffectDefinition::DrainExperience { .. }
            | MeleeBlowEffectDefinition::Bleeding { .. }
            | MeleeBlowEffectDefinition::Blind { .. }
            | MeleeBlowEffectDefinition::Paralysis { .. }
            | MeleeBlowEffectDefinition::Slow { .. }
            | MeleeBlowEffectDefinition::Stun { .. }
            | MeleeBlowEffectDefinition::Terrify { .. }
            | MeleeBlowEffectDefinition::Disenchant { .. }
            | MeleeBlowEffectDefinition::EatGold { .. }
            | MeleeBlowEffectDefinition::EatItem { .. }
            | MeleeBlowEffectDefinition::EatFood { .. }
            | MeleeBlowEffectDefinition::EatLight { .. } => None,
            MeleeBlowEffectDefinition::Confusion {
                damage_dice,
                damage_sides,
                ..
            } if *damage_dice > 0 && *damage_sides > 0 => {
                Some((*damage_dice, *damage_sides, DamageType::Confusion))
            }
            MeleeBlowEffectDefinition::Confusion { .. } => None,
        })
        .unwrap_or((1, 1, DamageType::Physical));
    DamageDiceDto {
        dice,
        sides,
        damage_type: damage_type.into(),
    }
}

impl ResolvedAttackProfile {
    pub(in crate::game) fn miss_event(&self, target_kind_id: &str) -> DomainEvent {
        self.source_mutation_id.as_ref().map_or_else(
            || DomainEvent::PlayerMeleeMissed {
                target_kind_id: target_kind_id.to_owned(),
            },
            |mutation_id| DomainEvent::MutationMeleeMissed {
                mutation_id: mutation_id.clone(),
                attack_name: self
                    .attack_name
                    .clone()
                    .expect("mutation attack profile must retain its name"),
                target_kind_id: target_kind_id.to_owned(),
            },
        )
    }

    pub(in crate::game) fn hit_event(
        &self,
        target_kind_id: &str,
        damage: DamageOutcome,
    ) -> DomainEvent {
        self.source_mutation_id.as_ref().map_or_else(
            || DomainEvent::PlayerMeleeHit {
                target_kind_id: target_kind_id.to_owned(),
                damage,
            },
            |mutation_id| DomainEvent::MutationMeleeHit {
                mutation_id: mutation_id.clone(),
                attack_name: self
                    .attack_name
                    .clone()
                    .expect("mutation attack profile must retain its name"),
                target_kind_id: target_kind_id.to_owned(),
                damage,
            },
        )
    }

    pub(in crate::game) fn slew_event(
        &self,
        target_kind_id: &str,
        damage: DamageOutcome,
    ) -> DomainEvent {
        self.source_mutation_id.as_ref().map_or_else(
            || DomainEvent::PlayerSlew {
                target_kind_id: target_kind_id.to_owned(),
                damage,
            },
            |mutation_id| DomainEvent::MutationMeleeSlew {
                mutation_id: mutation_id.clone(),
                attack_name: self
                    .attack_name
                    .clone()
                    .expect("mutation attack profile must retain its name"),
                target_kind_id: target_kind_id.to_owned(),
                damage,
            },
        )
    }

    pub(in crate::game) fn to_dto(&self) -> AttackProfileDto {
        AttackProfileDto {
            attacks: self.attacks,
            to_hit: self.to_hit,
            to_damage: self.to_damage,
            damage: DamageDiceDto {
                dice: self.damage_dice,
                sides: self.damage_sides,
                damage_type: self.damage_type.into(),
            },
            source_item_id: self.source_item_id.clone(),
        }
    }
}

fn add_nonzero_stat(
    pipeline: &mut DerivedStatsPipeline,
    kind: StatKind,
    layer: StatLayer,
    source_id: &str,
    amount: i32,
) {
    if amount != 0 {
        pipeline.add(kind, layer, source_id, amount);
    }
}

fn derived_stat_without_source(
    stat: &DerivedStat,
    source_id: &str,
    non_negative: bool,
) -> DerivedStat {
    let contributions = stat
        .contributions
        .iter()
        .filter(|contribution| contribution.source_id != source_id)
        .cloned()
        .collect::<Vec<_>>();
    let value = contributions.iter().fold(0_i32, |total, contribution| {
        total.saturating_add(contribution.amount)
    });
    DerivedStat {
        kind: stat.kind,
        value: if non_negative { value.max(0) } else { value },
        contributions,
    }
}

fn add_equipment_stat(
    pipeline: &mut DerivedStatsPipeline,
    kind: StatKind,
    source_id: &str,
    amount: i32,
) {
    if amount != 0 {
        pipeline.add(kind, StatLayer::Equipment, source_id, amount);
    }
}

pub(in crate::game) fn derived_speed(speed: &DerivedStat) -> u16 {
    u16::try_from(speed.value).expect("derived actor speed must fit u16")
}

impl Game {
    pub(super) fn item_base_modifiers(&self, kind_id: &str) -> StatModifiersDto {
        self.content
            .item(kind_id)
            .map_or_else(StatModifiersDto::default, |definition| StatModifiersDto {
                attack: definition.modifiers.attack,
                defense: definition.modifiers.defense,
                max_hp: definition.modifiers.max_hp,
                strength: definition.modifiers.strength,
                intelligence: definition.modifiers.intelligence,
                wisdom: definition.modifiers.wisdom,
                dexterity: definition.modifiers.dexterity,
                constitution: definition.modifiers.constitution,
                charisma: definition.modifiers.charisma,
                speed: definition.modifiers.speed,
            })
    }

    /// Combines resistance tiers from every defensive source the player
    /// carries: the actor's own profile, the build's race, and each equipped
    /// item plus its affixes. Deterministic merge: immune anywhere wins, then
    /// strong; a resistant source is cancelled back to normal by any
    /// vulnerable source; lone vulnerability stays vulnerable.
    pub(super) fn effective_player_resistances(&self) -> ResistanceProfile {
        let mut sources: BTreeMap<DamageType, (bool, bool, bool, bool)> = BTreeMap::new();
        let mut record = |damage_type: DamageType, level: ResistanceLevel| {
            let entry = sources.entry(damage_type).or_default();
            match level {
                ResistanceLevel::Immune => entry.0 = true,
                ResistanceLevel::Strong => entry.1 = true,
                ResistanceLevel::Resistant => entry.2 = true,
                ResistanceLevel::Vulnerable => entry.3 = true,
                ResistanceLevel::Normal => {}
            }
        };
        for (damage_type, level) in self.player.resistances.iter() {
            record(damage_type, level);
        }
        for status in &self.player.statuses {
            for (damage_type, level) in &status.granted_resistances {
                record(*damage_type, *level);
            }
        }
        if let Some((_, race, _, _)) = self.character_definitions() {
            for (damage_type, level) in &race.resistances {
                record(
                    DamageType::from(*damage_type),
                    ResistanceLevel::from(*level),
                );
            }
        }
        for mutation in self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
        {
            for (damage_type, level) in &mutation.resistances {
                record(
                    DamageType::from(*damage_type),
                    ResistanceLevel::from(*level),
                );
            }
        }
        for item in &self.items {
            if !matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            {
                continue;
            }
            if let Some(definition) = self.content.item(&item.kind_id) {
                for (damage_type, level) in &definition.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
            for affix_id in &item.affix_ids {
                if let Some(affix) = self.content.affix(affix_id) {
                    for (damage_type, level) in &affix.resistances {
                        record(
                            DamageType::from(*damage_type),
                            ResistanceLevel::from(*level),
                        );
                    }
                }
            }
            for rolled in &item.rolled_affixes {
                for (damage_type, level) in &rolled.properties.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
        }
        let mut profile = ResistanceProfile::default();
        for (damage_type, (immune, strong, resistant, vulnerable)) in sources {
            let level = if immune {
                ResistanceLevel::Immune
            } else if strong {
                ResistanceLevel::Strong
            } else if resistant {
                if vulnerable {
                    ResistanceLevel::Normal
                } else {
                    ResistanceLevel::Resistant
                }
            } else if vulnerable {
                ResistanceLevel::Vulnerable
            } else {
                ResistanceLevel::Normal
            };
            profile.set(damage_type, level);
        }
        profile
    }

    pub(super) fn player_can_pass_walls(&self) -> bool {
        self.player
            .statuses
            .iter()
            .any(|status| status.grants_wall_passage)
    }

    pub(super) fn player_incoming_damage_percent(&self) -> u8 {
        self.player
            .statuses
            .iter()
            .map(|status| status.incoming_damage_percent)
            .min()
            .unwrap_or(100)
    }

    pub(super) fn reduce_player_damage(&self, damage: DamageOutcome) -> DamageOutcome {
        scale_damage_outcome(damage, self.player_incoming_damage_percent())
    }

    /// Status kinds the player cannot receive: the union of the race's
    /// innate immunities and every equipped item's (plus affixes').
    pub(super) fn player_status_immunities(&self) -> BTreeSet<String> {
        let mut immunities = BTreeSet::new();
        for status in &self.player.statuses {
            immunities.extend(status.granted_status_immunities.iter().cloned());
        }
        if let Some((_, race, _, _)) = self.character_definitions() {
            immunities.extend(race.status_immunities.iter().cloned());
        }
        for mutation in self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
        {
            immunities.extend(mutation.status_immunities.iter().cloned());
        }
        for item in &self.items {
            if !matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            {
                continue;
            }
            if let Some(definition) = self.content.item(&item.kind_id) {
                immunities.extend(definition.status_immunities.iter().cloned());
            }
            for affix_id in &item.affix_ids {
                if let Some(affix) = self.content.affix(affix_id) {
                    immunities.extend(affix.status_immunities.iter().cloned());
                }
            }
            for rolled in &item.rolled_affixes {
                immunities.extend(rolled.properties.status_immunities.iter().cloned());
            }
        }
        immunities
    }

    fn item_modifiers(&self, item: &ItemInstance) -> StatModifiersDto {
        let mut modifiers = item.affix_ids.iter().fold(
            self.item_base_modifiers(&item.kind_id),
            |total, affix_id| {
                let affix = self
                    .content
                    .affix(affix_id)
                    .expect("item affix must remain available");
                StatModifiersDto {
                    attack: total.attack.saturating_add(affix.modifiers.attack),
                    defense: total.defense.saturating_add(affix.modifiers.defense),
                    max_hp: total.max_hp.saturating_add(affix.modifiers.max_hp),
                    strength: total.strength.saturating_add(affix.modifiers.strength),
                    intelligence: total
                        .intelligence
                        .saturating_add(affix.modifiers.intelligence),
                    wisdom: total.wisdom.saturating_add(affix.modifiers.wisdom),
                    dexterity: total.dexterity.saturating_add(affix.modifiers.dexterity),
                    constitution: total
                        .constitution
                        .saturating_add(affix.modifiers.constitution),
                    charisma: total.charisma.saturating_add(affix.modifiers.charisma),
                    speed: total.speed.saturating_add(affix.modifiers.speed),
                }
            },
        );
        for rolled in &item.rolled_affixes {
            add_stat_modifiers_dto(&mut modifiers, &rolled.properties.modifiers);
        }
        modifiers.defense = modifiers
            .defense
            .saturating_add(i32::from(item.enchantments.to_armor));
        modifiers
    }

    fn item_equipment_bonuses(&self, item: &ItemInstance) -> EquipmentBonuses {
        let mut bonuses = self
            .content
            .item(&item.kind_id)
            .map_or_else(EquipmentBonuses::default, |definition| {
                definition.equipment_bonuses.clone()
            });
        for affix_id in &item.affix_ids {
            if let Some(affix) = self.content.affix(affix_id) {
                merge_equipment_bonuses(&mut bonuses, &affix.equipment_bonuses);
            }
        }
        for rolled in &item.rolled_affixes {
            merge_equipment_bonuses(&mut bonuses, &rolled.properties.equipment_bonuses);
        }
        bonuses
    }

    pub(super) fn item_passives(&self, item: &ItemInstance) -> BTreeSet<EquipmentPassive> {
        let mut passives = self
            .content
            .item(&item.kind_id)
            .map_or_else(BTreeSet::new, |definition| definition.passives.clone());
        for affix_id in &item.affix_ids {
            if let Some(affix) = self.content.affix(affix_id) {
                passives.extend(&affix.passives);
            }
        }
        for rolled in &item.rolled_affixes {
            passives.extend(&rolled.properties.passives);
        }
        passives
    }

    pub(super) fn player_equipment_passives(&self) -> BTreeSet<EquipmentPassive> {
        self.items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            })
            .flat_map(|item| self.item_passives(item))
            .collect()
    }

    pub(super) fn player_see_invisible_sources(&self) -> usize {
        let equipment_sources = self.items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
                    && self
                        .item_passives(item)
                        .contains(&EquipmentPassive::SeeInvisible)
            })
            .count();
        equipment_sources
            + usize::from(
                self.player
                    .statuses
                    .iter()
                    .any(|status| status.kind_id == STATUS_SIGHT),
            )
    }

    pub(super) fn player_infravision_range(&self) -> i32 {
        let equipment = self
            .items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            })
            .fold(0_i32, |total, item| {
                total.saturating_add(self.item_equipment_bonuses(item).infravision)
            });
        let mutations = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0_i32, |total, mutation| {
                total.saturating_add(mutation.infravision)
            });
        self.player
            .statuses
            .iter()
            .fold(equipment.saturating_add(mutations), |total, status| {
                total.saturating_add(status.granted_equipment_bonuses.infravision)
            })
            .max(0)
    }

    pub(super) fn player_levitates(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.levitation && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }

    pub(super) fn player_has_telepathy(&self) -> bool {
        self.player_has_status_kind(STATUS_TELEPATHY)
            || self.content.mutations().any(|mutation| {
                mutation.telepathy && self.progress.active_mutation_ids.contains(&mutation.id)
            })
    }

    pub(super) fn player_regeneration_rate_percent(&self) -> u64 {
        let modifier = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0_i32, |total, mutation| {
                total.saturating_add(mutation.regeneration_rate_modifier_percent)
            });
        u64::try_from(100_i32.saturating_add(modifier).max(0))
            .expect("non-negative regeneration rate must fit u64")
    }

    pub(super) fn player_mutation_light_radius(&self) -> i32 {
        self.content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .map(|mutation| mutation.light_radius)
            .max()
            .unwrap_or_default()
            .max(0)
    }

    pub(super) fn equipment_modifiers(&self) -> StatModifiersDto {
        self.items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            })
            .fold(StatModifiersDto::default(), |total, item| {
                let item = self.item_modifiers(item);
                StatModifiersDto {
                    attack: total.attack.saturating_add(item.attack),
                    defense: total.defense.saturating_add(item.defense),
                    max_hp: total.max_hp.saturating_add(item.max_hp),
                    strength: total.strength.saturating_add(item.strength),
                    intelligence: total.intelligence.saturating_add(item.intelligence),
                    wisdom: total.wisdom.saturating_add(item.wisdom),
                    dexterity: total.dexterity.saturating_add(item.dexterity),
                    constitution: total.constitution.saturating_add(item.constitution),
                    charisma: total.charisma.saturating_add(item.charisma),
                    speed: total.speed.saturating_add(item.speed),
                }
            })
    }

    pub(super) fn victory_level_cap_unlocked(&self) -> bool {
        self.campaign_state.status != CampaignStatusDto::Active
    }

    pub(super) fn effective_player_max_hp(&self) -> i32 {
        self.player_derived_stats().max_hp.value
    }

    pub(super) fn player_derived_stats(&self) -> ActorDerivedStats {
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        self.actor_derived_stats(&self.player, definition, true)
    }

    pub(super) fn item_melee_profile(&self, item: &ItemInstance) -> Option<AttackProfileDto> {
        self.content
            .item(&item.kind_id)
            .and_then(|definition| definition.melee_profile.as_ref())
            .map(|profile| AttackProfileDto {
                attacks: profile.attacks,
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                source_item_id: Some(item.id.clone()),
            })
    }

    pub(super) fn item_projectile_profile(
        &self,
        item: &ItemInstance,
    ) -> Option<ProjectileProfileDto> {
        let profile = self
            .content
            .item(&item.kind_id)?
            .projectile_profile
            .as_ref()?;
        let ammunition = self.content.item_definitions().find(|definition| {
            definition
                .ammunition_profile
                .as_ref()
                .is_some_and(|ammo| ammo.ammunition_type == profile.ammunition_type)
        })?;
        let ammo = ammunition.ammunition_profile.as_ref()?;
        Some(ProjectileProfileDto {
            range: profile.range,
            to_hit: profile
                .to_hit
                .saturating_add(i32::from(item.enchantments.to_hit))
                .saturating_add(ammo.to_hit),
            to_damage: ammo
                .to_damage
                .saturating_mul(i32::from(profile.damage_multiplier_percent))
                / 100
                + profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
            damage: DamageDiceDto {
                dice: ammo.damage_dice,
                sides: ammo.damage_sides,
                damage_type: DamageType::from(ammo.damage_type).into(),
            },
            ammo_kind_id: ammunition.id.clone(),
            target_spec: projectile_target_spec(profile.range),
            source_item_id: item.id.clone(),
        })
    }

    pub(super) fn item_weight_tenths_pound(&self, kind_id: &str) -> u16 {
        self.content
            .item(kind_id)
            .map_or(0, |definition| definition.weight_tenths_pound)
    }

    pub(super) fn carried_weight_tenths_pound(&self) -> u32 {
        self.items
            .iter()
            .filter(|item| {
                matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                )
            })
            .fold(0_u32, |total, item| {
                total.saturating_add(
                    u32::from(self.item_weight_tenths_pound(&item.kind_id))
                        .saturating_mul(item.quantity),
                )
            })
    }

    pub(super) fn player_carry_capacity_tenths_pound(&self) -> u32 {
        crate::stats::carry_capacity_tenths_pound(self.effective_player_attributes().strength)
    }

    pub(super) fn player_encumbrance_speed_penalty(&self) -> i32 {
        crate::stats::encumbrance_speed_penalty(
            self.carried_weight_tenths_pound(),
            self.player_carry_capacity_tenths_pound(),
        )
    }

    pub(super) fn item_throw_profile(&self, item: &ItemInstance) -> Option<ThrowProfileDto> {
        let definition = self.content.item(&item.kind_id)?;
        definition
            .throw_profile
            .as_ref()
            .map(|profile| ThrowProfileDto {
                range: throw_range(
                    definition.weight_tenths_pound,
                    self.player_has_mighty_throw(),
                ),
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage: DamageDiceDto {
                    dice: profile.damage_dice,
                    sides: profile.damage_sides,
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                source_item_id: item.id.clone(),
            })
    }

    pub(super) fn body_slot_type(&self, slot_id: &str) -> Option<&str> {
        self.body_slots
            .iter()
            .find(|slot| slot.id == slot_id)
            .map(|slot| slot.slot_type.as_str())
    }

    pub(super) fn player_projectile_profile(&self) -> Option<ResolvedProjectileProfile> {
        self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if self.body_slot_type(slot_id) != Some("launcher") {
                return None;
            }
            self.content
                .item(&item.kind_id)?
                .projectile_profile
                .as_ref()
                .and_then(|profile| {
                    let ammunition = self
                        .items
                        .iter()
                        .filter(|ammunition| {
                            ammunition.location == ItemLocation::Inventory
                                && ammunition.quantity > 0
                                && self
                                    .content
                                    .item(&ammunition.kind_id)
                                    .and_then(|definition| definition.ammunition_profile.as_ref())
                                    .is_some_and(|ammo| {
                                        ammo.ammunition_type == profile.ammunition_type
                                    })
                        })
                        .min_by(|left, right| left.id.cmp(&right.id));
                    let ammo_definition = ammunition
                        .and_then(|item| self.content.item(&item.kind_id))
                        .or_else(|| {
                            self.content.item_definitions().find(|definition| {
                                definition.ammunition_profile.as_ref().is_some_and(|ammo| {
                                    ammo.ammunition_type == profile.ammunition_type
                                })
                            })
                        })?;
                    let ammo_profile = ammo_definition.ammunition_profile.as_ref()?;
                    let ammo_break_chance_percent = ammo_definition.break_chance_percent;
                    let ammunition_to_hit = ammo_profile.to_hit.saturating_add(i32::from(
                        ammunition.map_or(0, |item| item.enchantments.to_hit),
                    ));
                    let ammunition_to_damage = ammo_profile.to_damage.saturating_add(i32::from(
                        ammunition.map_or(0, |item| item.enchantments.to_damage),
                    ));
                    let launcher_to_damage = profile
                        .to_damage
                        .saturating_add(i32::from(item.enchantments.to_damage));
                    Some(ResolvedProjectileProfile {
                        range: profile.range,
                        to_hit: profile
                            .to_hit
                            .saturating_add(i32::from(item.enchantments.to_hit))
                            .saturating_add(ammunition_to_hit),
                        to_damage: ammunition_to_damage
                            .saturating_mul(i32::from(profile.damage_multiplier_percent))
                            / 100
                            + launcher_to_damage,
                        ammunition_to_hit,
                        ammunition_to_damage,
                        launcher_to_damage,
                        damage_multiplier_percent: profile.damage_multiplier_percent,
                        damage_dice: ammo_profile.damage_dice,
                        damage_sides: ammo_profile.damage_sides,
                        damage_type: DamageType::from(ammo_profile.damage_type),
                        ammo_item_id: ammunition.map(|item| item.id.clone()),
                        ammo_kind_id: ammo_definition.id.clone(),
                        ammo_break_chance_percent,
                        source_item_id: item.id.clone(),
                    })
                })
        })
    }

    pub(super) fn player_melee_profile(&self, stats: &ActorDerivedStats) -> ResolvedAttackProfile {
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        let equipped_weapon = self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if self.body_slot_type(slot_id) != Some("weapon") {
                return None;
            }
            self.content
                .item(&item.kind_id)
                .and_then(|definition| definition.melee_profile.as_ref())
                .map(|profile| (item.id.clone(), profile))
        });
        let (source_item_id, dice, sides, damage_type, to_hit) = equipped_weapon.map_or_else(
            || {
                (
                    None,
                    definition.damage_dice,
                    definition.damage_sides,
                    definition.damage_type,
                    0,
                )
            },
            |(item_id, profile)| {
                (
                    Some(item_id),
                    profile.damage_dice,
                    profile.damage_sides,
                    profile.damage_type,
                    profile.to_hit,
                )
            },
        );
        ResolvedAttackProfile {
            attacks: u16::try_from(stats.melee_attacks.value)
                .expect("derived melee attack count must fit u16"),
            melee_skill: stats.melee_skill.clone(),
            to_hit,
            to_damage: stats.melee_damage_bonus.value,
            damage_dice: dice,
            damage_sides: sides,
            damage_type: DamageType::from(damage_type),
            source_item_id,
            source_mutation_id: None,
            attack_name: None,
            critical_weight_tenths_pound: None,
        }
    }

    pub(super) fn player_mutation_innate_attack_profiles(
        &self,
        stats: &ActorDerivedStats,
        equipped_weapon_id: Option<&str>,
    ) -> Vec<ResolvedAttackProfile> {
        let innate_skill = equipped_weapon_id.map_or_else(
            || stats.melee_skill.clone(),
            |item_id| derived_stat_without_source(&stats.melee_skill, item_id, true),
        );
        let innate_damage_bonus = equipped_weapon_id.map_or(stats.melee_damage_bonus.value, |id| {
            derived_stat_without_source(&stats.melee_damage_bonus, id, false).value
        });
        let mut mutations = self
            .content
            .mutations()
            .filter(|mutation| {
                mutation.innate_attack.is_some()
                    && self.progress.active_mutation_ids.contains(&mutation.id)
            })
            .collect::<Vec<_>>();
        mutations.sort_by_key(|mutation| mutation.source_index);
        mutations
            .into_iter()
            .map(|mutation| {
                let attack = mutation
                    .innate_attack
                    .as_ref()
                    .expect("filtered mutation must retain its innate attack");
                let melee_skill = if attack.to_hit == 0 {
                    innate_skill.clone()
                } else {
                    innate_skill.with_modifier(
                        StatLayer::Status,
                        &mutation.id,
                        attack.to_hit,
                        StatBounds::NON_NEGATIVE,
                    )
                };
                let critical_to_hit = melee_skill
                    .contributions
                    .iter()
                    .filter(|contribution| {
                        matches!(
                            contribution.layer,
                            StatLayer::Equipment
                                | StatLayer::Status
                                | StatLayer::Stance
                                | StatLayer::Environment
                        )
                    })
                    .fold(0_i32, |total, contribution| {
                        total.saturating_add(contribution.amount)
                    });
                ResolvedAttackProfile {
                    attacks: 1,
                    melee_skill,
                    to_hit: critical_to_hit,
                    to_damage: innate_damage_bonus.saturating_add(attack.to_damage),
                    damage_dice: attack.damage_dice,
                    damage_sides: attack.damage_sides,
                    damage_type: DamageType::from(attack.damage_type),
                    source_item_id: None,
                    source_mutation_id: Some(mutation.id.clone()),
                    attack_name: Some(attack.name.clone()),
                    critical_weight_tenths_pound: Some(attack.weight_tenths_pound),
                }
            })
            .collect()
    }

    pub(super) fn player_has_mighty_throw(&self) -> bool {
        self.content.mutations().any(|mutation| {
            mutation.mighty_throw && self.progress.active_mutation_ids.contains(&mutation.id)
        })
    }

    pub(super) fn player_melee_damage_multiplier(
        &self,
        profile: &ResolvedAttackProfile,
        target: &Actor,
        definition: &rfb_content::ActorDefinition,
    ) -> i32 {
        if profile.source_item_id.is_none() {
            return 10;
        }
        let mut multiplier = 10;
        let mut apply = |slays: &BTreeMap<SlayTarget, SlayLevel>,
                         brands: &BTreeSet<WeaponBrand>| {
            for (slay_target, level) in slays {
                if slay_target_matches(*slay_target, definition) {
                    multiplier = multiplier.max(slay_multiplier(*slay_target, *level));
                }
            }
            for brand in brands {
                if target.resistances.level(brand_damage_type(*brand)) != ResistanceLevel::Immune {
                    multiplier = multiplier.max(24);
                }
            }
        };
        for item in &self.items {
            if !matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            {
                continue;
            }
            if let Some(item_definition) = self.content.item(&item.kind_id) {
                apply(&item_definition.slays, &item_definition.brands);
            }
            for affix_id in &item.affix_ids {
                if let Some(affix) = self.content.affix(affix_id) {
                    apply(&affix.slays, &affix.brands);
                }
            }
            for rolled in &item.rolled_affixes {
                apply(&rolled.properties.slays, &rolled.properties.brands);
            }
        }
        for status in &self.player.statuses {
            for brand in &status.granted_brands {
                if target.resistances.level(brand_damage_type(*brand)) != ResistanceLevel::Immune {
                    multiplier = multiplier.max(24);
                }
            }
        }
        multiplier
    }

    fn add_character_stat_contributions(&self, pipeline: &mut DerivedStatsPipeline) {
        let Some((_, race, class, personality)) = self.character_definitions() else {
            return;
        };
        for (layer, source_id, modifiers) in [
            (StatLayer::Species, race.id.as_str(), &race.modifiers),
            (StatLayer::Class, class.id.as_str(), &class.modifiers),
            (
                StatLayer::Personality,
                personality.id.as_str(),
                &personality.modifiers,
            ),
        ] {
            add_nonzero_stat(
                pipeline,
                StatKind::MaxHp,
                layer,
                source_id,
                modifiers.max_hp,
            );
            add_nonzero_stat(
                pipeline,
                StatKind::Attack,
                layer,
                source_id,
                modifiers.attack,
            );
            add_nonzero_stat(
                pipeline,
                StatKind::Defense,
                layer,
                source_id,
                modifiers.defense,
            );
        }
    }

    fn add_character_skill_contributions(&self, pipeline: &mut DerivedStatsPipeline) {
        let Some((_, race, class, personality)) = self.character_definitions() else {
            return;
        };
        for (layer, source_id, skill_set_id) in [
            (
                StatLayer::Species,
                race.id.as_str(),
                race.skill_set_id.as_str(),
            ),
            (
                StatLayer::Class,
                class.id.as_str(),
                class.skill_set_id.as_str(),
            ),
            (
                StatLayer::Personality,
                personality.id.as_str(),
                personality.skill_set_id.as_str(),
            ),
        ] {
            let skill_set = self
                .content
                .skill_set(skill_set_id)
                .expect("validated skill set must remain available");
            for entry in &skill_set.entries {
                let definition = self
                    .content
                    .skill(&entry.skill_id)
                    .expect("validated skill must remain available");
                let amount = entry.base.saturating_add(
                    entry
                        .growth_per_ten_levels
                        .saturating_mul(i32::from(self.progress.level))
                        .saturating_div(10),
                );
                match definition.kind {
                    SkillKind::Disarming => {
                        add_nonzero_stat(pipeline, StatKind::DoorSkill, layer, source_id, amount);
                        add_nonzero_stat(pipeline, StatKind::DisarmSkill, layer, source_id, amount);
                    }
                    SkillKind::Search => {
                        add_nonzero_stat(pipeline, StatKind::SearchSkill, layer, source_id, amount)
                    }
                    SkillKind::Melee => {
                        add_nonzero_stat(pipeline, StatKind::MeleeSkill, layer, source_id, amount)
                    }
                    SkillKind::Ranged => {
                        add_nonzero_stat(pipeline, StatKind::RangedSkill, layer, source_id, amount)
                    }
                    SkillKind::Throwing => add_nonzero_stat(
                        pipeline,
                        StatKind::ThrowingSkill,
                        layer,
                        source_id,
                        amount,
                    ),
                    SkillKind::Digging => {
                        add_nonzero_stat(pipeline, StatKind::DigSkill, layer, source_id, amount)
                    }
                    SkillKind::Device => {
                        add_nonzero_stat(pipeline, StatKind::DeviceSkill, layer, source_id, amount)
                    }
                    SkillKind::SavingThrow => add_nonzero_stat(
                        pipeline,
                        StatKind::SavingThrowSkill,
                        layer,
                        source_id,
                        amount,
                    ),
                    SkillKind::Stealth => {
                        add_nonzero_stat(pipeline, StatKind::StealthSkill, layer, source_id, amount)
                    }
                    SkillKind::Perception => add_nonzero_stat(
                        pipeline,
                        StatKind::PerceptionSkill,
                        layer,
                        source_id,
                        amount,
                    ),
                }
            }
        }
    }

    pub(super) fn actor_derived_stats(
        &self,
        actor: &Actor,
        definition: &rfb_content::ActorDefinition,
        include_equipment: bool,
    ) -> ActorDerivedStats {
        let definition = self.actor_runtime_definition(actor).unwrap_or(definition);
        let mut pipeline = DerivedStatsPipeline::new();
        let base_source = definition.id.as_str();
        pipeline.add(
            StatKind::MaxHp,
            StatLayer::Base,
            base_source,
            if include_equipment {
                self.character_base_max_hp_at_level(self.progress.level)
            } else {
                actor.max_hp
            },
        );
        pipeline.add(
            StatKind::Attack,
            StatLayer::Base,
            base_source,
            definition.attack,
        );
        pipeline.add(
            StatKind::Defense,
            StatLayer::Base,
            base_source,
            definition.defense,
        );
        pipeline.add(
            StatKind::Speed,
            StatLayer::Base,
            base_source,
            i32::from(actor.speed),
        );
        if include_equipment
            && let Some(mount_id) = self.riding_actor_id.as_deref()
            && let Some(mount) = self.entities.iter().find(|entity| entity.id == mount_id)
        {
            pipeline.add(
                StatKind::Speed,
                StatLayer::Environment,
                &mount.id,
                i32::from(mount.speed).saturating_sub(i32::from(actor.speed)),
            );
        }
        pipeline.add(
            StatKind::MeleeSkill,
            StatLayer::Base,
            base_source,
            if definition.role == ActorRole::Monster {
                monster_melee_skill(definition.attack, definition.level)
            } else if include_equipment && self.build.is_some() {
                0
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::ArmorClass,
            StatLayer::Base,
            base_source,
            rating_to_armor_class(definition.defense),
        );
        pipeline.add(StatKind::MeleeAttacks, StatLayer::Base, base_source, 1);
        pipeline.add(StatKind::MeleeDamageBonus, StatLayer::Base, base_source, 0);
        pipeline.add(
            StatKind::RangedSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::ThrowingSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::DoorSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.door_skill
            },
        );
        pipeline.add(
            StatKind::BashPower,
            StatLayer::Base,
            base_source,
            definition.bash_power,
        );
        pipeline.add(
            StatKind::SearchSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.search_skill
            },
        );
        pipeline.add(StatKind::DeviceSkill, StatLayer::Base, base_source, 0);
        pipeline.add(StatKind::SavingThrowSkill, StatLayer::Base, base_source, 0);
        pipeline.add(StatKind::StealthSkill, StatLayer::Base, base_source, 0);
        pipeline.add(StatKind::PerceptionSkill, StatLayer::Base, base_source, 0);
        pipeline.add(
            StatKind::DisarmSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.disarm_skill
            },
        );
        pipeline.add(
            StatKind::DigSkill,
            StatLayer::Base,
            base_source,
            if include_equipment && self.build.is_some() {
                0
            } else {
                definition.dig_skill
            },
        );

        if include_equipment {
            self.add_character_stat_contributions(&mut pipeline);
            self.add_character_skill_contributions(&mut pipeline);
            for mutation in self
                .content
                .mutations()
                .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            {
                let modifiers = &mutation.modifiers;
                for (kind, value) in [
                    (StatKind::MaxHp, modifiers.max_hp),
                    (StatKind::Attack, modifiers.attack),
                    (StatKind::Defense, modifiers.defense),
                    (
                        StatKind::MeleeSkill,
                        rating_to_combat_value(modifiers.attack),
                    ),
                    (
                        StatKind::ArmorClass,
                        rating_to_armor_class(modifiers.defense)
                            .saturating_add(mutation.armor_class),
                    ),
                    (StatKind::Speed, modifiers.speed),
                    (
                        StatKind::SavingThrowSkill,
                        mutation.saving_throw_skill.saturating_add(
                            mutation
                                .saving_throw_skill_per_five_levels
                                .saturating_mul(i32::from(self.progress.level / 5)),
                        ),
                    ),
                    (StatKind::StealthSkill, mutation.stealth_skill),
                    (StatKind::SearchSkill, mutation.search_skill),
                    (StatKind::PerceptionSkill, mutation.perception_skill),
                ] {
                    add_nonzero_stat(&mut pipeline, kind, StatLayer::Status, &mutation.id, value);
                }
            }
            for item in self
                .items
                .iter()
                .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            {
                if matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) == Some("tool"))
                {
                    let bonuses = self.item_equipment_bonuses(item);
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::DigSkill,
                        &item.id,
                        bonuses.digging_skill,
                    );
                    continue;
                }
                let modifiers = self.item_modifiers(item);
                add_equipment_stat(&mut pipeline, StatKind::MaxHp, &item.id, modifiers.max_hp);
                add_equipment_stat(&mut pipeline, StatKind::Attack, &item.id, modifiers.attack);
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::Defense,
                    &item.id,
                    modifiers.defense,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeSkill,
                    &item.id,
                    rating_to_combat_value(modifiers.attack),
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::ArmorClass,
                    &item.id,
                    rating_to_armor_class(modifiers.defense),
                );
                add_equipment_stat(&mut pipeline, StatKind::Speed, &item.id, modifiers.speed);
                let bonuses = self.item_equipment_bonuses(item);
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeAttacks,
                    &item.id,
                    bonuses.melee_attacks,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeSkill,
                    &item.id,
                    bonuses.melee_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeDamageBonus,
                    &item.id,
                    bonuses.melee_damage,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::RangedSkill,
                    &item.id,
                    bonuses.ranged_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::ThrowingSkill,
                    &item.id,
                    bonuses.throwing_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::DeviceSkill,
                    &item.id,
                    bonuses.device_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::SavingThrowSkill,
                    &item.id,
                    bonuses.saving_throw_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::StealthSkill,
                    &item.id,
                    bonuses.stealth_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::SearchSkill,
                    &item.id,
                    bonuses.search_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::PerceptionSkill,
                    &item.id,
                    bonuses.perception_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::DisarmSkill,
                    &item.id,
                    bonuses.disarming_skill,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::DigSkill,
                    &item.id,
                    bonuses.digging_skill,
                );
                let melee_profile = match &item.location {
                    ItemLocation::Equipped { slot_id }
                        if self.body_slot_type(slot_id) == Some("weapon") =>
                    {
                        self.content
                            .item(&item.kind_id)
                            .and_then(|definition| definition.melee_profile.as_ref())
                    }
                    _ => None,
                };
                if let Some(profile) = melee_profile {
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeAttacks,
                        &item.id,
                        i32::from(profile.attacks).saturating_sub(1),
                    );
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeSkill,
                        &item.id,
                        profile
                            .to_hit
                            .saturating_add(i32::from(item.enchantments.to_hit)),
                    );
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::MeleeDamageBonus,
                        &item.id,
                        profile
                            .to_damage
                            .saturating_add(i32::from(item.enchantments.to_damage)),
                    );
                }
                if let Some(profile) = self
                    .content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.projectile_profile.as_ref())
                {
                    add_equipment_stat(
                        &mut pipeline,
                        StatKind::RangedSkill,
                        &item.id,
                        profile
                            .to_hit
                            .saturating_add(i32::from(item.enchantments.to_hit)),
                    );
                }
            }

            let encumbrance_penalty = self.player_encumbrance_speed_penalty();
            if encumbrance_penalty > 0 {
                pipeline.add(
                    StatKind::Speed,
                    StatLayer::Environment,
                    "rfb.encumbrance",
                    encumbrance_penalty.saturating_neg(),
                );
            }
            if self.minor_slow > 0 {
                pipeline.add(
                    StatKind::Speed,
                    StatLayer::Status,
                    "rfb.status.minor-slow",
                    -i32::from(self.minor_slow),
                );
            }
        }

        for status in &actor.statuses {
            let modifiers = status.granted_modifiers;
            for (kind, value) in [
                (StatKind::MaxHp, modifiers.max_hp),
                (StatKind::Attack, modifiers.attack),
                (StatKind::Defense, modifiers.defense),
                (StatKind::MeleeSkill, modifiers.attack),
                (StatKind::ArmorClass, modifiers.defense),
                (StatKind::Speed, modifiers.speed),
            ] {
                pipeline.add_with_origin(
                    kind,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    value,
                );
            }
            let bonuses = status.granted_equipment_bonuses;
            for (kind, value) in [
                (StatKind::MeleeAttacks, bonuses.melee_attacks),
                (StatKind::MeleeSkill, bonuses.melee_skill),
                (StatKind::MeleeDamageBonus, bonuses.melee_damage),
                (StatKind::RangedSkill, bonuses.ranged_skill),
                (StatKind::ThrowingSkill, bonuses.throwing_skill),
                (StatKind::DeviceSkill, bonuses.device_skill),
                (StatKind::SavingThrowSkill, bonuses.saving_throw_skill),
                (StatKind::StealthSkill, bonuses.stealth_skill),
                (StatKind::SearchSkill, bonuses.search_skill),
                (StatKind::PerceptionSkill, bonuses.perception_skill),
                (StatKind::DisarmSkill, bonuses.disarming_skill),
                (StatKind::DigSkill, bonuses.digging_skill),
            ] {
                pipeline.add_with_origin(
                    kind,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    value,
                );
            }
            let amount = i32::from(status.intensity).saturating_mul(10);
            if status.kind_id == STATUS_HASTE {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount,
                );
            } else if status.kind_id == STATUS_SLOW {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount.saturating_neg(),
                );
            }
            if status.kind_id == STATUS_STUN {
                pipeline.add_with_origin(
                    StatKind::MeleeSkill,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    i32::from(status.intensity)
                        .saturating_mul(10)
                        .saturating_neg(),
                );
                pipeline.add_with_origin(
                    StatKind::ThrowingSkill,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    i32::from(status.intensity)
                        .saturating_mul(10)
                        .saturating_neg(),
                );
            }
        }

        ActorDerivedStats {
            max_hp: pipeline.resolve(StatKind::MaxHp, StatBounds::UNBOUNDED),
            attack: pipeline.resolve(StatKind::Attack, StatBounds::NON_NEGATIVE),
            defense: pipeline.resolve(StatKind::Defense, StatBounds::NON_NEGATIVE),
            speed: pipeline.resolve(StatKind::Speed, StatBounds::ACTOR_SPEED),
            melee_skill: pipeline.resolve(StatKind::MeleeSkill, StatBounds::NON_NEGATIVE),
            armor_class: pipeline.resolve(StatKind::ArmorClass, StatBounds::NON_NEGATIVE),
            melee_attacks: pipeline.resolve(StatKind::MeleeAttacks, StatBounds::NON_NEGATIVE),
            melee_damage_bonus: pipeline.resolve(StatKind::MeleeDamageBonus, StatBounds::UNBOUNDED),
            ranged_skill: pipeline.resolve(StatKind::RangedSkill, StatBounds::NON_NEGATIVE),
            throwing_skill: pipeline.resolve(StatKind::ThrowingSkill, StatBounds::NON_NEGATIVE),
            door_skill: pipeline.resolve(StatKind::DoorSkill, StatBounds::NON_NEGATIVE),
            bash_power: pipeline.resolve(StatKind::BashPower, StatBounds::NON_NEGATIVE),
            search_skill: pipeline.resolve(StatKind::SearchSkill, StatBounds::NON_NEGATIVE),
            device_skill: pipeline.resolve(StatKind::DeviceSkill, StatBounds::NON_NEGATIVE),
            saving_throw_skill: pipeline
                .resolve(StatKind::SavingThrowSkill, StatBounds::NON_NEGATIVE),
            stealth_skill: pipeline.resolve(StatKind::StealthSkill, StatBounds::NON_NEGATIVE),
            perception_skill: pipeline.resolve(StatKind::PerceptionSkill, StatBounds::NON_NEGATIVE),
            disarm_skill: pipeline.resolve(StatKind::DisarmSkill, StatBounds::NON_NEGATIVE),
            dig_skill: pipeline.resolve(StatKind::DigSkill, StatBounds::NON_NEGATIVE),
        }
    }
}

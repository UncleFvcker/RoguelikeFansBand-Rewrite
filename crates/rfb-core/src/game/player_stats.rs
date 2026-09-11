// SPDX-License-Identifier: MPL-2.0

use super::ability_scaling::prorated_level_value;
use super::*;

// RFB master a0d92b6378: combat.c::_blows_range and tables.c::adj_str_blow.
const RFB_BLOW_RANGES: [(u16, u16); 38] = [
    (0, 200),
    (0, 200),
    (10, 200),
    (20, 210),
    (30, 220),
    (40, 230),
    (50, 240),
    (60, 250),
    (70, 260),
    (80, 270),
    (90, 280),
    (100, 290),
    (110, 300),
    (120, 350),
    (130, 400),
    (140, 450),
    (150, 460),
    (160, 470),
    (170, 480),
    (180, 490),
    (190, 500),
    (200, 520),
    (210, 540),
    (220, 560),
    (230, 580),
    (240, 600),
    (250, 610),
    (260, 620),
    (280, 630),
    (300, 640),
    (320, 650),
    (340, 660),
    (350, 670),
    (360, 680),
    (370, 690),
    (380, 700),
    (390, 725),
    (400, 750),
];
const RFB_STRENGTH_BLOW: [u16; 38] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110,
    120, 130, 140, 150, 160, 170, 180, 190, 200, 210, 220, 230, 240,
];

pub(in crate::game) fn good_priest_weapon_penalty(
    priest_class: bool,
    good_realm: bool,
    weapon_tval: Option<u16>,
    blessed: bool,
) -> bool {
    priest_class && good_realm && matches!(weapon_tval, Some(22 | 23)) && !blessed
}

fn launcher_multiplier(base: u16, delta_percent: i32) -> u16 {
    u16::try_from(
        i32::from(base)
            .saturating_add(delta_percent)
            .clamp(0, i32::from(u16::MAX)),
    )
    .expect("clamped launcher multiplier fits u16")
}

fn launcher_range(multiplier_percent: u16) -> u16 {
    13_u16.saturating_add(multiplier_percent / 80).min(18)
}

fn draconian_innate_blows(attributes: AttributeSet, weight: u16, maximum: u16) -> u16 {
    let strength_index = usize::from(
        attributes
            .index(AttributeKind::Strength)
            .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP),
    );
    let dexterity_index = usize::from(
        attributes
            .index(AttributeKind::Dexterity)
            .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP),
    );
    let (minimum, maximum_for_dexterity) = RFB_BLOW_RANGES[dexterity_index];
    let strength = RFB_STRENGTH_BLOW[strength_index]
        .saturating_mul(55)
        .saturating_div(weight.max(70))
        .min(110);
    minimum
        .saturating_add(
            maximum_for_dexterity
                .saturating_sub(minimum)
                .saturating_mul(strength)
                / 110,
        )
        .max(100)
        .min(maximum)
}

pub(super) fn temporary_sustain_passive(status_kind_id: &str) -> Option<EquipmentPassive> {
    match status_kind_id {
        STATUS_HOLD_LIFE => Some(EquipmentPassive::HoldLife),
        STATUS_SUSTAIN_STRENGTH => Some(EquipmentPassive::SustainStrength),
        STATUS_SUSTAIN_INTELLIGENCE => Some(EquipmentPassive::SustainIntelligence),
        STATUS_SUSTAIN_WISDOM => Some(EquipmentPassive::SustainWisdom),
        STATUS_SUSTAIN_DEXTERITY => Some(EquipmentPassive::SustainDexterity),
        STATUS_SUSTAIN_CONSTITUTION => Some(EquipmentPassive::SustainConstitution),
        STATUS_SUSTAIN_CHARISMA => Some(EquipmentPassive::SustainCharisma),
        _ => None,
    }
}

#[derive(Clone)]
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

fn apply_monster_power(
    stat: DerivedStat,
    actor: &Actor,
    definition: &rfb_content::ActorDefinition,
    bounds: StatBounds,
) -> DerivedStat {
    if definition.role != ActorRole::Monster || actor.power_per_mille == BASE_ACTOR_POWER_PER_MILLE
    {
        return stat;
    }
    let scaled = scale_actor_power(stat.value, actor.power_per_mille);
    stat.with_modifier(
        StatLayer::Status,
        "rfb.monster-power",
        scaled.saturating_sub(stat.value),
        bounds,
    )
}

fn apply_player_life_force(stat: DerivedStat, life_force: i32) -> DerivedStat {
    if life_force >= 1_000 {
        return stat;
    }
    let lost = i32::try_from(
        i64::from(stat.value.max(0))
            .saturating_mul(i64::from(1_000_i32.saturating_sub(life_force)))
            .saturating_div(2_000),
    )
    .unwrap_or(i32::MAX);
    stat.with_modifier(
        StatLayer::Status,
        "rfb.life-force",
        lost.saturating_neg(),
        StatBounds::NON_NEGATIVE,
    )
}

#[derive(Clone)]
pub(in crate::game) struct ResolvedAttackProfile {
    pub(in crate::game) poison_needle: bool,
    pub(in crate::game) attacks: u16,
    pub(in crate::game) extra_attack_chance_percent: u8,
    pub(in crate::game) attack_sources: Vec<rfb_protocol::CharacterStatSourceDto>,
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
    pub(in crate::game) ammunition_to_damage: i32,
    pub(in crate::game) launcher_to_damage: i32,
    pub(in crate::game) damage_multiplier_percent: u16,
    pub(in crate::game) damage_dice: u16,
    pub(in crate::game) damage_sides: u16,
    pub(in crate::game) damage_type: DamageType,
    pub(in crate::game) ammunition_slays: BTreeMap<SlayTarget, SlayLevel>,
    pub(in crate::game) ammunition_brands: BTreeSet<WeaponBrand>,
    pub(in crate::game) ammunition_behavior: Option<AmmunitionBehaviorDefinition>,
    pub(in crate::game) ammunition_endurance: bool,
    pub(in crate::game) ammo_item_id: Option<String>,
    pub(in crate::game) ammo_kind_id: String,
    pub(in crate::game) ammunition_weight_tenths_pound: u16,
    pub(in crate::game) ammunition_type: AmmunitionTypeDefinition,
    pub(in crate::game) ammo_break_chance_percent: u8,
    pub(in crate::game) base_shot: i32,
    pub(in crate::game) energy_cost: i32,
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
    if effects.is_empty() {
        return DamageDiceDto {
            dice: 0,
            sides: 0,
            damage_type: DamageType::Physical.into(),
        };
    }
    let (dice, sides, damage_type) = effects
        .iter()
        .find_map(|effect| match effect {
            MeleeBlowEffectDefinition::Damage {
                damage_dice,
                damage_sides,
                damage_type,
                ..
            } => Some((*damage_dice, *damage_sides, DamageType::from(*damage_type))),
            MeleeBlowEffectDefinition::Shatter {
                damage_dice,
                damage_sides,
                ..
            } => Some((*damage_dice, *damage_sides, DamageType::Physical)),
            MeleeBlowEffectDefinition::Bomb {
                damage_dice,
                damage_sides,
                ..
            } => Some((*damage_dice, *damage_sides, DamageType::Shards)),
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
            | MeleeBlowEffectDefinition::DrainCharges { .. }
            | MeleeBlowEffectDefinition::DrainExperience { .. }
            | MeleeBlowEffectDefinition::Unlife { .. }
            | MeleeBlowEffectDefinition::Bleeding { .. }
            | MeleeBlowEffectDefinition::Blind { .. }
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
            to_hit: if self.poison_needle { 0 } else { self.to_hit },
            to_damage: if self.poison_needle {
                0
            } else {
                self.to_damage
            },
            damage: DamageDiceDto {
                dice: if self.poison_needle {
                    1
                } else {
                    self.damage_dice
                },
                sides: if self.poison_needle {
                    1
                } else {
                    self.damage_sides
                },
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
                spell_power_bonus: definition.modifiers.spell_power_bonus,
                device_power_bonus: definition.modifiers.device_power_bonus,
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
        if let Some((_, race, class, _)) = self.character_definitions() {
            for (damage_type, level) in &race.resistances {
                record(
                    DamageType::from(*damage_type),
                    ResistanceLevel::from(*level),
                );
            }
            for entry in &race.level_resistances {
                if self.progress.level < entry.minimum_level {
                    continue;
                }
                for (damage_type, level) in &entry.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
            for entry in &class.level_resistances {
                if self.progress.level < entry.minimum_level {
                    continue;
                }
                for (damage_type, level) in &entry.resistances {
                    record(
                        DamageType::from(*damage_type),
                        ResistanceLevel::from(*level),
                    );
                }
            }
        }
        for mutation in self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
        {
            let resistances = self
                .birth_race_mutation_override(&mutation.id)
                .and_then(|override_| override_.resistances.as_ref())
                .unwrap_or(&mutation.resistances);
            for (damage_type, level) in resistances {
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
            if item.artifact_name.is_some() {
                let flags = super::item_value::instance::value_object(&self.content, item)
                    .expect("random artifact kind has source values")
                    .flags;
                for (damage_type, token) in super::random_artifact::resistance_elements() {
                    for (prefix, level) in [
                        ("RES", ResistanceLevel::Resistant),
                        ("VULN", ResistanceLevel::Vulnerable),
                        ("IM", ResistanceLevel::Immune),
                    ] {
                        if flags.contains(&format!("{prefix}_{token}")) {
                            record(DamageType::from(damage_type), level);
                        }
                    }
                }
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
            for (damage_type, level) in &item.intrinsic_properties.resistances {
                record(
                    DamageType::from(*damage_type),
                    ResistanceLevel::from(*level),
                );
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
        self.player_has_wall_passage()
            && (self.riding_actor_id.is_none()
                || self.active_traveler_has_mode(rfb_content::ActorMovementMode::PassWall))
    }

    pub(super) fn player_has_wall_passage(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, race, _, _)| race.id == "rfb-legacy.race.spectre")
            || self
                .player
                .statuses
                .iter()
                .any(|status| status.grants_wall_passage)
    }

    pub(super) fn player_reflects_bolts(&self) -> bool {
        self.player_equipment_passives().contains(&EquipmentPassive::ReflectsBolts)
        || self.player_class_passives().contains(&EquipmentPassive::ReflectsBolts)
        || self.character_definitions().is_some_and(|(_, race, _, _)| {
            race.reflects_bolts_minimum_level
                .is_some_and(|minimum_level| self.progress.level >= minimum_level)
        }) || self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE)
            || self.player_has_status_kind(STATUS_MAGIC_ARMOR)
            || self.items.iter().any(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
                && self
                    .content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.reflects_bolts)
            })
    }

    pub(super) fn player_incoming_damage_percent(&self) -> u8 {
        self.player
            .statuses
            .iter()
            .map(|status| status.incoming_damage_percent)
            .min()
            .unwrap_or(100)
    }

    pub(super) fn adjust_player_resistance_percent(
        &self,
        damage_type: DamageType,
        resistance: ResistanceLevel,
    ) -> i32 {
        let percent = resistance.reduction_percent();
        // RFB master a0d92b6378: resist.c::res_pct_aux, after merging resistances.
        // Negative resistance and immunity bypass racial percentage adjustments.
        if !(0..100).contains(&percent) {
            return percent;
        }
        match self
            .character_definitions()
            .map(|(_, race, _, _)| race.id.as_str())
        {
            Some("rfb-legacy.race.tonberry") if damage_type == DamageType::Confusion => {
                (percent + 1) / 2
            }
            Some("rfb-legacy.race.ent") if damage_type == DamageType::Fire => percent * 7 / 10,
            _ if self.player_is_vampire() && damage_type == DamageType::Light => (percent + 1) / 2,
            _ => percent,
        }
    }

    pub(super) fn player_is_vampire(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, race, _, _)| race.tags.iter().any(|tag| tag == "vampire"))
    }

    pub(super) fn player_resistance_percent(&self, damage_type: DamageType) -> i32 {
        self.adjust_player_resistance_percent(
            damage_type,
            self.effective_player_resistances().level(damage_type),
        )
    }

    pub(super) fn resist_player_damage(&self, mut damage: DamageOutcome) -> DamageOutcome {
        let percent = self.adjust_player_resistance_percent(damage.damage_type, damage.resistance);
        if percent != damage.resistance.reduction_percent() {
            // Re-evaluate resistance on the post-armor amount before incoming-damage modifiers.
            damage = crate::effect::resolve_damage_with_resistance_percent(
                DamagePacket::after_armor(damage.raw, damage.requested, damage.damage_type),
                damage.resistance,
                percent,
            );
        }
        damage
    }

    pub(super) fn reduce_player_damage(&self, damage: DamageOutcome) -> DamageOutcome {
        scale_damage_outcome(
            self.resist_player_damage(damage),
            self.player_incoming_damage_percent(),
        )
    }

    /// Status kinds the player cannot receive: the union of the race's
    /// innate immunities and every equipped item's (plus affixes').
    pub(super) fn player_status_immunities(&self) -> BTreeSet<String> {
        let mut immunities = BTreeSet::new();
        if self.player_is_berserker() {
            immunities.extend([STATUS_FEAR.to_owned(), STATUS_PARALYSIS.to_owned()]);
            if self.progress.level >= 35 {
                immunities.insert(STATUS_STUN.to_owned());
            }
        }
        // effects.c::set_cut/set_unwell reject these conditions for the current nonliving body.
        if self.player_is_nonliving() {
            immunities.extend([STATUS_BLEEDING.to_owned(), STATUS_UNWELL.to_owned()]);
        }
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
            immunities.extend(item.intrinsic_properties.status_immunities.iter().cloned());
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
                    spell_power_bonus: total
                        .spell_power_bonus
                        .saturating_add(affix.modifiers.spell_power_bonus),
                    device_power_bonus: total
                        .device_power_bonus
                        .saturating_add(affix.modifiers.device_power_bonus),
                }
            },
        );
        add_stat_modifiers_dto(&mut modifiers, &item.intrinsic_properties.modifiers);
        for rolled in &item.rolled_affixes {
            add_stat_modifiers_dto(&mut modifiers, &rolled.properties.modifiers);
        }
        modifiers.defense = modifiers
            .defense
            .saturating_add(i32::from(item.enchantments.to_armor))
            .saturating_sub(self.equipped_curse_penalty(
                item,
                ItemCurseEffectDto::LowArmor,
                10,
                30,
            ));
        modifiers
    }

    pub(super) fn item_equipment_bonuses(&self, item: &ItemInstance) -> EquipmentBonuses {
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
        merge_equipment_bonuses(&mut bonuses, &item.intrinsic_properties.equipment_bonuses);
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
        passives.extend(&item.intrinsic_properties.passives);
        for rolled in &item.rolled_affixes {
            passives.extend(&rolled.properties.passives);
        }
        passives
    }

    pub(super) fn item_resists_enchantment(&self, item: &ItemInstance) -> bool {
        self.item_passives(item)
            .contains(&EquipmentPassive::NoEnchant)
            || self.content.item(&item.kind_id).is_some_and(|definition| {
                definition.resists_enchantment
                    || definition.tags.iter().any(|tag| tag == "no-enchant")
            })
    }

    fn armor_ego_index(&self, item: &ItemInstance) -> Option<u32> {
        self.content
            .item(&item.kind_id)?
            .rfb_base_kind
            .filter(|base| matches!(base.tval, 30..=38 | 40 | 45))?;
        Some(
            item.affix_ids
                .iter()
                .filter_map(|id| {
                    self.content
                        .affix(id)?
                        .rfb_ego
                        .as_ref()
                        .map(|ego| ego.source_index)
                })
                .next()
                .unwrap_or(0),
        )
    }

    fn armor_combat_enchantments(&self, item: &ItemInstance, ranged: bool) -> (i32, i32) {
        // master:equip.c also grants the Stone of War's non-weapon hit/damage
        // bonuses to archery. Melee already receives its equipment bonuses.
        if let Some(kind) = self.content.item(&item.kind_id)
            && kind
                .artifact_generation
                .as_ref()
                .is_some_and(|artifact| artifact.source_index == 291)
        {
            return (
                i32::from(item.enchantments.to_hit)
                    + if ranged {
                        kind.equipment_bonuses.melee_skill
                    } else {
                        0
                    },
                i32::from(item.enchantments.to_damage)
                    + if ranged {
                        kind.equipment_bonuses.melee_damage
                    } else {
                        0
                    },
            );
        }
        // master:equip.c excludes Terror Mask from shooter bonuses, including
        // enchantments applied after generation. Its static bonuses are melee-only.
        if self
            .content
            .item(&item.kind_id)
            .and_then(|kind| kind.artifact_generation.as_ref())
            .is_some_and(|artifact| artifact.source_index == 41)
        {
            return if ranged {
                (0, 0)
            } else {
                (
                    i32::from(item.enchantments.to_hit),
                    i32::from(item.enchantments.to_damage),
                )
            };
        }
        let Some(index) = self.armor_ego_index(item) else {
            return (0, 0);
        };
        // master:equip.c keeps sniper and magi bonuses out of melee, and the
        // listed melee egos out of archery.
        if matches!(index, 126 | 208 | 224)
            || (!ranged && matches!(index, 141 | 207))
            || (ranged
                && matches!(
                    index,
                    60 | 61 | 95 | 102 | 115 | 120 | 127 | 135..=137 | 142 | 206
                ))
        {
            return (0, 0);
        }
        (
            i32::from(item.enchantments.to_hit),
            i32::from(item.enchantments.to_damage),
        )
    }

    pub(super) fn armor_spell_damage_bonus(&self) -> u16 {
        self.items.iter().filter(|item| matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool")) && matches!(self.armor_ego_index(item), Some(126 | 208 | 224)))
            .map(|item| i32::from(item.enchantments.to_damage)).sum::<i32>().clamp(0, i32::from(u16::MAX)) as u16
    }

    pub(super) fn player_has_anti_magic(&self) -> bool {
        self.player_has_status_kind(crate::effect::STATUS_ANTI_MAGIC)
            || self
                .player_equipment_passives()
                .contains(&EquipmentPassive::AntiMagic)
    }

    pub(super) fn player_has_anti_teleport(&self) -> bool {
        self.player_equipment_passives()
            .contains(&EquipmentPassive::AntiTeleport)
    }

    pub(super) fn equipment_blocks_summoning(&mut self) -> bool {
        self.player_equipment_passives()
            .contains(&EquipmentPassive::AntiSummoning)
            && self.rng.bounded(3) != 0
    }

    pub(super) fn player_equipment_passives(&self) -> BTreeSet<EquipmentPassive> {
        let mut passives = self
            .items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            })
            .flat_map(|item| self.item_passives(item))
            .collect::<BTreeSet<_>>();
        passives.extend(
            self.player
                .statuses
                .iter()
                .filter_map(|status| temporary_sustain_passive(&status.kind_id)),
        );
        if self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE) {
            passives.extend([
                EquipmentPassive::HoldLife,
                EquipmentPassive::SustainStrength,
                EquipmentPassive::SustainIntelligence,
                EquipmentPassive::SustainWisdom,
                EquipmentPassive::SustainDexterity,
                EquipmentPassive::SustainConstitution,
                EquipmentPassive::SustainCharisma,
            ]);
        }
        passives
    }

    pub(super) fn player_equipment_bonuses(&self) -> EquipmentBonuses {
        self.items.iter().filter(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
        }).fold(EquipmentBonuses::default(), |mut total, item| {
            merge_equipment_bonuses(&mut total, &self.item_equipment_bonuses(item));
            total
        })
    }

    pub(super) fn player_sustains_attribute(&self, attribute: AttributeKind) -> bool {
        self.player_equipment_passives()
            .contains(&attribute_sustain_passive(attribute))
            || self
                .player_class_passives()
                .contains(&attribute_sustain_passive(attribute))
            || self.character_definitions().is_some_and(|(_, race, _, _)| {
                race.attribute_sustains
                    .iter()
                    .any(|sustain| Self::item_attribute_kind(sustain) == attribute)
            })
    }

    pub(super) fn player_hold_life_sources(&self) -> usize {
        self.items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
                    && self.item_passives(item).contains(&EquipmentPassive::HoldLife)
            })
            .count()
            + usize::from(
                self.player
                    .statuses
                    .iter()
                    .any(|status| {
                        matches!(
                            status.kind_id.as_str(),
                            STATUS_HOLD_LIFE
                                | STATUS_ULTIMATE_RESISTANCE
                                | STATUS_DEMON_LORD_TRANSFORMATION
                        )
                    }),
            )
            + usize::from(self.character_definitions().is_some_and(|(_, race, _, _)| {
                race.hold_life_minimum_level
                    .is_some_and(|minimum_level| self.progress.level >= minimum_level)
            }))
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
        let race_source = self.character_definitions().is_some_and(|(_, race, _, _)| {
            race.see_invisible
                || race
                    .see_invisible_minimum_level
                    .is_some_and(|minimum_level| self.progress.level >= minimum_level)
        });
        equipment_sources
            + usize::from(race_source)
            + usize::from(self.player.statuses.iter().any(|status| {
                matches!(
                    status.kind_id.as_str(),
                    STATUS_SIGHT | STATUS_SEE_INVISIBLE | STATUS_ULTIMATE_RESISTANCE
                )
            }))
    }

    pub(super) fn player_infravision_range(&self) -> i32 {
        let race = self
            .character_definitions()
            .map_or(0, |(_, race, _, _)| race.infravision);
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
            .fold(
                race.saturating_add(equipment).saturating_add(mutations),
                |total, status| total.saturating_add(status.granted_equipment_bonuses.infravision),
            )
            .max(0)
    }

    pub(super) fn player_levitates(&self) -> bool {
        self.player_has_status_kind(STATUS_LEVITATION)
            || self
                .character_definitions()
                .is_some_and(|(_, race, _, _)| race.levitation)
            || self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE)
            || self.player_has_status_kind(STATUS_MAGIC_ARMOR)
            || self.player_has_status_kind(STATUS_DEMON_LORD_TRANSFORMATION)
            || self
                .player_equipment_passives()
                .contains(&EquipmentPassive::Levitation)
            || self.content.mutations().any(|mutation| {
                mutation.levitation && self.progress.active_mutation_ids.contains(&mutation.id)
            })
    }

    pub(super) fn player_fairy_stealth_race_id(&self) -> Option<&str> {
        self.character_definitions()
            .and_then(|(_, race, _, _)| race.fairy_stealth.then_some(race.id.as_str()))
    }

    pub(super) fn player_has_targeted_esp(
        &self,
        definition: &rfb_content::ActorDefinition,
    ) -> bool {
        let passives = self.player_equipment_passives();
        [
            (EquipmentPassive::EspAnimal, "animal"),
            (EquipmentPassive::EspUndead, "undead"),
            (EquipmentPassive::EspDemon, "demon"),
            (EquipmentPassive::EspOrc, "orc"),
            (EquipmentPassive::EspTroll, "troll"),
            (EquipmentPassive::EspGiant, "giant"),
            (EquipmentPassive::EspDragon, "dragon"),
            (EquipmentPassive::EspHuman, "human"),
            (EquipmentPassive::EspGood, "good"),
            (EquipmentPassive::EspEvil, "evil"),
            (EquipmentPassive::EspLiving, "living"),
            (EquipmentPassive::EspNonliving, "nonliving"),
        ]
        .into_iter()
        .any(|(passive, tag)| {
            passives.contains(&passive) && definition.tags.iter().any(|value| value == tag)
        })
    }

    pub(super) fn player_equipment_life_percent(&self) -> i32 {
        self.items
            .iter()
            .filter(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
            })
            .fold(0_i32, |total, item| {
                total.saturating_add(self.item_equipment_bonuses(item).life_percent)
            })
    }

    pub(super) fn player_has_telepathy(&self) -> bool {
        self.player_has_status_kind(STATUS_TELEPATHY)
            || self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE)
            || self.player_has_status_kind(STATUS_DEMON_LORD_TRANSFORMATION)
            || self
                .player_equipment_passives()
                .contains(&EquipmentPassive::Telepathy)
            || self.player_has_permanent_telepathy()
    }

    pub(super) fn player_has_permanent_telepathy(&self) -> bool {
        self.character_definitions().is_some_and(|(_, race, _, _)| {
            race.telepathy_minimum_level
                .is_some_and(|minimum_level| self.progress.level >= minimum_level)
        }) || self.content.mutations().any(|mutation| {
            mutation.telepathy && self.progress.active_mutation_ids.contains(&mutation.id)
        }) || self
            .player_class_passives()
            .contains(&EquipmentPassive::Telepathy)
    }

    pub(super) fn player_is_mindcrafter(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, _, class, _)| class.id == "demo.class.mindcrafter")
    }

    pub(super) fn player_is_mage(&self) -> bool {
        self.build
            .as_ref()
            .is_some_and(|build| build.class_id == "demo.class.mage")
    }

    pub(super) fn player_is_berserker(&self) -> bool {
        self.build
            .as_ref()
            .is_some_and(|build| build.class_id == "demo.class.berserker")
    }

    pub(super) fn player_class_passives(&self) -> Vec<EquipmentPassive> {
        if self.player_is_berserker() {
            let mut passives = vec![
                EquipmentPassive::SustainStrength,
                EquipmentPassive::SustainDexterity,
                EquipmentPassive::SustainConstitution,
            ];
            if self.progress.level >= 40 {
                passives.push(EquipmentPassive::ReflectsBolts);
            }
            return passives;
        }
        // RFB master a0d92b6378: mindcrafter.c::_calc_bonuses.
        [
            (20, EquipmentPassive::SustainWisdom),
            (40, EquipmentPassive::Telepathy),
        ]
        .into_iter()
        .filter(|(level, _)| self.player_is_mindcrafter() && self.progress.level >= *level)
        .map(|(_, passive)| passive)
        .collect()
    }

    pub(super) fn player_regeneration_rate_percent(&self) -> u64 {
        let race_modifier = self
            .character_definitions()
            .map_or(0, |(_, race, _, _)| race.regeneration_rate_modifier_percent);
        let modifier = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(race_modifier, |total, mutation| {
                total.saturating_add(mutation.regeneration_rate_modifier_percent)
            });
        let timed_regeneration = if self.player_has_status_kind(STATUS_REGENERATION)
            || self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE)
        {
            100
        } else {
            0
        };
        u64::try_from(
            100_i32
                .saturating_add(modifier)
                .saturating_add(timed_regeneration)
                .saturating_add(if self.player_is_berserker() { 100 } else { 0 })
                .max(0),
        )
        .expect("non-negative regeneration rate must fit u64")
            / if self.player_has_equipped_curse_effect(ItemCurseEffectDto::SlowRegeneration) {
                5
            } else {
                1
            }
    }

    pub(super) fn player_slow_digestion(&self) -> bool {
        self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE)
            || self
                .character_definitions()
                .is_some_and(|(_, race, _, _)| race.tags.iter().any(|tag| tag == "slow-digestion"))
            || self
                .player_equipment_passives()
                .contains(&EquipmentPassive::SlowDigestion)
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
                    spell_power_bonus: total
                        .spell_power_bonus
                        .saturating_add(item.spell_power_bonus),
                    device_power_bonus: total
                        .device_power_bonus
                        .saturating_add(item.device_power_bonus),
                }
            })
    }

    pub(super) fn effective_player_spell_power_bonus(&self) -> i32 {
        let mutation_bonus = self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
            .fold(0_i32, |total, mutation| {
                total.saturating_add(mutation.modifiers.spell_power_bonus)
            });
        self.player.statuses.iter().fold(
            self.equipment_modifiers()
                .spell_power_bonus
                .saturating_add(mutation_bonus),
            |total, status| total.saturating_add(status.granted_modifiers.spell_power_bonus),
        )
    }

    pub(super) fn effective_player_device_power_bonus(&self) -> i32 {
        self.player.statuses.iter().fold(
            self.equipment_modifiers().device_power_bonus,
            |total, status| total.saturating_add(status.granted_modifiers.device_power_bonus),
        )
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
            .map(|profile| {
                let damage = item
                    .melee_damage_dice()
                    .map_or((profile.damage_dice, profile.damage_sides), |damage| {
                        (damage.dice, damage.sides)
                    });
                AttackProfileDto {
                    attacks: profile.attacks,
                    to_hit: profile
                        .to_hit
                        .saturating_add(i32::from(item.enchantments.to_hit)),
                    to_damage: profile
                        .to_damage
                        .saturating_add(i32::from(item.enchantments.to_damage)),
                    damage: DamageDiceDto {
                        dice: damage.0,
                        sides: damage.1,
                        damage_type: DamageType::from(profile.damage_type).into(),
                    },
                    source_item_id: Some(item.id.clone()),
                }
            })
    }

    pub(super) fn item_has_weapon_trait(
        &self,
        item: &ItemInstance,
        trait_: WeaponTraitDto,
    ) -> bool {
        (trait_ == WeaponTraitDto::ManaBrand
            && self.content.item(&item.kind_id).is_some_and(|definition| {
                definition
                    .rfb_value
                    .as_ref()
                    .is_some_and(|value| value.flags.contains("BRAND_MANA"))
            }))
            || item.intrinsic_weapon_traits.contains(&trait_)
            || (trait_ == WeaponTraitDto::Order && self.item_has_rfb_flag(item, "BRAND_ORDER"))
            || (trait_ == WeaponTraitDto::Blessed && self.item_has_rfb_flag(item, "BLESSED"))
            || item
                .rolled_affixes
                .iter()
                .any(|rolled| rolled.weapon_traits.contains(&trait_))
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
        let bonuses = self.item_equipment_bonuses(item);
        let multiplier = launcher_multiplier(
            profile.damage_multiplier_percent,
            bonuses.launcher_multiplier_delta_percent,
        );
        let range = launcher_range(multiplier);
        Some(ProjectileProfileDto {
            range,
            to_hit: profile
                .to_hit
                .saturating_add(i32::from(item.enchantments.to_hit))
                .saturating_add(ammo.to_hit),
            to_damage: ammo.to_damage.saturating_mul(i32::from(multiplier)) / 100
                + profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
            damage: DamageDiceDto {
                dice: ammo.damage_dice,
                sides: ammo.damage_sides,
                damage_type: DamageType::from(ammo.damage_type).into(),
            },
            ammo_kind_id: ammunition.id.clone(),
            target_spec: projectile_target_spec(range),
            source_item_id: item.id.clone(),
        })
    }

    pub(super) fn player_tomte_headgear_excess_weight(&self) -> u16 {
        if self
            .character_definitions()
            .is_none_or(|(_, race, _, _)| race.id != "rfb-legacy.race.tomte")
        {
            return 0;
        }
        // RFB master a0d92b6378: races_k.c::tomte_heavy_armor.
        // Helmets and crowns share the head equipment slot in this runtime.
        self.items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
            .find(|item| {
                self.content
                    .item(&item.kind_id)
                    .is_some_and(|kind| kind.equipment_slot.as_deref() == Some("head"))
            })
            .map_or(0, |item| self.item_instance_weight(item).saturating_sub(10))
    }

    pub(super) fn item_weight_tenths_pound(&self, kind_id: &str) -> u16 {
        self.content
            .item(kind_id)
            .map_or(0, |definition| definition.weight_tenths_pound)
    }

    pub(super) fn item_instance_weight(&self, item: &ItemInstance) -> u16 {
        item.weight_override()
            .unwrap_or_else(|| self.item_weight_tenths_pound(&item.kind_id))
    }

    pub(super) fn carried_weight_tenths_pound(&self) -> u32 {
        let phase_quiver = self.items.iter().any(|item| {
            matches!(item.location, ItemLocation::Equipped { .. })
                && self.content.item(&item.kind_id).is_some_and(|definition| {
                    definition
                        .rfb_base_kind
                        .is_some_and(|base| base.tval == 46 && base.sval == 0)
                })
                && super::ego::item_has_ego(&self.content, item, 268)
        });
        let weightless_ammunition = if phase_quiver {
            super::inventory::quivered_ammunition_item_ids(&self.content, &self.items)
        } else {
            BTreeSet::new()
        };
        self.items
            .iter()
            .filter(|item| {
                matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                )
            })
            .filter(|item| !weightless_ammunition.contains(item.id.as_str()))
            .fold(0_u32, |total, item| {
                total.saturating_add(
                    u32::from(self.item_instance_weight(item)).saturating_mul(item.quantity),
                )
            })
    }

    pub(super) fn player_carry_capacity_tenths_pound(&self) -> u32 {
        let capacity =
            crate::stats::carry_capacity_tenths_pound(self.effective_player_attributes().strength);
        if self.player_is_berserker() {
            capacity * 3 / 2
        } else {
            capacity
        }
    }

    pub(super) fn player_encumbrance_speed_penalty(&self) -> i32 {
        crate::stats::encumbrance_speed_penalty(
            self.carried_weight_tenths_pound(),
            self.player_carry_capacity_tenths_pound(),
        )
    }

    pub(super) fn item_throw_profile(&self, item: &ItemInstance) -> Option<ThrowProfileDto> {
        if self.item_has_rfb_flag(item, "THROWING") {
            let profile = self.item_melee_profile(item)?;
            return Some(ThrowProfileDto {
                range: self.item_throw_parameters(item).0,
                to_hit: profile.to_hit,
                to_damage: profile.to_damage,
                damage: profile.damage,
                source_item_id: item.id.clone(),
            });
        }
        let definition = self.content.item(&item.kind_id)?;
        definition
            .throw_profile
            .as_ref()
            .map(|profile| ThrowProfileDto {
                range: self.item_throw_parameters(item).0,
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage: DamageDiceDto {
                    dice: item
                        .melee_damage_dice()
                        .map_or(profile.damage_dice, |dice| dice.dice),
                    sides: item
                        .melee_damage_dice()
                        .map_or(profile.damage_sides, |dice| dice.sides),
                    damage_type: DamageType::from(profile.damage_type).into(),
                },
                source_item_id: item.id.clone(),
            })
    }

    pub(super) fn item_throw_parameters(&self, item: &ItemInstance) -> (u16, i32) {
        let weight = self.item_instance_weight(item);
        let mighty = self.player_has_mighty_throw();
        if !self.item_has_rfb_flag(item, "THROWING") {
            return (throw_range(weight, mighty), if mighty { 200 } else { 100 });
        }
        // RFB py_throw.c: THROWING adds 100 to the multiplier and halves the
        // effective weight for range; mighty throw adds another 100.
        const STRENGTH_DAMAGE: [i32; 38] = [
            -2, -2, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 3, 3, 3, 3, 3, 4, 5, 5, 6, 7, 8,
            9, 10, 11, 12, 13, 14, 15, 16, 18, 20,
        ];
        let index = usize::from(
            self.effective_player_attributes()
                .index(AttributeKind::Strength)
                .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP),
        );
        let multiplier = (200 + i32::from(mighty) * 100) * (100 + STRENGTH_DAMAGE[index]) / 100;
        let limit = 10 + 2 * (multiplier - 100) / 100;
        let range =
            (i32::from(RFB_STRENGTH_BLOW[index]) + 20) * limit / i32::from(weight.max(10) / 2);
        (range.min(limit).clamp(5, 18) as u16, multiplier)
    }

    pub(super) fn body_slot_type(&self, slot_id: &str) -> Option<&str> {
        self.body_slots
            .iter()
            .find(|slot| slot.id == slot_id)
            .map(|slot| slot.slot_type.as_str())
    }

    pub(super) fn sniping_profile(&self) -> Option<&rfb_content::SnipingProfileDefinition> {
        self.character_definitions()
            .and_then(|(_, _, class, _)| class.sniping_profile.as_ref())
    }

    pub(super) fn sniper_max_concentration(&self) -> Option<u8> {
        self.sniping_profile()
            .map(|profile| profile.maximum_concentration(self.progress.level))
    }

    pub(super) fn sniper_concentration_bonus_percent(&self, concentration: u8) -> i32 {
        self.sniping_profile().map_or(0, |profile| {
            i32::from(concentration)
                .saturating_mul(i32::from(profile.concentration_bonus_percent_per_level))
        })
    }

    pub(super) fn player_projectile_profile(&self) -> Option<ResolvedProjectileProfile> {
        self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if self.body_slot_type(slot_id) != Some("launcher") {
                return None;
            }
            let launcher_definition = self.content.item(&item.kind_id)?;
            launcher_definition
                .projectile_profile
                .as_ref()
                .and_then(|profile| {
                    let bonuses = self.item_equipment_bonuses(item);
                    let extra_might = self.items.iter().filter(|other| {
                        other.id != item.id && matches!(&other.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
                    }).map(|other| self.item_equipment_bonuses(other).launcher_multiplier_delta_percent).sum::<i32>();
                    let multiplier = launcher_multiplier(
                        profile.damage_multiplier_percent,
                        bonuses.launcher_multiplier_delta_percent + extra_might * i32::from(profile.shot_energy) / 10_000,
                    );
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
                    let mut ammunition_slays = ammo_definition.slays.clone();
                    let mut ammunition_brands = ammo_definition.brands.clone();
                    let mut ammunition_behavior = None;
                    let mut ammunition_endurance = false;
                    if let Some(ammunition) = ammunition {
                        for affix_id in &ammunition.affix_ids {
                            if let Some(affix) = self.content.affix(affix_id) {
                                ammunition_slays.extend(
                                    affix.slays.iter().map(|(target, level)| (*target, *level)),
                                );
                                ammunition_brands.extend(affix.brands.iter().copied());
                                ammunition_behavior =
                                    ammunition_behavior.or(affix.ammunition_behavior);
                                ammunition_endurance |= affix
                                    .rfb_ego
                                    .as_ref()
                                    .is_some_and(|ego| ego.source_index == 184);
                            }
                        }
                        ammunition_slays.extend(
                            ammunition
                                .intrinsic_properties
                                .slays
                                .iter()
                                .map(|(target, level)| (*target, *level)),
                        );
                        ammunition_brands
                            .extend(ammunition.intrinsic_properties.brands.iter().copied());
                        for rolled in &ammunition.rolled_affixes {
                            ammunition_slays.extend(
                                rolled
                                    .properties
                                    .slays
                                    .iter()
                                    .map(|(target, level)| (*target, *level)),
                            );
                            ammunition_brands.extend(rolled.properties.brands.iter().copied());
                        }
                    }
                    let ranged_skill = self.player_derived_stats().ranged_skill.value;
                    let hold = crate::stats::strength_hold_pounds(
                        self.effective_player_attributes().strength,
                    );
                    let launcher_weight_pounds = self.item_instance_weight(item) / 10;
                    let heavy_shoot = hold < launcher_weight_pounds;
                    let heavy_to_hit = if heavy_shoot {
                        2_i32.saturating_mul(
                            i32::from(hold).saturating_sub(i32::from(launcher_weight_pounds)),
                        )
                    } else {
                        0
                    };
                    let mut base_shot = if heavy_shoot {
                        100
                    } else {
                        ranged_skill.max(100)
                    };
                    let class = self.character_definitions().map(|(_, _, class, _)| class);
                    let sniping_profile = class.and_then(|class| class.sniping_profile.as_ref());
                    if let Some(sniping) = sniping_profile {
                        let excess = base_shot.saturating_sub(100);
                        base_shot = 100_i32.saturating_add(
                            excess.saturating_mul(i32::from(sniping.base_shot_excess_percent))
                                / 100,
                        );
                    }
                    let mounted_to_hit = self.riding_mount_level().map_or(0, |mount_level| {
                        riding_proficiency::mounted_projectile_to_hit_adjustment(
                            class.is_some_and(|class| class.riding_combat_expert),
                            profile.ammunition_type,
                            mount_level,
                            self.progress.riding_proficiency,
                        )
                    });
                    if self.riding_actor_id.is_some()
                        && profile.ammunition_type != AmmunitionTypeDefinition::Arrow
                        && let Some(cap) = class
                            .and_then(|class| class.mounted_non_arrow_base_shot_cap.map(i32::from))
                    {
                        base_shot = base_shot.min(cap);
                    }
                    base_shot = base_shot
                        .saturating_add(bonuses.base_shot_delta_percent)
                        .max(1);
                    let energy_cost = (i32::from(profile.shot_energy) / base_shot).max(1);
                    let breakage_modifier = if heavy_shoot {
                        0
                    } else {
                        self.character_definitions().map_or(0, |(_, _, class, _)| {
                            class.ammunition_breakage_factor_modifier
                        })
                    };
                    let breakage_factor = if ranged_skill > 80 {
                        90_i32.saturating_sub((ranged_skill - 80) / 2)
                    } else {
                        100
                    }
                    .saturating_add(i32::from(breakage_modifier))
                    .max(0);
                    let ammo_break_chance_percent = u8::try_from(
                        i32::from(ammo_definition.break_chance_percent)
                            .saturating_mul(breakage_factor)
                            / 100,
                    )
                    .unwrap_or(u8::MAX);
                    let ammunition_to_hit = ammo_profile.to_hit.saturating_add(i32::from(
                        ammunition.map_or(0, |item| item.enchantments.to_hit),
                    ));
                    let ammunition_to_damage = ammo_profile.to_damage.saturating_add(i32::from(
                        ammunition.map_or(0, |item| item.enchantments.to_damage),
                    ));
                    let sniping_to_hit = sniping_profile
                        .filter(|sniping| {
                            sniping.preferred_ammunition_type == profile.ammunition_type
                        })
                        .map_or(0, |sniping| {
                            i32::from(sniping.preferred_ammunition_to_hit_base).saturating_add(
                                i32::from(
                                    self.progress.level
                                        / sniping.preferred_ammunition_to_hit_level_divisor,
                                ),
                            )
                        });
                    let (armor_to_hit, armor_to_damage) = self.items.iter().filter(|item| matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool")))
                        .map(|item| self.armor_combat_enchantments(item, true)).fold((0, 0), |(hit, damage), (h, d)| (hit + h, damage + d));
                    let launcher_to_damage = profile
                        .to_damage
                        .saturating_add(i32::from(item.enchantments.to_damage))
                        .saturating_add(armor_to_damage);
                    Some(ResolvedProjectileProfile {
                        range: launcher_range(multiplier),
                        to_hit: profile
                            .to_hit
                            .saturating_add(i32::from(item.enchantments.to_hit))
                            .saturating_sub(self.equipped_curse_penalty(item, ItemCurseEffectDto::LowMelee, 5, 15))
                            .saturating_add(heavy_to_hit)
                            .saturating_add(mounted_to_hit)
                            .saturating_add(sniping_to_hit)
                            .saturating_add(armor_to_hit)
                            .saturating_add(ammunition_to_hit),
                        to_damage: ammunition_to_damage.saturating_mul(i32::from(multiplier)) / 100
                            + launcher_to_damage,
                        ammunition_to_damage,
                        launcher_to_damage,
                        damage_multiplier_percent: multiplier,
                        damage_dice: ammunition
                            .and_then(|item| item.melee_damage_dice().map(|dice| dice.dice).or(item.damage_dice_override))
                            .unwrap_or(ammo_profile.damage_dice),
                        damage_sides: ammunition.and_then(|item| item.melee_damage_dice()).map_or(ammo_profile.damage_sides, |dice| dice.sides),
                        damage_type: DamageType::from(ammo_profile.damage_type),
                        ammunition_slays,
                        ammunition_brands,
                        ammunition_behavior,
                        ammunition_endurance,
                        ammo_item_id: ammunition.map(|item| item.id.clone()),
                        ammo_kind_id: ammo_definition.id.clone(),
                        ammunition_weight_tenths_pound: ammunition.map_or(ammo_definition.weight_tenths_pound, |item| self.item_instance_weight(item)),
                        ammunition_type: profile.ammunition_type,
                        ammo_break_chance_percent,
                        base_shot,
                        energy_cost,
                        source_item_id: item.id.clone(),
                    })
                })
        })
    }

    pub(super) fn player_melee_damage_percent(&self) -> u16 {
        self.character_definitions()
            .map_or(100, |(_, race, _, _)| race.melee_damage_percent)
    }

    pub(super) fn scale_player_melee_damage(&self, damage: i32) -> i32 {
        // RFB cmd1.c: race multiplier follows dice, criticals and damage bonuses.
        let scaled =
            (i64::from(damage.max(0)) * i64::from(self.player_melee_damage_percent()) + 50) / 100;
        i32::try_from(scaled).unwrap_or(i32::MAX)
    }

    pub(super) fn player_melee_profile(&self, stats: &ActorDerivedStats) -> ResolvedAttackProfile {
        self.player_melee_profile_for_item(
            stats,
            self.equipped_melee_weapons()
                .first()
                .map(|item| item.id.as_str()),
        )
    }

    pub(super) fn equipped_melee_weapons(&self) -> Vec<&ItemInstance> {
        self.body_slots.iter().filter(|slot| matches!(slot.slot_type.as_str(), "weapon" | "shield"))
            .filter_map(|slot| self.items.iter().find(|item| {
                matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == &slot.id)
                    && self.content.item(&item.kind_id).is_some_and(|definition| definition.melee_profile.is_some())
            })).collect()
    }

    pub(super) fn player_melee_profiles(
        &self,
        stats: &ActorDerivedStats,
    ) -> Vec<ResolvedAttackProfile> {
        let weapons = self.equipped_melee_weapons();
        if weapons.is_empty() {
            return vec![self.player_melee_profile_for_item(stats, None)];
        }
        weapons
            .into_iter()
            .map(|item| self.player_melee_profile_for_item(stats, Some(&item.id)))
            .collect()
    }

    fn ring_affects_weapon(&self, ring_slot: &str, weapon_id: Option<&str>) -> bool {
        let Some(weapon) = self
            .items
            .iter()
            .find(|item| Some(item.id.as_str()) == weapon_id)
        else {
            return false;
        };
        let ItemLocation::Equipped { slot_id } = &weapon.location else {
            return false;
        };
        let hands: Vec<_> = self
            .body_slots
            .iter()
            .filter(|slot| matches!(slot.slot_type.as_str(), "weapon" | "shield"))
            .collect();
        let Some(hand) = hands.iter().position(|slot| &slot.id == slot_id) else {
            return false;
        };
        let ring_hand = self
            .body_slots
            .iter()
            .filter(|slot| slot.slot_type == "ring")
            .position(|slot| slot.id == ring_slot);
        if ring_hand == Some(hand) {
            return true;
        }
        self.weapon_uses_two_hands(weapon)
    }

    fn weapon_uses_two_hands(&self, weapon: &ItemInstance) -> bool {
        let ItemLocation::Equipped {
            slot_id: weapon_slot,
        } = &weapon.location
        else {
            return false;
        };
        let other_empty = self.body_slots.iter().any(|slot| {
            matches!(slot.slot_type.as_str(), "weapon" | "shield")
                && &slot.id != weapon_slot
                && !self.items.iter().any(|item| {
                    matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == &slot.id)
                })
        });
        let definition = self.content.item(&weapon.kind_id).unwrap();
        other_empty
            && self.riding_mount_level().is_none()
            && (self.item_instance_weight(weapon) > 99
                || definition
                    .rfb_base_kind
                    .is_some_and(|kind| kind.tval == 22 || (kind.tval == 21 && kind.sval == 51)))
    }

    fn class_base_blows(
        &self,
        weapon: &ItemInstance,
        cap: i32,
        minimum_weight: u16,
        multiplier: u32,
    ) -> i32 {
        // RFB combat.c::calculate_base_blows;
        // xtra1.c applies doubled hold and the omoi check for two-handed weapons.
        let attributes = self.effective_player_attributes();
        let strength = attributes
            .index(AttributeKind::Strength)
            .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
        let dexterity = attributes
            .index(AttributeKind::Dexterity)
            .min(crate::stats::PRE_VICTORY_ATTRIBUTE_INDEX_CAP);
        let weight = self.item_instance_weight(weapon);
        let two_hands = self.weapon_uses_two_hands(weapon);
        let hold = crate::stats::strength_hold_pounds(attributes.value(AttributeKind::Strength))
            * if two_hands { 2 } else { 1 };
        if hold < weight / 10 {
            return 100;
        }
        let two_hand_bonus = if two_hands && hold >= weight / 5 {
            10
        } else {
            0
        };
        let power = (u32::from(RFB_STRENGTH_BLOW[usize::from(strength)]) * multiplier
            / u32::from(weight.max(minimum_weight))
            + two_hand_bonus)
            .min(110);
        let (minimum, maximum) = RFB_BLOW_RANGES[usize::from(dexterity)];
        (i32::from(minimum) + i32::from(maximum - minimum) * power as i32 / 110).clamp(100, cap)
    }

    fn player_melee_profile_for_item(
        &self,
        stats: &ActorDerivedStats,
        selected_item_id: Option<&str>,
    ) -> ResolvedAttackProfile {
        let mut adjusted_stats = stats.clone();
        for other in self
            .equipped_melee_weapons()
            .into_iter()
            .filter(|item| Some(item.id.as_str()) != selected_item_id)
        {
            adjusted_stats.melee_skill =
                derived_stat_without_source(&adjusted_stats.melee_skill, &other.id, true);
            adjusted_stats.melee_damage_bonus =
                derived_stat_without_source(&adjusted_stats.melee_damage_bonus, &other.id, false);
            adjusted_stats.melee_attacks =
                derived_stat_without_source(&adjusted_stats.melee_attacks, &other.id, true);
        }
        let stats = &adjusted_stats;
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        let equipped_weapon = self.items.iter().find_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            if Some(item.id.as_str()) != selected_item_id
                || !matches!(self.body_slot_type(slot_id), Some("weapon" | "shield"))
            {
                return None;
            }
            self.content
                .item(&item.kind_id)
                .and_then(|item_definition| {
                    item_definition.melee_profile.as_ref().map(|profile| {
                        let (priest_class, good_realm) = self.character_definitions().map_or(
                            (false, false),
                            |(build, _, class, _)| {
                                (
                                    class.tags.iter().any(|tag| tag == "priest"),
                                    [
                                        build.first_realm_id.as_deref(),
                                        build.second_realm_id.as_deref(),
                                    ]
                                    .into_iter()
                                    .flatten()
                                    .any(|realm| matches!(realm, "life" | "crusade")),
                                )
                            },
                        );
                        (
                            item.id.clone(),
                            item.kind_id.clone(),
                            profile,
                            item_definition.riding_weapon_kind,
                            self.item_instance_weight(item),
                            i32::from(item.enchantments.to_hit)
                                - self.equipped_curse_penalty(
                                    item,
                                    ItemCurseEffectDto::LowMelee,
                                    5,
                                    15,
                                ),
                            item.melee_damage_dice(),
                            good_priest_weapon_penalty(
                                priest_class,
                                good_realm,
                                item_definition.rfb_base_kind.map(|kind| kind.tval),
                                self.item_has_weapon_trait(item, WeaponTraitDto::Blessed),
                            ),
                        )
                    })
                })
        });
        let (
            source_item_id,
            source_kind_id,
            dice,
            sides,
            damage_type,
            mut to_hit,
            mounted_to_hit,
            critical_weight_tenths_pound,
            priest_weapon_penalty,
        ) = equipped_weapon.map_or_else(
            || {
                (
                    None,
                    None,
                    definition.damage_dice,
                    definition.damage_sides,
                    definition.damage_type,
                    0,
                    0,
                    None,
                    false,
                )
            },
            |(
                item_id,
                kind_id,
                profile,
                riding_weapon_kind,
                weight,
                enchantment_to_hit,
                rolled_damage,
                priest_weapon_penalty,
            )| {
                let (mounted_to_hit, mounted_dice) =
                    self.riding_mount_level().map_or((0, 0), |mount_level| {
                        riding_proficiency::mounted_melee_adjustment(
                            self.character_definitions()
                                .is_some_and(|(_, _, class, _)| class.riding_combat_expert),
                            riding_weapon_kind,
                            mount_level,
                            self.progress.riding_proficiency,
                        )
                    });
                let (damage_dice, damage_sides) = rolled_damage
                    .map_or((profile.damage_dice, profile.damage_sides), |damage| {
                        (damage.dice, damage.sides)
                    });
                (
                    Some(item_id),
                    Some(kind_id),
                    damage_dice.saturating_add(mounted_dice),
                    damage_sides,
                    profile.damage_type,
                    profile
                        .to_hit
                        .saturating_add(enchantment_to_hit)
                        .saturating_add(mounted_to_hit),
                    mounted_to_hit,
                    Some(weight),
                    priest_weapon_penalty,
                )
            },
        );
        let mut melee_skill = source_kind_id
            .as_deref()
            .and_then(|kind_id| self.weapon_proficiency_hit_modifier(kind_id))
            .map_or_else(
                || stats.melee_skill.clone(),
                |(base_item_id, modifier)| {
                    if modifier == 0 {
                        stats.melee_skill.clone()
                    } else {
                        stats.melee_skill.with_modifier(
                            StatLayer::Class,
                            base_item_id,
                            modifier,
                            StatBounds::NON_NEGATIVE,
                        )
                    }
                },
            );
        if mounted_to_hit != 0 {
            melee_skill = melee_skill.with_modifier(
                StatLayer::Class,
                "rfb.riding",
                mounted_to_hit,
                StatBounds::NON_NEGATIVE,
            );
        }
        let mut to_damage = stats.melee_damage_bonus.value;
        let weapons = self.equipped_melee_weapons();
        let hand = weapons
            .iter()
            .position(|item| Some(item.id.as_str()) == source_item_id.as_deref())
            .unwrap_or(0) as i32;
        let count = weapons.len().max(1) as i32;
        if self.player_is_berserker()
            && let Some(weapon) = weapons
                .iter()
                .find(|item| Some(item.id.as_str()) == source_item_id.as_deref())
        {
            let attributes = self.effective_player_attributes();
            let two_hands = self.weapon_uses_two_hands(weapon)
                && crate::stats::strength_hold_pounds(attributes.strength) * 2
                    >= self.item_instance_weight(weapon) / 5;
            let multiplier = if two_hands { 2 } else { 1 };
            let class_hit = i32::from(self.progress.level / 5) * multiplier;
            // xtra1.c adds a second +12 actual to-hit to the first two hands.
            let extra_hit = if hand < 2 { 12 } else { 0 };
            melee_skill = melee_skill.with_modifier(
                StatLayer::Class,
                "demo.class.berserker",
                class_hit + extra_hit,
                StatBounds::UNBOUNDED,
            );
            to_hit += class_hit + 12 + extra_hit;
            to_damage += i32::from(self.progress.level / 6) * multiplier;
        }
        let mut mastery =
            if source_item_id.is_some() && self.player_has_status_kind(STATUS_WEAPON_MASTERY) {
                i32::from(self.progress.level / 23)
            } else {
                0
            };
        for item in &self.items {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                continue;
            };
            if self.body_slot_type(slot_id) == Some("tool") {
                continue;
            }
            let (hit, damage) = self.armor_combat_enchantments(item, false);
            let share = |value: i32| {
                if self.body_slot_type(slot_id) == Some("ring") {
                    if self.ring_affects_weapon(slot_id, source_item_id.as_deref()) {
                        value
                    } else {
                        0
                    }
                } else if count == 2 && self.body_slot_type(slot_id) == Some("gloves") {
                    if hand == 0 {
                        (value + 1) / 2
                    } else {
                        value / 2
                    }
                } else {
                    value / count
                        + if hand < (value % count).abs() {
                            value.signum()
                        } else {
                            0
                        }
                }
            };
            let hit_share = share(hit);
            mastery += share(self.item_equipment_bonuses(item).weapon_dice_bonus);
            if hit_share != hit {
                melee_skill = melee_skill.with_modifier(
                    StatLayer::Equipment,
                    &item.id,
                    hit_share - hit,
                    StatBounds::NON_NEGATIVE,
                );
            }
            to_hit += hit_share;
            to_damage += share(damage) - damage;
        }
        let dice = (i32::from(dice) + mastery).clamp(0, i32::from(u16::MAX)) as u16;
        if let Some(item_id) = source_item_id.as_deref() {
            let percent = self.dual_wielding_accuracy_per_mille(item_id);
            if percent != 1000 {
                let penalty = melee_skill.value * percent / 1000 - melee_skill.value;
                melee_skill = melee_skill.with_modifier(
                    StatLayer::Equipment,
                    "rfb.dual-wielding",
                    penalty,
                    StatBounds::NON_NEGATIVE,
                );
            }
        }
        if priest_weapon_penalty {
            melee_skill = melee_skill.with_modifier(
                StatLayer::Class,
                "rfb.class.priest-unblessed-weapon",
                -2,
                StatBounds::NON_NEGATIVE,
            );
            to_hit = to_hit.saturating_sub(2);
            to_damage = to_damage.saturating_sub(2);
        }
        let mut attack_sources = stats
            .melee_attacks
            .contributions
            .iter()
            .map(|entry| rfb_protocol::CharacterStatSourceDto {
                source_id: entry.source_id.clone(),
                amount: entry.amount.saturating_mul(100),
            })
            .collect::<Vec<_>>();
        let extra_sources = self.items.iter().filter(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))
                && (Some(item.id.as_str()) == selected_item_id || self.content.item(&item.kind_id).is_none_or(|definition| definition.melee_profile.is_none()))
        }).map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else { unreachable!(); };
            let amount = self.item_equipment_bonuses(item).melee_attacks_delta_percent;
            let amount = if self.body_slot_type(slot_id) == Some("ring") {
                if self.ring_affects_weapon(slot_id, source_item_id.as_deref()) { amount } else { 0 }
            } else if Some(item.id.as_str()) == selected_item_id || (self.body_slot_type(slot_id) == Some("gloves") && amount < 0) {
                amount
            } else { amount / count };
            rfb_protocol::CharacterStatSourceDto { source_id: item.id.clone(), amount }
        }).filter(|source| source.amount != 0).collect::<Vec<_>>();
        let extra_blows = extra_sources
            .iter()
            .map(|source| source.amount)
            .sum::<i32>();
        let extra_blows = if self.player_is_berserker() || self.player_is_duelist() {
            extra_blows
        } else {
            extra_blows.max(0)
        };
        if extra_blows != 0 {
            attack_sources.extend(extra_sources);
        }
        let mut blows = stats
            .melee_attacks
            .value
            .saturating_mul(100)
            .saturating_add(extra_blows);
        if (self.player_is_mindcrafter()
            || self.player_is_berserker()
            || self.player_is_duelist()
            || self.player_is_mage())
            && let Some(weapon) = self
                .items
                .iter()
                .find(|item| Some(item.id.as_str()) == selected_item_id)
        {
            let (class_id, base, extra) = if self.player_is_berserker() {
                (
                    "demo.class.berserker",
                    self.class_base_blows(weapon, 600, 70, 75),
                    i32::from(self.progress.level) * 4,
                )
            } else if self.player_is_duelist() {
                (
                    "demo.class.duelist",
                    self.class_base_blows(weapon, 100, 70, 40),
                    0,
                )
            } else if self.player_is_mage() {
                (
                    "demo.class.mage",
                    self.class_base_blows(weapon, 400, 100, 20),
                    0,
                )
            } else {
                (
                    "demo.class.mindcrafter",
                    self.class_base_blows(weapon, 500, 100, 35),
                    0,
                )
            };
            let delta = base - 100 + extra;
            blows = blows.saturating_add(delta);
            attack_sources.push(rfb_protocol::CharacterStatSourceDto {
                source_id: class_id.to_owned(),
                amount: delta,
            });
        }
        blows = blows.max(0);
        if self.player_is_duelist()
            && self.duelist_equipment_error().is_none()
            && let Some(weapon) = weapons
                .iter()
                .find(|item| Some(item.id.as_str()) == source_item_id.as_deref())
        {
            let dexterity = i32::from(
                self.effective_player_attributes()
                    .index(AttributeKind::Dexterity),
            );
            to_damage += dexterity + 3 - 10 + i32::from(self.progress.level / 2)
                - i32::from(self.item_instance_weight(weapon) / 10);
            if blows > 100 {
                attack_sources.push(rfb_protocol::CharacterStatSourceDto {
                    source_id: "demo.class.duelist".to_owned(),
                    amount: 100 - blows,
                });
                blows = 100;
            }
        }
        if source_item_id.is_some()
            && !self.player_is_duelist()
            && self
                .character_definitions()
                .is_some_and(|(_, race, _, _)| race.id == "rfb-legacy.race.tonberry")
        {
            // RFB master a0d92b6378: _tonberry_calc_bonuses, NUM_BLOWS.
            to_damage = to_damage.saturating_add(2 * i32::from(self.progress.level) / count);
            let penalty = (4 * i32::from(self.progress.level)).min(blows);
            blows -= penalty;
            attack_sources.push(rfb_protocol::CharacterStatSourceDto {
                source_id: "rfb-legacy.race.tonberry".to_owned(),
                amount: -penalty,
            });
        }
        let poison_needle = source_kind_id
            .as_deref()
            .and_then(|id| self.content.item(id))
            .and_then(|item| item.rfb_base_kind)
            .is_some_and(|kind| (kind.tval, kind.sval) == (23, 32));
        if poison_needle {
            // xtra1.c and cmd1.c: one blow, including after extra blows and
            // Tonberry's penalty; cmd1.c ignores every ordinary damage bonus.
            attack_sources.push(rfb_protocol::CharacterStatSourceDto {
                source_id: source_item_id.clone().unwrap(),
                amount: 100 - blows,
            });
            blows = 100;
        }
        ResolvedAttackProfile {
            poison_needle,
            attacks: u16::try_from(blows / 100).expect("derived melee attack count must fit u16"),
            extra_attack_chance_percent: u8::try_from(blows % 100)
                .expect("fractional melee blows must fit u8"),
            attack_sources,
            melee_skill,
            to_hit,
            to_damage,
            damage_dice: dice,
            damage_sides: sides,
            damage_type: DamageType::from(damage_type),
            source_item_id,
            source_mutation_id: None,
            attack_name: None,
            critical_weight_tenths_pound,
        }
    }

    pub(super) fn player_mutation_innate_attack_profiles(
        &self,
        stats: &ActorDerivedStats,
    ) -> Vec<ResolvedAttackProfile> {
        let mut innate_skill = stats.melee_skill.clone();
        let mut innate_damage = stats.melee_damage_bonus.clone();
        for item in self.equipped_melee_weapons() {
            innate_skill = derived_stat_without_source(&innate_skill, &item.id, true);
            innate_damage = derived_stat_without_source(&innate_damage, &item.id, false);
        }
        let innate_damage_bonus = innate_damage.value;
        let mut mutations = self
            .content
            .mutations()
            .filter(|mutation| {
                mutation.innate_attack.is_some()
                    && self.progress.active_mutation_ids.contains(&mutation.id)
            })
            .collect::<Vec<_>>();
        mutations.sort_by_key(|mutation| mutation.source_index);
        let mut profiles = mutations
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
                    poison_needle: false,
                    attacks: 1,
                    extra_attack_chance_percent: 0,
                    attack_sources: Vec::new(),
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
            .collect::<Vec<_>>();
        if self.player_has_draconian_metamorphosis() {
            profiles.extend(
                self.draconian_metamorphosis_attack_profiles(&innate_skill, innate_damage_bonus),
            );
            if let Some(profile) = profiles.first_mut() {
                let blows = u32::from(profile.attacks) * 100
                    + u32::from(profile.extra_attack_chance_percent)
                    + self
                        .player_equipment_bonuses()
                        .melee_attacks_delta_percent
                        .max(0) as u32;
                profile.attacks = (blows / 100) as u16;
                profile.extra_attack_chance_percent = (blows % 100) as u8;
            }
        }
        profiles
    }

    pub(super) fn player_has_draconian_metamorphosis(&self) -> bool {
        self.progress
            .active_mutation_ids
            .contains(DRACONIAN_METAMORPHOSIS_MUTATION_ID)
            && !self
                .player
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_PLAYER_POLYMORPH)
            && self
                .selected_race_definition()
                .is_some_and(|race| race.tags.iter().any(|tag| tag == "draconian"))
    }

    pub(super) fn draconian_metamorphosis_attack_level(&self) -> u16 {
        let level = self.progress.level.min(50).saturating_mul(2);
        let race_percent =
            self.selected_race_definition()
                .map_or(100, |race| match race.id.as_str() {
                    "rfb-legacy.race.draconian-red" | "rfb-legacy.race.draconian-white" => 105,
                    "rfb-legacy.race.draconian-blue" => 95,
                    "rfb-legacy.race.draconian-bronze"
                    | "rfb-legacy.race.draconian-crystal"
                    | "rfb-legacy.race.draconian-gold" => 90,
                    "rfb-legacy.race.draconian-shadow" => 85,
                    _ => 100,
                });
        let class_percent =
            self.build
                .as_ref()
                .map_or(100, |build| match build.class_id.as_str() {
                    "demo.class.warrior" => 120,
                    "demo.class.berserker" => 170,
                    "demo.class.paladin" => 110,
                    "demo.class.high-mage" | "demo.class.mage" => 80,
                    _ => 100,
                });
        level
            .saturating_mul(race_percent)
            .saturating_div(100)
            .max(1)
            .saturating_mul(class_percent)
            .saturating_div(100)
            .max(1)
    }

    fn draconian_metamorphosis_armor_class(&self) -> i32 {
        if !self.player_has_draconian_metamorphosis() {
            return 0;
        }
        let level = self.progress.level.min(50);
        15_i32
            .saturating_add(i32::from(level / 10).saturating_mul(5))
            .saturating_add(
                i32::try_from(prorated_level_value(75, level, 1, 1, 1))
                    .expect("prorated Draconian armor must fit i32"),
            )
    }

    fn draconian_metamorphosis_attack_profiles(
        &self,
        innate_skill: &DerivedStat,
        innate_damage_bonus: i32,
    ) -> Vec<ResolvedAttackProfile> {
        let level = self.progress.level.min(50);
        let attack_level = self.draconian_metamorphosis_attack_level();
        let to_hit = i32::from(level.saturating_mul(3) / 5);
        let skill = innate_skill.with_modifier(
            StatLayer::Status,
            DRACONIAN_METAMORPHOSIS_MUTATION_ID,
            to_hit,
            StatBounds::NON_NEGATIVE,
        );
        let critical_to_hit = skill
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
        let profile = |name: &str,
                       blows: u16,
                       damage_dice: u16,
                       damage_sides: u16,
                       weight_tenths_pound: u16| {
            ResolvedAttackProfile {
                poison_needle: false,
                attacks: blows / 100,
                extra_attack_chance_percent: u8::try_from(blows % 100)
                    .expect("fractional Draconian blows must fit u8"),
                attack_sources: Vec::new(),
                melee_skill: skill.clone(),
                to_hit: critical_to_hit,
                to_damage: innate_damage_bonus,
                damage_dice,
                damage_sides,
                damage_type: DamageType::Physical,
                source_item_id: None,
                source_mutation_id: Some(DRACONIAN_METAMORPHOSIS_MUTATION_ID.to_owned()),
                attack_name: Some(name.to_owned()),
                critical_weight_tenths_pound: Some(weight_tenths_pound),
            }
        };
        let claw_weight = 100_u16.saturating_add(attack_level);
        let claw_blows =
            draconian_innate_blows(self.effective_player_attributes(), claw_weight, 400);
        let bite_maximum = match attack_level {
            175.. => 400,
            160.. => 300,
            135.. => 250,
            85.. => 200,
            70.. => 150,
            _ => 100,
        };
        let bite_weight = 200_u16.saturating_add(attack_level.saturating_mul(2));
        let bite_blows = draconian_innate_blows(
            self.effective_player_attributes(),
            bite_weight,
            bite_maximum,
        );
        vec![
            profile(
                "爪击",
                claw_blows,
                1 + attack_level / 15,
                3 + level / 16,
                claw_weight,
            ),
            profile(
                "撕咬",
                bite_blows,
                1 + level / 10,
                4 + attack_level / 6,
                bite_weight,
            ),
        ]
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
            if matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) == Some("ring") && !self.ring_affects_weapon(slot_id, profile.source_item_id.as_deref()))
            {
                continue;
            }
            if let Some(item_definition) = self.content.item(&item.kind_id) {
                if item_definition.melee_profile.is_some()
                    && profile.source_item_id.as_deref() != Some(&item.id)
                {
                    continue;
                }
                apply(&item_definition.slays, &item_definition.brands);
            }
            for affix_id in &item.affix_ids {
                if let Some(affix) = self.content.affix(affix_id) {
                    apply(&affix.slays, &affix.brands);
                }
            }
            apply(
                &item.intrinsic_properties.slays,
                &item.intrinsic_properties.brands,
            );
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

    pub(super) fn player_projectile_damage_multiplier(
        &self,
        profile: &ResolvedProjectileProfile,
        target: &Actor,
        definition: &rfb_content::ActorDefinition,
    ) -> i32 {
        let mut multiplier = 10;
        for (slay_target, level) in &profile.ammunition_slays {
            if slay_target_matches(*slay_target, definition) {
                multiplier = multiplier.max(slay_multiplier(*slay_target, *level));
            }
        }
        for brand in &profile.ammunition_brands {
            if target.resistances.level(brand_damage_type(*brand)) != ResistanceLevel::Immune {
                multiplier = multiplier.max(24);
            }
        }
        multiplier
    }

    fn add_character_stat_contributions(&self, pipeline: &mut DerivedStatsPipeline) {
        let Some((_, race, class, personality)) = self.character_definitions() else {
            return;
        };
        let level_scaling = |stat| {
            race.level_stat_scalings
                .iter()
                .filter(|scaling| scaling.stat == stat)
                .fold(0_i32, |total, scaling| {
                    total.saturating_add(
                        i32::from(self.progress.level).saturating_mul(scaling.multiplier)
                            / i32::from(scaling.divisor),
                    )
                })
        };
        let mut speed = level_scaling(rfb_content::RaceLevelStatDefinition::Speed);
        if race.id == "rfb-legacy.race.tonberry" {
            // RFB master a0d92b6378: races_k.c::_tonberry_calc_bonuses.
            // Include the base -1 as well as the four level thresholds for this form.
            speed = speed.saturating_add(race.modifiers.speed);
            for threshold in [30, 40, 45, 50] {
                speed -= i32::from(self.progress.level >= threshold);
            }
        }
        add_nonzero_stat(
            pipeline,
            StatKind::Speed,
            StatLayer::Species,
            &race.id,
            speed,
        );
        add_nonzero_stat(
            pipeline,
            StatKind::ArmorClass,
            StatLayer::Species,
            &race.id,
            race.armor_class.saturating_add(level_scaling(
                rfb_content::RaceLevelStatDefinition::ArmorClass,
            )),
        );
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
        if self.player_is_duelist() {
            let x = i32::from(
                self.effective_player_attributes()
                    .index(AttributeKind::Intelligence),
            ) + 3;
            let bonus = if self.duelist_equipment_error().is_none() {
                x / 2 + x * i32::from(self.progress.level) / 50
            } else {
                0
            };
            add_nonzero_stat(
                pipeline,
                StatKind::ArmorClass,
                StatLayer::Class,
                &class.id,
                -50 + bonus,
            );
        }
        if self.player_is_berserker() {
            let level = i32::from(self.progress.level);
            for (kind, amount) in [
                (
                    StatKind::Speed,
                    2 + [30, 40, 45, 50]
                        .into_iter()
                        .filter(|threshold| level >= *threshold)
                        .count() as i32,
                ),
                (StatKind::ArmorClass, 10 + level / 2),
                (StatKind::DigSkill, 100 + 8 * level),
            ] {
                add_nonzero_stat(pipeline, kind, StatLayer::Class, &class.id, amount);
            }
            // Permanent shero uses the ordinary combat modifiers, without its +30 HP.
            for (kind, amount) in [
                (StatKind::ArmorClass, -10),
                (StatKind::MeleeSkill, 12),
                (StatKind::MeleeDamageBonus, 3 + level / 5),
                (StatKind::RangedSkill, -12),
                (StatKind::ThrowingSkill, -20),
                (StatKind::DeviceSkill, -20),
                (StatKind::SavingThrowSkill, -30),
                (StatKind::StealthSkill, -7),
                (StatKind::SearchSkill, -15),
                (StatKind::PerceptionSkill, -15),
                (StatKind::DigSkill, 30),
            ] {
                add_nonzero_stat(pipeline, kind, StatLayer::Class, &class.id, amount);
            }
        }
        if race.id == "rfb-legacy.race.ent"
            && !self.items.iter().any(|item| {
                matches!(item.location, ItemLocation::Equipped { .. })
                    && self
                        .content
                        .item(&item.kind_id)
                        .is_some_and(|definition| definition.melee_profile.is_some())
            })
        {
            // RFB's object_is_melee_weapon includes equipped digging tools.
            add_nonzero_stat(
                pipeline,
                StatKind::DigSkill,
                StatLayer::Species,
                &race.id,
                i32::from(self.progress.level) * 10,
            );
        }
        let headgear_excess = self.player_tomte_headgear_excess_weight();
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
                        let amount = if layer == StatLayer::Species && headgear_excess > 0 {
                            amount - (i32::from(headgear_excess) + 6) / 2
                        } else {
                            amount
                        };
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
        let monster_level = if definition.role == ActorRole::Monster {
            u32::try_from(
                scale_actor_power(
                    i32::try_from(definition.level).unwrap_or(i32::MAX),
                    actor.power_per_mille,
                )
                .max(1),
            )
            .unwrap_or(u32::MAX)
        } else {
            definition.level
        };
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
        if actor.minor_slow > 0 {
            let has_slow = actor
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_SLOW);
            let penalty = if has_slow {
                actor.minor_slow / 4
            } else {
                actor.minor_slow
            };
            if penalty > 0 {
                pipeline.add(
                    StatKind::Speed,
                    StatLayer::Status,
                    "rfb.status.monster-minor-slow",
                    -i32::from(penalty),
                );
            }
        }
        if include_equipment
            && let Some(mount_id) = self.riding_actor_id.as_deref()
            && let Some(mount) = self.entities.iter().find(|entity| entity.id == mount_id)
        {
            let mounted_speed = riding_proficiency::mounted_speed(
                mount.speed,
                self.progress.riding_proficiency,
                self.progress.level,
            );
            pipeline.add(
                StatKind::Speed,
                StatLayer::Environment,
                &mount.id,
                mounted_speed.saturating_sub(i32::from(actor.speed)),
            );
        }
        pipeline.add(
            StatKind::MeleeSkill,
            StatLayer::Base,
            base_source,
            if definition.role == ActorRole::Monster {
                monster_melee_skill(definition.attack, monster_level)
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
            if include_equipment {
                i32::from(crate::stats::strength_digging_bonus(
                    self.effective_player_attributes().strength,
                ))
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
                    (
                        StatKind::MaxHp,
                        modifiers.max_hp.saturating_add(
                            mutation
                                .max_hp_per_level
                                .saturating_mul(i32::from(self.progress.level)),
                        ),
                    ),
                    (StatKind::Attack, modifiers.attack),
                    (StatKind::Defense, modifiers.defense),
                    (
                        StatKind::MeleeSkill,
                        rating_to_combat_value(modifiers.attack),
                    ),
                    (
                        StatKind::ArmorClass,
                        rating_to_armor_class(modifiers.defense)
                            .saturating_add(
                                self.birth_race_mutation_override(&mutation.id)
                                    .and_then(|override_| override_.armor_class)
                                    .unwrap_or(mutation.armor_class),
                            )
                            .saturating_add(
                                if mutation.id == DRACONIAN_METAMORPHOSIS_MUTATION_ID {
                                    self.draconian_metamorphosis_armor_class()
                                } else {
                                    0
                                },
                            ),
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
                    (StatKind::DeviceSkill, mutation.device_skill),
                    (StatKind::MeleeSkill, mutation.melee_skill),
                    (StatKind::RangedSkill, mutation.ranged_skill),
                    (StatKind::StealthSkill, mutation.stealth_skill),
                    (StatKind::SearchSkill, mutation.search_skill),
                    (StatKind::PerceptionSkill, mutation.perception_skill),
                ] {
                    add_nonzero_stat(&mut pipeline, kind, StatLayer::Status, &mutation.id, value);
                }
            }
            let mut digging_equipment = ("rfb.digging-equipment".to_owned(), 0);
            for item in self
                .items
                .iter()
                .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            {
                let slot_type = match &item.location {
                    ItemLocation::Equipped { slot_id } => self.body_slot_type(slot_id),
                    _ => None,
                };
                if matches!(slot_type, Some("weapon" | "tool"))
                    && let Some(definition) = self.content.item(&item.kind_id)
                {
                    let tunneling = if item.artifact_name.is_some() {
                        0
                    } else {
                        definition.tunneling_pval
                    };
                    let bonus = i32::from(self.item_instance_weight(item) / 10)
                        .saturating_add(i32::from(tunneling).saturating_mul(20));
                    if bonus > digging_equipment.1 {
                        digging_equipment = (item.id.clone(), bonus);
                    }
                }
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::DigSkill,
                    &item.id,
                    self.item_equipment_bonuses(item).digging_skill,
                );
                if slot_type == Some("tool") {
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
                let (armor_to_hit, armor_to_damage) = self.armor_combat_enchantments(item, false);
                add_equipment_stat(&mut pipeline, StatKind::MeleeSkill, &item.id, armor_to_hit);
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeDamageBonus,
                    &item.id,
                    armor_to_damage,
                );
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
                let melee_profile = match &item.location {
                    ItemLocation::Equipped { slot_id }
                        if matches!(self.body_slot_type(slot_id), Some("weapon" | "shield")) =>
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
            add_equipment_stat(
                &mut pipeline,
                StatKind::DigSkill,
                &digging_equipment.0,
                digging_equipment.1,
            );

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
            if include_equipment && self.player_is_berserker() && status.kind_id == STATUS_BERSERK {
                continue;
            }
            let mut modifiers = status.granted_modifiers;
            if include_equipment
                && status.kind_id == STATUS_MAGIC_ARMOR
                && self.player_has_status_kind("rfb.status.stone-skin")
            {
                modifiers.defense = 0;
            }
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

        let max_hp = pipeline.resolve(StatKind::MaxHp, StatBounds::UNBOUNDED);
        let max_hp = if include_equipment {
            let scaled =
                apply_equipment_life_percent(max_hp.value, self.player_equipment_life_percent());
            max_hp.with_modifier(
                StatLayer::Equipment,
                "rfb.equipment.life",
                scaled.saturating_sub(max_hp.value),
                StatBounds::NON_NEGATIVE,
            )
        } else {
            max_hp
        };
        let speed = pipeline.resolve(StatKind::Speed, StatBounds::ACTOR_SPEED);
        let speed = if include_equipment
            && self.riding_actor_id.is_none()
            && self.player_has_status_kind(STATUS_LIGHT_SPEED)
        {
            speed.with_modifier(
                StatLayer::Status,
                STATUS_LIGHT_SPEED,
                199_i32.saturating_sub(speed.value),
                StatBounds::ACTOR_SPEED,
            )
        } else {
            speed
        };
        let skill_bounds = if include_equipment && self.player_is_berserker() {
            StatBounds::UNBOUNDED
        } else {
            StatBounds::NON_NEGATIVE
        };
        let armor_bounds = if include_equipment && self.player_is_duelist() {
            StatBounds::UNBOUNDED
        } else {
            StatBounds::NON_NEGATIVE
        };
        let saving_throw_skill = pipeline.resolve(StatKind::SavingThrowSkill, skill_bounds);
        let saving_throw_skill = if include_equipment
            && (self.player_has_status_kind(STATUS_MAGIC_RESISTANCE)
                || self.player_has_status_kind(STATUS_MAGIC_ARMOR))
        {
            let minimum = 95_i32.saturating_add(i32::from(self.progress.level));
            saving_throw_skill.with_modifier(
                StatLayer::Status,
                STATUS_MAGIC_RESISTANCE,
                minimum.saturating_sub(saving_throw_skill.value).max(0),
                StatBounds::NON_NEGATIVE,
            )
        } else {
            saving_throw_skill
        };
        let saving_throw_skill = if let Some((status, value)) =
            actor.statuses.iter().find_map(|status| {
                status
                    .granted_equipment_bonuses
                    .saving_throw_skill_override
                    .map(|value| (status, value))
            }) {
            saving_throw_skill.with_modifier(
                StatLayer::Status,
                &status.kind_id,
                value.saturating_sub(saving_throw_skill.value),
                StatBounds::NON_NEGATIVE,
            )
        } else {
            saving_throw_skill
        };
        let stealth_skill = pipeline.resolve(StatKind::StealthSkill, StatBounds::NON_NEGATIVE);
        let stealth_skill = if include_equipment && self.player_has_equipped_aggravation() {
            if let Some(race_id) = self.player_fairy_stealth_race_id() {
                let reduced = stealth_skill
                    .value
                    .saturating_sub(3)
                    .min(stealth_skill.value.saturating_add(2) / 2);
                stealth_skill.with_modifier(
                    StatLayer::Species,
                    race_id,
                    reduced.saturating_sub(stealth_skill.value),
                    StatBounds::NON_NEGATIVE,
                )
            } else {
                stealth_skill
            }
        } else {
            stealth_skill
        };
        let stealth_skill = if include_equipment
            && self.player_has_equipped_curse_effect(ItemCurseEffectDto::Catlike)
        {
            stealth_skill.with_modifier(
                StatLayer::Equipment,
                "equipment.curse.catlike",
                -4,
                StatBounds::NON_NEGATIVE,
            )
        } else {
            stealth_skill
        };
        ActorDerivedStats {
            max_hp: if include_equipment {
                apply_player_life_force(max_hp, self.progress.life_force)
            } else {
                max_hp
            },
            attack: apply_monster_power(
                pipeline.resolve(StatKind::Attack, StatBounds::NON_NEGATIVE),
                actor,
                definition,
                StatBounds::NON_NEGATIVE,
            ),
            defense: apply_monster_power(
                pipeline.resolve(StatKind::Defense, StatBounds::NON_NEGATIVE),
                actor,
                definition,
                StatBounds::NON_NEGATIVE,
            ),
            speed,
            melee_skill: pipeline.resolve(StatKind::MeleeSkill, StatBounds::NON_NEGATIVE),
            armor_class: apply_monster_power(
                pipeline.resolve(StatKind::ArmorClass, armor_bounds),
                actor,
                definition,
                armor_bounds,
            ),
            melee_attacks: pipeline.resolve(StatKind::MeleeAttacks, StatBounds::NON_NEGATIVE),
            melee_damage_bonus: pipeline.resolve(StatKind::MeleeDamageBonus, StatBounds::UNBOUNDED),
            ranged_skill: pipeline.resolve(StatKind::RangedSkill, skill_bounds),
            throwing_skill: pipeline.resolve(StatKind::ThrowingSkill, skill_bounds),
            door_skill: pipeline.resolve(StatKind::DoorSkill, skill_bounds),
            bash_power: pipeline.resolve(StatKind::BashPower, StatBounds::NON_NEGATIVE),
            search_skill: pipeline.resolve(StatKind::SearchSkill, skill_bounds),
            device_skill: {
                let skill = pipeline.resolve(StatKind::DeviceSkill, skill_bounds);
                let penalty = if include_equipment {
                    self.items
                        .iter()
                        .map(|item| {
                            self.equipped_curse_penalty(item, ItemCurseEffectDto::LowDevice, 5, 10)
                        })
                        .max()
                        .unwrap_or(0)
                } else {
                    0
                };
                skill.with_modifier(
                    StatLayer::Equipment,
                    "equipment.curse.low-device",
                    -penalty,
                    skill_bounds,
                )
            },
            saving_throw_skill,
            stealth_skill,
            perception_skill: pipeline.resolve(StatKind::PerceptionSkill, skill_bounds),
            disarm_skill: pipeline.resolve(StatKind::DisarmSkill, skill_bounds),
            dig_skill: pipeline.resolve(StatKind::DigSkill, StatBounds::NON_NEGATIVE),
        }
    }
}

pub(super) fn apply_equipment_life_percent(max_hp: i32, life_percent: i32) -> i32 {
    let multiplier = 100_i32.saturating_add(life_percent).max(1);
    i32::try_from(
        i64::from(max_hp)
            .saturating_mul(i64::from(multiplier))
            .saturating_div(100),
    )
    .unwrap_or(if max_hp.is_negative() {
        i32::MIN
    } else {
        i32::MAX
    })
    .max(1)
}

// SPDX-License-Identifier: MPL-2.0

use super::player_stats::{ActorDerivedStats, temporary_sustain_passive};
use super::*;
use rfb_protocol::{
    CharacterAttackTraitDto, CharacterAuraDto, CharacterCurseEffectDto, CharacterNegativeDto,
    CharacterPassiveDto, CharacterResistanceDto, CharacterStatDto, CharacterStatSourceDto,
    CharacterTraitDetailsDto, CharacterTraitSourceDto, TraitAttackScopeDto, TraitSourceKindDto,
};

fn source(kind: TraitSourceKindDto, id: &str) -> CharacterTraitSourceDto {
    CharacterTraitSourceDto {
        kind,
        source_id: id.to_owned(),
        resistances: Vec::new(),
        status_immunities: Vec::new(),
        passives: Vec::new(),
        reflects_bolts: false,
        passes_walls: false,
        life_percent: 0,
    }
}

impl Game {
    /// Read-only display projection. Totals come from rule functions, never summed by the UI.
    pub(super) fn character_trait_details(
        &self,
        stats: &ActorDerivedStats,
    ) -> CharacterTraitDetailsDto {
        use EquipmentPassiveDto as P;
        use TraitSourceKindDto as K;
        let equipped: Vec<_> = self.items.iter().filter(|item|
            matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))).collect();
        let complete = equipped.iter().all(|item| {
            self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware
                && self.item_identification(item) == ItemIdentificationDto::Identified
        });
        let mut sources = Vec::new();
        let mut base = source(K::Base, &self.player.kind_id);
        base.resistances = self.player.resistances.to_dtos();
        sources.push(base);
        if let Some((_, race, class, _)) = self.character_definitions() {
            let mut innate = source(K::Race, &race.id);
            innate
                .resistances
                .extend(race.resistances.iter().map(|(kind, level)| ResistanceDto {
                    damage_type: DamageType::from(*kind).into(),
                    level: ResistanceLevel::from(*level).into(),
                }));
            for entry in &race.level_resistances {
                if self.progress.level >= entry.minimum_level {
                    innate
                        .resistances
                        .extend(entry.resistances.iter().map(|(kind, level)| ResistanceDto {
                            damage_type: DamageType::from(*kind).into(),
                            level: ResistanceLevel::from(*level).into(),
                        }));
                }
            }
            innate
                .status_immunities
                .extend(race.status_immunities.iter().cloned());
            for sustain in &race.attribute_sustains {
                innate
                    .passives
                    .push(equipment_passive_dto(attribute_sustain_passive(
                        Self::item_attribute_kind(sustain),
                    )));
            }
            for (present, passive) in [
                (race.levitation, P::Levitation),
                (
                    race.tags.iter().any(|tag| tag == "slow-digestion"),
                    P::SlowDigestion,
                ),
                (
                    race.telepathy_minimum_level
                        .is_some_and(|level| self.progress.level >= level),
                    P::Telepathy,
                ),
                (
                    race.see_invisible
                        || race
                            .see_invisible_minimum_level
                            .is_some_and(|level| self.progress.level >= level),
                    P::SeeInvisible,
                ),
                (
                    race.hold_life_minimum_level
                        .is_some_and(|level| self.progress.level >= level),
                    P::HoldLife,
                ),
            ] {
                if present {
                    innate.passives.push(passive);
                }
            }
            innate.reflects_bolts = race
                .reflects_bolts_minimum_level
                .is_some_and(|level| self.progress.level >= level);
            sources.push(innate);
            let mut class_source = source(K::Class, &class.id);
            for entry in &class.level_resistances {
                if self.progress.level >= entry.minimum_level {
                    class_source
                        .resistances
                        .extend(entry.resistances.iter().map(|(kind, level)| ResistanceDto {
                            damage_type: DamageType::from(*kind).into(),
                            level: ResistanceLevel::from(*level).into(),
                        }));
                }
            }
            sources.push(class_source);
        }
        for mutation in self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
        {
            let mut entry = source(K::Mutation, &mutation.id);
            let resistances = self
                .birth_race_mutation_override(&mutation.id)
                .and_then(|entry| entry.resistances.as_ref())
                .unwrap_or(&mutation.resistances);
            entry
                .resistances
                .extend(resistances.iter().map(|(kind, level)| ResistanceDto {
                    damage_type: DamageType::from(*kind).into(),
                    level: ResistanceLevel::from(*level).into(),
                }));
            entry
                .status_immunities
                .extend(mutation.status_immunities.iter().cloned());
            if mutation.levitation {
                entry.passives.push(P::Levitation);
            }
            if mutation.telepathy {
                entry.passives.push(P::Telepathy);
            }
            sources.push(entry);
        }
        for status in &self.player.statuses {
            let mut entry = source(K::TemporaryEffect, &status.kind_id);
            entry
                .resistances
                .extend(
                    status
                        .granted_resistances
                        .iter()
                        .map(|(kind, level)| ResistanceDto {
                            damage_type: (*kind).into(),
                            level: (*level).into(),
                        }),
                );
            entry
                .status_immunities
                .extend(status.granted_status_immunities.iter().cloned());
            entry.passes_walls = status.grants_wall_passage;
            if let Some(passive) = temporary_sustain_passive(&status.kind_id) {
                entry.passives.push(equipment_passive_dto(passive));
            }
            let id = status.kind_id.as_str();
            if id == STATUS_ULTIMATE_RESISTANCE {
                entry.passives.extend([
                    P::SustainStrength,
                    P::SustainIntelligence,
                    P::SustainWisdom,
                    P::SustainDexterity,
                    P::SustainConstitution,
                    P::SustainCharisma,
                    P::SlowDigestion,
                ]);
                entry.reflects_bolts = true;
            }
            for (present, passive) in [
                (
                    matches!(
                        id,
                        STATUS_LEVITATION
                            | STATUS_ULTIMATE_RESISTANCE
                            | STATUS_DEMON_LORD_TRANSFORMATION
                    ),
                    P::Levitation,
                ),
                (
                    matches!(
                        id,
                        STATUS_TELEPATHY
                            | STATUS_ULTIMATE_RESISTANCE
                            | STATUS_DEMON_LORD_TRANSFORMATION
                    ),
                    P::Telepathy,
                ),
                (
                    matches!(
                        id,
                        STATUS_HOLD_LIFE
                            | STATUS_ULTIMATE_RESISTANCE
                            | STATUS_DEMON_LORD_TRANSFORMATION
                    ),
                    P::HoldLife,
                ),
                (
                    matches!(
                        id,
                        STATUS_SIGHT | STATUS_SEE_INVISIBLE | STATUS_ULTIMATE_RESISTANCE
                    ),
                    P::SeeInvisible,
                ),
            ] {
                if present && !entry.passives.contains(&passive) {
                    entry.passives.push(passive);
                }
            }
            sources.push(entry);
        }
        let mut attacks = Vec::new();
        for item in equipped {
            let mut entry = source(K::Equipment, &item.id);
            entry.resistances = self.visible_item_resistance_sources(item);
            entry.status_immunities = self.visible_item_status_immunities(item);
            entry.passives = self.visible_item_passives(item);
            entry.life_percent = self.visible_item_equipment_bonuses(item).life_percent;
            entry.reflects_bolts = self.item_knowledge_dto(&item.kind_id)
                == ItemKnowledgeDto::Aware
                && self
                    .content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.reflects_bolts);
            let slays = self.visible_item_slays(item);
            let brands = self.visible_item_brands(item);
            if !slays.is_empty() || !brands.is_empty() {
                attacks.push(CharacterAttackTraitDto {
                    source_id: item.id.clone(),
                    scope: TraitAttackScopeDto::ArmedMelee,
                    slays,
                    brands,
                    vampiric: false,
                });
            }
            if entry.passives.contains(&P::Vampiric) && self.item_melee_profile(item).is_some() {
                attacks.push(CharacterAttackTraitDto {
                    source_id: item.id.clone(),
                    scope: TraitAttackScopeDto::OwnWeapon,
                    slays: Vec::new(),
                    brands: Vec::new(),
                    vampiric: true,
                });
            }
            sources.push(entry);
        }
        for status in &self.player.statuses {
            if !status.granted_brands.is_empty() {
                attacks.push(CharacterAttackTraitDto {
                    source_id: status.kind_id.clone(),
                    scope: TraitAttackScopeDto::ArmedMelee,
                    slays: Vec::new(),
                    brands: status
                        .granted_brands
                        .iter()
                        .copied()
                        .map(weapon_brand_dto)
                        .collect(),
                    vampiric: false,
                });
            }
        }
        if let Some(profile) = self.player_projectile_profile()
            && let Some(item) = profile
                .ammo_item_id
                .as_ref()
                .and_then(|id| self.items.iter().find(|item| &item.id == id))
        {
            attacks.push(CharacterAttackTraitDto {
                source_id: item.id.clone(),
                scope: TraitAttackScopeDto::CurrentAmmunition,
                slays: self.visible_item_slays(item),
                brands: self.visible_item_brands(item),
                vampiric: false,
            });
        }
        // Save/load canonicalizes item and status order. Presentation must not depend
        // on their insertion order in memory; derived-stat pipeline order stays intact.
        sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        attacks.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        // A fixed roster prevents the presence of an unknown resistance from leaking its type.
        let resistance_types = [
            DamageType::Physical,
            DamageType::Acid,
            DamageType::Electricity,
            DamageType::Fire,
            DamageType::Cold,
            DamageType::Poison,
            DamageType::Light,
            DamageType::Dark,
            DamageType::Blindness,
            DamageType::Fear,
            DamageType::Confusion,
            DamageType::Nether,
            DamageType::Nexus,
            DamageType::Sound,
            DamageType::Shards,
            DamageType::Rock,
            DamageType::Chaos,
            DamageType::Disenchant,
            DamageType::Time,
            DamageType::Mana,
            DamageType::Gravity,
            DamageType::Inertia,
            DamageType::Plasma,
            DamageType::Force,
            DamageType::Nuke,
            DamageType::Disintegrate,
            DamageType::Storm,
            DamageType::HolyFire,
            DamageType::HellFire,
            DamageType::Ice,
            DamageType::Water,
            DamageType::Psi,
            DamageType::Curse,
            DamageType::Meteor,
            DamageType::Rocket,
            DamageType::Telekinesis,
        ];
        let resistance_profile = self.effective_player_resistances();
        let resistances = resistance_types
            .into_iter()
            .map(|kind| CharacterResistanceDto {
                damage_type: kind.into(),
                level: complete.then(|| resistance_profile.level(kind).into()),
                reduction_percent: complete.then(|| {
                    self.adjust_player_resistance_percent(kind, resistance_profile.level(kind))
                }),
            })
            .collect();
        let equipment_passives = self.player_equipment_passives();
        let mut passive_values: Vec<_> = [
            EquipmentPassive::Regeneration,
            EquipmentPassive::Warning,
            EquipmentPassive::EspAnimal,
            EquipmentPassive::EspUndead,
            EquipmentPassive::EspDemon,
            EquipmentPassive::EspOrc,
            EquipmentPassive::EspTroll,
            EquipmentPassive::EspGiant,
            EquipmentPassive::EspDragon,
            EquipmentPassive::EspHuman,
            EquipmentPassive::EspGood,
            EquipmentPassive::EspEvil,
            EquipmentPassive::EspLiving,
            EquipmentPassive::EspNonliving,
        ]
        .into_iter()
        .map(|passive| {
            (
                equipment_passive_dto(passive),
                equipment_passives.contains(&passive),
                None,
            )
        })
        .collect();
        passive_values.extend([
            (P::Levitation, self.player_levitates(), None),
            (P::Telepathy, self.player_has_telepathy(), None),
            (P::SlowDigestion, self.player_slow_digestion(), None),
            (
                P::HoldLife,
                self.player_hold_life_sources() > 0,
                Some(self.player_hold_life_sources() as u32),
            ),
            (
                P::SeeInvisible,
                self.player_see_invisible_sources() > 0,
                Some(self.player_see_invisible_sources() as u32),
            ),
        ]);
        for kind in [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ] {
            passive_values.push((
                equipment_passive_dto(attribute_sustain_passive(kind)),
                self.player_sustains_attribute(kind),
                None,
            ));
        }
        let passives = passive_values
            .into_iter()
            .map(|(passive, active, count)| CharacterPassiveDto {
                passive,
                active: if complete {
                    Some(active)
                } else {
                    sources
                        .iter()
                        .any(|entry| entry.passives.contains(&passive))
                        .then_some(true)
                },
                source_count: complete.then_some(count).flatten(),
            })
            .collect();
        let mut numeric = Vec::new();
        // Tool magic is excluded above, but an unknown tool's base tunneling value
        // can still contribute to digging. Do not expose that through the pipeline.
        let digging_complete = complete && self.items.iter().filter(|item|
            matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) == Some("tool")))
            .all(|item| self.item_knowledge_dto(&item.kind_id) == ItemKnowledgeDto::Aware);
        // Tool curses do participate in aggravation, including its racial stealth penalty.
        let stealth_complete = complete && self.items.iter().filter(|item|
            matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) == Some("tool")))
            .all(|item| self.item_identification(item) == ItemIdentificationDto::Identified);
        for (id, stat) in [
            ("speed", &stats.speed),
            ("melee-skill", &stats.melee_skill),
            ("ranged-skill", &stats.ranged_skill),
            ("throwing-skill", &stats.throwing_skill),
            ("device-skill", &stats.device_skill),
            ("saving-throw", &stats.saving_throw_skill),
            ("stealth", &stats.stealth_skill),
            ("search", &stats.search_skill),
            ("perception", &stats.perception_skill),
            ("disarm", &stats.disarm_skill),
            ("digging", &stats.dig_skill),
        ] {
            let known = complete
                && (id != "digging" || digging_complete)
                && (id != "stealth" || stealth_complete);
            numeric.push(CharacterStatDto {
                id: id.to_owned(),
                value: known.then(|| {
                    if id == "speed" {
                        i32::from(derived_speed(stat))
                    } else {
                        stat.value
                    }
                }),
                sources: if known {
                    stat.contributions
                        .iter()
                        .map(|entry| CharacterStatSourceDto {
                            source_id: entry.source_id.clone(),
                            amount: entry.amount,
                        })
                        .collect()
                } else {
                    Vec::new()
                },
            });
        }
        numeric.push(CharacterStatDto {
            id: "equipment-life".to_owned(),
            value: complete.then(|| {
                100_i32
                    .saturating_add(self.player_equipment_life_percent())
                    .max(1)
            }),
            sources: sources
                .iter()
                .filter(|entry| entry.life_percent != 0)
                .map(|entry| CharacterStatSourceDto {
                    source_id: entry.source_id.clone(),
                    amount: entry.life_percent,
                })
                .collect(),
        });
        let mut infrared_sources = Vec::new();
        let mut regeneration_sources = vec![CharacterStatSourceDto {
            source_id: "regeneration-base".to_owned(),
            amount: 100,
        }];
        if let Some((_, race, _, _)) = self.character_definitions() {
            infrared_sources.push(CharacterStatSourceDto {
                source_id: race.id.clone(),
                amount: race.infravision,
            });
            regeneration_sources.push(CharacterStatSourceDto {
                source_id: race.id.clone(),
                amount: race.regeneration_rate_modifier_percent,
            });
        }
        for mutation in self
            .content
            .mutations()
            .filter(|mutation| self.progress.active_mutation_ids.contains(&mutation.id))
        {
            infrared_sources.push(CharacterStatSourceDto {
                source_id: mutation.id.clone(),
                amount: mutation.infravision,
            });
            regeneration_sources.push(CharacterStatSourceDto {
                source_id: mutation.id.clone(),
                amount: mutation.regeneration_rate_modifier_percent,
            });
        }
        for item in self.items.iter().filter(|item| matches!(&item.location, ItemLocation::Equipped { slot_id } if self.body_slot_type(slot_id) != Some("tool"))) {
            infrared_sources.push(CharacterStatSourceDto { source_id: item.id.clone(), amount: self.visible_item_equipment_bonuses(item).infravision });
        }
        for status in &self.player.statuses {
            infrared_sources.push(CharacterStatSourceDto {
                source_id: status.kind_id.clone(),
                amount: status.granted_equipment_bonuses.infravision,
            });
        }
        if self.player_has_status_kind(STATUS_REGENERATION)
            || self.player_has_status_kind(STATUS_ULTIMATE_RESISTANCE)
        {
            regeneration_sources.push(CharacterStatSourceDto {
                source_id: "timed-regeneration".to_owned(),
                amount: 100,
            });
        }
        infrared_sources.retain(|entry| entry.amount != 0);
        regeneration_sources.retain(|entry| entry.amount != 0);
        infrared_sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        regeneration_sources.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        numeric.push(CharacterStatDto {
            id: "infravision".to_owned(),
            value: complete.then(|| self.player_infravision_range()),
            sources: infrared_sources,
        });
        // These two factors do not depend on unidentified equipment.
        numeric.push(CharacterStatDto {
            id: "natural-regeneration".to_owned(),
            value: Some(self.player_regeneration_rate_percent() as i32),
            sources: regeneration_sources,
        });
        numeric.push(CharacterStatDto {
            id: "mutation-regeneration".to_owned(),
            value: Some(self.mutation_regeneration_percent() as i32),
            sources: Vec::new(),
        });
        let melee = self.player_melee_profile(stats);
        let active_weapon_id = (!self.player_has_draconian_metamorphosis())
            .then(|| melee.source_item_id.clone())
            .flatten();
        let mut melee_profiles =
            self.player_mutation_innate_attack_profiles(stats, melee.source_item_id.as_deref());
        if !self.player_has_draconian_metamorphosis() {
            melee_profiles.insert(0, melee.clone());
        }
        let melee_damage = melee_profiles
            .into_iter()
            .map(|profile| {
                let order = profile.source_item_id.as_deref().is_some_and(|id| {
                    self.items
                        .iter()
                        .find(|item| item.id == id)
                        .is_some_and(|item| {
                            Self::item_has_weapon_trait(item, WeaponTraitDto::Order)
                        })
                });
                let maximum = i32::from(profile.damage_dice) * i32::from(profile.damage_sides);
                let minimum = if order {
                    maximum
                } else {
                    i32::from(profile.damage_dice)
                };
                rfb_protocol::MeleeDamagePreviewDto {
                    source_id: profile
                        .source_item_id
                        .or(profile.source_mutation_id)
                        .unwrap_or_else(|| self.player.kind_id.clone()),
                    attack_name: profile.attack_name,
                    damage_percent: self.player_melee_damage_percent(),
                    base_damage: complete.then(|| {
                        [
                            self.scale_player_melee_damage(
                                minimum.saturating_add(profile.to_damage),
                            ),
                            self.scale_player_melee_damage(
                                maximum.saturating_add(profile.to_damage),
                            ),
                        ]
                    }),
                }
            })
            .collect();
        if !self.player_has_draconian_metamorphosis() {
            let blows =
                i32::from(melee.attacks) * 100 + i32::from(melee.extra_attack_chance_percent);
            numeric.push(CharacterStatDto {
                id: "melee-attacks-hundredths".to_owned(),
                value: complete.then_some(blows),
                sources: if complete {
                    let mut sources: Vec<_> = stats
                        .melee_attacks
                        .contributions
                        .iter()
                        .map(|entry| CharacterStatSourceDto {
                            source_id: entry.source_id.clone(),
                            amount: entry.amount.saturating_mul(100),
                        })
                        .collect();
                    let penalty = blows - stats.melee_attacks.value.saturating_mul(100);
                    if penalty != 0 {
                        sources.push(CharacterStatSourceDto {
                            source_id: "rfb-legacy.race.tonberry".to_owned(),
                            amount: penalty,
                        });
                    }
                    sources
                } else {
                    Vec::new()
                },
            });
        }
        let projectile = self.player_projectile_profile();
        if let Some(profile) = &projectile {
            for (id, value) in [
                ("ranged-base-shot", profile.base_shot),
                ("ranged-energy", profile.energy_cost),
            ] {
                numeric.push(CharacterStatDto {
                    id: id.to_owned(),
                    value: complete.then_some(value),
                    sources: Vec::new(),
                });
            }
        }
        let mut auras = Vec::new();
        for damage_type in [DamageType::Fire, DamageType::Electricity, DamageType::Cold] {
            let mut source_ids: Vec<_> = self
                .player_elemental_contact_aura_sources(damage_type)
                .into_iter()
                .flatten()
                .collect();
            source_ids.sort();
            if !source_ids.is_empty() {
                auras.push(CharacterAuraDto {
                    damage_type: damage_type.into(),
                    source_ids,
                    evil_only: false,
                });
            }
        }
        if self.player_has_status_kind(STATUS_HOLY_AURA) {
            auras.push(CharacterAuraDto {
                damage_type: DamageType::Mana.into(),
                source_ids: vec![STATUS_HOLY_AURA.to_owned()],
                evil_only: true,
            });
        }
        let mut negatives = Vec::new();
        // Curse effects also operate in tool slots. Severity alone never implies an effect.
        for item in self
            .items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        {
            let curse = self.visible_item_curse(item);
            let knowledge = self.item_property_knowledge.get(&item.id);
            let mut effects = Vec::new();
            for effect in [ItemCurseEffectDto::Aggravate, ItemCurseEffectDto::Teleport] {
                let known = item.rolled_affixes.iter().any(|rolled| {
                    rolled.curse_effects.contains(&effect)
                        && knowledge
                            .is_some_and(|known| known.known_affix_ids.contains(&rolled.affix_id))
                });
                if known {
                    let active = (self.item_identification(item)
                        != ItemIdentificationDto::Unexamined)
                        .then(|| {
                            self.item_has_active_equipped_curse_effect(item, effect)
                                && (effect != ItemCurseEffectDto::Teleport
                                    || item.curse.is_some()
                                    || item
                                        .inscription
                                        .as_deref()
                                        .is_none_or(|inscription| !inscription.contains('.')))
                        });
                    effects.push(CharacterCurseEffectDto {
                        effect,
                        active,
                        as_stealth_penalty: effect == ItemCurseEffectDto::Aggravate
                            && self.player_fairy_stealth_race_id().is_some(),
                    });
                }
            }
            if curse.is_some() || !effects.is_empty() {
                negatives.push(CharacterNegativeDto {
                    source_id: item.id.clone(),
                    curse,
                    effects,
                });
            }
        }
        negatives.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        CharacterTraitDetailsDto {
            equipment_complete: complete,
            resistances,
            passives,
            stats: numeric,
            attacks,
            melee_damage,
            tomte_heavy_headgear: self
                .character_definitions()
                .filter(|(_, race, _, _)| race.id == "rfb-legacy.race.tomte")
                .map(|_| self.player_tomte_headgear_excess_weight() > 0),
            active_weapon_id,
            active_launcher_id: projectile.map(|profile| profile.source_item_id),
            auras,
            negatives,
            status_immunities: complete
                .then(|| self.player_status_immunities().into_iter().collect()),
            reflects_bolts: if complete {
                Some(self.player_reflects_bolts())
            } else {
                sources
                    .iter()
                    .any(|entry| entry.reflects_bolts)
                    .then_some(true)
            },
            passes_walls: self.player_can_pass_walls(),
            sources,
        }
    }
}

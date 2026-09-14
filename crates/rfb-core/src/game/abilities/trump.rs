// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, do-spell.c:214–258,508–797,4096–4760.
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::ability_scaling::spell_power_value;
use crate::game::{CategorySummonSpec, Game};
use crate::stats::{AttributeKind, CharacterProgress};
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilityTargetModeDefinition, ActorDamageType,
};
use rfb_protocol::{
    AbilityEffectResolutionDto, AbilityEffectsResolutionDto, Direction, Position, TargetSelection,
    VirtueKindDto,
};
use std::collections::BTreeSet;

impl Game {
    pub(super) fn resolve_trump_summoning(
        &mut self,
        ability: &AbilityDefinition,
        category: &str,
        mut center: Position,
        failed: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        // These two source SPELL_FAIL cases deliberately summon nothing.
        if failed && matches!(category, "phantom" | "high-dragon") {
            return;
        }
        let level = self.progress.level;
        if failed
            || (category != "kamikaze"
                && center != self.player.position
                && self.rng.bounded(3) == 0)
        {
            center = self.player.position;
        }
        let count = match category {
            "kamikaze" => 2 + self.rng.bounded(u64::from(level / 7).max(1)) as u16,
            "any-monster" => 1 + level.saturating_sub(15) / 10,
            _ => 1,
        };
        let maximum_level = if category == "phantom" {
            (level * 2 / 3 + 1 + self.rng.bounded(u64::from(level / 2).max(1)) as u16) * 3 / 2
        } else {
            let base = spell_power_value(u64::from(level), ability.spell_power_bonus);
            let sides =
                spell_power_value(u64::from(level * 2 / 3), ability.spell_power_bonus).max(1);
            (base + 1 + self.rng.bounded(sides)) as u16
        };
        let allow_unique = matches!(category, "high-undead" | "high-dragon")
            && (failed || self.rng.bounded(u64::from(50 + level)) + 1 < u64::from(level / 10));
        let category = if category == "animal" && !failed {
            "animal-ranger"
        } else {
            category
        };
        self.trump_summon_batch(
            ability,
            category,
            center,
            maximum_level,
            count,
            failed,
            allow_unique,
            matches!(category, "spider" | "hound"),
            events,
            changed,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn trump_summon_batch(
        &mut self,
        ability: &AbilityDefinition,
        category: &str,
        center: Position,
        maximum_level: u16,
        count: u16,
        hostile: bool,
        allow_unique: bool,
        groups: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let special = matches!(
            category,
            "phantom" | "kamikaze" | "high-undead" | "high-dragon"
        ) || category.starts_with("bizarre");
        for _ in 0..count {
            let candidates = self
                .summon_category_candidate_kind_ids(
                    if special { "any-monster" } else { category },
                    None,
                    maximum_level,
                    allow_unique,
                    true,
                )
                .into_iter()
                .filter(|id| {
                    let a = self.content.actor(id).expect("summon candidate");
                    match category {
                        "high-undead" => "LVW".contains(&a.glyph),
                        "high-dragon" => a.glyph == "D",
                        "phantom" => matches!(
                            a.allocation.as_ref().map(|a| a.legacy_index),
                            Some(152 | 385)
                        ),
                        "kamikaze" => a.melee_routine.as_ref().is_some_and(|r| {
                            r.blows.iter().any(|b| b.method_id == "rfb.blow.explode")
                        }),
                        "bizarre1" => a.glyph == "m",
                        "bizarre2" => a.glyph == "b",
                        "bizarre3" => a.glyph == "Q",
                        "bizarre4" => a.glyph == "v",
                        "bizarre5" => a.glyph == "$",
                        "bizarre6" => "!?=$|".contains(&a.glyph),
                        _ => true,
                    }
                })
                .collect::<Vec<_>>();
            let selected = (!candidates.is_empty())
                .then(|| candidates[self.rng.bounded(candidates.len() as u64) as usize].clone());
            let total = selected.as_ref().map_or(1, |id| {
                let a = self.content.actor(id).expect("selected summon");
                if groups && a.allocation.as_ref().is_some_and(|a| a.friends.is_some()) {
                    self.original_friend_total(&a.clone(), self.floor_depth(&self.current_floor_id))
                } else {
                    1
                }
            });
            let candidates = selected.into_iter().collect::<Vec<_>>();
            let positions = self.open_positions_around_for_actor_kinds(center, 2, &candidates);
            let owner = self.player.id.clone();
            let first_summoned = self.entities.len();
            let resolution = self.resolve_category_summon(
                CategorySummonSpec {
                    is_spell: true,
                    source_id: &ability.id,
                    owner_id: &owner,
                    category,
                    count_dice: 0,
                    count_sides: 0,
                    count_bonus: 1,
                    maximum_count: None,
                    hostile,
                    group_chance_percent: if groups { 100 } else { 0 },
                    group_count_dice: 0,
                    group_count_sides: 0,
                    group_count_bonus: total.min(255) as u8,
                    duration_turns: 0,
                },
                candidates,
                positions,
                changed,
            );
            if hostile {
                for actor in &mut self.entities[first_summoned..] {
                    actor.no_pet = true;
                }
            }
            events.push(DomainEvent::AbilitySummoned {
                ability_id: ability.id.clone(),
                resolution,
            });
        }
    }

    // The sole deferred card is Lovers. Its paid draw is retained by the existing direction state.
    pub(super) fn begin_trump_shuffle(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let mut die = if self
            .character_definitions()
            .is_some_and(|(_, _, class, _)| class.id == "demo.class.high-mage")
        {
            1 + self.rng.bounded(110) as i32 + i32::from(self.progress.level / 5)
        } else {
            1 + self.rng.bounded(120) as i32
        };
        let chance = self.virtue_current(VirtueKindDto::Chance);
        while chance != 0 && (1 + self.rng.bounded(400)) < u64::from(chance.unsigned_abs()) {
            die += i32::from(chance.signum());
        }
        if die < 30 {
            self.add_virtue(VirtueKindDto::Chance, 1);
        }
        self.resolve_trump_card(ability, die, events, changed, removed)
    }

    pub(in crate::game) fn resolve_trump_card(
        &mut self,
        ability: &AbilityDefinition,
        die: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            trace: None,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RandomChoice {
                    effect_index: 0,
                    roll: die,
                    branch_index: [
                        7, 14, 18, 22, 26, 30, 33, 38, 40, 42, 47, 52, 60, 72, 80, 82, 84, 86, 88,
                        96, 101, 111, 120,
                    ]
                    .iter()
                    .filter(|threshold| die >= **threshold)
                    .count() as u16,
                    maximum_roll: 120,
                }],
            },
        });
        use AbilityEffectDefinition as E;
        let depth = self.floor_depth(&self.current_floor_id);
        let mut effect = None;
        match die {
            ..=6 => {
                let mut i = 0;
                while i < 1 + self.rng.bounded(3) {
                    self.ty_curse_high_summon(&ability.id, depth, events, changed);
                    i += 1;
                }
            }
            7..=13 => self.trump_summon_batch(
                ability,
                "demon",
                self.player.position,
                depth,
                1,
                true,
                true,
                true,
                events,
                changed,
            ),
            14..=17 => self.resolve_equipped_ty_curse(&ability.id, events, changed, removed)?,
            18..=21 => effect = Some(E::AggravateMonsters),
            22..=25 => {
                self.resolve_item_drain_attribute(&ability.id, AttributeKind::Intelligence, events);
                self.resolve_item_drain_attribute(&ability.id, AttributeKind::Wisdom, events);
            }
            26..=29 => {
                let category = format!("bizarre{}", 1 + self.rng.bounded(6));
                self.trump_summon_batch(
                    ability,
                    &category,
                    self.player.position,
                    depth * 3 / 2,
                    1,
                    true,
                    true,
                    true,
                    events,
                    changed,
                );
            }
            30..=32 => self.trump_unlight(changed),
            33..=37 => self.trump_wild_magic(ability, events, changed, removed)?,
            38..=39 => self.trump_teleport(ability, 10, events, changed)?,
            40..=41 => {
                let mut blessing = self
                    .content
                    .ability("demo.ability.life-bless")
                    .expect("formal blessing")
                    .effect
                    .clone();
                if let E::ApplyStatus {
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    ..
                } = &mut blessing
                {
                    *duration_ticks = u32::from(self.progress.level);
                    *duration_dice = 0;
                    *duration_sides = 0;
                }
                effect = Some(blessing);
            }
            42..=46 => self.trump_teleport(ability, 100, events, changed)?,
            47..=51 => self.trump_teleport(ability, 200, events, changed)?,
            52..=59 => self.ty_curse_wall_breaker(&ability.id, events, changed, removed)?,
            60..=71 => self.trump_sleep_touch(ability, events, changed, removed)?,
            72..=79 => self.ty_curse_earthquake(&ability.id, 5, events, changed, removed)?,
            80..=87 => {
                let category = match die {
                    80..=81 => "bizarre1",
                    82..=83 => "bizarre2",
                    84..=85 => "bizarre4",
                    _ => "bizarre5",
                };
                self.trump_summon_batch(
                    ability,
                    category,
                    self.player.position,
                    depth * 3 / 2,
                    1,
                    false,
                    false,
                    false,
                    events,
                    changed,
                );
            }
            88..=95 => return Ok(true),
            96..=100 => {
                effect = Some(E::CreateAdjacentTerrain {
                    source_terrain_ids: vec!["demo.terrain.floor".into()],
                    target_terrain_id: "demo.terrain.wall".into(),
                })
            }
            101..=110 => {
                let previous = self.effective_player_max_hp();
                let resources = self.player_resource_maxima();
                let base = self.progress.hp_progression[0];
                self.progress.hp_progression =
                    CharacterProgress::roll_hp_progression(base, &mut self.rng);
                self.refresh_after_attribute_change(previous, &resources);
                self.lose_all_unlocked_mutations(events);
            }
            111..=119 => {
                effect = Some(E::Clairvoyance {
                    telepathy_duration_ticks: 0,
                    telepathy_duration_dice: 0,
                    telepathy_duration_sides: 0,
                    grants_virtues: true,
                    grants_telepathy: false,
                })
            }
            _ => {
                self.apply_player_experience((self.progress.experience / 25 + 1).min(5000), events)
            }
        }
        if let Some(effect) = effect {
            self.trump_self_effect(ability, effect, events, changed, removed)?;
        }
        Ok(false)
    }

    fn trump_self_effect(
        &mut self,
        ability: &AbilityDefinition,
        effect: AbilityEffectDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut branch = ability.clone();
        branch.effect = effect;
        let plan = self
            .ability_target_plan(&branch, &TargetSelection::SelfTarget)
            .expect("generated self card has a valid target");
        self.resolve_player_ability_effect(branch, plan, events, changed, removed)?;
        Ok(())
    }

    fn trump_teleport(
        &mut self,
        ability: &AbilityDefinition,
        range: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        self.resolve_player_teleport_with_range(
            &ability.id,
            range,
            false,
            true,
            None,
            events,
            changed,
        );
        Ok(())
    }

    fn trump_unlight(&mut self, changed: &mut BTreeSet<Position>) {
        let positions = self
            .area_damage_cells(self.player.position, 3)
            .into_iter()
            .map(|(_, p)| p)
            .chain(self.connected_glow_positions(self.player.position))
            .collect::<BTreeSet<_>>();
        for position in positions {
            if self.set_floor_glow_at(position, false) {
                changed.insert(position);
            }
        }
        // Source GF_DARK_WEAK darkens terrain; unlike GF_DARK it does not damage monsters.
    }

    fn trump_sleep_touch(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        self.trump_self_effect(
            ability,
            AbilityEffectDefinition::Sanctuary {
                power: self.progress.level,
                radius: 1,
            },
            events,
            changed,
            removed,
        )
    }

    pub(super) fn trump_lovers(
        &mut self,
        ability: &AbilityDefinition,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut branch = ability.clone();
        branch.effect = AbilityEffectDefinition::Control {
            category: "any-monster".into(),
            power: self.progress.level.min(20),
        };
        branch.target.modes = vec![AbilityTargetModeDefinition::Direction];
        branch.target.range = 18;
        branch.target.requires_line_of_effect = true;
        let plan = self
            .ability_target_plan(&branch, &TargetSelection::Direction { direction })
            .expect("valid paid direction");
        self.resolve_player_ability_effect(branch, plan, events, changed, removed)?;
        Ok(())
    }

    fn trump_wild_magic(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let spell = self.rng.bounded(32);
        let category = format!("bizarre{}", 1 + self.rng.bounded(6));
        let roll = 1 + self.rng.bounded(spell.max(1)) + 1 + self.rng.bounded(8) + 1;
        self.resolve_trump_wild_card(ability, spell, &category, roll, events, changed, removed)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::game) fn resolve_trump_wild_card(
        &mut self,
        ability: &AbilityDefinition,
        spell: u64,
        category: &str,
        roll: u64,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        use AbilityEffectDefinition as E;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            trace: None,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RandomChoice {
                    effect_index: 1,
                    roll: roll as i32,
                    branch_index: roll as u16,
                    maximum_roll: 40,
                }],
            },
        });
        let mut effect = None;
        match roll {
            1..=3 => self.trump_teleport(ability, 10, events, changed)?,
            4..=6 => self.trump_teleport(ability, 100, events, changed)?,
            7..=8 => self.trump_teleport(ability, 200, events, changed)?,
            9..=11 => self.trump_unlight(changed),
            12..=14 => {
                effect = Some(E::LightArea {
                    damage_dice: 2,
                    damage_sides: 3,
                    radius: 2,
                    sunlight_burn_damage_dice: 0,
                    sunlight_burn_damage_sides: 0,
                })
            }
            15 => effect = Some(E::DestroyAdjacentTrapsAndDoors),
            16..=18 => {
                if roll != 18 {
                    self.ty_curse_wall_breaker(&ability.id, events, changed, removed)?;
                }
                self.trump_sleep_touch(ability, events, changed, removed)?;
            }
            19..=22 => {
                effect = Some(E::CreateAdjacentTerrain {
                    source_terrain_ids: vec!["demo.terrain.floor".into()],
                    target_terrain_id: if roll < 21 {
                        "demo.terrain.created-trap"
                    } else {
                        "demo.terrain.door-closed"
                    }
                    .into(),
                })
            }
            23..=25 => effect = Some(E::AggravateMonsters),
            26 => self.ty_curse_earthquake(&ability.id, 5, events, changed, removed)?,
            27..=28 => {
                self.gain_random_mutation(events);
            }
            29..=30 => self.resolve_player_disenchantment(),
            31 => {
                self.clear_current_floor_memory(changed);
                for k in self.item_property_knowledge.values_mut() {
                    k.appraised = false;
                    k.identified = false;
                    k.known_affix_ids.clear();
                    k.known_blessed = false;
                }
            }
            32 => {
                effect = Some(E::AreaDamage {
                    damage_dice: 0,
                    damage_sides: 0,
                    damage_bonus: (spell + 5) as u16,
                    damage_type: ActorDamageType::Chaos,
                    radius: (1 + spell / 10) as u8,
                    target_category: None,
                })
            }
            33 => {
                effect = Some(E::CreateAdjacentTerrain {
                    source_terrain_ids: vec!["demo.terrain.floor".into()],
                    target_terrain_id: "demo.terrain.wall".into(),
                })
            }
            34..=35 => self.trump_summon_batch(
                ability,
                category,
                self.player.position,
                self.floor_depth(&self.current_floor_id) * 3 / 2,
                8,
                true,
                false,
                true,
                events,
                changed,
            ),
            36..=37 => {
                self.ty_curse_high_summon(
                    &ability.id,
                    self.floor_depth(&self.current_floor_id),
                    events,
                    changed,
                );
            }
            38 => self.trump_summon_batch(
                ability,
                "cyber",
                self.player.position,
                100,
                1,
                true,
                false,
                false,
                events,
                changed,
            ),
            _ => self.resolve_equipped_ty_curse(&ability.id, events, changed, removed)?,
        }
        if let Some(effect) = effect {
            self.trump_self_effect(ability, effect, events, changed, removed)?;
        }
        Ok(())
    }
}

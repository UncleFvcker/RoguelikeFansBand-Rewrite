// SPDX-License-Identifier: MPL-2.0
// do-spell.c / spells2.c / spells3.c, master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c.
use super::AbilityTargetPlan;
use crate::effect::{DamagePacket, STATUS_PLAYER_POLYMORPH, STATUS_SLEEP, resolve_damage};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::Game;
use crate::game::ability_scaling::spell_power_value;
use crate::game::damage::{FatalityPolicy, scale_damage_outcome};
use crate::game::projectile_geometry::{has_line_of_effect, rfb_distance};
use crate::resistance::{DamageType, ResistanceLevel};
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilityStatusStackingDefinition,
    CHAOS_POLYMORPH_RACES,
};
use rfb_protocol::{
    AbilityEffectResolutionDto, AbilityEffectsResolutionDto, Direction, Position, TargetSelection,
};
use std::collections::BTreeSet;

const DIRECTIONS: [Direction; 8] = [
    Direction::SouthWest,
    Direction::South,
    Direction::SouthEast,
    Direction::West,
    Direction::East,
    Direction::NorthWest,
    Direction::North,
    Direction::NorthEast,
];

// Preserve all 31 source entries, including the separate MISSILE and ARROW draws.
// Both use the existing physical projection representation.
pub(super) const CALL_CHAOS_TYPES: [DamageType; 31] = [
    DamageType::Electricity,
    DamageType::Poison,
    DamageType::Acid,
    DamageType::Cold,
    DamageType::Fire,
    DamageType::Physical,
    DamageType::Physical,
    DamageType::Plasma,
    DamageType::HolyFire,
    DamageType::Water,
    DamageType::Light,
    DamageType::Dark,
    DamageType::Force,
    DamageType::Inertia,
    DamageType::Mana,
    DamageType::Meteor,
    DamageType::Ice,
    DamageType::Chaos,
    DamageType::Nether,
    DamageType::Disenchant,
    DamageType::Shards,
    DamageType::Sound,
    DamageType::Nexus,
    DamageType::Confusion,
    DamageType::Time,
    DamageType::Gravity,
    DamageType::Rocket,
    DamageType::Nuke,
    DamageType::HellFire,
    DamageType::Disintegrate,
    DamageType::PsySpear,
];

impl Game {
    pub(super) fn resolve_chaos_meteor_swarm(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let level = self.progress.level;
        let profile = self
            .casting_profile()
            .expect("meteor spell has a casting profile");
        let bonus = profile.spell_damage_bonus_base
            + profile.spell_damage_bonus_per_level
                * (level / u16::from(profile.spell_damage_bonus_level_divisor))
            + self.armor_spell_damage_bonus();
        let damage =
            spell_power_value(u64::from(2 * level + bonus), ability.spell_power_bonus) as i32;
        let origin = self.player.position;
        let count = 11 + self.rng.bounded(10);
        for _ in 0..count {
            if self.player_is_dead() {
                break;
            }
            for _ in 0..21 {
                let position = Position {
                    x: origin.x - 8 + self.rng.bounded(17) as i32,
                    y: origin.y - 8 + self.rng.bounded(17) as i32,
                };
                if rfb_distance(origin, position) >= 9
                    || !self.projectile_can_cross(position)
                    || !has_line_of_effect(self, origin, position)
                {
                    continue;
                }
                // PROJECT_JUMP | PROJECT_KILL | PROJECT_ITEM: no terrain projection.
                self.resolve_player_area_damage_with_base_policy(
                    &ability.id,
                    vec![position],
                    false,
                    DamageType::Meteor,
                    2,
                    None,
                    damage,
                    true,
                    false,
                    false,
                    events,
                    changed,
                    removed,
                )?;
                break;
            }
        }
        Ok(())
    }

    // Return a compact server-owned draw only for the branch that needs a direction.
    // 1..=62 encodes (source type index * 2 + beam flag + 1); no client chooses it.
    pub(super) fn begin_call_chaos(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<u8>, CoreError> {
        let index = self.rng.bounded(31) as usize;
        let beam = self.rng.bounded(4) == 0;
        let roll = 1 + 2 * index as u8 + u8::from(beam);
        if self.rng.bounded(6) == 0 {
            for direction in DIRECTIONS {
                if self.player_is_dead() {
                    break;
                }
                self.call_chaos_projection(
                    ability, roll, direction, 150, 2, events, changed, removed,
                )?;
            }
        } else if self.rng.bounded(3) == 0 {
            self.resolve_player_area_damage_with_base(
                &ability.id,
                Vec::new(),
                false,
                CALL_CHAOS_TYPES[index],
                8,
                None,
                500,
                true,
                events,
                changed,
                removed,
            )?;
        } else {
            return Ok(Some(roll));
        }
        Ok(None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn call_chaos_projection(
        &mut self,
        ability: &AbilityDefinition,
        roll: u8,
        direction: Direction,
        damage: i32,
        radius: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let draw = roll - 1;
        let damage_type = CALL_CHAOS_TYPES[usize::from(draw / 2)];
        let path = self
            .projectile_path(&TargetSelection::Direction { direction }, 8)
            .unwrap();
        if draw % 2 == 1 {
            self.resolve_player_beam_damage_with_base(
                &ability.id,
                path,
                damage_type,
                damage,
                true,
                events,
                changed,
                removed,
            )
        } else {
            self.resolve_player_area_damage_with_base(
                &ability.id,
                path,
                true,
                damage_type,
                radius,
                None,
                damage,
                true,
                events,
                changed,
                removed,
            )
        }
    }

    pub(super) fn resolve_chaos_polymorph(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let birth_id = self
            .build
            .as_ref()
            .expect("book caster has a build")
            .race_id
            .clone();
        let index = match self.rng.bounded(50) + 1 {
            1 => {
                if self.rng.bounded(10) == 0 {
                    1001
                } else {
                    1000
                }
            }
            2 => 1002,
            _ => loop {
                let index = self.rng.bounded(75) as u16;
                if CHAOS_POLYMORPH_RACES
                    .iter()
                    .any(|(i, id)| *i == index && *id != birth_id)
                {
                    break index;
                }
            },
        };
        let race_id = CHAOS_POLYMORPH_RACES
            .iter()
            .find(|(i, _)| *i == index)
            .unwrap()
            .1;
        let duration = 51 + self.rng.bounded(50) as u16;
        // set_mimic checks the current race, after the selection and duration draws.
        if self.player_is_dead()
            || self
                .character_definitions()
                .is_some_and(|(_, race, _, _)| matches!(race.legacy_index, Some(36 | 65 | 69 | 72)))
        {
            return Ok(());
        }
        let remaining = self
            .player
            .statuses
            .iter()
            .filter(|s| s.granted_race_id.as_deref() == Some(race_id))
            .map(|s| s.remaining_ticks)
            .max()
            .unwrap_or(0);
        // One active mimic form; do not leave a demon status taking precedence over the new race.
        self.player.statuses.retain(|s| s.granted_race_id.is_none());
        let mut transformed = ability.clone();
        transformed.effect = AbilityEffectDefinition::ApplyStatus {
            status_kind_id: match index {
                1000 => "rfb.status.demon-transformation",
                1001 => crate::effect::STATUS_DEMON_LORD_TRANSFORMATION,
                1002 => "rfb.status.vampiric-transformation",
                _ => STATUS_PLAYER_POLYMORPH,
            }
            .into(),
            intensity: 1,
            duration_ticks: u32::from(duration).max(remaining),
            duration_dice: 0,
            duration_sides: 0,
            stacking: AbilityStatusStackingDefinition::Replace,
            resistance_type: None,
            power: None,
            granted_resistances: Default::default(),
            granted_brands: Default::default(),
            granted_modifiers: Default::default(),
            granted_equipment_bonuses: Default::default(),
            granted_status_immunities: Default::default(),
            granted_race_id: Some(race_id.into()),
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        };
        self.resolve_player_ability_effect(
            transformed,
            AbilityTargetPlan::SelfTarget,
            events,
            changed,
            &mut Vec::new(),
        )?;
        self.reconcile_player_body_slots_for_current_form();
        self.refresh_player_resource_maxima();
        self.clamp_player_hp_to_effective_max();
        Ok(())
    }

    pub(super) fn resolve_call_void(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let origin = self.player.position;
        let open = (-1..=1).all(|dy| {
            (-1..=1).all(|dx| {
                self.projectile_can_cross(Position {
                    x: origin.x + dx,
                    y: origin.y + dy,
                })
            })
        });
        if open {
            for (damage_type, radius) in [
                (DamageType::Rocket, 2),
                (DamageType::Mana, 3),
                (DamageType::Nuke, 4),
            ] {
                for direction in DIRECTIONS {
                    if self.player_is_dead() {
                        return Ok(());
                    }
                    let path = self
                        .projectile_path(&TargetSelection::Direction { direction }, 8)
                        .unwrap();
                    self.resolve_player_area_damage_with_base(
                        &ability.id,
                        path,
                        true,
                        damage_type,
                        radius,
                        None,
                        175,
                        true,
                        events,
                        changed,
                        removed,
                    )?;
                }
            }
            return Ok(());
        }
        let in_dungeon = self.content.world(&self.world_id).is_some_and(|world| {
            world
                .procedural_floors
                .iter()
                .any(|f| f.id == self.current_floor_id && f.dungeon_id.is_some())
        });
        if !in_dungeon {
            return Ok(());
        }
        let roll = self.rng.bounded(666) + 1;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            trace: None,
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RandomChoice {
                    effect_index: 0,
                    roll: roll as i32,
                    branch_index: u16::from(roll != 1),
                    maximum_roll: 666,
                }],
            },
        });
        if roll == 1 {
            if self.area_destruction_allowed() {
                self.vault_cells.fill(false);
                for actor in &mut self.entities {
                    actor
                        .statuses
                        .retain(|status| status.kind_id != STATUS_SLEEP);
                    changed.insert(actor.position);
                }
                let positions: Vec<_> = (1..self.height - 1)
                    .flat_map(|y| {
                        (1..self.width - 1).map(move |x| Position {
                            x: i32::from(x),
                            y: i32::from(y),
                        })
                    })
                    .collect();
                self.resolve_projectile_terrain_effects(
                    &positions,
                    DamageType::Disintegrate,
                    changed,
                );
            }
        } else {
            // Consume radius before checking protected terrain, as in call_the_().
            let radius = 15 + self.progress.level + self.rng.bounded(11) as u16;
            if self.area_destruction_allowed() {
                let plan = self.plan_area_destruction_with_radius(
                    radius as u8,
                    "demo.terrain.floor",
                    "demo.terrain.wall",
                    "demo.terrain.quartz-vein",
                    "demo.terrain.magma-vein",
                    Some(8 * self.progress.level),
                );
                self.apply_area_destruction_plan(plan, events, changed, removed);
            }
        }
        if !self.player_is_dead() {
            let damage = 101 + self.rng.bounded(150) as i32;
            // DAMAGE_NOESCAPE still permits invulnerability, wraith reduction and
            // Transcendence in effects.c::take_hit; use the shared damage consumers.
            let percent = self.player_spell_damage_percent(DamageType::Physical, damage);
            let outcome = scale_damage_outcome(
                resolve_damage(
                    DamagePacket::new(damage, DamageType::Physical),
                    ResistanceLevel::Normal,
                ),
                percent,
            );
            let plan = self.apply_final_player_damage(outcome, FatalityPolicy::BelowZero);
            changed.insert(self.player.position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                trace: None,
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(self.player.id.clone()),
                    target_kind_id: Some(self.player.kind_id.clone()),
                    effects: vec![AbilityEffectResolutionDto::SelfDamage {
                        effect_index: 0,
                        damage: plan.damage.applied,
                        fatal: plan.fatal,
                    }],
                },
            });
        }
        Ok(())
    }
}

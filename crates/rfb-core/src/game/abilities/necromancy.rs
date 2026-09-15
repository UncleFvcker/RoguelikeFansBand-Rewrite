// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: necromancer.c, gf.c, effects.c.
use super::AbilityTargetPlan;
use crate::effect::{STATUS_FEAR, STATUS_HASTE, STATUS_PARALYSIS, StatusStacking, apply_status};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::ability_scaling::spell_power_value;
use crate::game::monster_combat::melee_status;
use crate::game::terrain::TerrainChangeSource;
use crate::game::{Game, ItemLocation, actor_matches_category};
use crate::resistance::{DamageType, ResistanceLevel};
use crate::stats::AttributeKind;
use rfb_content::{
    AbilityDefinition, AbilityDetectSubjectDefinition, AbilityEffectDefinition as E,
    ActorDamageType as D,
};
use rfb_protocol::{Position, TargetSelection};
use std::collections::BTreeSet;

const TOUCH: &[u8] = &[0, 4, 7, 12, 13, 24, 27, 30];
const SUMMON: &[(u8, &str)] = &[
    (1, "rat"),
    (5, "bat"),
    (8, "wolf"),
    (14, "dread"),
    (16, "zombie"),
    (17, "skeleton"),
    (18, "ghost"),
    (19, "vampire"),
    (20, "wight"),
    (21, "lich"),
];

impl Game {
    pub(in crate::game) fn player_is_necromancer(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, _, c, _)| c.id == "demo.class.necromancer")
    }

    fn necromancy_can_touch(&self) -> bool {
        if self.player_has_status_kind(STATUS_FEAR) {
            return false;
        }
        let mut occupied = 0;
        for item in &self.items {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                continue;
            };
            match self.body_slot_type(slot_id) {
                Some("gloves") if !self.item_is_fixed_artifact(item, 268) => return false,
                Some("weapon" | "shield") => {
                    occupied += if self.weapon_uses_two_hands(item) {
                        2
                    } else {
                        1
                    }
                }
                _ => {}
            }
        }
        // Ordinary playable bodies have two hands; draconian metamorphosis has no weapons.
        self.player_has_draconian_metamorphosis()
            || occupied
                < self
                    .body_slots
                    .iter()
                    .filter(|s| matches!(s.slot_type.as_str(), "weapon" | "shield"))
                    .count()
    }

    pub(super) fn necromancy_target_plan(
        &self,
        ability: &AbilityDefinition,
        spell: u8,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        let mut probe = ability.clone();
        probe.effect = if SUMMON.iter().any(|(s, _)| *s == spell) {
            E::TrumpSummoning {
                category: "undead".into(),
            }
        } else if TOUCH.contains(&spell) || spell == 15 {
            if TOUCH.contains(&spell) && !self.necromancy_can_touch() {
                return None;
            }
            E::Damage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus: 0,
                damage_type: D::Cold,
            }
        } else if spell == 11 {
            E::IdentifyItem {
                full_identify_power: 0,
                full_identify_roll_sides: 0,
            }
        } else if spell == 25 {
            E::CreateCurrentTerrain {
                source_terrain_ids: vec!["demo.terrain.floor".into()],
                target_terrain_id: "demo.terrain.warding-glyph".into(),
            }
        } else {
            return matches!(target, TargetSelection::SelfTarget)
                .then_some(AbilityTargetPlan::SelfTarget);
        };
        let plan = self.ability_target_plan(&probe, target)?;
        if TOUCH.contains(&spell) {
            let AbilityTargetPlan::Projectile { ref path, .. } = plan else {
                return None;
            };
            let (_, index) = self.trace_projectile_path(path.clone());
            let actor = &self.entities[index?];
            if crate::game::chebyshev_distance(self.player.position, actor.position) != 1 {
                return None;
            }
        }
        Some(plan)
    }

    pub(super) fn necromancy_summon(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        mut center: Position,
        failed: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let Some((_, category)) = SUMMON.iter().find(|(s, _)| *s == spell) else {
            return;
        };
        let count = match spell {
            5 | 8 => 2 + self.rng.bounded(2),
            14 | 17 | 18 => 1 + self.rng.bounded(3),
            16 => 3 + self.rng.bounded(3),
            19..=21 => 1 + self.rng.bounded(2),
            _ => 1,
        };
        let count = if failed {
            (count / 4).max(1)
        } else {
            spell_power_value(count, ability.spell_power_bonus)
        };
        if failed || (center != self.player.position && self.rng.bounded(3) == 0) {
            center = self.player.position;
        }
        let level = self.progress.level;
        let power = spell_power_value(u64::from(level), ability.spell_power_bonus)
            + 1
            + self.rng.bounded(
                spell_power_value(u64::from(level * 2 / 3), ability.spell_power_bonus).max(1),
            );
        let uniques = failed || self.rng.bounded(u64::from(50 + level)) + 1 < u64::from(level / 10);
        self.trump_summon_batch(
            ability,
            category,
            center,
            power as u16,
            count as u16,
            failed,
            uniques,
            false,
            events,
            changed,
        );
    }

    fn necromancy_status(&mut self, ability: &AbilityDefinition, kind: &str, duration: u32) {
        let mut status = melee_status(kind, duration, &ability.id);
        status.stacking = StatusStacking::KeepStrongest;
        if kind == "rfb.status.necromancy-cloak" {
            status.status.granted_equipment_bonuses.stealth_skill =
                i32::from(3 + self.progress.level / 5);
        }
        if kind == "rfb.status.necromancy-shield" {
            status.status.granted_modifiers.defense = 50;
            for damage in [DamageType::Cold, DamageType::Poison, DamageType::Nether] {
                status
                    .status
                    .granted_resistances
                    .insert(damage, ResistanceLevel::Resistant);
            }
        }
        apply_status(&mut self.player.statuses, status);
    }

    pub(super) fn resolve_necromancy(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let level = self.progress.level;
        let power = |n: u16| spell_power_value(u64::from(n), ability.spell_power_bonus) as u16;
        if let AbilityTargetPlan::TrumpSummoning { center } = plan {
            self.necromancy_summon(ability, spell, center, false, events, changed);
            return Ok(None);
        }
        if TOUCH.contains(&spell) {
            return self.necromancy_touch(ability, spell, plan, events, changed, removed);
        }
        let effect = match spell {
            2 | 3 => Some(E::Detect {
                subject: AbilityDetectSubjectDefinition::Actor,
                category: if spell == 2 { "living" } else { "nonliving" }.into(),
                radius: 30,
                persistent: false,
                through_walls: false,
            }),
            6 => {
                let p = power(level * 3);
                for id in self.projected_monster_status_targets() {
                    let i = self.entities.iter().position(|a| a.id == id).unwrap();
                    let def = self
                        .actor_runtime_definition(&self.entities[i])
                        .unwrap()
                        .clone();
                    if def.tags.iter().any(|t| t == "resist-all") {
                        continue;
                    }
                    let duration = self.roll_damage(3, p / 2) + 1;
                    if def.tags.iter().any(|t| t == "unique")
                        || self.actor_has_status_immunity(i, STATUS_FEAR)
                    {
                        continue;
                    }
                    if u64::from(def.level)
                        > 11 + self.rng.bounded(u64::from(p.saturating_sub(10).max(1)))
                    {
                        continue;
                    }
                    self.apply_actor_melee_status(i, STATUS_FEAR, duration, &ability.id);
                    if u64::from(def.level)
                        <= 11 + self.rng.bounded(u64::from(p.saturating_sub(10).max(1)))
                    {
                        let duration = 1 + self.rng.bounded(3) as i32;
                        self.apply_actor_melee_status(i, STATUS_PARALYSIS, duration, &ability.id);
                    }
                    changed.insert(self.entities[i].position);
                }
                None
            }
            9 => {
                let duration = power(level + 1 + self.rng.bounded(u64::from(level)) as u16);
                self.necromancy_status(ability, "rfb.status.necromancy-cloak", u32::from(duration));
                None
            }
            10 => {
                for category in ["map", "trap", "door", "stairs-down", "stairs-up"] {
                    self.necromancy_shared(
                        ability,
                        E::Detect {
                            subject: AbilityDetectSubjectDefinition::Terrain,
                            category: category.into(),
                            radius: 30,
                            persistent: true,
                            through_walls: true,
                        },
                        &TargetSelection::SelfTarget,
                        events,
                        changed,
                        removed,
                    )?;
                }
                None
            }
            11 => Some(E::IdentifyItem {
                full_identify_power: 0,
                full_identify_roll_sides: 0,
            }),
            15 => {
                let AbilityTargetPlan::Projectile { path, .. } = plan else {
                    unreachable!()
                };
                let (_, i) = self.trace_projectile_path(path);
                if let Some(i) = i.filter(|i| {
                    self.entities[*i].controller_id.is_none() && !self.entities[*i].friendly
                }) {
                    let center = self.entities[i].position;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let pos = Position {
                                x: center.x + dx,
                                y: center.y + dy,
                            };
                            let Some(index) = self.index(pos) else {
                                continue;
                            };
                            if self.terrain[index] != "demo.terrain.floor"
                                || pos == self.player.position
                                || self.entities.iter().any(|a| a.position == pos)
                                || self.gold_piles.iter().any(|pile| pile.position == pos)
                                || self
                                    .items
                                    .iter()
                                    .any(|i| i.location == ItemLocation::Ground(pos))
                            {
                                continue;
                            }
                            self.replace_terrain_from_source(
                                pos,
                                if level < 45 {
                                    "demo.terrain.rubble"
                                } else {
                                    "demo.terrain.wall"
                                },
                                TerrainChangeSource::Magic,
                                events,
                                changed,
                            );
                        }
                    }
                }
                return Ok(None);
            }
            22 => {
                for id in self.projected_monster_status_targets() {
                    let i = self.entities.iter().position(|a| a.id == id).unwrap();
                    if self.entities[i].controller_id.as_deref() != Some(&self.player.id)
                        || !actor_matches_category(
                            self.actor_runtime_definition(&self.entities[i]).unwrap(),
                            "evil",
                        )
                    {
                        continue;
                    }
                    self.entities[i].hp =
                        (self.entities[i].hp + i32::from(level * 6)).min(self.entities[i].max_hp);
                    self.entities[i].statuses.retain(|s| {
                        !matches!(
                            s.kind_id.as_str(),
                            "rfb.status.sleep"
                                | "rfb.status.stun"
                                | "rfb.status.confusion"
                                | "rfb.status.fear"
                        )
                    });
                    apply_status(
                        &mut self.entities[i].statuses,
                        melee_status(STATUS_HASTE, 100, &ability.id),
                    );
                    changed.insert(self.entities[i].position);
                }
                None
            }
            23 => Some(E::ExplodePets),
            25 => Some(E::CreateCurrentTerrain {
                source_terrain_ids: vec!["demo.terrain.floor".into()],
                target_terrain_id: "demo.terrain.warding-glyph".into(),
            }),
            26 => {
                let base = power(20);
                let duration = base + 1 + self.rng.bounded(u64::from(base)) as u16;
                self.necromancy_status(
                    ability,
                    "rfb.status.necromancy-shield",
                    u32::from(duration),
                );
                None
            }
            28 => {
                let duration = 5 + self.rng.bounded(4) as u32;
                self.necromancy_status(ability, STATUS_PARALYSIS, duration);
                None
            }
            29 => Some(E::Banish {
                maximum_distance: power(level * 4),
            }),
            31 => {
                let p = power(level);
                let p = p + 1 + self.rng.bounded(u64::from(p)) as u16;
                for _ in 0..18 {
                    let positions = self.open_positions_around(self.player.position, 4);
                    if positions.is_empty() {
                        break;
                    }
                    let center = positions[self.rng.bounded(positions.len() as u64) as usize];
                    let category =
                        ["lich", "wight", "vampire", "ghost"][self.rng.bounded(4) as usize];
                    let first = self.entities.len();
                    self.trump_summon_batch(
                        ability, category, center, p, 1, false, false, true, events, changed,
                    );
                    for a in &mut self.entities[first..] {
                        apply_status(
                            &mut a.statuses,
                            melee_status(STATUS_HASTE, 100, &ability.id),
                        );
                    }
                }
                let duration =
                    u32::from(level) + 1 + self.rng.bounded(u64::from(20 + level)) as u32;
                self.necromancy_status(ability, STATUS_HASTE, duration);
                None
            }
            _ => unreachable!("validated necromancy source slot"),
        };
        if let Some(effect) = effect {
            let mut branch = ability.clone();
            branch.effect = effect;
            let plan = if matches!(spell, 2 | 3) {
                AbilityTargetPlan::Detect
            } else {
                plan
            };
            return self.resolve_player_ability_effect(branch, plan, events, changed, removed);
        }
        Ok(None)
    }

    fn necromancy_shared(
        &mut self,
        ability: &AbilityDefinition,
        effect: E,
        target: &TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut branch = ability.clone();
        branch.effect = effect;
        if matches!(target, TargetSelection::SelfTarget) {
            branch.target.modes = vec![rfb_content::AbilityTargetModeDefinition::SelfTarget];
            branch.target.range = 0;
            branch.target.requires_line_of_effect = false;
        }
        let plan = self
            .ability_target_plan(&branch, target)
            .expect("source necromancy shared effect target");
        self.resolve_player_ability_effect(branch, plan, events, changed, removed)?;
        Ok(())
    }

    fn necromancy_touch(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let AbilityTargetPlan::Projectile { ref path, .. } = plan else {
            unreachable!()
        };
        let (_, i) = self.trace_projectile_path(path.clone());
        let Some(i) = i else { return Ok(None) };
        let def = self
            .actor_runtime_definition(&self.entities[i])
            .unwrap()
            .clone();
        let pos = self.entities[i].position;
        let level = self.progress.level;
        let (dice, sides, base, kind) = match spell {
            0 => (2, 6, level, D::Cold),
            4 => (4, 6, level, D::Poison),
            7 => (6, 6, level * 3 / 2, D::Dark),
            13 => (0, 0, level * 4, D::Curse),
            24 => (5, 5, level / 2, D::Curse),
            27 => (20, 20, level, D::Disintegrate),
            30 => (0, 0, level * 200, D::Curse),
            12 => (0, 0, 0, D::Physical),
            _ => unreachable!(),
        };
        let multiplier = if self.player_has_equipped_artifact(268) {
            2
        } else {
            1
        };
        let bonus = if matches!(spell, 12 | 30) {
            0
        } else {
            self.casting_spell_damage_bonus()
        };
        let raw = self.roll_damage(dice * multiplier, sides)
            + (i32::from(base) + i32::from(bonus)) * i32::from(multiplier);
        let mut damage = spell_power_value(raw as u64, ability.spell_power_bonus) as u16;
        if spell != 12 {
            self.resolve_confusing_strike(i, &def, events);
        }
        self.resolve_monster_contact_auras(i, &def, events, changed);
        if spell == 12 {
            let dx = (pos.x - self.player.position.x).signum();
            let dy = (pos.y - self.player.position.y).signum();
            let mut dest = pos;
            for n in 1..=10 {
                let p = Position {
                    x: pos.x + dx * n,
                    y: pos.y + dy * n,
                };
                if !self.is_walkable(p)
                    || self.entities.iter().any(|a| a.position == p)
                    || self.player.position == p
                {
                    break;
                }
                dest = p;
            }
            self.entities[i].position = dest;
            changed.extend([pos, dest]);
            return Ok(None);
        }
        let living = actor_matches_category(&def, "living");
        if (spell == 13 && !living)
            || (spell == 24
                && def
                    .monster_casting
                    .as_ref()
                    .is_none_or(|c| c.abilities.iter().all(|a| a.innate)))
        {
            damage = 0;
        }
        if spell == 30
            && (!living
                || (def.tags.iter().any(|t| t == "unique") && self.rng.bounded(888) + 1 != 666)
                || self.monster_saves_against_attribute(i, AttributeKind::Intelligence)
                || self.monster_saves_against_attribute(i, AttributeKind::Intelligence))
        {
            damage = 0;
        }
        if spell == 24
            && damage > 0
            && let Some(mana) = self.resources.get_mut("demo.resource.mana")
        {
            mana.recover(u32::from(damage));
        }
        let mut branch = ability.clone();
        branch.spell_power_bonus = 0;
        // Source uses a radius-zero ball: touching is never reflected as a bolt.
        branch.effect = E::AreaDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: damage,
            damage_type: kind,
            radius: 0,
            target_category: None,
        };
        self.resolve_player_ability_effect(branch, plan, events, changed, removed)?;
        if spell == 13 && damage > 0 {
            self.necromancy_shared(
                ability,
                E::Heal {
                    amount: u32::from(damage),
                },
                &TargetSelection::SelfTarget,
                events,
                changed,
                removed,
            )?;
        }
        Ok(None)
    }

    pub(in crate::game) fn finish_necromancy_repose(&mut self, events: &mut Vec<DomainEvent>) {
        if self.player.hp <= 0 {
            return;
        }
        let mut ability = self
            .content
            .ability("demo.ability.necromancy-repose-of-the-dead")
            .expect("repose ability")
            .clone();
        ability.effect = E::RestoreVitality {
            life_force: 1000,
            restore_attributes: true,
        };
        self.resolve_player_restore_vitality_effect(&ability, events);
        for kind in [
            "rfb.status.poison",
            "rfb.status.blindness",
            "rfb.status.confusion",
            "rfb.status.hallucination",
            "rfb.status.stun",
            "rfb.status.bleeding",
            "rfb.status.berserk",
        ] {
            self.reduce_player_status(kind, u32::MAX, None, None);
        }
    }
}

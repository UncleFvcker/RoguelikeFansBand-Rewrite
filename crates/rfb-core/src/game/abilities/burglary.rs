// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: rogue.c.
use super::AbilityTargetPlan;
use crate::game::ability_scaling::spell_power_value;
use crate::game::monster_combat::melee_status;
use crate::game::player_combat::ProjectileMode;
use crate::game::*;
use rfb_content::{
    AbilityDetectSubjectDefinition as Subject, AbilityEffectDefinition as E,
    AbilityTerrainBeamOperationDefinition as TerrainOp,
};
use rfb_protocol::{DuelistPromptDto, TargetSelection};

impl Game {
    pub(in crate::game) fn player_is_rogue(&self) -> bool {
        self.build
            .as_ref()
            .is_some_and(|b| b.class_id == "demo.class.rogue")
    }

    fn burglary_effect(&self, spell: u8, power: i32) -> E {
        let l = self.progress.level;
        let radius = if l >= 45 { 255 } else { (30 + l) as u8 };
        let detect = |subject, category: &str, persistent| E::Detect {
            subject,
            category: category.into(),
            radius,
            persistent,
            through_walls: true,
        };
        match spell {
            0 => detect(Subject::Terrain, "trap", true),
            1 => E::TerrainBeam {
                operation: TerrainOp::DisarmTraps,
            },
            2 => detect(Subject::Terrain, "gold", true),
            3 => detect(Subject::Item, "item", false),
            6 | 23 => E::BlinkSelf {
                radius: if spell == 6 {
                    30
                } else {
                    (l * 5).min(255) as u8
                },
                line_of_sight: false,
            },
            7 | 14 | 30 => E::CreateCurrentTerrain {
                source_terrain_ids: vec!["demo.terrain.floor".into()],
                target_terrain_id: format!(
                    "demo.terrain.burglary-{}-trap",
                    if spell == 7 {
                        "minor"
                    } else if spell == 14 {
                        "major"
                    } else {
                        "ultimate"
                    }
                ),
            },
            8 => detect(Subject::Terrain, "map", true),
            11 => E::FetchItem {
                maximum_weight_tenths_pound: spell_power_value(u64::from(l) * 15, power)
                    .min(u64::from(u32::MAX)) as u32,
            },
            13 => E::IdentifyItem {
                full_identify_power: 0,
                full_identify_roll_sides: 1,
            },
            16 => E::CreateStair {
                up_terrain_id: "demo.terrain.stairs-up".into(),
                down_terrain_id: "demo.terrain.stairs-down".into(),
            },
            21 => E::TeleportLevel,
            22 => E::AlterReality,
            27 => E::AreaDamage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus: spell_power_value(
                    u64::from(10 * l.saturating_sub(20))
                        + u64::from(self.casting_spell_damage_bonus()),
                    power,
                )
                .min(u64::from(u16::MAX)) as u16,
                damage_type: ActorDamageType::Dark,
                radius: spell_power_value(4, power).min(255) as u8,
                target_category: None,
            },
            9 | 10 | 17 | 18 | 25 | 26 | 31 => E::Damage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus: 0,
                damage_type: ActorDamageType::Physical,
            },
            _ => E::SatisfyHunger, // Self-target probe; these slots execute below.
        }
    }

    fn burglary_target_index(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<usize> {
        let mut a = ability.clone();
        a.effect = self.burglary_effect(9, ability.spell_power_bonus);
        let AbilityTargetPlan::Projectile { path, .. } = self.ability_target_plan(&a, target)?
        else {
            return None;
        };
        let (_, index) = self.trace_projectile_path(path);
        index.filter(|i| {
            self.entity_is_visible_to_player(&self.entities[*i])
                && !self.player_has_status_kind(STATUS_HALLUCINATION)
        })
    }

    pub(super) fn burglary_target_plan(
        &self,
        ability: &AbilityDefinition,
        spell: u8,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        if matches!(spell, 9 | 10 | 17 | 25 | 26 | 31) {
            let i = self.burglary_target_index(ability, target)?;
            if matches!(spell, 9 | 17 | 26 | 31)
                && chebyshev_distance(self.player.position, self.entities[i].position) != 1
            {
                return None;
            }
            if spell == 10
                && (self.entities[i].friendly
                    || self.entities[i].controller_id.is_some()
                    || self.entities[i].summon.is_some())
            {
                return None;
            }
            if spell == 25 && self.riding_actor_id.as_deref() == Some(&self.entities[i].id) {
                return None;
            }
            if spell == 31
                && (!self.entities[i]
                    .statuses
                    .iter()
                    .any(|s| s.kind_id == STATUS_SLEEP)
                    || self.equipped_melee_weapons().is_empty())
            {
                return None;
            }
        } else {
            if spell == 18
                && self
                    .player_projectile_profile()
                    .is_none_or(|p| p.ammo_item_id.is_none())
            {
                return None;
            }
            if spell == 28 && self.player_has_status_kind("rfb.status.burglary-shadows") {
                return None;
            }
            let mut a = ability.clone();
            a.effect = self.burglary_effect(spell, ability.spell_power_bonus);
            self.ability_target_plan(&a, target)?;
        }
        Some(AbilityTargetPlan::Burglary {
            target: target.clone(),
        })
    }

    pub(in crate::game) fn burglary_blink(
        &mut self,
        radius: u16,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut a = self
            .content
            .ability("demo.ability.burglary-minor-getaway")
            .unwrap()
            .clone();
        a.effect = E::BlinkSelf {
            radius: radius.min(255) as u8,
            line_of_sight: false,
        };
        if let Some(plan) = self.ability_target_plan(&a, &TargetSelection::SelfTarget) {
            self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
        }
        Ok(())
    }

    fn burglary_save(&mut self, i: usize, bonus: i32) -> bool {
        let actor = self.actor_runtime_definition(&self.entities[i]).unwrap();
        let monster = actor.level
            + if actor.tags.iter().any(|t| t == "unique") {
                actor.level / 5
            } else {
                0
            };
        let power = (bonus
            + i32::from(self.progress.level)
            + crate::stats::original_save_adjustment(
                self.effective_player_attributes()
                    .index(AttributeKind::Dexterity),
            ))
        .max(1);
        self.rng.bounded(power as u64) <= self.rng.bounded(u64::from(monster.max(1)))
    }

    fn burglary_pick_pocket(
        &mut self,
        i: usize,
        bonus: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let actor = self.entities[i].clone();
        let sleeping = actor.statuses.iter().any(|s| s.kind_id == STATUS_SLEEP);
        let unique = self
            .actor_runtime_definition(&actor)
            .unwrap()
            .tags
            .iter()
            .any(|t| t == "unique");
        let success = !self.burglary_save(i, bonus) || sleeping && !self.burglary_save(i, bonus);
        let mut got_loot = false;
        if success {
            let carried = self.items.iter().position(|item| matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id == &actor.id));
            let (mut items, gold) = if let Some(item) = carried.filter(|_| self.rng.bounded(2) == 0)
            {
                (vec![self.items.remove(item)], Vec::new())
            } else {
                let definition = self.content.actor(&actor.kind_id).unwrap().clone();
                let count = self.entities[i]
                    .burglary_drops_remaining
                    .unwrap_or_else(|| self.roll_monster_drop_count(&definition));
                self.entities[i].burglary_drops_remaining = Some(count);
                if count > 0 {
                    let result = self.generate_ordinary_monster_drops(&actor, 1)?;
                    if !result.0.is_empty() || !result.1.is_empty() {
                        self.entities[i].burglary_drops_remaining = Some(count - 1);
                    }
                    result
                } else {
                    (Vec::new(), Vec::new())
                }
            };
            got_loot = !items.is_empty() || !gold.is_empty();
            if got_loot {
                let dropped = self.burglary_save(i, bonus);
                for mut item in items.drain(..) {
                    if !dropped
                        && self.inventory_quantity_capacity_for(&item, false) >= item.quantity
                    {
                        events.push(DomainEvent::ItemPickedUp {
                            target_kind_id: item.kind_id.clone(),
                            quantity: item.quantity,
                        });
                        self.carry_shop_purchase_item(item);
                    } else if let Some(p) =
                        self.ground_drop_position(actor.position, item.artifact_name.is_some())
                    {
                        item.location = ItemLocation::Ground(p);
                        self.items.push(item);
                        changed.insert(p);
                    }
                }
                for mut pile in gold {
                    if dropped {
                        if let Some(p) = self.ground_drop_position(actor.position, false) {
                            pile.position = p;
                            self.gold_piles.push(pile);
                            changed.insert(p);
                        }
                    } else {
                        self.gold = self
                            .gold
                            .saturating_add(pile.amount)
                            .min(crate::game::gold::MAX_PLAYER_GOLD);
                        events.push(DomainEvent::GoldPickedUp {
                            amount: pile.amount,
                            balance: self.gold,
                        });
                    }
                }
            }
        }
        if sleeping && (!success || unique || self.burglary_save(i, bonus)) {
            self.entities[i]
                .statuses
                .retain(|s| s.kind_id != STATUS_SLEEP);
            if !success || unique || self.burglary_save(i, bonus) {
                self.entities[i].anger = self.entities[i].anger.saturating_add(10);
            }
        } else if !success {
            self.entities[i].anger = self.entities[i].anger.saturating_add(10);
        }
        self.entities[i].friendly = false;
        self.entities[i].controller_id = None;
        self.entities[i].summon = None;
        changed.insert(actor.position);
        if got_loot && !self.burglary_save(i, bonus) {
            if self.progress.level >= 35 {
                self.begin_duelist_choice(DuelistPromptDto::BurglaryEscape);
            } else {
                self.burglary_blink(25 + self.progress.level / 2, events, changed, removed)?;
            }
        }
        Ok(())
    }

    pub(in crate::game) fn burglary_negotiate_choice(
        &mut self,
        target: &str,
        cost: u32,
        accepted: bool,
    ) {
        let i = self
            .entities
            .iter()
            .position(|a| a.id == target)
            .expect("validated negotiation target");
        if !accepted {
            self.entities[i].anger = self.entities[i].anger.saturating_add(10);
            return;
        }
        self.gold -= cost;
        if self.monster_saves_against_attribute(i, AttributeKind::Charisma) {
            self.entities[i].anger = self.entities[i].anger.saturating_add(10);
        } else if !self
            .actor_runtime_definition(&self.entities[i])
            .unwrap()
            .tags
            .iter()
            .any(|t| t == "unique")
            && !self.monster_saves_against_attribute(i, AttributeKind::Charisma)
        {
            self.entities[i].controller_id = Some(self.player.id.clone());
        } else {
            self.entities[i].friendly = true;
        }
    }

    pub(super) fn resolve_burglary(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let AbilityTargetPlan::Burglary { target } = plan else {
            unreachable!()
        };
        let l = self.progress.level;
        let scaled = |n: u16| {
            spell_power_value(u64::from(n), ability.spell_power_bonus).min(u64::from(u16::MAX))
                as u16
        };
        match spell {
            4 | 5 | 12 | 15 | 24 | 28 => {
                let (kind, base, sides) = match spell {
                    4 => ("rfb.status.burglary-infravision", scaled(100), scaled(100)),
                    5 => ("rfb.status.burglary-tread-softly", scaled(50), scaled(50)),
                    12 => (STATUS_TELEPATHY, 25, 30),
                    15 => (STATUS_HASTE, scaled(l), scaled(l + 20)),
                    24 => (STATUS_INVENTORY_PROTECTION, scaled(l * 2), scaled(l * 2)),
                    _ => ("rfb.status.burglary-shadows", scaled(l), scaled(l)),
                };
                let duration =
                    u32::from(base) + 1 + self.rng.bounded(u64::from(sides).max(1)) as u32;
                let mut request = melee_status(kind, duration, &ability.id);
                request.stacking = StatusStacking::KeepStrongest;
                if spell == 4 {
                    request.status.granted_equipment_bonuses.infravision = 3;
                }
                if spell == 5 {
                    request.status.granted_equipment_bonuses.stealth_skill = 3 + i32::from(l / 5);
                }
                apply_status(&mut self.player.statuses, request);
            }
            9 | 26 => {
                let i = self.burglary_target_index(ability, &target).unwrap();
                self.burglary_pick_pocket(
                    i,
                    if spell == 26 { 100 } else { 0 },
                    events,
                    changed,
                    removed,
                )?;
            }
            10 => {
                let i = self.burglary_target_index(ability, &target).unwrap();
                self.entities[i]
                    .statuses
                    .retain(|s| s.kind_id != STATUS_SLEEP);
                let actor = self.actor_runtime_definition(&self.entities[i]).unwrap();
                let thief = actor.tags.iter().any(|t| t == "thief");
                let cost = (10 + actor.level * 100)
                    * if actor.tags.iter().any(|t| t == "unique") {
                        10
                    } else {
                        1
                    };
                if thief && self.monster_saves_against_attribute(i, AttributeKind::Charisma) {
                    self.entities[i].anger = self.entities[i].anger.saturating_add(10);
                } else if thief && self.gold >= cost {
                    self.begin_duelist_choice(DuelistPromptDto::BurglaryNegotiate {
                        target_entity_id: self.entities[i].id.clone(),
                        cost,
                    });
                }
            }
            17 | 31 => {
                let i = self.burglary_target_index(ability, &target).unwrap();
                let energy = self.player.energy_need;
                if spell == 31 {
                    self.resolve_burglary_assassination(i, events, changed, removed)?;
                } else {
                    self.resolve_player_melee(i, true, events, changed, removed)?;
                }
                self.player.energy_need = energy;
            }
            18 => {
                let energy = self.player.energy_need;
                self.resolve_player_projectile(
                    target.clone(),
                    ProjectileMode::Normal,
                    events,
                    changed,
                    removed,
                )?;
                self.player.energy_need = energy;
            }
            19 | 29 => {
                let count = if spell == 19 {
                    self.roll_damage(2, 3) as u16
                } else {
                    12
                };
                let start = self.entities.len();
                self.trump_summon_batch(
                    ability,
                    "thief",
                    self.player.position,
                    l * 3 / 2,
                    count,
                    false,
                    false,
                    spell == 19,
                    events,
                    changed,
                );
                if spell == 29 {
                    for a in &mut self.entities[start..] {
                        apply_status(
                            &mut a.statuses,
                            melee_status(STATUS_HASTE, 100, &ability.id),
                        );
                    }
                }
            }
            20 => {
                let positions = self
                    .area_damage_cells(self.player.position, 1)
                    .into_iter()
                    .map(|(_, p)| p)
                    .collect::<Vec<_>>();
                for p in positions {
                    self.burglary_place_trap(
                        p,
                        "demo.terrain.burglary-minor-trap",
                        events,
                        changed,
                    );
                }
            }
            25 => self.law_subpoena(
                ability,
                match &target {
                    TargetSelection::Entity { entity_id } => entity_id,
                    _ => unreachable!(),
                },
                events,
                changed,
            ),
            _ => {
                let mut a = ability.clone();
                a.effect = self.burglary_effect(spell, ability.spell_power_bonus);
                if let Some(plan) = self.ability_target_plan(&a, &target) {
                    self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
                }
                if spell == 2 {
                    let mut a = ability.clone();
                    a.effect = E::Detect {
                        subject: Subject::Gold,
                        category: "gold".into(),
                        radius: if l >= 45 { 255 } else { (30 + l) as u8 },
                        persistent: false,
                        through_walls: true,
                    };
                    if let Some(p) = self.ability_target_plan(&a, &TargetSelection::SelfTarget) {
                        self.resolve_player_ability_effect(a, p, events, changed, removed)?;
                    }
                }
            }
        }
        if matches!(spell, 17..=20)
            && self
                .rng
                .bounded(self.player_derived_stats().disarm_skill.value.max(1) as u64)
                >= 7
        {
            self.burglary_blink(30, events, changed, removed)?;
        }
        Ok(None)
    }
}

impl Game {
    pub(in crate::game) fn burglary_place_trap(
        &mut self,
        position: Position,
        kind: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.terrain_at(position) == "demo.terrain.floor" {
            self.replace_terrain_from_source(
                position,
                kind,
                crate::game::terrain::TerrainChangeSource::Magic,
                events,
                changed,
            );
        }
    }

    pub(in crate::game) fn burglary_ultimate_trap(
        &mut self,
        ability: &AbilityDefinition,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let roll = self.rng.bounded(6);
        if matches!(roll, 3 | 4) {
            self.law_trap_damage(
                &ability.id,
                position,
                5,
                if roll == 3 {
                    DamageType::Disintegrate
                } else {
                    DamageType::Shards
                },
                800,
                events,
                changed,
                removed,
            )?;
        } else {
            self.law_trap_damage(
                &ability.id,
                position,
                10,
                DamageType::Disintegrate,
                100,
                events,
                changed,
                removed,
            )?;
            if roll == 1 {
                for (_, p) in self.area_damage_cells(position, 10) {
                    if self.is_walkable(p) {
                        self.replace_terrain_from_source(
                            p,
                            "demo.terrain.surface-lava-shallow",
                            crate::game::terrain::TerrainChangeSource::Magic,
                            events,
                            changed,
                        );
                    }
                }
            }
            let category = match roll {
                0 => "bird",
                1 => "demon",
                2 => "angel",
                _ => "high-dragon",
            };
            self.trump_summon_batch(
                ability,
                category,
                position,
                self.progress.level * 2,
                1 + self.progress.level / 10,
                false,
                false,
                true,
                events,
                changed,
            );
        }
        Ok(())
    }
}

// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: ragemage.c, gf.c, xtra1.c.
use super::AbilityTargetPlan;
use crate::effect::{
    STATUS_BERSERK, STATUS_HASTE, STATUS_MANA_BRAND, StatusInstance, apply_status,
};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::monster_combat::melee_status;
use crate::game::{Game, ItemLocation};
use crate::stats::AttributeKind;
use rfb_content::{
    AbilityDefinition, AbilityDetectSubjectDefinition as Subject, AbilityEffectDefinition as E,
    ActorDamageType as D,
};
use rfb_protocol::{Position, TargetSelection};
use std::collections::BTreeSet;

impl Game {
    fn rage_awesome_blow(
        &mut self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut a = ability.clone();
        a.effect = self.rage_effect(10);
        let Some(AbilityTargetPlan::Projectile { path, .. }) = self.ability_target_plan(&a, target)
        else {
            return Ok(());
        };
        let (_, Some(index)) = self.trace_projectile_path(path) else {
            return Ok(());
        };
        let id = self.entities[index].id.clone();
        let origin = self.entities[index].position;
        let start = events.len();
        self.resolve_rage_single_blow(index, events, changed, removed)?;
        if !events[start..]
            .iter()
            .any(|e| matches!(e, DomainEvent::PlayerMeleeHit { .. }))
        {
            return Ok(());
        }
        let dx = (origin.x - self.player.position.x).signum();
        let dy = (origin.y - self.player.position.y).signum();
        let distance = if self.player_has_status_kind(STATUS_BERSERK) {
            6
        } else {
            3
        };
        for n in 1..=distance {
            let Some(i) = self.entities.iter().position(|e| e.id == id) else {
                break;
            };
            let p = Position {
                x: origin.x + dx * n,
                y: origin.y + dy * n,
            };
            if self
                .knockback_projectile_target(&id, &[p], 1, changed)
                .is_none()
            {
                let soft = self.entities.iter().any(|e| e.hp > 0 && e.position == p)
                    || self.index(p).is_some_and(|i| {
                        self.terrain[i].contains("tree")
                            || self.terrain[i].contains("rubble")
                            || self.terrain[i].contains("pit")
                    });
                let trace = crate::event::ProjectileTrace {
                    origin: self.player.position,
                    impact: self.entities[i].position,
                    landing: self.entities[i].position,
                    traversed: vec![self.entities[i].position],
                };
                self.resolve_ability_damage_to_entity_with_resistance(
                    i,
                    &ability.id,
                    crate::resistance::DamageType::Physical,
                    (if soft { 25 } else { 50 }) * (distance - n + 1),
                    trace,
                    Some(crate::resistance::ResistanceLevel::Normal),
                    true,
                    true,
                    events,
                    changed,
                    removed,
                )?;
                break;
            }
        }
        Ok(())
    }

    pub(in crate::game) fn rage_armor_of_fury(&mut self, source: &str, ability: &str, damage: i32) {
        if damage <= 0
            || self.monster_ability_is_innate(ability)
            || !self.player_has_status_kind("rfb.status.rage-armor-of-fury")
        {
            return;
        }
        let Some(i) = self.entities.iter().position(|e| e.id == source) else {
            return;
        };
        let berserk = self.player_has_status_kind(STATUS_BERSERK);
        if self.monster_saves_against_attribute(i, AttributeKind::Strength)
            && (!berserk || self.monster_saves_against_attribute(i, AttributeKind::Strength))
        {
            return;
        }
        let turns = if berserk { 3 } else { 1 };
        if !self.entities[i]
            .statuses
            .iter()
            .any(|s| s.kind_id == "rfb.status.slow")
        {
            apply_status(
                &mut self.entities[i].statuses,
                melee_status("rfb.status.slow", turns, "demo.ability.rage-armor-of-fury"),
            );
        }
        let stun = crate::game::player_combat::monster_stun_amount(damage * turns as i32);
        self.apply_actor_melee_status(
            i,
            "rfb.status.stun",
            stun,
            "demo.ability.rage-armor-of-fury",
        );
    }

    fn rage_shatter_device(
        &mut self,
        ability: &AbilityDefinition,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let TargetSelection::Item { item_id } = target else {
            unreachable!()
        };
        let item = self
            .items
            .iter()
            .find(|i| i.id == item_id)
            .expect("validated device")
            .clone();
        let activation = item.activation.as_ref();
        let profile = activation
            .and_then(|a| {
                crate::game::item_device_generation(
                    &self.content,
                    &item.kind_id,
                    &item.affix_ids,
                    Some(&a.profile_id),
                    item.artifact_name.is_some(),
                )
                .and_then(|d| d.activations.iter().find(|p| p.id == a.profile_id))
            })
            .cloned();
        if let (Some(activation), Some(profile)) = (activation, profile) {
            use rfb_content::ItemUseEffectDefinition as I;
            let effects: Vec<_> = match &profile.effect {
                I::Sequence { effects } => effects.iter().collect(),
                effect => vec![effect],
            };
            if profile.id == "demo.device-activation.nothing" {
            } else if let Some(I::AreaDestruction {
                floor_terrain_id,
                wall_terrain_id,
                quartz_terrain_id,
                magma_terrain_id,
                ..
            }) = effects
                .iter()
                .find(|e| matches!(e, I::AreaDestruction { .. }))
            {
                let mut a = ability.clone();
                a.effect = E::AreaDestruction {
                    minimum_radius: (15 + self.progress.level) as u8,
                    maximum_radius: (25 + self.progress.level) as u8,
                    floor_terrain_id: floor_terrain_id.clone(),
                    wall_terrain_id: wall_terrain_id.clone(),
                    quartz_terrain_id: quartz_terrain_id.clone(),
                    magma_terrain_id: magma_terrain_id.clone(),
                };
                self.resolve_player_ability_effect(
                    a,
                    AbilityTargetPlan::SelfTarget,
                    events,
                    changed,
                    removed,
                )?;
            } else if profile.id.ends_with("heal-curing")
                || profile.id.ends_with("heal-curing-hero")
                || profile.id.ends_with("restoring")
            {
                self.restore_all_player_attributes();
                self.restore_player_experience_and_life_force(1000, events);
                self.player.statuses.retain(|s| {
                    !matches!(
                        s.kind_id.as_str(),
                        "rfb.status.poison"
                            | "rfb.status.blindness"
                            | "rfb.status.confusion"
                            | "rfb.status.hallucination"
                            | "rfb.status.stun"
                            | "rfb.status.bleeding"
                    )
                });
                self.refresh_player_resource_maxima();
                self.apply_player_healing(5000);
            } else if profile.id.ends_with("teleport-away")
                || profile.id.ends_with("banish-evil")
                || profile.id.ends_with("banish-all")
            {
                let mut a = ability.clone();
                a.effect = E::Banish {
                    maximum_distance: self.progress.level * 4,
                };
                self.resolve_player_ability_effect(
                    a,
                    AbilityTargetPlan::SelfTarget,
                    events,
                    changed,
                    removed,
                )?;
            } else {
                let damage_type = if profile.id == "demo.device-activation.confusing-light" {
                    D::Confusion
                } else {
                    effects
                        .iter()
                        .find_map(|e| match e {
                            I::Damage { damage_type, .. }
                            | I::AreaDamage { damage_type, .. }
                            | I::BeamDamage { damage_type, .. } => Some(*damage_type),
                            _ => None,
                        })
                        .unwrap_or(D::Mana)
                };
                let damage_type = match damage_type {
                    D::Plasma => D::Fire,
                    D::Ice => D::Cold,
                    D::Physical | D::Water => D::Mana,
                    _ => damage_type,
                };
                self.resolve_player_area_damage_with_base_policy(
                    &ability.id,
                    Vec::new(),
                    false,
                    damage_type.into(),
                    5,
                    None,
                    activation.device_check_difficulty * 16,
                    true,
                    true,
                    false,
                    events,
                    changed,
                    removed,
                )?;
            }
        }
        // Destruction can already have removed a floor device. Otherwise destroy its entire stack.
        self.items.retain(|i| i.id != item_id);
        self.item_property_knowledge.remove(&item_id);
        if let ItemLocation::Ground(p) = item.location {
            changed.insert(p);
        }
        Ok(None)
    }

    pub(in crate::game) fn player_is_rage_mage(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, _, c, _)| c.id == "demo.class.rage-mage")
    }

    fn rage_gain_mana(&mut self, amount: i32) {
        if let Some(p) = self.resources.get_mut("demo.resource.mana") {
            p.current = p
                .current
                .saturating_add(amount.max(0) as u32)
                .min(p.maximum);
        }
    }

    pub(in crate::game) fn rage_fueled(&mut self, damage: i32) {
        if !self.player_is_rage_mage() || damage <= 0 {
            return;
        }
        self.rage_gain_mana(Self::rage_damage_mana(
            damage,
            self.player.hp,
            self.effective_player_max_hp(),
        ) as i32);
    }

    pub(in crate::game) fn rage_damage_mana(damage: i32, current_hp: i32, maximum_hp: i32) -> u32 {
        if damage <= 0 {
            return 0;
        }
        let hp = i64::from(maximum_hp.max(1));
        (i64::from(damage) * (hp * 3 / 2 - i64::from(current_hp)) / hp)
            .clamp(1, i64::from(i32::MAX)) as u32
    }

    pub(in crate::game) fn rage_blood_lust(&mut self, damage: i32) {
        if !self.player_is_rage_mage() || damage <= 0 {
            return;
        }
        self.rage_gain_mana(
            (damage
                / if self.player_has_status_kind(STATUS_BERSERK) {
                    8
                } else {
                    12
                })
            .max(1),
        );
        self.rage_mana_sustained = true;
    }

    pub(in crate::game) fn rage_after_action(&mut self, energy: i32) {
        if !self.player_is_rage_mage() || energy <= 0 {
            return;
        }
        if self.rage_mana_sustained {
            self.rage_mana_sustained = false;
        } else if let Some(p) = self.resources.get_mut("demo.resource.mana") {
            let loss = (u64::from(p.current / 8 + u32::from(self.progress.level / 10) + 1)
                * energy as u64
                / 100)
                .min(u64::from(u32::MAX));
            p.current = p.current.saturating_sub(loss as u32);
        }
    }

    pub(in crate::game) fn rage_status(&self) -> Option<StatusInstance> {
        if !self.player_is_rage_mage() {
            return None;
        }
        let mut s = melee_status("rfb.status.rage-mage", 1, "demo.class.rage-mage").status;
        let l = i32::from(self.progress.level);
        s.granted_modifiers.defense = -(5 + if l == 50 {
            55
        } else {
            55 * l * 2 / 150 + 55 * l * l / 7500
        });
        if self.player_has_status_kind("rfb.status.rage-resist-curses") {
            s.granted_equipment_bonuses.saving_throw_skill =
                if self.player_has_status_kind(STATUS_BERSERK) {
                    40
                } else {
                    20
                };
        }
        Some(s)
    }

    pub(in crate::game) fn consume_one_item(&mut self, id: &str) {
        let i = self
            .items
            .iter()
            .position(|i| i.id == id)
            .expect("validated consumed item");
        if self.items[i].quantity > 1 {
            self.items[i].quantity -= 1;
        } else {
            self.items.remove(i);
            self.item_property_knowledge.remove(id);
        }
    }

    fn rage_self_damage(&mut self, amount: i32) {
        // DAMAGE_NOESCAPE, explicitly excluded from Rage Fueled in effects.c.
        let outcome = crate::effect::resolve_damage(
            crate::effect::DamagePacket::new(amount, crate::resistance::DamageType::Physical),
            crate::resistance::ResistanceLevel::Normal,
        );
        crate::game::damage::commit_final_player_damage(
            &mut self.player,
            self.resources.get_mut("demo.resource.mana"),
            false,
            outcome,
            crate::game::damage::FatalityPolicy::BelowZero,
        );
    }

    fn rage_focus_cost(&self, spell: u8) -> i32 {
        let l = i32::from(self.progress.level);
        if spell == 5 {
            10 + l / 2
        } else if self.player_has_status_kind(STATUS_BERSERK) {
            2 * l
        } else {
            10 + l
        }
    }

    pub(in crate::game) fn rage_failure(&mut self, spell: u8) {
        if matches!(spell, 5 | 26) {
            self.rage_self_damage(self.rage_focus_cost(spell));
        }
        if spell == 31 {
            self.resources
                .get_mut("demo.resource.mana")
                .unwrap()
                .current = 0;
        }
    }

    fn rage_effect(&self, spell: u8) -> E {
        let l = self.progress.level;
        let berserk = self.player_has_status_kind(STATUS_BERSERK);
        match spell {
            0 | 12 => E::ConeDamage {
                damage_dice: if spell == 0 { 3 + (l - 1) / 5 } else { l - 10 },
                damage_sides: if spell == 0 { 4 } else { 8 },
                damage_bonus: 0,
                damage_type: D::Sound,
                radius: if spell == 0 { 2 } else { 3 },
            },
            1 | 15 => E::Detect {
                subject: Subject::Actor,
                category: "magical".into(),
                radius: 30,
                persistent: false,
                through_walls: true,
            },
            2 => E::TerrainBeam {
                operation: rfb_content::AbilityTerrainBeamOperationDefinition::StoneToMud,
            },
            3 => E::Strafing,
            4 => {
                let mut e = self
                    .content
                    .ability("demo.ability.arcane-light-area")
                    .unwrap()
                    .effect
                    .clone();
                if let E::LightArea {
                    damage_sides,
                    radius,
                    ..
                } = &mut e
                {
                    *damage_sides = l / 2;
                    *radius = (l / 10 + 1) as u8;
                }
                e
            }
            6 => E::SatisfyHunger,
            8 => E::Detect {
                subject: Subject::Terrain,
                category: "map".into(),
                radius: 30,
                persistent: true,
                through_walls: true,
            },
            23 => E::AreaDamage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus: 18 * l,
                damage_type: D::Physical,
                radius: 2,
                target_category: None,
            },
            10 | 22 | 30 | 31 => E::Damage {
                damage_dice: 0,
                damage_sides: 0,
                damage_bonus: 0,
                damage_type: D::Physical,
            },
            13 => E::MeleeAdjacent,
            17 => E::SuppressMonsterReproduction {
                damage_dice: 1,
                damage_sides: 17,
                damage_bonus: 17,
            },
            18 => E::ResistElements {
                duration_dice: 1,
                duration_sides: if berserk { 20 } else { 10 },
                duration_bonus: if berserk { 20 } else { 10 },
            },
            24 => self
                .content
                .ability("demo.ability.arcane-identify")
                .unwrap()
                .effect
                .clone(),
            25 => self
                .content
                .ability("demo.ability.nature-earthquake")
                .unwrap()
                .effect
                .clone(),
            29 => E::RemoveEquippedCurses {
                include_heavy: false,
            },
            _ => E::SatisfyHunger, // Self-target probe; effect is handled explicitly below.
        }
    }

    pub(super) fn rage_target_plan(
        &self,
        ability: &AbilityDefinition,
        spell: u8,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        if spell == 25 {
            return matches!(target, TargetSelection::SelfTarget).then(|| {
                AbilityTargetPlan::Rage {
                    target: target.clone(),
                }
            });
        }
        if spell == 28 {
            let TargetSelection::Item { item_id } = target else {
                return None;
            };
            let item = self.items.iter().find(|i| i.id == *item_id)?;
            let valid = matches!(item.location, ItemLocation::Inventory)
                || item.location == ItemLocation::Ground(self.player.position);
            if !valid || item.charges.is_none() {
                return None;
            }
        } else {
            if spell == 10 && self.equipped_melee_weapons().is_empty() {
                return None;
            }
            let mut a = ability.clone();
            a.effect = self.rage_effect(spell);
            self.ability_target_plan(&a, target)?;
        }
        Some(AbilityTargetPlan::Rage {
            target: target.clone(),
        })
    }

    fn rage_timed(&mut self, ability: &AbilityDefinition, kind: &str, base: u32, sides: u32) {
        let duration = base + 1 + self.rng.bounded(u64::from(sides)) as u32;
        let mut request = melee_status(kind, duration, &ability.id);
        request.stacking = crate::effect::StatusStacking::KeepStrongest;
        apply_status(&mut self.player.statuses, request);
        self.refresh_player_resource_maxima();
    }

    pub(super) fn resolve_rage(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let AbilityTargetPlan::Rage { target } = plan else {
            unreachable!()
        };
        let berserk = self.player_has_status_kind(STATUS_BERSERK);
        let l = self.progress.level;
        let effect = match spell {
            2 => {
                let mut a = ability.clone();
                a.effect = self.rage_effect(2);
                let plan = self.ability_target_plan(&a, &target).expect("smash target");
                self.resolve_player_ability_effect(a.clone(), plan, events, changed, removed)?;
                a.effect = E::TerrainBeam {
                    operation:
                        rfb_content::AbilityTerrainBeamOperationDefinition::DestroyTrapsAndDoors,
                };
                let plan = self
                    .ability_target_plan(&a, &target)
                    .expect("smash doors target");
                return self.resolve_player_ability_effect(a, plan, events, changed, removed);
            }
            5 | 26 => {
                let hp = self.rage_focus_cost(spell);
                self.rage_self_damage(hp);
                if !self.player_is_dead() {
                    self.rage_gain_mana(hp * if spell == 26 { 2 } else { 1 });
                    self.rage_mana_sustained = true;
                }
                return Ok(None);
            }
            7 | 16 => {
                let id = if spell == 7 {
                    "demo.ability.craft-heroism"
                } else {
                    "demo.ability.craft-berserk"
                };
                let mut e = self
                    .content
                    .ability(id)
                    .expect("shared heroism/berserk")
                    .effect
                    .clone();
                if spell == 16
                    && let E::Sequence { effects } = &mut e
                {
                    for effect in effects {
                        if let E::ApplyStatus {
                            duration_ticks,
                            duration_sides,
                            ..
                        } = effect
                        {
                            *duration_ticks = 10;
                            *duration_sides = u32::from(l);
                        }
                    }
                }
                e
            }
            9 => {
                self.rage_timed(ability, "rfb.status.rage-resist-disenchantment", 10, 10);
                self.player
                    .statuses
                    .iter_mut()
                    .find(|s| s.kind_id == "rfb.status.rage-resist-disenchantment")
                    .unwrap()
                    .granted_resistances
                    .insert(
                        crate::resistance::DamageType::Disenchant,
                        crate::resistance::ResistanceLevel::Resistant,
                    );
                return Ok(None);
            }
            11 => {
                self.rage_timed(ability, "rfb.status.rage-spell-reaction", 30, 30);
                return Ok(None);
            }
            14 => {
                self.rage_timed(ability, "rfb.status.rage-resist-curses", 20, 20);
                return Ok(None);
            }
            20 => {
                self.rage_timed(ability, "rfb.status.rage-armor-of-fury", 25, 25);
                return Ok(None);
            }
            21 => {
                let n = if berserk { 10 } else { 4 };
                self.rage_timed(ability, STATUS_MANA_BRAND, n, n);
                return Ok(None);
            }
            27 => {
                self.rage_timed(ability, "rfb.status.rage-spell-turning", 20, 20);
                return Ok(None);
            }
            28 => return self.rage_shatter_device(ability, target, events, changed, removed),
            19 => {
                let before: BTreeSet<_> = self.entities.iter().map(|e| e.id.clone()).collect();
                let mut a = ability.clone();
                a.effect = E::Summon {
                    actor_kind_id: "demo.actor.warrior-of-the-dawn".into(),
                    count: 4 + self.rng.bounded(3) as u8,
                    radius: 2,
                    duration_turns: 0,
                    hostile: false,
                };
                let plan = self
                    .ability_target_plan(&a, &target)
                    .expect("summon target");
                self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
                if berserk {
                    for e in &mut self.entities {
                        if !before.contains(&e.id) {
                            apply_status(
                                &mut e.statuses,
                                melee_status(STATUS_HASTE, 100, &ability.id),
                            );
                        }
                    }
                }
                return Ok(None);
            }
            10 => {
                self.rage_awesome_blow(ability, &target, events, changed, removed)?;
                return Ok(None);
            }
            22 | 30 => {
                let mut a = ability.clone();
                a.effect = self.rage_effect(spell);
                let Some(AbilityTargetPlan::Projectile { path, .. }) =
                    self.ability_target_plan(&a, &target)
                else {
                    return Ok(None);
                };
                if let (_, Some(index)) = self.trace_projectile_path(path) {
                    if spell == 22 {
                        self.entities[index].statuses.retain(|s| {
                            !matches!(
                                s.kind_id.as_str(),
                                "rfb.status.haste" | "rfb.status.invulnerability"
                            )
                        });
                    } else {
                        let saved = self
                            .monster_saves_against_attribute(index, AttributeKind::Strength)
                            && (!berserk
                                || self.monster_saves_against_attribute(
                                    index,
                                    AttributeKind::Strength,
                                ));
                        self.entities[index]
                            .statuses
                            .retain(|s| s.kind_id != "rfb.status.rage-anti-magic");
                        if !saved {
                            let duration = 3 + self.rng.bounded(2) as u32;
                            apply_status(
                                &mut self.entities[index].statuses,
                                melee_status("rfb.status.rage-anti-magic", duration, &ability.id),
                            );
                        }
                    }
                    changed.insert(self.entities[index].position);
                }
                return Ok(None);
            }
            29 => E::RemoveEquippedCurses {
                include_heavy: self.rng.bounded(3) == 0,
            },
            31 => {
                let sp = u64::from(self.resources["demo.resource.mana"].current);
                let z = sp * sp / 100;
                E::Damage {
                    damage_dice: 0,
                    damage_sides: 0,
                    damage_bonus: (1200 * z / (1000 + z)) as u16,
                    damage_type: D::Physical,
                }
            }
            _ => self.rage_effect(spell),
        };
        let mut a = ability.clone();
        a.effect = effect;
        let plan = if spell == 25 {
            AbilityTargetPlan::SelfTarget
        } else {
            self.ability_target_plan(&a, &target)
                .expect("validated Rage target")
        };
        let result = self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
        if spell == 1 && berserk {
            self.rage_timed(ability, "rfb.status.rage-detect-magical", 20, 20);
        }
        if spell == 15 {
            let mut a = ability.clone();
            a.effect = E::Detect {
                subject: Subject::Item,
                category: "magic-item".into(),
                radius: 30,
                persistent: false,
                through_walls: true,
            };
            self.resolve_player_ability_effect(
                a,
                AbilityTargetPlan::Detect,
                events,
                changed,
                removed,
            )?;
        }
        if spell == 31 {
            self.rage_self_damage(100);
            if !berserk {
                let mut stun = melee_status("rfb.status.stun", 99, &ability.id);
                stun.status.intensity = 99;
                apply_status(&mut self.player.statuses, stun);
            }
            self.resources
                .get_mut("demo.resource.mana")
                .unwrap()
                .current = 0;
        }
        Ok(result)
    }
}

// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: samurai.c, hissatsu.c.
use super::AbilityTargetPlan;
use crate::action::GameAction;
use crate::effect::{STATUS_BLINDNESS, STATUS_CONFUSION, STATUS_FEAR, STATUS_STUN, StatusInstance};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::Game;
use crate::game::monster_combat::melee_status;
use crate::resistance::{DamageType, ResistanceLevel};
use rfb_content::{AbilityDefinition, AbilityEffectDefinition as E, EquipmentPassive as P};
use rfb_protocol::{Direction, Position, TargetSelection};
use std::collections::BTreeSet;

impl Game {
    pub(in crate::game) fn player_is_samurai(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, _, c, _)| c.id == "demo.class.samurai")
    }

    pub(in crate::game) fn samurai_mana_limit(maximum: u32, level: u16) -> u32 {
        maximum.saturating_mul(4).max(5 * u32::from(level) + 5)
    }

    pub(in crate::game) fn samurai_state_is_valid(&self) -> bool {
        if !self.player_is_samurai() {
            return self.samurai == Default::default();
        }
        self.samurai.posture <= 4
            && (self.samurai.posture == 0
                || self.progress.level >= 20 + 5 * u16::from(self.samurai.posture))
    }

    pub(in crate::game) fn samurai_grants_status(&self, kind: &str) -> bool {
        self.samurai.posture == 4 && kind == "rfb.status.ultimate-resistance"
    }

    pub(in crate::game) fn samurai_passives(&self) -> BTreeSet<P> {
        if self.samurai.posture == 4 {
            return [
                P::Telepathy,
                P::SeeInvisible,
                P::SlowDigestion,
                P::Regeneration,
                P::Levitation,
                P::HoldLife,
                P::ReflectsBolts,
                P::FireAura,
                P::ColdAura,
                P::ElectricityAura,
                P::SustainStrength,
                P::SustainIntelligence,
                P::SustainWisdom,
                P::SustainDexterity,
                P::SustainConstitution,
                P::SustainCharisma,
            ]
            .into();
        }
        if self.samurai.posture == 2
            && !self
                .player
                .statuses
                .iter()
                .any(|s| s.kind_id == STATUS_BLINDNESS)
        {
            return [P::ReflectsBolts].into();
        }
        BTreeSet::new()
    }

    pub(in crate::game) fn samurai_status(&self) -> Option<StatusInstance> {
        if self.samurai.posture == 0 {
            return None;
        }
        let mut s = melee_status("rfb.status.samurai-posture", 1, "demo.class.samurai").status;
        s.intensity = u16::from(self.samurai.posture);
        match self.samurai.posture {
            3 => {
                s.granted_modifiers.strength = 5;
                s.granted_modifiers.intelligence = 5;
                s.granted_modifiers.wisdom = 5;
                s.granted_modifiers.dexterity = 5;
                s.granted_modifiers.constitution = 5;
                s.granted_modifiers.charisma = 5;
                s.granted_modifiers.defense = -50;
                s.granted_equipment_bonuses.melee_skill = 150;
                for d in [
                    DamageType::Acid,
                    DamageType::Electricity,
                    DamageType::Fire,
                    DamageType::Cold,
                ] {
                    s.granted_resistances.insert(d, ResistanceLevel::Vulnerable);
                }
            }
            4 => {
                s.granted_modifiers.defense = 100;
                for d in [
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
                    DamageType::Chaos,
                    DamageType::Disenchant,
                ] {
                    s.granted_resistances.insert(d, ResistanceLevel::Resistant);
                }
                s.granted_equipment_bonuses.light_radius = 1;
                s.granted_status_immunities
                    .insert("rfb.status.paralysis".into());
            }
            _ => {}
        }
        Some(s)
    }

    pub(in crate::game) fn samurai_ability_unavailable_reason(
        &self,
        id: &str,
    ) -> Option<&'static str> {
        let a = self.content.ability(id)?;
        match a.effect {
            E::Hissatsu { spell } => {
                if self.equipped_melee_weapons().is_empty() {
                    return Some("weapon-required");
                }
                if self.player_has_status_kind(STATUS_CONFUSION) {
                    return Some("confused");
                }
                if self.player_has_status_kind(STATUS_FEAR) {
                    return Some("afraid");
                }
                if matches!(spell, 6 | 7) && self.riding_actor_id.is_some() {
                    return Some("riding");
                }
            }
            E::SamuraiPosture { posture } if posture != 0 => {
                if self.equipped_melee_weapons().is_empty() {
                    return Some("weapon-required");
                }
                if [STATUS_CONFUSION, STATUS_FEAR, STATUS_STUN]
                    .iter()
                    .any(|s| self.player_has_status_kind(s))
                {
                    return Some("status-prevents-ability");
                }
            }
            E::SamuraiConcentration => {
                if self.samurai.posture != 0 {
                    return Some("posture-active");
                }
                if self
                    .entities
                    .iter()
                    .any(|a| a.hp > 0 && self.actor_is_player_side(a))
                {
                    return Some("pets-present");
                }
            }
            _ => {}
        }
        None
    }

    pub(in crate::game) fn samurai_concentrate(&mut self) {
        let level = self.progress.level;
        for pool in self.resources.values_mut() {
            pool.current = pool
                .current
                .saturating_add(pool.maximum / 2)
                .min(Self::samurai_mana_limit(pool.maximum, level));
            if pool.current == Self::samurai_mana_limit(pool.maximum, level) {
                pool.fraction = 0;
            }
        }
    }

    pub(in crate::game) fn set_samurai_posture(&mut self, posture: u8) {
        self.samurai.posture = posture;
        self.refresh_player_resource_maxima();
        self.player.hp = self.player.hp.min(self.effective_player_max_hp());
    }

    pub(in crate::game) fn samurai_before_action(
        &mut self,
        action: &GameAction,
        advances_world: bool,
    ) {
        let item_action = matches!(
            action,
            GameAction::UseItem { .. }
                | GameAction::Equip { .. }
                | GameAction::Unequip { .. }
                | GameAction::Throw { .. }
                | GameAction::Fire { .. }
                | GameAction::FireTarget { .. }
        );
        if (self.samurai.posture == 4
            && advances_world
            && !matches!(action, GameAction::Wait | GameAction::CastAbility { .. }))
            || (matches!(self.samurai.posture, 1..=3) && item_action)
        {
            self.set_samurai_posture(0);
        }
    }

    pub(in crate::game) fn advance_samurai(&mut self) {
        let old_posture = self.samurai.posture;
        self.samurai.counter = false;
        self.samurai.sutemi = false;
        if self.samurai.posture != 0
            && (self.equipped_melee_weapons().is_empty()
                || [STATUS_CONFUSION, STATUS_FEAR, STATUS_STUN]
                    .iter()
                    .any(|s| self.player_has_status_kind(s)))
        {
            self.samurai.posture = 0;
        }
        if self.samurai.posture == 4
            && let Some(pool) = self.resources.values_mut().next()
        {
            if pool.current < 3 {
                self.samurai.posture = 0;
            } else {
                pool.current -= 2;
            }
        }
        if self.samurai.posture != old_posture {
            self.set_samurai_posture(self.samurai.posture);
        }
    }

    pub(in crate::game) fn hissatsu_target_plan(
        &self,
        ability: &AbilityDefinition,
        spell: u8,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        if spell == 11 {
            let TargetSelection::Item { item_id } = target else {
                return None;
            };
            let item = self.items.iter().find(|i| &i.id == item_id)?;
            let base = self.content.item(&item.kind_id)?.rfb_base_kind?;
            if !(16..=23).contains(&base.tval) && !(30..=38).contains(&base.tval) {
                return None;
            }
            let mut a = ability.clone();
            a.effect = E::IdentifyItem {
                full_identify_power: u16::from(self.progress.level >= 45),
                full_identify_roll_sides: 1,
            };
            return self.ability_target_plan(&a, target);
        }
        if matches!(spell, 4 | 6 | 19 | 22 | 25 | 31) {
            return matches!(target, TargetSelection::SelfTarget).then_some(
                AbilityTargetPlan::Hissatsu {
                    target: target.clone(),
                },
            );
        }
        let path = self.ability_path(ability, target)?;
        let p = *path.first()?;
        if spell == 27 {
            let destination = *path.last()?;
            if self.player_has_anti_teleport()
                || !path.iter().all(|p| self.projectile_can_cross(*p))
                || !self.player_can_teleport_to(destination, false)
            {
                return None;
            }
        } else if !matches!(spell, 0 | 1 | 2 | 12 | 18 | 20 | 21 | 26 | 29)
            && !self.entities.iter().any(|a| a.hp > 0 && a.position == p)
        {
            return None;
        }
        Some(AbilityTargetPlan::Hissatsu {
            target: target.clone(),
        })
    }

    pub(in crate::game) fn resolve_hissatsu(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let mut a = ability.clone();
        if spell == 11 {
            a.effect = E::IdentifyItem {
                full_identify_power: u16::from(self.progress.level >= 45),
                full_identify_roll_sides: 1,
            };
            return self.resolve_player_ability_effect(a, plan, events, changed, removed);
        }
        let AbilityTargetPlan::Hissatsu { target } = plan else {
            unreachable!("validated sword technique")
        };
        match spell {
            4 => {
                a.effect = E::Detect {
                    subject: rfb_content::AbilityDetectSubjectDefinition::Actor,
                    category: "mind".into(),
                    radius: 30,
                    persistent: false,
                    through_walls: true,
                };
                return self.resolve_player_ability_effect(
                    a,
                    AbilityTargetPlan::Detect,
                    events,
                    changed,
                    removed,
                );
            }
            6 => {
                self.samurai.counter = true;
                return Ok(None);
            }
            22 => {
                a.effect = E::VisibleDamage {
                    damage_dice: 1,
                    damage_sides: self.progress.level * 3,
                    damage_bonus: 0,
                    damage_type: rfb_content::ActorDamageType::Sound,
                    target_category: None,
                    unlife_change_on_hit: 0,
                };
                self.resolve_player_ability_effect(
                    a,
                    AbilityTargetPlan::SelfTarget,
                    events,
                    changed,
                    removed,
                )?;
                self.aggravate_monsters(None, &ability.id, changed);
                return Ok(None);
            }
            31 => {
                self.pay_class_ability_hit_points(9999);
                return Ok(None);
            }
            _ => {}
        }
        if spell == 19 {
            let cut = self
                .player
                .statuses
                .iter()
                .find(|s| s.kind_id == "rfb.status.bleeding")
                .map_or(0, |s| s.remaining_ticks);
            let duration = if cut < 300 {
                cut + 300
            } else {
                cut.saturating_mul(2)
            };
            self.player
                .statuses
                .retain(|s| s.kind_id != "rfb.status.bleeding");
            crate::effect::apply_status(
                &mut self.player.statuses,
                melee_status("rfb.status.bleeding", duration, &ability.id),
            );
            let origin = self.player.position;
            let ids: Vec<_> = self
                .entities
                .iter()
                .filter(|a| {
                    a.hp > 0
                        && (a.position.x - origin.x)
                            .abs()
                            .max((a.position.y - origin.y).abs())
                            == 1
                })
                .map(|a| a.id.clone())
                .collect();
            for id in ids {
                if let Some(i) = self.entities.iter().position(|a| a.id == id && a.hp > 0) {
                    self.resolve_hissatsu_melee(i, spell, events, changed, removed)?;
                }
            }
            return Ok(None);
        }
        if spell == 25 {
            return self.resolve_hissatsu_moon(ability, events, changed, removed);
        }
        let path = self
            .ability_path(ability, &target)
            .expect("validated technique path");
        let origin = self.player.position;
        let p = path[0];
        let delta = ((p.x - origin.x).signum(), (p.y - origin.y).signum());
        if spell == 2 {
            let id = self.equipped_melee_weapons()[0].id.clone();
            let direction = [
                Direction::North,
                Direction::NorthEast,
                Direction::East,
                Direction::SouthEast,
                Direction::South,
                Direction::SouthWest,
                Direction::West,
                Direction::NorthWest,
            ]
            .into_iter()
            .find(|d| d.delta() == delta)
            .expect("direction");
            let chance = 24 + self.rng.bounded(5) as i32 + 1;
            self.throw_hissatsu_weapon(&id, direction, chance, events, changed, removed)?;
            return Ok(None);
        }
        if matches!(spell, 21 | 29) {
            let damage = self.hissatsu_weapon_wave_damage(spell == 29);
            a.effect = if spell == 21 {
                E::BeamDamage {
                    damage_dice: 0,
                    damage_sides: 0,
                    damage_bonus: damage,
                    damage_type: rfb_content::ActorDamageType::Force,
                    maximum_range: None,
                }
            } else {
                E::AreaDamage {
                    damage_dice: 0,
                    damage_sides: 0,
                    damage_bonus: damage,
                    damage_type: rfb_content::ActorDamageType::Meteor,
                    radius: 5,
                    target_category: None,
                }
            };
            let plan = self.ability_target_plan(&a, &target).expect("wave target");
            return self.resolve_player_ability_effect(a, plan, events, changed, removed);
        }
        if spell == 1 {
            let dirs = [
                Direction::North,
                Direction::NorthEast,
                Direction::East,
                Direction::SouthEast,
                Direction::South,
                Direction::SouthWest,
                Direction::West,
                Direction::NorthWest,
            ];
            let center = dirs
                .iter()
                .position(|d| d.delta() == delta)
                .expect("direction");
            for n in [center, (center + 7) % 8, (center + 1) % 8] {
                let q = self.position_in_direction(dirs[n]);
                if let Some(i) = self
                    .entities
                    .iter()
                    .position(|a| a.hp > 0 && a.position == q)
                {
                    self.resolve_hissatsu_melee(i, spell, events, changed, removed)?;
                }
            }
            return Ok(None);
        }
        let attack_path: Vec<_> = if matches!(spell, 0 | 18 | 26 | 27) {
            path.clone()
        } else {
            vec![p]
        };
        let mut attacked = false;
        for q in attack_path {
            if let Some(i) = self
                .entities
                .iter()
                .position(|a| a.hp > 0 && a.position == q)
            {
                attacked = true;
                let id = self.entities[i].id.clone();
                for _ in 0..if spell == 23 {
                    3
                } else if spell == 28 {
                    2
                } else {
                    1
                } {
                    let Some(i) = self.entities.iter().position(|a| a.id == id && a.hp > 0) else {
                        break;
                    };
                    self.resolve_hissatsu_melee(i, spell, events, changed, removed)?;
                    if spell == 23 {
                        self.hissatsu_push(&id, delta, 1, true, events, changed);
                    }
                    if self.player_is_dead() {
                        break;
                    }
                }
                if spell == 10 {
                    self.hissatsu_push(&id, delta, 5, false, events, changed);
                }
                if spell != 27 {
                    break;
                }
            } else if matches!(spell, 18 | 26) {
                if !self.hissatsu_can_step(q) {
                    break;
                }
                events.extend(self.relocate_player(q, changed));
            }
            if !self.player_can_enter_position(q) && spell != 27 {
                break;
            }
        }
        if spell == 7 && attacked {
            let q = Position {
                x: origin.x + delta.0 * 2,
                y: origin.y + delta.1 * 2,
            };
            if self.hissatsu_can_step(q) && self.player_can_enter_position(p) {
                events.extend(self.relocate_player(q, changed));
            }
        }
        if spell == 12 {
            a.effect = E::TerrainBeam {
                operation: rfb_content::AbilityTerrainBeamOperationDefinition::StoneToMud,
            };
            self.resolve_player_ability_effect(
                a.clone(),
                AbilityTargetPlan::Projectile {
                    path: vec![p],
                    stop_at_actor: false,
                },
                events,
                changed,
                removed,
            )?;
        }
        if spell == 20 && !attacked {
            let id = self.equipped_melee_weapons()[0].id.clone();
            self.resolve_player_impact_earthquake(id, events, changed, removed)?;
        }
        if spell == 16 && attacked {
            self.samurai.sutemi = true;
        }
        if spell == 30 && attacked {
            let cost = 101 + self.rng.bounded(100) as u32;
            self.pay_class_ability_hit_points(cost);
        }
        if spell == 27 {
            self.resolve_player_teleport_effect(
                ability,
                *path.last().expect("destination"),
                events,
                changed,
            );
        }
        Ok(None)
    }

    pub(in crate::game) fn continue_hissatsu_slaughter(
        &mut self,
        direction: Option<Direction>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut pending = self
            .pending_ability_direction
            .take()
            .expect("pending sword technique");
        let Some(direction) = direction else {
            return Ok(());
        };
        if self.player_is_dead()
            || self
                .samurai_ability_unavailable_reason(&pending.ability_id)
                .is_some()
        {
            return Ok(());
        }
        let ability = self
            .content
            .ability(&pending.ability_id)
            .expect("formal sword technique")
            .clone();
        let target = TargetSelection::Direction { direction };
        let Some(plan) = self.hissatsu_target_plan(&ability, 26, &target) else {
            return Ok(());
        };
        let pool = self
            .resources
            .get_mut("demo.resource.mana")
            .expect("Samurai mana");
        if pool.current <= 8 {
            return Ok(());
        }
        pool.current -= 8;
        let before = removed.len();
        self.resolve_hissatsu(&ability, 26, plan, events, changed, removed)?;
        let current = self.resources["demo.resource.mana"].current;
        if removed.len() > before && !self.player_is_dead() && current > 8 {
            pending.cast_resolution.resource_cost += 8;
            pending.cast_resolution.resource_paid += 8;
            pending.cast_resolution.resource_after = current;
            self.pending_ability_direction = Some(pending);
        }
        Ok(())
    }

    fn hissatsu_can_step(&self, p: Position) -> bool {
        self.player_can_enter_position(p)
            && !self.entities.iter().any(|a| a.hp > 0 && a.position == p)
            && self
                .index(p)
                .and_then(|i| self.content.terrain(&self.terrain[i]))
                .is_some_and(|t| t.trap.is_none())
    }

    fn hissatsu_push(
        &mut self,
        id: &str,
        delta: (i32, i32),
        distance: u8,
        follow: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.dungeon_blocks_melee() {
            return;
        }
        let Some(i) = self.entities.iter().position(|a| a.hp > 0 && a.id == id) else {
            return;
        };
        for _ in 0..distance {
            let old = self.entities[i].position;
            let q = Position {
                x: old.x + delta.0,
                y: old.y + delta.1,
            };
            if !self.actor_can_enter_position(i, q)
                || self.entities.iter().any(|a| a.hp > 0 && a.position == q)
                || q == self.player.position
            {
                break;
            }
            self.entities[i].position = q;
            changed.extend([old, q]);
            if follow && self.hissatsu_can_step(old) {
                events.extend(self.relocate_player(old, changed));
            }
        }
    }

    fn hissatsu_weapon_wave_damage(&self, meteor: bool) -> u16 {
        self.player_melee_profiles(&self.player_derived_stats())
            .iter()
            .filter(|p| p.source_item_id.is_some())
            .map(|p| {
                let mut dice = i32::from(p.damage_dice) * (i32::from(p.damage_sides) + 1) * 50;
                if let Some(item) = p
                    .source_item_id
                    .as_ref()
                    .and_then(|id| self.items.iter().find(|i| &i.id == id))
                {
                    if self.item_has_rfb_flag(item, "VORPAL2") {
                        dice = dice * 5 / 3;
                    } else if self.item_has_rfb_flag(item, "VORPAL") {
                        dice = dice * 11 / 9;
                    }
                }
                let weapon_bonus = p
                    .source_item_id
                    .as_ref()
                    .and_then(|id| self.items.iter().find(|i| &i.id == id))
                    .and_then(|i| self.item_melee_profile(i))
                    .map_or(0, |p| p.to_damage);
                dice += if meteor { p.to_damage } else { weapon_bonus } * 100;
                dice * i32::from(p.attacks) / 200 * if meteor { 3 } else { 1 }
                    / if meteor { 2 } else { 1 }
            })
            .sum::<i32>()
            .clamp(0, i32::from(u16::MAX)) as u16
    }

    fn resolve_hissatsu_moon(
        &mut self,
        _ability: &AbilityDefinition,
        _events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        _removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        for _ in 0..3 {
            for i in 0..self.entities.len() {
                if self.entities[i].hp <= 0
                    || !self.entity_is_visible_to_player(&self.entities[i])
                    || self.entities[i]
                        .statuses
                        .iter()
                        .any(|s| s.kind_id == "rfb.status.sleep")
                {
                    continue;
                }
                let d = self
                    .actor_runtime_definition(&self.entities[i])
                    .expect("monster")
                    .clone();
                if d.tags
                    .iter()
                    .any(|t| t == "empty-mind" || t == "resist-all")
                {
                    continue;
                }
                let (kind, duration) = if self.rng.bounded(5) == 0 {
                    ("rfb.status.slow", 50)
                } else if self.rng.bounded(4) == 0 {
                    (
                        STATUS_STUN,
                        self.roll_damage(self.progress.level / 10 + 3, 4 * self.progress.level)
                            as u32
                            + 1,
                    )
                } else if self.rng.bounded(3) == 0 {
                    ("rfb.status.paralysis", 3)
                } else {
                    continue;
                };
                if d.tags.iter().any(|t| t == "unique")
                    || self.actor_has_status_immunity(i, kind)
                    || (kind == "rfb.status.paralysis"
                        && self.actor_has_status_immunity(i, "rfb.status.sleep"))
                    || u64::from(d.level)
                        > self.rng.bounded(u64::from(
                            (4 * self.progress.level).saturating_sub(10).max(1),
                        )) + 11
                {
                    continue;
                }
                crate::effect::apply_status(
                    &mut self.entities[i].statuses,
                    melee_status(kind, duration, "demo.ability.hissatsu-moon-dazzling"),
                );
                changed.insert(self.entities[i].position);
            }
        }
        Ok(None)
    }
}

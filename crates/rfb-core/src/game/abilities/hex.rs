// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: hex.c, do-spell.c, xtra1.c.
use super::AbilityTargetPlan;
use crate::action::GameAction;
use crate::effect::StatusInstance;
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::monster_combat::melee_status;
use crate::game::{Game, ItemInstance, ItemLocation};
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition as E, ActorDamageType as D, EquipmentPassive as P,
};
use rfb_protocol::{Direction, ItemCurseSeverityDto as Curse, Position, TargetSelection};
use std::collections::BTreeSet;

pub(in crate::game) fn continuous(spell: u8) -> bool {
    spell < 32 && !matches!(spell, 5 | 7 | 10 | 18 | 20 | 26 | 29 | 31)
}

impl Game {
    pub(in crate::game) fn player_uses_hex(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(b, _, _, _)| b.first_realm_id.as_deref() == Some("hex"))
    }

    pub(in crate::game) fn hexing(&self, spell: u8) -> bool {
        !self.hex.interrupted && self.hex.active & (1_u32 << spell) != 0
    }

    pub(in crate::game) fn hex_grants_status(&self, kind: &str) -> bool {
        (kind == "rfb.status.blessed" && self.hexing(0))
            || (kind == "rfb.status.vengeance" && self.hexing(23))
    }

    pub(in crate::game) fn hex_passives(&self) -> BTreeSet<P> {
        let mut p = BTreeSet::new();
        if self.hexing(2) {
            p.extend([P::FireAura, P::Regeneration]);
        }
        if self.hexing(6) {
            p.insert(P::EspEvil);
        }
        if self.hexing(8) {
            p.insert(P::ColdAura);
        }
        if self.hexing(16) {
            p.insert(P::ElectricityAura);
        }
        p
    }

    pub(in crate::game) fn hex_curse_bonus(&self, item: &ItemInstance, ty: bool) -> i32 {
        if !self.player_uses_hex() || item.curse.is_none() {
            return 0;
        }
        5 + if self.item_has_heavy_curse(item) {
            7
        } else {
            0
        } + if item.curse == Some(Curse::Permanent) {
            13
        } else {
            0
        } + if ty
            && self.item_has_active_equipped_curse_effect(
                item,
                rfb_protocol::ItemCurseEffectDto::TyCurse,
            ) {
            5
        } else {
            0
        }
    }

    pub(in crate::game) fn hex_status(&self) -> Option<StatusInstance> {
        if !self.player_uses_hex() {
            return None;
        }
        let mut s = melee_status("rfb.status.hex", 1, "demo.ability.hex-evil-blessing").status;
        if !self.hex.interrupted && self.hex.active != 0 {
            s.granted_equipment_bonuses.stealth_skill = -(1 + self.hex.active.count_ones() as i32);
        }
        if self.hexing(0)
            && !self
                .player
                .statuses
                .iter()
                .any(|s| s.kind_id == "rfb.status.blessed")
        {
            s.granted_modifiers.defense += 5;
            s.granted_equipment_bonuses.melee_skill += 10;
            s.granted_equipment_bonuses.ranged_skill += 10;
        }
        if self.hexing(4) {
            s.granted_modifiers.strength += 4;
            s.granted_modifiers.max_hp += 15;
        }
        if self.hexing(14) {
            s.granted_modifiers.strength += 4;
            s.granted_modifiers.dexterity += 4;
            s.granted_modifiers.constitution += 4;
            s.granted_modifiers.max_hp += 60;
        }
        if self.hexing(8) {
            s.granted_modifiers.defense += 30;
        }
        if self.hexing(16) {
            s.granted_modifiers.speed += 3;
        }
        for item in self
            .items
            .iter()
            .filter(|i| matches!(i.location, ItemLocation::Equipped { .. }))
        {
            if self
                .content
                .item(&item.kind_id)
                .and_then(|d| d.rfb_base_kind)
                .is_some_and(|b| (30..=38).contains(&b.tval))
            {
                s.granted_modifiers.defense += self.hex_curse_bonus(item, false);
            }
        }
        if self.hex.active == 0 && s.granted_modifiers.defense == 0 {
            None
        } else {
            Some(s)
        }
    }

    pub(in crate::game) fn stop_hex(&mut self, spell: Option<u8>) {
        if let Some(s) = spell {
            self.hex.active &= !(1_u32 << s);
        } else {
            self.hex.active = 0;
        }
        if self.hex.active == 0 {
            self.hex.interrupted = false;
        }
        self.refresh_player_resource_maxima();
        self.player.hp = self.player.hp.min(self.effective_player_max_hp());
    }

    pub(in crate::game) fn interrupt_hex(&mut self) {
        if self.hex.active != 0 {
            self.hex.interrupted = true;
            self.refresh_player_resource_maxima();
            self.player.hp = self.player.hp.min(self.effective_player_max_hp());
        }
    }

    pub(in crate::game) fn hex_before_action(&mut self, action: &GameAction) {
        if self.hex.active == 0 {
            return;
        }
        if let GameAction::UseItem { item_id, .. } = action {
            let tval = self
                .items
                .iter()
                .find(|i| &i.id == item_id)
                .and_then(|i| self.content.item(&i.kind_id))
                .and_then(|d| d.rfb_base_kind)
                .map(|b| b.tval);
            if tval == Some(75)
                || tval == Some(70)
                    && (self.progress.level < 35
                        || self.hex.active.count_ones()
                            >= u32::from((self.progress.level / 15 + 1).min(4)))
            {
                self.stop_hex(None);
            }
        }
    }

    fn hex_has_cursed_cloak(&self) -> bool {
        self.items.iter().any(|i| {
            matches!(i.location, ItemLocation::Equipped { .. })
                && i.curse.is_some()
                && self
                    .content
                    .item(&i.kind_id)
                    .and_then(|d| d.rfb_base_kind)
                    .is_some_and(|b| b.tval == 35)
        })
    }

    pub(in crate::game) fn hex_ability_unavailable_reason(&self, id: &str) -> Option<&'static str> {
        match self.content.ability(id)?.effect {
            E::StopHex { spell } => {
                if !self.player_uses_hex()
                    || self.hex.active == 0
                    || spell.is_some_and(|s| self.hex.active & (1 << s) == 0)
                {
                    Some("no-active-hex")
                } else {
                    None
                }
            }
            E::Hex { spell } => {
                if self.hex.active & (1 << spell) != 0 {
                    return Some("hex-already-active");
                }
                if self.hex.active.count_ones() >= (self.progress.level / 15 + 1).min(4) as u32 {
                    return Some("hex-capacity");
                }
                if matches!(spell, 7 | 31) && self.hex.revenge_kind != 0 {
                    return Some("revenge-active");
                }
                if spell == 21 && !self.hex_has_cursed_cloak() {
                    return Some("cursed-cloak-required");
                }
                None
            }
            _ => None,
        }
    }

    pub(in crate::game) fn hex_state_is_valid(&self) -> bool {
        if let Some(pending) = &self.pending_ability_direction
            && pending.ability_id == "demo.ability.hex-revenge"
            && (self.hex.revenge_kind != 2
                || self.hex.revenge_ticks != 0
                || self.hex.revenge_cast.as_ref() != Some(&pending.cast_resolution))
        {
            return false;
        }
        if !self.player_uses_hex() {
            return self.hex == Default::default();
        }
        let valid_bits = (0..32)
            .filter(|&s| continuous(s))
            .fold(0_u32, |bits, s| bits | 1 << s);
        self.hex.active & !valid_bits == 0
            && self.hex.active.count_ones() <= 4
            && (!self.hex.interrupted || self.hex.active != 0)
            && self.hex.revenge_kind <= 2
            && self.hex.revenge_ticks <= 9
            && (self.hex.revenge_kind != 0
                || (self.hex.revenge_ticks == 0
                    && self.hex.revenge_damage == 0
                    && self.hex.revenge_cast.is_none()))
            && (self.hex.revenge_kind == 0
                || self.hex.revenge_cast.as_ref().is_some_and(|cast| {
                    cast.succeeded
                        && cast.ability_id
                            == if self.hex.revenge_kind == 1 {
                                "demo.ability.hex-patience"
                            } else {
                                "demo.ability.hex-revenge"
                            }
                        && self.learned_abilities.contains(&cast.ability_id)
                        && (self.hex.revenge_ticks > 0
                            || self.hex.revenge_kind == 2
                                && self.pending_ability_direction.as_ref().is_some_and(|p| {
                                    p.ability_id == cast.ability_id && p.cast_resolution == *cast
                                }))
                }))
            && (0..32).all(|s| {
                self.hex.active & (1 << s) == 0
                    || self.content.abilities().any(|a| {
                        matches!(a.effect, E::Hex { spell } if spell == s)
                            && self.learned_abilities.contains(&a.id)
                    })
            })
    }

    pub(super) fn hex_target_plan(
        &self,
        ability: &AbilityDefinition,
        spell: u8,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        if spell == 18 {
            let mut a = ability.clone();
            a.effect = E::RechargeFromPlayer {
                power: self.progress.level * 2,
            };
            return self.ability_target_plan(&a, target);
        }
        if matches!(spell, 5 | 10 | 20 | 26) {
            let TargetSelection::Item { item_id } = target else {
                return None;
            };
            let i = self.items.iter().find(|i| i.id == *item_id)?;
            let d = self.content.item(&i.kind_id)?;
            let tval = d.rfb_base_kind.map(|b| b.tval);
            let valid = if spell == 10 {
                matches!(i.location, ItemLocation::Inventory) && tval == Some(75)
            } else {
                matches!(i.location, ItemLocation::Equipped { .. })
                    && match spell {
                        5 => tval.is_some_and(|t| (21..=23).contains(&t)),
                        20 => tval.is_some_and(|t| (30..=38).contains(&t)),
                        _ => i.curse.is_some(),
                    }
            };
            return valid.then(|| AbilityTargetPlan::Item {
                item_id: item_id.clone(),
            });
        }
        if spell == 29 {
            let mut a = ability.clone();
            a.effect = E::DimensionDoor {
                range: self.progress.level + 2,
            };
            return self.ability_target_plan(&a, target);
        }
        matches!(target, TargetSelection::SelfTarget).then_some(AbilityTargetPlan::SelfTarget)
    }

    pub(super) fn resolve_hex(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        if continuous(spell) {
            self.hex.active |= 1 << spell;
            self.hex.interrupted = false;
            self.refresh_player_resource_maxima();
            self.hex_pulse(ability, spell, events, changed, removed)?;
            return Ok(None);
        }
        match spell {
            5 | 20 | 26 => {
                let AbilityTargetPlan::Item { item_id } = plan else {
                    unreachable!()
                };
                self.hex_curse_item(&item_id, spell);
            }
            10 => {
                let AbilityTargetPlan::Item { item_id } = plan else {
                    unreachable!()
                };
                self.use_inventory_item(&item_id, None, None, events, changed, removed)?;
            }
            18 => {
                let mut a = ability.clone();
                a.effect = E::RechargeFromPlayer {
                    power: self.progress.level * 2,
                };
                return self.resolve_player_ability_effect(a, plan, events, changed, removed);
            }
            29 => {
                let AbilityTargetPlan::DimensionDoor {
                    requested,
                    destination_valid,
                    ..
                } = plan
                else {
                    unreachable!()
                };
                let adjacent = self
                    .entities
                    .iter()
                    .any(|e| e.hp > 0 && crate::game::adjacent(e.position, requested));
                if destination_valid
                    && adjacent
                    && self.rng.bounded(u64::from(self.progress.level).pow(2) / 2) != 0
                {
                    self.resolve_player_teleport_effect(ability, requested, events, changed);
                } else {
                    let candidates = self.random_teleport_candidates(30);
                    if !candidates.is_empty() {
                        let to = candidates[self.rng.bounded(candidates.len() as u64) as usize];
                        self.resolve_player_teleport_effect(ability, to, events, changed);
                    }
                }
            }
            7 | 31 => {
                self.hex.revenge_kind = if spell == 7 { 1 } else { 2 };
                let speed = self.player_derived_stats().speed.value;
                self.hex.revenge_ticks = 4
                    + self.rng.bounded(if spell == 7 { 3 } else { 2 }) as u8
                    + (3 - (speed - 100) / 10).clamp(0, 3) as u8;
                self.hex.revenge_damage = 0;
                self.hex.revenge_cast = events.iter().rev().find_map(|e| {
                    if let DomainEvent::AbilityCastSucceeded { resolution } = e {
                        Some(resolution.clone())
                    } else {
                        None
                    }
                });
            }
            _ => unreachable!(),
        }
        Ok(None)
    }

    fn hex_curse_item(&mut self, id: &str, spell: u8) {
        let index = self.items.iter().position(|i| i.id == id).unwrap();
        let i = &self.items[index];
        if spell == 26 {
            let level = u32::from(self.progress.level / 5);
            let mut amount = level + self.rng.bounded(u64::from(level)) as u32 + 1;
            if self.item_has_rfb_flag(&self.items[index], "TY_CURSE")
                || self.items[index]
                    .intrinsic_curse_effects
                    .contains(&rfb_protocol::ItemCurseEffectDto::TyCurse)
            {
                amount += self.rng.bounded(5) as u32 + 1;
            }
            let pool = self.resources.get_mut("demo.resource.mana").unwrap();
            pool.current = (pool.current + amount).min(pool.maximum);
            let chance = match self.items[index].curse {
                Some(Curse::Permanent) => 0,
                Some(Curse::Heavy) => 7,
                _ => 3,
            };
            if chance != 0 && self.rng.bounded(chance) == 0 {
                self.items[index].curse = None;
                self.items[index].intrinsic_curse_effects.clear();
            }
            return;
        }
        let artifact = i.artifact_name.is_some()
            || self
                .content
                .item(&i.kind_id)
                .is_some_and(|d| d.artifact_generation.is_some());
        let blessed = self.item_has_rfb_flag(i, "BLESSED");
        if self.rng.bounded(3) != 0 && (artifact || blessed) {
            if self.rng.bounded(3) == 0 {
                let losses = [
                    self.rng.bounded(3) as i16 + 1,
                    self.rng.bounded(3) as i16 + 1,
                    self.rng.bounded(3) as i16 + 1,
                ];
                let e = &mut self.items[index].enchantments;
                for (value, loss) in [
                    (&mut e.to_hit, losses[0]),
                    (&mut e.to_damage, losses[1]),
                    (&mut e.to_armor, losses[2]),
                ] {
                    if *value > 0 {
                        *value = (*value - loss % 2).max(0);
                    }
                }
            }
            return;
        }
        let ego = !self.items[index].affix_ids.is_empty();
        if self.items[index].curse.is_none() {
            self.items[index].curse = Some(Curse::Normal);
        }
        let mut power = 0;
        if artifact || ego {
            if self.rng.bounded(3) == 0 && self.items[index].curse != Some(Curse::Permanent) {
                self.items[index].curse = Some(Curse::Heavy);
            }
            if self.rng.bounded(666) == 0 {
                self.items[index].intrinsic_curse_effects.extend([
                    rfb_protocol::ItemCurseEffectDto::TyCurse,
                    rfb_protocol::ItemCurseEffectDto::Aggravate,
                ]);
                if self.rng.bounded(666) == 0 {
                    self.items[index].intrinsic_properties.rfb_heavy_curse =
                        self.item_has_heavy_curse(&self.items[index]);
                    self.items[index].curse = Some(Curse::Permanent);
                }
                let flags = if spell == 5 {
                    vec!["AGGRAVATE", "VORPAL", "BRAND_VAMP"]
                } else {
                    vec!["AGGRAVATE", "RES_POIS", "RES_DARK", "RES_NETHER"]
                };
                for f in flags {
                    self.items[index]
                        .intrinsic_properties
                        .rfb_flags
                        .insert(f.into());
                }
                if spell == 5 {
                    self.items[index]
                        .intrinsic_weapon_traits
                        .insert(rfb_protocol::WeaponTraitDto::Vorpal);
                    self.items[index]
                        .intrinsic_properties
                        .passives
                        .insert(P::Vampiric);
                } else {
                    for d in [D::Poison, D::Dark, D::Nether] {
                        self.items[index]
                            .intrinsic_properties
                            .resistances
                            .insert(d, rfb_content::ActorResistanceLevel::Resistant);
                    }
                }
                power = 2;
            }
        }
        let tval = self
            .content
            .item(&self.items[index].kind_id)
            .unwrap()
            .rfb_base_kind
            .unwrap()
            .tval;
        if spell == 20 || power > 0 || self.rng.bounded(26) == 0 {
            let curse = crate::game::ego::curses::get_curse(&mut self.rng, power, tval);
            self.items[index].intrinsic_curse_effects.insert(curse);
        } else if self.rng.bounded(2) == 0 {
            self.items[index]
                .intrinsic_curse_effects
                .insert(rfb_protocol::ItemCurseEffectDto::Allergy);
        }
    }

    fn hex_pulse(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let l = self.progress.level;
        let effect = match spell {
            1 | 9 | 17 => {
                let dice = if spell == 1 {
                    1
                } else if spell == 9 {
                    2
                } else {
                    4
                };
                let healing = self.roll_damage(dice, 10);
                self.apply_player_healing(healing);
                if spell == 17 {
                    self.player.statuses.retain(|s| {
                        !matches!(
                            s.kind_id.as_str(),
                            "rfb.status.bleeding" | "rfb.status.stun"
                        )
                    });
                } else if let Some(cut) = self
                    .player
                    .statuses
                    .iter_mut()
                    .find(|s| s.kind_id == "rfb.status.bleeding")
                {
                    cut.remaining_ticks =
                        (cut.remaining_ticks / if spell == 9 { 2 } else { 1 }).saturating_sub(10);
                    self.player.statuses.retain(|s| s.remaining_ticks > 0);
                }
                return Ok(());
            }
            3 | 22 => E::VisibleDamage {
                damage_dice: 1,
                damage_sides: (if spell == 3 { l / 2 + 5 } else { l * 3 / 2 })
                    .saturating_add(self.casting_spell_damage_bonus()),
                damage_bonus: 0,
                damage_type: if spell == 3 { D::Poison } else { D::PsiDrain },
                target_category: None,
                unlife_change_on_hit: 0,
            },
            11 => {
                let targets: Vec<_> = self
                    .entities
                    .iter()
                    .filter(|e| e.hp > 0 && self.entity_is_visible_to_player(e))
                    .map(|e| e.id.clone())
                    .collect();
                let damage = self
                    .roll_damage(
                        1,
                        (l / 2 + 5).saturating_add(self.casting_spell_damage_bonus()),
                    )
                    .max(0) as u16;
                for id in targets {
                    let mut a = ability.clone();
                    a.effect = E::DrainLife {
                        damage_dice: 0,
                        damage_sides: 0,
                        damage_bonus: damage,
                        damage_type: D::Physical,
                        target_category: "living".into(),
                        repeat: 1,
                        feeds: false,
                    };
                    a.target.range = 18;
                    if let Some(plan) =
                        self.ability_target_plan(&a, &TargetSelection::Entity { entity_id: id })
                    {
                        self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
                    }
                }
                return Ok(());
            }
            19 => self
                .content
                .ability("demo.ability.death-animate-dead")
                .unwrap()
                .effect
                .clone(),
            21 => {
                if !self.hex_has_cursed_cloak() {
                    self.stop_hex(Some(21));
                }
                return Ok(());
            }
            25 => {
                self.hex_restore_life(events);
                return Ok(());
            }
            28 => E::VisibleApplyStatus {
                status_kind_id: "rfb.status.stun".into(),
                intensity: 1,
                duration_ticks: u32::from(5 + l / 5),
                duration_dice: 0,
                duration_sides: 0,
                stacking: rfb_content::AbilityStatusStackingDefinition::Extend,
                resistance_type: None,
                power: Some(5 + l / 5),
                target_category: None,
            },
            _ => return Ok(()),
        };
        let mut a = ability.clone();
        a.effect = effect;
        a.spell_power_bonus = 0;
        let plan = self
            .ability_target_plan(&a, &TargetSelection::SelfTarget)
            .expect("self hex pulse");
        self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
        Ok(())
    }

    fn hex_restore_life(&mut self, events: &mut Vec<DomainEvent>) {
        let before = self.progress.clone();
        self.progress.experience = (self.progress.experience + self.progress.experience / 20)
            .min(self.progress.maximum_experience);
        self.progress.life_force = (self.progress.life_force + 15).min(1000);
        let max = self.progress.maximum_attributes;
        let a = &mut self.progress.attributes;
        for (value, maximum) in [
            (&mut a.strength, max.strength),
            (&mut a.intelligence, max.intelligence),
            (&mut a.wisdom, max.wisdom),
            (&mut a.dexterity, max.dexterity),
            (&mut a.constitution, max.constitution),
            (&mut a.charisma, max.charisma),
        ] {
            if *value < maximum {
                *value = (*value + if *value < 18 { 1 } else { 10 }).min(maximum);
            }
        }
        if self.progress == before {
            self.stop_hex(Some(25));
        } else {
            self.apply_player_experience(0, events);
            self.refresh_player_resource_maxima();
        }
    }

    pub(in crate::game) fn hex_barrier(&mut self, index: usize, spell: u8) -> bool {
        if !self.hexing(spell) {
            return false;
        }
        let level = self
            .actor_runtime_definition(&self.entities[index])
            .map_or(1, |d| d.level)
            .max(1);
        u64::from(self.progress.level * 3 / 2) > self.rng.bounded(u64::from(level))
    }

    pub(in crate::game) fn hex_shadow_aura(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let d = self
            .actor_runtime_definition(&self.entities[index])
            .expect("aura target");
        if d.tags.iter().any(|t| t == "resist-all")
            || self.entities[index]
                .resistances
                .level(crate::resistance::DamageType::Dark)
                != crate::resistance::ResistanceLevel::Normal
        {
            return Ok(false);
        }
        let stats = self.player_derived_stats();
        let damage = self
            .player_melee_profiles(&stats)
            .iter()
            .find(|p| p.source_item_id.is_some())
            .map_or(1, |p| {
                i32::from(p.damage_dice) * (i32::from(p.damage_sides) + 1) / 2 + p.to_damage
            })
            .max(0);
        let cursed_armor = self.items.iter().any(|i| {
            matches!(i.location, ItemLocation::Equipped { .. })
                && i.curse.is_some()
                && self
                    .content
                    .item(&i.kind_id)
                    .and_then(|d| d.rfb_base_kind)
                    .is_some_and(|b| (36..=38).contains(&b.tval))
        });
        self.resolve_player_contact_aura_damage(
            index,
            crate::resistance::DamageType::Dark,
            damage * if cursed_armor { 2 } else { 1 },
            events,
            changed,
            removed,
        )
    }

    pub(in crate::game) fn advance_hex(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if self.hex.revenge_ticks > 0 {
            self.hex.revenge_ticks -= 1;
            let patience = self.hex.revenge_kind == 1;
            let power = if patience {
                self.hex
                    .revenge_damage
                    .saturating_mul(2)
                    .saturating_add(u32::from(self.casting_spell_damage_bonus()))
                    .min(200)
            } else {
                self.hex
                    .revenge_damage
                    .saturating_add(u32::from(self.casting_spell_damage_bonus()))
            };
            if self.hex.revenge_ticks == 0 || patience && power >= 200 {
                if patience {
                    if power > 0 {
                        self.hex_revenge_blast(
                            None,
                            power,
                            2 + (power / 50) as u8,
                            events,
                            changed,
                            removed,
                        )?;
                    }
                    self.clear_hex_revenge();
                } else if power > 0 {
                    self.hex.revenge_ticks = 0;
                    self.pending_ability_direction =
                        Some(rfb_protocol::PendingAbilityDirectionDto {
                            ability_id: "demo.ability.hex-revenge".into(),
                            branch_roll: 1,
                            cast_resolution: self.hex.revenge_cast.clone().expect("revenge cast"),
                        });
                } else {
                    self.clear_hex_revenge();
                }
            }
        }
        if self.hex.active == 0 {
            return Ok(());
        }
        if self.player_is_dead() || self.player_has_anti_magic() {
            self.stop_hex(None);
            return Ok(());
        }
        let spells: Vec<_> = (0..32)
            .filter(|s| self.hex.active & (1 << s) != 0)
            .map(|spell| {
                (
                    spell,
                    self.content
                        .abilities()
                        .find(|a| matches!(a.effect, E::Hex { spell: s } if s == spell))
                        .unwrap()
                        .clone(),
                )
            })
            .collect();
        let sum: u64 = spells
            .iter()
            .map(|(_, a)| {
                u64::from(self.ability_effective_resource_cost(a, self.ability_progress_value(a)))
            })
            .sum();
        let cost = (sum << 32) / 3 + ((spells.len() as u64 - 1) << 32);
        let p = self.resources.get_mut("demo.resource.mana").unwrap();
        let available = ((u64::from(p.current) << 32) + u64::from(self.hex.mana_fraction))
            .min(u64::from(p.maximum) << 32);
        if available < cost {
            self.stop_hex(None);
            return Ok(());
        }
        let remaining = available - cost;
        p.current = (remaining >> 32) as u32;
        self.hex.mana_fraction = remaining as u32;
        self.hex.interrupted = false;
        self.refresh_player_resource_maxima();
        for (spell, a) in spells {
            let progress = self.ability_progress_value(&a);
            let depth = self.floor_depth(&self.current_floor_id);
            let level = self.progress.level;
            let minimum = Self::player_ability_parameters(&a).minimum_level;
            let gain = match progress.proficiency {
                0..900 => 5,
                900..1200 => u16::from(self.rng.bounded(2) == 0 && depth > 4 && depth + 10 > level),
                1200..1400 => {
                    u16::from(self.rng.bounded(5) == 0 && depth + 5 > level && depth + 5 > minimum)
                }
                1400..1600 => {
                    u16::from(self.rng.bounded(5) == 0 && depth + 5 > level && depth > minimum)
                }
                _ => 0,
            };
            if let Some(p) = self.ability_progress.get_mut(&a.id) {
                p.proficiency = p.proficiency.saturating_add(gain).min(p.proficiency_cap);
            }
            self.hex_pulse(&a, spell, events, changed, removed)?;
        }
        Ok(())
    }

    fn clear_hex_revenge(&mut self) {
        self.hex.revenge_kind = 0;
        self.hex.revenge_ticks = 0;
        self.hex.revenge_damage = 0;
        self.hex.revenge_cast = None;
    }

    fn hex_revenge_blast(
        &mut self,
        direction: Option<Direction>,
        power: u32,
        radius: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let mut a = self
            .content
            .ability("demo.ability.hex-revenge")
            .unwrap()
            .clone();
        a.effect = E::AreaDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: power.min(u32::from(u16::MAX)) as u16,
            damage_type: D::HellFire,
            radius,
            target_category: None,
        };
        a.target.range = if direction.is_some() { 18 } else { 0 };
        a.target.modes = vec![if direction.is_some() {
            rfb_content::AbilityTargetModeDefinition::Direction
        } else {
            rfb_content::AbilityTargetModeDefinition::SelfTarget
        }];
        let target = direction.map_or(TargetSelection::SelfTarget, |direction| {
            TargetSelection::Direction { direction }
        });
        let plan = self
            .ability_target_plan(&a, &target)
            .expect("revenge direction");
        self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
        Ok(())
    }

    pub(in crate::game) fn continue_hex_revenge(
        &mut self,
        direction: Option<Direction>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(direction) = direction else {
            return Ok(());
        };
        let power = self
            .hex
            .revenge_damage
            .saturating_add(u32::from(self.casting_spell_damage_bonus()));
        self.hex_revenge_blast(Some(direction), power, 1, events, changed, removed)?;
        self.pending_ability_direction = None;
        self.clear_hex_revenge();
        Ok(())
    }
}

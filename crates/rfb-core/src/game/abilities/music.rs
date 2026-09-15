// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: bard.c, do-spell.c.
use super::AbilityTargetPlan;
use crate::effect::{STATUS_FEAR, STATUS_HASTE, STATUS_INVULNERABILITY, StatusInstance};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::ability_scaling::spell_power_value;
use crate::game::monster_combat::melee_status;
use crate::game::{Game, ItemLocation};
use crate::resistance::{DamageType, ResistanceLevel};
use rfb_content::{AbilityDefinition, AbilityEffectDefinition as E, ActorDamageType as D};
use rfb_protocol::{Position, TargetSelection};
use std::collections::BTreeSet;

pub(in crate::game) fn continuous(spell: u8) -> bool {
    !matches!(spell, 2 | 5 | 14 | 19 | 22 | 23 | 26 | 29 | 30)
}

impl Game {
    pub(in crate::game) fn player_is_bard(&self) -> bool {
        self.character_definitions()
            .is_some_and(|(_, _, c, _)| c.id == "demo.class.bard")
    }

    pub(in crate::game) fn singing(&self, spell: u8) -> bool {
        self.music.spell == Some(spell) && !self.music.interrupted
    }

    pub(in crate::game) fn music_grants_status(&self, kind: &str) -> bool {
        match kind {
            STATUS_HASTE => self.singing(18) || self.singing(27),
            STATUS_INVULNERABILITY => self.singing(31),
            "rfb.status.hero" | "rfb.status.heroism" => self.singing(7) || self.singing(27),
            "rfb.status.blessed" => self.singing(1),
            _ => false,
        }
    }

    pub(in crate::game) fn music_status(&self) -> Option<StatusInstance> {
        let spell = self.music.spell.filter(|_| !self.music.interrupted)?;
        let source = self
            .content
            .abilities()
            .find(|a| matches!(a.effect, E::Music { spell: slot } if slot == spell))?;
        let mut s = melee_status("rfb.status.music", 1, &source.id).status;
        let has = |kind: &str| self.player.statuses.iter().any(|s| s.kind_id == kind);
        if spell == 1 && !has("rfb.status.blessed") {
            s.granted_modifiers.defense = 5;
            s.granted_equipment_bonuses.melee_skill = 10;
            s.granted_equipment_bonuses.ranged_skill = 10;
        }
        if matches!(spell, 7 | 27) && !has("rfb.status.hero") && !has("rfb.status.heroism") {
            s.granted_modifiers.max_hp = 10;
            s.granted_equipment_bonuses.melee_skill = 12;
            s.granted_equipment_bonuses.ranged_skill = 12;
            s.granted_status_immunities.insert(STATUS_FEAR.into());
        }
        if matches!(spell, 18 | 27) && !has(STATUS_HASTE) {
            s.granted_modifiers.speed = 10;
        }
        if spell == 11 {
            s.granted_equipment_bonuses.stealth_skill = 99;
        }
        if spell == 17 {
            for d in [
                DamageType::Acid,
                DamageType::Electricity,
                DamageType::Fire,
                DamageType::Cold,
                DamageType::Poison,
            ] {
                s.granted_resistances.insert(d, ResistanceLevel::Resistant);
            }
        }
        Some(s)
    }

    pub(in crate::game) fn stop_music(&mut self) {
        if self.music.spell.is_none() {
            return;
        }
        if self.singing(31)
            && !self
                .player
                .statuses
                .iter()
                .any(|s| s.kind_id == STATUS_INVULNERABILITY)
        {
            self.player.energy_need = self.player.energy_need.saturating_add(100);
        }
        self.music.spell = None;
        self.music.beats = 0;
        self.music.interrupted = false;
        self.player.hp = self.player.hp.min(self.effective_player_max_hp());
    }

    pub(in crate::game) fn interrupt_music(&mut self) {
        if self.music.spell.is_some() && !self.music.interrupted {
            self.music.interrupted = true;
            self.player.energy_need = self.player.energy_need.saturating_add(100);
            self.player.hp = self.player.hp.min(self.effective_player_max_hp());
        }
    }

    pub(in crate::game) fn music_state_is_valid(&self) -> bool {
        let Some(spell) = self.music.spell else {
            return self.music.beats == 0
                && !self.music.interrupted
                && (!self.music.half_mana || self.player_is_bard());
        };
        self.player_is_bard()
            && spell < 32
            && continuous(spell)
            && self.music.beats <= 19
            && (spell == 8 || self.music.beats == 0)
            && self.content.abilities().any(|a| {
                matches!(a.effect, E::Music { spell: s } if s == spell)
                    && self.learned_abilities.contains(&a.id)
            })
    }

    fn music_effect(&self, spell: u8) -> E {
        let l = self.progress.level;
        let bonus = self.casting_spell_damage_bonus();
        match spell {
            2 => E::Damage {
                damage_dice: 4 + (l - 1) / 5,
                damage_sides: 4,
                damage_bonus: bonus,
                damage_type: D::Sound,
            },
            5 => E::LightArea {
                damage_dice: 2,
                damage_sides: l / 2,
                radius: (l / 10 + 1) as u8,
                sunlight_burn_damage_dice: 0,
                sunlight_burn_damage_sides: 0,
            },
            14 => self
                .content
                .ability("demo.ability.death-animate-dead")
                .unwrap()
                .effect
                .clone(),
            22 => E::BeamDamage {
                damage_dice: 15 + (l - 1) / 2,
                damage_sides: 10,
                damage_bonus: bonus,
                damage_type: D::Sound,
                maximum_range: None,
            },
            23 => E::AlterReality,
            26 => E::CreateCurrentTerrain {
                source_terrain_ids: vec!["demo.terrain.floor".into()],
                target_terrain_id: "demo.terrain.warding-glyph".into(),
            },
            29 => E::RestoreVitality {
                life_force: 1000,
                restore_attributes: true,
            },
            30 => E::AreaDamage {
                damage_dice: 50 + l,
                damage_sides: 10,
                damage_bonus: bonus,
                damage_type: D::Sound,
                radius: 0,
                target_category: None,
            },
            _ => E::SatisfyHunger,
        }
    }

    pub(super) fn music_target_plan(
        &self,
        ability: &AbilityDefinition,
        spell: u8,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        let mut a = ability.clone();
        a.effect = self.music_effect(spell);
        self.ability_target_plan(&a, target)
    }

    pub(super) fn resolve_music(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        if continuous(spell) {
            self.music.spell = Some(spell);
            self.music.beats = 0;
            self.music.interrupted = false;
            if matches!(spell, 7 | 27) {
                self.apply_player_healing(10);
            }
            if spell == 7 {
                self.player.statuses.retain(|s| s.kind_id != STATUS_FEAR);
            }
            if matches!(spell, 10 | 16) {
                self.music_pulse(ability, spell, events, changed, removed)?;
            }
            return Ok(None);
        }
        if spell == 19 {
            let bonus = ability.spell_power_bonus;
            let radius = spell_power_value(u64::from(self.progress.level / 15 + 1), bonus) as u8;
            let power = spell_power_value(u64::from(self.progress.level * 3 + 1), bonus) as u16;
            let ids: Vec<_> = self
                .entities
                .iter()
                .filter(|e| {
                    crate::game::projectile_geometry::rfb_distance(self.player.position, e.position)
                        <= u32::from(radius)
                })
                .map(|e| e.id.clone())
                .collect();
            let mut resolutions = Vec::new();
            for id in ids {
                let index = self.entities.iter().position(|e| e.id == id).unwrap();
                if !crate::game::projectile_geometry::has_line_of_effect(
                    self,
                    self.player.position,
                    self.entities[index].position,
                ) {
                    continue;
                }
                let distance = crate::game::projectile_geometry::rfb_distance(
                    self.player.position,
                    self.entities[index].position,
                );
                let power = ((u32::from(power) + distance) / (distance + 1)) as u16;
                resolutions.push(self.resolve_teleport_away_target(index, 0, 0, power, changed));
            }
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: rfb_protocol::AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: resolutions,
                },
                trace: None,
            });
            return Ok(None);
        }
        let mut a = ability.clone();
        a.effect = self.music_effect(spell);
        if spell == 5 {
            a.spell_power_bonus = 0;
        }
        if matches!(spell, 2 | 22 | 30) {
            a.spell_power_fields
                .push(rfb_content::AbilitySpellPowerDefinition {
                    effect_index: 0,
                    field: rfb_content::AbilitySpellPowerField::FinalDamage,
                });
        }
        if spell == 2 {
            a.affects_ground_items = false;
        }
        self.resolve_player_ability_effect(a, plan, events, changed, removed)
    }

    pub(in crate::game) fn advance_music(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(spell) = self.music.spell else {
            return Ok(());
        };
        if self.player_is_dead() || self.player_has_anti_magic() {
            self.stop_music();
            return Ok(());
        }
        let mut a = self
            .content
            .abilities()
            .find(|a| matches!(a.effect, E::Music { spell: s } if s == spell))
            .expect("validated song")
            .clone();
        let progress = self.ability_progress_value(&a);
        let half_cost = self.ability_effective_resource_cost(&a, progress);
        let mana = self
            .resources
            .get_mut("demo.resource.mana")
            .expect("Bard mana");
        let available = (mana.current * 2 + u32::from(self.music.half_mana)).min(mana.maximum * 2);
        if available < half_cost {
            self.stop_music();
            return Ok(());
        }
        let remaining = available - half_cost;
        mana.current = remaining / 2;
        self.music.half_mana = remaining % 2 == 1;
        self.music.interrupted = false;
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
        Self::apply_player_spell_power(&mut a, self.effective_player_spell_power_bonus());
        self.music_pulse(&a, spell, events, changed, removed)?;
        Ok(())
    }

    fn music_pulse(
        &mut self,
        ability: &AbilityDefinition,
        spell: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let l = self.progress.level;
        let sp = |v: u16| {
            spell_power_value(u64::from(v), ability.spell_power_bonus).min(u64::from(u16::MAX))
                as u16
        };
        let status = |kind: &str, power| E::VisibleApplyStatus {
            status_kind_id: kind.into(),
            intensity: 1,
            duration_ticks: 25,
            duration_dice: 0,
            duration_sides: 0,
            stacking: rfb_content::AbilityStatusStackingDefinition::Extend,
            resistance_type: None,
            power: Some(power),
            target_category: None,
        };
        let bonus = self.casting_spell_damage_bonus();
        let damage = |sides: u16, kind, category| E::VisibleDamage {
            damage_dice: 1,
            damage_sides: sides,
            damage_bonus: bonus,
            damage_type: kind,
            target_category: category,
            unlife_change_on_hit: 0,
        };
        let mut effects = match spell {
            0 => vec![status("rfb.status.slow", l)],
            3 => {
                let roll = self.roll_damage(sp(l / 10), 2).max(0) as u16;
                vec![status("rfb.status.stun", roll)]
            }
            4 => vec![E::HealDice {
                dice: 2,
                sides: sp(6),
            }],
            6 => vec![status(STATUS_FEAR, sp(l))],
            9 => vec![damage(l * 3 / 2, D::Psi, None)],
            12 => vec![status("rfb.status.confusion", l * 2)],
            13 => vec![E::VisibleDamage {
                damage_dice: 10 + l / 5,
                damage_sides: 7,
                damage_bonus: bonus,
                damage_type: D::Sound,
                target_category: None,
                unlife_change_on_hit: 0,
            }],
            15 => {
                let power = self.roll_damage(sp(10 + l / 15), 6).max(0) as u16;
                vec![E::Domination { power, mass: true }]
            }
            20 => vec![
                damage(l * 3, D::Physical, None),
                damage(l * 3, D::Physical, Some("evil".into())),
            ],
            21 => vec![
                status("rfb.status.slow", sp(l)),
                status("rfb.status.sleep", sp(l)),
            ],
            24 => vec![
                self.content
                    .ability("demo.ability.nature-earthquake")
                    .unwrap()
                    .effect
                    .clone(),
            ],
            25 => vec![status("rfb.status.paralysis", sp(l * 3 + 10))],
            27 => vec![damage(l * 3, D::Physical, None)],
            28 => vec![
                E::HealDice {
                    dice: sp(15),
                    sides: 10,
                },
                E::RemoveStatus {
                    status_kind_id: "rfb.status.stun".into(),
                },
                E::RemoveStatus {
                    status_kind_id: "rfb.status.bleeding".into(),
                },
            ],
            _ => Vec::new(),
        };
        if spell == 8 {
            let count = self.music.beats;
            use rfb_content::AbilityDetectSubjectDefinition as S;
            let detect = |subject, category: &str, persistent| E::Detect {
                subject,
                category: category.into(),
                radius: 30,
                persistent,
                through_walls: true,
            };
            for category in ["trap", "door", "stairs-up", "stairs-down"] {
                effects.push(detect(S::Terrain, category, true));
            }
            if count >= 3 {
                effects.push(detect(S::Actor, "any-monster", false));
            }
            if count >= 6 {
                effects.push(detect(S::Item, "item", false));
                effects.push(detect(S::Terrain, "gold", true));
            }
            if count >= 11 {
                effects.push(detect(S::Terrain, "map", true));
            }
            if count >= 19 {
                effects.push(E::Clairvoyance {
                    telepathy_duration_ticks: 0,
                    telepathy_duration_dice: 0,
                    telepathy_duration_sides: 0,
                    grants_virtues: false,
                    grants_telepathy: false,
                });
            }
            let cap = if l >= 40 {
                19
            } else if l >= 25 {
                11
            } else if l >= 20 {
                6
            } else if l >= 15 {
                3
            } else {
                0
            };
            self.music.beats = count + u8::from(count < cap);
        }
        if spell == 10 {
            let ids: Vec<_> = self.items.iter().filter(|i| matches!(i.location, ItemLocation::Ground(p) if crate::game::projectile_geometry::rfb_distance(p, self.player.position) <= 1)).map(|i| i.id.clone()).collect();
            for id in ids {
                self.identify_item_instance(
                    &id,
                    crate::game::inventory::ItemIdentificationRequest::new(false),
                );
            }
        }
        if spell == 16 {
            self.resolve_ground_item_projectile_effects(
                &ability.id,
                &[self.player.position],
                DamageType::Disintegrate,
                true,
                events,
                changed,
                removed,
            );
        }
        for effect in effects {
            let mut a = ability.clone();
            a.effect = effect;
            a.spell_power_bonus = if matches!(spell, 9 | 13 | 20 | 27) {
                ability.spell_power_bonus
            } else {
                0
            };
            if matches!(spell, 9 | 13 | 20 | 27) {
                a.spell_power_fields
                    .push(rfb_content::AbilitySpellPowerDefinition {
                        effect_index: 0,
                        field: rfb_content::AbilitySpellPowerField::FinalDamage,
                    });
            }
            if let Some(plan) = self.ability_target_plan(&a, &TargetSelection::SelfTarget) {
                self.resolve_player_ability_effect(a, plan, events, changed, removed)?;
            }
        }
        Ok(())
    }
}

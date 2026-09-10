// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378: duelist.c. Challenge identity belongs to Game, not the aim cursor.

use crate::game::projectile_geometry::{projectile_path_through_target, rfb_distance};
use crate::game::visibility::has_line_of_sight;
use crate::game::*;

impl Game {
    pub(in crate::game) fn class_ability_hit_point_cost(
        &self,
        activation: &rfb_content::ClassAbilityDefinition,
    ) -> u32 {
        let cost = activation.hit_point_cost;
        if cost > 0
            && self.player_is_duelist()
            && self
                .player_equipment_passives()
                .contains(&EquipmentPassive::ReducedManaCost)
        {
            (cost * 3 / 4).max(1)
        } else {
            cost
        }
    }

    pub(in crate::game) fn duelist_charge_range(effect: &AbilityEffectDefinition) -> Option<u16> {
        match effect {
            AbilityEffectDefinition::DuelistCharge
            | AbilityEffectDefinition::DuelistDartingDuel => Some(5),
            AbilityEffectDefinition::DuelistAcrobaticCharge => Some(7),
            AbilityEffectDefinition::DuelistPhaseCharge => Some(10),
            _ => None,
        }
    }

    pub(super) fn resolve_duelist_isolation(
        &mut self,
        ability_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let power = self.progress.level * 4;
        let ids: Vec<_> = self
            .entities
            .iter()
            .filter(|actor| {
                actor.hp > 0
                    && !self.duelist_opponent(&actor.id)
                    && has_line_of_sight(self, self.player.position, actor.position)
            })
            .map(|actor| actor.id.clone())
            .collect();
        let floor_id = self.current_floor_id.clone();
        let mut effects = Vec::new();
        for id in ids {
            if self.player_is_dead() || self.current_floor_id != floor_id {
                break;
            }
            let Some(index) = self.entities.iter().position(|actor| actor.id == id) else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("isolation target exists");
            let resistant = definition.tags.iter().any(|tag| tag == "resist-teleport");
            let immune = resistant
                && definition
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "unique" | "resist-all"));
            let resisted = immune || (resistant && self.duelist_monster_saves(index));
            let from = self.entities[index].position;
            if !resisted {
                if self.riding_actor_id.as_ref() == Some(&id) {
                    self.resolve_player_teleport_with_range(
                        ability_id, power, false, true, None, events, changed,
                    );
                } else {
                    self.passive_teleport_actor(index, u32::from(power), changed);
                }
            }
            let to = self
                .entities
                .iter()
                .find(|actor| actor.id == id)
                .map(|actor| actor.position)
                .filter(|to| *to != from);
            effects.push(AbilityEffectResolutionDto::TeleportAway {
                effect_index: 0,
                target_entity_id: id,
                power,
                resistance_roll: None,
                resisted,
                from,
                to,
            });
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability_id.to_owned(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects,
            },
            trace: None,
        });
    }

    pub(super) fn resolve_duelist_charge(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        let target_id = self
            .duelist_target_id
            .clone()
            .expect("charge requires challenge");
        let target = self
            .entities
            .iter()
            .find(|actor| actor.id == target_id)
            .expect("charge target exists")
            .position;
        let range = Self::duelist_charge_range(&ability.effect).expect("charge effect has range");
        let acrobatic = matches!(
            ability.effect,
            AbilityEffectDefinition::DuelistAcrobaticCharge
        );
        let phase = matches!(ability.effect, AbilityEffectDefinition::DuelistPhaseCharge);
        let origin = self.player.position;
        let floor_id = self.current_floor_id.clone();
        let distance = origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y)) as u16;
        let path = projectile_path_through_target(origin, target, range.max(distance))
            .expect("distinct visible charge target");
        let mut landing = origin;
        let mut moved_at_monster = false;
        let mut succeeded = false;
        for position in path
            .into_iter()
            .take(usize::from(range))
            .take_while(|position| rfb_distance(origin, *position) <= u32::from(range))
        {
            let Some(cell) = self.index(position) else {
                break;
            };
            let terrain = self
                .content
                .terrain(&self.terrain[cell])
                .expect("charge terrain exists");
            let occupant = self
                .entities
                .iter()
                .position(|actor| actor.hp > 0 && actor.position == position);
            let can_phase = phase
                && if self.riding_actor_id.is_some() {
                    movement::actor_can_cross_terrain_with_wall_passage(
                        self.active_traveler_definition(),
                        terrain,
                        true,
                    )
                } else {
                    terrain.allows_wall_passage
                };
            if occupant.is_none() && (self.player_can_cross_terrain(terrain) || can_phase) {
                landing = position;
                continue;
            }
            let Some(index) = occupant else {
                break;
            };
            let actor_id = self.entities[index].id.clone();
            if landing != self.player.position {
                events.extend(self.relocate_player(landing, changed));
            }
            moved_at_monster = true;
            if self.player_is_dead()
                || self.current_floor_id != floor_id
                || self.player.position != landing
            {
                break;
            }
            let Some(index) = self
                .entities
                .iter()
                .position(|actor| actor.id == actor_id && actor.hp > 0)
            else {
                break;
            };
            if actor_id != target_id && acrobatic {
                self.wake_entity(index, events);
                self.entities[index].position = landing;
                events.extend(self.relocate_player(position, changed));
                landing = position;
                if self.player_is_dead()
                    || self.current_floor_id != floor_id
                    || self.player.position != landing
                {
                    break;
                }
                continue;
            }
            succeeded = self.duelist_target_id.as_ref() == Some(&target_id);
            if self.player_fear_blocks_melee(index) {
                events.push(DomainEvent::PlayerFearBlocked {
                    status_kind_id: STATUS_FEAR.to_owned(),
                });
            } else {
                let energy = self.player.energy_need;
                self.resolve_player_melee(index, true, events, changed, removed_entities)?;
                self.player.energy_need = energy;
            }
            break;
        }
        if !moved_at_monster && self.player.position != landing {
            events.extend(self.relocate_player(landing, changed));
        }
        if self.duelist_prompt().is_some() {
            self.continue_after_duelist_choice(rfb_protocol::DuelistContinuationDto::Charge {
                ability_id: ability.id.clone(),
                target_entity_id: target_id,
                floor_id,
                succeeded,
            });
            return Ok(None);
        }
        self.finish_duelist_charge(
            &ability.id,
            &target_id,
            &floor_id,
            succeeded,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)] // Keeps the charge identity and floor across an in-action choice.
    pub(in crate::game) fn finish_duelist_charge(
        &mut self,
        ability_id: &str,
        target_id: &str,
        floor_id: &str,
        succeeded: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<Position>, CoreError> {
        if self.player_is_dead() || self.current_floor_id != floor_id {
            return Ok(None);
        }
        let translation = match self
            .scroll_wilderness_for_player_entry(self.player.position, removed_entities)?
        {
            wilderness::WildernessPlayerEntry::Local { translation, .. } => translation,
            wilderness::WildernessPlayerEntry::Blocked => None,
        };
        if let Some(translation) = translation {
            self.populate_scrolled_wilderness(translation);
        }
        if ability_id == "demo.ability.duelist-darting-duel"
            && succeeded
            && self.duelist_target_id.as_deref() == Some(target_id)
        {
            if let Some(actor) = self.entities.iter_mut().find(|actor| actor.id == target_id) {
                actor.anger = actor.anger.saturating_add(10 + actor.anger / 2).min(100);
            }
            self.resolve_player_teleport_with_range(
                ability_id, 10, true, false, None, events, changed,
            );
        }
        Ok(translation)
    }

    pub(in crate::game) fn duelist_opponent(&self, entity_id: &str) -> bool {
        self.player_is_duelist() && self.duelist_target_id.as_deref() == Some(entity_id)
    }

    pub(in crate::game) fn duelist_reduce_damage(&self, entity_id: &str, damage: i32) -> i32 {
        if self.duelist_opponent(entity_id) {
            damage - damage / 3
        } else {
            damage
        }
    }

    pub(in crate::game) fn duelist_auto_challenge(
        &mut self,
        index: usize,
        weapon: bool,
        retaliation: bool,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        if !self.player_is_duelist()
            || self.duelist_equipment_error().is_some()
            || self.duelist_opponent(&self.entities[index].id)
            || (retaliation && self.duelist_target_id.is_some())
        {
            return false;
        }
        let actor = &self.entities[index];
        let definition = self
            .actor_runtime_definition(actor)
            .expect("melee actor exists");
        let level = u32::from(self.progress.level);
        let qualifies = if weapon {
            definition.level >= level * 4 / 5
                || actor.max_hp >= (level * 9) as i32
                || (level < 35 && definition.id == "demo.actor.ochre-jelly")
        } else {
            actor.max_hp > 100 && (definition.level >= level * 4 / 5 || actor.max_hp > 1000)
        };
        if qualifies {
            self.resolve_duelist_challenge(actor.id.clone(), events);
        }
        qualifies
    }

    // monster1.c::mon_save_p(A_DEX): draw player power first, then monster power.
    pub(in crate::game) fn duelist_monster_saves(&mut self, index: usize) -> bool {
        let actor = self
            .actor_runtime_definition(&self.entities[index])
            .expect("target exists");
        let monster_power = (actor.level
            + if actor.tags.iter().any(|tag| tag == "unique") {
                actor.level / 5
            } else {
                0
            })
        .max(1);
        let dex = self
            .effective_player_attributes()
            .index(AttributeKind::Dexterity);
        let player_power =
            (self.progress.level as i32 + crate::stats::original_save_adjustment(dex)).max(1);
        self.rng.bounded(player_power as u64) <= self.rng.bounded(u64::from(monster_power))
    }

    pub(in crate::game) fn duelist_strike_damage(
        &mut self,
        index: usize,
        base: i32,
    ) -> (i32, Option<i32>) {
        let level = self.progress.level;
        let mut damage = base + if level >= 10 { base } else { 0 };
        let unique = self
            .actor_runtime_definition(&self.entities[index])
            .expect("target exists")
            .tags
            .iter()
            .any(|tag| tag == "unique");
        if level >= 15 && !unique && !self.duelist_monster_saves(index) {
            self.apply_actor_melee_status(index, STATUS_SLOW, 50, "demo.class.duelist");
        }
        if level >= 25
            && !self.actor_has_status_immunity(index, STATUS_STUN)
            && !self.duelist_monster_saves(index)
        {
            let old = self.entities[index]
                .statuses
                .iter()
                .find(|status| status.kind_id == STATUS_STUN)
                .map_or(0, |status| status.remaining_ticks);
            let amount =
                (player_combat::monster_stun_amount(base) / (1 + (old / 20) as i32)).max(1);
            self.apply_actor_melee_status(index, STATUS_STUN, amount, "demo.class.duelist");
        }
        let mut drain = None;
        if level >= 20 && !self.duelist_monster_saves(index) {
            damage += (self.entities[index].hp / 5).min((self.rng.bounded(3) as i32 + 1) * base);
            drain = Some(damage);
        }
        if level >= 40 && !self.duelist_monster_saves(index) {
            damage +=
                (self.entities[index].hp * 2 / 5).min((self.rng.bounded(5) as i32 + 2) * base);
            drain = Some(damage);
        }
        (damage, drain)
    }

    pub(in crate::game) fn player_is_duelist(&self) -> bool {
        self.build
            .as_ref()
            .is_some_and(|build| build.class_id == "demo.class.duelist")
    }

    pub(in crate::game) fn duelist_equipment_error(&self) -> Option<&'static str> {
        let equipped = || {
            self.items
                .iter()
                .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        };
        let kind = |item: &ItemInstance| {
            self.content
                .item(&item.kind_id)
                .and_then(|definition| definition.rfb_base_kind)
        };
        let armor_weight: u32 = equipped()
            .filter(|item| kind(item).is_some_and(|kind| (30..=38).contains(&kind.tval)))
            .map(|item| u32::from(self.item_instance_weight(item)))
            .sum();
        if armor_weight > 120 + 3 * u32::from(self.progress.level) {
            Some("duelist-heavy-armor")
        } else if equipped().any(|item| kind(item).is_some_and(|kind| matches!(kind.tval, 11 | 34)))
        {
            Some("duelist-shield")
        } else if self.equipped_melee_weapons().len() > 1 {
            Some("duelist-multiple-weapons")
        } else if equipped()
            .any(|item| kind(item).is_some_and(|kind| (kind.tval, kind.sval) == (23, 32)))
        {
            Some("duelist-poison-needle")
        } else if self
            .player_equipment_passives()
            .contains(&EquipmentPassive::AntiMagic)
        {
            // The timed anti-magic status represents tim_no_spells, not an equipment error.
            Some("anti-magic")
        } else {
            None
        }
    }

    pub(in crate::game) fn duelist_challenge_is_valid(&self) -> bool {
        self.duelist_target_id.as_ref().is_none_or(|id| {
            self.player_is_duelist()
                && self.duelist_equipment_error().is_none()
                && self
                    .entities
                    .iter()
                    .any(|entity| entity.id == *id && entity.hp > 0)
        })
    }

    pub(in crate::game) fn refresh_duelist_challenge(&mut self) {
        if !self.duelist_challenge_is_valid() {
            self.duelist_target_id = None;
        }
    }

    pub(in crate::game) fn clear_duelist_challenge_for(&mut self, entity_id: &str) {
        if self.duelist_target_id.as_deref() == Some(entity_id) {
            self.duelist_target_id = None;
        }
    }

    pub(in crate::game) fn duelist_challenge_target(
        &self,
        target: &TargetSelection,
    ) -> Option<String> {
        let TargetSelection::Entity { entity_id } = target else {
            return None;
        };
        self.entities
            .iter()
            .find(|entity| {
                entity.id == *entity_id
                    && entity.hp > 0
                    // Mounted actors must remain controlled in the current riding model.
                    // Dismount before making this actor hostile; never leave overlapping actors.
                    && self.riding_actor_id.as_ref() != Some(entity_id)
                    && self.duelist_target_id.as_ref() != Some(entity_id)
                    && self.entity_is_visible_to_player(entity)
            })
            .map(|entity| entity.id.clone())
    }

    pub(in crate::game) fn duelist_cast_is_zero_time_unavailable(
        &self,
        ability_id: &str,
        target: &TargetSelection,
    ) -> bool {
        if !self.player_is_duelist() {
            return false;
        }
        let Some(activation) = self.class_ability_activation(ability_id) else {
            return false;
        };
        self.progress.level < activation.minimum_level
            || self.player_has_status_kind(STATUS_CONFUSION)
            || self.player_has_status_kind(STATUS_FEAR)
            || i64::from(self.player.hp) < i64::from(self.class_ability_hit_point_cost(activation))
            || self
                .content
                .ability(ability_id)
                .is_none_or(|ability| self.ability_target_plan(ability, target).is_none())
    }

    pub(super) fn resolve_duelist_challenge(
        &mut self,
        target_entity_id: String,
        events: &mut Vec<DomainEvent>,
    ) {
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == target_entity_id)
            .expect("validated challenge target must remain available");
        self.wake_entity(index, events);
        self.entities[index].alerted = true;
        self.entities[index].friendly = false;
        self.entities[index].controller_id = None;
        self.clear_riding_bond_for(&target_entity_id);
        self.duelist_target_id = Some(target_entity_id);
        events.push(DomainEvent::DuelistChallengeIssued {
            target_kind_id: self.entities[index].kind_id.clone(),
        });
    }
}

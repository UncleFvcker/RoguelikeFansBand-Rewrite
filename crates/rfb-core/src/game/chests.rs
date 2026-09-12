// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378: cmd2.c chest_death/chest_trap, tables.c indices 1..15.

use super::inventory::ItemIdentificationRequest;
use super::loot::{ItemGenerationMode, LootContext, LootSource};
use super::{Game, chebyshev_distance};
use crate::effect::{STATUS_BLINDNESS, STATUS_CONFUSION, STATUS_HALLUCINATION, STATUS_POISON};
use crate::state::{ItemInstance, ItemLocation};
use crate::stats::AttributeKind;
use crate::{CoreError, event::DomainEvent};
use rfb_protocol::{ChestDto, Position};
use std::collections::BTreeSet;

const CHEST: &str = "demo.item.large-wooden-chest";

fn has_trap(difficulty: i16) -> bool {
    difficulty > 0 && difficulty != 6
}

fn message(events: &mut Vec<DomainEvent>, key: &str) {
    events.push(DomainEvent::ChestInteracted {
        message_key: key.into(),
    });
}

impl Game {
    fn chest_is_known(&self, item: &ItemInstance) -> bool {
        self.item_property_knowledge
            .get(&item.id)
            .is_some_and(|known| known.appraised)
    }

    fn chest_is_reachable(&self, item: &ItemInstance) -> bool {
        let ItemLocation::Ground(position) = item.location else {
            return false;
        };
        self.map_scale == rfb_protocol::MapScaleDto::Local
            && !self.player_is_dead()
            && chebyshev_distance(self.player.position, position) <= 1
            && self.item_is_discovered(&item.id)
            && !self
                .entities
                .iter()
                .any(|actor| actor.hp > 0 && actor.position == position)
    }

    pub(super) fn chest_dto(&self, item: &ItemInstance) -> Option<ChestDto> {
        let chest = item.chest?;
        let reachable = self.chest_is_reachable(item);
        Some(ChestDto {
            empty: chest.difficulty == 0,
            can_open: reachable && chest.difficulty != 0,
            can_disarm: reachable && self.chest_is_known(item) && has_trap(chest.difficulty),
        })
    }

    fn chest_skill(&self, mut skill: i32) -> i32 {
        if self.player_has_status_kind(STATUS_BLINDNESS)
            || !self.position_is_lit(self.player.position)
        {
            skill /= 10;
        }
        if self.player_has_status_kind(STATUS_CONFUSION)
            || self.player_has_status_kind(STATUS_HALLUCINATION)
        {
            skill /= 10;
        }
        skill
    }

    pub(super) fn search_chest_traps(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let skill = self
            .chest_skill(self.player_derived_stats().search_skill.value)
            .max(0);
        let chests = self
            .items
            .iter()
            .filter(|item| {
                item.chest.is_some_and(|chest| has_trap(chest.difficulty))
                    && !self.chest_is_known(item)
                    && self.chest_is_reachable(item)
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        let mut found = false;
        for id in chests {
            if self.rng.bounded(100) >= skill as u64 {
                continue;
            }
            self.identify_item_instance(&id, ItemIdentificationRequest::new(false));
            let item = self
                .items
                .iter()
                .find(|item| item.id == id)
                .expect("searched chest exists");
            if let ItemLocation::Ground(position) = item.location {
                changed.insert(position);
            }
            message(events, "chest-trap-found");
            found = true;
        }
        found
    }

    pub(super) fn interact_chest(
        &mut self,
        item_id: &str,
        disarm: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let Some(index) = self.items.iter().position(|item| {
            item.id == item_id && item.chest.is_some() && self.chest_is_reachable(item)
        }) else {
            message(events, "chest-unavailable");
            return Ok(());
        };
        let chest = self.items[index].chest.expect("checked chest");
        let ItemLocation::Ground(position) = self.items[index].location else {
            unreachable!()
        };
        let skill = self.chest_skill(self.player_derived_stats().disarm_skill.value);
        let chance = (skill - i32::from(chest.difficulty)).max(2) as u64;
        if disarm {
            if !self.chest_is_known(&self.items[index]) {
                message(events, "chest-trap-unknown");
            } else if !has_trap(chest.difficulty) {
                message(events, "chest-no-trap");
            } else if self.rng.bounded(100) < chance {
                self.items[index]
                    .chest
                    .as_mut()
                    .expect("chest exists")
                    .difficulty = -chest.difficulty;
                self.apply_player_experience(chest.difficulty as u64, events);
                changed.insert(position);
                message(events, "chest-disarmed");
            } else if skill > 5 && self.rng.bounded(skill as u64) + 1 > 5 {
                message(events, "chest-disarm-failed");
            } else {
                message(events, "chest-trap-triggered");
                self.trigger_chest_trap(item_id, position, events, changed)?;
            }
        } else if chest.difficulty == 0 {
            message(events, "chest-empty");
        } else if chest.difficulty > 0 && self.rng.bounded(100) >= chance {
            message(events, "chest-unlock-failed");
        } else {
            if chest.difficulty > 0 {
                self.apply_player_experience(1, events);
                message(events, "chest-unlocked");
            }
            self.trigger_chest_trap(item_id, position, events, changed)?;
            self.open_chest_contents(item_id, position, false, changed)?;
            message(events, "chest-opened");
        }
        Ok(())
    }

    fn trigger_chest_trap(
        &mut self,
        item_id: &str,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let chest = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.chest)
            .expect("chest trap target exists");
        let difficulty = chest.difficulty;
        if difficulty <= 0 {
            return Ok(());
        }
        // Each needle damages first, then applies the existing sustain/stat drain rules.
        for (attribute, applies) in [
            (
                AttributeKind::Strength,
                matches!(difficulty, 2 | 4 | 9 | 13 | 14),
            ),
            (
                AttributeKind::Constitution,
                matches!(difficulty, 3 | 5 | 10 | 13 | 14),
            ),
        ] {
            if applies {
                message(events, "chest-needle");
                let damage = self.roll_damage(1, 4) as u32;
                self.resolve_item_life_loss(CHEST, damage, events);
                self.resolve_item_drain_attribute(CHEST, attribute, events);
            }
        }
        match difficulty {
            1 | 11 => {
                message(events, "chest-poison");
                self.resolve_item_status(
                    CHEST,
                    STATUS_POISON,
                    1,
                    20,
                    10,
                    rfb_content::AbilityStatusStackingDefinition::Extend,
                    Some(rfb_content::ActorDamageType::Poison),
                    &Default::default(),
                    &Default::default(),
                    &Default::default(),
                    100,
                    events,
                );
            }
            7 | 8 => {
                message(events, "chest-alarm");
                self.aggravate_monsters(None, CHEST, changed);
            }
            12 => {
                message(events, "chest-scatter");
                self.open_chest_contents(item_id, position, true, changed)?;
            }
            15 => {
                message(events, "chest-summon");
                let count = 3 + self.rng.bounded(3);
                let depth = self.floor_depth(&self.current_floor_id);
                for _ in 0..count {
                    if self.rng.bounded(100) + 1 < u64::from(depth) {
                        self.ty_curse_high_summon(CHEST, depth, events, changed);
                    } else {
                        self.summon_hostile_category_at(
                            CHEST,
                            "any-monster",
                            chest.opening_depth,
                            true,
                            position,
                            events,
                            changed,
                        );
                    }
                }
            }
            _ => {}
        }
        changed.insert(self.player.position);
        Ok(())
    }

    fn chest_scatter_position(&mut self) -> Option<Position> {
        for _ in 0..200 {
            let position = Position {
                y: self.rng.bounded(u64::from(self.height)) as i32,
                x: self.rng.bounded(u64::from(self.width)) as i32,
            };
            if position != self.player.position
                && self.can_drop_item_at(position)
                && !self
                    .entities
                    .iter()
                    .any(|actor| actor.hp > 0 && actor.position == position)
            {
                return Some(position);
            }
        }
        None
    }

    fn open_chest_contents(
        &mut self,
        item_id: &str,
        position: Position,
        scatter: bool,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let chest = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.chest)
            .expect("opening chest exists");
        // Large wooden sval=5: 3d1 gold and 2d1 good objects; preserve the dice draws.
        let gold_count = self.roll_damage(3, 1);
        let item_count = self.roll_damage(2, 1);
        if chest.difficulty == 0 {
            return Ok(());
        }
        // Command validation reserves capacity for two items and three gold piles.
        // Mark empty before the scatter caller returns to the outer open operation.
        self.items
            .iter_mut()
            .find(|item| item.id == item_id)
            .expect("chest exists")
            .chest
            .as_mut()
            .expect("chest state")
            .difficulty = 0;
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: self.current_floor_id.clone(),
            depth: chest.opening_depth,
            source: LootSource::Chest {
                item_id: item_id.into(),
            },
        };
        for _ in 0..item_count {
            if let Some(draft) = self.generate_one_loot_draft(&context, ItemGenerationMode::Good) {
                let origin = if scatter {
                    self.chest_scatter_position()
                } else {
                    Some(position)
                };
                if let Some(origin) = origin
                    && let Some((dropped_at, _)) = self.drop_generated_item_near(draft, origin)?
                {
                    changed.insert(dropped_at);
                }
            }
        }
        for _ in 0..gold_count {
            let mut pile = self.generate_gold_pile(position, chest.opening_depth, true)?;
            let destination = if scatter {
                self.chest_scatter_position()
            } else {
                self.ground_drop_position(position, false)
            };
            if let Some(destination) = destination {
                pile.position = destination;
                changed.insert(destination);
                self.gold_piles.push(pile);
            }
        }
        self.identify_item_instance(item_id, ItemIdentificationRequest::new(true));
        changed.insert(position);
        Ok(())
    }
}

// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_protocol::{ItemEnchantmentsDto, ItemQualityDto, MonsterPackRoleDto, Position};

use crate::{
    error::CoreError,
    event::DomainEvent,
    state::{Actor, ItemInstance, ItemLocation},
};

use super::{Game, initial_item_curse, initial_item_runtime_state};

struct CarriedDrop {
    item_id: String,
    kind_id: String,
    quantity: u32,
}

struct ActorDeathPlan {
    actor: Actor,
    corpse: Option<ItemInstance>,
    generated_loot: Vec<ItemInstance>,
    carried: Vec<CarriedDrop>,
    has_drops: bool,
    dissolved_pack_id: Option<String>,
    death_event: DomainEvent,
}

impl Game {
    fn plan_actor_death(
        &mut self,
        index: usize,
        death_event: DomainEvent,
    ) -> Result<ActorDeathPlan, CoreError> {
        let actor = self.entities[index].clone();
        let corpse_kind_id = self
            .content
            .actor(&actor.kind_id)
            .and_then(|definition| definition.corpse_item_kind_id.clone());
        let corpse = if let Some(kind_id) = corpse_kind_id {
            let (activation, charges) =
                initial_item_runtime_state(&self.content, &mut self.rng, &kind_id, 1);
            Some(ItemInstance {
                id: self.allocate_item_instance_id()?,
                activation,
                charges,
                device_recovery_progress: 0,
                curse: initial_item_curse(&self.content, &kind_id),
                kind_id,
                quantity: 1,
                quality: ItemQualityDto::Ordinary,
                affix_ids: Vec::new(),
                rolled_affixes: Vec::new(),
                enchantments: ItemEnchantmentsDto::default(),
                location: ItemLocation::Ground(actor.position),
            })
        } else {
            None
        };
        let generated_loot = self.generate_death_loot(&actor)?;
        let mut carried = self
            .items
            .iter()
            .filter_map(|item| match &item.location {
                ItemLocation::CarriedBy { actor_id } if actor_id == &actor.id => {
                    Some(CarriedDrop {
                        item_id: item.id.clone(),
                        kind_id: item.kind_id.clone(),
                        quantity: item.quantity,
                    })
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        carried.sort_by(|left, right| left.item_id.cmp(&right.item_id));
        let has_drops = !carried.is_empty() || !generated_loot.is_empty() || corpse.is_some();
        let dissolved_pack_id = actor
            .pack
            .as_ref()
            .and_then(|pack| (pack.role == MonsterPackRoleDto::Leader).then(|| pack.id.clone()));

        Ok(ActorDeathPlan {
            actor,
            corpse,
            generated_loot,
            carried,
            has_drops,
            dissolved_pack_id,
            death_event,
        })
    }

    pub(super) fn resolve_actor_death(
        &mut self,
        index: usize,
        death_event: DomainEvent,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let plan = self.plan_actor_death(index, death_event)?;
        let ActorDeathPlan {
            actor,
            corpse,
            generated_loot,
            carried,
            has_drops,
            dissolved_pack_id,
            death_event,
        } = plan;

        let removed = self.entities.remove(index);
        debug_assert_eq!(removed.id, actor.id);
        if let Some(pack_id) = dissolved_pack_id {
            for entity in &mut self.entities {
                if entity.pack.as_ref().is_some_and(|pack| pack.id == pack_id) {
                    entity.pack = None;
                }
            }
        }
        removed_entities.push(removed.id.clone());
        events.push(death_event);
        let experience_value = self
            .content
            .actor(&removed.kind_id)
            .expect("removed actor definition must remain available")
            .experience_value;
        self.apply_player_experience(experience_value, events);
        let defeated_guardian = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .and_then(|floor| {
                floor.guardian.as_ref().and_then(|guardian| {
                    (guardian.instance_id == removed.id).then(|| {
                        (
                            floor
                                .dungeon_id
                                .clone()
                                .expect("guardian floor must have a dungeon ID"),
                            floor.id.clone(),
                            guardian.actor_kind_id.clone(),
                        )
                    })
                })
            });
        if let Some((dungeon_id, floor_id, target_kind_id)) = defeated_guardian {
            let state = self
                .dungeon_states
                .get_mut(&dungeon_id)
                .expect("guardian dungeon state must remain available");
            let first_defeat = !state.guardian_defeated;
            if first_defeat {
                state.guardian_defeated = true;
                events.push(DomainEvent::DungeonGuardianDefeated {
                    dungeon_id: dungeon_id.clone(),
                    floor_id,
                    target_kind_id,
                });
                let mirror_ids = self
                    .content
                    .world(&self.world_id)
                    .expect("active world must remain available")
                    .procedural_floors
                    .iter()
                    .filter(|floor| {
                        floor.dungeon_id.as_deref() == Some(dungeon_id.as_str())
                            && floor.final_floor
                    })
                    .filter_map(|floor| {
                        floor
                            .guardian
                            .as_ref()
                            .map(|guardian| guardian.instance_id.as_str())
                    })
                    .collect::<BTreeSet<_>>();
                for floor in self.stored_floors.values_mut() {
                    floor
                        .entities
                        .retain(|entity| !mirror_ids.contains(entity.id.as_str()));
                    floor.items.retain(|item| {
                        !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if mirror_ids.contains(actor_id.as_str()))
                    });
                }
            }
        }
        let defeated_entrance_guardian = self.content.world(&self.world_id).and_then(|world| {
            world.dungeons.iter().find_map(|dungeon| {
                dungeon.entrance_guardian.as_ref().and_then(|guardian| {
                    (self.current_floor_id == world.initial_floor_id
                        && guardian.instance_id == removed.id)
                        .then(|| (dungeon.id.clone(), guardian.actor_kind_id.clone()))
                })
            })
        });
        if let Some((dungeon_id, target_kind_id)) = defeated_entrance_guardian {
            let state = self
                .dungeon_states
                .get_mut(&dungeon_id)
                .expect("entrance guardian dungeon state must remain available");
            if !state.entrance_guardian_defeated {
                state.entrance_guardian_defeated = true;
                events.push(DomainEvent::DungeonEntranceGuardianDefeated {
                    dungeon_id,
                    target_kind_id,
                });
            }
        }

        for CarriedDrop {
            item_id,
            kind_id,
            quantity,
        } in carried
        {
            let item = self
                .items
                .iter_mut()
                .find(|item| item.id == item_id)
                .expect("carried item collected from authoritative item set");
            item.location = ItemLocation::Ground(removed.position);
            events.push(DomainEvent::LootDropped {
                source_kind_id: removed.kind_id.clone(),
                target_kind_id: kind_id,
                quantity,
            });
        }
        for item in generated_loot {
            events.push(DomainEvent::LootDropped {
                source_kind_id: removed.kind_id.clone(),
                target_kind_id: item.kind_id.clone(),
                quantity: item.quantity,
            });
            self.items.push(item);
        }
        if let Some(corpse) = corpse {
            self.items.push(corpse);
        }
        if has_drops {
            changed.insert(removed.position);
        }
        Ok(())
    }
}

// SPDX-License-Identifier: MPL-2.0
//! Persistent knowledge adapted from RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c,
//! cmd4.c knowledge browsers: aware/found objects, known egos, r_sights/r_pkills and max_dlv.
//! Discovery is recorded by rules; projecting or browsing it never changes state.
use super::{Game, floor_dungeon_id};
use crate::{error::CoreError, state::ItemLocation};
use rfb_protocol::{
    DiscoveryDto, DiscoveryEntryDto, DungeonDiscoveryDto, DungeonDiscoverySaveDto,
    ItemIdentificationDto, ItemKnowledgeDto, MapScaleDto, MonsterDiscoveryDto,
    MonsterDiscoverySaveDto, RandomArtifactDiscoveryDto,
};

fn remember(ids: &mut Vec<String>, id: &str) {
    if let Err(index) = ids.binary_search_by(|value| value.as_str().cmp(id)) {
        ids.insert(index, id.to_owned());
    }
}

impl Game {
    pub(super) fn discover_object_kind(&mut self, kind_id: &str) {
        remember(&mut self.discovery.objects, kind_id);
    }

    pub(super) fn discover_item(&mut self, item_id: &str) {
        let Some(item) = self.items.iter().find(|item| item.id == item_id) else {
            return;
        };
        if self.item_knowledge_dto(&item.kind_id) != ItemKnowledgeDto::Aware {
            return;
        }
        let identified = self.item_identification(item) != ItemIdentificationDto::Unexamined;
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("discovered item exists");
        let fixed_artifact = definition.tags.iter().any(|tag| tag == "artifact");
        // An unknown artifact's true base identity must not leak through the ordinary list.
        if fixed_artifact && !identified {
            return;
        }
        remember(&mut self.discovery.objects, &item.kind_id);
        if fixed_artifact {
            remember(&mut self.discovery.artifacts, &item.kind_id);
        }
        let known = self.item_property_knowledge.get(item_id);
        for affix in item.affix_ids.iter().filter(|id| {
            identified || known.is_some_and(|knowledge| knowledge.known_affix_ids.contains(*id))
        }) {
            if self
                .content
                .affix(affix)
                .is_some_and(|definition| definition.rfb_ego.is_some())
            {
                remember(&mut self.discovery.egos, affix);
            }
        }
        if identified && let Some(name) = &item.artifact_name {
            match self
                .discovery
                .random_artifacts
                .binary_search_by(|entry| entry.id.cmp(&item.id))
            {
                Ok(index) => self.discovery.random_artifacts[index].name = name.clone(),
                Err(index) => self.discovery.random_artifacts.insert(
                    index,
                    RandomArtifactDiscoveryDto {
                        id: item.id.clone(),
                        kind_id: item.kind_id.clone(),
                        name: name.clone(),
                    },
                ),
            }
        }
    }

    fn monster_discovery(&mut self, kind_id: &str) -> &mut MonsterDiscoverySaveDto {
        let index = match self
            .discovery
            .monsters
            .binary_search_by(|entry| entry.kind_id.as_str().cmp(kind_id))
        {
            Ok(index) => index,
            Err(index) => {
                self.discovery.monsters.insert(
                    index,
                    MonsterDiscoverySaveDto {
                        kind_id: kind_id.to_owned(),
                        seen: false,
                        kills: 0,
                    },
                );
                index
            }
        };
        &mut self.discovery.monsters[index]
    }

    pub(super) fn record_discovery_kill(&mut self, kind_id: &str) {
        let entry = self.monster_discovery(kind_id);
        entry.kills = entry.kills.saturating_add(1);
    }

    pub(super) fn record_visible_discoveries(&mut self) {
        // Use only the same identities the player can read, excluding hallucination and fuzzy ESP.
        if self.map_scale == MapScaleDto::Local
            && !self.player_has_status_kind("rfb.status.hallucination")
        {
            for entity in self.entities_dto() {
                if self
                    .content
                    .actor(&entity.kind_id)
                    .is_some_and(|actor| actor.role == rfb_content::ActorRole::Monster)
                {
                    self.monster_discovery(&entity.kind_id).seen = true;
                }
            }
        }
        let item_ids = self
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                ) || self
                    .item_property_knowledge
                    .get(&item.id)
                    .is_some_and(|known| known.discovered)
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        for id in item_ids {
            self.discover_item(&id);
        }
        self.record_dungeon_discovery();
    }

    pub(super) fn record_dungeon_discovery(&mut self) {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world exists");
        let Some(dungeon_id) = floor_dungeon_id(world, &self.current_floor_id) else {
            return;
        };
        let depth = self.floor_depth(&self.current_floor_id);
        match self
            .discovery
            .dungeons
            .binary_search_by(|entry| entry.dungeon_id.cmp(&dungeon_id))
        {
            Ok(index) => {
                self.discovery.dungeons[index].max_depth =
                    self.discovery.dungeons[index].max_depth.max(depth)
            }
            Err(index) => self.discovery.dungeons.insert(
                index,
                DungeonDiscoverySaveDto {
                    dungeon_id,
                    max_depth: depth,
                },
            ),
        }
    }

    pub(super) fn discovery_knows_monster(&self, kind_id: &str) -> bool {
        self.probed_actor_kind_ids.contains(kind_id)
            || self
                .discovery
                .monsters
                .binary_search_by(|entry| entry.kind_id.as_str().cmp(kind_id))
                .ok()
                .is_some_and(|index| self.discovery.monsters[index].seen)
    }

    pub(super) fn discovery_dto(&self) -> DiscoveryDto {
        let item_entry = |id: &String| {
            let item = self.content.item(id).expect("discovery item validated");
            DiscoveryEntryDto {
                id: id.clone(),
                name_key: item.name_key.clone(),
                description_key: item.description_key.clone(),
                custom_name: None,
            }
        };
        let mut artifacts = self
            .discovery
            .artifacts
            .iter()
            .map(item_entry)
            .collect::<Vec<_>>();
        artifacts.extend(self.discovery.random_artifacts.iter().map(|entry| {
            let mut projected = item_entry(&entry.kind_id);
            projected.id = entry.id.clone();
            projected.custom_name = Some(entry.name.clone());
            projected
        }));
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world exists");
        DiscoveryDto {
            objects: self.discovery.objects.iter().map(item_entry).collect(),
            artifacts,
            egos: self
                .discovery
                .egos
                .iter()
                .map(|id| {
                    let affix = self.content.affix(id).expect("discovery ego validated");
                    DiscoveryEntryDto {
                        id: id.clone(),
                        name_key: affix.name_key.clone(),
                        description_key: affix.description_key.clone(),
                        custom_name: None,
                    }
                })
                .collect(),
            monsters: self
                .monster_recall_dtos()
                .into_iter()
                .map(|monster| {
                    let kills = self
                        .discovery
                        .monsters
                        .binary_search_by(|entry| entry.kind_id.cmp(&monster.kind_id))
                        .ok()
                        .map_or(0, |index| self.discovery.monsters[index].kills);
                    // Availability also subtracts currently living instances, so it cannot indicate death.
                    let dead = self
                        .defeated_limited_actor_counts
                        .get(&monster.kind_id)
                        .is_some_and(|count| *count > 0)
                        || world.dungeons.iter().any(|dungeon| {
                            dungeon.guardian_actor_kind_id.as_deref() == Some(&monster.kind_id)
                                && self
                                    .dungeon_states
                                    .get(&dungeon.id)
                                    .is_some_and(|state| state.guardian_defeated)
                        });
                    MonsterDiscoveryDto {
                        alive: monster.unique.then_some(!dead),
                        monster,
                        kills,
                    }
                })
                .collect(),
            dungeons: self
                .discovery
                .dungeons
                .iter()
                .map(|entry| {
                    let dungeon = world
                        .dungeons
                        .iter()
                        .find(|dungeon| dungeon.id == entry.dungeon_id)
                        .expect("discovery dungeon validated");
                    let floor = world
                        .procedural_floors
                        .iter()
                        .find(|floor| floor.id == dungeon.root_floor_id)
                        .expect("dungeon root validated");
                    DungeonDiscoveryDto {
                        dungeon_id: entry.dungeon_id.clone(),
                        name_key: floor.name_key.clone(),
                        max_depth: entry.max_depth,
                        conquered: self.dungeon_states[&entry.dungeon_id].guardian_defeated,
                    }
                })
                .collect(),
        }
    }

    pub(super) fn validate_discovery(&self) -> Result<(), CoreError> {
        let archive = &self.discovery;
        let ordered = |ids: &[String]| ids.windows(2).all(|pair| pair[0] < pair[1]);
        if !ordered(&archive.objects)
            || !ordered(&archive.artifacts)
            || !ordered(&archive.egos)
            || archive
                .objects
                .iter()
                .any(|id| self.content.item(id).is_none())
            || archive.artifacts.iter().any(|id| {
                self.content
                    .item(id)
                    .is_none_or(|item| !item.tags.iter().any(|tag| tag == "artifact"))
                    || archive.objects.binary_search(id).is_err()
            })
            || archive.egos.iter().any(|id| {
                self.content
                    .affix(id)
                    .is_none_or(|affix| affix.rfb_ego.is_none())
            })
            || !archive
                .monsters
                .windows(2)
                .all(|pair| pair[0].kind_id < pair[1].kind_id)
            || archive.monsters.iter().any(|entry| {
                (!entry.seen && entry.kills == 0)
                    || self
                        .content
                        .actor(&entry.kind_id)
                        .is_none_or(|actor| actor.role != rfb_content::ActorRole::Monster)
            })
            || !archive
                .random_artifacts
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
            || archive.random_artifacts.iter().any(|entry| {
                entry.id.is_empty()
                    || entry.name.trim().is_empty()
                    || entry.name.chars().count() > 256
                    || self.content.item(&entry.kind_id).is_none()
                    || archive.objects.binary_search(&entry.kind_id).is_err()
            })
        {
            return Err(CoreError::InvalidSave("discovery archive is invalid"));
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world exists");
        if !archive
            .dungeons
            .windows(2)
            .all(|pair| pair[0].dungeon_id < pair[1].dungeon_id)
            || archive.dungeons.iter().any(|entry| {
                !world
                    .dungeons
                    .iter()
                    .any(|dungeon| dungeon.id == entry.dungeon_id)
                    || !world.procedural_floors.iter().any(|floor| {
                        floor.dungeon_id.as_deref() == Some(&entry.dungeon_id)
                            && self.floor_depth(&floor.id) == entry.max_depth
                    })
            })
        {
            return Err(CoreError::InvalidSave("discovery dungeon depth is invalid"));
        }
        Ok(())
    }
}

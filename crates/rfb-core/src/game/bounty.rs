// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::{
    ActorDefinition, ActorRole, ContentCatalog, FloorLifecycle, ProceduralFloorDefinition,
    TownFacilityBountyDefinition, TownFacilityDefinition, WorldDefinition,
};
use rfb_protocol::{
    BountyDailyTargetDto, BountyMissionDto, BountyMissionSaveDto, BountyMissionStatusDto,
    BountyOfficeActionDto, BountyOfficeDto, BountyStateSaveDto, BountyTurnInDto,
    BountyTurnInRewardDto, BountyWantedTargetDto, ItemEnchantmentsDto, ItemQualityDto,
};

use crate::{
    error::CoreError,
    rng::RfbRng,
    save::initial_item_fuel,
    state::{ItemInstance, ItemLocation},
};

use super::{
    Game, initial_item_curse, initial_item_runtime_state,
    inventory::item_instances_stack_compatible, wilderness::WILDERNESS_DAY_TICKS,
};

const BOUNTY_TARGET_ID_MARKER: &str = ".bounty-target.";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct BountyState {
    pub(super) daily_day: u32,
    pub(super) daily_actor_kind_id: String,
    pub(super) completed_wanted_actor_kind_ids: BTreeSet<String>,
    pub(super) mission: Option<BountyMissionState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BountyMissionState {
    pub(super) dungeon_id: String,
    pub(super) floor_id: String,
    pub(super) actor_kind_id: String,
    pub(super) total: u8,
    pub(super) remaining: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BountyOfficeOutcome {
    DailyTurnIn {
        actor_kind_id: String,
        gold: u32,
    },
    WantedTurnIn {
        actor_kind_id: String,
        item_kind_id: String,
    },
    MissionRequested {
        actor_kind_id: String,
        floor_id: String,
        total: u8,
    },
    MissionAbandoned,
    MissionRewarded {
        item_kind_id: String,
    },
}

impl BountyState {
    pub(super) fn from_save(
        saved: BountyStateSaveDto,
        content: &ContentCatalog,
        world: &WorldDefinition,
        wanted_actor_kind_ids: &BTreeSet<String>,
        world_tick: u32,
    ) -> Result<Self, CoreError> {
        let completed_wanted_actor_kind_ids = saved
            .completed_wanted_actor_kind_ids
            .into_iter()
            .collect::<BTreeSet<_>>();
        if completed_wanted_actor_kind_ids
            .iter()
            .any(|actor_id| !wanted_actor_kind_ids.contains(actor_id))
        {
            return Err(CoreError::InvalidSave("bounty wanted state is invalid"));
        }
        let daily = content.actor(&saved.daily_actor_kind_id);
        if saved.daily_day != world_tick / WILDERNESS_DAY_TICKS
            || daily.is_none_or(|actor| {
                actor.role != ActorRole::Monster
                    || actor
                        .tags
                        .iter()
                        .any(|tag| tag == "unique" || tag == "guardian")
                    || actor.remains.as_ref().is_none_or(|remains| {
                        remains.corpse_item_kind_id.is_none()
                            || remains.skeleton_item_kind_id.is_none()
                    })
            })
        {
            return Err(CoreError::InvalidSave("daily bounty state is invalid"));
        }
        let mission = saved
            .mission
            .map(|mission| {
                let floor = world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == mission.floor_id)
                    .filter(|floor| floor.dungeon_id.as_deref() == Some(&mission.dungeon_id));
                let actor = content.actor(&mission.actor_kind_id);
                if !(1..=5).contains(&mission.total)
                    || mission.remaining > mission.total
                    || floor.is_none()
                    || actor.is_none_or(|actor| {
                        actor.role != ActorRole::Monster
                            || actor
                                .tags
                                .iter()
                                .any(|tag| tag == "unique" || tag == "guardian")
                            || actor.allocation.as_ref().is_none_or(|allocation| {
                                allocation.task_id.is_some()
                                    || allocation.wild_only
                                    || allocation.multiplies
                            })
                    })
                {
                    return Err(CoreError::InvalidSave("bounty mission state is invalid"));
                }
                Ok(BountyMissionState {
                    dungeon_id: mission.dungeon_id,
                    floor_id: mission.floor_id,
                    actor_kind_id: mission.actor_kind_id,
                    total: mission.total,
                    remaining: mission.remaining,
                })
            })
            .transpose()?;
        Ok(Self {
            daily_day: saved.daily_day,
            daily_actor_kind_id: saved.daily_actor_kind_id,
            completed_wanted_actor_kind_ids,
            mission,
        })
    }

    pub(super) fn to_save(&self) -> BountyStateSaveDto {
        BountyStateSaveDto {
            daily_day: self.daily_day,
            daily_actor_kind_id: self.daily_actor_kind_id.clone(),
            completed_wanted_actor_kind_ids: self
                .completed_wanted_actor_kind_ids
                .iter()
                .cloned()
                .collect(),
            mission: self.mission.as_ref().map(|mission| BountyMissionSaveDto {
                dungeon_id: mission.dungeon_id.clone(),
                floor_id: mission.floor_id.clone(),
                actor_kind_id: mission.actor_kind_id.clone(),
                total: mission.total,
                remaining: mission.remaining,
            }),
        }
    }
}

impl Game {
    fn wanted_bounty_actor_ids(&self) -> Vec<String> {
        let mut ids = self
            .mogaminator
            .wanted_actor_kind_ids
            .iter()
            .filter_map(|actor_id| self.content.actor(actor_id).map(|actor| (actor_id, actor)))
            .collect::<Vec<_>>();
        ids.sort_by(|(left_id, left), (right_id, right)| {
            (
                left.level,
                left.allocation
                    .as_ref()
                    .map_or(u32::MAX, |entry| entry.legacy_index),
                left_id.as_str(),
            )
                .cmp(&(
                    right.level,
                    right
                        .allocation
                        .as_ref()
                        .map_or(u32::MAX, |entry| entry.legacy_index),
                    right_id.as_str(),
                ))
        });
        ids.into_iter().map(|(id, _)| id.clone()).collect()
    }

    fn bounty_reference_depth(&self) -> u16 {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let floor_depth = |floor_id: &str| {
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == floor_id)
                .map(|floor| floor.depth)
        };
        self.recall
            .as_ref()
            .and_then(|recall| floor_depth(&recall.floor_id))
            .or_else(|| floor_depth(&self.current_floor_id))
            .unwrap_or(self.progress.level)
            .max(3)
    }

    fn select_daily_bounty_actor(&self) -> Option<&ActorDefinition> {
        let reference_depth = self.bounty_reference_depth();
        let minimum_level = u32::from((reference_depth / 2).min(40));
        let maximum_level = u32::from(reference_depth.saturating_add(8).max(10));
        let eligible = |actor: &&ActorDefinition| {
            actor.role == ActorRole::Monster
                && !actor
                    .tags
                    .iter()
                    .any(|tag| tag == "unique" || tag == "guardian")
                && actor.allocation.as_ref().is_some_and(|allocation| {
                    allocation.task_id.is_none()
                        && !allocation.wild_only
                        && !allocation.multiplies
                        && allocation.rarity <= 10
                })
                && actor.remains.as_ref().is_some_and(|remains| {
                    remains.corpse_item_kind_id.is_some() && remains.skeleton_item_kind_id.is_some()
                })
        };
        let mut candidates = self
            .content
            .actor_definitions()
            .filter(eligible)
            .filter(|actor| actor.level >= minimum_level && actor.level <= maximum_level)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            candidates = self.content.actor_definitions().filter(eligible).collect();
        }
        candidates.sort_by_key(|actor| {
            (
                actor.level,
                actor
                    .allocation
                    .as_ref()
                    .map_or(u32::MAX, |entry| entry.legacy_index),
                actor.id.as_str(),
            )
        });
        if candidates.is_empty() {
            return None;
        }
        let day = self.world_tick / WILDERNESS_DAY_TICKS;
        let mut rng = RfbRng::seeded(self.wilderness_seed ^ 0x424F_554E_5459_4441 ^ u64::from(day));
        let index = usize::try_from(rng.bounded(candidates.len() as u64)).ok()?;
        candidates.get(index).copied()
    }

    fn daily_bounty_actor(&self) -> Option<&ActorDefinition> {
        self.content
            .actor(&self.bounty_state.daily_actor_kind_id)
            .or_else(|| self.select_daily_bounty_actor())
    }

    pub(super) fn refresh_daily_bounty_target(&mut self) {
        let day = self.world_tick / WILDERNESS_DAY_TICKS;
        if self.bounty_state.daily_day == day
            && self
                .content
                .actor(&self.bounty_state.daily_actor_kind_id)
                .is_some()
        {
            return;
        }
        let actor_kind_id = self
            .select_daily_bounty_actor()
            .expect("built-in bounty content must retain a daily target")
            .id
            .clone();
        self.bounty_state.daily_day = day;
        self.bounty_state.daily_actor_kind_id = actor_kind_id;
    }

    fn bounty_reward_item_kind_id(bounty: &TownFacilityBountyDefinition, index: usize) -> &str {
        &bounty.wanted_reward_item_kind_ids[index.min(19)]
    }

    fn bounty_mission_reward_item_kind_id<'a>(
        &'a self,
        bounty: &'a TownFacilityBountyDefinition,
        mission: &BountyMissionState,
    ) -> &'a str {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let depth = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == mission.floor_id)
            .map_or(1, |floor| floor.depth);
        let actor_level = self
            .content
            .actor(&mission.actor_kind_id)
            .map_or(1, |actor| u16::try_from(actor.level).unwrap_or(u16::MAX));
        let index = usize::from(depth.max(actor_level) / 5).min(19);
        Self::bounty_reward_item_kind_id(bounty, index)
    }

    pub(super) fn bounty_office_dto(
        &self,
        facility: &TownFacilityDefinition,
    ) -> Option<BountyOfficeDto> {
        let bounty = facility.bounty_office.as_ref()?;
        let daily = self.daily_bounty_actor()?;
        let daily_target = BountyDailyTargetDto {
            actor_kind_id: daily.id.clone(),
            actor_name_key: daily.name_key.clone(),
            corpse_reward: daily.level.saturating_add(2).saturating_mul(50),
            skeleton_reward: daily.level.saturating_add(2).saturating_mul(30),
        };
        let wanted_ids = self.wanted_bounty_actor_ids();
        let wanted_targets = wanted_ids
            .iter()
            .enumerate()
            .map(|(index, actor_id)| {
                let actor = self
                    .content
                    .actor(actor_id)
                    .expect("wanted actor must remain available");
                let reward_item_kind_id =
                    Self::bounty_reward_item_kind_id(bounty, index).to_owned();
                let reward = self
                    .content
                    .item(&reward_item_kind_id)
                    .expect("validated bounty reward must remain available");
                BountyWantedTargetDto {
                    actor_kind_id: actor.id.clone(),
                    actor_name_key: actor.name_key.clone(),
                    completed: self
                        .bounty_state
                        .completed_wanted_actor_kind_ids
                        .contains(&actor.id),
                    reward_item_kind_id,
                    reward_name_key: reward.name_key.clone(),
                }
            })
            .collect::<Vec<_>>();
        let mut turn_ins = self
            .items
            .iter()
            .filter(|item| item.location == ItemLocation::Inventory && item.quantity > 0)
            .filter_map(|item| {
                let actor_id = item.origin_actor_kind_id.as_ref()?;
                let actor = self.content.actor(actor_id)?;
                let definition = self.content.item(&item.kind_id)?;
                if !definition.tags.iter().any(|tag| tag == "corpse") {
                    return None;
                }
                if actor_id == &daily_target.actor_kind_id {
                    let amount = if definition.tags.iter().any(|tag| tag == "skeleton") {
                        daily_target.skeleton_reward
                    } else {
                        daily_target.corpse_reward
                    };
                    return Some(BountyTurnInDto {
                        item_id: item.id.clone(),
                        actor_kind_id: actor.id.clone(),
                        actor_name_key: actor.name_key.clone(),
                        reward: BountyTurnInRewardDto::Gold { amount },
                    });
                }
                let index = wanted_ids
                    .iter()
                    .position(|candidate| candidate == actor_id)?;
                if self
                    .bounty_state
                    .completed_wanted_actor_kind_ids
                    .contains(actor_id)
                {
                    return None;
                }
                let item_kind_id = Self::bounty_reward_item_kind_id(bounty, index).to_owned();
                let reward = self.content.item(&item_kind_id)?;
                Some(BountyTurnInDto {
                    item_id: item.id.clone(),
                    actor_kind_id: actor.id.clone(),
                    actor_name_key: actor.name_key.clone(),
                    reward: BountyTurnInRewardDto::Item {
                        item_kind_id,
                        item_name_key: reward.name_key.clone(),
                    },
                })
            })
            .collect::<Vec<_>>();
        turn_ins.sort_by(|left, right| left.item_id.cmp(&right.item_id));
        let mission = self.bounty_state.mission.as_ref().map(|mission| {
            let world = self
                .content
                .world(&self.world_id)
                .expect("active world must remain available");
            let floor = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == mission.floor_id)
                .expect("validated bounty floor must remain available");
            let actor = self
                .content
                .actor(&mission.actor_kind_id)
                .expect("validated bounty actor must remain available");
            let reward_item_kind_id = self
                .bounty_mission_reward_item_kind_id(bounty, mission)
                .to_owned();
            let reward = self
                .content
                .item(&reward_item_kind_id)
                .expect("validated bounty reward must remain available");
            BountyMissionDto {
                status: if mission.remaining == 0 {
                    BountyMissionStatusDto::RewardAvailable
                } else {
                    BountyMissionStatusDto::Active
                },
                dungeon_id: mission.dungeon_id.clone(),
                floor_id: mission.floor_id.clone(),
                floor_name_key: floor.name_key.clone(),
                depth: floor.depth,
                actor_kind_id: actor.id.clone(),
                actor_name_key: actor.name_key.clone(),
                total: mission.total,
                remaining: mission.remaining,
                reward_item_kind_id,
                reward_name_key: reward.name_key.clone(),
            }
        });
        Some(BountyOfficeDto {
            daily_target,
            wanted_targets,
            turn_ins,
            mission,
        })
    }

    fn bounty_mission_floor(&mut self) -> Option<ProceduralFloorDefinition> {
        let world = self.content.world(&self.world_id)?;
        let eligible = |floor: &&ProceduralFloorDefinition| {
            floor.lifecycle == FloorLifecycle::Dungeon
                && floor.task_id.is_none()
                && !floor.final_floor
                && floor
                    .dungeon_id
                    .as_deref()
                    .is_some_and(|id| self.dungeon_is_active(id))
                && !self.stored_floors.contains_key(&floor.id)
        };
        let floors = world
            .procedural_floors
            .iter()
            .filter(eligible)
            .collect::<Vec<_>>();
        if let Some(recall) = &self.recall {
            let target = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == recall.floor_id)
                .map_or(self.bounty_reference_depth(), |floor| floor.depth);
            let mut recalled_floors = floors
                .iter()
                .copied()
                .filter(|floor| floor.dungeon_id.as_deref() == Some(&recall.dungeon_id))
                .collect::<Vec<_>>();
            recalled_floors.sort_by_key(|floor| {
                (floor.depth.abs_diff(target), floor.depth, floor.id.as_str())
            });
            if let Some(floor) = recalled_floors.first() {
                return Some((*floor).clone());
            }
        }
        let maximum_depth = self.progress.level.saturating_add(5).max(5);
        let mut shallow = floors
            .iter()
            .copied()
            .filter(|floor| floor.depth <= maximum_depth)
            .collect::<Vec<_>>();
        if shallow.is_empty() {
            shallow = floors;
        }
        if shallow.is_empty() {
            return None;
        }
        shallow.sort_by_key(|floor| (floor.depth, floor.id.as_str()));
        let index = usize::try_from(self.rng.bounded(shallow.len() as u64)).ok()?;
        shallow.get(index).map(|floor| (*floor).clone())
    }

    fn bounty_mission_actor(&mut self, depth: u16) -> Option<String> {
        let actor_eligible = |actor: &&ActorDefinition| {
            actor.role == ActorRole::Monster
                && !actor.tags.iter().any(|tag| {
                    matches!(tag.as_str(), "unique" | "guardian" | "no-quest" | "aquatic")
                })
                && actor.allocation.as_ref().is_some_and(|allocation| {
                    allocation.task_id.is_none()
                        && !allocation.wild_only
                        && !allocation.multiplies
                        && allocation.rarity <= 100
                })
        };
        let minimum = u32::from(depth.saturating_sub(8));
        let maximum = u32::from(depth.saturating_add(7));
        let mut actors = self
            .content
            .actor_definitions()
            .filter(actor_eligible)
            .filter(|actor| actor.level >= minimum && actor.level <= maximum)
            .collect::<Vec<_>>();
        if actors.is_empty() {
            let minimum = u32::from(depth.saturating_sub(15));
            let maximum = u32::from(depth.saturating_add(15));
            actors = self
                .content
                .actor_definitions()
                .filter(actor_eligible)
                .filter(|actor| actor.level >= minimum && actor.level <= maximum)
                .collect();
        }
        actors.sort_by_key(|actor| {
            (
                actor.level,
                actor
                    .allocation
                    .as_ref()
                    .map_or(u32::MAX, |entry| entry.legacy_index),
                actor.id.as_str(),
            )
        });
        if actors.is_empty() {
            return None;
        }
        let index = usize::try_from(self.rng.bounded(actors.len() as u64)).ok()?;
        actors.get(index).map(|actor| actor.id.clone())
    }

    fn make_bounty_reward(&mut self, kind_id: &str) -> Result<ItemInstance, &'static str> {
        let id = self
            .allocate_item_instance_id()
            .map_err(|_| "reward-item-id-unavailable")?;
        let (activation, charges) =
            initial_item_runtime_state(&self.content, &mut self.rng, kind_id, &[], 1);
        Ok(ItemInstance {
            id,
            kind_id: kind_id.to_owned(),
            quantity: 1,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: initial_item_curse(&self.content, kind_id),
            permanent_destruction_immunities: Default::default(),
            activation,
            charges,
            fuel: initial_item_fuel(&self.content, kind_id),
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Inventory,
        })
    }

    fn grant_bounty_reward(&mut self, reward: ItemInstance) {
        let kind_id = reward.kind_id.clone();
        let definition = self
            .content
            .item(&kind_id)
            .expect("validated bounty reward must remain available");
        if let Some(existing) = self.items.iter_mut().find(|item| {
            item.location == ItemLocation::Inventory
                && item.quantity < definition.max_stack
                && item_instances_stack_compatible(item, &reward)
        }) {
            existing.quantity += 1;
        } else {
            self.items.push(reward);
        }
        self.register_generated_artifact(&kind_id);
    }

    fn turn_in_bounty(
        &mut self,
        bounty: &TownFacilityBountyDefinition,
        item_id: &str,
    ) -> Result<BountyOfficeOutcome, &'static str> {
        let item_index = self
            .items
            .iter()
            .position(|item| {
                item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
            })
            .ok_or("item-unavailable")?;
        let actor_kind_id = self.items[item_index]
            .origin_actor_kind_id
            .clone()
            .ok_or("not-remains")?;
        let item_kind_id = self.items[item_index].kind_id.clone();
        if self
            .content
            .item(&item_kind_id)
            .is_none_or(|item| !item.tags.iter().any(|tag| tag == "corpse"))
        {
            return Err("not-remains");
        }
        let daily_id = self.daily_bounty_actor().map(|actor| actor.id.clone());
        if daily_id.as_deref() == Some(&actor_kind_id) {
            let actor = self
                .content
                .actor(&actor_kind_id)
                .expect("daily bounty actor must remain available");
            let skeleton = self
                .content
                .item(&item_kind_id)
                .is_some_and(|item| item.tags.iter().any(|tag| tag == "skeleton"));
            let multiplier = if skeleton { 30 } else { 50 };
            let gold = actor.level.saturating_add(2).saturating_mul(multiplier);
            self.items.remove(item_index);
            self.gold = self.gold.saturating_add(gold);
            return Ok(BountyOfficeOutcome::DailyTurnIn {
                actor_kind_id,
                gold,
            });
        }
        let wanted = self.wanted_bounty_actor_ids();
        let wanted_index = wanted
            .iter()
            .position(|candidate| candidate == &actor_kind_id)
            .ok_or("target-not-wanted")?;
        if self
            .bounty_state
            .completed_wanted_actor_kind_ids
            .contains(&actor_kind_id)
        {
            return Err("target-already-claimed");
        }
        let reward_kind_id = Self::bounty_reward_item_kind_id(bounty, wanted_index).to_owned();
        let preview_rng = self.rng.clone();
        let serial = self.next_item_instance_serial;
        let preview = self.make_bounty_reward(&reward_kind_id)?;
        self.rng = preview_rng;
        self.next_item_instance_serial = serial;
        let remains = self.items.remove(item_index);
        if self.inventory_quantity_capacity_for(&preview, false) < 1 {
            self.items.insert(item_index, remains);
            return Err("inventory-full");
        }
        let reward = self.make_bounty_reward(&reward_kind_id)?;
        self.grant_bounty_reward(reward);
        self.bounty_state
            .completed_wanted_actor_kind_ids
            .insert(actor_kind_id.clone());
        Ok(BountyOfficeOutcome::WantedTurnIn {
            actor_kind_id,
            item_kind_id: reward_kind_id,
        })
    }

    pub(super) fn use_bounty_office(
        &mut self,
        facility_id: &str,
        action: BountyOfficeActionDto,
        item_id: Option<&str>,
    ) -> Result<BountyOfficeOutcome, &'static str> {
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        let bounty = self
            .content
            .town_facility(facility_id)
            .and_then(|facility| facility.bounty_office.clone())
            .ok_or("service-unavailable")?;
        match action {
            BountyOfficeActionDto::TurnIn => {
                self.turn_in_bounty(&bounty, item_id.ok_or("item-required")?)
            }
            BountyOfficeActionDto::RequestMission => {
                if item_id.is_some() {
                    return Err("unexpected-item");
                }
                if self.bounty_state.mission.is_some() {
                    return Err("mission-already-active");
                }
                let floor = self.bounty_mission_floor().ok_or("mission-unavailable")?;
                let dungeon_id = floor.dungeon_id.clone().ok_or("mission-unavailable")?;
                let actor_kind_id = self
                    .bounty_mission_actor(floor.depth)
                    .ok_or("mission-unavailable")?;
                let actor_level = self
                    .content
                    .actor(&actor_kind_id)
                    .map_or(0, |actor| u16::try_from(actor.level).unwrap_or(u16::MAX));
                let mut total = 1_u16.saturating_add(floor.depth / 25);
                if actor_level.saturating_add(5) < floor.depth {
                    total = total.saturating_add(1);
                }
                let total = u8::try_from(total.clamp(1, 5)).unwrap_or(5);
                self.bounty_state.mission = Some(BountyMissionState {
                    dungeon_id,
                    floor_id: floor.id.clone(),
                    actor_kind_id: actor_kind_id.clone(),
                    total,
                    remaining: total,
                });
                Ok(BountyOfficeOutcome::MissionRequested {
                    actor_kind_id,
                    floor_id: floor.id,
                    total,
                })
            }
            BountyOfficeActionDto::AbandonMission => {
                if item_id.is_some() {
                    return Err("unexpected-item");
                }
                let mission = self
                    .bounty_state
                    .mission
                    .as_ref()
                    .ok_or("mission-unavailable")?;
                if mission.remaining == 0 {
                    return Err("mission-completed");
                }
                self.bounty_state.mission = None;
                Ok(BountyOfficeOutcome::MissionAbandoned)
            }
            BountyOfficeActionDto::ClaimMissionReward => {
                if item_id.is_some() {
                    return Err("unexpected-item");
                }
                let mission = self
                    .bounty_state
                    .mission
                    .as_ref()
                    .filter(|mission| mission.remaining == 0)
                    .cloned()
                    .ok_or("reward-unavailable")?;
                let reward_kind_id = self
                    .bounty_mission_reward_item_kind_id(&bounty, &mission)
                    .to_owned();
                let preview_rng = self.rng.clone();
                let serial = self.next_item_instance_serial;
                let preview = self.make_bounty_reward(&reward_kind_id)?;
                self.rng = preview_rng;
                self.next_item_instance_serial = serial;
                if self.inventory_quantity_capacity_for(&preview, false) < 1 {
                    return Err("inventory-full");
                }
                let reward = self.make_bounty_reward(&reward_kind_id)?;
                self.grant_bounty_reward(reward);
                self.bounty_state.mission = None;
                Ok(BountyOfficeOutcome::MissionRewarded {
                    item_kind_id: reward_kind_id,
                })
            }
        }
    }

    pub(super) fn bounty_target_for_floor(&self, floor_id: &str) -> Option<(String, u8)> {
        let mission = self.bounty_state.mission.as_ref()?;
        (mission.floor_id == floor_id && mission.remaining > 0)
            .then(|| (mission.actor_kind_id.clone(), mission.remaining))
    }

    pub(super) fn apply_bounty_deaths(&mut self) -> Option<String> {
        let mission = self.bounty_state.mission.as_mut()?;
        if mission.floor_id != self.current_floor_id || mission.remaining == 0 {
            return None;
        }
        let deaths = self
            .command_actor_deaths
            .iter()
            .filter(|death| {
                death.credit_player
                    && death.actor_kind_id == mission.actor_kind_id
                    && death.actor_id.contains(BOUNTY_TARGET_ID_MARKER)
            })
            .count();
        mission.remaining = mission
            .remaining
            .saturating_sub(u8::try_from(deaths).unwrap_or(u8::MAX));
        (mission.remaining == 0 && deaths > 0).then(|| mission.actor_kind_id.clone())
    }
}

pub(super) fn bounty_target_instance_id(floor_id: &str, ordinal: u8) -> String {
    format!("{floor_id}{BOUNTY_TARGET_ID_MARKER}{ordinal}")
}

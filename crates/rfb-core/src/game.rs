// SPDX-License-Identifier: MPL-2.0
// Game aggregate and rule orchestration.

use std::{
    collections::{BTreeSet, VecDeque},
    sync::Arc,
};

use crate::resistance::DamageType;
use crate::{
    action::GameAction,
    check::{CheckContext, CheckKind, resolve_check},
    combat::{
        adjacent, apply_melee_armor_reduction, monster_melee_skill, rating_to_armor_class,
        rating_to_combat_value,
    },
    effect::{
        DamageOutcome, DamagePacket, EffectOutcome, EffectSpec, EffectTarget, STATUS_BLEEDING,
        STATUS_HASTE, STATUS_POISON, STATUS_SLOW, advance_status_ticks, apply_effect,
        resolve_damage,
    },
    error::CoreError,
    event::{DomainEvent, project_events},
    rng::{RNG_ALGORITHM, RfbRng},
    save::{
        GENERATED_ITEM_ID_PREFIX, actor_from_entity, actor_from_player, actor_from_spawn,
        actors_to_save, derive_next_item_instance_serial, equipment_item_from_dto,
        equipment_to_save, inventory_item_from_dto, inventory_to_save, item_from_dto,
        items_to_save, player_to_save, position_from_content,
    },
    scheduler::{
        INITIAL_MONSTER_ENERGY_NEED, INITIAL_PLAYER_ENERGY_NEED, STANDARD_ACTION_COST, gain_energy,
        spend_energy,
    },
    state::{Actor, EquipOutcome, ItemInstance, ItemLocation},
    stats::{DerivedStat, DerivedStatsPipeline, StatBounds, StatKind, StatLayer},
};
use rfb_content::{ActorRole, ContentCatalog};
use rfb_protocol::{
    ActorSaveDto, CellDto, CellLightDto, CellVisualDto, ContentVisualDto, DamageDiceDto, EntityDto,
    EquipmentItemDto, EquipmentItemSaveDto, GameCommandEnvelope, GameSnapshot, GameUpdate,
    InventoryItemDto, InventoryItemSaveDto, ItemDto, ItemSaveDto, PROTOCOL_VERSION, PlayerDto,
    PlayerSaveDto, Position, RngSaveDto, SavePayloadV1, StatModifiersDto, TerrainSaveDto,
    VisibilityState,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const BUILT_IN_WORLD_ID: &str = "demo.world.original-v1";
const PREVIOUS_BUILT_IN_CONTENT_HASHES: [&str; 7] = [
    "880610557b208e7c2459ff876c4ace1cb2ef9903986cb7883a04d511ca13c025",
    "0a76daadea3a9683ea8173aa8f65e6195a5582bdf7fdad215cea1a2896dfefcc",
    "cd2c813d224189c925a940e60a915fe3dcf6efa0ccadfc7363d06d428f56525f",
    "36bdba260173b9ba7477e85b886c134affed0369aa4f7a485e59e4408e618ebd",
    "d0537220f093719e623b51bf589dd0a3d8a67ccdc534a1502adcebe094120e9b",
    "e597eb10e3eec454ea78e8ad4e874a8ef41732c6f497083f4fb698d9a1935c69",
    "ee3446edab3354c091bd1edc6e0b5e8d478fd090767fee6796614d9372286a53",
];
const BUILT_IN_CONTENT_HASH: &str =
    "12ba3295dfa8a9884bc7464a78b7dbb9cded01409ff22777db02df85d1aabed7";
const BUILT_IN_CONTENT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rfb-demo-original.rfbcontent"));
const VISIBILITY_RADIUS: i32 = 8;
const AMBIENT_LIGHT: u8 = 28;
const PLAYER_LIGHT_RADIUS: i32 = 6;
const ACTOR_LIGHT_RADIUS: i32 = 5;
const ITEM_LIGHT_RADIUS: i32 = 4;
const PLAYER_LIGHT_COLOR: u32 = 0xffd7a3;
const ACTOR_LIGHT_COLOR: u32 = 0xff8a4c;
const ITEM_LIGHT_COLOR: u32 = 0x8ad9ff;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateHashPayloadV9 {
    schema_version: u16,
    revision: u32,
    turn: u32,
    world_tick: u32,
    last_command_seq: u32,
    terrain: TerrainSaveDto,
    player: PlayerSaveDto,
    entities: Vec<ActorSaveDto>,
    items: Vec<ItemSaveDto>,
    inventory: Vec<InventoryItemSaveDto>,
    equipment: Vec<EquipmentItemSaveDto>,
    next_item_instance_serial: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    explored: Vec<bool>,
    rng: RngSaveDto,
    content_id: String,
    content_hash: String,
    world_id: String,
}

#[derive(Debug, Clone)]
pub struct Game {
    content: Arc<ContentCatalog>,
    world_id: String,
    width: u16,
    height: u16,
    terrain: Vec<String>,
    player: Actor,
    entities: Vec<Actor>,
    items: Vec<ItemInstance>,
    next_item_instance_serial: u64,
    explored: Vec<bool>,
    rng: RfbRng,
    revision: u32,
    turn: u32,
    world_tick: u32,
    last_command_seq: u32,
}

impl Game {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::from_content(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            BUILT_IN_WORLD_ID,
        )
        .expect("built-in world should create a game")
    }

    pub fn from_content(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
    ) -> Result<Self, CoreError> {
        let world = content
            .world(world_id)
            .ok_or_else(|| CoreError::UnknownWorld(world_id.to_owned()))?;
        let width = world.width;
        let height = world.height;
        let mut terrain =
            vec![world.fill_terrain_id.clone(); usize::from(width) * usize::from(height)];
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    terrain[usize::from(y) * usize::from(width) + usize::from(x)] =
                        world.border_terrain_id.clone();
                }
            }
        }
        for terrain_override in &world.terrain_overrides {
            for position in &terrain_override.positions {
                terrain[usize::from(position.y) * usize::from(width) + usize::from(position.x)] =
                    terrain_override.terrain_id.clone();
            }
        }
        let player_definition = content
            .actor(&world.player.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(world.player.kind_id.clone()))?;
        let player = actor_from_spawn(
            &world.player.instance_id,
            &world.player.kind_id,
            world.player.position,
            player_definition.max_hp,
            player_definition.speed,
            INITIAL_PLAYER_ENERGY_NEED,
        );
        let entities = world
            .actors
            .iter()
            .map(|spawn| {
                let definition = content
                    .actor(&spawn.kind_id)
                    .ok_or_else(|| CoreError::UnknownActor(spawn.kind_id.clone()))?;
                Ok(actor_from_spawn(
                    &spawn.instance_id,
                    &spawn.kind_id,
                    spawn.position,
                    definition.max_hp,
                    definition.speed,
                    INITIAL_MONSTER_ENERGY_NEED,
                ))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let items = world
            .items
            .iter()
            .map(|spawn| ItemInstance {
                id: spawn.instance_id.clone(),
                kind_id: spawn.kind_id.clone(),
                quantity: spawn.quantity,
                location: ItemLocation::Ground(position_from_content(spawn.position)),
            })
            .collect::<Vec<_>>();
        let next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items)?;
        let mut game = Self {
            content,
            world_id: world_id.to_owned(),
            width,
            height,
            terrain,
            player,
            entities,
            items,
            next_item_instance_serial,
            explored: vec![false; usize::from(width) * usize::from(height)],
            rng: RfbRng::seeded(seed),
            revision: 0,
            turn: 0,
            world_tick: 0,
            last_command_seq: 0,
        };
        game.reveal_current_visibility();
        game.validate_state()?;
        Ok(game)
    }

    pub fn from_save(payload: SavePayloadV1) -> Result<Self, CoreError> {
        Self::from_save_with_content(
            payload,
            load_built_in_content().expect("built-in content should decode"),
        )
    }

    pub fn from_save_with_content(
        payload: SavePayloadV1,
        content: Arc<ContentCatalog>,
    ) -> Result<Self, CoreError> {
        if payload.schema_version != 1 {
            return Err(CoreError::UnsupportedSaveVersion(payload.schema_version));
        }
        if payload.content_id != content.pack_id()
            || (payload.content_hash != content.content_hash()
                && !(content.pack_id() == "rfb.demo.original-v1"
                    && content.content_hash() == BUILT_IN_CONTENT_HASH
                    && PREVIOUS_BUILT_IN_CONTENT_HASHES.contains(&payload.content_hash.as_str())))
        {
            return Err(CoreError::ContentMismatch);
        }
        if content.world(&payload.world_id).is_none() {
            return Err(CoreError::UnknownWorld(payload.world_id));
        }
        let expected_len = usize::from(payload.terrain.width) * usize::from(payload.terrain.height);
        if expected_len == 0 || payload.terrain.terrain_ids.len() != expected_len {
            return Err(CoreError::InvalidSave("terrain dimensions are invalid"));
        }
        let terrain = payload
            .terrain
            .terrain_ids
            .iter()
            .map(|id| {
                content
                    .terrain(id)
                    .map(|_| id.clone())
                    .ok_or_else(|| CoreError::UnknownTerrain(id.clone()))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let player = actor_from_player(payload.player, &content)?;
        let entities = payload
            .entities
            .into_iter()
            .map(|entity| actor_from_entity(entity, &content))
            .collect::<Result<Vec<_>, CoreError>>()?;
        let mut items = payload
            .items
            .into_iter()
            .map(item_from_dto)
            .collect::<Vec<_>>();
        items.extend(
            payload
                .inventory
                .into_iter()
                .map(|item| inventory_item_from_dto(item, &content))
                .collect::<Result<Vec<_>, CoreError>>()?,
        );
        items.extend(
            payload
                .equipment
                .into_iter()
                .map(|item| equipment_item_from_dto(item, &content))
                .collect::<Result<Vec<_>, CoreError>>()?,
        );
        let derived_next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items)?;
        let next_item_instance_serial = if payload.next_item_instance_serial == 0 {
            derived_next_item_instance_serial
        } else if payload.next_item_instance_serial < derived_next_item_instance_serial {
            return Err(CoreError::InvalidSave(
                "item instance allocator is behind existing IDs",
            ));
        } else {
            payload.next_item_instance_serial
        };
        let mut explored = payload.explored;
        if explored.is_empty() {
            explored = vec![false; expected_len];
        } else if explored.len() != expected_len {
            return Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid",
            ));
        }
        let mut game = Self {
            content,
            world_id: payload.world_id,
            width: payload.terrain.width,
            height: payload.terrain.height,
            terrain,
            player,
            entities,
            items,
            next_item_instance_serial,
            explored,
            rng: RfbRng::from_save(&payload.rng)?,
            revision: payload.revision,
            turn: payload.turn,
            world_tick: payload.world_tick,
            last_command_seq: payload.last_command_seq,
        };
        game.reveal_current_visibility();
        game.validate_state()?;
        Ok(game)
    }

    #[must_use]
    pub fn to_save(&self) -> SavePayloadV1 {
        SavePayloadV1 {
            schema_version: 1,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            terrain: TerrainSaveDto {
                width: self.width,
                height: self.height,
                terrain_ids: self.terrain.clone(),
            },
            player: player_to_save(&self.player),
            entities: actors_to_save(&self.entities),
            items: items_to_save(&self.items),
            inventory: inventory_to_save(&self.items),
            equipment: equipment_to_save(&self.items),
            next_item_instance_serial: self.next_item_instance_serial,
            explored: self.explored.clone(),
            rng: self.rng.to_save(),
            content_id: self.content.pack_id().to_owned(),
            content_hash: self.content.content_hash().to_owned(),
            world_id: self.world_id.clone(),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> GameSnapshot {
        let mut cells = Vec::with_capacity(self.terrain.len());
        for y in 0..self.height {
            for x in 0..self.width {
                cells.push(self.cell_dto(Position {
                    x: i32::from(x),
                    y: i32::from(y),
                }));
            }
        }
        let visual_cells = self.visual_cells();
        GameSnapshot {
            protocol_version: PROTOCOL_VERSION.to_owned(),
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            width: self.width,
            height: self.height,
            cells,
            visual_cells,
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
            inventory: self.inventory_dto(),
            equipment: self.equipment_dto(),
            content_id: self.content.pack_id().to_owned(),
            content_hash: self.content.content_hash().to_owned(),
            content_visuals: self.content_visuals(),
            world_id: self.world_id.clone(),
            state_hash: self.state_hash(),
        }
    }

    pub fn dispatch(&mut self, envelope: GameCommandEnvelope) -> Result<GameUpdate, CoreError> {
        if envelope.expected_revision != self.revision {
            return Err(CoreError::RevisionMismatch {
                expected: self.revision,
                received: envelope.expected_revision,
            });
        }
        let expected_seq = self.last_command_seq.saturating_add(1);
        if envelope.command_seq != expected_seq {
            return Err(CoreError::CommandSequence {
                expected: expected_seq,
                received: envelope.command_seq,
            });
        }
        if self.player_is_dead() {
            return Err(CoreError::PlayerDead);
        }

        let base_revision = self.revision;
        let previous_visuals = self.visual_cells();
        let mut changed = BTreeSet::new();
        let mut events = Vec::new();
        let mut removed_entities = Vec::new();
        let action = GameAction::from(envelope.command);
        let action_cost = action.energy_cost();

        match action {
            GameAction::Drop { item_ids } => {
                if let Some((stacks, quantity)) = self.drop_inventory_items(&item_ids) {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::ItemsDropped { stacks, quantity });
                } else {
                    events.push(DomainEvent::NoItemsDropped);
                }
            }
            GameAction::DropQuantity { item_id, quantity } => {
                if let Some((stacks, dropped_quantity)) =
                    self.drop_inventory_quantity(&item_id, quantity)?
                {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::ItemsDropped {
                        stacks,
                        quantity: dropped_quantity,
                    });
                } else {
                    events.push(DomainEvent::NoItemsDropped);
                }
            }
            GameAction::Equip { item_id } => {
                if let Some(outcome) = self.equip_inventory_item(&item_id) {
                    events.push(DomainEvent::ItemEquipped {
                        target_kind_id: outcome.kind_id,
                        slot_id: outcome.slot_id,
                        replaced_kind_id: outcome.replaced_kind_id,
                    });
                } else {
                    events.push(DomainEvent::ItemEquipUnavailable);
                }
            }
            GameAction::Wait => events.push(DomainEvent::Waited),
            GameAction::PickUp => {
                if let Some((kind_id, quantity)) = self.pick_up_at_player()? {
                    changed.insert(self.player.position);
                    events.push(DomainEvent::ItemPickedUp {
                        target_kind_id: kind_id,
                        quantity,
                    });
                } else {
                    events.push(DomainEvent::NothingToPickUp);
                }
            }
            GameAction::Unequip { slot_id } => {
                if let Some(kind_id) = self.unequip_slot(&slot_id) {
                    events.push(DomainEvent::ItemUnequipped {
                        target_kind_id: kind_id,
                        slot_id,
                    });
                } else {
                    events.push(DomainEvent::ItemUnequipUnavailable { slot_id });
                }
            }
            GameAction::Move { direction } => {
                let (dx, dy) = direction.delta();
                let target = Position {
                    x: self.player.position.x + dx,
                    y: self.player.position.y + dy,
                };
                if !self.is_walkable(target) {
                    events.push(DomainEvent::MoveBlocked);
                } else if let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.position == target)
                {
                    changed.insert(target);
                    self.resolve_player_melee(index, &mut events, &mut removed_entities);
                } else {
                    let old_position = self.player.position;
                    self.player.position = target;
                    changed.insert(old_position);
                    changed.insert(target);
                }
            }
        }

        spend_energy(&mut self.player.energy_need, action_cost);
        self.advance_until_player_ready(&mut events, &mut changed, &mut removed_entities);

        self.last_command_seq = envelope.command_seq;
        self.turn = self.turn.saturating_add(1);
        self.revision = self.revision.saturating_add(1);
        self.reveal_current_visibility();
        let changed_visual_cells = self.changed_visual_cells(&previous_visuals);
        let events = project_events(events);

        Ok(GameUpdate {
            base_revision,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            command_seq: self.last_command_seq,
            events,
            changed_cells: changed
                .into_iter()
                .map(|position| self.cell_dto(position))
                .collect(),
            changed_visual_cells,
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
            inventory: self.inventory_dto(),
            equipment: self.equipment_dto(),
            removed_entities,
            state_hash: self.state_hash(),
        })
    }

    #[must_use]
    pub fn state_hash(&self) -> String {
        let payload = StateHashPayloadV9 {
            schema_version: 9,
            revision: self.revision,
            turn: self.turn,
            world_tick: self.world_tick,
            last_command_seq: self.last_command_seq,
            terrain: TerrainSaveDto {
                width: self.width,
                height: self.height,
                terrain_ids: self.terrain.clone(),
            },
            player: player_to_save(&self.player),
            entities: actors_to_save(&self.entities),
            items: items_to_save(&self.items),
            inventory: inventory_to_save(&self.items),
            equipment: equipment_to_save(&self.items),
            next_item_instance_serial: self.next_item_instance_serial,
            explored: Vec::new(),
            rng: self.rng.to_save(),
            content_id: self.content.pack_id().to_owned(),
            content_hash: self.content.content_hash().to_owned(),
            world_id: self.world_id.clone(),
        };
        let bytes = rmp_serde::to_vec_named(&payload)
            .expect("serializing the internal save state should not fail");
        let digest = Sha256::digest(bytes);
        format!("{digest:x}")
    }

    #[must_use]
    pub const fn rng_draw_counter(&self) -> u64 {
        self.rng.draw_counter
    }

    #[must_use]
    pub const fn rng_algorithm(&self) -> &'static str {
        RNG_ALGORITHM
    }

    #[must_use]
    pub fn content_id(&self) -> &str {
        self.content.pack_id()
    }

    #[must_use]
    pub fn content_hash(&self) -> &str {
        self.content.content_hash()
    }

    #[must_use]
    pub fn world_id(&self) -> &str {
        &self.world_id
    }

    #[must_use]
    pub fn location_key(&self) -> &str {
        &self
            .content
            .world(&self.world_id)
            .expect("game world must remain in its content catalog")
            .name_key
    }

    fn player_dto(&self) -> PlayerDto {
        let stats = self.player_derived_stats();
        let equipment_modifiers = self.equipment_modifiers();
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        PlayerDto {
            id: self.player.id.clone(),
            kind_id: self.player.kind_id.clone(),
            position: self.player.position,
            hp: self.player.hp,
            max_hp: stats.max_hp.value,
            speed: derived_speed(&stats.speed),
            energy_need: self.player.energy_need,
            base_max_hp: self.player.max_hp,
            attack: stats.attack.value,
            base_attack: definition.attack,
            defense: stats.defense.value,
            base_defense: definition.defense,
            melee_skill: stats.melee_skill.value,
            armor_class: stats.armor_class.value,
            melee_damage: DamageDiceDto {
                dice: definition.damage_dice,
                sides: definition.damage_sides,
                damage_type: DamageType::from(definition.damage_type).into(),
            },
            is_dead: self.player_is_dead(),
            equipment_modifiers,
            statuses: self
                .player
                .statuses
                .iter()
                .map(crate::effect::StatusInstance::to_dto)
                .collect(),
            resistances: self.player.resistances.to_dtos(),
        }
    }

    fn entities_dto(&self) -> Vec<EntityDto> {
        let mut entities = self
            .entities
            .iter()
            .map(|entity| {
                let definition = self
                    .content
                    .actor(&entity.kind_id)
                    .expect("entity actor definition must remain available");
                let stats = self.actor_derived_stats(entity, definition, false);
                EntityDto {
                    id: entity.id.clone(),
                    kind_id: entity.kind_id.clone(),
                    position: entity.position,
                    hp: entity.hp,
                    max_hp: entity.max_hp,
                    speed: derived_speed(&stats.speed),
                    energy_need: entity.energy_need,
                    attack: stats.attack.value,
                    defense: stats.defense.value,
                    melee_skill: stats.melee_skill.value,
                    armor_class: stats.armor_class.value,
                    melee_damage: DamageDiceDto {
                        dice: definition.damage_dice,
                        sides: definition.damage_sides,
                        damage_type: DamageType::from(definition.damage_type).into(),
                    },
                    statuses: entity
                        .statuses
                        .iter()
                        .map(crate::effect::StatusInstance::to_dto)
                        .collect(),
                }
            })
            .collect::<Vec<_>>();
        entities.sort_by(|left, right| left.id.cmp(&right.id));
        entities
    }

    fn items_dto(&self) -> Vec<ItemDto> {
        let mut items = self
            .items
            .iter()
            .filter_map(|item| {
                let ItemLocation::Ground(position) = &item.location else {
                    return None;
                };
                Some(ItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    position: *position,
                    quantity: item.quantity,
                })
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.id.cmp(&right.id));
        items
    }

    fn inventory_dto(&self) -> Vec<InventoryItemDto> {
        let mut inventory = self
            .items
            .iter()
            .filter_map(|item| {
                if item.location != ItemLocation::Inventory {
                    return None;
                }
                Some(InventoryItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    quantity: item.quantity,
                    equipment_slot: self
                        .content
                        .item(&item.kind_id)
                        .and_then(|definition| definition.equipment_slot.clone()),
                    modifiers: self.item_modifiers(&item.kind_id),
                })
            })
            .collect::<Vec<_>>();
        inventory.sort_by(|left, right| left.id.cmp(&right.id));
        inventory
    }

    fn equipment_dto(&self) -> Vec<EquipmentItemDto> {
        let mut equipment = self
            .items
            .iter()
            .filter_map(|item| {
                let ItemLocation::Equipped { slot_id } = &item.location else {
                    return None;
                };
                Some(EquipmentItemDto {
                    id: item.id.clone(),
                    kind_id: item.kind_id.clone(),
                    quantity: item.quantity,
                    slot_id: slot_id.clone(),
                    modifiers: self.item_modifiers(&item.kind_id),
                })
            })
            .collect::<Vec<_>>();
        equipment.sort_by(|left, right| left.slot_id.cmp(&right.slot_id));
        equipment
    }

    fn drop_inventory_items(&mut self, item_ids: &[String]) -> Option<(usize, u64)> {
        let selected = item_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
        if selected.is_empty() {
            return None;
        }
        let mut stacks = 0_usize;
        let mut quantity = 0_u64;
        for item in &mut self.items {
            if item.location == ItemLocation::Inventory && selected.contains(item.id.as_str()) {
                item.location = ItemLocation::Ground(self.player.position);
                stacks += 1;
                quantity = quantity.saturating_add(u64::from(item.quantity));
            }
        }
        if stacks == 0 {
            return None;
        }
        Some((stacks, quantity))
    }

    fn drop_inventory_quantity(
        &mut self,
        item_id: &str,
        quantity: u32,
    ) -> Result<Option<(usize, u64)>, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .position(|item| item.id == item_id && item.location == ItemLocation::Inventory)
        else {
            return Ok(None);
        };
        if quantity == 0 || quantity > self.items[index].quantity {
            return Ok(None);
        }
        if quantity == self.items[index].quantity {
            self.items[index].location = ItemLocation::Ground(self.player.position);
        } else {
            let id = self.allocate_item_instance_id()?;
            let kind_id = self.items[index].kind_id.clone();
            self.items[index].quantity -= quantity;
            self.items.push(ItemInstance {
                id,
                kind_id,
                quantity,
                location: ItemLocation::Ground(self.player.position),
            });
        }
        Ok(Some((1, u64::from(quantity))))
    }

    fn equip_inventory_item(&mut self, item_id: &str) -> Option<EquipOutcome> {
        let inventory_index = self
            .items
            .iter()
            .position(|item| item.id == item_id && item.location == ItemLocation::Inventory)?;
        let carried = &self.items[inventory_index];
        let slot_id = self
            .content
            .item(&carried.kind_id)?
            .equipment_slot
            .clone()?;
        if carried.quantity != 1 {
            return None;
        }
        let replaced_kind_id = self
            .items
            .iter()
            .position(|equipped| {
                matches!(
                    &equipped.location,
                    ItemLocation::Equipped { slot_id: equipped_slot } if equipped_slot == &slot_id
                )
            })
            .map(|index| {
                let kind_id = self.items[index].kind_id.clone();
                self.items[index].location = ItemLocation::Inventory;
                kind_id
            });
        let kind_id = self.items[inventory_index].kind_id.clone();
        self.items[inventory_index].location = ItemLocation::Equipped {
            slot_id: slot_id.clone(),
        };
        self.clamp_player_hp_to_effective_max();
        Some(EquipOutcome {
            kind_id,
            slot_id,
            replaced_kind_id,
        })
    }

    fn unequip_slot(&mut self, slot_id: &str) -> Option<String> {
        let index = self.items.iter().position(|item| {
            matches!(
                &item.location,
                ItemLocation::Equipped { slot_id: equipped_slot } if equipped_slot == slot_id
            )
        })?;
        let kind_id = self.items[index].kind_id.clone();
        self.items[index].location = ItemLocation::Inventory;
        self.clamp_player_hp_to_effective_max();
        Some(kind_id)
    }

    fn pick_up_at_player(&mut self) -> Result<Option<(String, u32)>, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.location == ItemLocation::Ground(self.player.position))
            .min_by(|(_, left), (_, right)| left.id.cmp(&right.id))
            .map(|(index, _)| index)
        else {
            return Ok(None);
        };

        let kind_id = self.items[index].kind_id.clone();
        let max_stack = self
            .content
            .item(&kind_id)
            .ok_or_else(|| CoreError::UnknownItem(kind_id.clone()))?
            .max_stack;
        let original_quantity = self.items[index].quantity;
        let mut remaining = original_quantity;
        let mut stack_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, carried)| {
                carried.location == ItemLocation::Inventory
                    && carried.kind_id == kind_id
                    && carried.quantity < max_stack
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        stack_indices.sort_by(|left, right| self.items[*left].id.cmp(&self.items[*right].id));
        for stack_index in stack_indices {
            let stack = &mut self.items[stack_index];
            let transferred = remaining.min(max_stack - stack.quantity);
            stack.quantity += transferred;
            remaining -= transferred;
            if remaining == 0 {
                break;
            }
        }
        if remaining == 0 {
            self.items.remove(index);
        } else {
            self.items[index].quantity = remaining;
            self.items[index].location = ItemLocation::Inventory;
        }
        Ok(Some((kind_id, original_quantity)))
    }

    fn item_modifiers(&self, kind_id: &str) -> StatModifiersDto {
        self.content
            .item(kind_id)
            .map_or_else(StatModifiersDto::default, |definition| StatModifiersDto {
                attack: definition.modifiers.attack,
                defense: definition.modifiers.defense,
                max_hp: definition.modifiers.max_hp,
            })
    }

    fn equipment_modifiers(&self) -> StatModifiersDto {
        self.items
            .iter()
            .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            .fold(StatModifiersDto::default(), |total, item| {
                let item = self.item_modifiers(&item.kind_id);
                StatModifiersDto {
                    attack: total.attack.saturating_add(item.attack),
                    defense: total.defense.saturating_add(item.defense),
                    max_hp: total.max_hp.saturating_add(item.max_hp),
                }
            })
    }

    fn effective_player_max_hp(&self) -> i32 {
        self.player_derived_stats().max_hp.value
    }

    fn player_derived_stats(&self) -> ActorDerivedStats {
        let definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available");
        self.actor_derived_stats(&self.player, definition, true)
    }

    fn actor_derived_stats(
        &self,
        actor: &Actor,
        definition: &rfb_content::ActorDefinition,
        include_equipment: bool,
    ) -> ActorDerivedStats {
        let mut pipeline = DerivedStatsPipeline::new();
        let base_source = definition.id.as_str();
        pipeline.add(StatKind::MaxHp, StatLayer::Base, base_source, actor.max_hp);
        pipeline.add(
            StatKind::Attack,
            StatLayer::Base,
            base_source,
            definition.attack,
        );
        pipeline.add(
            StatKind::Defense,
            StatLayer::Base,
            base_source,
            definition.defense,
        );
        pipeline.add(
            StatKind::Speed,
            StatLayer::Base,
            base_source,
            i32::from(actor.speed),
        );
        pipeline.add(
            StatKind::MeleeSkill,
            StatLayer::Base,
            base_source,
            if definition.role == ActorRole::Monster {
                monster_melee_skill(definition.attack, definition.level)
            } else {
                rating_to_combat_value(definition.attack)
            },
        );
        pipeline.add(
            StatKind::ArmorClass,
            StatLayer::Base,
            base_source,
            rating_to_armor_class(definition.defense),
        );

        if include_equipment {
            for item in self
                .items
                .iter()
                .filter(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
            {
                let modifiers = self.item_modifiers(&item.kind_id);
                add_equipment_stat(&mut pipeline, StatKind::MaxHp, &item.id, modifiers.max_hp);
                add_equipment_stat(&mut pipeline, StatKind::Attack, &item.id, modifiers.attack);
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::Defense,
                    &item.id,
                    modifiers.defense,
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::MeleeSkill,
                    &item.id,
                    rating_to_combat_value(modifiers.attack),
                );
                add_equipment_stat(
                    &mut pipeline,
                    StatKind::ArmorClass,
                    &item.id,
                    rating_to_armor_class(modifiers.defense),
                );
            }
        }

        for status in &actor.statuses {
            let amount = i32::from(status.intensity).saturating_mul(10);
            if status.kind_id == STATUS_HASTE {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount,
                );
            } else if status.kind_id == STATUS_SLOW {
                pipeline.add_with_origin(
                    StatKind::Speed,
                    StatLayer::Status,
                    &status.kind_id,
                    status.source_id.clone(),
                    amount.saturating_neg(),
                );
            }
        }

        ActorDerivedStats {
            max_hp: pipeline.resolve(StatKind::MaxHp, StatBounds::UNBOUNDED),
            attack: pipeline.resolve(StatKind::Attack, StatBounds::NON_NEGATIVE),
            defense: pipeline.resolve(StatKind::Defense, StatBounds::NON_NEGATIVE),
            speed: pipeline.resolve(StatKind::Speed, StatBounds::ACTOR_SPEED),
            melee_skill: pipeline.resolve(StatKind::MeleeSkill, StatBounds::NON_NEGATIVE),
            armor_class: pipeline.resolve(StatKind::ArmorClass, StatBounds::NON_NEGATIVE),
        }
    }

    fn player_is_dead(&self) -> bool {
        self.player.hp < 0
    }

    fn resolve_player_melee(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        removed_entities: &mut Vec<String>,
    ) {
        let definition = self
            .content
            .actor(&self.entities[index].kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let target_kind = self.entities[index].kind_id.clone();
        let attacker = self.player_derived_stats();
        let target = self.actor_derived_stats(&self.entities[index], &definition, false);
        if attacker.melee_skill.value <= 0
            || !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::MeleeHit,
                    actor_id: self.player.id.clone(),
                    target_id: Some(self.entities[index].id.clone()),
                    ability: attacker.melee_skill,
                    difficulty: target.armor_class,
                },
            )
            .succeeded()
        {
            events.push(DomainEvent::PlayerMeleeMissed {
                target_kind_id: target_kind,
            });
            return;
        }

        let player_definition = self
            .content
            .actor(&self.player.kind_id)
            .expect("player actor definition must remain available")
            .clone();
        let rolled_damage = self.roll_damage(
            player_definition.damage_dice,
            player_definition.damage_sides,
        );
        let damage_type = DamageType::from(player_definition.damage_type);
        let resistance = self.entities[index].resistances.level(damage_type);
        let damage = resolve_damage(DamagePacket::new(rolled_damage, damage_type), resistance);
        self.entities[index].hp = self.entities[index].hp.saturating_sub(damage.applied);
        events.push(DomainEvent::PlayerMeleeHit {
            target_kind_id: target_kind,
            damage,
        });
        if self.entities[index].hp <= 0 {
            let removed = self.entities.remove(index);
            removed_entities.push(removed.id);
            events.push(DomainEvent::PlayerSlew {
                target_kind_id: removed.kind_id,
                damage,
            });
        }
    }

    fn advance_until_player_ready(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        loop {
            self.world_tick = self.world_tick.saturating_add(1);
            self.process_status_tick(events, changed, removed_entities);
            if self.player_is_dead() {
                break;
            }
            self.process_monster_energy_pulse(events, changed);
            if self.player_is_dead() {
                break;
            }
            let speed = derived_speed(&self.player_derived_stats().speed);
            gain_energy(&mut self.player.energy_need, speed);
            if self.player.energy_need <= 0 {
                break;
            }
        }
    }

    fn process_status_tick(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let player_tick = process_actor_status_tick(&mut self.player, false);
        for damage in player_tick.damage {
            events.push(DomainEvent::PlayerStatusDamaged {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
        }
        for status_kind_id in player_tick.expired {
            events.push(DomainEvent::PlayerStatusExpired { status_kind_id });
        }
        if let Some(damage) = player_tick.fatal_damage {
            events.push(DomainEvent::PlayerDiedFromStatus {
                status_kind_id: damage.status_kind_id,
                damage: damage.outcome,
            });
            return;
        }

        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();
        for entity_id in entity_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let tick = process_actor_status_tick(&mut self.entities[index], true);
            for damage in tick.damage {
                events.push(DomainEvent::EntityStatusDamaged {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id: damage.status_kind_id,
                    damage: damage.outcome,
                });
            }
            for status_kind_id in tick.expired {
                events.push(DomainEvent::EntityStatusExpired {
                    target_kind_id: target_kind_id.clone(),
                    status_kind_id,
                });
            }
            if let Some(damage) = tick.fatal_damage {
                let removed = self.entities.remove(index);
                changed.insert(removed.position);
                removed_entities.push(removed.id);
                events.push(DomainEvent::EntityDiedFromStatus {
                    target_kind_id,
                    status_kind_id: damage.status_kind_id,
                    damage: damage.outcome,
                });
            }
        }
    }

    fn process_monster_energy_pulse(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let mut entity_ids = self
            .entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        entity_ids.sort();

        for entity_id in entity_ids {
            if self.player_is_dead() {
                break;
            }
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("monster actor definition must remain available");
            let speed = derived_speed(
                &self
                    .actor_derived_stats(&self.entities[index], definition, false)
                    .speed,
            );
            gain_energy(&mut self.entities[index].energy_need, speed);
            if self.entities[index].energy_need > 0 {
                continue;
            }
            spend_energy(&mut self.entities[index].energy_need, STANDARD_ACTION_COST);
            self.resolve_monster_action(index, events, changed);
        }
    }

    fn resolve_monster_action(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if adjacent(self.entities[index].position, self.player.position) {
            self.resolve_monster_melee(index, events);
            return;
        }
        let Some(next_position) = self.next_monster_step(index) else {
            return;
        };
        let old_position = self.entities[index].position;
        self.entities[index].position = next_position;
        changed.insert(old_position);
        changed.insert(next_position);
    }

    fn resolve_monster_melee(&mut self, index: usize, events: &mut Vec<DomainEvent>) {
        let kind_id = self.entities[index].kind_id.clone();
        let definition = self
            .content
            .actor(&kind_id)
            .expect("monster actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[index], &definition, false);
        let target = self.player_derived_stats();
        let armor_class = target.armor_class.value;
        if !resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::MeleeHit,
                actor_id: self.entities[index].id.clone(),
                target_id: Some(self.player.id.clone()),
                ability: attacker.melee_skill,
                difficulty: target.armor_class,
            },
        )
        .succeeded()
        {
            events.push(DomainEvent::MonsterMeleeMissed {
                source_kind_id: kind_id,
            });
            return;
        }

        let raw_damage = self.roll_damage(definition.damage_dice, definition.damage_sides);
        let damage_type = DamageType::from(definition.damage_type);
        let prepared_damage = if damage_type == DamageType::Physical {
            apply_melee_armor_reduction(raw_damage, armor_class)
        } else {
            raw_damage
        };
        let resistance = self.player.resistances.level(damage_type);
        let damage = resolve_damage(
            DamagePacket::after_armor(raw_damage, prepared_damage, damage_type),
            resistance,
        );
        self.player.hp = self.player.hp.saturating_sub(damage.applied);
        events.push(DomainEvent::MonsterMeleeHit {
            source_kind_id: kind_id.clone(),
            damage,
        });
        if self.player_is_dead() {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: kind_id,
                damage,
            });
        }
    }

    fn next_monster_step(&self, index: usize) -> Option<Position> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];

        let start = self.entities[index].position;
        let occupied = self
            .entities
            .iter()
            .enumerate()
            .filter(|(entity_index, _)| *entity_index != index)
            .map(|(_, entity)| entity.position)
            .collect::<BTreeSet<_>>();
        let mut visited = BTreeSet::from([start]);
        let mut queue = VecDeque::new();

        let mut initial = DELTAS
            .iter()
            .enumerate()
            .map(|(order, (dx, dy))| {
                let position = Position {
                    x: start.x + dx,
                    y: start.y + dy,
                };
                (
                    squared_distance(position, self.player.position),
                    order,
                    position,
                )
            })
            .collect::<Vec<_>>();
        initial.sort();
        for (_, _, position) in initial {
            if position == self.player.position
                || occupied.contains(&position)
                || !self.is_walkable(position)
                || !visited.insert(position)
            {
                continue;
            }
            if adjacent(position, self.player.position) {
                return Some(position);
            }
            queue.push_back((position, position));
        }

        while let Some((position, first_step)) = queue.pop_front() {
            let mut neighbors = DELTAS
                .iter()
                .enumerate()
                .map(|(order, (dx, dy))| {
                    let next = Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    };
                    (squared_distance(next, self.player.position), order, next)
                })
                .collect::<Vec<_>>();
            neighbors.sort();
            for (_, _, next) in neighbors {
                if next == self.player.position
                    || occupied.contains(&next)
                    || !self.is_walkable(next)
                    || !visited.insert(next)
                {
                    continue;
                }
                if adjacent(next, self.player.position) {
                    return Some(first_step);
                }
                queue.push_back((next, first_step));
            }
        }
        None
    }

    fn roll_damage(&mut self, dice: u16, sides: u16) -> i32 {
        (0..dice).fold(0_i32, |total, _| {
            let roll = i32::try_from(self.rng.bounded(u64::from(sides)))
                .unwrap_or(i32::MAX)
                .saturating_add(1);
            total.saturating_add(roll)
        })
    }

    fn clamp_player_hp_to_effective_max(&mut self) {
        self.player.hp = self.player.hp.min(self.effective_player_max_hp());
    }

    fn allocate_item_instance_id(&mut self) -> Result<String, CoreError> {
        loop {
            let serial = self.next_item_instance_serial;
            let next = serial.checked_add(1).ok_or(CoreError::ItemIdExhausted)?;
            let candidate = format!("{GENERATED_ITEM_ID_PREFIX}{serial}");
            self.next_item_instance_serial = next;
            if !self.instance_id_exists(&candidate) {
                return Ok(candidate);
            }
        }
    }

    fn instance_id_exists(&self, candidate: &str) -> bool {
        self.player.id == candidate
            || self.entities.iter().any(|entity| entity.id == candidate)
            || self.items.iter().any(|item| item.id == candidate)
    }

    fn content_visuals(&self) -> Vec<ContentVisualDto> {
        self.content
            .visual_glyphs()
            .into_iter()
            .map(|(id, glyph)| ContentVisualDto { id, glyph })
            .collect()
    }

    fn cell_dto(&self, position: Position) -> CellDto {
        let actor_id = if self.player.position == position {
            Some(self.player.id.clone())
        } else {
            self.entities
                .iter()
                .find(|entity| entity.position == position)
                .map(|entity| entity.id.clone())
        };
        CellDto {
            position,
            terrain_id: self.terrain_at(position).to_owned(),
            item_id: self
                .items
                .iter()
                .find(|item| item.location == ItemLocation::Ground(position))
                .map(|item| item.id.clone()),
            actor_id,
        }
    }

    fn visual_cells(&self) -> Vec<CellVisualDto> {
        let mut visuals = Vec::with_capacity(self.terrain.len());
        for y in 0..self.height {
            for x in 0..self.width {
                visuals.push(self.cell_visual(Position {
                    x: i32::from(x),
                    y: i32::from(y),
                }));
            }
        }
        visuals
    }

    fn changed_visual_cells(&self, previous: &[CellVisualDto]) -> Vec<CellVisualDto> {
        self.visual_cells()
            .into_iter()
            .zip(previous.iter())
            .filter_map(|(current, before)| (current != *before).then_some(current))
            .collect()
    }

    fn cell_visual(&self, position: Position) -> CellVisualDto {
        let index = self.index(position).expect("validated visual position");
        CellVisualDto {
            position,
            visibility: if self.is_visible(position) {
                VisibilityState::Visible
            } else if self.explored[index] {
                VisibilityState::Remembered
            } else {
                VisibilityState::Hidden
            },
            light: self.light_at(position),
        }
    }

    fn reveal_current_visibility(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if self.is_visible(position) {
                    let index = self.index(position).expect("visibility position is valid");
                    self.explored[index] = true;
                }
            }
        }
    }

    fn is_visible(&self, position: Position) -> bool {
        if squared_distance(self.player.position, position) > VISIBILITY_RADIUS * VISIBILITY_RADIUS
        {
            return false;
        }
        has_line_of_sight(self, self.player.position, position)
    }

    fn light_at(&self, position: Position) -> CellLightDto {
        let mut strongest = (0_u8, PLAYER_LIGHT_COLOR);
        let player_boost =
            source_intensity(self.player.position, position, PLAYER_LIGHT_RADIUS, 72);
        if player_boost > strongest.0 {
            strongest = (player_boost, PLAYER_LIGHT_COLOR);
        }

        for entity in &self.entities {
            let Some(definition) = self.content.actor(&entity.kind_id) else {
                continue;
            };
            if !definition.tags.iter().any(|tag| tag == "light-source") {
                continue;
            }
            let boost = source_intensity(entity.position, position, ACTOR_LIGHT_RADIUS, 64);
            if boost > strongest.0 {
                strongest = (boost, ACTOR_LIGHT_COLOR);
            }
        }

        for item in &self.items {
            let ItemLocation::Ground(item_position) = &item.location else {
                continue;
            };
            let Some(definition) = self.content.item(&item.kind_id) else {
                continue;
            };
            if !definition.tags.iter().any(|tag| tag == "light-source") {
                continue;
            }
            let boost = source_intensity(*item_position, position, ITEM_LIGHT_RADIUS, 52);
            if boost > strongest.0 {
                strongest = (boost, ITEM_LIGHT_COLOR);
            }
        }

        CellLightDto {
            color: strongest.1,
            intensity: AMBIENT_LIGHT.saturating_add(strongest.0),
        }
    }

    fn terrain_at(&self, position: Position) -> &str {
        &self.terrain[self.index(position).expect("validated map position")]
    }

    fn index(&self, position: Position) -> Option<usize> {
        if position.x < 0
            || position.y < 0
            || position.x >= i32::from(self.width)
            || position.y >= i32::from(self.height)
        {
            return None;
        }
        Some(position.y as usize * usize::from(self.width) + position.x as usize)
    }

    fn is_walkable(&self, position: Position) -> bool {
        self.index(position)
            .and_then(|index| self.content.terrain(&self.terrain[index]))
            .is_some_and(|terrain| terrain.walkable)
    }

    fn validate_state(&self) -> Result<(), CoreError> {
        if self.explored.len() != self.terrain.len() {
            return Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid",
            ));
        }
        for terrain_id in &self.terrain {
            if self.content.terrain(terrain_id).is_none() {
                return Err(CoreError::UnknownTerrain(terrain_id.clone()));
            }
        }
        self.validate_actor(&self.player, ActorRole::Player)?;
        if !self.is_walkable(self.player.position) {
            return Err(CoreError::InvalidSave("player position is invalid"));
        }
        let mut instance_ids = BTreeSet::new();
        instance_ids.insert(self.player.id.clone());
        let mut positions = BTreeSet::new();
        positions.insert(self.player.position);
        for entity in &self.entities {
            self.validate_actor(entity, ActorRole::Monster)?;
            if !instance_ids.insert(entity.id.clone())
                || !self.is_walkable(entity.position)
                || !positions.insert(entity.position)
            {
                return Err(CoreError::InvalidSave("entity position is invalid"));
            }
        }
        let mut equipment_slots = BTreeSet::new();
        for item in &self.items {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            let common_valid = instance_ids.insert(item.id.clone()) && item.quantity != 0;
            match &item.location {
                ItemLocation::Ground(position) => {
                    if !common_valid
                        || !self.is_walkable(*position)
                        || item.quantity > definition.max_stack
                    {
                        return Err(CoreError::InvalidSave("item state is invalid"));
                    }
                }
                ItemLocation::Inventory => {
                    if !common_valid || item.quantity > definition.max_stack {
                        return Err(CoreError::InvalidSave("inventory item state is invalid"));
                    }
                }
                ItemLocation::Equipped { slot_id } => {
                    if !common_valid
                        || item.quantity != 1
                        || definition.equipment_slot.as_deref() != Some(slot_id.as_str())
                        || !equipment_slots.insert(slot_id.clone())
                    {
                        return Err(CoreError::InvalidSave("equipment item state is invalid"));
                    }
                }
            }
        }
        if self.next_item_instance_serial == 0
            || self.next_item_instance_serial
                < derive_next_item_instance_serial(&self.player, &self.entities, &self.items)?
        {
            return Err(CoreError::InvalidSave(
                "item instance allocator is behind existing IDs",
            ));
        }
        Ok(())
    }

    fn validate_actor(&self, actor: &Actor, expected_role: ActorRole) -> Result<(), CoreError> {
        let definition = self
            .content
            .actor(&actor.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(actor.kind_id.clone()))?;
        let effective_max_hp = if expected_role == ActorRole::Player {
            self.effective_player_max_hp()
        } else {
            actor.max_hp
        };
        let statuses_are_valid = actor.statuses.iter().all(|status| {
            status.intensity > 0
                && status.remaining_ticks > 0
                && !status.kind_id.is_empty()
                && status.kind_id.len() <= 128
        }) && actor
            .statuses
            .windows(2)
            .all(|window| window[0].kind_id < window[1].kind_id);
        if definition.role != expected_role
            || actor.max_hp != definition.max_hp
            || actor.speed != definition.speed
            || actor.speed > 199
            || !statuses_are_valid
            || (expected_role == ActorRole::Monster && actor.hp <= 0)
            || (expected_role == ActorRole::Player && actor.hp < -1_000_000)
            || (expected_role == ActorRole::Monster
                && !(1..=STANDARD_ACTION_COST).contains(&actor.energy_need))
            || (expected_role == ActorRole::Player && actor.hp >= 0 && actor.energy_need > 0)
            || actor.energy_need < -STANDARD_ACTION_COST
            || actor.hp > effective_max_hp
        {
            return Err(CoreError::InvalidSave("actor state is invalid"));
        }
        Ok(())
    }
}

struct ActorStatusTick {
    damage: Vec<StatusDamageTick>,
    expired: Vec<String>,
    fatal_damage: Option<StatusDamageTick>,
}

#[derive(Clone)]
struct StatusDamageTick {
    status_kind_id: String,
    outcome: DamageOutcome,
}

struct ActorDerivedStats {
    max_hp: DerivedStat,
    attack: DerivedStat,
    defense: DerivedStat,
    speed: DerivedStat,
    melee_skill: DerivedStat,
    armor_class: DerivedStat,
}

fn add_equipment_stat(
    pipeline: &mut DerivedStatsPipeline,
    kind: StatKind,
    source_id: &str,
    amount: i32,
) {
    if amount != 0 {
        pipeline.add(kind, StatLayer::Equipment, source_id, amount);
    }
}

fn derived_speed(speed: &DerivedStat) -> u16 {
    u16::try_from(speed.value).expect("derived actor speed must fit u16")
}

fn process_actor_status_tick(actor: &mut Actor, lethal_at_zero: bool) -> ActorStatusTick {
    let periodic = actor
        .statuses
        .iter()
        .filter_map(|status| {
            let damage_type = match status.kind_id.as_str() {
                STATUS_BLEEDING => DamageType::Physical,
                STATUS_POISON => DamageType::Poison,
                _ => return None,
            };
            Some((
                status.kind_id.clone(),
                i32::from(status.intensity),
                damage_type,
            ))
        })
        .collect::<Vec<_>>();
    let mut damage = Vec::new();
    let mut fatal_damage = None;
    for (status_kind_id, amount, damage_type) in periodic {
        let mut target = EffectTarget {
            hp: &mut actor.hp,
            max_hp: actor.max_hp,
            resistances: &actor.resistances,
            statuses: &mut actor.statuses,
        };
        let EffectOutcome::Damage(outcome) = apply_effect(
            &mut target,
            EffectSpec::Damage(DamagePacket::new(amount, damage_type)),
        ) else {
            unreachable!("damage effects must produce damage outcomes");
        };
        let damage_tick = StatusDamageTick {
            status_kind_id: status_kind_id.clone(),
            outcome,
        };
        damage.push(damage_tick.clone());
        if actor.hp < 0 || (lethal_at_zero && actor.hp == 0) {
            fatal_damage = Some(damage_tick);
            break;
        }
    }
    let expired = advance_status_ticks(&mut actor.statuses, 1);
    ActorStatusTick {
        damage,
        expired,
        fatal_damage,
    }
}

fn squared_distance(left: Position, right: Position) -> i32 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    dx * dx + dy * dy
}

fn source_intensity(source: Position, target: Position, radius: i32, maximum: u8) -> u8 {
    let distance = squared_distance(source, target);
    let radius_squared = radius * radius;
    if distance > radius_squared {
        return 0;
    }
    let remaining = radius_squared - distance;
    u8::try_from(
        (u32::from(maximum) * u32::try_from(remaining).unwrap_or(0))
            / u32::try_from(radius_squared).unwrap_or(1),
    )
    .unwrap_or(maximum)
}

fn has_line_of_sight(game: &Game, from: Position, to: Position) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;

    loop {
        if x == to.x && y == to.y {
            return true;
        }
        if !(x == from.x && y == from.y)
            && game
                .index(Position { x, y })
                .and_then(|index| game.content.terrain(&game.terrain[index]))
                .is_some_and(|terrain| terrain.blocks_sight)
        {
            return false;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
    }
}

pub fn load_built_in_content() -> Result<Arc<ContentCatalog>, CoreError> {
    Ok(Arc::new(ContentCatalog::from_bytes(
        BUILT_IN_CONTENT_BYTES,
    )?))
}

#[cfg(test)]
mod tests {
    use crate::effect::StatusInstance;
    use rfb_protocol::{
        DamageTypeDto, Direction, GameCommand, GameCommandEnvelope, ResistanceLevelDto,
        ResistanceSaveDto, StatusSaveDto, VisibilityState,
    };

    use super::*;

    fn command(seq: u32, revision: u32, command: GameCommand) -> GameCommandEnvelope {
        GameCommandEnvelope {
            command_seq: seq,
            expected_revision: revision,
            command,
        }
    }

    fn visual_at(snapshot: &GameSnapshot, position: Position) -> CellVisualDto {
        *snapshot
            .visual_cells
            .iter()
            .find(|visual| visual.position == position)
            .expect("snapshot should contain every visual cell")
    }

    #[test]
    fn built_in_game_is_created_from_the_compiled_content_pack() {
        let snapshot = Game::new(42).snapshot();
        let shard = snapshot
            .items
            .iter()
            .find(|item| item.id == "demo.item.luminous-shard.1")
            .expect("compiled world should spawn its item");

        assert_eq!(snapshot.content_id, "rfb.demo.original-v1");
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.world_id, BUILT_IN_WORLD_ID);
        assert_eq!(
            snapshot.player.melee_damage.damage_type,
            DamageTypeDto::Physical
        );
        assert_eq!(
            snapshot.entities[0].melee_damage.damage_type,
            DamageTypeDto::Fire
        );
        assert_eq!(snapshot.player.id, "demo.actor.player.1");
        assert_eq!(snapshot.player.kind_id, "demo.actor.explorer");
        assert_eq!(snapshot.player.base_attack, 2);
        assert_eq!(snapshot.player.attack, 2);
        assert_eq!(snapshot.player.base_defense, 1);
        assert_eq!(snapshot.player.defense, 1);
        assert!(snapshot.inventory.is_empty());
        assert!(snapshot.equipment.is_empty());
        assert_eq!(snapshot.items.len(), 2);
        assert_eq!(snapshot.entities[0].position, Position { x: 8, y: 5 });
        assert_eq!(snapshot.entities[0].attack, 1);
        assert_eq!(snapshot.entities[0].defense, 1);
        assert_eq!(shard.position, Position { x: 4, y: 3 });
        assert_eq!(
            snapshot
                .cells
                .iter()
                .find(|cell| cell.position == shard.position)
                .and_then(|cell| cell.item_id.as_deref()),
            Some("demo.item.luminous-shard.1")
        );
        assert!(
            snapshot
                .content_visuals
                .iter()
                .any(|visual| visual.id == "demo.item.luminous-shard" && visual.glyph == "!")
        );
        assert_eq!(snapshot.visual_cells.len(), snapshot.cells.len());
        assert_eq!(
            visual_at(&snapshot, snapshot.player.position).visibility,
            VisibilityState::Visible
        );
        assert_eq!(
            visual_at(&snapshot, Position { x: 19, y: 19 }).visibility,
            VisibilityState::Hidden
        );
        assert_eq!(
            visual_at(&snapshot, Position { x: 8, y: 5 }).light.color,
            ACTOR_LIGHT_COLOR
        );
    }

    #[test]
    fn movement_produces_fov_deltas_and_remembers_explored_cells() {
        let mut game = Game::new(42);
        game.entities.clear();
        let first = game
            .dispatch(command(
                1,
                0,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("movement should execute");
        assert!(!first.changed_visual_cells.is_empty());
        let snapshot = game.snapshot();
        assert_eq!(
            visual_at(&snapshot, Position { x: 11, y: 3 }).visibility,
            VisibilityState::Visible
        );
        assert_eq!(
            visual_at(&snapshot, Position { x: 12, y: 3 }).visibility,
            VisibilityState::Hidden
        );

        for seq in 2..=7 {
            game.dispatch(command(
                seq,
                seq - 1,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("eastward exploration should execute");
        }
        assert_eq!(
            visual_at(&game.snapshot(), Position { x: 1, y: 3 }).visibility,
            VisibilityState::Remembered
        );
    }

    #[test]
    fn exploration_memory_does_not_change_authoritative_state_hash() {
        let mut game = Game::new(42);
        let before = game.state_hash();
        game.explored.fill(true);
        assert_eq!(game.state_hash(), before);
    }

    #[test]
    fn malformed_exploration_memory_is_rejected() {
        let mut payload = Game::new(42).to_save();
        payload.explored.pop();
        assert!(matches!(
            Game::from_save(payload),
            Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid"
            ))
        ));
    }

    #[test]
    fn haste_and_slow_modify_scheduler_speed_without_changing_base_speed() {
        let mut haste_payload = Game::new(42).to_save();
        haste_payload.player.statuses = vec![StatusSaveDto {
            kind_id: STATUS_HASTE.to_owned(),
            intensity: 1,
            remaining_ticks: 20,
            source_id: None,
        }];
        let mut haste = Game::from_save(haste_payload).expect("haste setup should load");
        assert_eq!(haste.snapshot().player.speed, 120);
        let haste_update = haste
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("hasted wait should execute");
        assert_eq!(haste_update.world_tick, 5);
        assert_eq!(haste_update.player.speed, 120);
        assert_eq!(haste.to_save().player.base_speed, 110);
        assert_eq!(haste_update.player.statuses[0].remaining_ticks, 15);

        let mut slow_payload = Game::new(42).to_save();
        slow_payload.player.statuses = vec![StatusSaveDto {
            kind_id: STATUS_SLOW.to_owned(),
            intensity: 1,
            remaining_ticks: 40,
            source_id: None,
        }];
        let mut slow = Game::from_save(slow_payload).expect("slow setup should load");
        let slow_update = slow
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("slowed wait should execute");
        assert_eq!(slow_update.world_tick, 20);
        assert_eq!(slow_update.player.speed, 100);
        assert_eq!(slow_update.player.statuses[0].remaining_ticks, 20);
    }

    #[test]
    fn poison_uses_resistance_then_expires_and_round_trips() {
        let mut payload = Game::new(42).to_save();
        payload.player.statuses = vec![StatusSaveDto {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 2,
            remaining_ticks: 3,
            source_id: Some("demo.actor.ember-mote.1".to_owned()),
        }];
        payload.player.resistances = vec![ResistanceSaveDto {
            damage_type: DamageTypeDto::Poison,
            level: ResistanceLevelDto::Resistant,
        }];
        let mut game = Game::from_save(payload).expect("poison setup should load");
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("poisoned wait should execute");

        assert_eq!(update.player.hp, 7);
        assert!(update.player.statuses.is_empty());
        assert_eq!(update.player.resistances.len(), 1);
        assert_eq!(
            update
                .events
                .iter()
                .filter(|event| event.message_key == "status-player-damage")
                .count(),
            3
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "status-player-expired")
        );
        let restored = Game::from_save(game.to_save()).expect("status save should restore");
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn bleeding_ticks_as_physical_damage_in_stable_status_order() {
        let mut payload = Game::new(42).to_save();
        payload.player.statuses = vec![
            StatusSaveDto {
                kind_id: STATUS_POISON.to_owned(),
                intensity: 1,
                remaining_ticks: 1,
                source_id: None,
            },
            StatusSaveDto {
                kind_id: STATUS_BLEEDING.to_owned(),
                intensity: 2,
                remaining_ticks: 2,
                source_id: None,
            },
        ];
        let mut game = Game::from_save(payload).expect("bleeding setup should load");
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("bleeding wait should execute");

        assert_eq!(update.player.hp, 5);
        assert!(update.player.statuses.is_empty());
        let damage_statuses = update
            .events
            .iter()
            .filter(|event| event.message_key == "status-player-damage")
            .map(|event| event.args["status"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            damage_statuses,
            [STATUS_BLEEDING, STATUS_POISON, STATUS_BLEEDING]
        );
    }

    #[test]
    fn content_driven_fire_melee_uses_the_player_resistance_profile() {
        let (seed, normal_damage) = (0_u64..1_000)
            .find_map(|seed| {
                let mut game = Game::new(42);
                game.rng = RfbRng::seeded(seed);
                let mut events = Vec::new();
                game.resolve_monster_melee(0, &mut events);
                events.into_iter().find_map(|event| match event {
                    DomainEvent::MonsterMeleeHit { damage, .. } if damage.applied >= 2 => {
                        Some((seed, damage.applied))
                    }
                    _ => None,
                })
            })
            .expect("a deterministic seed should produce a fire hit of at least two damage");

        let mut resistant = Game::new(42);
        resistant.player.resistances.set(
            DamageType::Fire,
            crate::resistance::ResistanceLevel::Resistant,
        );
        resistant.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        resistant.resolve_monster_melee(0, &mut events);
        let resisted_damage = events
            .into_iter()
            .find_map(|event| match event {
                DomainEvent::MonsterMeleeHit { damage, .. } => Some(damage.applied),
                _ => None,
            })
            .expect("the same seed should preserve the hit result");

        assert_eq!(resisted_damage, normal_damage - normal_damage / 2);
        assert_eq!(resistant.player.hp, 10 - resisted_damage);
    }

    #[test]
    fn lethal_monster_status_removes_the_entity_before_energy_actions() {
        let mut payload = Game::new(42).to_save();
        payload.entities[0].statuses = vec![StatusSaveDto {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 3,
            remaining_ticks: 1,
            source_id: Some("demo.player.1".to_owned()),
        }];
        let mut game = Game::from_save(payload).expect("monster poison setup should load");
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("wait should process monster poison");

        assert!(update.entities.is_empty());
        assert_eq!(update.removed_entities, ["demo.monster.ember-mote.1"]);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "status-entity-death")
        );
    }

    #[test]
    fn previous_built_in_content_hash_migrates_without_spawning_new_items() {
        for previous_hash in PREVIOUS_BUILT_IN_CONTENT_HASHES {
            let mut payload = Game::new(42).to_save();
            payload.content_hash = previous_hash.to_owned();
            payload
                .items
                .retain(|item| item.kind_id != "demo.item.echo-charm");

            let restored = Game::from_save(payload).expect("known previous content should migrate");
            let snapshot = restored.snapshot();
            assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
            assert_eq!(snapshot.items.len(), 1);
            assert!(
                snapshot
                    .items
                    .iter()
                    .all(|item| item.kind_id != "demo.item.echo-charm")
            );
        }
    }

    #[test]
    fn previous_equipment_content_migrates_to_derived_modifiers() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");
        let mut payload = game.to_save();
        payload.content_hash = PREVIOUS_BUILT_IN_CONTENT_HASHES[1].to_owned();
        payload.player.base_max_hp = 0;
        payload.next_item_instance_serial = 0;

        let restored = Game::from_save(payload).expect("known 1.1 content should migrate");
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.player.base_max_hp, 10);
        assert_eq!(snapshot.player.max_hp, 14);
        assert_eq!(snapshot.player.attack, 3);
        assert_eq!(snapshot.player.defense, 2);
        assert_eq!(snapshot.player.equipment_modifiers.attack, 1);
        assert_eq!(snapshot.player.equipment_modifiers.defense, 1);
        assert_eq!(snapshot.player.equipment_modifiers.max_hp, 4);
        assert_eq!(restored.next_item_instance_serial, 1);
    }

    #[test]
    fn previous_combat_content_migrates_to_current_actor_stats() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");
        let mut payload = game.to_save();
        payload.content_hash = PREVIOUS_BUILT_IN_CONTENT_HASHES[2].to_owned();

        let restored = Game::from_save(payload).expect("known 1.2 content should migrate");
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.player.base_attack, 2);
        assert_eq!(snapshot.player.attack, 3);
        assert_eq!(snapshot.player.base_defense, 1);
        assert_eq!(snapshot.player.defense, 2);
        assert_eq!(snapshot.entities[0].attack, 1);
        assert_eq!(snapshot.entities[0].defense, 1);
    }

    #[test]
    fn fixed_seed_and_commands_are_deterministic() {
        let mut left = Game::new(42);
        let mut right = Game::new(42);
        let commands = [
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::South,
            },
            GameCommand::Wait,
        ];

        for (index, game_command) in commands.into_iter().enumerate() {
            let seq = index as u32 + 1;
            let revision = index as u32;
            left.dispatch(command(seq, revision, game_command.clone()))
                .expect("left command should execute");
            right
                .dispatch(command(seq, revision, game_command))
                .expect("right command should execute");
        }

        assert_eq!(left.state_hash(), right.state_hash());
    }

    #[test]
    fn normal_speed_monster_tracks_once_per_player_action() {
        let mut game = Game::new(42);
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("wait should advance the scheduler");

        assert_eq!(update.world_tick, 10);
        assert_eq!(update.player.energy_need, 0);
        assert_eq!(update.entities[0].position, Position { x: 7, y: 4 });
        assert_eq!(update.entities[0].energy_need, STANDARD_ACTION_COST);
        assert_eq!(update.changed_cells.len(), 2);
    }

    #[test]
    fn fast_and_slow_monsters_use_the_same_energy_scheduler() {
        let mut fast = Game::new(42);
        fast.entities[0].speed = 120;
        let fast_update = fast
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("fast scheduler case should execute");
        assert_eq!(fast_update.world_tick, 10);
        assert_eq!(fast_update.entities[0].position, Position { x: 6, y: 3 });
        assert_eq!(fast_update.entities[0].energy_need, STANDARD_ACTION_COST);

        let mut slow = Game::new(42);
        slow.entities[0].speed = 100;
        let first = slow
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("first slow scheduler case should execute");
        assert_eq!(first.entities[0].position, Position { x: 8, y: 5 });
        assert_eq!(first.entities[0].energy_need, 50);
        let second = slow
            .dispatch(command(2, 1, GameCommand::Wait))
            .expect("second slow scheduler case should execute");
        assert_eq!(second.entities[0].position, Position { x: 7, y: 4 });
        assert_eq!(second.entities[0].energy_need, STANDARD_ACTION_COST);
    }

    #[test]
    fn multiple_monsters_use_stable_id_order_when_paths_compete() {
        let mut left = Game::new(42);
        let mut second = left.entities[0].clone();
        second.id = "demo.monster.ember-mote.0".to_owned();
        second.position = Position { x: 8, y: 6 };
        left.entities.push(second);

        let mut right = left.clone();
        right.entities.reverse();

        let left_update = left
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("left scheduler should execute");
        let right_update = right
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("right scheduler should execute");

        assert_eq!(left_update.entities, right_update.entities);
        assert_eq!(left_update.changed_cells, right_update.changed_cells);
        assert_eq!(left_update.state_hash, right_update.state_hash);
        assert_ne!(
            left_update.entities[0].position,
            left_update.entities[1].position
        );
    }

    #[test]
    fn player_death_stops_the_remaining_monster_queue_immediately() {
        let mut game = Game::new(0);
        game.entities[0].id = "demo.monster.ember-mote.0".to_owned();
        game.entities[0].position = Position { x: 4, y: 3 };
        let mut second = game.entities[0].clone();
        second.id = "demo.monster.ember-mote.1".to_owned();
        second.position = Position { x: 4, y: 4 };
        game.entities.push(second);
        game.player.hp = 0;

        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("fatal scheduler case should execute");

        assert!(update.player.is_dead);
        assert_eq!(
            update
                .events
                .iter()
                .filter(|event| event.message_key == "combat-player-death")
                .count(),
            1
        );
        let second = update
            .entities
            .iter()
            .find(|entity| entity.id == "demo.monster.ember-mote.1")
            .expect("second monster should remain present");
        assert_eq!(second.energy_need, 10);
    }

    #[test]
    fn save_payload_restores_identical_state() {
        let mut game = Game::new(7);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");

        let restored = Game::from_save(game.to_save()).expect("save should restore");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.snapshot(), game.snapshot());
        assert_eq!(restored.snapshot().equipment.len(), 1);
    }

    #[test]
    fn pickup_moves_the_ground_stack_into_inventory() {
        let mut game = Game::new(42);
        game.entities.clear();
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("move should execute");
        let update = game
            .dispatch(command(2, 1, GameCommand::PickUp))
            .expect("pickup should execute");

        assert_eq!(update.items.len(), 1);
        assert_eq!(update.inventory.len(), 1);
        assert_eq!(update.inventory[0].id, "demo.item.luminous-shard.1");
        assert_eq!(update.inventory[0].quantity, 5);
        assert_eq!(update.changed_cells.len(), 1);
        assert_eq!(update.changed_cells[0].position, Position { x: 4, y: 3 });
        assert_eq!(update.changed_cells[0].item_id, None);
        assert_eq!(update.events[0].message_key, "item-pickup-success");
    }

    #[test]
    fn equipping_and_unequipping_moves_an_item_between_authoritative_lists() {
        let mut game = Game::new(42);
        game.entities.clear();
        collect_both_demo_items(&mut game);
        let equipped = game
            .dispatch(command(
                5,
                4,
                GameCommand::Equip {
                    item_id: "demo.item.echo-charm.1".to_owned(),
                },
            ))
            .expect("equipping should execute");

        assert_eq!(equipped.inventory.len(), 1);
        assert_eq!(equipped.equipment.len(), 1);
        assert_eq!(equipped.equipment[0].slot_id, "charm");
        assert_eq!(equipped.equipment[0].modifiers.attack, 1);
        assert_eq!(equipped.equipment[0].modifiers.defense, 1);
        assert_eq!(equipped.equipment[0].modifiers.max_hp, 4);
        assert_eq!(equipped.player.base_max_hp, 10);
        assert_eq!(equipped.player.max_hp, 14);
        assert_eq!(equipped.player.base_attack, 2);
        assert_eq!(equipped.player.attack, 3);
        assert_eq!(equipped.player.base_defense, 1);
        assert_eq!(equipped.player.defense, 2);
        assert_eq!(equipped.player.equipment_modifiers.attack, 1);
        assert_eq!(equipped.player.equipment_modifiers.defense, 1);
        assert_eq!(equipped.player.equipment_modifiers.max_hp, 4);
        assert_eq!(equipped.events[0].message_key, "item-equip-success");

        game.player.hp = 14;

        let unequipped = game
            .dispatch(command(
                6,
                5,
                GameCommand::Unequip {
                    slot_id: "charm".to_owned(),
                },
            ))
            .expect("unequipping should execute");
        assert_eq!(unequipped.inventory.len(), 2);
        assert!(unequipped.equipment.is_empty());
        assert_eq!(unequipped.player.hp, 10);
        assert_eq!(unequipped.player.max_hp, 10);
        assert_eq!(unequipped.player.attack, 2);
        assert_eq!(unequipped.player.defense, 1);
        assert_eq!(unequipped.events[0].message_key, "item-unequip-success");
    }

    #[test]
    fn player_derived_stats_retain_equipment_and_status_sources() {
        let mut game = Game::new(42);
        game.entities.clear();
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equipping should execute");
        game.player.statuses.push(StatusInstance {
            kind_id: STATUS_HASTE.to_owned(),
            intensity: 2,
            remaining_ticks: 3,
            source_id: Some("demo.item.temporary-tonic.1".to_owned()),
        });

        let stats = game.player_derived_stats();

        assert_eq!(stats.attack.value, 3);
        assert_eq!(stats.speed.value, 130);
        assert!(stats.attack.contributions.iter().any(|contribution| {
            contribution.layer == StatLayer::Equipment
                && contribution.source_id == "demo.item.echo-charm.1"
                && contribution.amount == 1
        }));
        assert!(stats.speed.contributions.iter().any(|contribution| {
            contribution.layer == StatLayer::Status
                && contribution.source_id == STATUS_HASTE
                && contribution.origin_id.as_deref() == Some("demo.item.temporary-tonic.1")
                && contribution.amount == 20
        }));
    }

    #[test]
    fn item_instance_identity_survives_location_transitions() {
        let mut game = Game::new(42);
        game.entities.clear();
        let original_instance_count = game.items.len();
        collect_both_demo_items(&mut game);

        let charm_id = "demo.item.echo-charm.1";
        assert_eq!(game.items.len(), original_instance_count);
        assert!(game.items.iter().any(|item| {
            item.id == charm_id && item.location == ItemLocation::Inventory && item.quantity == 1
        }));

        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: charm_id.to_owned(),
            },
        ))
        .expect("equip should execute");
        assert!(game.items.iter().any(|item| {
            item.id == charm_id
                && item.location
                    == ItemLocation::Equipped {
                        slot_id: "charm".to_owned(),
                    }
        }));

        game.dispatch(command(
            6,
            5,
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
        ))
        .expect("unequip should execute");
        game.dispatch(command(
            7,
            6,
            GameCommand::Drop {
                item_ids: vec![charm_id.to_owned()],
            },
        ))
        .expect("drop should execute");

        assert_eq!(game.items.len(), original_instance_count);
        assert!(game.items.iter().any(|item| {
            item.id == charm_id
                && item.location == ItemLocation::Ground(game.player.position)
                && item.quantity == 1
        }));
    }

    #[test]
    fn equipped_attack_modifier_changes_authoritative_melee_skill() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        game.dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equip should execute");
        game.entities[0].position = Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        };
        game.entities[0].energy_need = STANDARD_ACTION_COST;
        game.rng = RfbRng::seeded(42);
        let update = game
            .dispatch(command(
                6,
                5,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("equipped attack should execute");

        assert_eq!(update.events[0].message_key, "combat-player-hit");
        assert_eq!(update.player.melee_skill, 60);
        assert_eq!(update.events[0].args["damage"], "2");
        assert_eq!(update.entities[0].hp, 1);
    }

    #[test]
    fn dropping_multiple_selected_stacks_is_atomic_and_deterministic() {
        let mut game = Game::new(42);
        collect_both_demo_items(&mut game);
        let update = game
            .dispatch(command(
                5,
                4,
                GameCommand::Drop {
                    item_ids: vec![
                        "demo.item.luminous-shard.1".to_owned(),
                        "demo.item.echo-charm.1".to_owned(),
                    ],
                },
            ))
            .expect("batch drop should execute");

        assert!(update.inventory.is_empty());
        assert_eq!(update.items.len(), 2);
        assert!(
            update
                .items
                .iter()
                .all(|item| item.position == Position { x: 5, y: 3 })
        );
        assert_eq!(update.changed_cells.len(), 1);
        assert_eq!(update.events[0].message_key, "item-drop-success");
        assert_eq!(update.events[0].args["stacks"], "2");
        assert_eq!(update.events[0].args["quantity"], "6");
    }

    #[test]
    fn pickup_on_empty_ground_is_a_deterministic_turn() {
        let mut game = Game::new(42);
        game.entities.clear();
        let before = game.state_hash();
        let update = game
            .dispatch(command(1, 0, GameCommand::PickUp))
            .expect("empty pickup should still execute");

        assert_eq!(update.turn, 1);
        assert!(update.changed_cells.is_empty());
        assert!(update.inventory.is_empty());
        assert_eq!(update.events[0].message_key, "item-pickup-none");
        assert_ne!(update.state_hash, before);
    }

    #[test]
    fn pickup_merges_into_the_lowest_id_compatible_stack() {
        let mut game = Game::new(42);
        game.entities.clear();
        game.items.push(ItemInstance {
            id: "demo.inventory.luminous-shard.1".to_owned(),
            kind_id: "demo.item.luminous-shard".to_owned(),
            quantity: 19,
            location: ItemLocation::Inventory,
        });
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("move should execute");
        let update = game
            .dispatch(command(2, 1, GameCommand::PickUp))
            .expect("pickup should execute");

        assert_eq!(update.inventory.len(), 2);
        assert_eq!(update.inventory[0].id, "demo.inventory.luminous-shard.1");
        assert_eq!(update.inventory[0].quantity, 20);
        assert_eq!(update.inventory[1].id, "demo.item.luminous-shard.1");
        assert_eq!(update.inventory[1].quantity, 4);
    }

    #[test]
    fn partial_drop_allocates_stable_ids_and_survives_save_round_trip() {
        let mut game = Game::new(42);
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("move should execute");
        game.dispatch(command(2, 1, GameCommand::PickUp))
            .expect("pickup should execute");
        let first_drop = game
            .dispatch(command(
                3,
                2,
                GameCommand::DropQuantity {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    quantity: 2,
                },
            ))
            .expect("partial drop should execute");

        assert_eq!(first_drop.inventory[0].quantity, 3);
        assert!(first_drop.items.iter().any(|item| {
            item.id == "generated.item.1"
                && item.quantity == 2
                && item.position == Position { x: 4, y: 3 }
        }));
        assert_eq!(game.next_item_instance_serial, 2);

        let mut restored = Game::from_save(game.to_save()).expect("save should preserve allocator");
        let second_drop = restored
            .dispatch(command(
                4,
                3,
                GameCommand::DropQuantity {
                    item_id: "demo.item.luminous-shard.1".to_owned(),
                    quantity: 1,
                },
            ))
            .expect("second partial drop should execute");
        assert!(
            second_drop
                .items
                .iter()
                .any(|item| item.id == "generated.item.2" && item.quantity == 1)
        );
        assert_eq!(restored.next_item_instance_serial, 3);
    }

    #[test]
    fn stale_revision_is_rejected_without_mutation() {
        let mut game = Game::new(1);
        let before = game.state_hash();
        let error = game
            .dispatch(command(1, 99, GameCommand::Wait))
            .expect_err("stale command should fail");
        assert!(matches!(error, CoreError::RevisionMismatch { .. }));
        assert_eq!(game.state_hash(), before);
    }

    #[test]
    fn rfb_style_armor_reduction_uses_the_legacy_linear_cap() {
        assert_eq!(apply_melee_armor_reduction(100, 0), 100);
        assert_eq!(apply_melee_armor_reduction(100, 90), 70);
        assert_eq!(apply_melee_armor_reduction(100, 180), 40);
        assert_eq!(apply_melee_armor_reduction(100, 999), 40);
    }

    #[test]
    fn fixed_seed_exercises_player_miss_and_death_rejection() {
        let mut miss_game = Game::new(0);
        miss_game.entities[0].position = Position { x: 4, y: 4 };
        miss_game.entities[0].energy_need = STANDARD_ACTION_COST;
        let miss_update = miss_game
            .dispatch(command(
                1,
                0,
                GameCommand::Move {
                    direction: Direction::SouthEast,
                },
            ))
            .expect("fixed-seed player attack should execute");
        assert!(
            miss_update
                .events
                .iter()
                .any(|event| event.message_key == "combat-player-miss")
        );

        let mut game = Game::new(0);
        game.entities[0].position = Position { x: 4, y: 4 };
        game.entities[0].energy_need = STANDARD_ACTION_COST;
        game.player.hp = 0;
        let update = game
            .dispatch(command(1, 0, GameCommand::Wait))
            .expect("adjacent monster turn should execute");
        assert!(update.player.is_dead);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.message_key == "combat-player-death")
        );
        assert!(matches!(
            game.dispatch(command(2, 1, GameCommand::Wait)),
            Err(CoreError::PlayerDead)
        ));

        let mut full_health_game = Game::new(0);
        full_health_game.entities[0].position = Position { x: 4, y: 4 };
        full_health_game.entities[0].energy_need = STANDARD_ACTION_COST;
        let death_command = (1..100_u32).find(|seq| {
            full_health_game
                .dispatch(command(*seq, *seq - 1, GameCommand::Wait))
                .is_ok_and(|update| update.player.is_dead)
        });
        assert!(death_command.is_some());
    }

    fn collect_both_demo_items(game: &mut Game) {
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("movement to shard should execute");
        game.dispatch(command(2, 1, GameCommand::PickUp))
            .expect("shard pickup should execute");
        game.dispatch(command(
            3,
            2,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("movement to charm should execute");
        game.dispatch(command(4, 3, GameCommand::PickUp))
            .expect("charm pickup should execute");
    }
}

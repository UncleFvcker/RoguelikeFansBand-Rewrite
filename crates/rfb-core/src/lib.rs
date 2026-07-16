// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use rfb_content::{ActorRole, ContentCatalog, ContentError, ContentPosition};
use rfb_protocol::{
    CellDto, CellLightDto, CellVisualDto, ContentVisualDto, EntityDto, EquipmentItemDto,
    GameCommand, GameCommandEnvelope, GameEventDto, GameSnapshot, GameUpdate, InventoryItemDto,
    ItemDto, PROTOCOL_VERSION, PlayerDto, Position, RngSaveDto, SavePayloadV1, StatModifiersDto,
    TerrainSaveDto, VisibilityState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const BUILT_IN_WORLD_ID: &str = "demo.world.original-v1";
pub const RNG_ALGORITHM: &str = "rfb-rng-xoshiro256ss-v1";
const PREVIOUS_BUILT_IN_CONTENT_HASHES: [&str; 3] = [
    "880610557b208e7c2459ff876c4ace1cb2ef9903986cb7883a04d511ca13c025",
    "0a76daadea3a9683ea8173aa8f65e6195a5582bdf7fdad215cea1a2896dfefcc",
    "cd2c813d224189c925a940e60a915fe3dcf6efa0ccadfc7363d06d428f56525f",
];
const BUILT_IN_CONTENT_HASH: &str =
    "36bdba260173b9ba7477e85b886c134affed0369aa4f7a485e59e4408e618ebd";
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
const GENERATED_ITEM_ID_PREFIX: &str = "generated.item.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Actor {
    id: String,
    kind_id: String,
    position: Position,
    hp: i32,
    max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Item {
    id: String,
    kind_id: String,
    position: Position,
    quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InventoryItem {
    id: String,
    kind_id: String,
    quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EquipmentItem {
    id: String,
    kind_id: String,
    quantity: u32,
    slot_id: String,
}

struct EquipOutcome {
    kind_id: String,
    slot_id: String,
    replaced_kind_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfbRng {
    state: [u64; 4],
    draw_counter: u64,
}

impl RfbRng {
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        let mut splitmix_state = seed;
        let mut state = [0_u64; 4];
        for value in &mut state {
            *value = splitmix64(&mut splitmix_state);
        }
        if state == [0; 4] {
            state[0] = 1;
        }
        Self {
            state,
            draw_counter: 0,
        }
    }

    fn from_save(save: &RngSaveDto) -> Result<Self, CoreError> {
        if save.algorithm != RNG_ALGORITHM {
            return Err(CoreError::UnsupportedRng(save.algorithm.clone()));
        }
        if save.state == [0; 4] {
            return Err(CoreError::InvalidSave("RNG state cannot be all zero"));
        }
        Ok(Self {
            state: save.state,
            draw_counter: save.draw_counter,
        })
    }

    fn to_save(&self) -> RngSaveDto {
        RngSaveDto {
            algorithm: RNG_ALGORITHM.to_owned(),
            state: self.state,
            draw_counter: self.draw_counter,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;

        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);
        self.draw_counter = self.draw_counter.wrapping_add(1);
        result
    }

    fn bounded(&mut self, upper_exclusive: u64) -> u64 {
        assert!(upper_exclusive > 0, "RNG bound must be positive");
        let threshold = upper_exclusive.wrapping_neg() % upper_exclusive;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % upper_exclusive;
            }
        }
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
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
    items: Vec<Item>,
    inventory: Vec<InventoryItem>,
    equipment: Vec<EquipmentItem>,
    next_item_instance_serial: u64,
    explored: Vec<bool>,
    rng: RfbRng,
    revision: u32,
    turn: u32,
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
                ))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let items = world
            .items
            .iter()
            .map(|spawn| Item {
                id: spawn.instance_id.clone(),
                kind_id: spawn.kind_id.clone(),
                position: position_from_content(spawn.position),
                quantity: spawn.quantity,
            })
            .collect::<Vec<_>>();
        let next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items, &[], &[])?;
        let mut game = Self {
            content,
            world_id: world_id.to_owned(),
            width,
            height,
            terrain,
            player,
            entities,
            items,
            inventory: Vec::new(),
            equipment: Vec::new(),
            next_item_instance_serial,
            explored: vec![false; usize::from(width) * usize::from(height)],
            rng: RfbRng::seeded(seed),
            revision: 0,
            turn: 0,
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
        let items = payload
            .items
            .into_iter()
            .map(item_from_dto)
            .collect::<Vec<_>>();
        let inventory = payload
            .inventory
            .into_iter()
            .map(|item| inventory_item_from_dto(item, &content))
            .collect::<Result<Vec<_>, CoreError>>()?;
        let equipment = payload
            .equipment
            .into_iter()
            .map(|item| equipment_item_from_dto(item, &content))
            .collect::<Result<Vec<_>, CoreError>>()?;
        let derived_next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items, &inventory, &equipment)?;
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
            inventory,
            equipment,
            next_item_instance_serial,
            explored,
            rng: RfbRng::from_save(&payload.rng)?,
            revision: payload.revision,
            turn: payload.turn,
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
            last_command_seq: self.last_command_seq,
            terrain: TerrainSaveDto {
                width: self.width,
                height: self.height,
                terrain_ids: self.terrain.clone(),
            },
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
            inventory: self.inventory_dto(),
            equipment: self.equipment_dto(),
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

        let base_revision = self.revision;
        let previous_visuals = self.visual_cells();
        let mut changed = BTreeSet::new();
        let mut events = Vec::new();
        let mut removed_entities = Vec::new();

        match envelope.command {
            GameCommand::Drop { item_ids } => {
                if let Some((stacks, quantity)) = self.drop_inventory_items(&item_ids) {
                    changed.insert(self.player.position);
                    events.push(event_with_args(
                        "item.drop",
                        "item-drop-success",
                        [
                            ("stacks", stacks.to_string()),
                            ("quantity", quantity.to_string()),
                        ],
                    ));
                } else {
                    events.push(event("item.drop.none", "item-drop-none"));
                }
            }
            GameCommand::DropQuantity { item_id, quantity } => {
                if let Some((stacks, dropped_quantity)) =
                    self.drop_inventory_quantity(&item_id, quantity)?
                {
                    changed.insert(self.player.position);
                    events.push(event_with_args(
                        "item.drop",
                        "item-drop-success",
                        [
                            ("stacks", stacks.to_string()),
                            ("quantity", dropped_quantity.to_string()),
                        ],
                    ));
                } else {
                    events.push(event("item.drop.none", "item-drop-none"));
                }
            }
            GameCommand::Equip { item_id } => {
                if let Some(outcome) = self.equip_inventory_item(&item_id) {
                    if let Some(replaced_kind_id) = outcome.replaced_kind_id {
                        events.push(event_with_args(
                            "item.equip.swap",
                            "item-equip-swap",
                            [
                                ("target", outcome.kind_id),
                                ("replaced", replaced_kind_id),
                                ("slot", outcome.slot_id),
                            ],
                        ));
                    } else {
                        events.push(event_with_args(
                            "item.equip",
                            "item-equip-success",
                            [("target", outcome.kind_id), ("slot", outcome.slot_id)],
                        ));
                    }
                } else {
                    events.push(event("item.equip.none", "item-equip-unavailable"));
                }
            }
            GameCommand::Wait => events.push(event("turn.wait", "game-wait")),
            GameCommand::PickUp => {
                if let Some((kind_id, quantity)) = self.pick_up_at_player()? {
                    changed.insert(self.player.position);
                    events.push(event_with_args(
                        "item.pickup",
                        "item-pickup-success",
                        [("target", kind_id), ("quantity", quantity.to_string())],
                    ));
                } else {
                    events.push(event("item.pickup.none", "item-pickup-none"));
                }
            }
            GameCommand::Unequip { slot_id } => {
                if let Some(kind_id) = self.unequip_slot(&slot_id) {
                    events.push(event_with_args(
                        "item.unequip",
                        "item-unequip-success",
                        [("target", kind_id), ("slot", slot_id)],
                    ));
                } else {
                    events.push(event_with_args(
                        "item.unequip.none",
                        "item-unequip-none",
                        [("slot", slot_id)],
                    ));
                }
            }
            GameCommand::Move { direction } => {
                let (dx, dy) = direction.delta();
                let target = Position {
                    x: self.player.position.x + dx,
                    y: self.player.position.y + dy,
                };
                if !self.is_walkable(target) {
                    events.push(event("move.blocked", "game-move-blocked"));
                } else if let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.position == target)
                {
                    changed.insert(target);
                    let attack = self.effective_player_attack();
                    let defense = self
                        .content
                        .actor(&self.entities[index].kind_id)
                        .map_or(0, |definition| definition.defense);
                    let roll = i32::try_from(self.rng.bounded(2)).unwrap_or(0);
                    let damage = attack.saturating_add(roll).saturating_sub(defense).max(1);
                    let monster = &mut self.entities[index];
                    monster.hp -= damage;
                    events.push(event_with_args(
                        "combat.hit",
                        "combat-player-hit",
                        [
                            ("target", monster.kind_id.clone()),
                            ("damage", damage.to_string()),
                        ],
                    ));
                    if monster.hp <= 0 {
                        let removed = self.entities.remove(index);
                        removed_entities.push(removed.id);
                        events.push(event_with_args(
                            "combat.slay",
                            "combat-player-slay",
                            [("target", removed.kind_id)],
                        ));
                    }
                } else {
                    let old_position = self.player.position;
                    self.player.position = target;
                    changed.insert(old_position);
                    changed.insert(target);
                }
            }
        }

        self.last_command_seq = envelope.command_seq;
        self.turn = self.turn.saturating_add(1);
        self.revision = self.revision.saturating_add(1);
        self.reveal_current_visibility();
        let changed_visual_cells = self.changed_visual_cells(&previous_visuals);

        Ok(GameUpdate {
            base_revision,
            revision: self.revision,
            turn: self.turn,
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
        let mut payload = self.to_save();
        payload.explored.clear();
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
            max_hp: self
                .player
                .max_hp
                .saturating_add(equipment_modifiers.max_hp),
            base_max_hp: self.player.max_hp,
            attack: self.effective_player_attack(),
            base_attack: definition.attack,
            defense: self.effective_player_defense(),
            base_defense: definition.defense,
            equipment_modifiers,
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
                EntityDto {
                    id: entity.id.clone(),
                    kind_id: entity.kind_id.clone(),
                    position: entity.position,
                    hp: entity.hp,
                    max_hp: entity.max_hp,
                    attack: definition.attack,
                    defense: definition.defense,
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
            .map(|item| ItemDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                position: item.position,
                quantity: item.quantity,
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.id.cmp(&right.id));
        items
    }

    fn inventory_dto(&self) -> Vec<InventoryItemDto> {
        let mut inventory = self
            .inventory
            .iter()
            .map(|item| InventoryItemDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                equipment_slot: self
                    .content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.equipment_slot.clone()),
                modifiers: self.item_modifiers(&item.kind_id),
            })
            .collect::<Vec<_>>();
        inventory.sort_by(|left, right| left.id.cmp(&right.id));
        inventory
    }

    fn equipment_dto(&self) -> Vec<EquipmentItemDto> {
        let mut equipment = self
            .equipment
            .iter()
            .map(|item| EquipmentItemDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                slot_id: item.slot_id.clone(),
                modifiers: self.item_modifiers(&item.kind_id),
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
        let mut kept = Vec::with_capacity(self.inventory.len());
        let mut dropped = Vec::new();
        for item in std::mem::take(&mut self.inventory) {
            if selected.contains(item.id.as_str()) {
                dropped.push(item);
            } else {
                kept.push(item);
            }
        }
        self.inventory = kept;
        if dropped.is_empty() {
            return None;
        }
        dropped.sort_by(|left, right| left.id.cmp(&right.id));
        let quantity = dropped.iter().fold(0_u64, |total, item| {
            total.saturating_add(u64::from(item.quantity))
        });
        let stacks = dropped.len();
        self.items.extend(dropped.into_iter().map(|item| Item {
            id: item.id,
            kind_id: item.kind_id,
            position: self.player.position,
            quantity: item.quantity,
        }));
        Some((stacks, quantity))
    }

    fn drop_inventory_quantity(
        &mut self,
        item_id: &str,
        quantity: u32,
    ) -> Result<Option<(usize, u64)>, CoreError> {
        let Some(index) = self.inventory.iter().position(|item| item.id == item_id) else {
            return Ok(None);
        };
        if quantity == 0 || quantity > self.inventory[index].quantity {
            return Ok(None);
        }
        let dropped = if quantity == self.inventory[index].quantity {
            let item = self.inventory.remove(index);
            Item {
                id: item.id,
                kind_id: item.kind_id,
                position: self.player.position,
                quantity,
            }
        } else {
            let id = self.allocate_item_instance_id()?;
            self.inventory[index].quantity -= quantity;
            Item {
                id,
                kind_id: self.inventory[index].kind_id.clone(),
                position: self.player.position,
                quantity,
            }
        };
        self.items.push(dropped);
        Ok(Some((1, u64::from(quantity))))
    }

    fn equip_inventory_item(&mut self, item_id: &str) -> Option<EquipOutcome> {
        let inventory_index = self.inventory.iter().position(|item| item.id == item_id)?;
        let carried = &self.inventory[inventory_index];
        let slot_id = self
            .content
            .item(&carried.kind_id)?
            .equipment_slot
            .clone()?;
        if carried.quantity != 1 {
            return None;
        }
        let item = self.inventory.remove(inventory_index);
        let replaced_kind_id = self
            .equipment
            .iter()
            .position(|equipped| equipped.slot_id == slot_id)
            .map(|index| {
                let replaced = self.equipment.remove(index);
                let kind_id = replaced.kind_id.clone();
                self.inventory.push(InventoryItem {
                    id: replaced.id,
                    kind_id: replaced.kind_id,
                    quantity: replaced.quantity,
                });
                kind_id
            });
        let kind_id = item.kind_id.clone();
        self.equipment.push(EquipmentItem {
            id: item.id,
            kind_id: item.kind_id,
            quantity: item.quantity,
            slot_id: slot_id.clone(),
        });
        self.clamp_player_hp_to_effective_max();
        Some(EquipOutcome {
            kind_id,
            slot_id,
            replaced_kind_id,
        })
    }

    fn unequip_slot(&mut self, slot_id: &str) -> Option<String> {
        let index = self
            .equipment
            .iter()
            .position(|item| item.slot_id == slot_id)?;
        let item = self.equipment.remove(index);
        let kind_id = item.kind_id.clone();
        self.inventory.push(InventoryItem {
            id: item.id,
            kind_id: item.kind_id,
            quantity: item.quantity,
        });
        self.clamp_player_hp_to_effective_max();
        Some(kind_id)
    }

    fn pick_up_at_player(&mut self) -> Result<Option<(String, u32)>, CoreError> {
        let Some(index) = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.position == self.player.position)
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
        let item = self.items.remove(index);
        let mut remaining = item.quantity;
        let mut stack_indices = self
            .inventory
            .iter()
            .enumerate()
            .filter(|(_, carried)| carried.kind_id == item.kind_id && carried.quantity < max_stack)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        stack_indices
            .sort_by(|left, right| self.inventory[*left].id.cmp(&self.inventory[*right].id));
        for stack_index in stack_indices {
            let stack = &mut self.inventory[stack_index];
            let transferred = remaining.min(max_stack - stack.quantity);
            stack.quantity += transferred;
            remaining -= transferred;
            if remaining == 0 {
                break;
            }
        }
        if remaining > 0 {
            self.inventory.push(InventoryItem {
                id: item.id,
                kind_id: item.kind_id.clone(),
                quantity: remaining,
            });
        }
        Ok(Some((item.kind_id, item.quantity)))
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
        self.equipment
            .iter()
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
        self.player
            .max_hp
            .saturating_add(self.equipment_modifiers().max_hp)
    }

    fn effective_player_attack(&self) -> i32 {
        let base = self
            .content
            .actor(&self.player.kind_id)
            .map_or(0, |definition| definition.attack);
        effective_stat(base, self.equipment_modifiers().attack)
    }

    fn effective_player_defense(&self) -> i32 {
        let base = self
            .content
            .actor(&self.player.kind_id)
            .map_or(0, |definition| definition.defense);
        effective_stat(base, self.equipment_modifiers().defense)
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
            || self.inventory.iter().any(|item| item.id == candidate)
            || self.equipment.iter().any(|item| item.id == candidate)
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
                .find(|item| item.position == position)
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
            let Some(definition) = self.content.item(&item.kind_id) else {
                continue;
            };
            if !definition.tags.iter().any(|tag| tag == "light-source") {
                continue;
            }
            let boost = source_intensity(item.position, position, ITEM_LIGHT_RADIUS, 52);
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
        for item in &self.items {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            if !instance_ids.insert(item.id.clone())
                || !self.is_walkable(item.position)
                || item.quantity == 0
                || item.quantity > definition.max_stack
            {
                return Err(CoreError::InvalidSave("item state is invalid"));
            }
        }
        for item in &self.inventory {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            if !instance_ids.insert(item.id.clone())
                || item.quantity == 0
                || item.quantity > definition.max_stack
            {
                return Err(CoreError::InvalidSave("inventory item state is invalid"));
            }
        }
        let mut equipment_slots = BTreeSet::new();
        for item in &self.equipment {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            if !instance_ids.insert(item.id.clone())
                || item.quantity != 1
                || definition.equipment_slot.as_deref() != Some(item.slot_id.as_str())
                || !equipment_slots.insert(item.slot_id.clone())
            {
                return Err(CoreError::InvalidSave("equipment item state is invalid"));
            }
        }
        if self.next_item_instance_serial == 0
            || self.next_item_instance_serial
                < derive_next_item_instance_serial(
                    &self.player,
                    &self.entities,
                    &self.items,
                    &self.inventory,
                    &self.equipment,
                )?
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
        if definition.role != expected_role
            || actor.max_hp != definition.max_hp
            || actor.hp <= 0
            || actor.hp > effective_max_hp
        {
            return Err(CoreError::InvalidSave("actor state is invalid"));
        }
        Ok(())
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

fn actor_from_spawn(id: &str, kind_id: &str, position: ContentPosition, max_hp: i32) -> Actor {
    Actor {
        id: id.to_owned(),
        kind_id: kind_id.to_owned(),
        position: position_from_content(position),
        hp: max_hp,
        max_hp,
    }
}

const fn position_from_content(position: ContentPosition) -> Position {
    Position {
        x: position.x as i32,
        y: position.y as i32,
    }
}

fn effective_stat(base: i32, modifier: i32) -> i32 {
    base.saturating_add(modifier).max(0)
}

fn actor_from_player(player: PlayerDto, content: &ContentCatalog) -> Result<Actor, CoreError> {
    let definition = content
        .actor(&player.kind_id)
        .ok_or_else(|| CoreError::UnknownActor(player.kind_id.clone()))?;
    if player.base_max_hp != 0 && player.base_max_hp != definition.max_hp {
        return Err(CoreError::InvalidSave("player base max HP is invalid"));
    }
    if player.base_attack != 0 && player.base_attack != definition.attack {
        return Err(CoreError::InvalidSave("player base attack is invalid"));
    }
    if player.base_defense != 0 && player.base_defense != definition.defense {
        return Err(CoreError::InvalidSave("player base defense is invalid"));
    }
    Ok(Actor {
        id: player.id,
        kind_id: player.kind_id,
        position: player.position,
        hp: player.hp,
        max_hp: definition.max_hp,
    })
}

fn derive_next_item_instance_serial(
    player: &Actor,
    entities: &[Actor],
    items: &[Item],
    inventory: &[InventoryItem],
    equipment: &[EquipmentItem],
) -> Result<u64, CoreError> {
    let maximum = std::iter::once(player.id.as_str())
        .chain(entities.iter().map(|entity| entity.id.as_str()))
        .chain(items.iter().map(|item| item.id.as_str()))
        .chain(inventory.iter().map(|item| item.id.as_str()))
        .chain(equipment.iter().map(|item| item.id.as_str()))
        .filter_map(generated_item_serial)
        .max()
        .unwrap_or(0);
    maximum.checked_add(1).ok_or(CoreError::ItemIdExhausted)
}

fn generated_item_serial(id: &str) -> Option<u64> {
    id.strip_prefix(GENERATED_ITEM_ID_PREFIX)?.parse().ok()
}

fn actor_from_entity(entity: EntityDto, content: &ContentCatalog) -> Result<Actor, CoreError> {
    let definition = content
        .actor(&entity.kind_id)
        .ok_or_else(|| CoreError::UnknownActor(entity.kind_id.clone()))?;
    if entity.max_hp != definition.max_hp
        || (entity.attack != 0 && entity.attack != definition.attack)
        || (entity.defense != 0 && entity.defense != definition.defense)
    {
        return Err(CoreError::InvalidSave("entity base stats are invalid"));
    }
    Ok(Actor {
        id: entity.id,
        kind_id: entity.kind_id,
        position: entity.position,
        hp: entity.hp,
        max_hp: definition.max_hp,
    })
}

fn item_from_dto(item: ItemDto) -> Item {
    Item {
        id: item.id,
        kind_id: item.kind_id,
        position: item.position,
        quantity: item.quantity,
    }
}

fn inventory_item_from_dto(
    item: InventoryItemDto,
    content: &ContentCatalog,
) -> Result<InventoryItem, CoreError> {
    let equipment_slot = content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?
        .equipment_slot
        .as_deref();
    if item.equipment_slot.as_deref() != equipment_slot {
        return Err(CoreError::InvalidSave(
            "inventory equipment metadata is invalid",
        ));
    }
    Ok(InventoryItem {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
    })
}

fn equipment_item_from_dto(
    item: EquipmentItemDto,
    content: &ContentCatalog,
) -> Result<EquipmentItem, CoreError> {
    let definition = content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    if definition.equipment_slot.as_deref() != Some(item.slot_id.as_str()) {
        return Err(CoreError::InvalidSave("equipment metadata is invalid"));
    }
    Ok(EquipmentItem {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        slot_id: item.slot_id,
    })
}

fn event(kind: &str, message_key: &str) -> GameEventDto {
    GameEventDto {
        kind: kind.to_owned(),
        message_key: message_key.to_owned(),
        args: BTreeMap::new(),
    }
}

fn event_with_args<const N: usize>(
    kind: &str,
    message_key: &str,
    args: [(&str, String); N],
) -> GameEventDto {
    GameEventDto {
        kind: kind.to_owned(),
        message_key: message_key.to_owned(),
        args: args
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    }
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("revision mismatch: core is at {expected}, command expected {received}")]
    RevisionMismatch { expected: u32, received: u32 },
    #[error("command sequence mismatch: expected {expected}, received {received}")]
    CommandSequence { expected: u32, received: u32 },
    #[error("unsupported save schema version {0}")]
    UnsupportedSaveVersion(u16),
    #[error("save uses unsupported RNG algorithm {0}")]
    UnsupportedRng(String),
    #[error("save content set does not match the demo content set")]
    ContentMismatch,
    #[error("content set does not define world {0}")]
    UnknownWorld(String),
    #[error("save contains unknown terrain ID {0}")]
    UnknownTerrain(String),
    #[error("content set does not define actor {0}")]
    UnknownActor(String),
    #[error("content set does not define item {0}")]
    UnknownItem(String),
    #[error("generated item instance ID space is exhausted")]
    ItemIdExhausted,
    #[error("invalid save: {0}")]
    InvalidSave(&'static str),
    #[error(transparent)]
    Content(#[from] ContentError),
}

#[cfg(test)]
mod tests {
    use rfb_protocol::{Direction, GameCommand, GameCommandEnvelope, VisibilityState};

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
    fn previous_built_in_content_hash_migrates_without_spawning_new_items() {
        let mut payload = Game::new(42).to_save();
        payload.content_hash = PREVIOUS_BUILT_IN_CONTENT_HASHES[0].to_owned();
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
        payload.player.max_hp = 10;
        payload.player.equipment_modifiers = StatModifiersDto::default();
        payload.equipment[0].modifiers = StatModifiersDto::default();
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
        payload.player.attack = 0;
        payload.player.base_attack = 0;
        payload.player.defense = 0;
        payload.player.base_defense = 0;
        payload.player.equipment_modifiers.attack = 0;
        payload.player.equipment_modifiers.defense = 0;
        for entity in &mut payload.entities {
            entity.attack = 0;
            entity.defense = 0;
        }
        payload.equipment[0].modifiers.attack = 0;
        payload.equipment[0].modifiers.defense = 0;

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
    fn equipped_attack_modifier_changes_authoritative_melee_damage() {
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
        for (seq, direction) in [(6, Direction::SouthEast), (7, Direction::SouthEast)] {
            game.dispatch(command(seq, seq - 1, GameCommand::Move { direction }))
                .expect("path to monster should execute");
        }
        let update = game
            .dispatch(command(
                8,
                7,
                GameCommand::Move {
                    direction: Direction::East,
                },
            ))
            .expect("equipped attack should execute");

        assert_eq!(update.events[0].message_key, "combat-player-hit");
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
        game.inventory.push(InventoryItem {
            id: "demo.inventory.luminous-shard.1".to_owned(),
            kind_id: "demo.item.luminous-shard".to_owned(),
            quantity: 19,
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

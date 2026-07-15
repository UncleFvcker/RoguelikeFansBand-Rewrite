// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use rfb_content::{ActorRole, ContentCatalog, ContentError, ContentPosition};
use rfb_protocol::{
    CellDto, ContentVisualDto, EntityDto, GameCommand, GameCommandEnvelope, GameEventDto,
    GameSnapshot, GameUpdate, ItemDto, PROTOCOL_VERSION, PlayerDto, Position, RngSaveDto,
    SavePayloadV1, TerrainSaveDto,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const BUILT_IN_WORLD_ID: &str = "demo.world.original-v1";
pub const RNG_ALGORITHM: &str = "rfb-rng-xoshiro256ss-v1";
const BUILT_IN_CONTENT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rfb-demo-original.rfbcontent"));

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
            .collect();
        let game = Self {
            content,
            world_id: world_id.to_owned(),
            width,
            height,
            terrain,
            player,
            entities,
            items,
            rng: RfbRng::seeded(seed),
            revision: 0,
            turn: 0,
            last_command_seq: 0,
        };
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
        if payload.content_id != content.pack_id() || payload.content_hash != content.content_hash()
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
        let player = actor_from_player(payload.player);
        let entities = payload
            .entities
            .into_iter()
            .map(actor_from_entity)
            .collect::<Vec<_>>();
        let items = payload.items.into_iter().map(item_from_dto).collect();
        let game = Self {
            content,
            world_id: payload.world_id,
            width: payload.terrain.width,
            height: payload.terrain.height,
            terrain,
            player,
            entities,
            items,
            rng: RfbRng::from_save(&payload.rng)?,
            revision: payload.revision,
            turn: payload.turn,
            last_command_seq: payload.last_command_seq,
        };
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
        GameSnapshot {
            protocol_version: PROTOCOL_VERSION.to_owned(),
            revision: self.revision,
            turn: self.turn,
            last_command_seq: self.last_command_seq,
            width: self.width,
            height: self.height,
            cells,
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
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
        let mut changed = BTreeSet::new();
        let mut events = Vec::new();
        let mut removed_entities = Vec::new();

        match envelope.command {
            GameCommand::Wait => events.push(event("turn.wait", "game-wait")),
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
                    let damage = 1 + i32::try_from(self.rng.bounded(2)).unwrap_or(0);
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
            player: self.player_dto(),
            entities: self.entities_dto(),
            items: self.items_dto(),
            removed_entities,
            state_hash: self.state_hash(),
        })
    }

    #[must_use]
    pub fn state_hash(&self) -> String {
        let bytes = rmp_serde::to_vec_named(&self.to_save())
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
        PlayerDto {
            id: self.player.id.clone(),
            kind_id: self.player.kind_id.clone(),
            position: self.player.position,
            hp: self.player.hp,
            max_hp: self.player.max_hp,
        }
    }

    fn entities_dto(&self) -> Vec<EntityDto> {
        let mut entities = self
            .entities
            .iter()
            .map(|entity| EntityDto {
                id: entity.id.clone(),
                kind_id: entity.kind_id.clone(),
                position: entity.position,
                hp: entity.hp,
                max_hp: entity.max_hp,
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
        for terrain_id in &self.terrain {
            if self.content.terrain(terrain_id).is_none() {
                return Err(CoreError::UnknownTerrain(terrain_id.clone()));
            }
        }
        self.validate_actor(&self.player, ActorRole::Player)?;
        if !self.is_walkable(self.player.position) {
            return Err(CoreError::InvalidSave("player position is invalid"));
        }
        let mut positions = BTreeSet::new();
        positions.insert(self.player.position);
        for entity in &self.entities {
            self.validate_actor(entity, ActorRole::Monster)?;
            if !self.is_walkable(entity.position) || !positions.insert(entity.position) {
                return Err(CoreError::InvalidSave("entity position is invalid"));
            }
        }
        let mut item_ids = BTreeSet::new();
        for item in &self.items {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            if !item_ids.insert(item.id.clone())
                || !self.is_walkable(item.position)
                || item.quantity == 0
                || item.quantity > definition.max_stack
            {
                return Err(CoreError::InvalidSave("item state is invalid"));
            }
        }
        Ok(())
    }

    fn validate_actor(&self, actor: &Actor, expected_role: ActorRole) -> Result<(), CoreError> {
        let definition = self
            .content
            .actor(&actor.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(actor.kind_id.clone()))?;
        if definition.role != expected_role
            || actor.max_hp != definition.max_hp
            || actor.hp <= 0
            || actor.hp > actor.max_hp
        {
            return Err(CoreError::InvalidSave("actor state is invalid"));
        }
        Ok(())
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

fn actor_from_player(player: PlayerDto) -> Actor {
    Actor {
        id: player.id,
        kind_id: player.kind_id,
        position: player.position,
        hp: player.hp,
        max_hp: player.max_hp,
    }
}

fn actor_from_entity(entity: EntityDto) -> Actor {
    Actor {
        id: entity.id,
        kind_id: entity.kind_id,
        position: entity.position,
        hp: entity.hp,
        max_hp: entity.max_hp,
    }
}

fn item_from_dto(item: ItemDto) -> Item {
    Item {
        id: item.id,
        kind_id: item.kind_id,
        position: item.position,
        quantity: item.quantity,
    }
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
    #[error("invalid save: {0}")]
    InvalidSave(&'static str),
    #[error(transparent)]
    Content(#[from] ContentError),
}

#[cfg(test)]
mod tests {
    use rfb_protocol::{Direction, GameCommand, GameCommandEnvelope};

    use super::*;

    fn command(seq: u32, revision: u32, command: GameCommand) -> GameCommandEnvelope {
        GameCommandEnvelope {
            command_seq: seq,
            expected_revision: revision,
            command,
        }
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
        assert_eq!(
            snapshot.content_hash,
            "880610557b208e7c2459ff876c4ace1cb2ef9903986cb7883a04d511ca13c025"
        );
        assert_eq!(snapshot.world_id, BUILT_IN_WORLD_ID);
        assert_eq!(snapshot.player.id, "demo.actor.player.1");
        assert_eq!(snapshot.player.kind_id, "demo.actor.explorer");
        assert_eq!(snapshot.entities[0].position, Position { x: 8, y: 5 });
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
        game.dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("move should execute");

        let restored = Game::from_save(game.to_save()).expect("save should restore");
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.snapshot(), game.snapshot());
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
}

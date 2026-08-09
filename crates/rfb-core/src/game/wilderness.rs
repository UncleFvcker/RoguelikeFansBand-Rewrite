// SPDX-License-Identifier: MPL-2.0

use super::*;
use rfb_content::{
    ActorMovementMode, WildernessDefinition, WildernessLegendEntry, WildernessLocationDefinition,
    WildernessTerrain,
};

pub(super) const WILDERNESS_FLOOR_ID: &str = "core.floor.wilderness";
pub(super) const WORLD_MAP_ACTION_MULTIPLIER: i32 = 132;
pub(super) const WILDERNESS_DAY_TICKS: u32 = 100_000;

const WILDERNESS_BORDER_BLEND: i32 = 8;
const WILDERNESS_AMBUSH_ROLLS: u16 = 20;
const WILDERNESS_AMBUSH_RNG_SALT: u64 = 0xA8B0_5A11;
const WILDERNESS_AMBUSH_ID_MARKER: &str = ".ambush.";
const WILDERNESS_INTERESTING_CHANCE: u64 = 10;
const SURFACE_PATH_ID: &str = "demo.terrain.surface-path";
const SURFACE_GRASS_ID: &str = "demo.terrain.surface-grass";
const SURFACE_WOODLAND_ID: &str = "demo.terrain.surface-woodland";
const SURFACE_TREE_ID: &str = "demo.terrain.surface-tree";
const SURFACE_WATER_SHALLOW_ID: &str = "demo.terrain.surface-water-shallow";
const SURFACE_WATER_DEEP_ID: &str = "demo.terrain.surface-water-deep";
const SURFACE_SWAMP_ID: &str = "demo.terrain.surface-swamp";
const SURFACE_WASTE_ID: &str = "demo.terrain.surface-waste";
const SURFACE_LAVA_SHALLOW_ID: &str = "demo.terrain.surface-lava-shallow";
const SURFACE_LAVA_DEEP_ID: &str = "demo.terrain.surface-lava-deep";
const SURFACE_MOUNTAIN_ID: &str = "demo.terrain.surface-mountain";
const SURFACE_GLACIER_ID: &str = "demo.terrain.surface-glacier";
const SURFACE_SNOW_ID: &str = "demo.terrain.surface-snow";
const SURFACE_PACK_ICE_ID: &str = "demo.terrain.surface-pack-ice";
const SURFACE_ROCK_ID: &str = "demo.terrain.surface-rock";

const RUINED_HOME: [&str; 10] = [
    "###%%%######%%TT%#",
    "#TT.....~..%TT*.T%",
    "+.........~T.....T",
    "#*,,,......%.....%",
    "#T,,.......###T,,.",
    "T....TT....#,TTT,#",
    "#TT...TT...#*,T,.#",
    ",.TT.......#.T,..%",
    "#........~.,TTT..%",
    "##   ##%%,,..T#%%,",
];

fn wilderness_legend_at(
    wilderness: &WildernessDefinition,
    position: Position,
) -> Option<&WildernessLegendEntry> {
    let symbol = usize::try_from(position.y)
        .ok()
        .and_then(|y| wilderness.rows.get(y))
        .and_then(|row| {
            usize::try_from(position.x)
                .ok()
                .and_then(|x| row.as_bytes().get(x))
        })?;
    wilderness
        .legend
        .iter()
        .find(|entry| entry.symbol.as_bytes() == [*symbol])
}

fn wilderness_has_town(wilderness: &WildernessDefinition, position: Position) -> bool {
    wilderness.locations.iter().any(|location| {
        matches!(
            location,
            WildernessLocationDefinition::Town {
                position: candidate,
                ..
            } if position_from_content(*candidate) == position
        )
    })
}

fn wilderness_has_location(wilderness: &WildernessDefinition, position: Position) -> bool {
    wilderness.locations.iter().any(|location| match location {
        WildernessLocationDefinition::Town {
            position: candidate,
            ..
        }
        | WildernessLocationDefinition::Dungeon {
            position: candidate,
            ..
        } => position_from_content(*candidate) == position,
    })
}

pub(super) const fn wilderness_is_daytime_at(world_tick: u32) -> bool {
    world_tick % WILDERNESS_DAY_TICKS < WILDERNESS_DAY_TICKS / 2
}

pub(super) fn wilderness_ambush_denominator(
    player_level: u16,
    danger_level: u16,
    road: bool,
    daytime: bool,
) -> u64 {
    let mut denominator = 125_i64
        .saturating_add(i64::from(player_level).saturating_mul(10))
        .saturating_sub(i64::from(danger_level))
        .max(1);
    if road {
        denominator = denominator.saturating_mul(8);
    }
    if !daytime {
        denominator /= 2;
    }
    u64::try_from(denominator.max(1)).expect("positive ambush denominator must fit u64")
}

fn terrain_id_for_wilderness(terrain: WildernessTerrain) -> &'static str {
    match terrain {
        WildernessTerrain::Edge | WildernessTerrain::Mountain => SURFACE_MOUNTAIN_ID,
        WildernessTerrain::Town => SURFACE_PATH_ID,
        WildernessTerrain::DeepWater => SURFACE_WATER_DEEP_ID,
        WildernessTerrain::ShallowWater => SURFACE_WATER_SHALLOW_ID,
        WildernessTerrain::Swamp => SURFACE_SWAMP_ID,
        WildernessTerrain::Dirt | WildernessTerrain::Desert => SURFACE_WASTE_ID,
        WildernessTerrain::Grass => SURFACE_GRASS_ID,
        WildernessTerrain::Trees => SURFACE_WOODLAND_ID,
        WildernessTerrain::ShallowLava => SURFACE_LAVA_SHALLOW_ID,
        WildernessTerrain::DeepLava => SURFACE_LAVA_DEEP_ID,
        WildernessTerrain::Glacier => SURFACE_GLACIER_ID,
        WildernessTerrain::Snow => SURFACE_SNOW_ID,
        WildernessTerrain::PackIce => SURFACE_PACK_ICE_ID,
    }
}

fn coordinate_seed(seed: u64, position: Position) -> u64 {
    let mut value = seed
        ^ u64::try_from(position.x)
            .unwrap_or_default()
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ u64::try_from(position.y)
            .unwrap_or_default()
            .wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn cell_noise(seed: u64, x: i32, y: i32, salt: u64) -> u64 {
    coordinate_seed(
        seed ^ salt,
        Position {
            x: x.max(0),
            y: y.max(0),
        },
    )
}

fn varied_terrain_id(terrain: WildernessTerrain, noise: u64) -> &'static str {
    match terrain {
        WildernessTerrain::Trees if noise.is_multiple_of(5) => SURFACE_TREE_ID,
        WildernessTerrain::Glacier if noise.is_multiple_of(5) => SURFACE_PACK_ICE_ID,
        WildernessTerrain::Snow if noise.is_multiple_of(7) => SURFACE_GLACIER_ID,
        WildernessTerrain::Mountain if noise.is_multiple_of(6) => SURFACE_WASTE_ID,
        _ => terrain_id_for_wilderness(terrain),
    }
}

fn neighbor_position(position: Position, dx: i32, dy: i32) -> Position {
    Position {
        x: position.x + dx,
        y: position.y + dy,
    }
}

fn blended_wilderness_terrain(
    wilderness: &WildernessDefinition,
    world_position: Position,
    local_position: Position,
    width: u16,
    height: u16,
    seed: u64,
) -> WildernessTerrain {
    let current = wilderness_legend_at(wilderness, world_position)
        .expect("validated wilderness position must remain defined")
        .terrain;
    let candidates = [
        (local_position.y, 0, -1, local_position.x),
        (
            i32::from(height) - 1 - local_position.y,
            0,
            1,
            local_position.x,
        ),
        (local_position.x, -1, 0, local_position.y),
        (
            i32::from(width) - 1 - local_position.x,
            1,
            0,
            local_position.y,
        ),
    ];
    let mut selected = None;
    for (distance, dx, dy, along) in candidates {
        let thickness = 3 + i32::try_from(cell_noise(seed, along, dx + dy, 0xB0A4_D3A5) % 6)
            .expect("border thickness must fit i32");
        if distance >= thickness.min(WILDERNESS_BORDER_BLEND) {
            continue;
        }
        let neighbor = wilderness_legend_at(wilderness, neighbor_position(world_position, dx, dy))
            .map_or(WildernessTerrain::Edge, |entry| entry.terrain);
        let noise = cell_noise(seed, dx, dy, 0xED6E);
        if selected.is_none_or(|(best_distance, best_noise, _)| {
            distance < best_distance || (distance == best_distance && noise < best_noise)
        }) {
            selected = Some((distance, noise, neighbor));
        }
    }
    selected.map_or(current, |(_, _, terrain)| terrain)
}

fn road_reaches(wilderness: &WildernessDefinition, position: Position, dx: i32, dy: i32) -> bool {
    wilderness_legend_at(wilderness, neighbor_position(position, dx, dy))
        .is_some_and(|entry| entry.road)
}

fn is_road_cell(
    wilderness: &WildernessDefinition,
    world_position: Position,
    local_position: Position,
    width: u16,
    height: u16,
) -> bool {
    let Some(current) = wilderness_legend_at(wilderness, world_position) else {
        return false;
    };
    if !current.road {
        return false;
    }
    let center_x = i32::from(width) / 2;
    let center_y = i32::from(height) / 2;
    let on_vertical = (local_position.x - center_x).abs() <= 1;
    let on_horizontal = (local_position.y - center_y).abs() <= 1;
    (on_vertical
        && ((local_position.y <= center_y && road_reaches(wilderness, world_position, 0, -1))
            || (local_position.y >= center_y && road_reaches(wilderness, world_position, 0, 1))))
        || (on_horizontal
            && ((local_position.x <= center_x && road_reaches(wilderness, world_position, -1, 0))
                || (local_position.x >= center_x
                    && road_reaches(wilderness, world_position, 1, 0))))
        || ((local_position.x - center_x).abs() <= 1 && (local_position.y - center_y).abs() <= 1)
}

fn arrival_position(direction: Direction, previous: Position, width: u16, height: u16) -> Position {
    let (dx, dy) = direction.delta();
    Position {
        x: if dx < 0 {
            i32::from(width) - 2
        } else if dx > 0 {
            1
        } else {
            previous.x.clamp(1, i32::from(width) - 2)
        },
        y: if dy < 0 {
            i32::from(height) - 2
        } else if dy > 0 {
            1
        } else {
            previous.y.clamp(1, i32::from(height) - 2)
        },
    }
}

impl Game {
    pub(super) fn is_wilderness_floor(&self) -> bool {
        self.current_floor_id == WILDERNESS_FLOOR_ID
    }

    pub(super) fn wilderness_position_is_town(&self, position: Position) -> bool {
        self.content
            .world(&self.world_id)
            .and_then(|world| world.wilderness.as_ref())
            .is_some_and(|wilderness| wilderness_has_town(wilderness, position))
    }

    pub(super) const fn wilderness_is_daytime(&self) -> bool {
        wilderness_is_daytime_at(self.world_tick)
    }

    pub(super) fn wilderness_ambush_threat_remains(&self) -> bool {
        self.entities.iter().any(|actor| {
            actor.hp > 0
                && (actor.id.contains(WILDERNESS_AMBUSH_ID_MARKER)
                    || actor.summon.as_ref().is_some_and(|summon| {
                        summon.owner_id.contains(WILDERNESS_AMBUSH_ID_MARKER)
                    }))
                && !self.actor_is_player_side(actor)
        })
    }

    pub(super) fn player_has_following_pet(&self) -> bool {
        self.entities.iter().any(|actor| {
            actor.hp > 0
                && self.riding_actor_id.as_deref() != Some(actor.id.as_str())
                && (actor.controller_id.as_deref() == Some(self.player.id.as_str())
                    || actor
                        .summon
                        .as_ref()
                        .is_some_and(|summon| summon.owner_id == self.player.id))
        })
    }

    pub(super) fn recall_is_active(&self) -> bool {
        self.recall
            .as_ref()
            .is_some_and(|recall| recall.remaining_turns.is_some())
    }

    pub(super) fn wilderness_danger_level(&self, position: Position) -> u16 {
        let wilderness = self.wilderness();
        let current = wilderness_legend_at(wilderness, position)
            .expect("validated wilderness position must remain defined");
        if wilderness_has_location(wilderness, position) {
            return current.level.min(60);
        }
        let mut total = 0_u32;
        let mut count = 0_u32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(entry) =
                    wilderness_legend_at(wilderness, neighbor_position(position, dx, dy))
                    && entry.terrain != WildernessTerrain::Edge
                {
                    total = total.saturating_add(u32::from(entry.level));
                    count = count.saturating_add(1);
                }
            }
        }
        u16::try_from(total / count.max(1))
            .unwrap_or(u16::MAX)
            .min(60)
    }

    fn world_cell_terrain_id(&self, position: Position) -> Option<&'static str> {
        let wilderness = self.wilderness();
        let legend = wilderness_legend_at(wilderness, position)?;
        if legend.terrain == WildernessTerrain::Edge {
            return None;
        }
        Some(
            if legend.road || wilderness_has_town(wilderness, position) {
                SURFACE_PATH_ID
            } else {
                terrain_id_for_wilderness(legend.terrain)
            },
        )
    }

    fn active_traveler_definition(&self) -> &rfb_content::ActorDefinition {
        self.riding_actor_id
            .as_deref()
            .and_then(|mount_id| self.entities.iter().find(|actor| actor.id == mount_id))
            .and_then(|mount| self.content.actor(&mount.kind_id))
            .or_else(|| self.content.actor(&self.player.kind_id))
            .expect("active player or mount definition must remain available")
    }

    fn active_traveler_has_mode(&self, mode: ActorMovementMode) -> bool {
        self.active_traveler_definition()
            .movement
            .modes
            .contains(&mode)
    }

    fn player_can_cross_surface_terrain(&self, terrain: &rfb_content::TerrainDefinition) -> bool {
        if self.riding_actor_id.is_none() && terrain.id == SURFACE_WATER_DEEP_ID {
            return true;
        }
        movement::actor_can_cross_terrain(self.active_traveler_definition(), terrain)
    }

    fn player_can_enter_world_cell(&self, position: Position) -> bool {
        let Some(terrain) = self
            .world_cell_terrain_id(position)
            .and_then(|terrain_id| self.content.terrain(terrain_id))
        else {
            return false;
        };
        self.player_can_cross_surface_terrain(terrain)
    }

    pub(super) fn player_can_enter_local_wilderness(&self, position: Position) -> Option<bool> {
        self.is_wilderness_floor().then(|| {
            self.index(position)
                .and_then(|index| self.content.terrain(&self.terrain[index]))
                .is_some_and(|terrain| self.player_can_cross_surface_terrain(terrain))
        })
    }

    pub(super) fn next_world_travel_direction(&self, destination: Position) -> Option<Direction> {
        const DIRECTIONS: [Direction; 8] = [
            Direction::North,
            Direction::NorthEast,
            Direction::East,
            Direction::SouthEast,
            Direction::South,
            Direction::SouthWest,
            Direction::West,
            Direction::NorthWest,
        ];
        let start = self.wilderness_position?;
        if start == destination || !self.player_can_enter_world_cell(destination) {
            return None;
        }
        let mut visited = BTreeSet::from([start]);
        let mut queue = VecDeque::new();
        let ordered_neighbors = |position: Position| {
            let mut neighbors = DIRECTIONS
                .iter()
                .copied()
                .enumerate()
                .map(|(order, direction)| {
                    let (dx, dy) = direction.delta();
                    let next = neighbor_position(position, dx, dy);
                    (squared_distance(next, destination), order, direction, next)
                })
                .collect::<Vec<_>>();
            neighbors.sort_by_key(|(distance, order, _, _)| (*distance, *order));
            neighbors
        };
        for (_, _, direction, position) in ordered_neighbors(start) {
            if !self.player_can_enter_world_cell(position) || !visited.insert(position) {
                continue;
            }
            if position == destination {
                return Some(direction);
            }
            queue.push_back((position, direction));
        }
        while let Some((position, first_direction)) = queue.pop_front() {
            for (_, _, _, next) in ordered_neighbors(position) {
                if !self.player_can_enter_world_cell(next) || !visited.insert(next) {
                    continue;
                }
                if next == destination {
                    return Some(first_direction);
                }
                queue.push_back((next, first_direction));
            }
        }
        None
    }

    pub(super) fn move_on_world_map(
        &mut self,
        direction: Direction,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let current = self
            .wilderness_position
            .expect("world map requires a wilderness position");
        let (dx, dy) = direction.delta();
        let target = neighbor_position(current, dx, dy);
        if !self.player_can_enter_world_cell(target) {
            return false;
        }
        self.wilderness_position = Some(target);
        changed.insert(current);
        changed.insert(target);
        true
    }

    pub(super) fn roll_wilderness_ambush(&mut self) -> bool {
        let position = self
            .wilderness_position
            .expect("world map requires a wilderness position");
        let wilderness = self.wilderness();
        if wilderness_has_town(wilderness, position) {
            return false;
        }
        let danger_level = self.wilderness_danger_level(position);
        if danger_level.saturating_add(5) <= self.progress.level / 2 {
            return false;
        }
        let road = wilderness_legend_at(wilderness, position)
            .expect("validated wilderness position must remain defined")
            .road;
        let denominator = wilderness_ambush_denominator(
            self.progress.level,
            danger_level,
            road,
            self.wilderness_is_daytime(),
        );
        let threshold = 21_i32
            .saturating_sub(self.player_derived_stats().stealth_skill.value)
            .max(0);
        self.rng.bounded(denominator) < u64::try_from(threshold).unwrap_or_default()
    }

    pub(super) fn activate_wilderness_ambush(&mut self) -> Result<(), CoreError> {
        self.activate_wilderness_position(None, true)?;
        self.map_scale = MapScaleDto::Local;
        Ok(())
    }

    pub(super) fn move_across_wilderness_edge(
        &mut self,
        direction: Direction,
    ) -> Result<bool, CoreError> {
        if !self.is_wilderness_floor() {
            return Ok(false);
        }
        let current = self
            .wilderness_position
            .expect("local wilderness requires a wilderness position");
        let (dx, dy) = direction.delta();
        let target = neighbor_position(current, dx, dy);
        if !self.player_can_enter_world_cell(target) {
            return Ok(false);
        }
        let arrival = arrival_position(direction, self.player.position, self.width, self.height);
        self.wilderness_position = Some(target);
        self.activate_wilderness_position(Some(arrival), false)?;
        Ok(true)
    }

    pub(super) fn leave_world_map(&mut self) -> Result<bool, CoreError> {
        let interesting = self.wilderness_has_interesting_site();
        self.activate_wilderness_position(None, false)?;
        self.map_scale = MapScaleDto::Local;
        Ok(interesting)
    }

    fn activate_wilderness_position(
        &mut self,
        arrival: Option<Position>,
        ambush: bool,
    ) -> Result<(), CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let initial_floor_id = world.initial_floor_id.clone();
        let wilderness = world
            .wilderness
            .as_ref()
            .expect("wilderness position requires wilderness content");
        let position = self
            .wilderness_position
            .expect("wilderness position must remain available");
        let destination_is_town = wilderness_has_town(wilderness, position);

        if destination_is_town && self.current_floor_id == initial_floor_id {
            return Ok(());
        }

        let (active_floor, global_items, riding_actor) = self.take_active_wilderness_floor();
        if active_floor.id == initial_floor_id
            && self
                .stored_floors
                .insert(initial_floor_id.clone(), active_floor)
                .is_some()
        {
            return Err(CoreError::InvalidSave("surface floor state is duplicated"));
        }

        if destination_is_town {
            let town = self
                .stored_floors
                .remove(&initial_floor_id)
                .ok_or(CoreError::InvalidSave("surface floor state is missing"))?;
            self.activate_floor(town, global_items);
            self.restore_riding_actor(riding_actor);
            return Ok(());
        }

        let floor = self.generate_local_wilderness_floor(position, arrival);
        self.activate_floor(floor, global_items);
        self.restore_riding_actor(riding_actor);
        self.populate_local_wilderness(position, ambush);
        Ok(())
    }

    fn take_active_wilderness_floor(&mut self) -> (FloorState, Vec<ItemInstance>, Option<Actor>) {
        let riding_actor = self.riding_actor_id.as_deref().and_then(|mount_id| {
            self.entities
                .iter()
                .position(|actor| actor.id == mount_id)
                .map(|index| self.entities.remove(index))
        });
        let riding_actor_id = riding_actor.as_ref().map(|actor| actor.id.as_str());
        let (floor_items, global_items): (Vec<_>, Vec<_>) = std::mem::take(&mut self.items)
            .into_iter()
            .partition(|item| {
                matches!(item.location, ItemLocation::Ground(_))
                    || matches!(
                        &item.location,
                        ItemLocation::CarriedBy { actor_id }
                            if Some(actor_id.as_str()) != riding_actor_id
                    )
            });
        (
            FloorState {
                id: self.current_floor_id.clone(),
                dungeon_instance_id: self.current_dungeon_instance_id.clone(),
                width: self.width,
                height: self.height,
                terrain: std::mem::take(&mut self.terrain),
                glow: std::mem::take(&mut self.glow),
                player_position: self.player.position,
                entities: std::mem::take(&mut self.entities),
                items: floor_items,
                gold_piles: std::mem::take(&mut self.gold_piles),
                explored: std::mem::take(&mut self.explored),
                revealed_terrain: std::mem::take(&mut self.revealed_terrain),
                connections: std::mem::take(&mut self.floor_connections),
                regions: std::mem::take(&mut self.floor_regions),
            },
            global_items,
            riding_actor,
        )
    }

    fn restore_riding_actor(&mut self, riding_actor: Option<Actor>) {
        if let Some(mut riding_actor) = riding_actor {
            riding_actor.position = self.player.position;
            self.entities.push(riding_actor);
            self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        }
    }

    fn generate_local_wilderness_floor(
        &self,
        world_position: Position,
        arrival: Option<Position>,
    ) -> FloorState {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let wilderness = world
            .wilderness
            .as_ref()
            .expect("local wilderness requires wilderness content");
        let width = world.width;
        let height = world.height;
        let seed = coordinate_seed(self.wilderness_seed, world_position);
        let current = wilderness_legend_at(wilderness, world_position)
            .expect("validated wilderness position must remain defined");
        let player_position = arrival.unwrap_or(Position {
            x: i32::from(width) / 2,
            y: i32::from(height) / 2,
        });
        let mut terrain = Vec::with_capacity(usize::from(width) * usize::from(height));
        for y in 0..height {
            for x in 0..width {
                let local = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                let terrain_id = if is_road_cell(wilderness, world_position, local, width, height) {
                    SURFACE_PATH_ID
                } else {
                    let biome = blended_wilderness_terrain(
                        wilderness,
                        world_position,
                        local,
                        width,
                        height,
                        seed,
                    );
                    varied_terrain_id(biome, cell_noise(seed, local.x, local.y, 0xCE11))
                };
                terrain.push(terrain_id.to_owned());
            }
        }
        if self.wilderness_has_interesting_site() {
            paint_ruined_home(&mut terrain, width, height, player_position, seed);
        }
        let player_index = usize::try_from(player_position.y).expect("arrival y must fit usize")
            * usize::from(width)
            + usize::try_from(player_position.x).expect("arrival x must fit usize");
        terrain[player_index] = if current.road {
            SURFACE_PATH_ID
        } else {
            terrain_id_for_wilderness(current.terrain)
        }
        .to_owned();
        FloorState {
            id: WILDERNESS_FLOOR_ID.to_owned(),
            dungeon_instance_id: None,
            width,
            height,
            terrain,
            glow: vec![false; usize::from(width) * usize::from(height)],
            player_position,
            entities: Vec::new(),
            items: Vec::new(),
            gold_piles: Vec::new(),
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            connections: Vec::new(),
            regions: Vec::new(),
        }
    }

    pub(super) fn wilderness_has_interesting_site(&self) -> bool {
        let position = self
            .wilderness_position
            .expect("local wilderness requires a wilderness position");
        let wilderness = self.wilderness();
        let current = wilderness_legend_at(wilderness, position)
            .expect("validated wilderness position must remain defined");
        !current.road
            && !wilderness_has_location(wilderness, position)
            && matches!(
                current.terrain,
                WildernessTerrain::Grass | WildernessTerrain::Dirt | WildernessTerrain::Desert
            )
            && coordinate_seed(self.wilderness_seed ^ 0x01A7_EE57, position)
                .is_multiple_of(WILDERNESS_INTERESTING_CHANCE)
    }

    pub(super) fn resolve_wilderness_terrain_hazard(
        &mut self,
        position: Position,
    ) -> Vec<DomainEvent> {
        let terrain_id = if self.map_scale == MapScaleDto::World {
            self.wilderness_position
                .and_then(|world_position| self.world_cell_terrain_id(world_position))
        } else if self.is_wilderness_floor() {
            self.index(position)
                .and_then(|index| self.terrain.get(index))
                .map(String::as_str)
        } else {
            None
        };
        let Some(terrain_id) = terrain_id.map(str::to_owned) else {
            return Vec::new();
        };
        let flies = self.active_traveler_has_mode(ActorMovementMode::Fly);
        let swims = self.active_traveler_has_mode(ActorMovementMode::Aquatic);
        let raw_damage = match terrain_id.as_str() {
            SURFACE_WATER_DEEP_ID
                if !flies
                    && !swims
                    && self.carried_weight_tenths_pound()
                        > self.player_carry_capacity_tenths_pound() =>
            {
                self.roll_damage(1, self.progress.level.max(1))
            }
            SURFACE_LAVA_SHALLOW_ID if !flies => 30_i32.saturating_add(
                i32::try_from(self.rng.bounded(20)).expect("small lava roll must fit i32"),
            ),
            SURFACE_LAVA_DEEP_ID => {
                let damage = 60_i32.saturating_add(
                    i32::try_from(self.rng.bounded(40)).expect("small lava roll must fit i32"),
                );
                if flies { damage / 5 } else { damage }
            }
            SURFACE_SNOW_ID | SURFACE_GLACIER_ID | SURFACE_PACK_ICE_ID
                if self.effective_player_resistances().level(DamageType::Cold)
                    == ResistanceLevel::Normal
                    && self.rng.bounded(10) == 0 =>
            {
                1
            }
            _ => return Vec::new(),
        };
        let damage_type = match terrain_id.as_str() {
            SURFACE_LAVA_SHALLOW_ID | SURFACE_LAVA_DEEP_ID => DamageType::Fire,
            SURFACE_SNOW_ID | SURFACE_GLACIER_ID | SURFACE_PACK_ICE_ID => DamageType::Cold,
            _ => DamageType::Water,
        };
        let damage = self.reduce_player_damage(resolve_damage(
            DamagePacket::new(raw_damage, damage_type),
            self.effective_player_resistances().level(damage_type),
        ));
        let application = plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
        commit_damage_application(&mut self.player, &application);
        let mut events = vec![DomainEvent::WildernessTerrainDamaged {
            terrain_id: terrain_id.clone(),
            damage,
        }];
        if self.player_is_dead() {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: terrain_id,
                method_id: None,
                damage,
            });
        }
        events
    }

    pub(super) fn wilderness_blocks_regeneration(&self) -> bool {
        let terrain_id = if self.map_scale == MapScaleDto::World {
            self.wilderness_position
                .and_then(|position| self.world_cell_terrain_id(position))
        } else if self.is_wilderness_floor() {
            self.index(self.player.position)
                .and_then(|index| self.terrain.get(index))
                .map(String::as_str)
        } else {
            None
        };
        matches!(
            terrain_id,
            Some(SURFACE_SNOW_ID | SURFACE_GLACIER_ID | SURFACE_PACK_ICE_ID)
        ) && matches!(
            self.effective_player_resistances().level(DamageType::Cold),
            ResistanceLevel::Normal | ResistanceLevel::Vulnerable
        )
    }

    fn populate_local_wilderness(&mut self, position: Position, ambush: bool) {
        let level = self.wilderness_danger_level(position);
        let seed = coordinate_seed(self.wilderness_seed, position)
            ^ if ambush {
                WILDERNESS_AMBUSH_RNG_SALT
            } else {
                0
            };
        let local_rng = RfbRng::seeded(seed);
        let global_rng = std::mem::replace(&mut self.rng, local_rng);
        let division_remainders = std::mem::take(&mut self.monster_division_remainders);
        let spawn_kind = if ambush { "ambush" } else { "surface" };
        let rolls = if ambush {
            WILDERNESS_AMBUSH_ROLLS
        } else {
            self.content
                .world(&self.world_id)
                .and_then(|world| world.surface_actor_allocation)
                .map_or(0, |allocation| allocation.rolls)
        };
        self.initialize_wilderness_monsters(level, rolls, spawn_kind);
        self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        self.monster_division_remainders = division_remainders;
        self.rng = global_rng;
    }
}

fn paint_ruined_home(
    terrain: &mut [String],
    width: u16,
    height: u16,
    player_position: Position,
    seed: u64,
) {
    let room_width = i32::try_from(RUINED_HOME[0].len()).expect("room width must fit i32");
    let room_height = i32::try_from(RUINED_HOME.len()).expect("room height must fit i32");
    if i32::from(width) <= room_width + 4 || i32::from(height) <= room_height + 4 {
        return;
    }
    let origin_x = if player_position.x > i32::from(width) / 2 {
        2
    } else {
        i32::from(width) - room_width - 2
    };
    let y_span = i32::from(height) - room_height - 4;
    let origin_y =
        2 + i32::try_from(seed % u64::try_from(y_span + 1).expect("positive span must fit u64"))
            .expect("room offset must fit i32");
    for (dy, row) in RUINED_HOME.iter().enumerate() {
        for (dx, symbol) in row.bytes().enumerate() {
            let terrain_id = match symbol {
                b'#' | b'%' => SURFACE_ROCK_ID,
                b'T' => SURFACE_TREE_ID,
                b',' => SURFACE_GRASS_ID,
                b'.' | b'*' => SURFACE_WASTE_ID,
                b'+' | b'~' => SURFACE_PATH_ID,
                b' ' => continue,
                _ => continue,
            };
            let x = origin_x + i32::try_from(dx).expect("small room x must fit i32");
            let y = origin_y + i32::try_from(dy).expect("small room y must fit i32");
            let index = usize::try_from(y).expect("room y must fit usize") * usize::from(width)
                + usize::try_from(x).expect("room x must fit usize");
            terrain[index] = terrain_id.to_owned();
        }
    }
}

#[cfg(test)]
mod w3_tests {
    use super::*;

    #[test]
    fn wilderness_daylight_uses_original_half_day_boundaries() {
        assert!(wilderness_is_daytime_at(0));
        assert!(wilderness_is_daytime_at(49_999));
        assert!(!wilderness_is_daytime_at(50_000));
        assert!(!wilderness_is_daytime_at(99_999));
        assert!(wilderness_is_daytime_at(100_000));
    }

    #[test]
    fn wilderness_ambush_denominator_applies_road_and_night_modifiers() {
        assert_eq!(wilderness_ambush_denominator(1, 10, false, true), 125);
        assert_eq!(wilderness_ambush_denominator(1, 10, true, true), 1_000);
        assert_eq!(wilderness_ambush_denominator(1, 10, false, false), 62);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_seed_is_stable_and_coordinate_specific() {
        assert_eq!(
            coordinate_seed(42, Position { x: 28, y: 52 }),
            coordinate_seed(42, Position { x: 28, y: 52 })
        );
        assert_ne!(
            coordinate_seed(42, Position { x: 28, y: 52 }),
            coordinate_seed(42, Position { x: 29, y: 52 })
        );
    }

    #[test]
    fn world_movement_rejects_edges_but_allows_unmounted_deep_water_entry() {
        let game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
            .expect("Warrens journey should create");

        assert!(!game.player_can_enter_world_cell(Position { x: 0, y: 0 }));
        assert!(game.player_can_enter_world_cell(Position { x: 1, y: 1 }));
        assert!(game.player_can_enter_world_cell(Position { x: 29, y: 52 }));
    }

    #[test]
    fn world_travel_uses_eight_direction_pathfinding() {
        let game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
            .expect("Warrens journey should create");

        assert_eq!(
            game.next_world_travel_direction(Position { x: 29, y: 52 }),
            Some(Direction::East)
        );
        assert_eq!(
            game.next_world_travel_direction(Position { x: 28, y: 52 }),
            None
        );
        assert_eq!(
            game.next_world_travel_direction(Position { x: 0, y: 0 }),
            None
        );
    }

    #[test]
    fn low_level_interesting_sites_paint_the_authoritative_ruined_home() {
        let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
            .expect("Warrens journey should create");
        let site = (0..66)
            .flat_map(|y| (0..99).map(move |x| Position { x, y }))
            .find(|position| {
                game.wilderness_position = Some(*position);
                game.wilderness_has_interesting_site()
            })
            .expect("authoritative map should contain an eligible deterministic site");
        game.wilderness_position = Some(site);

        let floor = game.generate_local_wilderness_floor(site, None);

        assert!(
            floor
                .terrain
                .iter()
                .any(|terrain_id| terrain_id == SURFACE_ROCK_ID)
        );
    }
}

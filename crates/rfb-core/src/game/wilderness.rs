// SPDX-License-Identifier: MPL-2.0

use super::*;
use rfb_content::{
    ActorHabitat, ActorMovementMode, WILDERNESS_WORLD_CELL_HEIGHT, WILDERNESS_WORLD_CELL_WIDTH,
    WildernessDefinition, WildernessLegendEntry, WildernessLocationDefinition, WildernessTerrain,
};

pub(super) const WILDERNESS_FLOOR_ID: &str = "core.floor.wilderness";
pub(super) const WORLD_MAP_ACTION_MULTIPLIER: i32 = 132;
pub(super) const WILDERNESS_DAY_TICKS: u32 = 100_000;
pub(super) const WILDERNESS_CHUNK_WIDTH: u16 = 32;
pub(super) const WILDERNESS_CHUNK_HEIGHT: u16 = 11;
pub(super) const WILDERNESS_VIEW_WIDTH: u16 = WILDERNESS_WORLD_CELL_WIDTH;
pub(super) const WILDERNESS_VIEW_HEIGHT: u16 = WILDERNESS_WORLD_CELL_HEIGHT;

const WILDERNESS_BORDER_BLEND: i32 = 8;
const WILDERNESS_CACHE_RADIUS_CHUNKS: i32 = 2;
const WILDERNESS_AMBUSH_ROLLS: u16 = 20;
const WILDERNESS_AMBUSH_RNG_SALT: u64 = 0xA8B0_5A11;
const WILDERNESS_AMBUSH_ID_MARKER: &str = ".ambush.";
const WILDERNESS_SCROLL_RNG_SALT: u64 = 0x5C20_11ED;
const WILDERNESS_VIEW_CHUNK_COUNT: u64 = 9;
const WILDERNESS_INTERESTING_CHANCE: u64 = 10;
pub(super) const WILDERNESS_SEED_STEP: u64 = 0x9E37_79B9_7F4A_7C15;
// RFB wild.c makes four allocation attempts on roads and ten elsewhere.
const WILDERNESS_ROAD_MONSTER_ROLLS: u16 = 4;
const WILDERNESS_OFF_ROAD_MONSTER_ROLLS: u16 = 10;
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

pub(super) enum WildernessPlayerEntry {
    Blocked,
    Local {
        target: Position,
        crossed_world_cell: bool,
        translation: Option<Position>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleTown {
    town_id: String,
    floor_id: String,
    view_origin: Position,
    width: u16,
    height: u16,
}

impl VisibleTown {
    fn visible_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let start_x = (-self.view_origin.x).max(0);
        let start_y = (-self.view_origin.y).max(0);
        let end_x =
            i32::from(self.width).min(i32::from(WILDERNESS_VIEW_WIDTH) - self.view_origin.x);
        let end_y =
            i32::from(self.height).min(i32::from(WILDERNESS_VIEW_HEIGHT) - self.view_origin.y);
        (start_x < end_x && start_y < end_y).then_some((start_x, start_y, end_x, end_y))
    }

    fn view_to_local(&self, position: Position) -> Option<Position> {
        let local = Position {
            x: position.x - self.view_origin.x,
            y: position.y - self.view_origin.y,
        };
        (local.x >= 0
            && local.y >= 0
            && local.x < i32::from(self.width)
            && local.y < i32::from(self.height))
        .then_some(local)
    }

    fn local_to_view(&self, position: Position) -> Option<Position> {
        translate_wilderness_position(
            Position {
                x: self.view_origin.x + position.x,
                y: self.view_origin.y + position.y,
            },
            Position::default(),
        )
    }
}

fn merge_floor_region(regions: &mut Vec<FloorRegionState>, mut incoming: FloorRegionState) {
    if let Some(existing) = regions.iter_mut().find(|region| {
        region.region_id == incoming.region_id
            && region.theme_id == incoming.theme_id
            && region.encounter_table_id == incoming.encounter_table_id
            && region.loot_table_id == incoming.loot_table_id
    }) {
        existing.cells.append(&mut incoming.cells);
        existing.cells.sort();
        existing.cells.dedup();
    } else {
        incoming.cells.sort();
        regions.push(incoming);
        regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    }
}

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

fn wilderness_initial_monster_rolls(configured_rolls: u16, road: bool) -> u16 {
    configured_rolls.min(if road {
        WILDERNESS_ROAD_MONSTER_ROLLS
    } else {
        WILDERNESS_OFF_ROAD_MONSTER_ROLLS
    })
}

pub(super) fn snow_movement_action_cost(
    action_cost: i32,
    carried_weight: u32,
    carry_capacity: u32,
    mounted: bool,
) -> i32 {
    let percent = if mounted {
        40
    } else {
        let load_percent = u64::from(carried_weight)
            .saturating_mul(100)
            .saturating_div(u64::from(carry_capacity.max(1)))
            .min(200);
        33 + i32::try_from(load_percent.saturating_sub(100)).unwrap_or(100)
    };
    action_cost.saturating_add(
        action_cost
            .clamp(0, 120)
            .saturating_mul(percent)
            .saturating_div(100),
    )
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

fn wilderness_view_center_chunk(world_position: Position, view_offset: Position) -> Position {
    Position {
        x: world_position.x * 3 + view_offset.x,
        y: world_position.y * 3 + view_offset.y,
    }
}

fn wilderness_chunk_world_position(chunk: Position) -> Position {
    Position {
        x: (chunk.x + 1).div_euclid(3),
        y: (chunk.y + 1).div_euclid(3),
    }
}

fn wilderness_chunk_local_origin(chunk: Position) -> Position {
    Position {
        x: (chunk.x + 1).rem_euclid(3) * i32::from(WILDERNESS_CHUNK_WIDTH),
        y: (chunk.y + 1).rem_euclid(3) * i32::from(WILDERNESS_CHUNK_HEIGHT),
    }
}

fn varied_terrain_id(terrain: WildernessTerrain, noise: u64) -> &'static str {
    match terrain {
        WildernessTerrain::Grass if noise.is_multiple_of(7) => SURFACE_WOODLAND_ID,
        WildernessTerrain::Dirt | WildernessTerrain::Desert if noise.is_multiple_of(11) => {
            SURFACE_GRASS_ID
        }
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

fn wilderness_site_is_interesting(
    wilderness: &WildernessDefinition,
    wilderness_seed: u64,
    position: Position,
) -> bool {
    let Some(current) = wilderness_legend_at(wilderness, position) else {
        return false;
    };
    !current.road
        && !wilderness_has_location(wilderness, position)
        && matches!(
            current.terrain,
            WildernessTerrain::Grass | WildernessTerrain::Dirt | WildernessTerrain::Desert
        )
        && coordinate_seed(wilderness_seed ^ 0x01A7_EE57, position)
            .is_multiple_of(WILDERNESS_INTERESTING_CHANCE)
}

fn ruined_home_terrain_at(local_position: Position, seed: u64) -> Option<&'static str> {
    let room_width = i32::try_from(RUINED_HOME[0].len()).expect("room width must fit i32");
    let room_height = i32::try_from(RUINED_HOME.len()).expect("room height must fit i32");
    let origin_x = if seed.is_multiple_of(2) {
        2
    } else {
        i32::from(WILDERNESS_VIEW_WIDTH) - room_width - 2
    };
    let y_span = i32::from(WILDERNESS_VIEW_HEIGHT) - room_height - 4;
    let origin_y =
        2 + i32::try_from(seed % u64::try_from(y_span + 1).expect("positive span must fit u64"))
            .expect("room offset must fit i32");
    let dx = usize::try_from(local_position.x - origin_x).ok()?;
    let dy = usize::try_from(local_position.y - origin_y).ok()?;
    let symbol = *RUINED_HOME.get(dy)?.as_bytes().get(dx)?;
    match symbol {
        b'#' | b'%' => Some(SURFACE_ROCK_ID),
        b'T' => Some(SURFACE_TREE_ID),
        b',' => Some(SURFACE_GRASS_ID),
        b'.' | b'*' => Some(SURFACE_WASTE_ID),
        b'+' | b'~' => Some(SURFACE_PATH_ID),
        b' ' => None,
        _ => None,
    }
}

fn generate_wilderness_chunk(
    wilderness: &WildernessDefinition,
    wilderness_seed: u64,
    chunk: Position,
) -> Vec<String> {
    let world_position = wilderness_chunk_world_position(chunk);
    let local_origin = wilderness_chunk_local_origin(chunk);
    let seed = coordinate_seed(wilderness_seed, world_position);
    let interesting = wilderness_site_is_interesting(wilderness, wilderness_seed, world_position);
    let mut terrain = Vec::with_capacity(
        usize::from(WILDERNESS_CHUNK_WIDTH) * usize::from(WILDERNESS_CHUNK_HEIGHT),
    );
    for y in 0..WILDERNESS_CHUNK_HEIGHT {
        for x in 0..WILDERNESS_CHUNK_WIDTH {
            let local_position = Position {
                x: local_origin.x + i32::from(x),
                y: local_origin.y + i32::from(y),
            };
            let terrain_id = if interesting
                && let Some(terrain_id) = ruined_home_terrain_at(local_position, seed)
            {
                terrain_id
            } else if is_road_cell(
                wilderness,
                world_position,
                local_position,
                WILDERNESS_VIEW_WIDTH,
                WILDERNESS_VIEW_HEIGHT,
            ) {
                SURFACE_PATH_ID
            } else {
                let biome = blended_wilderness_terrain(
                    wilderness,
                    world_position,
                    local_position,
                    WILDERNESS_VIEW_WIDTH,
                    WILDERNESS_VIEW_HEIGHT,
                    seed,
                );
                varied_terrain_id(
                    biome,
                    cell_noise(seed, local_position.x, local_position.y, 0xCE11),
                )
            };
            terrain.push(terrain_id.to_owned());
        }
    }
    terrain
}

fn wilderness_scroll_delta(position: Position) -> Position {
    Position {
        x: if position.x < i32::from(WILDERNESS_CHUNK_WIDTH) {
            -1
        } else if position.x >= i32::from(WILDERNESS_VIEW_WIDTH - WILDERNESS_CHUNK_WIDTH) {
            1
        } else {
            0
        },
        y: if position.y < i32::from(WILDERNESS_CHUNK_HEIGHT) {
            -1
        } else if position.y >= i32::from(WILDERNESS_VIEW_HEIGHT - WILDERNESS_CHUNK_HEIGHT) {
            1
        } else {
            0
        },
    }
}

fn normalize_wilderness_view(
    mut world_position: Position,
    mut view_offset: Position,
) -> (Position, Position) {
    if view_offset.x < -1 {
        world_position.x -= 1;
        view_offset.x = 1;
    } else if view_offset.x > 1 {
        world_position.x += 1;
        view_offset.x = -1;
    }
    if view_offset.y < -1 {
        world_position.y -= 1;
        view_offset.y = 1;
    } else if view_offset.y > 1 {
        world_position.y += 1;
        view_offset.y = -1;
    }
    (world_position, view_offset)
}

fn translate_wilderness_position(position: Position, translation: Position) -> Option<Position> {
    let translated = Position {
        x: position.x + translation.x,
        y: position.y + translation.y,
    };
    (translated.x >= 0
        && translated.x < i32::from(WILDERNESS_VIEW_WIDTH)
        && translated.y >= 0
        && translated.y < i32::from(WILDERNESS_VIEW_HEIGHT))
    .then_some(translated)
}

fn wilderness_exposed_chunks(center: Position, scroll: Position) -> BTreeSet<Position> {
    let mut chunks = BTreeSet::new();
    if scroll.x != 0 {
        for dy in -1..=1 {
            chunks.insert(Position {
                x: center.x + scroll.x,
                y: center.y + dy,
            });
        }
    }
    if scroll.y != 0 {
        for dx in -1..=1 {
            chunks.insert(Position {
                x: center.x + dx,
                y: center.y + scroll.y,
            });
        }
    }
    chunks
}

fn wilderness_exposed_positions(scroll: Position) -> BTreeSet<Position> {
    (0..i32::from(WILDERNESS_VIEW_HEIGHT))
        .flat_map(|y| {
            (0..i32::from(WILDERNESS_VIEW_WIDTH)).filter_map(move |x| {
                ((scroll.x < 0 && x < i32::from(WILDERNESS_CHUNK_WIDTH))
                    || (scroll.x > 0
                        && x >= i32::from(WILDERNESS_VIEW_WIDTH - WILDERNESS_CHUNK_WIDTH))
                    || (scroll.y < 0 && y < i32::from(WILDERNESS_CHUNK_HEIGHT))
                    || (scroll.y > 0
                        && y >= i32::from(WILDERNESS_VIEW_HEIGHT - WILDERNESS_CHUNK_HEIGHT)))
                .then_some(Position { x, y })
            })
        })
        .collect()
}

fn wilderness_view_positions() -> BTreeSet<Position> {
    (0..i32::from(WILDERNESS_VIEW_HEIGHT))
        .flat_map(|y| (0..i32::from(WILDERNESS_VIEW_WIDTH)).map(move |x| Position { x, y }))
        .collect()
}

fn wilderness_chunk_set_seed(seed: u64, chunks: &BTreeSet<Position>) -> u64 {
    chunks.iter().fold(seed, |value, position| {
        coordinate_seed(value.rotate_left(17), *position)
    })
}

fn wilderness_scroll_monster_rolls(initial_rolls: u16, exposed_chunks: &BTreeSet<Position>) -> u16 {
    let scaled = u64::from(initial_rolls)
        * u64::try_from(exposed_chunks.len()).expect("visible chunk count must fit u64");
    let whole = scaled / WILDERNESS_VIEW_CHUNK_COUNT;
    let remainder = scaled % WILDERNESS_VIEW_CHUNK_COUNT;
    let rounded = remainder > 0
        && wilderness_chunk_set_seed(WILDERNESS_SCROLL_RNG_SALT, exposed_chunks)
            % WILDERNESS_VIEW_CHUNK_COUNT
            < remainder;
    u16::try_from(whole + u64::from(rounded)).expect("scaled wilderness rolls must fit u16")
}

fn wilderness_monster_rolls_for_allowed_area(
    rolls: u16,
    allowed_cells: usize,
    considered_cells: usize,
    seed: u64,
) -> u16 {
    if rolls == 0 || allowed_cells == 0 || considered_cells == 0 {
        return 0;
    }
    if allowed_cells == considered_cells {
        return rolls;
    }

    let denominator = u64::try_from(considered_cells).expect("wilderness area must fit u64");
    let scaled = u64::from(rolls)
        * u64::try_from(allowed_cells).expect("allowed wilderness area must fit u64");
    let whole = scaled / denominator;
    let remainder = scaled % denominator;
    let rounded = remainder > 0 && seed % denominator < remainder;
    u16::try_from(whole + u64::from(rounded)).expect("scaled wilderness rolls must fit u16")
}

impl Game {
    pub(super) fn wilderness_terrain_at_view_position(
        &self,
        position: Position,
    ) -> Option<WildernessTerrain> {
        let center =
            wilderness_view_center_chunk(self.wilderness_position?, self.wilderness_view_offset);
        let chunk = Position {
            x: center.x + position.x.div_euclid(i32::from(WILDERNESS_CHUNK_WIDTH)) - 1,
            y: center.y + position.y.div_euclid(i32::from(WILDERNESS_CHUNK_HEIGHT)) - 1,
        };
        wilderness_legend_at(self.wilderness(), wilderness_chunk_world_position(chunk))
            .map(|entry| entry.terrain)
    }

    pub(super) fn is_wilderness_floor(&self) -> bool {
        self.current_floor_id == WILDERNESS_FLOOR_ID
    }

    pub(super) fn advance_wilderness_generation(&mut self) {
        self.wilderness_seed = self.wilderness_seed.wrapping_add(WILDERNESS_SEED_STEP);
        self.wilderness_terrain_cache.clear();
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

    pub(super) fn world_cell_terrain_id(&self, position: Position) -> Option<&'static str> {
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

    pub(super) fn active_traveler_has_mode(&self, mode: ActorMovementMode) -> bool {
        if self.riding_actor_id.is_none()
            && mode == ActorMovementMode::Fly
            && self.player_levitates()
        {
            return true;
        }
        self.active_traveler_definition()
            .movement
            .modes
            .contains(&mode)
    }

    pub(super) fn player_snow_movement_action_cost(&self, action_cost: i32) -> i32 {
        let terrain_id = if self.map_scale == MapScaleDto::World {
            self.wilderness_position
                .and_then(|position| self.world_cell_terrain_id(position))
        } else {
            self.index(self.player.position)
                .and_then(|index| self.terrain.get(index))
                .map(String::as_str)
        };
        let on_snow = terrain_id
            .and_then(|terrain_id| self.content.terrain(terrain_id))
            .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "snow"));
        let snow_adapted_race = self
            .character_definitions()
            .is_some_and(|(_, race, _, _)| race.tags.iter().any(|tag| tag == "snow-adapted"));
        let snow_adapted_mount = self
            .riding_actor_id
            .as_deref()
            .and_then(|mount_id| self.entities.iter().find(|actor| actor.id == mount_id))
            .and_then(|mount| self.content.actor(&mount.kind_id))
            .and_then(|mount| mount.allocation.as_ref())
            .is_some_and(|allocation| allocation.habitats.contains(&ActorHabitat::Snow));
        if !on_snow
            || snow_adapted_race
            || snow_adapted_mount
            || self.player_can_pass_walls()
            || self.active_traveler_has_mode(ActorMovementMode::Fly)
            || self.active_traveler_has_mode(ActorMovementMode::PassWall)
        {
            return action_cost;
        }
        snow_movement_action_cost(
            action_cost,
            self.carried_weight_tenths_pound(),
            self.player_carry_capacity_tenths_pound(),
            self.riding_actor_id.is_some(),
        )
    }

    pub(super) fn player_can_cross_surface_terrain(
        &self,
        terrain: &rfb_content::TerrainDefinition,
    ) -> bool {
        if self.riding_actor_id.is_none()
            && self.player_levitates()
            && terrain.movement_modes.contains(&ActorMovementMode::Fly)
        {
            return true;
        }
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
        if self.map_scale != MapScaleDto::World {
            return false;
        }
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
        if self.map_scale != MapScaleDto::World {
            return Err(CoreError::WorldMapTransitionUnavailable);
        }
        self.activate_wilderness_position(None, true)?;
        self.map_scale = MapScaleDto::Local;
        Ok(())
    }

    pub(super) fn scroll_wilderness_for_player_entry(
        &mut self,
        target: Position,
        removed_entities: &mut Vec<String>,
    ) -> Result<WildernessPlayerEntry, CoreError> {
        if !self.is_wilderness_floor() {
            return Ok(WildernessPlayerEntry::Local {
                target,
                crossed_world_cell: false,
                translation: None,
            });
        }
        let scroll = wilderness_scroll_delta(target);
        if scroll == Position::default() {
            return Ok(WildernessPlayerEntry::Local {
                target,
                crossed_world_cell: false,
                translation: None,
            });
        }

        let current_world = self
            .wilderness_position
            .expect("local wilderness requires a wilderness position");
        let requested_offset = Position {
            x: self.wilderness_view_offset.x + scroll.x,
            y: self.wilderness_view_offset.y + scroll.y,
        };
        let (next_world, next_offset) = normalize_wilderness_view(current_world, requested_offset);
        let crossed_world_cell = next_world != current_world;
        if crossed_world_cell && !self.player_can_enter_world_cell(next_world) {
            return Ok(WildernessPlayerEntry::Blocked);
        }

        let stored_town_actor_ids = self.store_visible_town_states();
        self.wilderness_position = Some(next_world);
        self.wilderness_view_offset = next_offset;

        let translation = Position {
            x: -scroll.x * i32::from(WILDERNESS_CHUNK_WIDTH),
            y: -scroll.y * i32::from(WILDERNESS_CHUNK_HEIGHT),
        };
        let translated_target = translate_wilderness_position(target, translation)
            .expect("wilderness scroll target must enter the retained center region");
        let translated_player = translate_wilderness_position(self.player.position, translation)
            .expect("wilderness scroll must retain the player");

        let old_terrain = std::mem::take(&mut self.terrain);
        let old_glow = std::mem::take(&mut self.glow);
        let old_explored = std::mem::take(&mut self.explored);
        let mut terrain = self.cached_wilderness_view_terrain(next_world);
        let mut glow = vec![false; terrain.len()];
        let mut explored = vec![false; terrain.len()];
        let width = usize::from(WILDERNESS_VIEW_WIDTH);
        for y in 0..i32::from(WILDERNESS_VIEW_HEIGHT) {
            for x in 0..i32::from(WILDERNESS_VIEW_WIDTH) {
                let source = Position { x, y };
                let Some(destination) = translate_wilderness_position(source, translation) else {
                    continue;
                };
                let source_index = usize::try_from(y).expect("wilderness y must fit usize") * width
                    + usize::try_from(x).expect("wilderness x must fit usize");
                let destination_index = usize::try_from(destination.y)
                    .expect("translated wilderness y must fit usize")
                    * width
                    + usize::try_from(destination.x)
                        .expect("translated wilderness x must fit usize");
                terrain[destination_index] = old_terrain[source_index].clone();
                glow[destination_index] = old_glow[source_index];
                explored[destination_index] = old_explored[source_index];
            }
        }
        self.terrain = terrain;
        self.glow = glow;
        self.explored = explored;
        self.revealed_terrain = std::mem::take(&mut self.revealed_terrain)
            .into_iter()
            .filter_map(|position| translate_wilderness_position(position, translation))
            .collect();

        self.player.position = translated_player;
        for entity in &mut self.entities {
            entity.position = Position {
                x: entity.position.x + translation.x,
                y: entity.position.y + translation.y,
            };
        }
        let leaving_pack_ids = self
            .entities
            .iter()
            .filter(|entity| {
                translate_wilderness_position(entity.position, Position::default()).is_none()
            })
            .filter_map(|entity| entity.pack.as_ref().map(|pack| pack.id.clone()))
            .collect::<BTreeSet<_>>();
        let leaving_entity_ids = self
            .entities
            .iter()
            .filter(|entity| {
                translate_wilderness_position(entity.position, Position::default()).is_none()
                    || entity
                        .pack
                        .as_ref()
                        .is_some_and(|pack| leaving_pack_ids.contains(&pack.id))
            })
            .map(|entity| entity.id.clone())
            .collect::<BTreeSet<_>>();
        self.entities
            .retain(|entity| !leaving_entity_ids.contains(&entity.id));
        self.monster_division_remainders
            .retain(|entity_id, _| !leaving_entity_ids.contains(entity_id));
        removed_entities.extend(leaving_entity_ids.iter().cloned());

        let mut removed_item_ids = BTreeSet::new();
        self.items.retain_mut(|item| match &mut item.location {
            ItemLocation::Ground(position) => {
                let Some(translated) = translate_wilderness_position(*position, translation) else {
                    removed_item_ids.insert(item.id.clone());
                    return false;
                };
                *position = translated;
                true
            }
            ItemLocation::CarriedBy { actor_id } if leaving_entity_ids.contains(actor_id) => {
                removed_item_ids.insert(item.id.clone());
                false
            }
            _ => true,
        });
        for item_id in removed_item_ids {
            self.item_property_knowledge.remove(&item_id);
        }
        self.gold_piles.retain_mut(|pile| {
            let Some(position) = translate_wilderness_position(pile.position, translation) else {
                return false;
            };
            pile.position = position;
            true
        });

        let loaded_town_actor_ids = self.load_visible_town_states()?;
        removed_entities.extend(
            stored_town_actor_ids
                .difference(&loaded_town_actor_ids)
                .cloned(),
        );

        if self.summon_command.mode == SummonCommandModeDto::Guard {
            self.summon_command.guard_position = self
                .summon_command
                .guard_position
                .and_then(|position| translate_wilderness_position(position, translation))
                .or(Some(translated_player));
        }

        Ok(WildernessPlayerEntry::Local {
            target: translated_target,
            crossed_world_cell,
            translation: Some(translation),
        })
    }

    pub(super) fn leave_world_map(&mut self) -> Result<bool, CoreError> {
        let interesting = self.wilderness_has_interesting_site();
        self.activate_wilderness_position(None, false)?;
        self.map_scale = MapScaleDto::Local;
        self.mark_current_town_visited();
        Ok(interesting)
    }

    pub(super) fn activate_wilderness_position(
        &mut self,
        arrival: Option<Position>,
        ambush: bool,
    ) -> Result<(), CoreError> {
        let position = self
            .wilderness_position
            .expect("wilderness position must remain available");
        let destination_town = self.town_at_wilderness_position(position).cloned();

        let (active_floor, global_items, riding_actor) = self.take_active_wilderness_floor();
        if self.town_for_floor(&active_floor.id).is_some()
            && self
                .stored_floors
                .insert(active_floor.id.clone(), active_floor)
                .is_some()
        {
            return Err(CoreError::InvalidSave("town floor state is duplicated"));
        }

        if let Some(town) = &destination_town {
            let visible = self
                .visible_towns(position)
                .into_iter()
                .find(|visible| visible.floor_id == town.floor_id)
                .expect("destination town must intersect its own world cell");
            self.ensure_town_floor_is_stored(&visible)?;
        }

        let town_arrival = destination_town.as_ref().and_then(|town| {
            let visible = self
                .visible_towns(position)
                .into_iter()
                .find(|visible| visible.floor_id == town.floor_id)?;
            self.stored_floors
                .get(&town.floor_id)
                .and_then(|floor| visible.local_to_view(floor.player_position))
        });
        let floor = self.generate_local_wilderness_floor(position, arrival.or(town_arrival));
        self.activate_floor(floor, global_items);
        self.restore_riding_actor(riding_actor);
        self.load_visible_town_states()?;
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
                reproduction_suppressed: self.reproduction_suppressed,
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

    fn town_template_dimensions(&self, town_id: &str) -> (u16, u16) {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let town = self
            .content
            .town(town_id)
            .expect("validated wilderness town must remain available");
        if town.floor_id == world.initial_floor_id {
            return (world.width, world.height);
        }
        let floor = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == town.floor_id && floor.lifecycle == FloorLifecycle::Town)
            .expect("validated town floor must remain available");
        (floor.width, floor.height)
    }

    fn visible_towns(&self, world_position: Position) -> Vec<VisibleTown> {
        self.wilderness()
            .locations
            .iter()
            .filter_map(|location| {
                let WildernessLocationDefinition::Town {
                    position,
                    map_origin,
                    town_id,
                } = location
                else {
                    return None;
                };
                let town = self
                    .content
                    .town(town_id)
                    .expect("validated wilderness town must remain available");
                let town_world_position = position_from_content(*position);
                let (width, height) = self.town_template_dimensions(town_id);
                let visible = VisibleTown {
                    town_id: town_id.clone(),
                    floor_id: town.floor_id.clone(),
                    view_origin: Position {
                        x: (town_world_position.x - world_position.x)
                            * i32::from(WILDERNESS_VIEW_WIDTH)
                            + i32::from(map_origin.x)
                            - self.wilderness_view_offset.x * i32::from(WILDERNESS_CHUNK_WIDTH),
                        y: (town_world_position.y - world_position.y)
                            * i32::from(WILDERNESS_VIEW_HEIGHT)
                            + i32::from(map_origin.y)
                            - self.wilderness_view_offset.y * i32::from(WILDERNESS_CHUNK_HEIGHT),
                    },
                    width,
                    height,
                };
                visible.visible_bounds().is_some().then_some(visible)
            })
            .collect()
    }

    fn wilderness_positions_outside_visible_towns(
        &self,
        positions: BTreeSet<Position>,
    ) -> BTreeSet<Position> {
        let Some(world_position) = self.wilderness_position else {
            return positions;
        };
        let towns = self.visible_towns(world_position);
        positions
            .into_iter()
            .filter(|position| {
                towns
                    .iter()
                    .all(|town| town.view_to_local(*position).is_none())
            })
            .collect()
    }

    pub(super) fn town_at_wilderness_view_position(
        &self,
        position: Position,
    ) -> Option<&rfb_content::TownDefinition> {
        let world_position = self.wilderness_position?;
        let town_id = self
            .visible_towns(world_position)
            .into_iter()
            .find(|town| town.view_to_local(position).is_some())?
            .town_id;
        self.content.town(&town_id)
    }

    pub(super) fn town_local_to_wilderness_view_position(
        &self,
        town_id: &str,
        position: Position,
    ) -> Option<Position> {
        let world_position = self.wilderness_position?;
        let town = self
            .visible_towns(world_position)
            .into_iter()
            .find(|town| town.town_id == town_id)?;
        Some(Position {
            x: town.view_origin.x + position.x,
            y: town.view_origin.y + position.y,
        })
    }

    fn ensure_town_floor_is_stored(&mut self, town: &VisibleTown) -> Result<(), CoreError> {
        if self.stored_floors.contains_key(&town.floor_id) {
            return Ok(());
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        if town.floor_id == world.initial_floor_id {
            return Err(CoreError::InvalidSave("birth town floor state is missing"));
        }
        let definition = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == town.floor_id && floor.lifecycle == FloorLifecycle::Town)
            .expect("validated town floor must remain available")
            .clone();
        let division_remainders = std::mem::take(&mut self.monster_division_remainders);
        let generated = self.generate_procedural_floor(&definition, None);
        self.monster_division_remainders = division_remainders;
        let floor = generated?;
        self.stored_floors.insert(town.floor_id.clone(), floor);
        Ok(())
    }

    pub(super) fn initialize_continuous_wilderness_surface(&mut self) -> Result<(), CoreError> {
        let Some(world_position) = self.wilderness_position else {
            return Ok(());
        };
        let Some(town) = self.town_for_floor(&self.current_floor_id).cloned() else {
            return Ok(());
        };
        let player_position = self
            .town_local_to_wilderness_view_position(&town.id, self.player.position)
            .ok_or(CoreError::InvalidSave(
                "embedded town position is unavailable",
            ))?;
        let (town_floor, global_items, riding_actor) = self.take_active_wilderness_floor();
        if self
            .stored_floors
            .insert(town_floor.id.clone(), town_floor)
            .is_some()
        {
            return Err(CoreError::InvalidSave("town floor state is duplicated"));
        }
        let wilderness =
            self.generate_local_wilderness_floor(world_position, Some(player_position));
        self.activate_floor(wilderness, global_items);
        self.restore_riding_actor(riding_actor);
        self.load_visible_town_states()?;
        Ok(())
    }

    pub(super) fn activate_embedded_town_floor(&mut self) -> Result<(), CoreError> {
        let town = self
            .current_town()
            .filter(|_| self.is_wilderness_floor())
            .cloned()
            .ok_or(CoreError::InvalidSave("embedded town state is unavailable"))?;
        self.store_visible_town_states();
        let town_floor =
            self.stored_floors
                .remove(&town.floor_id)
                .ok_or(CoreError::InvalidSave(
                    "embedded town floor state is missing",
                ))?;
        let (_, global_items, riding_actor) = self.take_active_wilderness_floor();
        self.activate_floor(town_floor, global_items);
        self.restore_riding_actor(riding_actor);
        Ok(())
    }

    pub(super) fn store_visible_town_states(&mut self) -> BTreeSet<String> {
        let Some(world_position) = self
            .wilderness_position
            .filter(|_| self.is_wilderness_floor())
        else {
            return BTreeSet::new();
        };
        let towns = self.visible_towns(world_position);
        let mut stored_actor_ids = BTreeSet::new();
        for town in towns {
            let Some(mut floor) = self.stored_floors.remove(&town.floor_id) else {
                continue;
            };
            let (start_x, start_y, end_x, end_y) = town
                .visible_bounds()
                .expect("visible town must retain an intersection");
            for local_y in start_y..end_y {
                for local_x in start_x..end_x {
                    let local = Position {
                        x: local_x,
                        y: local_y,
                    };
                    let view = town
                        .local_to_view(local)
                        .expect("visible town cell must map into the view");
                    let local_index = usize::try_from(local_y).expect("town y must fit usize")
                        * usize::from(floor.width)
                        + usize::try_from(local_x).expect("town x must fit usize");
                    let view_index = usize::try_from(view.y).expect("view y must fit usize")
                        * usize::from(WILDERNESS_VIEW_WIDTH)
                        + usize::try_from(view.x).expect("view x must fit usize");
                    floor.terrain[local_index] = self.terrain[view_index].clone();
                    floor.glow[local_index] = self.glow[view_index];
                    floor.explored[local_index] = self.explored[view_index];
                }
            }
            if let Some(player_position) = town.view_to_local(self.player.position) {
                floor.player_position = player_position;
            }

            let mut remaining_revealed = BTreeSet::new();
            for position in std::mem::take(&mut self.revealed_terrain) {
                if let Some(local) = town.view_to_local(position) {
                    floor.revealed_terrain.insert(local);
                } else {
                    remaining_revealed.insert(position);
                }
            }
            self.revealed_terrain = remaining_revealed;

            let riding_actor_id = self.riding_actor_id.clone();
            let player_id = self.player.id.clone();
            let follows_player = |actor: &Actor| {
                riding_actor_id.as_deref() == Some(actor.id.as_str())
                    || actor.controller_id.as_deref() == Some(player_id.as_str())
                    || actor
                        .summon
                        .as_ref()
                        .is_some_and(|summon| summon.owner_id == player_id)
            };
            let pack_ids = self
                .entities
                .iter()
                .filter_map(|actor| actor.pack.as_ref().map(|pack| pack.id.clone()))
                .collect::<BTreeSet<_>>();
            let stored_pack_ids = pack_ids
                .into_iter()
                .filter(|pack_id| {
                    self.entities
                        .iter()
                        .filter(|actor| {
                            actor
                                .pack
                                .as_ref()
                                .is_some_and(|pack| pack.id.as_str() == pack_id.as_str())
                        })
                        .all(|actor| {
                            !follows_player(actor) && town.view_to_local(actor.position).is_some()
                        })
                })
                .collect::<BTreeSet<_>>();
            let mut active_entities = Vec::with_capacity(self.entities.len());
            let mut town_actor_ids = BTreeSet::new();
            for mut actor in std::mem::take(&mut self.entities) {
                let should_store = !follows_player(&actor)
                    && actor.pack.as_ref().map_or_else(
                        || town.view_to_local(actor.position).is_some(),
                        |pack| stored_pack_ids.contains(pack.id.as_str()),
                    );
                if should_store {
                    actor.position = town
                        .view_to_local(actor.position)
                        .expect("stored town actor must be inside the town");
                    town_actor_ids.insert(actor.id.clone());
                    stored_actor_ids.insert(actor.id.clone());
                    floor.entities.push(actor);
                } else {
                    active_entities.push(actor);
                }
            }
            self.entities = active_entities;
            floor.entities.sort_by(|left, right| left.id.cmp(&right.id));

            let mut active_items = Vec::with_capacity(self.items.len());
            for mut item in std::mem::take(&mut self.items) {
                let should_store = match &mut item.location {
                    ItemLocation::Ground(position) => {
                        if let Some(local) = town.view_to_local(*position) {
                            *position = local;
                            true
                        } else {
                            false
                        }
                    }
                    ItemLocation::CarriedBy { actor_id } => town_actor_ids.contains(actor_id),
                    _ => false,
                };
                if should_store {
                    floor.items.push(item);
                } else {
                    active_items.push(item);
                }
            }
            self.items = active_items;

            let mut active_gold = Vec::with_capacity(self.gold_piles.len());
            for mut pile in std::mem::take(&mut self.gold_piles) {
                if let Some(local) = town.view_to_local(pile.position) {
                    pile.position = local;
                    floor.gold_piles.push(pile);
                } else {
                    active_gold.push(pile);
                }
            }
            self.gold_piles = active_gold;

            let mut active_connections = Vec::with_capacity(self.floor_connections.len());
            for mut connection in std::mem::take(&mut self.floor_connections) {
                if let Some(local) = town.view_to_local(connection.position) {
                    connection.position = local;
                    floor.connections.push(connection);
                } else {
                    active_connections.push(connection);
                }
            }
            self.floor_connections = active_connections;
            floor
                .connections
                .sort_by(|left, right| left.id.cmp(&right.id));

            let mut active_regions = Vec::with_capacity(self.floor_regions.len());
            for mut region in std::mem::take(&mut self.floor_regions) {
                let mut local_cells = Vec::new();
                let mut active_cells = Vec::new();
                for position in std::mem::take(&mut region.cells) {
                    if let Some(local) = town.view_to_local(position) {
                        local_cells.push(local);
                    } else {
                        active_cells.push(position);
                    }
                }
                if !local_cells.is_empty() {
                    let mut stored = region.clone();
                    stored.cells = local_cells;
                    merge_floor_region(&mut floor.regions, stored);
                }
                if !active_cells.is_empty() {
                    region.cells = active_cells;
                    active_regions.push(region);
                }
            }
            self.floor_regions = active_regions;
            self.stored_floors.insert(town.floor_id, floor);
        }
        if !stored_actor_ids.is_empty() {
            self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        }
        stored_actor_ids
    }

    fn load_visible_town_states(&mut self) -> Result<BTreeSet<String>, CoreError> {
        let world_position = self
            .wilderness_position
            .expect("local wilderness must retain its world position");
        let towns = self.visible_towns(world_position);
        let mut loaded_actor_ids = BTreeSet::new();
        for town in towns {
            self.ensure_town_floor_is_stored(&town)?;
            let mut floor = self
                .stored_floors
                .remove(&town.floor_id)
                .expect("visible town floor must be stored");
            let (start_x, start_y, end_x, end_y) = town
                .visible_bounds()
                .expect("visible town must retain an intersection");
            for local_y in start_y..end_y {
                for local_x in start_x..end_x {
                    let local = Position {
                        x: local_x,
                        y: local_y,
                    };
                    let view = town
                        .local_to_view(local)
                        .expect("visible town cell must map into the view");
                    let local_index = usize::try_from(local_y).expect("town y must fit usize")
                        * usize::from(floor.width)
                        + usize::try_from(local_x).expect("town x must fit usize");
                    let view_index = usize::try_from(view.y).expect("view y must fit usize")
                        * usize::from(WILDERNESS_VIEW_WIDTH)
                        + usize::try_from(view.x).expect("view x must fit usize");
                    self.terrain[view_index] = floor.terrain[local_index].clone();
                    self.glow[view_index] = floor.glow[local_index];
                    self.explored[view_index] = floor.explored[local_index];
                }
            }

            let mut stored_revealed = BTreeSet::new();
            for position in std::mem::take(&mut floor.revealed_terrain) {
                if let Some(view) = town.local_to_view(position) {
                    self.revealed_terrain.insert(view);
                } else {
                    stored_revealed.insert(position);
                }
            }
            floor.revealed_terrain = stored_revealed;

            let pack_ids = floor
                .entities
                .iter()
                .filter_map(|actor| actor.pack.as_ref().map(|pack| pack.id.clone()))
                .collect::<BTreeSet<_>>();
            let loaded_pack_ids = pack_ids
                .into_iter()
                .filter(|pack_id| {
                    floor
                        .entities
                        .iter()
                        .filter(|actor| {
                            actor
                                .pack
                                .as_ref()
                                .is_some_and(|pack| pack.id.as_str() == pack_id.as_str())
                        })
                        .all(|actor| town.local_to_view(actor.position).is_some())
                })
                .collect::<BTreeSet<_>>();
            let mut stored_entities = Vec::with_capacity(floor.entities.len());
            let mut town_actor_ids = BTreeSet::new();
            for mut actor in std::mem::take(&mut floor.entities) {
                let should_load = actor.pack.as_ref().map_or_else(
                    || town.local_to_view(actor.position).is_some(),
                    |pack| loaded_pack_ids.contains(pack.id.as_str()),
                );
                if should_load {
                    actor.position = town
                        .local_to_view(actor.position)
                        .expect("loaded town actor must enter the view");
                    town_actor_ids.insert(actor.id.clone());
                    loaded_actor_ids.insert(actor.id.clone());
                    self.entities.push(actor);
                } else {
                    stored_entities.push(actor);
                }
            }
            floor.entities = stored_entities;

            let mut stored_items = Vec::with_capacity(floor.items.len());
            for mut item in std::mem::take(&mut floor.items) {
                let should_load = match &mut item.location {
                    ItemLocation::Ground(position) => {
                        if let Some(view) = town.local_to_view(*position) {
                            *position = view;
                            true
                        } else {
                            false
                        }
                    }
                    ItemLocation::CarriedBy { actor_id } => town_actor_ids.contains(actor_id),
                    _ => false,
                };
                if should_load {
                    self.items.push(item);
                } else {
                    stored_items.push(item);
                }
            }
            floor.items = stored_items;

            let mut stored_gold = Vec::with_capacity(floor.gold_piles.len());
            for mut pile in std::mem::take(&mut floor.gold_piles) {
                if let Some(view) = town.local_to_view(pile.position) {
                    pile.position = view;
                    self.gold_piles.push(pile);
                } else {
                    stored_gold.push(pile);
                }
            }
            floor.gold_piles = stored_gold;

            let mut stored_connections = Vec::with_capacity(floor.connections.len());
            for mut connection in std::mem::take(&mut floor.connections) {
                if let Some(view) = town.local_to_view(connection.position) {
                    connection.position = view;
                    self.floor_connections.push(connection);
                } else {
                    stored_connections.push(connection);
                }
            }
            floor.connections = stored_connections;

            let mut stored_regions = Vec::with_capacity(floor.regions.len());
            for mut region in std::mem::take(&mut floor.regions) {
                let mut visible_cells = Vec::new();
                let mut stored_cells = Vec::new();
                for position in std::mem::take(&mut region.cells) {
                    if let Some(view) = town.local_to_view(position) {
                        visible_cells.push(view);
                    } else {
                        stored_cells.push(position);
                    }
                }
                if !visible_cells.is_empty() {
                    let mut visible = region.clone();
                    visible.cells = visible_cells;
                    merge_floor_region(&mut self.floor_regions, visible);
                }
                if !stored_cells.is_empty() {
                    region.cells = stored_cells;
                    stored_regions.push(region);
                }
            }
            floor.regions = stored_regions;
            self.stored_floors.insert(town.floor_id, floor);
        }
        if !loaded_actor_ids.is_empty() {
            self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        }
        Ok(loaded_actor_ids)
    }

    fn town_template_terrain(&self, town_id: &str) -> (u16, u16, Vec<String>) {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let town = self
            .content
            .town(town_id)
            .expect("validated wilderness town must remain available");
        if town.floor_id == world.initial_floor_id {
            let mut terrain = vec![
                world.fill_terrain_id.clone();
                usize::from(world.width) * usize::from(world.height)
            ];
            for y in 0..world.height {
                for x in 0..world.width {
                    if x == 0 || y == 0 || x + 1 == world.width || y + 1 == world.height {
                        terrain[usize::from(y) * usize::from(world.width) + usize::from(x)] =
                            world.border_terrain_id.clone();
                    }
                }
            }
            for terrain_override in &world.terrain_overrides {
                for position in &terrain_override.positions {
                    terrain[usize::from(position.y) * usize::from(world.width)
                        + usize::from(position.x)] = terrain_override.terrain_id.clone();
                }
            }
            return (world.width, world.height, terrain);
        }

        let floor = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == town.floor_id && floor.lifecycle == FloorLifecycle::Town)
            .expect("validated town floor must remain available");
        let inline_map = floor
            .inline_map
            .as_ref()
            .expect("validated town floor must retain its inline map");
        let mut terrain = vec![
            floor.wall_terrain_id.clone();
            usize::from(floor.width) * usize::from(floor.height)
        ];
        for terrain_override in &inline_map.terrain_overrides {
            debug_assert_eq!(terrain_override.chance_percent, 100);
            for position in &terrain_override.positions {
                terrain[usize::from(position.y) * usize::from(floor.width)
                    + usize::from(position.x)] = terrain_override.terrain_id.clone();
            }
        }
        (floor.width, floor.height, terrain)
    }

    fn overlay_visible_town_terrain(&self, world_position: Position, terrain: &mut [String]) {
        for town in self.visible_towns(world_position) {
            let (_, _, town_terrain) = self.town_template_terrain(&town.town_id);
            let (source_start_x, source_start_y, source_end_x, source_end_y) = town
                .visible_bounds()
                .expect("visible town must retain an intersection");
            let copy_width = usize::try_from(source_end_x - source_start_x)
                .expect("visible town width must fit usize");
            for source_y in source_start_y..source_end_y {
                let destination_y = town.view_origin.y + source_y;
                let source_start = usize::try_from(source_y)
                    .expect("visible town y must fit usize")
                    * usize::from(town.width)
                    + usize::try_from(source_start_x).expect("visible town x must fit usize");
                let destination_start = usize::try_from(destination_y)
                    .expect("visible destination y must fit usize")
                    * usize::from(WILDERNESS_VIEW_WIDTH)
                    + usize::try_from(town.view_origin.x + source_start_x)
                        .expect("visible destination x must fit usize");
                terrain[destination_start..destination_start + copy_width]
                    .clone_from_slice(&town_terrain[source_start..source_start + copy_width]);
            }
        }
    }

    fn overlay_visible_dungeon_entrances(&self, world_position: Position, terrain: &mut [String]) {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let wilderness = world
            .wilderness
            .as_ref()
            .expect("local wilderness requires wilderness content");
        for location in &wilderness.locations {
            let WildernessLocationDefinition::Dungeon {
                position,
                dungeon_id,
            } = location
            else {
                continue;
            };
            let dungeon_world_position = position_from_content(*position);
            if wilderness_has_town(wilderness, dungeon_world_position) {
                continue;
            }
            let view = Position {
                x: (dungeon_world_position.x - world_position.x) * i32::from(WILDERNESS_VIEW_WIDTH)
                    + i32::from(WILDERNESS_VIEW_WIDTH / 2)
                    - self.wilderness_view_offset.x * i32::from(WILDERNESS_CHUNK_WIDTH),
                y: (dungeon_world_position.y - world_position.y)
                    * i32::from(WILDERNESS_VIEW_HEIGHT)
                    + i32::from(WILDERNESS_VIEW_HEIGHT / 2)
                    - self.wilderness_view_offset.y * i32::from(WILDERNESS_CHUNK_HEIGHT),
            };
            if !(0..i32::from(WILDERNESS_VIEW_WIDTH)).contains(&view.x)
                || !(0..i32::from(WILDERNESS_VIEW_HEIGHT)).contains(&view.y)
            {
                continue;
            }
            let dungeon = world
                .dungeons
                .iter()
                .find(|dungeon| dungeon.id == *dungeon_id)
                .expect("validated wilderness dungeon must remain available");
            let entrance = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == dungeon.root_floor_id)
                .and_then(|floor| floor.entry_terrain_id.as_ref())
                .expect("validated dungeon root must retain an entrance");
            let index = usize::try_from(view.y).expect("dungeon entrance y must fit usize")
                * usize::from(WILDERNESS_VIEW_WIDTH)
                + usize::try_from(view.x).expect("dungeon entrance x must fit usize");
            terrain[index] = entrance.clone();
        }
    }

    fn cached_wilderness_view_terrain(&mut self, world_position: Position) -> Vec<String> {
        let center = wilderness_view_center_chunk(world_position, self.wilderness_view_offset);
        self.wilderness_terrain_cache.retain(|position, _| {
            (position.x - center.x).abs() <= WILDERNESS_CACHE_RADIUS_CHUNKS
                && (position.y - center.y).abs() <= WILDERNESS_CACHE_RADIUS_CHUNKS
        });
        let wilderness_seed = self.wilderness_seed;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let chunk = Position {
                    x: center.x + dx,
                    y: center.y + dy,
                };
                if self.wilderness_terrain_cache.contains_key(&chunk) {
                    continue;
                }
                let terrain = generate_wilderness_chunk(self.wilderness(), wilderness_seed, chunk);
                self.wilderness_terrain_cache.insert(chunk, terrain);
            }
        }

        let view_width = usize::from(WILDERNESS_VIEW_WIDTH);
        let chunk_width = usize::from(WILDERNESS_CHUNK_WIDTH);
        let chunk_height = usize::from(WILDERNESS_CHUNK_HEIGHT);
        let mut terrain = vec![String::new(); view_width * usize::from(WILDERNESS_VIEW_HEIGHT)];
        for chunk_y in 0..3_usize {
            for chunk_x in 0..3_usize {
                let chunk = Position {
                    x: center.x + i32::try_from(chunk_x).expect("small chunk x must fit i32") - 1,
                    y: center.y + i32::try_from(chunk_y).expect("small chunk y must fit i32") - 1,
                };
                let source = self
                    .wilderness_terrain_cache
                    .get(&chunk)
                    .expect("visible wilderness chunk must be cached");
                for local_y in 0..chunk_height {
                    let source_start = local_y * chunk_width;
                    let destination_start =
                        (chunk_y * chunk_height + local_y) * view_width + chunk_x * chunk_width;
                    terrain[destination_start..destination_start + chunk_width]
                        .clone_from_slice(&source[source_start..source_start + chunk_width]);
                }
            }
        }
        self.overlay_visible_town_terrain(world_position, &mut terrain);
        self.overlay_visible_dungeon_entrances(world_position, &mut terrain);
        terrain
    }

    fn generate_local_wilderness_floor(
        &mut self,
        world_position: Position,
        arrival: Option<Position>,
    ) -> FloorState {
        let width = WILDERNESS_VIEW_WIDTH;
        let height = WILDERNESS_VIEW_HEIGHT;
        let current_road = wilderness_legend_at(self.wilderness(), world_position)
            .expect("validated wilderness position must remain defined");
        let current_road = current_road.road;
        let player_position = arrival.unwrap_or(Position {
            x: i32::from(width) / 2,
            y: i32::from(height) / 2,
        });
        let mut terrain = self.cached_wilderness_view_terrain(world_position);
        let player_index = usize::try_from(player_position.y).expect("arrival y must fit usize")
            * usize::from(width)
            + usize::try_from(player_position.x).expect("arrival x must fit usize");
        let player_on_dungeon_entrance = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available")
            .procedural_floors
            .iter()
            .any(|floor| {
                floor.lifecycle == FloorLifecycle::Dungeon
                    && floor.return_floor_id
                        == self
                            .content
                            .world(&self.world_id)
                            .expect("active world must remain available")
                            .initial_floor_id
                    && floor.entry_terrain_id.as_deref() == Some(terrain[player_index].as_str())
            });
        if !player_on_dungeon_entrance {
            terrain[player_index] = if current_road {
                SURFACE_PATH_ID
            } else {
                let current = wilderness_legend_at(self.wilderness(), world_position)
                    .expect("validated wilderness position must remain defined");
                terrain_id_for_wilderness(current.terrain)
            }
            .to_owned();
        }
        FloorState {
            id: WILDERNESS_FLOOR_ID.to_owned(),
            dungeon_instance_id: None,
            reproduction_suppressed: false,
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
        wilderness_site_is_interesting(self.wilderness(), self.wilderness_seed, position)
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

    fn wilderness_initial_monster_rolls_at(&self, position: Position) -> u16 {
        let road =
            wilderness_legend_at(self.wilderness(), position).is_some_and(|entry| entry.road);
        self.content
            .world(&self.world_id)
            .and_then(|world| world.surface_actor_allocation)
            .map_or(0, |allocation| {
                wilderness_initial_monster_rolls(allocation.rolls, road)
            })
    }

    pub(super) fn populate_scrolled_wilderness(&mut self, translation: Position) {
        if self.map_scale != MapScaleDto::Local || !self.is_wilderness_floor() {
            return;
        }
        let scroll = Position {
            x: -translation.x / i32::from(WILDERNESS_CHUNK_WIDTH),
            y: -translation.y / i32::from(WILDERNESS_CHUNK_HEIGHT),
        };
        let world_position = self
            .wilderness_position
            .expect("local wilderness must retain its world position");
        let center = wilderness_view_center_chunk(world_position, self.wilderness_view_offset);
        let exposed_chunks = wilderness_exposed_chunks(center, scroll);
        let exposed_positions = wilderness_exposed_positions(scroll);
        let exposed_cell_count = exposed_positions.len();
        let allowed_positions = self.wilderness_positions_outside_visible_towns(exposed_positions);
        let spawn_seed = wilderness_chunk_set_seed(
            self.wilderness_seed ^ WILDERNESS_SCROLL_RNG_SALT,
            &exposed_chunks,
        );
        let rolls = wilderness_monster_rolls_for_allowed_area(
            wilderness_scroll_monster_rolls(
                self.wilderness_initial_monster_rolls_at(world_position),
                &exposed_chunks,
            ),
            allowed_positions.len(),
            exposed_cell_count,
            spawn_seed,
        );
        if rolls == 0 {
            return;
        }

        let local_rng = RfbRng::seeded(spawn_seed);
        let global_rng = std::mem::replace(&mut self.rng, local_rng);
        let division_remainders = std::mem::take(&mut self.monster_division_remainders);
        let spawn_kind = format!("scroll.{}.{}.{}.{}", center.x, center.y, scroll.x, scroll.y);
        self.initialize_wilderness_monsters_in_positions(
            self.wilderness_danger_level(world_position),
            rolls,
            &spawn_kind,
            &allowed_positions,
        );
        self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        self.monster_division_remainders = division_remainders;
        self.rng = global_rng;
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
        let view_positions = wilderness_view_positions();
        let view_cell_count = view_positions.len();
        let allowed_positions = self.wilderness_positions_outside_visible_towns(view_positions);
        let rolls = if ambush {
            WILDERNESS_AMBUSH_ROLLS
        } else {
            wilderness_monster_rolls_for_allowed_area(
                self.wilderness_initial_monster_rolls_at(position),
                allowed_positions.len(),
                view_cell_count,
                seed,
            )
        };
        self.initialize_wilderness_monsters_in_positions(
            level,
            rolls,
            spawn_kind,
            &allowed_positions,
        );
        self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        self.monster_division_remainders = division_remainders;
        self.rng = global_rng;
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

    #[test]
    fn wilderness_initial_monster_rolls_follow_original_road_density() {
        assert_eq!(wilderness_initial_monster_rolls(12, true), 4);
        assert_eq!(wilderness_initial_monster_rolls(12, false), 10);
        assert_eq!(wilderness_initial_monster_rolls(3, true), 3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilderness_view_is_three_by_three_chunks() {
        assert_eq!(WILDERNESS_VIEW_WIDTH, 96);
        assert_eq!(WILDERNESS_VIEW_HEIGHT, 33);
        assert_eq!(WILDERNESS_CHUNK_WIDTH, 32);
        assert_eq!(WILDERNESS_CHUNK_HEIGHT, 11);
        assert_eq!(WILDERNESS_VIEW_WIDTH, WILDERNESS_CHUNK_WIDTH * 3);
        assert_eq!(WILDERNESS_VIEW_HEIGHT, WILDERNESS_CHUNK_HEIGHT * 3);
    }

    #[test]
    fn exposed_wilderness_area_is_one_third_or_five_ninths() {
        let center = Position { x: 90, y: 156 };
        let horizontal = wilderness_exposed_chunks(center, Position { x: 1, y: 0 });
        let diagonal = wilderness_exposed_chunks(center, Position { x: 1, y: 1 });
        assert_eq!(horizontal.len(), 3);
        assert_eq!(diagonal.len(), 5);
        assert_eq!(
            wilderness_exposed_positions(Position { x: 1, y: 0 }).len(),
            32 * 33
        );
        assert_eq!(
            wilderness_exposed_positions(Position { x: 1, y: 1 }).len(),
            32 * 33 + 96 * 11 - 32 * 11
        );
    }

    #[test]
    fn scroll_monster_rolls_round_by_absolute_chunk_coordinates() {
        let horizontal =
            wilderness_exposed_chunks(Position { x: 90, y: 156 }, Position { x: 1, y: 0 });
        let diagonal =
            wilderness_exposed_chunks(Position { x: 90, y: 156 }, Position { x: 1, y: 1 });
        assert_eq!(wilderness_scroll_monster_rolls(4, &horizontal), 1);
        assert_eq!(wilderness_scroll_monster_rolls(10, &horizontal), 3);
        assert_eq!(wilderness_scroll_monster_rolls(4, &diagonal), 3);
        assert_eq!(wilderness_scroll_monster_rolls(10, &diagonal), 6);

        let outcomes = (0..100)
            .map(|x| {
                let chunks =
                    wilderness_exposed_chunks(Position { x, y: 156 }, Position { x: 1, y: 0 });
                wilderness_scroll_monster_rolls(10, &chunks)
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(outcomes, BTreeSet::from([3, 4]));
    }

    #[test]
    fn monster_rolls_scale_to_the_non_town_area() {
        assert_eq!(wilderness_monster_rolls_for_allowed_area(10, 0, 100, 0), 0);
        assert_eq!(
            wilderness_monster_rolls_for_allowed_area(10, 100, 100, 0),
            10
        );
        assert_eq!(wilderness_monster_rolls_for_allowed_area(10, 50, 100, 0), 5);
        assert_eq!(wilderness_monster_rolls_for_allowed_area(4, 15, 100, 0), 1);
        assert_eq!(wilderness_monster_rolls_for_allowed_area(4, 15, 100, 99), 0);
    }

    #[test]
    fn monster_candidates_exclude_the_actual_town_rectangle_only() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        game.wilderness_position = Some(Position { x: 26, y: 39 });
        game.wilderness_view_offset = Position::default();
        let view = wilderness_view_positions();
        let view_cell_count = view.len();

        let allowed = game.wilderness_positions_outside_visible_towns(view);

        assert_eq!(allowed.len(), view_cell_count - 23 * 11);
        assert!(!allowed.contains(&Position { x: 27, y: 6 }));
        assert!(!allowed.contains(&Position { x: 49, y: 16 }));
        assert!(allowed.contains(&Position { x: 26, y: 6 }));
        assert!(allowed.contains(&Position { x: 50, y: 16 }));
    }

    #[test]
    fn adjacent_cached_views_share_the_same_overlapping_terrain() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        let position = game
            .wilderness_position
            .expect("Warrens journey should define a wilderness start");
        let initial = game.cached_wilderness_view_terrain(position);

        game.wilderness_view_offset = Position { x: 1, y: 0 };
        let shifted = game.cached_wilderness_view_terrain(position);

        let width = usize::from(WILDERNESS_VIEW_WIDTH);
        let overlap_width = width - usize::from(WILDERNESS_CHUNK_WIDTH);
        for y in 0..usize::from(WILDERNESS_VIEW_HEIGHT) {
            assert_eq!(
                &initial[y * width + usize::from(WILDERNESS_CHUNK_WIDTH)..y * width + width],
                &shifted[y * width..y * width + overlap_width]
            );
        }
    }

    #[test]
    fn town_terrain_is_composed_after_the_seeded_cache() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        let position = game
            .wilderness_position
            .expect("Warrens journey should define a wilderness start");
        let terrain = game.cached_wilderness_view_terrain(position);
        let view_index = 6 * usize::from(WILDERNESS_VIEW_WIDTH) + 22;
        assert_eq!(terrain[view_index], "demo.terrain.outpost-fortification");

        let center = wilderness_view_center_chunk(position, Position::default());
        let top_left_chunk = Position {
            x: center.x - 1,
            y: center.y - 1,
        };
        let cached = game
            .wilderness_terrain_cache
            .get(&top_left_chunk)
            .expect("visible base chunk should remain cached");
        let chunk_index = 6 * usize::from(WILDERNESS_CHUNK_WIDTH) + 22;
        assert_ne!(cached[chunk_index], "demo.terrain.outpost-fortification");
    }

    #[test]
    fn town_terrain_stays_fixed_when_wilderness_seed_advances() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        let anambar = Position { x: 26, y: 39 };
        let initial = game.cached_wilderness_view_terrain(anambar);
        game.advance_wilderness_generation();
        let evolved = game.cached_wilderness_view_terrain(anambar);

        let width = usize::from(WILDERNESS_VIEW_WIDTH);
        let mut outside_changed = false;
        for y in 0..usize::from(WILDERNESS_VIEW_HEIGHT) {
            for x in 0..width {
                let index = y * width + x;
                if (27..50).contains(&x) && (6..17).contains(&y) {
                    assert_eq!(evolved[index], initial[index]);
                } else if evolved[index] != initial[index] {
                    outside_changed = true;
                }
            }
        }
        assert_eq!(initial[16 * width + 48], "demo.terrain.outpost-gate");
        assert!(outside_changed);
    }

    #[test]
    fn adjacent_view_crops_town_at_its_map_origin() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        let anambar = Position { x: 26, y: 39 };
        let full = game.cached_wilderness_view_terrain(anambar);

        game.wilderness_view_offset = Position { x: 1, y: 0 };
        let partial = game.cached_wilderness_view_terrain(Position { x: 25, y: 39 });
        let width = usize::from(WILDERNESS_VIEW_WIDTH);
        assert_eq!(partial[6 * width + 91], "demo.terrain.outpost-wall");
        for y in 0..11 {
            for x in 0..5 {
                assert_eq!(
                    partial[(6 + y) * width + 91 + x],
                    full[(6 + y) * width + 27 + x]
                );
            }
        }
    }

    #[test]
    fn town_overlay_remains_continuous_when_revealed_from_each_direction() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        let anambar = Position { x: 26, y: 39 };
        let approaches = [
            (
                Position { x: 25, y: 39 },
                Position { x: 1, y: 0 },
                Position { x: -1, y: 0 },
                Position { x: -32, y: 0 },
            ),
            (
                Position { x: 27, y: 39 },
                Position { x: -1, y: 0 },
                Position { x: 1, y: 0 },
                Position { x: 32, y: 0 },
            ),
            (
                Position { x: 26, y: 38 },
                Position { x: 0, y: 1 },
                Position { x: 0, y: -1 },
                Position { x: 0, y: -11 },
            ),
            (
                Position { x: 26, y: 40 },
                Position { x: 0, y: -1 },
                Position { x: 0, y: 1 },
                Position { x: 0, y: 11 },
            ),
        ];
        let width = usize::from(WILDERNESS_VIEW_WIDTH);

        for (before_world, before_offset, after_offset, translation) in approaches {
            game.wilderness_view_offset = before_offset;
            let before = game.cached_wilderness_view_terrain(before_world);
            game.wilderness_view_offset = after_offset;
            let after = game.cached_wilderness_view_terrain(anambar);
            assert!(
                after
                    .iter()
                    .any(|terrain_id| terrain_id == "demo.terrain.outpost-wall")
            );

            for y in 0..i32::from(WILDERNESS_VIEW_HEIGHT) {
                for x in 0..i32::from(WILDERNESS_VIEW_WIDTH) {
                    let position = Position { x, y };
                    let Some(translated) = translate_wilderness_position(position, translation)
                    else {
                        continue;
                    };
                    let before_index = usize::try_from(y).expect("view y must fit usize") * width
                        + usize::try_from(x).expect("view x must fit usize");
                    let after_index =
                        usize::try_from(translated.y).expect("translated y must fit usize") * width
                            + usize::try_from(translated.x).expect("translated x must fit usize");
                    assert_eq!(after[after_index], before[before_index]);
                }
            }
        }
    }

    #[test]
    fn birth_town_uses_the_continuous_wilderness_surface() {
        let game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");

        assert_eq!(game.current_floor_id, WILDERNESS_FLOOR_ID);
        assert_eq!((game.width, game.height), (96, 33));
        assert_eq!(
            game.current_town().map(|town| town.id.as_str()),
            Some("demo.town.outpost")
        );
        assert!(game.stored_floors.contains_key("demo.floor.surface"));
        assert!(game.items.iter().any(|item| {
            item.id == "demo.item.warrens-short-sword.1"
                && item.location == ItemLocation::Ground(Position { x: 45, y: 16 })
        }));
        assert!(game.stored_floors["demo.floor.surface"].items.is_empty());
    }

    #[test]
    fn town_state_moves_to_backing_storage_and_returns_with_the_view() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        let remembered = Position { x: 10, y: 10 };
        let actor_definition = game
            .content
            .actor("demo.actor.small-kobold")
            .expect("small kobold should remain available")
            .clone();
        game.entities.push(spawn_actor_from_definition(
            &mut game.rng,
            &actor_definition,
            "test.town-surface.actor",
            remembered,
            INITIAL_MONSTER_ENERGY_NEED,
            actor_starts_alerted(&actor_definition),
        ));
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.warrens-short-sword.1")
            .expect("birth town item should remain active");
        item.location = ItemLocation::Ground(Position { x: 11, y: 10 });
        let gold = game
            .generate_gold_pile(Position { x: 12, y: 10 }, 1, false)
            .expect("test gold should generate");
        let gold_id = gold.id.clone();
        game.gold_piles.push(gold);
        let remembered_index = game.index(remembered).expect("town cell should exist");
        game.terrain[remembered_index] = "demo.terrain.created-trap".to_owned();
        game.glow[remembered_index] = true;
        game.explored[remembered_index] = true;
        game.revealed_terrain.insert(remembered);

        game.player.position = Position { x: 63, y: 16 };
        let east = game
            .scroll_wilderness_for_player_entry(Position { x: 64, y: 16 }, &mut Vec::new())
            .expect("eastward scroll should resolve");
        let WildernessPlayerEntry::Local { target, .. } = east else {
            panic!("eastward scroll should retain the wilderness floor");
        };
        game.relocate_player(target, &mut BTreeSet::new());

        let backing = &game.stored_floors["demo.floor.surface"];
        let backing_index = 10 * usize::from(backing.width) + 10;
        assert_eq!(backing.terrain[backing_index], "demo.terrain.created-trap");
        assert!(backing.glow[backing_index]);
        assert!(backing.explored[backing_index]);
        assert!(backing.revealed_terrain.contains(&remembered));
        assert!(backing.entities.iter().any(|actor| {
            actor.id == "test.town-surface.actor" && actor.position == remembered
        }));
        assert!(backing.items.iter().any(|item| {
            item.id == "demo.item.warrens-short-sword.1"
                && item.location == ItemLocation::Ground(Position { x: 11, y: 10 })
        }));
        assert!(
            backing
                .gold_piles
                .iter()
                .any(|pile| pile.id == gold_id && pile.position == Position { x: 12, y: 10 })
        );
        assert!(
            !game
                .entities
                .iter()
                .any(|actor| actor.id == "test.town-surface.actor")
        );

        let west = game
            .scroll_wilderness_for_player_entry(Position { x: 31, y: 16 }, &mut Vec::new())
            .expect("westward scroll should resolve");
        let WildernessPlayerEntry::Local { target, .. } = west else {
            panic!("westward scroll should retain the wilderness floor");
        };
        game.relocate_player(target, &mut BTreeSet::new());

        assert!(game.entities.iter().any(|actor| {
            actor.id == "test.town-surface.actor" && actor.position == remembered
        }));
        assert!(game.items.iter().any(|item| {
            item.id == "demo.item.warrens-short-sword.1"
                && item.location == ItemLocation::Ground(Position { x: 11, y: 10 })
        }));
        assert!(
            game.gold_piles
                .iter()
                .any(|pile| pile.id == gold_id && pile.position == Position { x: 12, y: 10 })
        );
        assert_eq!(game.terrain[remembered_index], "demo.terrain.created-trap");
        assert!(game.glow[remembered_index]);
        assert!(game.explored[remembered_index]);
        assert!(game.revealed_terrain.contains(&remembered));
        let backing = &game.stored_floors["demo.floor.surface"];
        assert!(backing.entities.is_empty());
        assert!(
            !backing
                .items
                .iter()
                .any(|item| item.id == "demo.item.warrens-short-sword.1")
        );
        assert!(!backing.gold_piles.iter().any(|pile| pile.id == gold_id));

        let restored = Game::from_save(game.to_save()).expect("town surface state should reload");
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    #[test]
    fn first_visible_anambar_slice_initializes_its_backing_floor() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        game.store_visible_town_states();
        game.wilderness_position = Some(Position { x: 25, y: 39 });
        game.wilderness_view_offset = Position { x: 1, y: 0 };
        let terrain = game.cached_wilderness_view_terrain(Position { x: 25, y: 39 });
        game.terrain = terrain;
        game.glow.fill(false);
        game.explored.fill(false);
        game.revealed_terrain.clear();

        game.load_visible_town_states()
            .expect("visible Anambar slice should initialize");

        let anambar = &game.stored_floors["demo.floor.anambar"];
        assert_eq!((anambar.width, anambar.height), (23, 11));
        assert_eq!(
            game.terrain_at(Position { x: 91, y: 6 }),
            "demo.terrain.outpost-wall"
        );
    }

    #[test]
    fn wilderness_terrain_cache_is_derived_bounded_and_seeded_by_generation() {
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
        let position = game
            .wilderness_position
            .expect("Warrens journey should define a wilderness start");
        let state_hash = game.state_hash();
        let initial = game.cached_wilderness_view_terrain(position);
        assert_eq!(game.wilderness_terrain_cache.len(), 9);
        assert_eq!(game.state_hash(), state_hash);

        game.wilderness_terrain_cache.clear();
        assert_eq!(game.cached_wilderness_view_terrain(position), initial);
        assert_eq!(game.state_hash(), state_hash);

        let old_center = wilderness_view_center_chunk(position, Position::default());
        let far_position = Position {
            x: position.x + 2,
            y: position.y,
        };
        game.cached_wilderness_view_terrain(far_position);
        assert!(game.wilderness_terrain_cache.len() <= 25);
        assert!(!game.wilderness_terrain_cache.contains_key(&old_center));

        game.advance_wilderness_generation();
        assert!(game.wilderness_terrain_cache.is_empty());
        let evolved = game.cached_wilderness_view_terrain(position);
        assert_ne!(evolved, initial);
    }

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
        let game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");

        assert!(!game.player_can_enter_world_cell(Position { x: 0, y: 0 }));
        assert!(game.player_can_enter_world_cell(Position { x: 1, y: 1 }));
        assert!(game.player_can_enter_world_cell(Position { x: 29, y: 52 }));
    }

    #[test]
    fn world_travel_uses_eight_direction_pathfinding() {
        let game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");

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
        let mut game =
            Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
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

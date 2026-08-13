// SPDX-License-Identifier: MPL-2.0

use super::support::*;
use super::*;

const SNOW_ID: &str = "demo.terrain.surface-snow";

fn local_snow_game(seed: u64) -> (Game, Position, Position) {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("snow test game should create");
    clear_monsters(&mut game);
    let start = Position { x: 48, y: 16 };
    let target = Position { x: 49, y: 16 };
    replace_terrain(&mut game, start, "demo.terrain.floor");
    replace_terrain(&mut game, target, SNOW_ID);
    game.player.position = start;
    (game, start, target)
}

fn expected_ticks(game: &Game, action_cost: i32) -> u32 {
    let gain = energy_gain(derived_speed(&game.player_derived_stats().speed));
    u32::try_from(action_cost.saturating_add(gain - 1) / gain)
        .expect("test action ticks should fit u32")
}

fn mounted_snow_game(seed: u64, snow_adapted: bool) -> Game {
    let mut game = game_with_actor_definition(seed, "demo.actor.horse", |horse| {
        if snow_adapted {
            let allocation = horse
                .allocation
                .as_mut()
                .expect("horse should have wilderness allocation");
            allocation.habitats.push(rfb_content::ActorHabitat::Snow);
        }
    });
    clear_monsters(&mut game);
    let position = Position { x: 48, y: 16 };
    replace_terrain(&mut game, position, SNOW_ID);
    game.player.position = position;
    game.push_generated_actor("test.snow-mount".to_owned(), "demo.actor.horse", position);
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.snow-mount".to_owned());
    game
}

#[test]
fn snow_action_cost_matches_original_weight_mount_and_world_caps() {
    assert_eq!(
        wilderness::snow_movement_action_cost(100, 1_000, 1_000, false),
        133
    );
    assert_eq!(
        wilderness::snow_movement_action_cost(100, 1_500, 1_000, false),
        183
    );
    assert_eq!(
        wilderness::snow_movement_action_cost(100, 3_000, 1_000, false),
        233
    );
    assert_eq!(
        wilderness::snow_movement_action_cost(100, 3_000, 1_000, true),
        140
    );
    assert_eq!(
        wilderness::snow_movement_action_cost(
            STANDARD_ACTION_COST * wilderness::WORLD_MAP_ACTION_MULTIPLIER,
            1_000,
            1_000,
            false,
        ),
        13_239
    );
}

#[test]
fn snow_penalty_applies_only_after_a_successful_local_move() {
    let (mut moved, _, target) = local_snow_game(51);
    let moved_ticks = expected_ticks(&moved, 133);
    let tick_before = moved.world_tick;
    dispatch_next(
        &mut moved,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(moved.player.position, target);
    assert_eq!(moved.world_tick - tick_before, moved_ticks);

    let (mut blocked, start, target) = local_snow_game(52);
    blocked.push_generated_actor("test.snow-blocker".to_owned(), "demo.actor.horse", target);
    blocked.entities[0].controller_id = Some(blocked.player.id.clone());
    let blocked_ticks = expected_ticks(&blocked, STANDARD_ACTION_COST);
    let tick_before = blocked.world_tick;
    dispatch_next(
        &mut blocked,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(blocked.player.position, start);
    assert_eq!(blocked.world_tick - tick_before, blocked_ticks);
}

#[test]
fn flight_high_elf_and_snow_adapted_mounts_ignore_snow() {
    let (mut ordinary, _, target) = local_snow_game(53);
    ordinary.player.position = target;
    assert_eq!(ordinary.player_snow_movement_action_cost(100), 133);

    let mut flying = ordinary.clone();
    assert!(flying.gain_mutation("rfb.mutation.wings", &mut Vec::new()));
    assert_eq!(flying.player_snow_movement_action_cost(100), 100);

    let mut high_elf = Game::new_with_build_race_and_name(
        53,
        "demo.build.warrior",
        "rfb-legacy.race.high-elf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal High-Elf should create");
    clear_monsters(&mut high_elf);
    replace_terrain(&mut high_elf, target, SNOW_ID);
    high_elf.player.position = target;
    assert_eq!(high_elf.player_snow_movement_action_cost(100), 100);

    assert_eq!(
        mounted_snow_game(54, false).player_snow_movement_action_cost(100),
        140
    );
    assert_eq!(
        mounted_snow_game(55, true).player_snow_movement_action_cost(100),
        100
    );
}

#[test]
fn successful_world_map_move_into_snow_uses_the_capped_surcharge() {
    let mut game =
        Game::new_with_build(56, "demo.build.warrior").expect("world snow game should create");
    game.progress.level = 50;
    game.progress.max_level = 50;
    choose_human_talent_if_pending(&mut game);
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );

    let directions = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];
    let (start, target, direction) = (1..i32::from(game.height) - 1)
        .flat_map(|y| (1..i32::from(game.width) - 1).map(move |x| Position { x, y }))
        .find_map(|target| {
            (game.world_cell_terrain_id(target) == Some(SNOW_ID)).then(|| {
                directions.iter().copied().find_map(|direction| {
                    let (dx, dy) = direction.delta();
                    let start = Position {
                        x: target.x - dx,
                        y: target.y - dy,
                    };
                    game.world_cell_terrain_id(start)
                        .is_some()
                        .then_some((start, target, direction))
                })
            })?
        })
        .expect("Middle-earth should contain an enterable snow cell");
    game.wilderness_position = Some(start);

    let action_cost = STANDARD_ACTION_COST * wilderness::WORLD_MAP_ACTION_MULTIPLIER + 39;
    let ticks = expected_ticks(&game, action_cost);
    let tick_before = game.world_tick;
    dispatch_next(&mut game, GameCommand::Move { direction });

    assert_eq!(game.wilderness_position, Some(target));
    assert_eq!(game.world_tick - tick_before, ticks);
}

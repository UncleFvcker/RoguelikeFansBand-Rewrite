// SPDX-License-Identifier: MPL-2.0
use super::*;

pub(super) fn command(seq: u32, revision: u32, command: GameCommand) -> GameCommandEnvelope {
    GameCommandEnvelope {
        command_seq: seq,
        expected_revision: revision,
        command,
    }
}

pub(super) fn dispatch_next(game: &mut Game, command_value: GameCommand) -> GameUpdate {
    let snapshot = game.snapshot();
    game.dispatch(command(
        snapshot.last_command_seq + 1,
        snapshot.revision,
        command_value,
    ))
    .expect("test command should execute")
}

pub(super) fn descend_one_floor(game: &mut Game) {
    if game.current_floor_id == "demo.floor.surface" {
        game.player.position = Position { x: 3, y: 4 };
    } else {
        let down_index = game
            .terrain
            .iter()
            .position(|terrain_id| terrain_id == "demo.terrain.stairs-down")
            .expect("current floor should contain descending stairs");
        game.player.position = Position {
            x: i32::try_from(down_index % usize::from(game.width))
                .expect("descending stair x must fit i32"),
            y: i32::try_from(down_index / usize::from(game.width))
                .expect("descending stair y must fit i32"),
        };
    }
    game.traverse_stairs(false)
        .expect("descent should resolve")
        .expect("descent should transition");
}

pub(super) fn place_player_on_terrain(game: &mut Game, terrain_id: &str) {
    let index = game
        .terrain
        .iter()
        .position(|candidate| candidate == terrain_id)
        .unwrap_or_else(|| panic!("current floor should contain {terrain_id}"));
    game.player.position = Position {
        x: i32::try_from(index % usize::from(game.width)).expect("terrain x must fit i32"),
        y: i32::try_from(index / usize::from(game.width)).expect("terrain y must fit i32"),
    };
}

pub(super) fn connection_position(game: &Game, connection_id: &str) -> Position {
    game.floor_connections
        .iter()
        .find(|connection| connection.id == connection_id)
        .unwrap_or_else(|| panic!("floor should contain connection {connection_id}"))
        .position
}

pub(super) fn traverse_connection(game: &mut Game, connection_id: &str) {
    game.player.position = connection_position(game, connection_id);
    game.traverse_stairs(false)
        .expect("connection traversal should resolve")
        .expect("connection traversal should transition");
}

pub(super) fn stored_floor<'a>(game: &'a Game, floor_id: &str) -> &'a FloorState {
    game.stored_floors
        .values()
        .find(|floor| floor.id == floor_id)
        .unwrap_or_else(|| panic!("stored floor {floor_id} should exist"))
}

pub(super) fn region_at(game: &Game, position: Position) -> &FloorRegionState {
    game.floor_regions
        .iter()
        .find(|region| region.cells.contains(&position))
        .unwrap_or_else(|| panic!("position {position:?} should belong to a floor region"))
}

pub(super) fn visual_at(snapshot: &GameSnapshot, position: Position) -> CellVisualDto {
    *snapshot
        .visual_cells
        .iter()
        .find(|visual| visual.position == position)
        .expect("snapshot should contain every visual cell")
}

pub(super) fn assert_invariant_error_without_mutation(
    game: &mut Game,
    game_command: GameCommand,
    expected: &str,
) {
    let before = game.clone();
    let error = game
        .dispatch(command(1, 0, game_command))
        .expect_err("broken runtime reference should fail");
    match error {
        CoreError::Invariant(message) => assert_eq!(message, expected),
        other => panic!("expected an invariant error, got {other}"),
    }
    assert_eq!(game.to_save(), before.to_save());
    assert_eq!(game.resources_touched, before.resources_touched);
    assert_eq!(game.last_visual_cells, before.last_visual_cells);
}

pub(super) fn prepare_death_caster(seed: u64, level: u16, ability_id: &str) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.progress.level = level;
    game.progress.max_level = level;
    game.learned_abilities.insert(ability_id.to_owned());
    game.ability_progress
        .get_mut(ability_id)
        .expect("Death ability progress should exist")
        .proficiency = SPELL_EXP_MASTER;
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have Mana");
    mana.current = 1_000;
    mana.maximum = 1_000;
    game
}

pub(super) fn skill_check_game(seed: u64, build_id: &str) -> Game {
    let mut game = Game::new_with_build(seed, build_id).expect("skill-check build should create");
    clear_monsters(&mut game);
    game
}

pub(super) fn give_inventory_item(game: &mut Game, id: &str, kind_id: &str) {
    let (activation, charges) =
        initial_item_runtime_state(&game.content, &mut game.rng, kind_id, 1);
    game.items.push(ItemInstance {
        id: id.to_owned(),
        kind_id: kind_id.to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation,
        charges,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });
}

pub(super) fn replace_terrain(game: &mut Game, position: Position, terrain_id: &str) {
    let index = game
        .index(position)
        .expect("test terrain should be in bounds");
    game.terrain[index] = terrain_id.to_owned();
}

pub(super) fn check_resolution<'a>(
    update: &'a GameUpdate,
    event_kind: &str,
) -> &'a CheckResolutionDto {
    update
        .events
        .iter()
        .find(|event| event.kind == event_kind)
        .and_then(|event| event.outcome.as_ref())
        .and_then(|outcome| match outcome {
            GameEventOutcomeDto::Check { resolution } => Some(resolution),
            _ => None,
        })
        .unwrap_or_else(|| panic!("check event {event_kind} should exist"))
}

pub(super) fn ability_book_item_id(game: &Game) -> String {
    ability_book_item_id_for(game, "demo.item.echo-primer")
}

pub(super) fn ability_book_item_id_for(game: &Game, kind_id: &str) -> String {
    game.items
        .iter()
        .find(|item| item.kind_id == kind_id && item.location == ItemLocation::Inventory)
        .map(|item| item.id.clone())
        .unwrap_or_else(|| panic!("scholar should carry {kind_id}"))
}

pub(super) fn ability_cast_resolution(update: &GameUpdate) -> &AbilityCastResolutionDto {
    update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityCast { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("ability cast resolution should exist")
}

pub(super) fn assert_teleport_target_rejected(
    game: &mut Game,
    ability_id: &str,
    target: TargetSelection,
) {
    let position_before = game.player.position;
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let progress_before = game.ability_progress[ability_id];
    let update = dispatch_next(
        game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target,
        },
    );
    assert_eq!(game.player.position, position_before);
    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.ability_progress[ability_id], progress_before);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert!(!update.events.iter().any(|event| {
        matches!(
            event.outcome.as_ref(),
            Some(GameEventOutcomeDto::AbilityCast { .. })
                | Some(GameEventOutcomeDto::AbilityTeleport { .. })
        )
    }));
}

pub(super) fn rest_resolution(update: &GameUpdate) -> &RestResolutionDto {
    update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::Rest { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("rest resolution should exist")
}

pub(super) fn assert_check(
    resolution: &CheckResolutionDto,
    skill_id: &str,
    ability: i32,
    difficulty: i32,
    percentile_roll: u8,
    contest_roll: Option<i32>,
    threshold: i32,
) {
    assert_eq!(resolution.skill_id, skill_id);
    assert_eq!(resolution.ability, ability);
    assert_eq!(resolution.difficulty, difficulty);
    assert_eq!(resolution.percentile_roll, percentile_roll);
    assert_eq!(resolution.contest_roll, contest_roll);
    assert_eq!(resolution.threshold, threshold);
}

pub(super) fn collect_both_demo_items(game: &mut Game) {
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

pub(super) fn add_player_summon(
    game: &mut Game,
    entity_id: &str,
    position: Position,
    remaining_turns: u16,
) {
    let mut companion =
        game.generated_actor(entity_id.to_owned(), "demo.actor.echo-companion", position);
    companion.summon = Some(SummonIdentity {
        owner_id: game.player.id.clone(),
        source_ability_id: "demo.ability.echo-companion".to_owned(),
        remaining_turns,
    });
    game.entities.push(companion);
}

pub(super) fn clear_monsters(game: &mut Game) {
    game.entities.clear();
    game.dungeon_states
        .get_mut("demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .entrance_guardian_defeated = true;
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
}

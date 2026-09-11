// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::monster_ecology::actor_matches_allocation_terrain;
use crate::game::movement::actor_can_cross_terrain;
use rfb_content::{ActorDamageType, ActorMovementMode, ActorResistanceLevel};

const SHALLOW: &str = "demo.terrain.shallow-waste";
const DEEP: &str = "demo.terrain.deep-waste";
const FLOOR: &str = "demo.terrain.floor";
const START: Position = Position { x: 99, y: 33 };

fn waste_game(terrain: &str) -> Game {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = START;
    for y in 29..=37 {
        for x in 95..=103 {
            replace_terrain(&mut game, Position { x, y }, FLOOR);
        }
    }
    replace_terrain(&mut game, START, terrain);
    game.world_tick = 10;
    game
}

fn tick(game: &mut Game) -> Vec<DomainEvent> {
    game.player.energy_need = 1;
    let mut events = Vec::new();
    game.advance_until_player_ready(
        false,
        true,
        false,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn exposure_seed() -> u64 {
    (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(3) != 0)
        .unwrap()
}

#[test]
fn waste_environment_keeps_source_units_rounding_and_draw_order() {
    let mut game = waste_game(DEEP);
    game.player.hp = 100;
    let seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(3) != 0 && {
                rng.bounded(800);
                rng.bounded(100);
                rng.bounded(100);
                rng.bounded(16) != 0 && rng.bounded(32) != 0
            }
        })
        .unwrap();
    let mut expected = RfbRng::seeded(seed);
    assert_ne!(expected.bounded(3), 0);
    let base = 1400 + expected.bounded(800) as i32;
    let acid = base / 100 + i32::from(expected.bounded(100) < (base % 100) as u64);
    let poison_base = base * 6 / 5;
    let poison = poison_base / 100 + i32::from(expected.bounded(100) < (poison_base % 100) as u64);
    expected.bounded(16);
    expected.bounded(32);
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    assert_eq!(
        game.process_player_waste_damage(&mut events),
        Some((DEEP.to_owned(), poison))
    );
    assert_eq!(game.player.hp, 100 - acid);
    assert!(
        !game.player_has_status_kind(STATUS_POISON),
        "poison is queued for the shared status boundary"
    );
    assert_eq!(game.rng, expected);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::PlayerWasteDamaged { flying: false, .. }]
    ));
}

#[test]
fn waste_resistances_apply_independently_before_poison_accumulates() {
    let mut normal = waste_game(SHALLOW);
    normal.player.hp = 100;
    normal.rng = RfbRng::seeded(exposure_seed());
    let mut acid_immune = normal.clone();
    acid_immune
        .player
        .resistances
        .set(DamageType::Acid, ResistanceLevel::Immune);
    let mut poison_immune = normal.clone();
    poison_immune
        .player
        .resistances
        .set(DamageType::Poison, ResistanceLevel::Immune);
    let normal_poison = normal
        .process_player_waste_damage(&mut Vec::new())
        .unwrap()
        .1;
    let immune_poison = acid_immune
        .process_player_waste_damage(&mut Vec::new())
        .unwrap()
        .1;
    assert_eq!(
        normal_poison, immune_poison,
        "acid resistance must not reduce poison's base"
    );
    assert_eq!(acid_immune.player.hp, 100);
    assert_eq!(
        poison_immune
            .process_player_waste_damage(&mut Vec::new())
            .unwrap()
            .1,
        0
    );
    assert_eq!(normal.player.hp, poison_immune.player.hp);

    let mut game = waste_game(SHALLOW);
    game.player
        .resistances
        .set(DamageType::Acid, ResistanceLevel::Immune);
    game.world_tick = 9;
    game.rng = RfbRng::seeded(exposure_seed());
    let hp = game.player.hp;
    tick(&mut game);
    let poison = game
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_POISON)
        .unwrap()
        .remaining_ticks;
    assert!(poison > 0);
    assert_eq!(
        game.player.hp, hp,
        "new poison must not deal an immediate second hit"
    );
    tick(&mut game);
    assert_eq!(game.player.hp, hp - 1);
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|s| s.kind_id == STATUS_POISON)
            .unwrap()
            .remaining_ticks,
        poison - 1
    );
    game.world_tick = 19;
    game.rng = RfbRng::seeded(exposure_seed());
    tick(&mut game);
    assert!(
        game.player
            .statuses
            .iter()
            .find(|s| s.kind_id == STATUS_POISON)
            .unwrap()
            .remaining_ticks
            > poison
    );
}

#[test]
fn waste_flight_invulnerability_and_nonlocal_ticks_keep_their_source_gates() {
    let mut game = waste_game(SHALLOW);
    game.apply_player_melee_status(STATUS_LEVITATION, 100, "test.flight");
    game.rng = RfbRng::seeded(exposure_seed());
    let mut expected = game.rng.clone();
    expected.bounded(3);
    assert!(game.process_player_waste_damage(&mut Vec::new()).is_none());
    assert_eq!(
        game.rng, expected,
        "shallow flight draws only the exposure gate"
    );
    replace_terrain(&mut game, START, DEEP);
    game.rng = RfbRng::seeded(exposure_seed());
    let mut events = Vec::new();
    let poison = game.process_player_waste_damage(&mut events).unwrap().1;
    assert!((1..=2).contains(&poison));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::PlayerWasteDamaged { flying: true, .. }))
    );
    let mut fractional = game.clone();
    fractional
        .player
        .resistances
        .set(DamageType::Acid, ResistanceLevel::Strong);
    fractional
        .player
        .resistances
        .set(DamageType::Poison, ResistanceLevel::Strong);
    assert!(
        (0..1000).any(|seed| {
            let mut probe = fractional.clone();
            let hp = probe.player.hp;
            probe.rng = RfbRng::seeded(seed);
            probe.process_player_waste_damage(&mut Vec::new()) == Some((DEEP.to_owned(), 0))
                && probe.player.hp == hp
        }),
        "rounding both components to zero still blocks this interval's healing"
    );

    for (tick, scale, invulnerable) in [
        (11, MapScaleDto::Local, false),
        (10, MapScaleDto::World, false),
        (10, MapScaleDto::Local, true),
    ] {
        game.world_tick = tick;
        game.map_scale = scale;
        if invulnerable {
            game.apply_player_melee_status(STATUS_INVULNERABILITY, 1, "test.invulnerability");
        }
        let before = game.rng.clone();
        assert!(game.process_player_waste_damage(&mut Vec::new()).is_none());
        assert_eq!(game.rng, before);
    }
    game.world_tick = 9;
    // Invulnerability expiry itself charges a turn; isolate its last world tick.
    game.player.energy_need = -STANDARD_ACTION_COST;
    let mut events = Vec::new();
    game.advance_until_player_ready(
        false,
        true,
        false,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.world_tick, 10);
    assert!(!game.player_has_status_kind(STATUS_INVULNERABILITY));
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, DomainEvent::PlayerWasteDamaged { .. }))
    );
}

#[test]
fn waste_wait_rest_movement_and_save_share_the_world_clock() {
    let mut waiting = waste_game(SHALLOW);
    waiting
        .player
        .resistances
        .set(DamageType::Acid, ResistanceLevel::Immune);
    waiting.world_tick = 0;
    waiting.player.hp -= 1;
    waiting.player.energy_need = 0;
    waiting.rng = RfbRng::seeded(exposure_seed());
    let mut resting = waiting.clone();
    let mut moving = waiting.clone();
    replace_terrain(&mut moving, Position { x: 100, y: 33 }, SHALLOW);
    dispatch_next(&mut waiting, GameCommand::Wait);
    dispatch_next(&mut resting, GameCommand::Rest { turns: 1 });
    dispatch_next(
        &mut moving,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(
        (waiting.world_tick, resting.world_tick, moving.world_tick),
        (10, 10, 10)
    );
    assert_eq!(waiting.player.statuses, resting.player.statuses);
    assert_eq!(waiting.player.statuses, moving.player.statuses);
    assert!(waiting.player_has_status_kind(STATUS_POISON));
    waiting.reveal_current_visibility();
    let mut restored = Game::from_save(waiting.to_save()).unwrap();
    assert_eq!(restored.state_hash(), waiting.state_hash());
    let left = dispatch_next(&mut waiting, GameCommand::Wait);
    let right = dispatch_next(&mut restored, GameCommand::Wait);
    assert_eq!(left.events, right.events);
    assert_eq!(restored.state_hash(), waiting.state_hash());
}

#[test]
fn waste_armor_corrosion_uses_equipped_armor_and_respects_acid_protection() {
    let mut game = Game::new_with_build(42, RFB_WARRIOR_BUILD_ID).unwrap();
    game.items.clear();
    give_inventory_item(&mut game, "test.armor", "demo.item.soft-leather-armour");
    assert!(game.equip_inventory_item("test.armor", None).is_some());
    let mut protected = game.clone();
    protected.items[0]
        .permanent_destruction_immunities
        .insert(rfb_content::ItemDestructionElement::Acid);
    let defense = game.player_derived_stats().defense.value;
    let mut events = Vec::new();
    assert!(game.corrode_player_armor(&mut events));
    assert_eq!(game.items[0].enchantments.to_armor, -1);
    assert_eq!(game.player_derived_stats().defense.value, defense - 1);
    assert!(protected.corrode_player_armor(&mut Vec::new()));
    assert_eq!(protected.items[0].enchantments.to_armor, 0);
    protected.player.position = START;
    replace_terrain(&mut protected, START, SHALLOW);
    protected.world_tick = 10;
    protected.player.hp = 100;
    protected
        .player
        .resistances
        .set(DamageType::Poison, ResistanceLevel::Immune);
    let mut unarmored = protected.clone();
    unarmored.items.clear();
    let seed = (0..1000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(3) != 0 && {
                rng.bounded(400);
                rng.bounded(100);
                rng.bounded(100);
                rng.bounded(16) == 0
            }
        })
        .unwrap();
    protected.rng = RfbRng::seeded(seed);
    unarmored.rng = RfbRng::seeded(seed);
    protected.process_player_waste_damage(&mut Vec::new());
    unarmored.process_player_waste_damage(&mut Vec::new());
    assert_eq!(
        100 - protected.player.hp,
        (100 - unarmored.player.hp + 1) / 2
    );
    assert_eq!(
        protected.items[0].enchantments.to_armor, 0,
        "acid-proof armor still halves the selected exposure"
    );
    game.items[0].enchantments.to_armor = -100;
    assert!(!game.corrode_player_armor(&mut Vec::new()));
    game.items[0].location = ItemLocation::Inventory;
    let rng = game.rng.clone();
    assert!(!game.corrode_player_armor(&mut Vec::new()));
    assert_eq!(game.rng, rng);
}

#[test]
fn waste_monster_allocation_and_movement_keep_distinct_source_checks() {
    let game = waste_game(DEEP);
    let shallow = game.content.terrain(SHALLOW).unwrap();
    let deep = game.content.terrain(DEEP).unwrap();
    let mut actor = game
        .content
        .actor("demo.actor.small-kobold")
        .unwrap()
        .clone();
    assert!(!actor_can_cross_terrain(&actor, shallow));
    actor
        .resistances
        .insert(ActorDamageType::Poison, ActorResistanceLevel::Immune);
    assert!(actor_matches_allocation_terrain(&actor, shallow));
    assert!(!actor_can_cross_terrain(&actor, shallow));
    actor
        .resistances
        .insert(ActorDamageType::Acid, ActorResistanceLevel::Resistant);
    assert!(actor_can_cross_terrain(&actor, shallow));
    assert!(!actor_can_cross_terrain(&actor, deep));
    actor.movement.modes.push(ActorMovementMode::Swim);
    assert!(actor_can_cross_terrain(&actor, deep));
    actor.movement.modes.push(ActorMovementMode::Aquatic);
    assert!(!actor_can_cross_terrain(&actor, deep), "waste is not water");
    actor.movement.modes.push(ActorMovementMode::Fly);
    actor.resistances.clear();
    assert!(actor_can_cross_terrain(&actor, deep));
    assert!(game.player_can_cross_terrain_unmounted(deep));
    assert!(game.projectile_can_cross(START));
}

#[test]
fn waste_drops_projectiles_and_terrain_conversion_never_use_walkability_as_drop_permission() {
    let mut game = waste_game(DEEP);
    assert!(game.is_walkable(START));
    assert!(!game.terrain_allows_items(START));
    give_inventory_item(&mut game, "test.arrow", "demo.item.arrow");
    let ammunition = game.items.pop().unwrap();
    game.settle_projectile_ammunition(
        ammunition,
        START,
        false,
        0,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    let ItemLocation::Ground(landing) = game.items[0].location else {
        panic!("arrow must land");
    };
    assert_ne!(landing, START);
    assert!(game.terrain_allows_items(landing));
    game.replace_terrain_from_source(
        landing,
        DEEP,
        terrain::TerrainChangeSource::Disintegration,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    let ItemLocation::Ground(next) = game.items[0].location else {
        panic!("arrow must move");
    };
    assert_ne!(next, landing);
    assert!(game.terrain_allows_items(next));
    for y in 29..=37 {
        for x in 95..=103 {
            replace_terrain(&mut game, Position { x, y }, DEEP);
        }
    }
    give_inventory_item(&mut game, "test.drop", "demo.item.iron-shot");
    assert!(
        game.drop_inventory_items(&["test.drop".to_owned()])
            .is_none()
    );
    assert_eq!(game.items.last().unwrap().location, ItemLocation::Inventory);
    assert!(
        game.ground_drop_position(START, true).is_some(),
        "artifact search reaches distant land"
    );
    assert!(game.ground_drop_position(START, false).is_none());
}

#[test]
fn waste_save_rejects_ground_items_and_gold_on_deep_waste() {
    let mut game = waste_game(DEEP);
    give_inventory_item(&mut game, "test.drop", "demo.item.iron-shot");
    game.items[0].location = ItemLocation::Ground(START);
    assert!(matches!(
        Game::from_save(game.to_save()),
        Err(CoreError::InvalidSave("item state is invalid"))
    ));
    replace_terrain(&mut game, START, SHALLOW);
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    game.items.clear();
    let pile = game.generate_gold_pile(START, 1, false).unwrap();
    game.gold_piles.push(pile);
    replace_terrain(&mut game, START, DEEP);
    assert!(matches!(
        Game::from_save(game.to_save()),
        Err(CoreError::InvalidSave("gold pile state is invalid"))
    ));
}

#[test]
fn waste_exposure_blocks_recovery_and_poison_save_controls_constitution_loss() {
    let mut base = waste_game(SHALLOW);
    base.player.hp = 100;
    let before = base.progress.attributes.constitution;
    let seed = (0..2_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            game.process_player_waste_damage(&mut Vec::new());
            game.progress.attributes.constitution < before
        })
        .expect("the source 1/32 branch must be reachable");
    let mut resisted = base.clone();
    resisted
        .player
        .resistances
        .set(DamageType::Poison, ResistanceLevel::Strong);
    resisted.rng = RfbRng::seeded(seed);
    resisted.process_player_waste_damage(&mut Vec::new());
    assert_eq!(resisted.progress.attributes.constitution, before);

    let mut game = waste_game(SHALLOW);
    give_inventory_item(&mut game, "test.regeneration", "demo.item.slayer");
    assert!(
        game.equip_inventory_item("test.regeneration", None)
            .is_some()
    );
    game.player.hp = game.effective_player_max_hp() - 5;
    game.player
        .resistances
        .set(DamageType::Poison, ResistanceLevel::Immune);
    game.world_tick = 9;
    game.rng = RfbRng::seeded(exposure_seed());
    let hp = game.player.hp;
    let events = tick(&mut game);
    let acid = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::PlayerWasteDamaged { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .unwrap();
    assert_eq!(game.player.hp, hp - acid);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::EquipmentRegenerated { .. }))
    );
    game.player
        .resistances
        .set(DamageType::Acid, ResistanceLevel::Immune);
    game.world_tick = 19;
    game.rng = RfbRng::seeded(exposure_seed());
    let hp = game.player.hp;
    let events = tick(&mut game);
    assert!(game.player.hp > hp);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::EquipmentRegenerated { .. }))
    );
}

#[test]
fn waste_flight_uses_the_mounts_effective_movement() {
    let mut game = waste_game(SHALLOW);
    game.apply_player_melee_status(STATUS_LEVITATION, 100, "test.flight");
    game.push_generated_actor("test.mount".to_owned(), "demo.actor.horse", START);
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.mount".to_owned());
    game.rng = RfbRng::seeded(exposure_seed());
    assert!(
        game.process_player_waste_damage(&mut Vec::new()).is_some(),
        "a grounded mount overrides the rider's levitation"
    );
    game.player.statuses.clear();
    game.entities.clear();
    game.push_generated_actor(
        "test.mount".to_owned(),
        "demo.actor.ancient-black-dragon",
        START,
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.rng = RfbRng::seeded(exposure_seed());
    assert!(game.process_player_waste_damage(&mut Vec::new()).is_none());
    replace_terrain(&mut game, START, DEEP);
    game.rng = RfbRng::seeded(exposure_seed());
    assert!(game.process_player_waste_damage(&mut Vec::new()).is_some());
    assert_eq!(
        game.entities[0].hp, game.entities[0].max_hp,
        "player exposure does not poison every monster"
    );
}

#[test]
fn waste_created_food_finds_shore_or_is_lost_without_panicking() {
    let mut game = waste_game(DEEP);
    let ability = game
        .content
        .ability("rfb.ability.race.create-food")
        .unwrap()
        .clone();
    let mut events = Vec::new();
    game.resolve_player_ability_effect(
        ability.clone(),
        AbilityTargetPlan::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.items.len(), 1);
    let ItemLocation::Ground(position) = game.items[0].location else {
        panic!("created food must land");
    };
    assert_ne!(position, START);
    assert!(game.terrain_allows_items(position));
    game.items.clear();
    for y in 29..=37 {
        for x in 95..=103 {
            replace_terrain(&mut game, Position { x, y }, DEEP);
        }
    }
    events.clear();
    game.resolve_player_ability_effect(
        ability,
        AbilityTargetPlan::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.items.is_empty());
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::ItemDestroyed { quantity: 1, .. }))
    );
}

#[test]
fn waste_generated_river_keeps_items_gold_and_monsters_on_legal_tiles() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    let floor = artifact.content.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .unwrap();
    floor.layout.as_mut().unwrap().river = Some(rfb_content::ProceduralRiverDefinition {
        rfb_depth_chance: false,
        deep_terrain_id: DEEP.to_owned(),
        shallow_terrain_id: SHALLOW.to_owned(),
        chance_one_in: None,
        alternative: None,
    });
    floor.generation_budget.as_mut().unwrap().river_area_tiles = Some(160);
    let content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    let base =
        Game::from_content_with_build(42, content, DEFAULT_WORLD_ID, RFB_WARRIOR_BUILD_ID).unwrap();
    let definition = base
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    for seed in [0, 1, 7, 42] {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let floor = game.generate_procedural_floor(&definition, None).unwrap();
        assert!(floor.terrain.iter().any(|id| id == DEEP));
        assert!(floor.terrain.iter().any(|id| id == SHALLOW));
        let at = |position: Position| {
            game.content
                .terrain(
                    &floor.terrain
                        [position.y as usize * usize::from(floor.width) + position.x as usize],
                )
                .unwrap()
        };
        assert!(!floor.items.is_empty());
        for item in &floor.items {
            if let ItemLocation::Ground(position) = item.location {
                assert!(at(position).allows_items());
            }
        }
        assert!(
            floor
                .gold_piles
                .iter()
                .all(|pile| at(pile.position).allows_items())
        );
        assert!(floor.entities.iter().all(|actor| actor_can_cross_terrain(
            game.content.actor(&actor.kind_id).unwrap(),
            at(actor.position)
        )));
    }
}

#[test]
fn waste_monster_carried_loot_and_guardian_rewards_use_legal_drop_search() {
    let mut game = waste_game(DEEP);
    game.push_generated_actor("test.bat".to_owned(), "demo.actor.fruit-bat", START);
    give_inventory_item(&mut game, "test.loot", "demo.item.iron-shot");
    game.items[0].location = ItemLocation::CarriedBy {
        actor_id: "test.bat".to_owned(),
    };
    game.resolve_actor_death(
        0,
        DomainEvent::PlayerSlew {
            target_kind_id: "demo.actor.fruit-bat".to_owned(),
            damage: resolve_damage(
                DamagePacket::new(1, DamageType::Physical),
                ResistanceLevel::Normal,
            ),
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.items.iter().any(|item| item.id == "test.loot"));
    for item in &game.items {
        if let ItemLocation::Ground(position) = item.location {
            assert!(game.terrain_allows_items(position));
        }
    }
    let definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| {
            floor
                .dungeon_id
                .as_deref()
                .is_some_and(|id| game.dungeon_is_active(id))
                && floor
                    .guardian
                    .as_ref()
                    .is_some_and(|guardian| guardian.reward_artifact_item_kind_id.is_some())
        })
        .unwrap()
        .clone();
    let guardian = definition.guardian.unwrap();
    let reward_kind = guardian.reward_artifact_item_kind_id.unwrap();
    game.current_floor_id = definition.id;
    let mut actor = game.player.clone();
    actor.id = guardian.instance_id;
    actor.kind_id = guardian.actor_kind_id;
    actor.position = START;
    for y in 29..=37 {
        for x in 95..=103 {
            replace_terrain(&mut game, Position { x, y }, DEEP);
        }
    }
    let mut no_land = game.clone();
    no_land.terrain.fill(DEEP.to_owned());
    let (lost, _) = no_land.generate_death_loot(&actor).unwrap();
    assert!(!lost.iter().any(|item| item.kind_id == reward_kind));
    let (reward, _) = game.generate_death_loot(&actor).unwrap();
    let item = reward
        .iter()
        .find(|item| item.kind_id == reward_kind)
        .unwrap();
    let ItemLocation::Ground(position) = item.location else {
        panic!("reward must land");
    };
    assert!(game.terrain_allows_items(position));
    assert!(position.x.abs_diff(START.x) > 3 || position.y.abs_diff(START.y) > 3);
}

#[test]
fn waste_stored_floor_items_use_the_same_save_boundary() {
    let mut game = Game::new_with_build(42, RFB_WARRIOR_BUILD_ID).unwrap();
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    let floor_id = game.current_floor_id.clone();
    let position = game.player.position;
    give_inventory_item(&mut game, "test.stored-drop", "demo.item.iron-shot");
    game.items[0].location = ItemLocation::Ground(position);
    descend_one_floor(&mut game);
    game.reveal_current_visibility();
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
    let floor = game
        .stored_floors
        .values_mut()
        .find(|floor| floor.id == floor_id)
        .unwrap();
    floor.terrain[position.y as usize * usize::from(floor.width) + position.x as usize] =
        DEEP.to_owned();
    assert!(matches!(
        Game::from_save(game.to_save()),
        Err(CoreError::InvalidSave("stored floor item state is invalid"))
    ));
}

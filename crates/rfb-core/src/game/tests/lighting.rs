// SPDX-License-Identifier: MPL-2.0

use super::support::*;
use super::*;
use crate::game::lighting::{DUNGEON_AMBIENT_LIGHT, SURFACE_AMBIENT_LIGHT};
use crate::game::{
    gold::starting_gold, hunger::starting_food_supply, lighting::starting_torch_supply,
};
use rfb_protocol::{ItemFuelDto, ItemFuelKindDto};

const TORCH_KIND_ID: &str = "demo.item.wooden-torch";
const LANTERN_KIND_ID: &str = "demo.item.brass-lantern";
const OIL_KIND_ID: &str = "demo.item.flask-of-oil";

fn dark_cave_game() -> Game {
    static CONTENT: std::sync::OnceLock<Arc<ContentCatalog>> = std::sync::OnceLock::new();
    let content = CONTENT.get_or_init(|| {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
        let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
        artifact.content.worlds[0]
            .dungeons
            .iter_mut()
            .find(|d| d.id == "demo.dungeon.warrens")
            .unwrap()
            .darkness = true;
        Arc::new(ContentCatalog::from_artifact(
            rfb_content::encode_content(artifact.content).unwrap(),
        ))
    });
    let mut game =
        Game::from_content_with_build(42, content.clone(), DEFAULT_WORLD_ID, RFB_WARRIOR_BUILD_ID)
            .unwrap();
    descend_one_floor(&mut game);
    assert!(game.dungeon_has_darkness());
    game
}

fn dark_cave_room() -> Game {
    let mut game = dark_cave_game();
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=12 {
        for x in 8..=20 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game
}

#[test]
fn dark_cave_generated_glow_light_cap_and_return_survive_save() {
    let mut game = dark_cave_game();
    assert!(game.glow.iter().all(|glow| !glow));
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.lantern".to_owned(),
            slot_id: None,
        },
    );
    assert_eq!(game.player_light_radius(), Some(1));
    let position = game.player.position;
    assert!(!game.set_floor_glow_at(position, true));
    let hash = game.state_hash();
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), hash);
    assert_eq!(restored.player_light_radius(), Some(1));
    assert!(restored.glow.iter().all(|glow| !glow));
    clear_monsters(&mut restored);
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    restored.traverse_stairs(false).unwrap().unwrap();
    assert!(!restored.dungeon_has_darkness());
    assert_eq!(restored.player_light_radius(), Some(2));
    descend_one_floor(&mut restored);
    assert!(restored.dungeon_has_darkness());
    assert_eq!(restored.player_light_radius(), Some(1));
    assert!(restored.glow.iter().all(|glow| !glow));
}

#[test]
fn dark_cave_caps_positive_monster_light_preserves_negative_light_and_night_vision() {
    let mut game = dark_cave_room();
    let source = Position { x: 12, y: 10 };
    let positive = game
        .content
        .actor_definitions()
        .find(|a| a.light.is_some_and(|l| !l.darkness && l.radius >= 2))
        .unwrap()
        .id
        .clone();
    game.push_generated_actor("test.light".to_owned(), &positive, source);
    game.entities[0].statuses.clear();
    assert!(game.position_is_lit(Position { x: 13, y: 10 }));
    assert!(!game.position_is_lit(Position { x: 14, y: 10 }));
    game.entities.clear();
    let negative = game
        .content
        .actor_definitions()
        .find(|a| a.light.is_some_and(|l| l.darkness && l.radius >= 2))
        .unwrap()
        .id
        .clone();
    game.push_generated_actor("test.dark".to_owned(), &negative, source);
    game.entities[0].statuses.clear();
    let edge = Position { x: 14, y: 10 };
    let edge_index = game.index(edge).unwrap();
    game.glow[edge_index] = true;
    assert!(!game.position_is_lit(edge));
    game.entities[0].position = Position { x: 16, y: 10 };
    assert!(
        game.position_is_lit(edge),
        "distant monster darkness is not processed"
    );
    give_inventory_item(&mut game, "test.night-vision", "demo.item.cloak");
    let cloak = game
        .items
        .iter_mut()
        .find(|i| i.id == "test.night-vision")
        .unwrap();
    cloak.location = ItemLocation::Equipped {
        slot_id: "cloak".to_owned(),
    };
    cloak
        .intrinsic_properties
        .passives
        .insert(rfb_content::EquipmentPassive::NightVision);
    assert!(
        !game.position_is_lit(edge),
        "night vision retains distant monster sources"
    );
}

#[test]
fn dark_cave_visual_sight_limits_infra_but_not_esp_or_night_vision() {
    let mut game = dark_cave_room();
    let near = Position { x: 14, y: 10 };
    let far = Position { x: 15, y: 10 };
    game.push_generated_actor("test.near".to_owned(), "demo.actor.newt", near);
    game.push_generated_actor("test.far".to_owned(), "demo.actor.newt", far);
    give_inventory_item(&mut game, "test.senses", "demo.item.cloak");
    let cloak = game
        .items
        .iter_mut()
        .find(|i| i.id == "test.senses")
        .unwrap();
    cloak.location = ItemLocation::Equipped {
        slot_id: "cloak".to_owned(),
    };
    cloak.intrinsic_properties.equipment_bonuses.infravision = 8;
    assert!(game.entity_is_visually_visible_to_player(&game.entities[0]));
    assert!(!game.entity_is_visually_visible_to_player(&game.entities[1]));
    assert!(!game.is_visible(far), "infravision does not reveal terrain");
    game.player.statuses.push(
        crate::game::monster_combat::melee_status("rfb.status.telepathy", 20, "test.senses").status,
    );
    assert!(game.entity_is_visible_by_telepathy(&game.entities[1]));
    assert!(game.entity_is_fuzzy_to_player(&game.entities[1]));
    game.items[0]
        .intrinsic_properties
        .passives
        .insert(rfb_content::EquipmentPassive::NightVision);
    assert!(game.entity_is_visually_visible_to_player(&game.entities[1]));
    assert!(game.is_visible(far));
    assert!(!game.position_is_lit(far));
    game.player.statuses.push(
        crate::game::monster_combat::melee_status("rfb.status.blindness", 20, "test.senses").status,
    );
    assert!(!game.entity_is_visually_visible_to_player(&game.entities[1]));
    assert!(game.entity_is_visible_by_telepathy(&game.entities[1]));
}

#[test]
fn dark_cave_detection_is_reduced_once_for_abilities_scrolls_and_mapping() {
    let mut game = dark_cave_room();
    for (id, x) in [("test.near", 12), ("test.far", 13)] {
        game.push_generated_actor(id.to_owned(), "demo.actor.newt", Position { x, y: 10 });
        game.entities.last_mut().unwrap().energy_need = 10000;
    }
    let mut ability = game
        .content
        .ability("demo.ability.arcane-light-area")
        .unwrap()
        .clone();
    ability.effect = AbilityEffectDefinition::Detect {
        subject: rfb_content::AbilityDetectSubjectDefinition::Actor,
        category: "normal-monster".to_owned(),
        radius: 8,
        persistent: false,
        through_walls: true,
    };
    let mut events = Vec::new();
    game.resolve_player_detection_effect(&ability, &mut events, &mut BTreeSet::new());
    let DomainEvent::AbilityDetected { resolution, .. } = &events[0] else {
        panic!("detection event")
    };
    assert_eq!(resolution.radius, 2);
    assert_eq!(resolution.detected_entity_ids, ["test.near"]);
    give_inventory_item(&mut game, "test.detect", "demo.item.detect-monsters-scroll");
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.lantern");
    let before = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.detect".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    let detection = update
        .events
        .iter()
        .find_map(|e| match &e.outcome {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .unwrap();
    assert_eq!(detection.radius, 2);
    assert_eq!(detection.detected_entity_ids, ["test.near"]);
    assert!(!game.items.iter().any(|i| i.id == "test.detect"));
    assert!(game.world_tick > before);
    game.explored.fill(false);
    let mapped = game.detect_terrain_positions("map", 8, true, true);
    assert!(mapped.contains(&Position { x: 12, y: 10 }));
    assert!(!mapped.contains(&Position { x: 13, y: 10 }));
    assert!(!game.glow.iter().any(|g| *g));
    assert_eq!(game.dungeon_detection_radius(2), 0);
    for (id, x) in [("test.near-item", 12), ("test.far-item", 13)] {
        give_inventory_item(&mut game, id, "demo.item.dagger");
        game.items.iter_mut().find(|i| i.id == id).unwrap().location =
            ItemLocation::Ground(Position { x, y: 10 });
        game.gold_piles.push(crate::state::GoldPile {
            id: format!("{id}.gold"),
            position: Position { x, y: 10 },
            amount: 10,
            appearance: rfb_protocol::GoldAppearanceDto::Gold,
            discovered: false,
        });
    }
    assert_eq!(
        game.detect_item_positions("item", 8, true).1,
        ["test.near-item"]
    );
    assert_eq!(
        game.detect_gold_positions(8, true).1,
        ["test.near-item.gold"]
    );
    game.explored.fill(false);
    assert_eq!(
        game.detect_terrain_positions("map", 2, false, true),
        [game.player.position]
    );
}

#[test]
fn dark_cave_successful_book_cast_spends_mana_and_a_turn_even_when_light_is_absorbed() {
    let mut game = Game::from_content_with_build(
        42,
        dark_cave_game().content,
        DEFAULT_WORLD_ID,
        "demo.build.high-mage-arcane",
    )
    .unwrap();
    game.progress.level = 10;
    game.progress.max_level = 10;
    game.progress.experience = game.experience_required_for_level(10);
    game.progress.maximum_experience = game.progress.experience;
    game.ability_learning_order
        .push("demo.ability.arcane-light-area".to_owned());
    game.refresh_player_resource_maxima();
    let mana = game.resources.get_mut("demo.resource.mana").unwrap();
    mana.current = mana.maximum;
    game.debug_ability_casts_succeed = true;
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.lantern");
    let before = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.arcane-light-area".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    let cast = update
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::AbilityCast { resolution }) => Some(resolution),
            _ => None,
        })
        .unwrap();
    assert!(cast.succeeded);
    assert!(cast.resource_paid > 0);
    assert!(game.world_tick > before);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "dungeon.darkness-absorbed-light")
    );
    assert!(game.glow.iter().all(|glow| !glow));
}

#[test]
fn dark_cave_blocks_area_projection_but_retains_daylight_backlash_and_light_ray_damage() {
    let mut game = dark_cave_room();
    let target = Position { x: 11, y: 10 };
    game.push_generated_actor("test.target".to_owned(), "demo.actor.newt", target);
    game.entities[0].hp = 100;
    game.entities[0].max_hp = 100;
    game.entities[0]
        .resistances
        .set(DamageType::Light, ResistanceLevel::Vulnerable);
    let mut ability = game
        .content
        .ability("demo.ability.nature-daylight")
        .unwrap()
        .clone();
    ability.effect = AbilityEffectDefinition::LightArea {
        damage_dice: 2,
        damage_sides: 2,
        radius: 2,
        sunlight_burn_damage_dice: 2,
        sunlight_burn_damage_sides: 2,
    };
    game.player
        .statuses
        .push(race_form_status("demo.race.vampire-lord"));
    let hp = game.player.hp;
    let mut events = Vec::new();
    game.resolve_player_ability_effect(
        ability.clone(),
        AbilityTargetPlan::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.entities[0].hp, 100);
    assert!((2..=4).contains(&(hp - game.player.hp)));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::DungeonDarknessAbsorbedLight))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityAreaDamage { .. }))
    );
    assert!(game.glow.iter().all(|g| !g));
    ability.effect = AbilityEffectDefinition::LightLine {
        damage_dice: 2,
        damage_sides: 2,
    };
    game.resolve_player_ability_effect(
        ability,
        AbilityTargetPlan::Projectile {
            path: vec![target],
            stop_at_actor: false,
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.entities[0].hp < 100);
    assert!(game.glow.iter().all(|g| !g));
}

#[test]
fn dark_cave_full_revelation_keeps_knowledge_without_glow_and_light_scroll_spends_turn() {
    let mut game = dark_cave_room();
    let ability = game
        .content
        .ability("demo.ability.nature-call-sunlight")
        .unwrap()
        .clone();
    let mut events = Vec::new();
    game.resolve_player_ability_effect(
        ability,
        AbilityTargetPlan::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.explored.iter().all(|e| *e));
    assert!(game.glow.iter().all(|g| !g));
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.lantern");
    give_inventory_item(&mut game, "test.light", "demo.item.light-scroll");
    let before = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.light".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert!(!game.items.iter().any(|i| i.id == "test.light"));
    assert!(game.world_tick > before);
    assert!(
        update
            .events
            .iter()
            .any(|e| e.kind == "dungeon.darkness-absorbed-light")
    );
    assert!(game.glow.iter().all(|g| !g));
}

fn intrinsic_see_invisible_game(seed: u64) -> Game {
    Game::new_with_build_race_and_name(
        seed,
        RFB_WARRIOR_BUILD_ID,
        "rfb-legacy.race.high-elf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal High-Elf game should create")
}

fn race_form_status(race_id: &str) -> StatusInstance {
    StatusInstance {
        kind_id: STATUS_PLAYER_POLYMORPH.to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.race-form".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: Some(race_id.to_owned()),
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }
}

fn set_inventory_light_equipped(game: &mut Game, item_id: &str) {
    game.items
        .iter_mut()
        .find(|item| item.id == item_id)
        .expect("test light should exist")
        .location = ItemLocation::Equipped {
        slot_id: "light".to_owned(),
    };
}

fn set_fuel(game: &mut Game, item_id: &str, current: u16) {
    game.items
        .iter_mut()
        .find(|item| item.id == item_id)
        .and_then(|item| item.fuel.as_mut())
        .expect("test item should have fuel")
        .current = current;
}

fn fuel(game: &Game, item_id: &str) -> ItemFuelDto {
    game.items
        .iter()
        .find(|item| item.id == item_id)
        .and_then(|item| item.fuel)
        .expect("test item should have fuel")
}

#[test]
fn warrior_birth_rolls_three_to_seven_matching_torches_after_food() {
    let content = Game::new(0).content;
    for seed in 0..32 {
        let build = CharacterBuildIdentity {
            build_id: RFB_WARRIOR_BUILD_ID.to_owned(),
            race_id: "demo.race.rfb-human".to_owned(),
            class_id: "demo.class.warrior".to_owned(),
            personality_id: "demo.personality.ordinary".to_owned(),
        };
        let mut expected_rng = RfbRng::seeded(seed);
        let _ = crate::game::virtues::initial_virtues(&content, Some(&build), &mut expected_rng);
        let _ = starting_gold(Some(&build), &mut expected_rng);
        let _ = starting_food_supply(Some(&build), &mut expected_rng);
        let expected = starting_torch_supply(Some(&build), &mut expected_rng)
            .expect("Warrior should receive birth torches");
        let shop_draws_before = expected_rng.draw_counter;

        let game = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let torches = game
            .items
            .iter()
            .filter(|item| {
                item.kind_id == TORCH_KIND_ID && item.location == ItemLocation::Inventory
            })
            .collect::<Vec<_>>();

        assert_eq!(torches.len(), usize::try_from(expected.quantity).unwrap());
        assert!((3..=7).contains(&torches.len()));
        assert!(torches.iter().all(|torch| {
            torch.quantity == 1
                && torch.fuel
                    == Some(ItemFuelDto {
                        kind: ItemFuelKindDto::Torch,
                        current: expected.fuel,
                        maximum: 5_000,
                        light_radius: 1,
                    })
        }));
        assert!((1_500..=3_500).contains(&expected.fuel));
        assert_eq!(expected.fuel % 500, 0);
        assert!(game.rng_draw_counter() > shop_draws_before);
    }
}

#[test]
fn fuel_items_start_with_original_capacity_weight_and_radius() {
    let mut game = Game::new(42);
    give_inventory_item(&mut game, "test.torch", TORCH_KIND_ID);
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    give_inventory_item(&mut game, "test.oil", OIL_KIND_ID);

    assert_eq!(
        fuel(&game, "test.torch"),
        ItemFuelDto {
            kind: ItemFuelKindDto::Torch,
            current: 4_000,
            maximum: 5_000,
            light_radius: 1,
        }
    );
    assert_eq!(
        fuel(&game, "test.lantern"),
        ItemFuelDto {
            kind: ItemFuelKindDto::Lantern,
            current: 7_500,
            maximum: 15_000,
            light_radius: 2,
        }
    );
    assert_eq!(
        fuel(&game, "test.oil"),
        ItemFuelDto {
            kind: ItemFuelKindDto::Oil,
            current: 7_500,
            maximum: 7_500,
            light_radius: 0,
        }
    );
    assert_eq!(game.item_weight_tenths_pound(TORCH_KIND_ID), 30);
    assert_eq!(game.item_weight_tenths_pound(LANTERN_KIND_ID), 50);
    assert_eq!(game.item_weight_tenths_pound(OIL_KIND_ID), 10);
}

#[test]
fn torch_refuel_consumes_a_torch_adds_source_fuel_plus_five_and_costs_fifty_energy() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.target", TORCH_KIND_ID);
    give_inventory_item(&mut game, "test.source", TORCH_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.target");
    set_fuel(&mut game, "test.target", 1_000);
    set_fuel(&mut game, "test.source", 2_000);
    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::RefuelLight {
            target_item_id: "test.target".to_owned(),
            source_item_id: "test.source".to_owned(),
        },
    );

    assert_eq!(
        GameAction::RefuelLight {
            target_item_id: String::new(),
            source_item_id: String::new(),
        }
        .energy_cost(),
        50
    );
    assert_eq!(game.world_tick - tick_before, 5);
    assert_eq!(fuel(&game, "test.target").current, 3_005);
    assert!(game.items.iter().all(|item| item.id != "test.source"));
    assert_eq!(game.rng_draw_counter(), draws_before);
    let event = update
        .events
        .iter()
        .find(|event| event.kind == "light.refueled")
        .expect("refueling should report the applied amount");
    assert_eq!(event.args["amount"], "2005");
    assert_eq!(event.args["current"], "3005");
    assert_eq!(event.args["maximum"], "5000");
}

#[test]
fn lantern_accepts_oil_or_another_lantern_and_caps_after_consuming_the_source() {
    let mut oil_game = Game::new(42);
    give_inventory_item(&mut oil_game, "test.target", LANTERN_KIND_ID);
    give_inventory_item(&mut oil_game, "test.oil", OIL_KIND_ID);
    set_inventory_light_equipped(&mut oil_game, "test.target");
    set_fuel(&mut oil_game, "test.target", 1_000);
    let oil = oil_game
        .refuel_equipped_light("test.target", "test.oil")
        .expect("oil should refuel a lantern");
    assert_eq!(
        (oil.amount, oil.current, oil.maximum),
        (7_500, 8_500, 15_000)
    );
    assert!(oil_game.items.iter().all(|item| item.id != "test.oil"));

    give_inventory_item(&mut oil_game, "test.source-lantern", LANTERN_KIND_ID);
    set_fuel(&mut oil_game, "test.target", 14_900);
    set_fuel(&mut oil_game, "test.source-lantern", 1_000);
    let lantern = oil_game
        .refuel_equipped_light("test.target", "test.source-lantern")
        .expect("another lantern should refuel a lantern");
    assert_eq!((lantern.amount, lantern.current), (100, 15_000));
    assert!(
        oil_game
            .items
            .iter()
            .all(|item| item.id != "test.source-lantern")
    );
}

#[test]
fn unavailable_refuel_is_zero_world_time_rng_and_item_mutation() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.target", TORCH_KIND_ID);
    give_inventory_item(&mut game, "test.oil", OIL_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.target");
    set_fuel(&mut game, "test.target", 1_000);
    let items_before = game.items.clone();
    let tick_before = game.world_tick;
    let energy_before = game.player.energy_need;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::RefuelLight {
            target_item_id: "test.target".to_owned(),
            source_item_id: "test.oil".to_owned(),
        },
    );

    assert_eq!(game.items, items_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.player.energy_need, energy_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    let unavailable = update
        .events
        .iter()
        .find(|event| event.kind == "light.refuel-unavailable")
        .expect("incompatible fuel should be reported");
    assert_eq!(unavailable.args["reason"], "source-incompatible");
}

#[test]
fn equipped_light_spends_one_fuel_per_ten_ticks_and_reports_extinction() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.torch", TORCH_KIND_ID);
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.torch".to_owned(),
            slot_id: None,
        },
    );
    set_fuel(&mut game, "test.torch", 1);
    let tick_before = game.world_tick;

    let update = dispatch_next(&mut game, GameCommand::Wait);

    assert_eq!(game.world_tick - tick_before, 10);
    assert_eq!(fuel(&game, "test.torch").current, 0);
    assert!(update.events.iter().any(|event| {
        event.kind == "light.extinguished" && event.args["targetItem"] == "test.torch"
    }));
    let restored = Game::from_save(game.to_save()).expect("spent light fuel should reload");
    assert_eq!(fuel(&restored, "test.torch").current, 0);
    assert_eq!(restored.to_save(), game.to_save());
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn nightcap_reduces_carried_light_preserves_glow_and_senses_only_undead_after_save() {
    let mut game = Game::new_with_build(425, RFB_WARRIOR_BUILD_ID).unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=12 {
        for x in 8..=14 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.glow.fill(false);
    let adjacent = Position { x: 11, y: 10 };
    let edge = Position { x: 12, y: 10 };
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    game.equip_inventory_item("test.lantern", None).unwrap();
    assert_eq!(game.player_light_radius(), Some(2));
    assert!(game.is_visible(edge));
    // Generation is covered in A2's item test; this fixture isolates consumers.
    give_inventory_item(&mut game, "test.nightcap", "demo.item.nightcap");
    game.register_generated_artifact("demo.item.nightcap");
    game.equip_inventory_item("test.nightcap", None).unwrap();
    assert_eq!(game.player_light_radius(), Some(1));
    assert!(game.is_visible(adjacent));
    assert!(!game.is_visible(edge));
    let edge_index = game.index(edge).unwrap();
    game.glow[edge_index] = true;
    assert!(
        game.is_visible(edge),
        "darkness equipment does not erase permanent glow"
    );
    game.glow[edge_index] = false;
    replace_terrain(&mut game, edge, "demo.terrain.wall");
    game.push_generated_actor(
        "test.undead".into(),
        "demo.actor.skeleton-human",
        Position { x: 13, y: 10 },
    );
    game.push_generated_actor(
        "test.living".into(),
        "demo.actor.sheep",
        Position { x: 14, y: 10 },
    );
    assert!(game.entity_is_visible_by_telepathy(&game.entities[0]));
    assert!(game.entity_is_fuzzy_to_player(&game.entities[0]));
    assert!(!game.entity_is_visible_to_player(&game.entities[1]));
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.player_light_radius(), Some(1));
    assert!(
        restored.entity_is_visible_by_telepathy(
            restored
                .entities
                .iter()
                .find(|actor| actor.id == "test.undead")
                .unwrap()
        )
    );
    restored.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
    assert_eq!(restored.player_light_radius(), Some(1));
    let head = match &restored
        .items
        .iter()
        .find(|item| item.id == "test.nightcap")
        .unwrap()
        .location
    {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => panic!("nightcap must remain equipped"),
    };
    restored.unequip_slot(&head).unwrap();
    assert_eq!(restored.player_light_radius(), Some(2));
    assert!(
        !restored.entity_is_visible_by_telepathy(
            restored
                .entities
                .iter()
                .find(|actor| actor.id == "test.undead")
                .unwrap()
        )
    );
    give_inventory_item(&mut restored, "test.four-winds", "demo.item.four-winds");
    restored.register_generated_artifact("demo.item.four-winds");
    restored
        .equip_inventory_item("test.four-winds", None)
        .unwrap();
    assert_eq!(
        restored.player_light_radius(),
        Some(2),
        "same base does not imply darkness"
    );
}

#[test]
fn surface_is_ambient_lit_and_dungeon_visibility_follows_equipped_light_radius() {
    let mut game =
        Game::new_with_build(42, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    let surface_neighbor = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, surface_neighbor, "demo.terrain.surface-grass");
    let surface = visual_at(&game.snapshot(), surface_neighbor);
    assert_eq!(surface.visibility, VisibilityState::Visible);
    assert_eq!(surface.light.intensity, SURFACE_AMBIENT_LIGHT);

    descend_one_floor(&mut game);
    game.entities.clear();
    game.glow.fill(false);
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=13 {
        for x in 8..=13 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.explored.fill(false);
    for item in &mut game.items {
        if matches!(item.location, ItemLocation::Equipped { .. }) {
            item.location = ItemLocation::Inventory;
        }
    }
    let adjacent = Position { x: 11, y: 10 };
    let adjacent_diagonal = Position { x: 11, y: 11 };
    let distance_two = Position { x: 12, y: 10 };
    let lantern_edge = Position { x: 12, y: 11 };
    let lantern_corner = Position { x: 12, y: 12 };
    let distance_three = Position { x: 13, y: 10 };
    assert_eq!(
        visual_at(&game.snapshot(), adjacent).visibility,
        VisibilityState::Hidden
    );

    let torch_id = game
        .items
        .iter()
        .find(|item| item.kind_id == TORCH_KIND_ID)
        .expect("Warrior should carry torches")
        .id
        .clone();
    set_inventory_light_equipped(&mut game, &torch_id);
    assert_eq!(
        visual_at(&game.snapshot(), adjacent).visibility,
        VisibilityState::Visible
    );
    let torch_diagonal = visual_at(&game.snapshot(), adjacent_diagonal);
    assert_eq!(torch_diagonal.visibility, VisibilityState::Visible);
    assert!(torch_diagonal.light.intensity > DUNGEON_AMBIENT_LIGHT);
    assert_eq!(
        visual_at(&game.snapshot(), distance_two).visibility,
        VisibilityState::Hidden
    );

    game.items
        .iter_mut()
        .find(|item| item.id == torch_id)
        .expect("torch should remain available")
        .location = ItemLocation::Inventory;
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.lantern");
    assert_eq!(
        visual_at(&game.snapshot(), distance_two).visibility,
        VisibilityState::Visible
    );
    let lantern_outer_band = visual_at(&game.snapshot(), lantern_edge);
    assert_eq!(lantern_outer_band.visibility, VisibilityState::Visible);
    assert!(lantern_outer_band.light.intensity > DUNGEON_AMBIENT_LIGHT);
    assert_eq!(
        visual_at(&game.snapshot(), lantern_corner).visibility,
        VisibilityState::Hidden
    );
    assert_eq!(
        visual_at(&game.snapshot(), distance_three).visibility,
        VisibilityState::Hidden
    );
}

#[test]
fn infravision_does_not_reveal_cold_blooded_monsters() {
    let mut game =
        Game::new_with_build(43, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.glow.fill(false);
    game.player.position = Position { x: 10, y: 10 };
    for item in &mut game.items {
        if matches!(item.location, ItemLocation::Equipped { .. }) {
            item.location = ItemLocation::Inventory;
        }
    }
    assert!(game.gain_mutation("rfb.mutation.infravision", &mut Vec::new()));
    let warm_position = Position { x: 11, y: 10 };
    let cold_position = Position { x: 10, y: 11 };
    for position in [game.player.position, warm_position, cold_position] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor("test.warm".to_owned(), "demo.actor.newt", warm_position);
    game.push_generated_actor("test.cold".to_owned(), "demo.actor.ghast", cold_position);

    let snapshot = game.snapshot();
    assert!(
        snapshot
            .entities
            .iter()
            .any(|actor| actor.id == "test.warm")
    );
    assert!(
        snapshot
            .entities
            .iter()
            .all(|actor| actor.id != "test.cold")
    );
}

#[test]
fn invisible_actors_are_hidden_until_detected_and_detection_round_trips() {
    let mut game = Game::new(12);
    clear_monsters(&mut game);
    let player_position = Position { x: 3, y: 3 };
    game.player.position = player_position;
    let target = Position { x: 4, y: 3 };
    replace_terrain(&mut game, player_position, "demo.terrain.floor");
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.push_generated_actor(
        "test.invisible".to_owned(),
        "demo.actor.clear-icky-thing",
        target,
    );

    let hidden = game.snapshot();
    assert!(
        hidden
            .entities
            .iter()
            .all(|actor| actor.id != "test.invisible")
    );
    assert!(
        hidden
            .cells
            .iter()
            .any(|cell| { cell.position == target && cell.actor_id.is_none() })
    );

    game.entities[0].visible_invisible = true;
    let detected = game.snapshot();
    assert!(
        detected
            .entities
            .iter()
            .any(|actor| actor.id == "test.invisible")
    );
    assert!(detected.cells.iter().any(|cell| {
        cell.position == target && cell.actor_id.as_deref() == Some("test.invisible")
    }));

    let restored = Game::from_save(game.to_save()).expect("invisible detection should reload");
    assert!(restored.entities[0].visible_invisible);
    assert!(
        restored
            .snapshot()
            .entities
            .iter()
            .any(|actor| actor.id == "test.invisible")
    );
}

#[test]
fn intrinsic_race_see_invisible_stacks_and_follows_the_current_form() {
    let mut game = intrinsic_see_invisible_game(47);
    assert_eq!(game.player_see_invisible_sources(), 1);

    for item in &mut game.items {
        if matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "weapon") {
            item.location = ItemLocation::Inventory;
        }
    }
    give_inventory_item(&mut game, "test.crisdurian", "demo.item.crisdurian");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.crisdurian")
        .expect("test artifact should exist")
        .location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_SEE_INVISIBLE.to_owned(),
        granted_race_id: None,
        ..race_form_status("rfb-legacy.race.high-elf")
    });
    assert_eq!(game.player_see_invisible_sources(), 3);

    game.player
        .statuses
        .push(race_form_status("rfb-legacy.race.half-orc"));
    assert_eq!(game.player_see_invisible_sources(), 2);
    game.player
        .statuses
        .last_mut()
        .expect("race form should exist")
        .granted_race_id = Some("rfb-legacy.race.high-elf".to_owned());
    assert_eq!(game.player_see_invisible_sources(), 3);
}

#[test]
fn intrinsic_race_see_invisible_uses_the_original_detection_roll() {
    let mut game = intrinsic_see_invisible_game(48);
    clear_monsters(&mut game);
    game.glow.fill(true);
    let player_position = Position { x: 3, y: 3 };
    game.player.position = player_position;
    let target = Position { x: 4, y: 3 };
    replace_terrain(&mut game, player_position, "demo.terrain.floor");
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.push_generated_actor(
        "test.intrinsic-invisible".to_owned(),
        "demo.actor.clear-icky-thing",
        target,
    );

    let search_skill = game.player_derived_stats().search_skill.value.max(0) as u64;
    let actor_level = game
        .content
        .actor("demo.actor.clear-icky-thing")
        .expect("test actor should exist")
        .level;
    let difficulty = u64::from(50_u32.saturating_add(actor_level / 2));
    let success_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(difficulty) < search_skill
        })
        .expect("a bounded seed should pass the invisible-detection check");
    game.rng = RfbRng::seeded(success_seed);
    let mut expected_rng = game.rng.clone();
    assert!(expected_rng.bounded(difficulty) < search_skill);
    game.refresh_invisible_visibility(true, &BTreeMap::new());
    assert!(game.entities[0].visible_invisible);
    assert_eq!(game.rng, expected_rng);

    game.entities[0].visible_invisible = false;
    game.player
        .statuses
        .push(race_form_status("rfb-legacy.race.half-orc"));
    game.rng = RfbRng::seeded(9);
    let rng_before = game.rng.clone();
    game.refresh_invisible_visibility(true, &BTreeMap::new());
    assert!(!game.entities[0].visible_invisible);
    assert_eq!(game.rng, rng_before);
}

#[test]
fn room_glow_darkening_persists_in_stored_floor_save_and_state_hash() {
    let mut game =
        Game::new_with_build(42, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    descend_one_floor(&mut game);
    let floor_id = game.current_floor_id.clone();
    assert!(game.glow.iter().any(|glow| *glow));
    let hash_before = game.state_hash();
    let darkened = game.darken_room(game.player.position);
    assert!(!darkened.is_empty());
    assert_ne!(game.state_hash(), hash_before);
    let glow_after = game.glow.clone();

    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    game.traverse_stairs(false)
        .expect("deeper descent should resolve")
        .expect("deeper descent should transition");
    assert_eq!(stored_floor(&game, &floor_id).glow, glow_after);

    let restored = Game::from_save(game.to_save()).expect("room glow should reload");
    assert_eq!(stored_floor(&restored, &floor_id).glow, glow_after);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn warrens_light_attempts_are_seeded_walkable_weighted_and_persistent() {
    let mut saw_miss = false;
    let mut saw_oil = false;
    let mut saw_lantern = false;
    for seed in 1..=64 {
        let mut left = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let mut right = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("matching Warrens Warrior should create");
        descend_one_floor(&mut left);
        descend_one_floor(&mut right);
        assert_eq!(left.items, right.items);
        let supplies = left
            .items
            .iter()
            .filter(|item| {
                matches!(item.location, ItemLocation::Ground(_))
                    && matches!(item.kind_id.as_str(), OIL_KIND_ID | LANTERN_KIND_ID)
            })
            .collect::<Vec<_>>();
        if supplies.is_empty() {
            saw_miss = true;
        }
        for supply in supplies {
            let ItemLocation::Ground(position) = supply.location else {
                unreachable!()
            };
            assert!(left.is_walkable(position));
            saw_oil |= supply.kind_id == OIL_KIND_ID;
            saw_lantern |= supply.kind_id == LANTERN_KIND_ID;
        }
        let restored = Game::from_save(left.to_save()).expect("generated light should reload");
        assert_eq!(restored.state_hash(), left.state_hash());
    }
    assert!(saw_miss && saw_oil && saw_lantern);
}

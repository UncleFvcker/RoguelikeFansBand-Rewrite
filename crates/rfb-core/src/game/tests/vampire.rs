// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

const VAMPIRE: &str = "rfb-legacy.race.vampire";
const HUMAN: &str = "demo.race.rfb-human";
const BITE: &str = "rfb.ability.race.vampirism";
const START: Position = Position { x: 99, y: 33 };
const EAST: Position = Position { x: 100, y: 33 };

fn native(race: &str) -> Game {
    let mut game = Game::new_with_build(83, "demo.build.high-mage-death").unwrap();
    clear_monsters(&mut game);
    // Explicit post-conversion precondition; this neither opens birth nor implements change_race.
    game.build.as_mut().unwrap().race_id = race.to_owned();
    game.player.position = START;
    for y in 31..=35 {
        for x in 97..=103 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.world_tick = 10;
    game.refresh_character_skills();
    game.refresh_player_resource_maxima();
    game.player.hp = game.effective_player_max_hp();
    game
}

fn form(game: &mut Game, race: &str) {
    let mut status =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.vampire").status;
    status.granted_race_id = Some(race.to_owned());
    game.player.statuses.push(status);
    game.refresh_player_resource_maxima();
}

fn consume(game: &mut Game, kind: &str) {
    give_inventory_item(game, "test.food", kind);
    dispatch_next(
        game,
        GameCommand::UseItem {
            item_id: "test.food".to_owned(),
            target: None,
        },
    );
    assert!(!game.items.iter().any(|item| item.id == "test.food"));
}

#[test]
fn five_native_targets_survive_act_use_resources_and_continue_saved_state() {
    for (race, infrared, food_gain) in [
        (VAMPIRE, 5, 500),
        ("rfb-legacy.race.skeleton", 2, 0),
        ("rfb-legacy.race.zombie", 2, 250),
        ("rfb-legacy.race.spectre", 5, 250),
        ("rfb-legacy.race.einheri", 3, 5000),
    ] {
        let mut game = native(race);
        assert!(game.player_is_nonliving(), "{race}");
        for status in [STATUS_BLEEDING, STATUS_UNWELL] {
            game.apply_player_melee_status(status, 100, "test.nonliving");
            assert!(!game.player_has_status_kind(status), "{race} {status}");
        }
        assert!(game.player_hold_life_sources() > 0, "{race}");
        assert_eq!(game.player_infravision_range(), infrared, "{race}");
        let item_ids = game
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        game.reconcile_player_body_slots_for_current_form();
        assert_eq!(
            game.items
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<_>>(),
            item_ids
        );
        game.world_tick = 50_010;
        game.nutrition = 1000;
        if race == "rfb-legacy.race.skeleton" {
            give_inventory_item(&mut game, "test.food", "demo.item.ration-of-food");
            dispatch_next(
                &mut game,
                GameCommand::UseItem {
                    item_id: "test.food".to_owned(),
                    target: None,
                },
            );
            assert!(game.items.iter().any(|item| item.id == "test.food"
                && matches!(item.location, ItemLocation::Ground { .. })));
        } else {
            consume(&mut game, "demo.item.ration-of-food");
        }
        assert_eq!(game.nutrition, 1000 + food_gain, "{race}");
        let level = if matches!(race, "rfb-legacy.race.skeleton" | "rfb-legacy.race.zombie") {
            30
        } else {
            4
        };
        game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
        game.player.hp = game.effective_player_max_hp();
        let mana = game.resources.get_mut("demo.resource.mana").unwrap();
        mana.current = mana.maximum;
        let ability = match race {
            VAMPIRE => BITE,
            "rfb-legacy.race.einheri" => "rfb.ability.race.berserk",
            "rfb-legacy.race.skeleton" | "rfb-legacy.race.zombie" => {
                "rfb.ability.race.restore-life"
            }
            _ => "rfb.ability.race.scare-monster",
        };
        assert!(
            game.snapshot()
                .player
                .abilities
                .iter()
                .any(|entry| entry.id == ability)
        );
        game.push_generated_actor(
            "test.power".to_owned(),
            "demo.actor.sheep",
            Position {
                x: START.x,
                y: START.y - 1,
            },
        );
        let seed = (0..100)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) >= 99)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let cast = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: ability.to_owned(),
                target: if matches!(
                    race,
                    "rfb-legacy.race.einheri"
                        | "rfb-legacy.race.skeleton"
                        | "rfb-legacy.race.zombie"
                ) {
                    TargetSelection::SelfTarget
                } else {
                    TargetSelection::Direction {
                        direction: Direction::North,
                    }
                },
            },
        );
        assert!(cast.events.iter().any(|event| matches!(&event.outcome, Some(GameEventOutcomeDto::AbilityCast { resolution }) if resolution.succeeded)), "{race}: {:?}", cast.events);
        clear_monsters(&mut game);
        dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        assert_eq!(game.player.position, EAST, "{race}");
        assert!(game.player.hp > 0, "{race}");
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.build.as_ref().unwrap().race_id, race);
        assert_eq!(restored.snapshot(), game.snapshot());
        assert_eq!(
            dispatch_next(&mut restored, GameCommand::Wait),
            dispatch_next(&mut game, GameCommand::Wait)
        );
        assert_eq!(restored.state_hash(), game.state_hash());
    }
    assert!(Game::new_with_build_race_and_name(83, "demo.build.warrior", VAMPIRE, "test").is_err());
}

#[test]
fn vampire_daylight_and_lamps_burn_independently_even_without_fuel() {
    let mut game = native(VAMPIRE);
    let hp = game.player.hp;
    let draws = game.rng_draw_counter();
    let mut events = Vec::new();
    assert!(game.process_vampire_light_damage(&mut events));
    assert_eq!(game.player.hp, hp - 1);
    assert_eq!(events.len(), 1);
    assert_eq!(game.rng_draw_counter(), draws);
    give_inventory_item(&mut game, "test.light", "demo.item.wooden-torch");
    let torch = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.light")
        .unwrap();
    torch.location = ItemLocation::Equipped {
        slot_id: "light".to_owned(),
    };
    torch.fuel.as_mut().unwrap().current = 0;
    game.process_vampire_light_damage(&mut events);
    assert_eq!(game.player.hp, hp - 3);
    game.world_tick = 50_010;
    game.process_vampire_light_damage(&mut events);
    assert_eq!(game.player.hp, hp - 4);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.light")
        .unwrap()
        .intrinsic_properties
        .equipment_bonuses
        .light_radius = -1;
    assert!(!game.process_vampire_light_damage(&mut events));
    assert_eq!(game.player_light_radius(), None);
    game.items.retain(|item| item.id != "test.light");
    assert_eq!(game.player_light_radius(), Some(1));
    game.world_tick = 10;
    form(&mut game, HUMAN);
    assert!(!game.process_vampire_light_damage(&mut events));
    assert!(!game.player_is_nonliving());
    game.player.statuses.clear();
    game.player.hp = 0;
    game.process_vampire_light_damage(&mut events);
    assert_eq!(game.player.hp, -1);
    assert!(matches!(
        events.last(),
        Some(DomainEvent::PlayerDiedFromLight { .. })
    ));
}

#[test]
fn vampire_invulnerability_and_light_resistance_control_damage_and_regeneration() {
    let mut game = native(VAMPIRE);
    game.player.hp = 1;
    game.player.statuses.push(
        monster_combat::melee_status(STATUS_INVULNERABILITY, 1, "test.invulnerability").status,
    );
    assert!(!game.process_vampire_light_damage(&mut Vec::new()));
    give_inventory_item(&mut game, "test.light", "demo.item.wooden-torch");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.light")
        .unwrap()
        .location = ItemLocation::Equipped {
        slot_id: "light".to_owned(),
    };
    assert!(game.process_vampire_light_damage(&mut Vec::new()));
    assert_eq!(game.player.hp, 1);
    game.player.statuses.clear();
    // A single light resistance cancels the native vulnerability; extra positive resistance is halved.
    for (level, expected) in [
        (ResistanceLevel::Resistant, 0),
        (ResistanceLevel::Strong, 33),
        (ResistanceLevel::Immune, 100),
    ] {
        game.player.statuses.clear();
        let mut status =
            monster_combat::melee_status(STATUS_BASIC_RESISTANCE, 100, "test.resistance").status;
        status.granted_resistances.insert(DamageType::Light, level);
        game.player.statuses.push(status);
        assert_eq!(game.player_resistance_percent(DamageType::Light), expected);
        assert!(!game.process_vampire_light_damage(&mut Vec::new()));
    }
}

#[test]
fn vampire_daytime_wait_burns_and_blocks_hp_recovery_while_shadow_allows_it() {
    let mut exposed = native(VAMPIRE);
    exposed.apply_player_experience(exposed.experience_required_for_level(50), &mut Vec::new());
    exposed.player.hp = 20;
    exposed.world_tick = 19;
    let mut sheltered =
        Game::from_save_with_content(exposed.to_save(), exposed.content.clone()).unwrap();
    sheltered.set_floor_glow_at(START, false);
    let result = dispatch_next(&mut exposed, GameCommand::Wait);
    dispatch_next(&mut sheltered, GameCommand::Wait);
    assert!(
        result
            .events
            .iter()
            .any(|event| event.message_key == "player-sunlight-burn")
    );
    assert_eq!(exposed.player.hp, 19);
    assert!(sheltered.player.hp > 20);
}

#[test]
fn vampire_darkness_scroll_shelter_persists_until_relit_or_dawn() {
    let mut game = native(VAMPIRE);
    let before = game.state_hash();
    consume(&mut game, "demo.item.darkness-scroll");
    let index = game.index(START).unwrap();
    assert!(game.daylight_suppressed[index]);
    let shadow_hash = game.state_hash();
    game.daylight_suppressed[index] = false;
    assert_ne!(shadow_hash, game.state_hash());
    game.daylight_suppressed[index] = true;
    assert_eq!(game.ambient_light(START, &game.collect_light_sources()), 0);
    assert!(!game.process_vampire_light_damage(&mut Vec::new()));
    assert_ne!(game.state_hash(), before);
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        dispatch_next(&mut restored, GameCommand::Wait),
        dispatch_next(&mut game, GameCommand::Wait)
    );
    game.set_floor_glow_at(START, true);
    assert_eq!(game.ambient_light(START, &game.collect_light_sources()), 48);
    game.set_floor_glow_at(START, false);
    game.world_tick = 100_000;
    game.clear_daylight_suppression_at_dawn();
    assert!(!game.daylight_suppressed[index]);
    assert!(game.process_vampire_light_damage(&mut Vec::new()));
    let mut save = game.to_save();
    save.terrain.daylight_suppressed.pop();
    assert!(Game::from_save_with_content(save, game.content.clone()).is_err());
}

#[test]
fn vampire_food_and_signed_potion_nutrition_follow_current_form() {
    for variant in 0..4 {
        for (kind, pval) in [
            ("water-potion", 200),
            ("light-healing-potion", 50),
            ("invulnerability-potion", -2500),
        ] {
            let mut game = native(if variant == 1 { HUMAN } else { VAMPIRE });
            match variant {
                1 => form(&mut game, "demo.race.vampire-lord"),
                2 => form(&mut game, HUMAN),
                3 => form(&mut game, VAMPIRE),
                _ => {}
            }
            game.world_tick = 50_010;
            game.player.hp = 1;
            game.nutrition = 5000;
            consume(&mut game, &format!("demo.item.{kind}"));
            assert_eq!(
                i32::from(game.nutrition),
                5000 + if variant == 2 { pval } else { pval / 10 },
                "{variant} {kind}"
            );
            if kind == "light-healing-potion" {
                assert!(game.player.hp > 1);
            }
        }
    }
    let mut game = native(VAMPIRE);
    game.nutrition = 0;
    game.player.hp = 4;
    game.process_hunger(&mut Vec::new());
    assert_eq!(game.player.hp, -6);
}

#[test]
fn vampire_current_body_clears_inherited_wounds_and_ignores_existing_suffocation() {
    let mut game = native(HUMAN);
    for kind in [STATUS_BLEEDING, STATUS_UNWELL, STATUS_NO_AIR] {
        game.player
            .statuses
            .push(monster_combat::melee_status(kind, 20, "test.prior-body").status);
    }
    form(&mut game, VAMPIRE);
    game.player.hp = 20;
    let mut events = Vec::new();
    game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut Vec::new(), false)
        .unwrap();
    assert_eq!(game.player.hp, 20);
    assert!(!game.player_has_status_kind(STATUS_BLEEDING));
    assert!(!game.player_has_status_kind(STATUS_UNWELL));
    assert!(game.player_has_status_kind(STATUS_NO_AIR));
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut Vec::new(), false)
        .unwrap();
    assert!(game.player.hp < 20);
}

#[test]
fn vampire_actor_darkness_shelters_daylight_without_changing_day_or_room_glow() {
    let mut game = native(VAMPIRE);
    game.push_generated_actor(
        "test.darkness".to_owned(),
        "demo.actor.ungoliant-the-unlight",
        EAST,
    );
    assert!(!game.process_vampire_light_damage(&mut Vec::new()));
    assert!(game.wilderness_is_daytime());
    assert!(!game.daylight_suppressed[game.index(START).unwrap()]);
    clear_monsters(&mut game);
    assert!(game.process_vampire_light_damage(&mut Vec::new()));
}

#[test]
fn vampire_bite_restores_life_before_hp_and_feeds_by_nominal_damage() {
    for (food, life, expected_life, healed) in [
        (1000, 990, 1000, true),
        (1000, 900, 914, false),
        (10000, 900, 900, false),
        (14998, 1000, 1000, false),
    ] {
        let mut game = native(VAMPIRE);
        game.apply_player_experience(game.experience_required_for_level(10), &mut Vec::new());
        game.progress.life_force = life;
        game.refresh_player_resource_maxima();
        game.player.hp = 1;
        game.nutrition = food;
        let mana = game.resources.get_mut("demo.resource.mana").unwrap();
        mana.current = mana.maximum;
        let seed = (0..100)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) >= 90)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        game.entities.push(actor_from_runtime_spawn(
            "test.bite",
            "demo.actor.sheep",
            EAST,
            14,
            1,
            100,
            true,
        ));
        let mut events = Vec::new();
        game.resolve_player_ability(
            BITE,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.progress.life_force, expected_life, "{events:?}");
        assert_eq!(game.player.hp > 1, healed);
        assert_eq!(game.nutrition, (food + 2000).min(14999));
    }
}

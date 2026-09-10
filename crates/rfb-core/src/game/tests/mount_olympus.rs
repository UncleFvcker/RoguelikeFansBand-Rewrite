// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::movement::actor_can_cross_terrain;
use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};
use std::sync::OnceLock;

const PERMANENT: &str = "demo.terrain.permanent-wall";
const DEEP: &str = "demo.terrain.surface-water-deep";
const SHALLOW: &str = "demo.terrain.surface-water-shallow";

const ZEUS: &str = "demo.actor.zeus-king-of-the-olympians";

fn ol4_defeat_guardian(game: &mut Game, id: &str) -> GameUpdate {
    // Keep production combat/death/loot, but shorten the encounter to one hit.
    // Select a reproducible successful hit (and Zeus's probabilistic artifact).
    game.entities.retain(|a| a.id == id);
    game.items
        .retain(|i| !matches!(&i.location, ItemLocation::CarriedBy { actor_id } if actor_id != id));
    let position = (1..game.height - 1)
        .find_map(|y| {
            (1..game.width - 2)
                .map(|x| Position {
                    x: i32::from(x),
                    y: i32::from(y),
                })
                .find(|p| game.is_walkable(*p) && game.is_walkable(Position { x: p.x + 1, y: p.y }))
        })
        .unwrap();
    game.player.position = position;
    let actor = game.entities.first_mut().unwrap();
    actor.position = Position {
        x: position.x + 1,
        y: position.y,
    };
    actor.hp = 1;
    actor.energy_need = 100_000;
    actor.nice = true;
    let base = game.clone();
    for seed in 0..256 {
        let mut attempt = base.clone();
        attempt.rng = RfbRng::seeded(seed);
        let update = super::support::dispatch_next(
            &mut attempt,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        if attempt.entities.iter().all(|a| a.id != id)
            && (id != "demo.guardian.mount-olympus.1"
                || attempt.items.iter().any(|i| i.kind_id == "demo.item.zeus"))
        {
            *game = attempt;
            return update;
        }
    }
    panic!("guardian melee/drop did not resolve: {id}");
}

#[test]
fn mount_olympus_formal_eleven_floor_round_trip_uses_rewards_and_preserves_conquest() {
    use super::support::*;
    let mut game = (0..32)
        .map(|seed| Game::new_with_build(seed, "demo.build.warrior").unwrap())
        .find(|g| g.active_pantheons & 2 != 0)
        .unwrap();
    choose_human_talent_if_pending(&mut game);
    let other_dungeons = game.dungeon_states.clone();
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    // Travel preparation skips the overland journey; entry and stairs are real commands.
    let world_position = Position { x: 5, y: 9 };
    game.wilderness_position = Some(world_position);
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    place_player_on_terrain(&mut game, "demo.terrain.mount-olympus-entrance");
    let departure = game.player.position;
    let guardian = game
        .entities
        .iter()
        .find(|a| a.id == "demo.guardian.mount-olympus-entrance.1")
        .unwrap();
    assert_eq!(guardian.kind_id, "demo.actor.sky-drake");
    assert_eq!(
        guardian.position,
        Position {
            x: departure.x - 1,
            y: departure.y - 1
        }
    );
    let killed = ol4_defeat_guardian(&mut game, "demo.guardian.mount-olympus-entrance.1");
    assert_eq!(killed.campaign.conquered_dungeons, 0);
    assert!(game.dungeon_states["demo.dungeon.mount-olympus"].entrance_guardian_defeated);
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.mount-olympus-entrance");
    for depth in 80..=90 {
        let entered = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            entered.floor_id,
            format!("demo.floor.mount-olympus-depth-{depth}")
        );
        assert_eq!((game.width, game.height), (96, 33));
        assert!(game.terrain.iter().all(|id| !matches!(
            id.as_str(),
            "demo.terrain.shaft-up"
                | "demo.terrain.shaft-down"
                | "demo.terrain.door-secret"
                | "demo.terrain.door-closed"
        )));
        assert_eq!(
            game.entities
                .iter()
                .filter(|a| a.id == "demo.guardian.mount-olympus.1")
                .count(),
            usize::from(depth == 90)
        );
        if depth < 90 {
            assert!(game.entities.iter().all(|a| a.kind_id != ZEUS));
            clear_monsters(&mut game);
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }
    assert!(
        game.terrain
            .iter()
            .all(|id| id != "demo.terrain.stairs-down")
    );
    game.items
        .retain(|i| !matches!(i.location, ItemLocation::Ground(_)));
    let conquered = ol4_defeat_guardian(&mut game, "demo.guardian.mount-olympus.1");
    assert_eq!(conquered.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.mount-olympus"].guardian_defeated);
    choose_human_talent_if_pending(&mut game);
    let rewards = game
        .items
        .iter()
        .filter(|i| matches!(i.location, ItemLocation::Ground(_)))
        .collect::<Vec<_>>();
    assert!(rewards.iter().any(|i| !matches!(
        i.kind_id.as_str(),
        "demo.item.zeus" | "demo.item.acquirement-scroll"
    )));
    let scroll = rewards
        .iter()
        .find(|i| i.kind_id == "demo.item.acquirement-scroll")
        .unwrap()
        .id
        .clone();
    let artifact = rewards
        .iter()
        .find(|i| i.kind_id == "demo.item.zeus")
        .unwrap()
        .id
        .clone();
    for id in [&scroll, &artifact] {
        let ItemLocation::Ground(position) =
            game.items.iter().find(|i| &i.id == id).unwrap().location
        else {
            unreachable!()
        };
        game.player.position = position;
        game.pick_up_item_at_player(Some(id)).unwrap();
        assert_eq!(
            game.items.iter().find(|i| &i.id == id).unwrap().location,
            ItemLocation::Inventory
        );
    }
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: scroll.clone(),
            target: None,
        },
    );
    assert!(!game.items.iter().any(|i| i.id == scroll));
    assert!(
        game.items
            .iter()
            .any(|i| i.origin_kind == Some(rfb_protocol::ItemOriginKindDto::Acquire))
    );
    assert!(game.equip_inventory_item(&artifact, None).is_some());
    game.refresh_player_resource_maxima();
    ol3_activate(
        &mut game,
        &artifact,
        Some(&TargetSelection::Direction {
            direction: Direction::East,
        }),
    );
    let consumed = game.to_save();
    let mut game = Game::from_save(consumed).unwrap();
    assert!(game.generated_artifact_ids.contains("demo.item.zeus"));
    for depth in (80..90).rev() {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
        let returned = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            returned.floor_id,
            format!("demo.floor.mount-olympus-depth-{depth}")
        );
    }
    clear_monsters(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(world_position));
    assert_eq!(game.player.position, departure);
    assert!(
        game.entities
            .iter()
            .all(|a| a.id != "demo.guardian.mount-olympus-entrance.1")
    );
    clear_monsters(&mut game);
    game.start_recall(0);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.mount-olympus-depth-90");
    assert!(game.entities.iter().all(|a| a.kind_id != ZEUS));
    assert!(game.items.iter().all(|i| i.id != scroll));
    let state = game.state_hash();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), state);
    clear_monsters(&mut game);
    clear_monsters(&mut restored);
    for current in [&mut game, &mut restored] {
        current.start_recall(0);
        dispatch_next(current, GameCommand::Wait);
        assert_eq!(current.wilderness_position, Some(world_position));
        assert_eq!(current.player.position, departure);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    for (id, state) in other_dungeons {
        if id != "demo.dungeon.mount-olympus" {
            assert_eq!(game.dungeon_states[&id], state, "{id}");
        }
    }
}

fn ol3_game() -> Game {
    let mut game = (0..32)
        .map(|seed| {
            Game::from_content_with_build(
                seed,
                catalog(),
                DEFAULT_WORLD_ID,
                "demo.build.high-mage-sorcery",
            )
            .unwrap()
        })
        .find(|g| g.active_pantheons & 2 != 0)
        .unwrap();
    game.transition_floor(
        "demo.floor.mount-olympus-depth-85".into(),
        None,
        None,
        false,
    )
    .unwrap()
    .unwrap();
    game.entities.clear();
    game.items.clear();
    game.terrain.fill("demo.terrain.floor".into());
    game.player.position = Position { x: 20, y: 15 };
    game
}

fn ol3_god(game: &Game, kind: &str) -> Actor {
    let def = game.content.actor(kind).unwrap();
    spawn_actor_from_definition(
        &mut RfbRng::seeded(0),
        def,
        "test.olympus.summoned",
        Position { x: 21, y: 15 },
        0,
        true,
    )
}

fn ol3_artifact(game: &mut Game, name: &str) -> String {
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 90,
        source: LootSource::MonsterDeath {
            actor_id: "test.olympus.drop".into(),
        },
    };
    let draft = game.fixed_item_draft(&context, format!("demo.item.{name}"));
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.identify_item_instance(
        &id,
        crate::game::inventory::ItemIdentificationRequest::new(true),
    );
    assert!(game.equip_inventory_item(&id, None).is_some(), "{name}");
    game.refresh_player_resource_maxima();
    let slot = match &game.items.iter().find(|i| i.id == id).unwrap().location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => unreachable!(),
    };
    assert!(
        game.unequip_slot(&slot).is_none(),
        "permanent curse: {name}"
    );
    id
}

fn ol3_activate(game: &mut Game, id: &str, target: Option<&TargetSelection>) -> Vec<DomainEvent> {
    let base = game.clone();
    for seed in 0..256 {
        let mut attempt = base.clone();
        attempt.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        attempt
            .use_inventory_item(
                id,
                target,
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        if events.iter().any(|e| {
            matches!(
                e,
                DomainEvent::DeviceSkillChecked {
                    succeeded: true,
                    ..
                }
            )
        }) {
            *game = attempt;
            return events;
        }
    }
    panic!("activation never accepted: {id} {target:?}");
}

#[test]
fn mount_olympus_special_drop_rolls_source_boundaries_and_preserves_other_rewards() {
    let base = ol3_game();
    for (bad_luck, roll, expected) in [
        (false, 19, true),
        (false, 20, false),
        (true, 14, true),
        (true, 15, false),
    ] {
        let mut game = base.clone();
        if bad_luck {
            game.progress
                .active_mutation_ids
                .insert("rfb.mutation.bad-luck".into());
        }
        let seed = (0..10_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) == roll)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let actor = ol3_god(&game, ZEUS);
        let (drops, _) = game.generate_death_loot(&actor).unwrap();
        assert_eq!(
            drops.iter().any(|i| i.kind_id == "demo.item.zeus"),
            expected,
            "{bad_luck} {roll}"
        );
        assert_eq!(
            drops
                .iter()
                .filter(|i| i.kind_id == "demo.item.acquirement-scroll")
                .count(),
            1
        );
        assert!(drops.iter().any(|i| !matches!(
            i.kind_id.as_str(),
            "demo.item.zeus" | "demo.item.acquirement-scroll"
        )));
        if expected {
            assert!(game.generated_artifact_ids.contains("demo.item.zeus"));
            game.rng = RfbRng::seeded(seed);
            assert!(
                game.generate_death_loot(&actor)
                    .unwrap()
                    .0
                    .iter()
                    .all(|i| i.kind_id != "demo.item.zeus")
            );
        }
    }
}

#[test]
fn mount_olympus_guardians_use_real_melee_and_quest_artifacts_skip_normal_generation() {
    let base = ol3_game();
    for kind in ["demo.actor.sky-drake", ZEUS] {
        let mut game = base.clone();
        game.entities.push(ol3_god(&game, kind));
        // Isolate incoming combat with ample HP; no natural levelling claim.
        game.player.hp = 100_000;
        let target = MonsterHostileTarget::Player {
            entity_id: game.player.id.clone(),
            kind_id: game.player.kind_id.clone(),
            position: game.player.position,
        };
        let mut events = Vec::new();
        for _ in 0..8 {
            game.resolve_monster_melee_target(
                0,
                &target,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        }
        assert!(game.player.hp < 100_000, "{kind}");
        assert!(!events.is_empty());
    }
    let mut game = base;
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 100,
        source: LootSource::MonsterDeath {
            actor_id: "test.natural".into(),
        },
    };
    let draws = game.rng_draw_counter();
    assert!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.trident"), false)
            .is_none()
    );
    assert_eq!(
        game.rng_draw_counter(),
        draws,
        "QUESTITEM is excluded before its rarity draw"
    );
    let id = ol3_artifact(&mut game, "poseidon");
    let item = game.items.iter().find(|i| i.id == id).unwrap();
    assert!(game.item_has_weapon_trait(item, WeaponTraitDto::Order));
    assert_eq!(
        game.content
            .item(&item.kind_id)
            .unwrap()
            .melee_profile
            .as_ref()
            .unwrap()
            .damage_dice,
        20
    );
}

#[test]
fn mount_olympus_dead_zeus_outside_does_not_grant_conquest_or_respawn() {
    let mut game = ol3_game();
    let surface = game
        .content
        .world(&game.world_id)
        .unwrap()
        .initial_floor_id
        .clone();
    game.transition_floor(surface.clone(), None, None, false)
        .unwrap()
        .unwrap();
    game.transition_floor("demo.floor.warrens-depth-3".into(), None, None, false)
        .unwrap()
        .unwrap();
    game.entities.clear();
    let mut actor = ol3_god(&game, ZEUS);
    actor.position = game.player.position;
    game.entities.push(actor);
    let (update, _) = super::world::defeat_guardian_with_status(
        &mut game,
        "test.olympus.summoned",
        STATUS_POISON,
    );
    assert!(
        !update
            .events
            .iter()
            .any(|e| e.kind == "dungeon.guardian-defeated")
    );
    assert!(!game.dungeon_states["demo.dungeon.mount-olympus"].guardian_defeated);
    game.transition_floor(surface, None, None, false)
        .unwrap()
        .unwrap();
    game.transition_floor(
        "demo.floor.mount-olympus-depth-90".into(),
        None,
        None,
        false,
    )
    .unwrap()
    .unwrap();
    assert!(game.entities.iter().all(|a| a.kind_id != ZEUS));
    assert!(!game.dungeon_states["demo.dungeon.mount-olympus"].guardian_defeated);
}

#[test]
fn mount_olympus_early_zeus_conquest_survives_save_and_reward_scroll_is_usable() {
    use super::support::dispatch_next;
    let mut game = ol3_game();
    let mut actor = ol3_god(&game, ZEUS);
    actor.hp = 1;
    game.entities.push(actor);
    let (update, _) = super::world::defeat_guardian_with_status(
        &mut game,
        "test.olympus.summoned",
        STATUS_POISON,
    );
    assert!(
        update
            .events
            .iter()
            .any(|e| e.kind == "dungeon.guardian-defeated")
    );
    assert!(game.dungeon_states["demo.dungeon.mount-olympus"].guardian_defeated);
    assert!(!game.unique_actor_kind_is_available(ZEUS));
    super::support::choose_human_talent_if_pending(&mut game);
    let scroll = game
        .items
        .iter_mut()
        .find(|i| i.kind_id == "demo.item.acquirement-scroll")
        .unwrap();
    scroll.location = ItemLocation::Inventory;
    let id = scroll.id.clone();
    let before = game.items.len();
    let used = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: id.clone(),
            target: None,
        },
    );
    assert!(!game.items.iter().any(|i| i.id == id));
    assert!(game.items.len() >= before, "{used:?}");
    assert!(
        game.items
            .iter()
            .any(|i| i.origin_kind == Some(rfb_protocol::ItemOriginKindDto::Acquire))
    );
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let mut restored = restored;
    restored
        .transition_floor(
            "demo.floor.mount-olympus-depth-90".into(),
            None,
            None,
            false,
        )
        .unwrap()
        .unwrap();
    assert!(restored.entities.iter().all(|a| a.kind_id != ZEUS));
    assert!(
        !restored
            .items
            .iter()
            .any(|i| i.kind_id == "demo.item.acquirement-scroll")
    );
}

#[test]
fn mount_olympus_artifacts_equip_activate_and_round_trip() {
    use super::support::give_inventory_item;
    let base = ol3_game();
    for name in [
        "zeus",
        "poseidon",
        "hades",
        "athena",
        "ares",
        "hermes",
        "apollo",
        "artemis",
        "hephaestus",
        "hera",
        "demeter",
        "aphrodite",
    ] {
        let mut game = base.clone();
        let id = ol3_artifact(&mut game, name);
        assert_eq!(
            game.items.iter().find(|i| i.id == id).unwrap().curse,
            Some(rfb_protocol::ItemCurseSeverityDto::Permanent)
        );
        let mut target = None;
        if name == "zeus" {
            let mut actor = ol3_god(&game, "demo.actor.sky-drake");
            actor.hp = 10_000;
            actor.max_hp = 10_000;
            game.entities.push(actor);
            target = Some(TargetSelection::Direction {
                direction: Direction::East,
            });
        }
        if name == "hermes" {
            let item = game.items.iter().find(|i| i.id == id).unwrap();
            assert_eq!(
                game.inventory_item_dto(item).use_target_spec.unwrap().range,
                10
            );
            target = Some(TargetSelection::Position {
                position: Position { x: 22, y: 15 },
            });
        }
        if name == "athena" {
            give_inventory_item(&mut game, "test.device", "demo.item.magic-missile-wand");
            game.items
                .last_mut()
                .unwrap()
                .charges
                .as_mut()
                .unwrap()
                .current = 0;
            target = Some(TargetSelection::Item {
                item_id: "test.device".into(),
            });
        }
        if name == "hephaestus" {
            give_inventory_item(&mut game, "test.weapon", "demo.item.dagger");
            target = Some(TargetSelection::Item {
                item_id: "test.weapon".into(),
            });
            assert!(
                game.item_use_plan(
                    &id,
                    &rfb_content::ItemUseEffectDefinition::EnchantEquipment,
                    None,
                    Some(&TargetSelection::Item {
                        item_id: id.clone()
                    }),
                    None
                )
                .is_none()
            );
        }
        if name == "hera" {
            game.resources
                .get_mut("demo.resource.mana")
                .unwrap()
                .current = 0;
        }
        if name == "hades" {
            game.progress.attributes.strength -= 1;
        }
        game.player.hp = 1;
        game.nutrition = 1_000;
        let events = ol3_activate(&mut game, &id, target.as_ref());
        match name {
            "zeus" => assert!(game.entities[0].hp < 10_000, "{events:?}"),
            "poseidon" => {
                assert!(game.terrain.iter().any(|t| t != "demo.terrain.floor"));
                assert_eq!(game.items.iter().find(|i| i.id == id).unwrap().charges.unwrap().current, 1);
                ol3_activate(&mut game, &id, None);
            }
            "hades" => assert_eq!(game.progress.attributes.strength, game.progress.maximum_attributes.strength),
            "athena" => assert!(events.iter().any(|e| matches!(e, DomainEvent::DeviceRechargeResolved { attempted, .. } if *attempted > 0))),
            "ares" => { assert!(game.player.hp > 1); assert!(game.player.statuses.iter().any(|s| s.kind_id == STATUS_BERSERK && s.remaining_ticks >= 260)); }
            "hermes" => {
                assert!(game.player.statuses.iter().any(|s| s.kind_id == STATUS_HASTE && s.remaining_ticks >= 760));
                assert_eq!(game.player.position, Position { x: 22, y: 15 });
            }
            "apollo" => { assert!(game.glow.iter().all(|g| *g)); assert!(game.explored.iter().all(|e| *e)); }
            "artemis" => { let arrows = game.items.iter().find(|i| i.kind_id == "demo.item.arrow" || i.kind_id == "demo.item.sheaf-arrow").unwrap(); assert!((5..=10).contains(&arrows.quantity)); assert_eq!(arrows.discount_percent, 99); assert_eq!(arrows.origin_kind, Some(rfb_protocol::ItemOriginKindDto::Acquire)); }
            "hephaestus" => assert_eq!(game.items.iter().find(|i| i.id == "test.weapon").unwrap().enchantments.to_hit, 3),
            "hera" => { let mana = game.resources["demo.resource.mana"]; assert_eq!(mana.current, mana.maximum); }
            "demeter" => { assert!(game.player.hp > 1); assert_eq!(game.nutrition, 14_999); }
            "aphrodite" => assert!(events.iter().any(|e| matches!(e, DomainEvent::ItemSummoned { resolution, .. } if !resolution.entity_ids.is_empty()))),
            _ => unreachable!(),
        }
        // Combat fixture uses inflated monster HP only before its damage assertion.
        if name == "zeus" {
            game.entities.clear();
        }
        game.reveal_current_visibility();
        let save = game.to_save();
        let restored = Game::from_save_with_content(save.clone(), game.content.clone())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let left = serde_json::to_value(restored.to_save()).unwrap();
        let right = serde_json::to_value(&save).unwrap();
        let differences = right
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| left[*k] != right[*k])
            .collect::<Vec<_>>();
        assert!(
            differences.is_empty(),
            "{name}: save differences {differences:?}"
        );
        if matches!(name, "artemis" | "hephaestus") {
            let mut invalid = game.clone();
            invalid
                .items
                .iter_mut()
                .find(|i| i.discount_percent == 99)
                .unwrap()
                .discount_percent = 98;
            assert!(Game::from_save_with_content(invalid.to_save(), game.content.clone()).is_err());
        }
        if name != "poseidon" {
            let state = game.to_save();
            game.use_inventory_item(
                &id,
                target.as_ref(),
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(game.to_save(), state, "cooldown: {name}");
        }
    }
}

#[test]
fn mount_olympus_artemis_partial_stack_preserves_origins_and_saved_continuation() {
    let mut game = ol3_game();
    let id = ol3_artifact(&mut game, "artemis");
    // Use a production activation to prepare matching carried ammunition, then
    // repeat its RNG with that stack present. No generation table is replaced.
    let mut preview = game.clone();
    ol3_activate(&mut preview, &id, None);
    let generated = preview
        .items
        .iter()
        .find(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
        .unwrap()
        .clone();
    let maximum = game.content.item(&generated.kind_id).unwrap().max_stack;
    let mut carried = generated.clone();
    carried.id = "test.carried".into();
    carried.quantity = maximum - 1;
    carried.origin_kind = None;
    carried.discount_percent = 0;
    carried.inscription = Some("keep".into());
    game.items.push(carried);
    ol3_activate(&mut game, &id, None);
    let merged = game
        .items
        .iter()
        .find(|item| item.id == "test.carried")
        .unwrap();
    assert_eq!(merged.quantity, maximum);
    assert_eq!(merged.origin_kind, Some(ItemOriginKindDto::Mixed));
    assert_eq!(merged.discount_percent, 99);
    assert_eq!(merged.inscription.as_deref(), Some("keep"));
    let remainder = game
        .items
        .iter()
        .find(|item| item.id == generated.id)
        .unwrap();
    assert_eq!(remainder.quantity, generated.quantity - 1);
    assert_eq!(remainder.origin_kind, Some(ItemOriginKindDto::Acquire));
    assert_eq!(remainder.discount_percent, 99);
    assert_eq!(remainder.inscription, None);
    assert_eq!(game.rng, preview.rng);
    game.reveal_current_visibility();
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for current in [&mut game, &mut restored] {
        current
            .drop_inventory_quantity("test.carried", 2)
            .unwrap()
            .unwrap();
        let split = current
            .items
            .iter()
            .find(|item| item.location == ItemLocation::Ground(current.player.position))
            .unwrap()
            .id
            .clone();
        current.pick_up_item_at_player(Some(&split)).unwrap();
        assert!(!current.items.iter().any(|item| item.id == split));
        assert_eq!(
            current
                .items
                .iter()
                .filter(|item| item.kind_id == generated.kind_id)
                .map(|item| item.quantity)
                .sum::<u32>(),
            maximum + generated.quantity - 1
        );
        assert!(current.generated_artifact_ids.contains("demo.item.artemis"));
    }
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn mount_olympus_enchantment_preserves_mundane_and_mixed_origins_after_save() {
    use super::support::give_inventory_item;
    let mut base = ol3_game();
    let id = ol3_artifact(&mut base, "hephaestus");
    for (kind, mix) in [("demo.item.dagger", false), ("demo.item.arrow", true)] {
        let mut game = base.clone();
        give_inventory_item(&mut game, "test.target", kind);
        assert!(game.mundanify_item("test.target"));
        let expected_origin = if mix {
            give_inventory_item(&mut game, "test.incoming", kind);
            for target in ["test.target", "test.incoming"] {
                game.identify_item_instance(
                    target,
                    crate::game::inventory::ItemIdentificationRequest::new(true),
                );
            }
            game.items
                .iter_mut()
                .find(|item| item.id == "test.incoming")
                .unwrap()
                .location = ItemLocation::Ground(game.player.position);
            game.pick_up_item_at_player(Some("test.incoming")).unwrap();
            assert!(!game.items.iter().any(|item| item.id == "test.incoming"));
            ItemOriginKindDto::Mixed
        } else {
            ItemOriginKindDto::Mundanity
        };
        ol3_activate(
            &mut game,
            &id,
            Some(&TargetSelection::Item {
                item_id: "test.target".into(),
            }),
        );
        let target = game
            .items
            .iter()
            .find(|item| item.id == "test.target")
            .unwrap();
        assert_eq!(target.origin_kind, Some(expected_origin));
        assert_eq!(target.discount_percent, 99);
        assert_eq!(
            (target.enchantments.to_hit, target.enchantments.to_damage),
            (3, 3)
        );
        game.reveal_current_visibility();
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        for current in [&mut game, &mut restored] {
            current
                .drop_inventory_quantity("test.target", 1)
                .unwrap()
                .unwrap();
            let dropped = current
                .items
                .iter()
                .find(|item| item.location == ItemLocation::Ground(current.player.position))
                .unwrap()
                .id
                .clone();
            current.pick_up_item_at_player(Some(&dropped)).unwrap();
        }
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        game.items
            .iter_mut()
            .find(|item| item.id == "test.target")
            .unwrap()
            .discount_percent = 98;
        assert!(matches!(
            Game::from_save_with_content(game.to_save(), game.content.clone()),
            Err(CoreError::InvalidSave("item creation state is invalid"))
        ));
    }
}

#[test]
fn mount_olympus_aphrodite_summons_follow_the_hostile_roll_and_keep_pet_ownership() {
    let mut base = ol3_game();
    let id = ol3_artifact(&mut base, "aphrodite");
    for roll in [0, 1] {
        let seed = (0..100_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(100) < 5 && rng.bounded(3) == 0 && rng.bounded(10) == roll
            })
            .unwrap();
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.use_inventory_item(
            &id,
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        let resolutions = events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::ItemSummoned { resolution, .. } => Some(resolution),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(resolutions.len(), 1);
        assert_eq!(resolutions[0].hostile, roll == 0);
        assert!(!resolutions[0].entity_ids.is_empty());
        for actor in &game.entities {
            assert_eq!(
                actor.controller_id.as_deref(),
                (roll != 0).then_some(game.player.id.as_str())
            );
            if roll != 0 {
                assert!(
                    !game
                        .original_pack_spell_flags(game.content.actor(&actor.kind_id).unwrap())
                        .1
                );
            }
        }
        game.reveal_current_visibility();
        let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
    }
}

#[test]
fn mount_olympus_ambrosia_is_local_and_preserves_satiated_nutrition() {
    use super::support::give_inventory_item;
    let mut game = ol3_game();
    let context = LootContext {
        table_id: "test.loot-table.olympus-food".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 100,
        source: LootSource::MonsterDeath {
            actor_id: "test.food".into(),
        },
    };
    let mut found = false;
    for seed in 0..128 {
        game.rng = RfbRng::seeded(seed);
        let drops = game
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap();
        found |= drops.iter().any(|i| i.kind_id == "demo.item.sunlit-feast");
    }
    assert!(found);
    let outside = LootContext {
        floor_id: "demo.floor.warrens-depth-3".into(),
        ..context
    };
    assert!(
        game.generate_loot_instances(&outside, ItemLocation::Inventory)
            .unwrap()
            .is_empty()
    );
    for food in [1_000, 15_000] {
        game.nutrition = food;
        game.player.hp = 1;
        game.player.statuses =
            vec![super::monster_combat::melee_status(STATUS_POISON, 1500, "test").status];
        give_inventory_item(&mut game, "test.food", "demo.item.sunlit-feast");
        game.use_inventory_item(
            "test.food",
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.nutrition, food.max(14_999));
        assert!(game.player.hp > 1);
        assert_eq!(
            game.player
                .statuses
                .iter()
                .find(|s| s.kind_id == STATUS_POISON)
                .unwrap()
                .remaining_ticks,
            500
        );
        assert!(!game.items.iter().any(|i| i.id == "test.food"));
    }
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut zombie = super::support::zombie_game(47);
    super::support::clear_monsters(&mut zombie);
    zombie.nutrition = 1_000;
    give_inventory_item(&mut zombie, "test.food", "demo.item.sunlit-feast");
    zombie
        .use_inventory_item(
            "test.food",
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(zombie.nutrition, 1_375);
}

fn catalog() -> Arc<ContentCatalog> {
    static CONTENT: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packs/rfb-demo-original");
            let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
            let mut food = artifact
                .content
                .loot_tables
                .iter()
                .find(|t| t.id == "demo.loot-table.base-items")
                .unwrap()
                .clone();
            food.id = "test.loot-table.olympus-food".into();
            food.entries
                .retain(|e| e.item_kind_id == "demo.item.sunlit-feast");
            artifact.content.loot_tables.push(food);
            Arc::new(ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content).unwrap(),
            ))
        })
        .clone()
}

fn game_and_floor(depth: u16) -> (Game, rfb_content::ProceduralFloorDefinition) {
    let game = (0..32)
        .map(|seed| Game::from_content(seed, catalog(), DEFAULT_WORLD_ID).unwrap())
        .find(|game| game.active_pantheons & 2 != 0)
        .unwrap();
    let floor = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|f| f.id == format!("demo.floor.mount-olympus-depth-{depth}"))
        .unwrap()
        .clone();
    (game, floor)
}

#[test]
fn mount_olympus_representative_floors_keep_routes_spawns_and_normal_ecology() {
    let mut wet = 0;
    let mut dry = 0;
    let mut companions = 0;
    let mut objects = 0;
    let mut gold = 0;
    let mut low_level = false;
    for depth in [80, 85, 90] {
        let (base, definition) = game_and_floor(depth);
        for seed in 0..16 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let floor = game.generate_procedural_floor(&definition, None).unwrap();
            let at = |p| {
                game.content
                    .terrain(&floor.terrain[generated_terrain_index(floor.width, p)])
                    .unwrap()
            };
            let walkable = floor
                .terrain
                .iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    game.content
                        .terrain(id)
                        .unwrap()
                        .walkable
                        .then_some(Position {
                            x: (i % usize::from(floor.width)) as i32,
                            y: (i / usize::from(floor.width)) as i32,
                        })
                })
                .collect::<BTreeSet<_>>();
            let reached = maze_floor_distances(&walkable, floor.player_position);
            let mut occupied = BTreeSet::from([floor.player_position]);
            assert!(at(floor.player_position).walkable);
            for (i, id) in floor.terrain.iter().enumerate() {
                let terrain = game.content.terrain(id).unwrap();
                assert!(!terrain.tags.iter().any(|tag| tag == "door"));
                if terrain
                    .tags
                    .iter()
                    .any(|tag| tag == "stairs-up" || tag == "stairs-down")
                {
                    let p = Position {
                        x: (i % usize::from(floor.width)) as i32,
                        y: (i / usize::from(floor.width)) as i32,
                    };
                    assert!(
                        reached.contains_key(&p),
                        "depth {depth}, seed {seed}, stair {p:?}"
                    );
                    // Entry on an up stair is normal; no actor may occupy it.
                    occupied.insert(p);
                }
            }
            assert!(floor.terrain.iter().any(|id| id == PERMANENT));
            assert!(
                floor
                    .terrain
                    .iter()
                    .any(|id| id == &definition.trap_terrain_id)
            );
            let water = floor.terrain.iter().any(|id| id == DEEP || id == SHALLOW);
            wet += usize::from(water);
            dry += usize::from(!water);
            for actor in &floor.entities {
                assert!(
                    occupied.insert(actor.position),
                    "overlap at {:?}",
                    actor.position
                );
                let kind = game.content.actor(&actor.kind_id).unwrap();
                assert!(actor_can_cross_terrain(kind, at(actor.position)));
                if actor.id == "demo.guardian.mount-olympus.1" {
                    assert!(reached.contains_key(&actor.position));
                } else {
                    assert!(game.pantheon_allows_allocation(&definition.id, kind));
                    low_level |= kind.level < 50;
                }
                companions += usize::from(actor.id.contains("companion"));
            }
            assert_eq!(
                floor
                    .entities
                    .iter()
                    .filter(|a| a.id == "demo.guardian.mount-olympus.1")
                    .count(),
                usize::from(depth == 90)
            );
            assert!(floor.items.iter().all(|item| match item.location {
                ItemLocation::Ground(position) => at(position).allows_items(),
                _ => false,
            }));
            assert!(
                floor
                    .gold_piles
                    .iter()
                    .all(|pile| at(pile.position).allows_items())
            );
            objects += floor.items.len();
            gold += floor.gold_piles.len();
        }
    }
    assert!(wet > 0 && dry > 0, "wet={wet}, dry={dry}");
    assert!(
        companions > 0 && low_level,
        "ordinary groups and low-level candidates remain available"
    );
    assert!(objects > 0 && gold > 0, "objects={objects}, gold={gold}");
}

#[test]
fn mount_olympus_cavern_gate_matches_source_roll_and_preserves_rng() {
    let (base, mut definition) = game_and_floor(80);
    definition.layout.as_mut().unwrap().river = None;
    for depth in [20, 80, 85, 90] {
        definition.depth = depth;
        for success in [false, true] {
            if depth == 20 && success {
                continue;
            }
            let seed = (0..10000)
                .find(|seed| {
                    (RfbRng::seeded(*seed).bounded(1000) + 1 < u64::from(depth)) == success
                })
                .unwrap();
            let mut actual = base.clone();
            actual.rng = RfbRng::seeded(seed);
            let mut expected = actual.clone();
            let mut forced = definition.clone();
            let selected = depth > 20 && expected.rng.bounded(1000) + 1 < u64::from(depth);
            if selected {
                forced
                    .layout
                    .as_mut()
                    .unwrap()
                    .cavern
                    .as_mut()
                    .unwrap()
                    .rfb_depth_chance = false;
            } else {
                forced.layout.as_mut().unwrap().cavern = None;
            }
            let actual_floor = actual.generate_procedural_floor(&definition, None).unwrap();
            let expected_floor = expected.generate_procedural_floor(&forced, None).unwrap();
            assert_eq!(actual_floor.terrain, expected_floor.terrain);
            assert_eq!(actual.rng.draw_counter, expected.rng.draw_counter);
        }
    }
}

#[test]
fn mount_olympus_water_river_keeps_permanent_fill_and_source_depth_exclusions() {
    let (base, mut definition) = game_and_floor(80);
    let mut plain = vec![
        definition.wall_terrain_id.clone();
        usize::from(definition.width) * usize::from(definition.height)
    ];
    let mut mixed = plain.clone();
    for (i, tile) in mixed.iter_mut().enumerate() {
        if i % 5 < 2 {
            *tile = PERMANENT.into();
        }
    }
    let permanent = mixed
        .iter()
        .enumerate()
        .filter_map(|(i, id)| (id == PERMANENT).then_some(i))
        .collect::<BTreeSet<_>>();
    let mut ordinary_game = base.clone();
    let mut mixed_game = base.clone();
    let center = Position { x: 48, y: 16 };
    ordinary_game.generate_river(&definition, DEEP, SHALLOW, center, &mut plain);
    mixed_game.generate_river(&definition, DEEP, SHALLOW, center, &mut mixed);
    assert!(
        permanent
            .iter()
            .any(|i| plain[*i] == DEEP || plain[*i] == SHALLOW)
    );
    for i in 0..mixed.len() {
        assert_eq!(
            mixed[i],
            if permanent.contains(&i) {
                PERMANENT
            } else {
                &plain[i]
            }
        );
    }
    assert_eq!(ordinary_game.rng.draw_counter, mixed_game.rng.draw_counter);
    definition.layout.as_mut().unwrap().cavern = None;
    definition
        .layout
        .as_mut()
        .unwrap()
        .river
        .as_mut()
        .unwrap()
        .chance_one_in = None;
    for depth in [5, 255] {
        definition.depth = depth;
        let floor = base
            .clone()
            .generate_procedural_floor(&definition, None)
            .unwrap();
        assert!(!floor.terrain.iter().any(|id| id == DEEP || id == SHALLOW));
    }
}

#[test]
fn mount_olympus_preferences_keep_source_rarity_and_divisor_eight() {
    let (mut game, definition) = game_and_floor(80);
    let policy = game
        .content
        .encounter_table("demo.encounter-table.mount-olympus")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    for tag in ["giant", "olympian", "olympian2"] {
        let actor = game
            .content
            .actor_definitions()
            .find(|a| a.tags.iter().any(|t| t == tag) && a.allocation.is_some())
            .unwrap()
            .clone();
        let before = game.rng.draw_counter;
        assert_eq!(
            game.original_dungeon_weight(&actor, &policy),
            100 / actor.allocation.as_ref().unwrap().rarity
        );
        assert_eq!(game.rng.draw_counter, before);
    }
    let actor = game.content.actor("demo.actor.newt").unwrap().clone();
    assert!([12, 13].contains(&game.original_dungeon_weight(&actor, &policy)));

    let floor = game.generate_procedural_floor(&definition, None).unwrap();
    game.activate_floor(floor, Vec::new());
    super::support::clear_monsters(&mut game);
    // Keep the source 160 parameter and the consumer's depth scaling; select
    // a reproducible successful roll without increasing ambient frequency.
    let chance = u64::from(policy.ambient_chance_one_in) * 180 / 100;
    let seed = (0..10000)
        .find(|seed| RfbRng::seeded(*seed).bounded(chance) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.process_ambient_monster_allocation(&mut BTreeSet::new())
        .unwrap();
    assert!(!game.entities.is_empty());
    for actor in &game.entities {
        let kind = game.content.actor(&actor.kind_id).unwrap();
        assert!(game.pantheon_allows_allocation(&definition.id, kind));
        assert!(
            crate::game::projectile_geometry::rfb_distance(game.player.position, actor.position)
                > 25
        );
    }
}

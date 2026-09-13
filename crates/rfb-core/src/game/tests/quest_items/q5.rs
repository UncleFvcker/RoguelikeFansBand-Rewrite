// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::player_combat::ProjectileMode;

const REWARDS: [(&str, &str); 14] = [
    ("hanuman-the-monkey-god", "gada-of-hanuman"),
    ("karthikeya-the-six-headed-warrior", "murugan"),
    ("rama-the-exiled-prince", "rama"),
    ("krishna-avatar-of-vishnu", "krishna"),
    ("vishnu-the-preserver", "kaumodaki"),
    ("vishnu-the-preserver", "kaustubha"),
    ("shiva-the-destroyer", "shiva"),
    ("shiva-the-destroyer", "shiva-avatar-jacket"),
    ("shiva-the-destroyer", "shiva-avatar-boots"),
    ("kali-mother-of-rage", "kali"),
    ("brahma-the-creating-spirit", "brahmastra"),
    ("saraswati-goddess-of-knowledge", "saraswati"),
    ("lakshmi-the-goddess-of-prosperity", "lakshmi"),
    ("vayu-the-embodied-wind", "space-suit-of-vayu"),
];

fn meru_game() -> Game {
    let mut game = (0..32)
        .map(|seed| Game::new_with_build(seed, "demo.build.warrior").unwrap())
        .find(|game| game.active_pantheons & 16 != 0)
        .unwrap();
    choose_human_talent_if_pending(&mut game);
    // Explicit level/protection preparation for source-depth content, not natural leveling.
    game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    game.apply_player_melee_status(STATUS_INVULNERABILITY, 200_000, "test.q5.protection");
    clear_monsters(&mut game);
    game
}

fn artifact(game: &mut Game, slug: &str) -> String {
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 100,
        source: LootSource::ItemUse {
            item_id: "test.q5.activation".into(),
        },
    };
    let draft = game.fixed_item_draft(&context, format!("demo.item.{slug}"));
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    id
}

fn activate(game: &mut Game, item: &str, target: Option<&TargetSelection>) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.use_inventory_item(
        item,
        target,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

#[test]
fn god_activations_apply_effects_and_restore_partial_cooldowns() {
    for slug in [
        "murugan",
        "krishna",
        "kaumodaki",
        "shiva",
        "kaustubha",
        "kali",
        "saraswati",
        "lakshmi",
    ] {
        let mut game = meru_game();
        prepare_combat(
            &mut game,
            if slug == "saraswati" {
                "demo.actor.goblin"
            } else {
                "demo.actor.greater-balrog"
            },
        );
        game.entities[0].hp = game.entities[0].max_hp;
        if slug == "krishna" {
            game.entities[0].controller_id = Some(game.player.id.clone());
            game.entities[0].position = Position { x: 17, y: 10 };
            replace_terrain(&mut game, Position { x: 17, y: 10 }, "demo.terrain.floor");
        }
        let id = artifact(&mut game, slug);
        game.equip_inventory_item(&id, None).unwrap();
        if slug == "saraswati" {
            give_inventory_item(&mut game, "test.q5.probe-light", "demo.item.wooden-torch");
            game.equip_inventory_item("test.q5.probe-light", None)
                .unwrap();
            assert!(game.entity_is_visually_visible_to_player(&game.entities[0]));
        }
        game.nutrition = 1_000;
        game.reveal_current_visibility();
        let target = if slug == "kaustubha" {
            TargetSelection::Direction {
                direction: Direction::East,
            }
        } else {
            TargetSelection::SelfTarget
        };
        let seed = (0..1000)
            .find(|seed| {
                let mut candidate = game.clone();
                candidate.rng = RfbRng::seeded(*seed);
                activate(&mut candidate, &id, Some(&target))
                    .iter()
                    .any(|event| {
                        matches!(
                            event,
                            DomainEvent::DeviceSkillChecked {
                                succeeded: true,
                                ..
                            }
                        )
                    })
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let hp = game.entities[0].hp;
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            activate(&mut game, &id, Some(&target)),
            activate(&mut loaded, &id, Some(&target)),
            "{slug}"
        );
        assert_eq!(game.state_hash(), loaded.state_hash(), "{slug}");
        match slug {
            "murugan" | "shiva" | "kaustubha" => assert!(game.entities[0].hp < hp, "{slug}"),
            "krishna" => {
                assert!(rfb_distance(game.player.position, game.entities[0].position) <= 2)
            }
            "kaumodaki" => assert!(
                game.player_has_status_kind(STATUS_HASTE)
                    && game.player_has_status_kind("rfb.status.hero")
            ),
            "kali" => assert!(game.player_has_status_kind(STATUS_BERSERK)),
            "lakshmi" => assert!(game.nutrition > 1_000),
            "saraswati" => assert!(game.probed_actor_kind_ids.contains("demo.actor.goblin")),
            _ => unreachable!(),
        }
        let interval = game
            .content
            .item(&format!("demo.item.{slug}"))
            .unwrap()
            .device_generation
            .as_ref()
            .unwrap()
            .recovery
            .unwrap()
            .interval_ticks;
        let charge = |game: &Game| {
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current
        };
        assert_eq!(charge(&game), 0);
        for tick in 1..=interval / 2 {
            game.world_tick = u32::from(tick);
            game.process_inventory_device_recovery(&mut Vec::new());
        }
        game.reveal_current_visibility();
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        for tick in interval / 2 + 1..=interval {
            for game in [&mut game, &mut loaded] {
                game.world_tick = u32::from(tick);
                game.process_inventory_device_recovery(&mut Vec::new());
            }
            assert_eq!(charge(&game), u32::from(tick == interval), "{slug}");
        }
        assert_eq!(game.state_hash(), loaded.state_hash(), "{slug}");
    }
}

fn kill_guard(game: &mut Game, id: &str) {
    game.entities.retain(|actor| actor.id == id);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    game.entities[0].position = Position { x: 11, y: 10 };
    game.entities[0].hp = 1;
    game.entities[0].nice = true;
    game.entities[0].energy_need = STANDARD_ACTION_COST;
    game.player.position = Position { x: 10, y: 10 };
    for x in 10..=11 {
        replace_terrain(game, Position { x, y: 10 }, "demo.terrain.floor");
    }
    game.reveal_current_visibility();
    let seed = (0..1024)
        .find(|seed| {
            let mut candidate = game.clone();
            candidate.rng = RfbRng::seeded(*seed);
            dispatch_next(
                &mut candidate,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            candidate.entities.iter().all(|actor| actor.id != id)
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    dispatch_next(
        game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
}

#[test]
fn meru_real_entry_shafts_guardian_reward_and_saved_return() {
    let mut game = meru_game();
    assert!(
        game.wilderness_cell_dto(Position { x: 75, y: 51 })
            .locations
            .iter()
            .any(|location| location.id == "demo.dungeon.mount-meru")
    );
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 75, y: 51 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.id == "demo.guardian.mount-meru-entrance.1")
    );
    kill_guard(&mut game, "demo.guardian.mount-meru-entrance.1");
    assert!(game.dungeon_states["demo.dungeon.mount-meru"].entrance_guardian_defeated);
    place_player_on_terrain(&mut game, "demo.terrain.mount-meru-entrance");
    let entry = game.player.position;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.mount-meru-depth-72");
    for depth in [74, 76, 78, 80, 82, 84, 86, 88, 90, 92] {
        clear_monsters(&mut game);
        game.player
            .statuses
            .retain(|status| status.kind_id != STATUS_PARALYSIS);
        place_player_on_terrain(&mut game, "demo.terrain.shaft-down");
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            game.current_floor_id,
            format!("demo.floor.mount-meru-depth-{depth}")
        );
    }
    kill_guard(&mut game, "demo.guardian.mount-meru.1");
    assert!(game.dungeon_states["demo.dungeon.mount-meru"].guardian_defeated);
    assert!(
        game.items
            .iter()
            .any(|item| item.kind_id == "demo.item.indra")
    );
    for depth in [90, 88, 86, 84, 82, 80, 78, 76, 74, 72] {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.shaft-up");
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            game.current_floor_id,
            format!("demo.floor.mount-meru-depth-{depth}")
        );
    }
    clear_monsters(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    let mut loaded = Game::from_save(game.to_save()).unwrap();
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    dispatch_next(&mut loaded, GameCommand::TraverseStairs);
    assert_eq!(game.state_hash(), loaded.state_hash());
    assert_eq!(game.player.position, entry);
    assert_eq!(game.wilderness_position, Some(Position { x: 75, y: 51 }));
}

#[test]
fn shiva_sequential_choices_precede_chance_and_preserve_generated_choice() {
    let mut base = meru_game();
    prepare_combat(&mut base, "demo.actor.shiva-the-destroyer");
    let target = base.entities[0].clone();
    let kinds = [
        "demo.item.shiva",
        "demo.item.shiva-avatar-jacket",
        "demo.item.shiva-avatar-boots",
    ];
    for selected in 0..3 {
        for hit in [true, false] {
            let seed = (0..50_000)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    let choice = if rng.bounded(4) == 0 {
                        0
                    } else if rng.bounded(4) == 0 {
                        1
                    } else {
                        2
                    };
                    choice == selected && (rng.bounded(100) < 40) == hit
                })
                .unwrap();
            for generated in [false, true] {
                let mut game = base.clone();
                if generated {
                    game.generated_artifact_ids.insert(kinds[selected].into());
                }
                game.rng = RfbRng::seeded(seed);
                let drops = game.generate_death_loot(&target).unwrap().0;
                let named = drops
                    .iter()
                    .filter(|item| kinds.contains(&item.kind_id.as_str()))
                    .collect::<Vec<_>>();
                assert_eq!(named.len(), usize::from(hit && !generated));
                if let Some(item) = named.first() {
                    assert_eq!(item.kind_id, kinds[selected]);
                }
            }
        }
    }
}

#[test]
fn meru_full_allocation_named_deaths_pickup_equipment_and_saved_continuation() {
    let mut base = meru_game();
    assert!(
        base.transition_floor("demo.floor.mount-meru-depth-90".into(), None, None, false)
            .unwrap()
            .is_some()
    );
    clear_monsters(&mut base);
    let policy = base
        .content
        .encounter_table("demo.encounter-table.mount-meru")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    let needed = REWARDS
        .iter()
        .map(|(actor, _)| format!("demo.actor.{actor}"))
        .collect::<BTreeSet<_>>();
    let mut found = BTreeSet::new();
    for _ in 0..200_000 {
        if let Some(kind) = base.select_original_allocated_monster(
            "demo.floor.mount-meru-depth-90",
            &policy,
            100,
            90,
            None,
            &[],
            None,
            None,
        ) && needed.contains(&kind)
        {
            found.insert(kind);
        }
        if found == needed {
            break;
        }
    }
    assert_eq!(
        found, needed,
        "full formal Meru allocation admits each source god"
    );
    for (actor, slug) in REWARDS {
        let mut game = base.clone();
        let kind = format!("demo.item.{slug}");
        prepare_combat(&mut game, &format!("demo.actor.{actor}"));
        game = successful_kill_start(&game, &kind);
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        let action = GameCommand::Move {
            direction: Direction::East,
        };
        assert_eq!(
            dispatch_next(&mut game, action.clone()).events,
            dispatch_next(&mut loaded, action).events
        );
        assert_eq!(game.state_hash(), loaded.state_hash(), "{slug}");
        let item = game
            .items
            .iter()
            .find(|item| item.kind_id == kind)
            .unwrap()
            .clone();
        let ItemLocation::Ground(position) = item.location else {
            panic!("death reward")
        };
        game.player.position = position;
        game.pick_up_item_at_player(Some(&item.id)).unwrap();
        if slug != "brahmastra" {
            game.equip_inventory_item(&item.id, None).unwrap();
        }
        if matches!(slug, "rama" | "kaustubha" | "kali") {
            assert_eq!(item.curse, Some(ItemCurseSeverityDto::Permanent));
        }
        if slug == "krishna" {
            assert!(game.item_resists_enchantment(&item));
        }
        game.reveal_current_visibility();
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        assert_eq!(game.state_hash(), loaded.state_hash(), "{slug}");
        assert_eq!(
            dispatch_next(&mut game, GameCommand::Wait).events,
            dispatch_next(&mut loaded, GameCommand::Wait).events
        );
        assert_eq!(game.state_hash(), loaded.state_hash(), "{slug}");
    }
}

#[test]
fn rama_shot_uses_real_ammunition_triples_damage_and_cancels_without_spending() {
    let mut base = meru_game();
    prepare_combat(&mut base, "demo.actor.great-hell-wyrm");
    base.entities[0].hp = base.entities[0].max_hp;
    let bow = artifact(&mut base, "rama");
    base.equip_inventory_item(&bow, None).unwrap();
    let empty = (base.world_tick, base.player.energy_need, base.items.clone());
    activate(&mut base, &bow, None);
    assert_eq!(
        (base.world_tick, base.player.energy_need, base.items.clone()),
        empty
    );
    give_inventory_item(&mut base, "test.q5.arrow", "demo.item.arrow");
    base.items
        .iter_mut()
        .find(|item| item.id == "test.q5.arrow")
        .unwrap()
        .quantity = 10;
    let target = TargetSelection::Direction {
        direction: Direction::East,
    };
    let mut seen = false;
    for seed in 0..1000 {
        let mut damage = Vec::new();
        for mode in [ProjectileMode::Normal, ProjectileMode::Rama] {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            game.resolve_player_projectile(
                target.clone(),
                mode,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            damage.push(events.iter().find_map(|event| match event {
                DomainEvent::ProjectileHit { damage, .. }
                | DomainEvent::ProjectileSlew { damage, .. } => Some(damage.raw),
                _ => None,
            }));
        }
        if let [Some(normal), Some(rama)] = damage.as_slice() {
            assert_eq!(*rama, normal * 3);
            seen = true;
            break;
        }
    }
    assert!(seen);
    let seed = (0..1000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            activate(&mut game, &bow, Some(&target))
                .iter()
                .any(|event| {
                    matches!(
                        event,
                        DomainEvent::DeviceSkillChecked {
                            succeeded: true,
                            ..
                        }
                    )
                })
        })
        .unwrap();
    base.rng = RfbRng::seeded(seed);
    base.reveal_current_visibility();
    let mut loaded = Game::from_save(base.to_save()).unwrap();
    assert_eq!(
        activate(&mut base, &bow, Some(&target)),
        activate(&mut loaded, &bow, Some(&target))
    );
    assert_eq!(base.state_hash(), loaded.state_hash());
    assert_eq!(
        base.items
            .iter()
            .find(|item| item.id == bow)
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
    assert_eq!(
        base.items
            .iter()
            .find(|item| item.id == "test.q5.arrow")
            .unwrap()
            .quantity,
        9
    );
}

#[test]
fn brahmastra_return_roll_and_failed_return_preserve_single_artifact() {
    let mut base = meru_game();
    prepare_combat(&mut base, "demo.actor.great-hell-wyrm");
    base.entities[0].hp = base.entities[0].max_hp;
    give_inventory_item(&mut base, "test.q5.bow", "demo.item.long-bow");
    base.equip_inventory_item("test.q5.bow", None).unwrap();
    let id = artifact(&mut base, "brahmastra");
    base.reveal_current_visibility();
    for returned in [true, false] {
        let mut game = base.clone();
        let seed = (0..1000)
            .find(|seed| (RfbRng::seeded(*seed).bounded(100) < 75) == returned)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut loaded] {
            game.resolve_player_projectile(
                TargetSelection::Direction {
                    direction: Direction::East,
                },
                ProjectileMode::Normal,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        }
        assert_eq!(game.state_hash(), loaded.state_hash());
        let item = game.items.iter().find(|item| item.id == id).unwrap();
        assert_eq!(item.quantity, 1);
        assert_eq!(item.location == ItemLocation::Inventory, returned);
        assert_eq!(
            game.items
                .iter()
                .filter(|item| item.kind_id == "demo.item.brahmastra")
                .count(),
            1
        );
    }
}

#[test]
fn vayu_equipped_suit_restores_breathing_regeneration_and_minor_slow_recovery() {
    let mut game = meru_game();
    prepare_combat(&mut game, "demo.actor.vayu-the-embodied-wind");
    game.player.statuses.clear();
    let ability = game
        .content
        .ability("rfb-legacy.ability.no-air-40")
        .unwrap()
        .clone();
    game.resolve_monster_player_effects(
        "test.q1.target",
        "demo.actor.vayu-the-embodied-wind",
        &ability,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    clear_monsters(&mut game);
    assert!(game.player_has_status_kind(STATUS_NO_AIR));
    let id = artifact(&mut game, "space-suit-of-vayu");
    assert!(
        !game.player_ignores_suffocation(),
        "carrying the suit offers no protection"
    );
    game.player.hp = 1;
    game.world_tick = 100;
    game.process_natural_hp_regeneration(true);
    assert_eq!(game.player.hp, 1);
    game.minor_slow = 10;
    let before = (
        game.minor_slow,
        game.minor_slow_energy,
        game.rng_draw_counter(),
    );
    game.process_minor_slow_recovery();
    assert_eq!(
        (
            game.minor_slow,
            game.minor_slow_energy,
            game.rng_draw_counter()
        ),
        before
    );
    game.equip_inventory_item(&id, None).unwrap();
    assert!(game.player_ignores_suffocation());
    game.process_natural_hp_regeneration(true);
    assert!(game.player.hp > 1);
    game.process_minor_slow_recovery();
    assert_ne!(
        (game.minor_slow, game.minor_slow_energy),
        (before.0, before.1)
    );
    let mut loaded = Game::from_save(game.to_save()).unwrap();
    let hp = game.player.hp;
    dispatch_next(&mut game, GameCommand::Wait);
    dispatch_next(&mut loaded, GameCommand::Wait);
    assert_eq!(game.state_hash(), loaded.state_hash());
    assert!(game.player.hp >= hp);
    let ItemLocation::Equipped { slot_id } = game
        .items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .location
        .clone()
    else {
        panic!("suit")
    };
    game.unequip_slot(&slot_id).unwrap();
    assert!(!game.player_ignores_suffocation());
}

// SPDX-License-Identifier: MPL-2.0
use super::existing_realms::prepared;
use super::support::{clear_monsters, dispatch_next, give_inventory_item};
use super::*;

const BOOKS: [&str; 4] = [
    "conjurings-and-tricks",
    "deck-of-many-things",
    "trumps-of-doom",
    "five-aces",
];

fn caster() -> Game {
    let mut game = prepared("demo.build.high-mage-trump");
    arena(&mut game);
    game
}

fn arena(game: &mut Game) {
    clear_monsters(game);
    game.player.position = Position { x: 10, y: 10 };
    for y in 6..=17 {
        for x in 6..=17 {
            let i = game.index(Position { x, y }).unwrap();
            game.terrain[i] = "demo.terrain.floor".into();
        }
    }
    game.glow.fill(true);
    game.reveal_current_visibility();
}

fn learn(game: &mut Game, slug: &str) -> String {
    let id = format!("demo.ability.trump-{slug}");
    let book = BOOKS
        .iter()
        .find(|b| {
            game.content
                .ability_book(&format!("demo.ability-book.{b}"))
                .unwrap()
                .ability_ids
                .contains(&id)
        })
        .unwrap();
    let kind = format!("demo.item.{book}");
    let instance = game
        .items
        .iter()
        .find(|i| i.kind_id == kind)
        .map(|i| i.id.clone())
        .unwrap_or_else(|| {
            let instance = format!("test.{book}");
            give_inventory_item(game, &instance, &kind);
            instance
        });
    game.study_player_ability(&instance, &id).unwrap();
    id
}

fn target(game: &mut Game) -> String {
    let mut actor = game.generated_actor(
        "test.trump-target".into(),
        "demo.actor.war-bear",
        Position { x: 11, y: 10 },
    );
    actor.hp = 1;
    let id = actor.id.clone();
    game.entities.push(actor);
    game.reveal_current_visibility();
    id
}

pub(super) fn dungeon(game: &mut Game) {
    let floor = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|f| f.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    game.dungeon_states
        .get_mut("demo.dungeon.warrens")
        .unwrap()
        .next_instance_ordinal = 1;
    let generated = game
        .generate_procedural_floor(&floor, Some("demo.dungeon.warrens.instance.1".into()))
        .unwrap();
    let items = game
        .items
        .iter()
        .filter(|i| !matches!(i.location, ItemLocation::Ground(_)))
        .cloned()
        .collect();
    game.activate_floor(generated, items);
    arena(game);
}

#[test]
fn trump_all_formal_builds_learn_cast_and_resume() {
    let content = load_built_in_content().unwrap();
    let builds = content
        .builds()
        .filter(|b| {
            b.first_realm_id.as_deref() == Some("trump")
                || b.second_realm_id.as_deref() == Some("trump")
        })
        .map(|b| b.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(builds.len(), 29);
    for build in builds {
        let mut g = prepared(&build);
        arena(&mut g);
        let book = g
            .items
            .iter()
            .find(|i| i.kind_id == "demo.item.conjurings-and-tricks")
            .unwrap()
            .id
            .clone();
        let id = "demo.ability.trump-phase-door";
        for _ in 0..8 {
            if g.learned_abilities.contains(id) {
                break;
            }
            if build.contains("priest-") || build.contains("ranger-") {
                dispatch_next(
                    &mut g,
                    GameCommand::StudyPrayer {
                        book_item_id: book.clone(),
                    },
                );
            } else {
                g.study_player_ability(&book, id).unwrap();
            }
        }
        assert!(g.learned_abilities.contains(id), "{build}");
        let before = g.player.position;
        cast_saved(&mut g, id, TargetSelection::SelfTarget);
        assert_ne!(g.player.position, before, "{build}");
        Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    }
}

#[test]
fn trump_four_books_generated_picked_up_studied_and_saved() {
    for (rank, book) in BOOKS.iter().enumerate() {
        let mut g = caster();
        let kind = format!("demo.item.{book}");
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: g.current_floor_id.clone(),
            depth: [20, 30, 55, 85][rank],
            source: LootSource::ItemUse {
                item_id: "test.trump-pool".into(),
            },
        };
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == kind)
            })
            .unwrap_or_else(|| panic!("book {book} is not generated"));
        let item = g
            .commit_generated_item_draft(draft, ItemLocation::Ground(g.player.position))
            .unwrap();
        let id = item.id.clone();
        g.items.push(item);
        g.pick_up_item_at_player(Some(&id)).unwrap();
        let spell = g
            .content
            .ability_book(&format!("demo.ability-book.{book}"))
            .unwrap()
            .ability_ids[0]
            .clone();
        g.study_player_ability(&id, &spell).unwrap();
        assert!(g.learned_abilities.contains(&spell));
        assert_eq!(
            g.content
                .item(&kind)
                .unwrap()
                .rfb_base_kind
                .as_ref()
                .unwrap()
                .source_index,
            520 + rank as u32
        );
        assert_eq!(
            Game::from_save(g.to_save(), g.behavior_preferences())
                .unwrap()
                .state_hash(),
            g.state_hash()
        );
    }
}

#[test]
fn trump_summons_are_pets_and_failures_are_hostile_with_two_exceptions() {
    for slug in [
        "spiders",
        "animals",
        "kamikaze",
        "phantasmal-servant",
        "undead",
        "reptiles",
        "monsters",
        "hounds",
        "cyberdemon",
        "dragon",
        "demon",
        "greater-undead",
        "ancient-dragon",
    ] {
        let mut g = caster();
        let id = learn(&mut g, slug);
        let aim = if slug == "kamikaze" {
            TargetSelection::Position {
                position: Position { x: 13, y: 10 },
            }
        } else {
            TargetSelection::SelfTarget
        };
        let events = cast_saved(&mut g, &id, aim.clone());
        let summons = events
            .iter()
            .filter_map(|e| {
                if let DomainEvent::AbilitySummoned { resolution, .. } = e {
                    Some(resolution)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert!(!summons.is_empty(), "{slug}");
        assert!(summons.iter().any(|s| !s.entity_ids.is_empty()), "{slug}");
        assert!(
            g.entities
                .iter()
                .all(|a| a.controller_id.as_deref() == Some(&g.player.id)),
            "{slug}"
        );
        g.reveal_current_visibility();
        Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
        let mut base = caster();
        learn(&mut base, slug);
        let failed = (0..1024)
            .find_map(|seed| {
                let mut trial = base.clone();
                for r in trial.resources.values_mut() {
                    r.current = r.maximum;
                }
                trial.rng = RfbRng::seeded(0x9e3779b97f4a7c15_u64.wrapping_mul(seed + 1));
                let mut events = Vec::new();
                trial
                    .resolve_player_ability(
                        &id,
                        aim.clone(),
                        &mut events,
                        &mut BTreeSet::new(),
                        &mut Vec::new(),
                    )
                    .unwrap();
                (events
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityCastFailed { .. }))
                    && (matches!(slug, "phantasmal-servant" | "ancient-dragon")
                        || !trial.entities.is_empty()))
                .then_some((trial, events))
            })
            .unwrap();
        let (mut g, events) = failed;
        if matches!(slug, "phantasmal-servant" | "ancient-dragon") {
            assert!(g.entities.is_empty());
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilitySummoned { .. }))
            );
        } else {
            assert!(!g.entities.is_empty(), "{slug}");
            assert!(
                g.entities
                    .iter()
                    .all(|a| a.controller_id.is_none() && a.summon.is_none() && a.no_pet)
            );
        }
        g.reveal_current_visibility();
        Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    }
}

#[test]
fn trump_utility_spells_change_real_targets_items_and_recall() {
    for slug in [
        "teleport",
        "spying",
        "teleport-away",
        "reach",
        "haste-monster",
        "dimension-door",
        "banish",
        "swap-position",
        "branding",
        "living-trump",
        "divination",
        "lore",
        "heal-monster",
        "meteors",
    ] {
        let mut g = caster();
        let id = learn(&mut g, slug);
        let actor = target(&mut g);
        let origin = g.player.position;
        let position = g.entities[0].position;
        if matches!(slug, "haste-monster" | "heal-monster") {
            g.entities[0].controller_id = Some(g.player.id.clone());
        }
        let aim = match slug {
            "teleport-away" => TargetSelection::Direction {
                direction: Direction::East,
            },
            "haste-monster" | "swap-position" | "heal-monster" => TargetSelection::Entity {
                entity_id: actor.clone(),
            },
            "dimension-door" => TargetSelection::Position {
                position: Position { x: 13, y: 10 },
            },
            "reach" => {
                give_inventory_item(&mut g, "test.fetch", "demo.item.ration-of-food");
                g.items
                    .iter_mut()
                    .find(|i| i.id == "test.fetch")
                    .unwrap()
                    .location = ItemLocation::Ground(Position { x: 13, y: 10 });
                TargetSelection::Direction {
                    direction: Direction::East,
                }
            }
            "branding" | "lore" => {
                give_inventory_item(&mut g, "test.weapon", "demo.item.dagger");
                TargetSelection::Item {
                    item_id: "test.weapon".into(),
                }
            }
            _ => TargetSelection::SelfTarget,
        };
        let events = cast_saved(&mut g, &id, aim);
        match slug {
            "teleport" | "dimension-door" => assert_ne!(g.player.position, origin),
            "spying" => assert!(
                g.player
                    .statuses
                    .iter()
                    .any(|s| s.kind_id == "rfb.status.telepathy")
            ),
            "swap-position" => {
                assert_eq!(g.player.position, position);
                assert_eq!(g.entities[0].position, origin);
            }
            "teleport-away" | "banish" => assert_ne!(g.entities[0].position, position),
            "haste-monster" => assert!(
                g.entities[0]
                    .statuses
                    .iter()
                    .any(|s| s.kind_id == "rfb.status.haste")
            ),
            "heal-monster" => assert_eq!(g.entities[0].hp, g.entities[0].max_hp),
            "reach" => assert_eq!(
                g.items
                    .iter()
                    .find(|i| i.id == "test.fetch")
                    .unwrap()
                    .location,
                ItemLocation::Ground(origin)
            ),
            "branding" => assert!(
                g.items
                    .iter()
                    .find(|i| i.id == "test.weapon")
                    .unwrap()
                    .affix_ids
                    .contains(&"rfb-legacy.affix.trump".into())
            ),
            "living-trump" => assert!(g.progress.active_mutation_ids.iter().any(|id| matches!(
                id.as_str(),
                "rfb.mutation.teleport" | "rfb.mutation.teleport-rnd"
            ))),
            "lore" => assert!(
                g.item_property_knowledge
                    .get("test.weapon")
                    .unwrap()
                    .identified
            ),
            "meteors" => assert!(
                events
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityAreaDamage { .. }))
            ),
            "divination" => assert!(
                events
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityDetected { .. }))
            ),
            _ => unreachable!(),
        }
        g.reveal_current_visibility();
        Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    }
    for slug in ["reset-recall", "word-of-recall", "teleport-level"] {
        let mut g = caster();
        let id = learn(&mut g, slug);
        dungeon(&mut g);
        let floor = g.current_floor_id.clone();
        cast_saved(&mut g, &id, TargetSelection::SelfTarget);
        match slug {
            "reset-recall" => assert_eq!(
                g.recall
                    .as_ref()
                    .unwrap()
                    .destination
                    .as_ref()
                    .unwrap()
                    .floor_id,
                floor
            ),
            "word-of-recall" => assert!(g.recall.as_ref().unwrap().remaining_turns.is_some()),
            _ => assert_ne!(g.current_floor_id, floor),
        }
        Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    }
}

#[test]
fn trump_shuffle_lovers_paid_direction_cancel_and_save_are_exact() {
    let mut base = caster();
    let id = learn(&mut base, "shuffle");
    target(&mut base);
    let mut paid = (0..2048)
        .find_map(|seed| {
            let mut g = base.clone();
            for r in g.resources.values_mut() {
                r.current = r.maximum;
            }
            g.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            g.resolve_player_ability(
                &id,
                TargetSelection::SelfTarget,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            g.pending_ability_direction.is_some().then_some(g)
        })
        .unwrap();
    let mut invalid = paid.to_save();
    invalid
        .player
        .pending_ability_direction
        .as_mut()
        .unwrap()
        .branch_roll = 2;
    assert!(Game::from_save(invalid, Game::default_behavior_preferences()).is_err());
    let mut resumed = Game::from_save(paid.to_save(), paid.behavior_preferences()).unwrap();
    let rng = paid.rng.clone();
    let mana = paid.resources["demo.resource.mana"].current;
    let mut direct_cancel = paid.clone();
    direct_cancel
        .resolve_pending_call_chaos(None, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert_eq!(direct_cancel.rng, rng);
    assert_eq!(direct_cancel.resources["demo.resource.mana"].current, mana);
    let mut resolved = paid.clone();
    let mut resolved_saved = Game::from_save(paid.to_save(), paid.behavior_preferences()).unwrap();
    for g in [&mut resolved, &mut resolved_saved] {
        dispatch_next(
            g,
            GameCommand::ResolveAbilityDirection {
                direction: Direction::East,
            },
        );
        assert!(g.pending_ability_direction.is_none());
    }
    assert_eq!(resolved.state_hash(), resolved_saved.state_hash());
    for g in [&mut paid, &mut resumed] {
        dispatch_next(g, GameCommand::CancelAbilityDirection);
        assert!(g.pending_ability_direction.is_none());
        assert!(g.resources["demo.resource.mana"].current >= mana);
    }
    assert_eq!(paid.state_hash(), resumed.state_hash());
    assert_eq!(paid.rng, resumed.rng);
}

#[test]
fn trump_shuffle_every_card_boundary_and_wild_magic_executes_without_invalid_state() {
    let base = caster();
    let ability = base
        .content
        .ability("demo.ability.trump-shuffle")
        .unwrap()
        .clone();
    for die in [
        1, 7, 14, 18, 22, 26, 30, 33, 38, 40, 42, 47, 52, 60, 72, 80, 82, 84, 86, 88, 96, 101, 111,
        120,
    ] {
        let mut g = base.clone();
        target(&mut g);
        let mut events = Vec::new();
        let pending = g
            .resolve_trump_card(
                &ability,
                die,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(pending, die == 88);
        if die == 101 {
            assert_ne!(g.progress.hp_progression, base.progress.hp_progression);
        }
        if die == 120 {
            assert!(g.progress.experience > base.progress.experience);
        }
        if die == 30 {
            assert_eq!(g.entities[0].hp, 1);
            assert!(!g.glow[g.index(g.player.position).unwrap()]);
        }
        g.reveal_current_visibility();
        Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    }
}

#[test]
fn trump_wheel_all_nested_outcomes_preserve_valid_saves() {
    let mut base = caster();
    dungeon(&mut base);
    let ability = base
        .content
        .ability("demo.ability.trump-shuffle")
        .unwrap()
        .clone();
    for roll in 3..=40 {
        let mut g = base.clone();
        g.resolve_trump_wild_card(
            &ability,
            31,
            "bizarre1",
            roll,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        g.reveal_current_visibility();
        if !g.player_is_dead() {
            Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
        }
        if matches!(roll, 19 | 20) {
            assert!(g.terrain.iter().any(|t| t == "demo.terrain.created-trap"));
        }
        if matches!(roll, 21 | 22) {
            assert!(g.terrain.iter().any(|t| t == "demo.terrain.door-closed"));
        }
        if roll == 38 {
            assert!(
                g.entities
                    .iter()
                    .any(|a| a.kind_id == "demo.actor.cyberdemon")
            );
        }
    }
}

#[test]
fn trump_no_space_still_pays_and_invalid_kamikaze_target_does_not() {
    let mut g = caster();
    let spiders = learn(&mut g, "spiders");
    for y in 8..=12 {
        for x in 8..=12 {
            if (x, y) != (10, 10) {
                let i = g.index(Position { x, y }).unwrap();
                g.terrain[i] = "demo.terrain.wall".into();
            }
        }
    }
    g.reveal_current_visibility();
    for r in g.resources.values_mut() {
        r.current = r.maximum;
    }
    let before = g.resources["demo.resource.mana"].current;
    let mut events = Vec::new();
    g.resolve_player_ability(
        &spiders,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(g.entities.is_empty());
    assert!(g.resources["demo.resource.mana"].current < before);
    Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    let mut g = caster();
    let kamikaze = learn(&mut g, "kamikaze");
    let before = g.state_hash();
    let rng = g.rng.clone();
    g.resolve_player_ability(
        &kamikaze,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(g.state_hash(), before);
    assert_eq!(g.rng, rng);
}

fn cast_saved(game: &mut Game, id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    game.reveal_current_visibility();
    for r in game.resources.values_mut() {
        r.current = r.maximum;
    }
    for seed in 0..256 {
        let mut trial = game.clone();
        trial.rng = RfbRng::seeded(0x9e3779b97f4a7c15_u64.wrapping_mul(seed + 1));
        let saved = trial.to_save();
        let mut events = Vec::new();
        trial
            .resolve_player_ability(
                id,
                target.clone(),
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, DomainEvent::AbilityTargetUnavailable { .. })),
            "{id}: {events:?}"
        );
        if !events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
        {
            continue;
        }
        let summoned = events
            .iter()
            .filter_map(|e| {
                if let DomainEvent::AbilitySummoned { resolution, .. } = e {
                    Some(resolution)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !summoned.is_empty() && summoned.iter().all(|s| s.entity_ids.is_empty()) {
            continue;
        }
        let mut restored = Game::from_save_with_content(
            saved,
            trial.content.clone(),
            Game::default_behavior_preferences(),
        )
        .unwrap();
        restored
            .resolve_player_ability(
                id,
                target.clone(),
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(trial.state_hash(), restored.state_hash(), "{id}");
        assert_eq!(trial.rng, restored.rng);
        *game = trial;
        return events;
    }
    panic!("no successful cast for {id}");
}

#[test]
#[ignore = "controlled preparation for ordinary standalone Trump acceptance"]
fn export_trump_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, _) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    let mut scenarios = Vec::new();
    for (name, slug) in [
        ("first-book", "phase-door"),
        ("pets", "spiders"),
        ("lovers", "shuffle"),
        ("cancel-lovers", "shuffle"),
        ("heal-pet", "heal-monster"),
        ("branding", "branding"),
    ] {
        let mut base = caster();
        let ability = if name == "first-book" {
            "demo.ability.trump-phase-door".into()
        } else {
            learn(&mut base, slug)
        };
        base.apply_player_melee_status(STATUS_INVULNERABILITY, 1000, "test.trump-desktop");
        let aim = match name {
            "pets" => TargetSelection::Position {
                position: base.player.position,
            },
            "lovers" | "cancel-lovers" | "heal-pet" => {
                target(&mut base);
                if name == "heal-pet" {
                    base.entities[0].controller_id = Some(base.player.id.clone());
                    TargetSelection::Direction {
                        direction: Direction::East,
                    }
                } else {
                    TargetSelection::SelfTarget
                }
            }
            "branding" => {
                give_inventory_item(&mut base, "test.trump-weapon", "demo.item.dagger");
                TargetSelection::Item {
                    item_id: "test.trump-weapon".into(),
                }
            }
            _ => TargetSelection::SelfTarget,
        };
        for resource in base.resources.values_mut() {
            resource.current = resource.maximum;
        }
        base.reveal_current_visibility();
        let (start, steps) = (0..4096)
            .find_map(|seed| {
                let mut game = base.clone();
                game.rng = RfbRng::seeded(0x9e3779b97f4a7c15_u64.wrapping_mul(seed + 1));
                let start = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
                let mut steps = Vec::new();
                if name == "first-book" {
                    let book = game
                        .items
                        .iter()
                        .find(|i| i.kind_id == "demo.item.conjurings-and-tricks")
                        .unwrap()
                        .id
                        .clone();
                    let command = GameCommand::StudyAbility {
                        book_item_id: book,
                        ability_id: ability.clone(),
                    };
                    dispatch_next(&mut game, command.clone());
                    steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                }
                let command = GameCommand::CastAbility {
                    ability_id: ability.clone(),
                    target: aim.clone(),
                };
                let update = game
                    .dispatch(GameCommandEnvelope {
                        expected_revision: game.revision,
                        command_seq: game.last_command_seq + 1,
                        command: command.clone(),
                    })
                    .ok()?;
                if !update
                    .events
                    .iter()
                    .any(|e| e.message_key == "ability-cast-success")
                {
                    return None;
                }
                if name.contains("lovers") && game.pending_ability_direction.is_none() {
                    return None;
                }
                if name == "pets" && game.entities.is_empty() {
                    return None;
                }
                if name == "heal-pet" && game.entities[0].hp <= 1 {
                    return None;
                }
                if name == "branding"
                    && game
                        .items
                        .iter()
                        .find(|i| i.id == "test.trump-weapon")
                        .unwrap()
                        .affix_ids
                        .is_empty()
                {
                    return None;
                }
                assert_eq!(
                    Game::from_save(game.to_save(), game.behavior_preferences())
                        .unwrap()
                        .state_hash(),
                    game.state_hash()
                );
                steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                if game.pending_ability_direction.is_some() {
                    let command = if name == "cancel-lovers" {
                        GameCommand::CancelAbilityDirection
                    } else {
                        GameCommand::ResolveAbilityDirection {
                            direction: Direction::East,
                        }
                    };
                    dispatch_next(&mut game, command.clone());
                    steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                }
                dispatch_next(&mut game, GameCommand::Wait);
                steps.push(
                    serde_json::json!({"command":GameCommand::Wait,"hash":game.state_hash()}),
                );
                assert_eq!(
                    Game::from_save(game.to_save(), game.behavior_preferences())
                        .unwrap()
                        .state_hash(),
                    game.state_hash()
                );
                Some((start, steps))
            })
            .expect("successful production Trump commands");
        std::fs::write(
            directory.join(format!("{name}.rfbsave")),
            rfb_save::encode(&header, &start.to_save()).unwrap(),
        )
        .unwrap();
        scenarios
            .push(serde_json::json!({"name":name,"initialHash":start.state_hash(),"steps":steps}));
    }
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&scenarios).unwrap(),
    )
    .unwrap();
}

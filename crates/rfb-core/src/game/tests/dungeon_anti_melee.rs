// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use std::sync::OnceLock;

const FLOOR: &str = "demo.floor.anti-melee-cave-depth-40";
const MONSTER: &str = "demo.actor.small-kobold";

fn catalog() -> Arc<ContentCatalog> {
    static CONTENT: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packs/rfb-demo-original");
            let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
            enable_test_caster(&mut artifact.content);
            artifact
                .content
                .items
                .iter_mut()
                .find(|item| item.id == "demo.item.dagger")
                .unwrap()
                .throw_profile = Some(rfb_content::ThrowProfileDefinition {
                to_hit: 0,
                to_damage: 0,
                damage_dice: 1,
                damage_sides: 4,
                damage_type: rfb_content::ActorDamageType::Physical,
            });
            for id in ["demo.actor.beholder", "demo.actor.ash-drake"] {
                artifact
                    .content
                    .actors
                    .iter_mut()
                    .find(|a| a.id == id)
                    .unwrap()
                    .monster_casting
                    .as_mut()
                    .unwrap()
                    .frequency_percent = 100;
            }
            Arc::new(ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content).unwrap(),
            ))
        })
        .clone()
}

fn arena() -> Game {
    arena_with_build("demo.build.warrior")
}

fn arena_with_build(build_id: &str) -> Game {
    let mut game =
        Game::from_content_with_build(785, catalog(), DEFAULT_WORLD_ID, build_id).unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.current_floor_id = FLOOR.to_owned();
    game.player.position = Position { x: 8, y: 8 };
    for y in 3..=14 {
        for x in 3..=14 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.push_generated_actor("test.enemy".to_owned(), MONSTER, Position { x: 9, y: 8 });
    game.entities[0].nice = false;
    game.entities[0].alerted = true;
    game.entities[0].statuses.clear();
    game
}

#[test]
fn dungeon_anti_melee_blocks_player_blows_before_rng_training_and_contact_but_spends_turn() {
    let mut game = arena();
    let before = (
        game.rng.clone(),
        game.player.hp,
        game.entities.clone(),
        game.progress.clone(),
    );
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    let outcome = game
        .resolve_player_melee(0, true, &mut events, &mut changed, &mut removed)
        .unwrap();
    assert!(!outcome.killed);
    assert_eq!(outcome.attacks_used, 0);
    game.resolve_player_draconian_strike(
        0,
        DraconianStrikeModeDefinition::Vampiric,
        &mut events,
        &mut changed,
        &mut removed,
    )
    .unwrap();
    game.resolve_player_revenge_blow(0, &mut events, &mut changed, &mut removed)
        .unwrap();
    assert_eq!(
        (
            game.rng.clone(),
            game.player.hp,
            game.entities.clone(),
            game.progress.clone()
        ),
        before
    );
    assert_eq!(events.len(), 3);
    assert!(
        events
            .iter()
            .all(|event| matches!(event, DomainEvent::PlayerMeleeBlocked))
    );
    assert!(changed.is_empty() && removed.is_empty());

    let tick = game.world_tick;
    let position = game.player.position;
    let update = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(game.world_tick > tick);
    assert_eq!(game.player.position, position);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "player.melee-blocked")
    );
    assert_eq!(game.player.hp, before.1);
    assert_eq!(game.entities[0].hp, before.2[0].hp);

    game.current_floor_id = "demo.floor.surface".to_owned();
    events.clear();
    let draws = game.rng_draw_counter();
    game.resolve_player_melee(0, true, &mut events, &mut changed, &mut removed)
        .unwrap();
    assert!(game.rng_draw_counter() > draws);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
    )));
}

#[test]
fn dungeon_anti_melee_blocks_enemy_pet_friend_and_revenge_without_hit_side_effects() {
    let mut game = arena();
    game.push_generated_actor("test.pet".to_owned(), MONSTER, Position { x: 9, y: 9 });
    game.entities[1].controller_id = Some(game.player.id.clone());
    let player = MonsterHostileTarget::Player {
        entity_id: game.player.id.clone(),
        kind_id: game.player.kind_id.clone(),
        position: game.player.position,
    };
    let pet = MonsterHostileTarget::Summon {
        entity_id: game.entities[1].id.clone(),
        kind_id: MONSTER.to_owned(),
        position: game.entities[1].position,
    };
    let before = (
        game.rng.clone(),
        game.player.clone(),
        game.entities.clone(),
        game.items.clone(),
    );
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    game.resolve_monster_melee_target(0, &player, &mut events, &mut changed, &mut removed)
        .unwrap();
    game.resolve_monster_melee_target(0, &pet, &mut events, &mut changed, &mut removed)
        .unwrap();
    assert!(
        !game
            .resolve_monster_revenge_blow(0, 0, &mut events, &mut changed, &mut removed)
            .unwrap()
    );
    game.resolve_player_summon_melee(1, "test.enemy", &mut events, &mut changed, &mut removed)
        .unwrap();
    assert_eq!(
        (
            game.rng.clone(),
            game.player.clone(),
            game.entities.clone(),
            game.items.clone()
        ),
        before
    );
    assert!(events.is_empty() && changed.is_empty() && removed.is_empty());
    game.entities[1].controller_id = None;
    game.entities[1].friendly = true;
    game.resolve_monster_action(
        1,
        &mut events,
        &mut changed,
        &mut removed,
        &mut BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(game.entities[0].hp, before.2[0].hp);
    assert!(events.is_empty());
}

#[test]
fn dungeon_anti_melee_scheduling_tries_another_step_except_for_stupid_or_confused() {
    let base = arena();
    let mut smart = base.clone();
    smart
        .resolve_monster_action(
            0,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
            &mut BTreeSet::new(),
        )
        .unwrap();
    assert_ne!(smart.entities[0].position, base.entities[0].position);
    assert_eq!(smart.player.hp, base.player.hp);
    let mut confused = base.clone();
    confused.entities[0]
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 100, "test").status);
    confused
        .resolve_monster_action(
            0,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
            &mut BTreeSet::new(),
        )
        .unwrap();
    assert_eq!(confused.entities[0].position, base.entities[0].position);
    assert_eq!(confused.player.hp, base.player.hp);
    // The STUPID decision follows the runtime form, as the spell AI does.
    let mut stupid = base.clone();
    stupid.entities[0].kind_id = "demo.actor.chameleon".to_owned();
    stupid.entities[0].appearance_kind_id = Some("demo.actor.blinking-dot".to_owned());
    assert!(stupid.monster_attempts_melee(0));
    assert!(!base.monster_attempts_melee(0));

    let mut body = base.clone();
    body.entities[0].kind_id = "demo.actor.poseidon-lord-of-seas-and-storm".to_owned();
    body.push_generated_actor("test.blocker".to_owned(), MONSTER, Position { x: 10, y: 8 });
    for actor in &mut body.entities {
        actor.controller_id = Some(body.player.id.clone());
    }
    for y in 7..=9 {
        for x in 9..=12 {
            replace_terrain(
                &mut body,
                Position { x, y },
                "demo.terrain.surface-water-shallow",
            );
        }
    }
    // NO_MELEE must not turn KILL_BODY into MOVE_BODY when both source flags exist.
    assert!(body.actor_can_kill_body_blocker(0, 1));
    assert!(!body.actor_can_move_body_blocker(0, 1));
    let blocker_position = body.entities[1].position;
    assert_eq!(
        body.move_entity(
            0,
            blocker_position,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new()
        )
        .unwrap(),
        ActorStepOutcome::Blocked
    );
    assert_eq!(body.entities[1].position, blocker_position);
    assert!(
        body.next_monster_step_toward(0, Position { x: 12, y: 8 }, true)
            .is_some_and(|step| step != blocker_position)
    );
}

#[test]
fn dungeon_anti_melee_gaze_still_casts_but_cannot_execute_its_melee_routine() {
    let mut game = arena();
    game.entities[0].kind_id = "demo.actor.beholder".to_owned();
    game.entities[0].position = Position { x: 4, y: 8 };
    let mut normal = game.clone();
    normal.current_floor_id = "demo.floor.surface".to_owned();
    let hp = game.player.hp;
    let draws = game.rng_draw_counter();
    let mut events = Vec::new();
    assert!(game.resolve_monster_ability(0, &mut events));
    assert_eq!(game.player.hp, hp);
    assert_eq!(game.rng_draw_counter() - draws, 2); // Frequency and candidate, no hit/damage.
    assert!(events.iter().any(
        |event| matches!(event, DomainEvent::MonsterAbilityCast { resolution, .. }
        if resolution.ability_id == "rfb-legacy.ability.gaze")
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterMeleeHit { .. } | DomainEvent::MonsterMeleeMissed { .. }
    )));
    assert!(normal.resolve_monster_ability(0, &mut Vec::new()));
    assert!(normal.rng_draw_counter() > game.rng_draw_counter());
    assert_eq!(
        normal.entities[0].casting_cooldown_remaining,
        game.entities[0].casting_cooldown_remaining
    );
}

#[test]
fn dungeon_anti_melee_preserves_movement_shooting_throwing_and_ranged_spells() {
    let mut base = arena_with_build("demo.build.archer");
    base.entities[0].position = Position { x: 10, y: 8 };
    base.entities[0].hp = 10_000;
    base.entities[0].max_hp = 10_000;
    give_inventory_item(&mut base, "test.throw", "demo.item.dagger");
    for shoot in [true, false] {
        let mut blocked = base.clone();
        let mut normal = base.clone();
        normal.current_floor_id = "demo.floor.surface".to_owned();
        let mut results = Vec::new();
        for game in [&mut blocked, &mut normal] {
            let mut events = Vec::new();
            if shoot {
                game.resolve_player_projectile(
                    TargetSelection::Direction {
                        direction: Direction::East,
                    },
                    player_combat::ProjectileMode::Normal,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
                assert!(events.iter().any(|event| matches!(
                    event,
                    DomainEvent::ProjectileHit { .. } | DomainEvent::ProjectileMissed { .. }
                )));
            } else {
                game.throw_inventory_item(
                    "test.throw",
                    Direction::East,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
                assert!(events.iter().any(|event| matches!(
                    event,
                    DomainEvent::ItemThrowHit { .. } | DomainEvent::ItemThrowMissed { .. }
                )));
            }
            results.push(events);
        }
        assert_eq!(results[0], results[1]);
        assert_eq!(blocked.entities, normal.entities);
        assert_eq!(blocked.items, normal.items);
        assert_eq!(blocked.rng, normal.rng);
    }
    for kind in ["demo.actor.ash-drake", "demo.actor.cyberdemon"] {
        let mut blocked = base.clone();
        blocked.player.hp = 10_000;
        blocked.entities[0].kind_id = kind.to_owned();
        blocked.entities[0].position = Position { x: 12, y: 8 };
        let frequency = blocked
            .content
            .actor(kind)
            .unwrap()
            .monster_casting
            .as_ref()
            .unwrap()
            .frequency_percent;
        let seed = (0..100)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(frequency))
            .unwrap();
        blocked.rng = RfbRng::seeded(seed);
        let mut normal = blocked.clone();
        normal.current_floor_id = "demo.floor.surface".to_owned();
        let mut blocked_events = Vec::new();
        let mut normal_events = Vec::new();
        assert!(
            blocked.resolve_monster_ability(0, &mut blocked_events),
            "{kind}"
        );
        assert!(
            normal.resolve_monster_ability(0, &mut normal_events),
            "{kind}"
        );
        assert_eq!(blocked_events, normal_events);
        assert_eq!(blocked.player.hp, normal.player.hp);
        assert_eq!(blocked.rng, normal.rng);
        assert!(blocked.player.hp < 10_000);
    }
    let mut spell = prepare_death_caster(0, 40, "demo.ability.death-berserk");
    spell.content = catalog();
    spell.current_floor_id = FLOOR.to_owned();
    spell.debug_set_ability_casts_succeed(true);
    spell
        .resolve_player_ability(
            "demo.ability.death-berserk",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert!(spell.player_has_status_kind(STATUS_BERSERK));
    clear_monsters(&mut base);
    let before = base.player.position;
    dispatch_next(
        &mut base,
        GameCommand::Move {
            direction: Direction::West,
        },
    );
    assert_eq!(
        base.player.position,
        Position {
            x: before.x - 1,
            y: before.y
        }
    );
}

#[test]
fn dungeon_anti_melee_special_casts_keep_costs_and_vampirism_checks_failure_before_cancelling() {
    let mut frenzy = arena();
    frenzy.progress.level = 40;
    frenzy.player.hp = 500;
    frenzy
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.fantastic-frenzy".to_owned());
    frenzy.progress.locked_mutation_ids = frenzy.progress.active_mutation_ids.clone();
    frenzy.debug_set_ability_casts_succeed(true);
    let hp = frenzy.entities[0].hp;
    let mut events = Vec::new();
    frenzy
        .resolve_player_ability(
            "rfb.ability.mutation.fantastic-frenzy",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(frenzy.player.hp, 450);
    assert_eq!(frenzy.entities[0].hp, hp);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerMeleeBlocked))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
    );

    let mut panic = arena();
    panic.progress.level = 40;
    panic.player.hp = 500;
    assert!(panic.gain_mutation("rfb.mutation.panic-hit", &mut Vec::new()));
    panic.debug_set_ability_casts_succeed(true);
    events.clear();
    let hp = panic.entities[0].hp;
    panic
        .resolve_player_ability(
            "rfb.ability.mutation.panic-hit",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(panic.entities[0].hp, hp);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerMeleeBlocked))
    );
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. }
        if resolution.effects.iter().any(|effect| matches!(effect, AbilityEffectResolutionDto::MeleeThenTeleport { .. })))));
    assert!(panic.player.hp < 500);

    let mut vampire = arena();
    vampire.progress.level = 2;
    assert!(vampire.gain_mutation("rfb.mutation.vampirism", &mut Vec::new()));
    choose_human_talent_if_pending(&mut vampire);
    let ability = "rfb.ability.mutation.vampirism";
    vampire.debug_set_ability_casts_succeed(true);
    assert!(
        !vampire
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|a| a.id == ability)
            .unwrap()
            .can_cast
    );
    let before = (vampire.player.clone(), vampire.world_tick);
    let mut expected_rng = vampire.rng.clone();
    expected_rng.bounded(100);
    let update = dispatch_next(
        &mut vampire,
        GameCommand::CastAbility {
            ability_id: ability.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert_eq!((vampire.player.clone(), vampire.world_tick), before);
    assert_eq!(vampire.rng, expected_rng);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.cast-unavailable")
    );

    let mut native = vampire.clone();
    native.build.as_mut().unwrap().race_id = "rfb-legacy.race.vampire".to_owned();
    native
        .resolve_player_ability(
            "rfb.ability.race.vampirism",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert!(
        matches!(events.last(), Some(DomainEvent::AbilityCastUnavailable { reason, .. }) if reason == "anti-melee")
    );
    assert_eq!(native.player.hp, vampire.player.hp);

    vampire.debug_set_ability_casts_succeed(false);
    let activation = vampire.mutation_activation_for_ability(ability).unwrap();
    let failure = vampire.innate_power_failure_percent(activation);
    assert!(failure > 0);
    let seed = (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(failure))
        .unwrap();
    vampire.rng = RfbRng::seeded(seed);
    clear_monsters(&mut vampire);
    let tick = vampire.world_tick;
    let mut direct = vampire.clone();
    let hp = direct.player.hp;
    events.clear();
    direct
        .resolve_player_ability(
            ability,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert!(
        matches!(events.as_slice(), [DomainEvent::AbilityCastFailed { resolution }]
        if resolution.hp_paid > 0 && direct.player.hp == hp - resolution.hp_paid as i32)
    );
    let update = dispatch_next(
        &mut vampire,
        GameCommand::CastAbility {
            ability_id: ability.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(vampire.world_tick > tick);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.cast-failure")
    );
    vampire.current_floor_id = "demo.floor.surface".to_owned();
    assert!(!vampire.dungeon_blocks_vampirism(ability));
}

#[test]
fn dungeon_anti_melee_allocation_filters_attack_spells_without_improving_player_summons() {
    let mut game = arena();
    for (kind, allowed) in [
        ("novice-mage", true),
        ("beholder", true),
        ("chameleon", true),
        ("yeti", false),
        ("shrieker-mushroom-patch", false),
        ("kobold", false),
        ("angelic-quylthulg", false),
        ("ar-pharazon-the-golden", false),
    ] {
        let actor = game.content.actor(&format!("demo.actor.{kind}")).unwrap();
        assert_eq!(
            game.dungeon_allows_monster(FLOOR, actor, false),
            allowed,
            "{kind}"
        );
        assert!(game.dungeon_allows_monster(FLOOR, actor, true));
    }
    let player_candidates =
        game.summon_category_candidate_kind_ids("any-monster", None, 80, false, true);
    let natural_candidates =
        game.summon_category_candidate_kind_ids("any-monster", None, 80, false, false);
    assert!(player_candidates.len() > natural_candidates.len());
    assert!(player_candidates.contains(&"demo.actor.kobold".to_owned()));
    assert!(!natural_candidates.contains(&"demo.actor.kobold".to_owned()));
    let mut normal = game.clone();
    normal.current_floor_id = "demo.floor.surface".to_owned();
    assert_eq!(
        player_candidates,
        normal.summon_category_candidate_kind_ids("any-monster", None, 80, false, true)
    );
    // A pet casting its own spell is still a monster summoner, not SUMMON_WHO_PLAYER.
    let spell = game
        .content
        .ability("rfb-legacy.ability.summon-demon-l42-1d3-1")
        .unwrap()
        .clone();
    for player_owned in [false, true] {
        let mut summoner = game.clone();
        if player_owned {
            summoner.entities[0].controller_id = Some(summoner.player.id.clone());
        }
        let plan = summoner.monster_ability_plan(0, spell.clone(), 1).unwrap();
        let MonsterAbilityTargetPlan::SummonCategory {
            candidate_kind_ids, ..
        } = &plan.target
        else {
            panic!("category summon plan expected")
        };
        assert!(!candidate_kind_ids.is_empty());
        assert!(!candidate_kind_ids.contains(&"demo.actor.manes".to_owned()));
        assert!(
            candidate_kind_ids
                .iter()
                .all(|id| summoner.dungeon_allows_monster(
                    FLOOR,
                    summoner.content.actor(id).unwrap(),
                    false
                ))
        );
        let result = summoner.resolve_monster_ability_plan(
            0,
            MONSTER,
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );
        assert!(!result.summon.unwrap().entity_ids.is_empty());
    }
    // Both hostile scrolls and pet scrolls use SUMMON_WHO_PLAYER in devices.c.
    normal.current_floor_id = normal
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.castle-depth-40")
        .unwrap()
        .id
        .clone();
    for kind in [
        "demo.item.summoning-scroll",
        "demo.item.pet-summoning-scroll",
    ] {
        let mut blocked = game.clone();
        let mut normal = normal.clone();
        for summon in [&mut blocked, &mut normal] {
            clear_monsters(summon);
            give_inventory_item(summon, "test.scroll", kind);
            summon.rng = RfbRng::seeded(7);
            summon
                .use_inventory_item(
                    "test.scroll",
                    None,
                    None,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
        }
        assert!(!blocked.entities.is_empty());
        assert_eq!(blocked.entities, normal.entities);
        assert_eq!(blocked.rng, normal.rng);
    }
    // Generation must use the destination floor while the player is still outside.
    let floor = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == FLOOR)
        .unwrap()
        .clone();
    game.current_floor_id = "demo.floor.surface".to_owned();
    let generated = game.generate_procedural_floor(&floor, None).unwrap();
    assert!(!generated.entities.is_empty());
    assert!(
        generated
            .entities
            .iter()
            .all(|a| game.dungeon_allows_monster(
                FLOOR,
                game.content.actor(&a.kind_id).unwrap(),
                false
            ))
    );
}

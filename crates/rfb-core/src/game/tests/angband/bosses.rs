// SPDX-License-Identifier: MPL-2.0
use super::*;

const OBERON: &str = "demo.actor.oberon-king-of-amber";
const SERPENT: &str = "demo.actor.the-serpent-of-chaos";
const JEWEL: &str = "demo.item.jewel-of-judgement";
const GROND: &str = "demo.item.grond";
const CROWN: &str = "demo.item.crown-of-chaos";

fn room(kind: &str) -> Game {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=14 {
        for x in 8..=20 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    // Only the player's survival budget is prepared; source boss HP and attacks stay intact.
    game.apply_player_melee_status(STATUS_BERSERK, 100_000, "test.ag5");
    game.player
        .statuses
        .iter_mut()
        .find(|s| s.kind_id == STATUS_BERSERK)
        .unwrap()
        .granted_modifiers
        .max_hp = 10_000;
    game.player.hp = game.effective_player_max_hp();
    game.push_generated_actor("test.ag5.boss".into(), kind, Position { x: 12, y: 10 });
    game.entities[0].nice = false;
    game.reveal_current_visibility();
    game
}

fn cast(game: &mut Game, ability_id: &str) -> MonsterAbilityPlanResolution {
    let ability = game.content.ability(ability_id).unwrap().clone();
    let plan = game.monster_ability_target_plan(0, ability, 1).unwrap();
    let kind = game.entities[0].kind_id.clone();
    game.resolve_monster_ability_plan(
        0,
        &kind,
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
}

fn artifact(game: &mut Game, kind: &str) -> String {
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 100,
        source: LootSource::MonsterDeath {
            actor_id: "test.ag5.boss".into(),
        },
    };
    let draft = game.fixed_item_draft(&context, kind.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    id
}

#[test]
fn angband_bosses_use_source_melee_and_spells_without_reducing_their_hp() {
    for (kind, hp, speed, spell) in [
        (
            OBERON,
            10_989,
            145,
            "rfb-legacy.ability.ball-mana-10d10-446",
        ),
        (
            SERPENT,
            30_000,
            155,
            "rfb-legacy.ability.ball-shards-1d1-599",
        ),
    ] {
        let mut game = room(kind);
        assert_eq!(
            (
                game.entities[0].hp,
                game.entities[0].max_hp,
                game.entities[0].speed
            ),
            (hp, hp, speed)
        );
        let before = game.player.hp;
        let result = cast(&mut game, spell);
        assert!(game.player.hp < before && !result.effects.is_empty());
        if kind == SERPENT {
            assert_eq!(before - game.player.hp, 600);
            assert!(
                game.player
                    .statuses
                    .iter()
                    .any(|s| s.kind_id == STATUS_BLEEDING)
            );
            assert!(
                game.player
                    .statuses
                    .iter()
                    .any(|s| s.kind_id == STATUS_STUN)
            );
        }
        game.entities[0].position = Position { x: 11, y: 10 };
        let before = game.player.hp;
        for _ in 0..10 {
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            if game.player.hp < before {
                break;
            }
        }
        assert!(
            game.player.hp < before,
            "{kind} must actually land source melee damage"
        );
        assert_eq!(game.entities[0].max_hp, hp);
    }
}

#[test]
fn angband_rocket_stops_at_a_creature_and_uses_shards_resistance() {
    let mut game = room(SERPENT);
    game.entities[0].position = Position { x: 14, y: 10 };
    game.push_generated_actor(
        "test.ag5.interceptor".into(),
        "demo.actor.ancient-red-dragon",
        Position { x: 12, y: 10 },
    );
    game.entities[1].controller_id = Some(game.player.id.clone());
    let hp = game.entities[1].hp;
    let before = game.player.hp;
    let result = cast(&mut game, "rfb-legacy.ability.ball-shards-1d1-599");
    assert_eq!(result.trace.unwrap().impact, Position { x: 12, y: 10 });
    assert_eq!(
        before - game.player.hp,
        200,
        "distance is measured from the explosion, not the requested target"
    );
    assert!(
        game.entities
            .iter()
            .find(|a| a.id == "test.ag5.interceptor")
            .is_none_or(|a| a.hp < hp)
    );
    let mut resisted = room(SERPENT);
    resisted
        .player
        .resistances
        .set(DamageType::Shards, ResistanceLevel::Resistant);
    let before = resisted.player.hp;
    cast(&mut resisted, "rfb-legacy.ability.ball-shards-1d1-599");
    assert!(before - resisted.player.hp < 600);
}

#[test]
fn angband_source_summons_remain_after_a_tick_and_save() {
    let mut game = room(OBERON);
    let summon = cast(&mut game, "rfb-legacy.ability.summon-demon-l99-1d3")
        .summon
        .unwrap();
    assert!(!summon.entity_ids.is_empty());
    assert_eq!(summon.duration_turns, 0);
    let mut loaded = restored(&game);
    for sample in [&mut game, &mut loaded] {
        sample.advance_summon_lifetimes(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new());
        for id in &summon.entity_ids {
            let actor = sample
                .entities
                .iter()
                .find(|actor| &actor.id == id)
                .unwrap();
            assert_eq!(actor.summon.as_ref().unwrap().remaining_turns, 0);
            assert!(!sample.actor_kind_is_reserved_task_target(&actor.kind_id));
        }
    }
    assert_eq!(game.state_hash(), loaded.state_hash());
}

#[test]
fn angband_serpent_death_drops_both_artifacts_and_remains_dead_after_save() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    complete_task_gates(&mut game);
    let task = game
        .task_states
        .get_mut("demo.task.angband-serpent-of-chaos")
        .unwrap();
    task.status = TaskStatusKindDto::Taken;
    task.current = 0;
    game.transition_floor(floor_id(100), None, None, false)
        .unwrap()
        .unwrap();
    let actor_id = game
        .entities
        .iter()
        .find(|actor| actor.kind_id == SERPENT)
        .unwrap()
        .id
        .clone();
    let mut events = task_death(&mut game, &actor_id, true);
    game.apply_campaign_events(&mut events);
    game.turn += 1;
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, DomainEvent::TaskRewarded { .. }))
            .count(),
        5
    );
    assert!(
        events
            .iter()
            .rposition(|e| matches!(e, DomainEvent::TaskRewarded { .. }))
            .unwrap()
            < events
                .iter()
                .position(|e| matches!(e, DomainEvent::LootDropped { .. }))
                .unwrap()
    );
    assert_eq!(game.defeated_limited_actor_counts.get(SERPENT), Some(&1));
    for kind in [GROND, CROWN] {
        let items = game
            .items
            .iter()
            .filter(|item| item.kind_id == kind)
            .collect::<Vec<_>>();
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0].location, ItemLocation::Ground(_)));
        assert_eq!(items[0].origin_actor_kind_id.as_deref(), Some(SERPENT));
        assert!(game.generated_artifact_ids.contains(kind));
    }
    let mut loaded = restored(&game);
    assert!(!loaded.unique_actor_kind_is_available(SERPENT));
    // A repeated drop request cannot duplicate a generated fixed artifact.
    let actor = room(SERPENT).entities.remove(0);
    assert!(
        loaded
            .generate_angband_chosen_drops(&actor)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn angband_oberon_choice_bad_luck_and_generated_choice_have_no_fallback() {
    let template = room(OBERON);
    let mut covered = BTreeSet::new();
    for (branch, roll, bad_luck, expected) in [
        (0, 32, false, Some(JEWEL)),
        (1, 49, false, Some("demo.item.amber")),
        (0, 33, false, None),
        (1, 50, false, None),
        (0, 24, true, Some(JEWEL)),
        (0, 25, true, None),
        (1, 37, true, Some("demo.item.amber")),
        (1, 38, true, None),
    ] {
        let seed = (0..100_000)
            .find(|seed| {
                let mut rng = crate::rng::RfbRng::seeded(*seed);
                rng.bounded(3) == branch && rng.bounded(100) == roll
            })
            .unwrap();
        let mut game = template.clone();
        if bad_luck {
            game.progress
                .active_mutation_ids
                .insert("rfb.mutation.bad-luck".into());
        }
        game.rng = crate::rng::RfbRng::seeded(seed);
        let actor = game.entities[0].clone();
        let items = game.generate_angband_chosen_drops(&actor).unwrap();
        assert_eq!(items.first().map(|item| item.kind_id.as_str()), expected);
        if let Some(kind) = expected {
            covered.insert(kind);
            game.rng = crate::rng::RfbRng::seeded(seed);
            let mut expected_rng = game.rng.clone();
            expected_rng.bounded(3);
            expected_rng.bounded(100);
            assert!(
                game.generate_angband_chosen_drops(&actor)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(game.rng, expected_rng);
        }
    }
    assert_eq!(covered, BTreeSet::from([JEWEL, "demo.item.amber"]));
}

#[test]
fn angband_chosen_drops_reject_pets_and_failed_placement_preserves_uniqueness() {
    let mut game = room(SERPENT);
    let mut actor = game.entities[0].clone();
    actor.controller_id = Some(game.player.id.clone());
    let rng = game.rng.clone();
    assert!(
        game.generate_angband_chosen_drops(&actor)
            .unwrap()
            .is_empty()
    );
    assert_eq!(game.rng, rng);
    actor.controller_id = None;
    game.terrain.fill("demo.terrain.wall".into());
    assert!(
        game.generate_angband_chosen_drops(&actor)
            .unwrap()
            .is_empty()
    );
    assert!(!game.generated_artifact_ids.contains(GROND));
    assert!(!game.generated_artifact_ids.contains(CROWN));
}

fn use_jewel(game: &mut Game, id: &str, recall: bool) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.use_inventory_item_with_recall(
        id,
        None,
        None,
        Some(recall),
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

#[test]
fn angband_jewel_activation_round_trips_life_cost_discovery_cooldown_and_recall_choice() {
    let mut game = room(OBERON);
    clear_monsters(&mut game);
    let trap = Position { x: 18, y: 12 };
    replace_terrain(&mut game, trap, "demo.terrain.warren-snare");
    game.revealed_terrain.remove(&trap);
    let id = artifact(&mut game, JEWEL);
    game.equip_inventory_item(&id, None).unwrap();
    let seed = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = crate::rng::RfbRng::seeded(*seed);
            use_jewel(&mut trial, &id, false)
                .iter()
                .any(|e| matches!(e, DomainEvent::ItemLifeLost { .. }))
        })
        .unwrap();
    game.rng = crate::rng::RfbRng::seeded(seed);
    let mut loaded = restored(&game);
    let before = game.player.hp;
    let events = use_jewel(&mut game, &id, false);
    assert_eq!(events, use_jewel(&mut loaded, &id, false));
    assert!(events.iter().any(
        |event| matches!(event, DomainEvent::ItemActivationDetected { resolution, .. }
        if resolution.category == "trap" && resolution.detected_positions.contains(&trap))
    ));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::ItemFloorGlowChanged { glow: true, .. }))
    );
    assert!((3..=24).contains(&(before - game.player.hp)));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::ItemLifeLost { fatal: false, .. }))
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, DomainEvent::ItemRecallStarted { .. }))
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .charges
            .as_ref()
            .unwrap()
            .current,
        0
    );
    assert_eq!(game.state_hash(), loaded.state_hash());
    let mut loaded = restored(&game);
    let before = loaded.state_hash();
    assert!(
        use_jewel(&mut loaded, &id, true)
            .iter()
            .any(|e| matches!(e, DomainEvent::ItemUseUnavailable))
    );
    assert_eq!(before, loaded.state_hash());
    let item = game.items.iter_mut().find(|item| item.id == id).unwrap();
    item.charges.as_mut().unwrap().current = 1;
    item.device_recovery_progress = 0;
    game.rng = crate::rng::RfbRng::seeded(seed);
    game.reset_recall(crate::game::floor::RecallDestination {
        dungeon_id: DUNGEON.into(),
        floor_id: "demo.floor.angband-depth-1".into(),
    });
    assert!(
        use_jewel(&mut game, &id, true)
            .iter()
            .any(|e| matches!(e, DomainEvent::ItemRecallStarted { turns: 15..=35, .. }))
    );
}

#[test]
fn angband_final_artifacts_apply_equipment_rules_and_save_them() {
    let mut game = room(SERPENT);
    clear_monsters(&mut game);
    let grond = artifact(&mut game, GROND);
    let crown = artifact(&mut game, CROWN);
    game.equip_inventory_item(&grond, None).unwrap();
    game.equip_inventory_item(&crown, None).unwrap();
    let item = game.items.iter().find(|item| item.id == grond).unwrap();
    assert!(game.item_has_weapon_trait(item, rfb_protocol::WeaponTraitDto::Impact));
    assert!(game.player_has_anti_magic());
    assert!(game.player_has_anti_teleport());
    assert!(game.player_aggravates_monsters());
    let item = game.items.iter().find(|item| item.id == crown).unwrap();
    assert_eq!(
        item.curse,
        Some(rfb_protocol::ItemCurseSeverityDto::Permanent)
    );
    let slot = match &item.location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => panic!("equipped crown"),
    };
    assert!(game.unequip_slot(&slot).is_none());
    assert!(game.player_infravision_range() >= 125);
    let loaded = restored(&game);
    assert!(loaded.player_has_anti_magic() && loaded.player_has_anti_teleport());
}

#[test]
fn angband_jewel_periodic_life_loss_is_blocked_by_anti_magic() {
    let mut game = room(OBERON);
    clear_monsters(&mut game);
    let jewel = artifact(&mut game, JEWEL);
    game.equip_inventory_item(&jewel, None).unwrap();
    game.world_tick = 10;
    let seed = (0..100_000)
        .find(|seed| crate::rng::RfbRng::seeded(*seed).bounded(999) == 0)
        .unwrap();
    let mut protected = game.clone();
    let grond = artifact(&mut protected, GROND);
    protected.equip_inventory_item(&grond, None).unwrap();
    for (sample, expected) in [(&mut game, true), (&mut protected, false)] {
        sample.rng = crate::rng::RfbRng::seeded(seed);
        let mut events = Vec::new();
        sample
            .process_equipped_curse_effects(&mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        assert_eq!(events.iter().any(|e| matches!(e, DomainEvent::ItemLifeLost { source_kind_id, amount: 1, .. } if source_kind_id == JEWEL)), expected);
    }
}

#[test]
fn angband_quest_artifacts_never_enter_the_ordinary_fixed_pool() {
    let mut game = room(SERPENT);
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 127,
        source: LootSource::MonsterDeath {
            actor_id: "test.ag5.boss".into(),
        },
    };
    for base in ["demo.item.mighty-hammer", "demo.item.massive-iron-crown"] {
        assert_eq!(
            game.roll_fixed_artifact_kind_id(&context, Some(base), true),
            None
        );
    }
    assert!(
        !game.generated_artifact_ids.contains(GROND)
            && !game.generated_artifact_ids.contains(CROWN)
    );
}

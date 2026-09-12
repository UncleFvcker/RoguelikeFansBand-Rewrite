// SPDX-License-Identifier: MPL-2.0
use super::*;

const DUNGEON: &str = "demo.dungeon.asgard";
const ODIN: &str = "demo.actor.odin-the-all-father";
const HEIMDALL: &str = "demo.actor.heimdall-guardian-of-bifrost";
const ENTRANCE: &str = "demo.guardian.asgard-entrance.1";

fn generation_game(active: bool) -> Game {
    let mut game = (0..32)
        .map(|seed| Game::new_with_build(seed, "demo.build.warrior").unwrap())
        .find(|game| (game.active_pantheons & 8 != 0) == active)
        .unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game
}

fn enter_site(game: &mut Game) {
    dispatch_next(
        game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    // Only the overland coordinate is prepared; local generation and projection
    // use the real world-map command. Full combat/travel belongs to AS6.
    game.wilderness_position = Some(Position { x: 94, y: 11 });
    dispatch_next(game, GameCommand::LeaveWorldMap);
}

#[test]
fn asgard_formal_surface_entry_obeys_birth_pantheon_and_preserves_unique_identity() {
    for active in [true, false] {
        let mut game = generation_game(active);
        assert_eq!(game.active_pantheons.count_ones(), 2);
        assert_eq!(
            game.wilderness_cell_dto(Position { x: 94, y: 11 })
                .locations
                .iter()
                .any(|location| location.id == DUNGEON),
            active
        );
        enter_site(&mut game);
        assert_eq!(
            game.terrain
                .iter()
                .any(|id| id == "demo.terrain.asgard-entrance"),
            active
        );
        assert_eq!(
            game.entities
                .iter()
                .filter(|actor| actor.id == ENTRANCE)
                .count(),
            usize::from(active)
        );
        if active {
            assert_eq!(
                game.entities
                    .iter()
                    .find(|actor| actor.id == ENTRANCE)
                    .unwrap()
                    .kind_id,
                HEIMDALL
            );
        }
        let saved = game.to_save();
        let mut restored = Game::from_save(saved.clone()).unwrap();
        assert_eq!(restored.to_save(), saved);
        enter_site(&mut restored);
        assert_eq!(
            restored
                .entities
                .iter()
                .filter(|actor| actor.kind_id == HEIMDALL)
                .count(),
            usize::from(active)
        );
        if active {
            deaths::death(&mut restored, ENTRANCE, true);
            assert!(restored.dungeon_states[DUNGEON].entrance_guardian_defeated);
            assert!(!restored.dungeon_states[DUNGEON].guardian_defeated);
            enter_site(&mut restored);
            assert!(
                !restored
                    .entities
                    .iter()
                    .any(|actor| actor.kind_id == HEIMDALL)
            );
            // Only admission is under test: keep entry-turn high-level attacks
            // from killing this level-one fixture before the dungeon assertion.
            restored.apply_player_melee_status(STATUS_INVULNERABILITY, 10_000, "test.asgard.entry");
            restored
                .player
                .statuses
                .iter_mut()
                .find(|status| status.kind_id == STATUS_INVULNERABILITY)
                .unwrap()
                .incoming_damage_percent = 0;
            place_player_on_terrain(&mut restored, "demo.terrain.asgard-entrance");
            assert_eq!(
                dispatch_next(&mut restored, GameCommand::TraverseStairs).floor_id,
                "demo.floor.asgard-depth-64"
            );
            assert!(!restored.player_is_dead());
            assert!(Game::from_save(restored.to_save()).is_ok());
        } else {
            assert!(
                restored
                    .transition_floor("demo.floor.asgard-depth-64".into(), None, None, false)
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[test]
fn asgard_outside_unique_deaths_do_not_conquer_or_respawn_at_entrance_or_bottom() {
    let mut game = generation_game(true);
    // Actual death consumer, with synthetic placement/status death to isolate
    // global quota from player accuracy. No dungeon flags are assigned by tests.
    for (id, kind) in [
        ("test.outside-heimdall", HEIMDALL),
        ("test.outside-odin", ODIN),
    ] {
        game.push_generated_actor(id.into(), kind, Position { x: 11, y: 10 });
        deaths::death(&mut game, id, true);
        assert!(!game.unique_actor_kind_is_available(kind));
    }
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
    assert!(!game.dungeon_states[DUNGEON].entrance_guardian_defeated);
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.kind_id == "demo.item.acquirement-scroll")
    );
    clear_monsters(&mut game);
    enter_site(&mut game);
    assert!(!game.entities.iter().any(|actor| actor.kind_id == HEIMDALL));
    assert!(
        game.terrain
            .iter()
            .any(|id| id == "demo.terrain.asgard-entrance")
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert!(
        restored
            .transition_floor("demo.floor.asgard-depth-88".into(), None, None, false)
            .unwrap()
            .is_some()
    );
    assert!(!restored.entities.iter().any(|actor| actor.kind_id == ODIN));
    assert!(!restored.dungeon_states[DUNGEON].guardian_defeated);
    assert!(Game::from_save(restored.to_save()).is_ok());
}

#[test]
fn asgard_early_odin_death_keeps_conquest_scroll_artifact_and_avenger_independent() {
    let mut game = generation_game(true);
    game.transition_floor("demo.floor.asgard-depth-80".into(), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    // Prepare open summon space and search a bounded RNG branch through the
    // real hostile Norse category consumer; do not inject the guardian itself.
    game.player.position = Position { x: 10, y: 10 };
    for y in 6..=16 {
        for x in 6..=18 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    let base = game.clone();
    let mut selected = None;
    for seed in 0..128 {
        let mut attempt = base.clone();
        attempt.rng = RfbRng::seeded(seed);
        attempt.summon_hostile_category_at(
            HEIMDALL,
            "norse",
            100,
            true,
            Position { x: 12, y: 10 },
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );
        if let Some(odin) = attempt.entities.iter().find(|actor| actor.kind_id == ODIN) {
            selected = Some((odin.id.clone(), attempt));
            break;
        }
    }
    let (odin, summoned) = selected.expect("a Norse summon should select Odin");
    game = summoned;
    deaths::death(&mut game, &odin, true);
    assert!(game.dungeon_states[DUNGEON].guardian_defeated);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.runespear")
            .count(),
        1
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.acquirement-scroll")
            .count(),
        1
    );
    assert!(game.items.iter().any(|item| !matches!(
        item.kind_id.as_str(),
        "demo.item.runespear" | "demo.item.acquirement-scroll"
    )));
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == "demo.actor.vidarr-the-silent-avenger")
            .count(),
        1
    );
    // Keep the avenger alive in a saved sleep preparation while isolating
    // reward use; full-HP combat is exercised separately by AS6.
    for actor in &mut game.entities {
        apply_status(
            &mut actor.statuses,
            monster_combat::melee_status(STATUS_SLEEP, 200_000, "test.asgard.reward"),
        );
    }
    let scroll = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.acquirement-scroll")
        .unwrap();
    let id = scroll.id.clone();
    let ItemLocation::Ground(position) = scroll.location else {
        panic!("reward must drop");
    };
    game.player.position = position;
    game.pick_up_item_at_player(Some(&id)).unwrap();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        dispatch_next(
            game,
            GameCommand::UseItem {
                item_id: id.clone(),
                target: None,
            },
        );
        assert!(!game.items.iter().any(|item| item.id == id));
        assert!(
            game.items
                .iter()
                .any(|item| item.origin_kind == Some(rfb_protocol::ItemOriginKindDto::Acquire))
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    for depth in [88, 80, 88] {
        game.transition_floor(
            format!("demo.floor.asgard-depth-{depth}"),
            None,
            None,
            false,
        )
        .unwrap()
        .unwrap();
        assert!(!game.entities.iter().any(|actor| actor.kind_id == ODIN));
        assert!(
            !game
                .items
                .iter()
                .any(|item| item.kind_id == "demo.item.acquirement-scroll")
        );
    }
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn asgard_norse_ecology_uses_source_divisor_and_actual_summon_consumer() {
    let mut game = generation_game(true);
    let policy = game
        .content
        .encounter_table("demo.encounter-table.asgard")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    for kind in [ODIN, "demo.actor.loki-the-trickster", "demo.actor.valkyrie"] {
        let actor = game.content.actor(kind).unwrap().clone();
        let before = game.rng.clone();
        assert_eq!(
            game.original_dungeon_weight(&actor, &policy),
            100 / actor.allocation.as_ref().unwrap().rarity
        );
        assert_eq!(game.rng, before);
    }
    let newt = game.content.actor("demo.actor.newt").unwrap().clone();
    let numerator = 2 * (100 / newt.allocation.as_ref().unwrap().rarity);
    for rounded in [false, true] {
        let seed = (0..1000)
            .find(|seed| (RfbRng::seeded(*seed).bounded(64) < u64::from(numerator % 64)) == rounded)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        game.monster_division_remainders.clear();
        assert_eq!(
            game.original_dungeon_weight(&newt, &policy),
            numerator / 64 + u32::from(rounded)
        );
        let after = game.rng.clone();
        game.original_dungeon_weight(&newt, &policy);
        assert_eq!(game.rng, after);
    }
    for active in [true, false] {
        let mut game = generation_game(active);
        let odin = game.content.actor(ODIN).unwrap();
        let loki = game.content.actor("demo.actor.loki-the-trickster").unwrap();
        let valkyrie = game.content.actor("demo.actor.valkyrie").unwrap();
        assert_eq!(
            game.pantheon_allows_allocation("demo.floor.asgard-depth-80", odin),
            active
        );
        assert!(!game.pantheon_allows_allocation("demo.floor.castle-depth-40", odin));
        assert_eq!(
            game.pantheon_allows_allocation("demo.floor.castle-depth-40", loki),
            active
        );
        assert!(!game.pantheon_allows_allocation("demo.floor.mount-olympus-depth-80", loki));
        assert!(game.pantheon_allows_allocation("demo.floor.castle-depth-40", valkyrie));
        let mut seen_secondary = false;
        for _ in 0..64 {
            let kind = game
                .select_original_allocated_monster(
                    "demo.floor.asgard-depth-80",
                    &policy,
                    88,
                    80,
                    None,
                    &[],
                    None,
                    None,
                )
                .unwrap();
            assert_ne!(
                kind, ODIN,
                "ordinary allocation excludes the configured final guardian"
            );
            let actor = game.content.actor(&kind).unwrap();
            assert!(game.pantheon_allows_allocation("demo.floor.asgard-depth-80", actor));
            seen_secondary |= actor.tags.iter().any(|tag| tag == "norse2");
        }
        assert!(seen_secondary);
        // Synthetic open arena isolates summoning; this is not a saved floor.
        game.current_floor_id = if active {
            "demo.floor.asgard-depth-80"
        } else {
            "demo.floor.castle-depth-40"
        }
        .into();
        let candidates = game.summon_category_candidate_kind_ids("norse", None, 100, true, false);
        assert!(!candidates.is_empty());
        game.terrain.fill("demo.terrain.floor".into());
        game.player.position = Position { x: 80, y: 20 };
        game.push_generated_actor("test.odin-caster".into(), ODIN, Position { x: 20, y: 20 });
        let ability = game
            .content
            .ability("rfb-legacy.ability.summon-norse-l90-1d2")
            .unwrap()
            .clone();
        let plan = game.monster_ability_target_plan(0, ability, 1).unwrap();
        let summoned = game
            .resolve_monster_ability_plan(
                0,
                ODIN,
                &plan,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .summon
            .unwrap();
        assert!((1..=2).contains(&summoned.summoned_kind_ids.len()));
        for kind in summoned.summoned_kind_ids {
            assert_ne!(kind, ODIN);
            assert!(candidates.contains(&kind));
            let actor = game.content.actor(&kind).unwrap();
            assert!(actor.tags.iter().any(|tag| tag == "unique"));
            assert_eq!(actor.tags.iter().any(|tag| tag == "norse"), active);
        }
    }
}

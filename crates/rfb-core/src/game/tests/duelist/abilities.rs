// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::projectile_geometry::rfb_distance;
use crate::game::tests::support::replace_terrain;
use crate::game::visibility::has_line_of_sight;

fn cast(game: &mut Game, slug: &str) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        &format!("demo.ability.duelist-{slug}"),
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn sturdy_target(game: &mut Game, offset: i32) {
    target(game, "test.opponent", offset);
    game.entities.last_mut().unwrap().hp = 10_000;
    game.entities.last_mut().unwrap().max_hp = 10_000;
    game.duelist_target_id = Some("test.opponent".to_owned());
}

fn weapon_passive(game: &mut Game, passive: EquipmentPassive) {
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.rapier")
        .unwrap()
        .intrinsic_properties
        .passives
        .insert(passive);
}

#[test]
fn strafing_uses_los_without_a_challenge_and_disengage_clears_even_when_blocked() {
    let mut game = at_level(24);
    let from = game.player.position;
    let hp = game.player.hp;
    cast(&mut game, "strafing");
    assert_ne!(game.player.position, from);
    assert!(rfb_distance(from, game.player.position) <= 10);
    assert!(has_line_of_sight(&game, from, game.player.position));
    assert_eq!(game.player.hp, hp - 8);
    assert_eq!(game.duelist_target_id, None);
    sturdy_target(&mut game, 2);
    weapon_passive(&mut game, EquipmentPassive::AntiTeleport);
    let from = game.player.position;
    let hp = game.player.hp;
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(100); // The legal zero-failure cast still draws once.
    cast(&mut game, "disengage");
    assert_eq!(game.player.position, from);
    assert_eq!(game.player.hp, hp - 25);
    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.duelist_target_id, None);
}

#[test]
fn charge_attacks_the_interceptor_but_acrobatics_swaps_and_wakes_it() {
    for (slug, level, cost, swapped) in
        [("charge", 8, 10, false), ("acrobatic-charge", 32, 30, true)]
    {
        let mut game = at_level(level);
        sturdy_target(&mut game, 4);
        target(&mut game, "test.blocker", 2);
        game.entities[1]
            .statuses
            .push(monster_combat::melee_status(STATUS_SLEEP, 100, "test").status);
        let origin = game.player.position;
        let hp = game.player.hp;
        cast(&mut game, slug);
        assert_eq!(
            game.player.position,
            Position {
                x: origin.x + if swapped { 3 } else { 1 },
                y: origin.y
            }
        );
        assert_eq!(game.player.hp, hp - cost);
        if swapped {
            let blocker = game
                .entities
                .iter()
                .find(|actor| actor.id == "test.blocker")
                .unwrap();
            assert_eq!(
                blocker.position,
                Position {
                    x: origin.x + 1,
                    y: origin.y
                }
            );
            assert!(
                !blocker
                    .statuses
                    .iter()
                    .any(|status| status.kind_id == STATUS_SLEEP)
            );
            assert!(game.entities[0].hp < 10_000);
        } else {
            assert_eq!(game.entities[0].hp, 10_000);
        }
    }
}

#[test]
fn phase_charge_crosses_ordinary_walls_but_stops_at_permanent_terrain() {
    for permanent in [false, true] {
        let mut game = at_level(48);
        sturdy_target(&mut game, 4);
        weapon_passive(&mut game, EquipmentPassive::Telepathy);
        game.entities[0].visible_weird_mind = true;
        let origin = game.player.position;
        replace_terrain(
            &mut game,
            Position {
                x: origin.x + 2,
                y: origin.y,
            },
            if permanent {
                "demo.terrain.permanent-wall"
            } else {
                "demo.terrain.wall"
            },
        );
        assert!(game.entity_is_visible_to_player(&game.entities[0]));
        let hp = game.player.hp;
        cast(&mut game, "phase-charge");
        assert_eq!(
            game.player.position.x,
            origin.x + if permanent { 1 } else { 3 }
        );
        assert_eq!(game.player.hp, hp - 80);
    }
}

#[test]
fn isolation_keeps_the_marked_instance_and_darting_duel_angers_then_strafes() {
    let mut game = at_level(45);
    sturdy_target(&mut game, 3);
    target(&mut game, "test.other", -2);
    let opponent_position = game.entities[0].position;
    let other_position = game.entities[1].position;
    let hp = game.player.hp;
    cast(&mut game, "isolation");
    assert_eq!(game.entities[0].position, opponent_position);
    assert_ne!(game.entities[1].position, other_position);
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.opponent"));
    assert_eq!(game.player.hp, hp - 60);
    game.entities[0].anger = 20;
    let events = cast(&mut game, "darting-duel");
    assert_eq!(game.entities[0].anger, 40);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityTeleported { .. }))
    );
    assert_eq!(game.player.hp, hp - 120);
}

#[test]
fn reduced_hp_cost_matches_projection_and_only_standalone_strafing_uses_thirty_energy() {
    let mut game = at_level(45);
    weapon_passive(&mut game, EquipmentPassive::ReducedManaCost);
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.duelist-strafing")
        .unwrap();
    assert_eq!(ability.hit_point_cost, 6);
    let hp = game.player.hp;
    cast(&mut game, "strafing");
    assert_eq!(game.player.hp, hp - 6);
    let mut normal = at_level(16);
    let mut guide = normal.clone();
    guide
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.astral-guide".to_owned());
    let cmd = GameCommand::CastAbility {
        ability_id: "demo.ability.duelist-strafing".to_owned(),
        target: TargetSelection::SelfTarget,
    };
    let before = normal.world_tick;
    dispatch_next(&mut normal, cmd.clone());
    dispatch_next(&mut guide, cmd);
    assert_eq!(normal.world_tick - before, 10);
    assert_eq!(guide.world_tick - before, 3);
}

#[test]
fn class_hp_boundary_rejects_insufficient_health_without_a_draw_and_allows_exact_payment() {
    let mut game = at_level(16);
    game.player.hp = 7;
    let rng = game.rng.clone();
    let position = game.player.position;
    cast(&mut game, "strafing");
    assert_eq!(game.rng, rng);
    assert_eq!(game.player.hp, 7);
    assert_eq!(game.player.position, position);
    game.player.hp = 8;
    cast(&mut game, "strafing");
    assert_eq!(game.player.hp, 0);
    assert_ne!(game.player.position, position);
}

#[test]
fn charge_triggers_only_landing_traps_and_landing_death_stops_the_attack() {
    let mut game = at_level(8);
    sturdy_target(&mut game, 4);
    let origin = game.player.position;
    for offset in [1, 3] {
        replace_terrain(
            &mut game,
            Position {
                x: origin.x + offset,
                y: origin.y,
            },
            "demo.terrain.warren-snare",
        );
    }
    let hp = game.player.hp;
    let events = cast(&mut game, "charge");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::TrapTriggered { .. }))
            .count(),
        1
    );
    assert_eq!(game.player.hp, hp - 12);
    game.player.position = origin;
    game.player.hp = 1;
    let target_hp = game.entities[0].hp;
    let ability = game
        .content
        .ability("demo.ability.duelist-charge")
        .unwrap()
        .clone();
    let mut events = Vec::new();
    game.resolve_player_ability_effect(
        ability,
        crate::game::abilities::AbilityTargetPlan::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.player_is_dead());
    assert_eq!(game.entities[0].hp, target_hp);
    assert!(game.pending_duelist.is_none());
}

#[test]
fn darting_duel_strafes_after_an_interceptor_but_not_after_replacing_the_killed_mark() {
    let mut game = at_level(45);
    sturdy_target(&mut game, 4);
    target(&mut game, "test.interceptor", 2);
    game.entities[1].hp = 100;
    game.entities[1].max_hp = 100;
    let events = cast(&mut game, "darting-duel");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityTeleported { .. }))
    );
    let mut game = at_level(45);
    target(&mut game, "test.dying", 2);
    target(&mut game, "test.next", 4);
    game.entities[0].hp = 1;
    game.duelist_target_id = Some("test.dying".to_owned());
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.duelist-darting-duel".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    let result = dispatch_next(
        &mut game,
        GameCommand::ResolveDuelistChoice {
            choice: rfb_protocol::DuelistChoiceDto::Challenge {
                entity_id: Some("test.next".to_owned()),
            },
        },
    );
    assert!(
        !result
            .events
            .iter()
            .any(|event| event.kind == "ability.teleported")
    );
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.next"));
}

#[test]
fn isolation_uses_dex_saves_only_for_resistant_nonunique_targets() {
    for tags in [
        vec!["unique"],
        vec!["unique", "resist-teleport"],
        vec!["guardian", "resist-teleport"],
    ] {
        let content =
            super::super::support::game_with_actor_definition(923, "demo.actor.sheep", |actor| {
                actor.tags.extend(tags.iter().map(|tag| (*tag).to_owned()));
            })
            .content;
        let mut game =
            Game::from_content_with_build(923, content, DEFAULT_WORLD_ID, BUILD).unwrap();
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 77, y: 33 };
        game.apply_player_experience(game.experience_required_for_level(40), &mut Vec::new());
        super::super::support::choose_human_talent_if_pending(&mut game);
        game.player.hp = game.effective_player_max_hp();
        target(&mut game, "test.isolate", 2);
        let marked = game.generated_actor(
            "test.mark".to_owned(),
            "demo.actor.small-kobold",
            Position { x: 75, y: 33 },
        );
        game.entities.push(marked);
        game.duelist_target_id = Some("test.mark".to_owned());
        let before = game.entities[0].position;
        let mut oracle = game.clone();
        oracle.rng.bounded(100);
        let resistant = tags.contains(&"resist-teleport");
        let immune = resistant && tags.contains(&"unique");
        let expected = immune || (resistant && oracle.duelist_monster_saves(0));
        let events = cast(&mut game, "isolation");
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. }
            if resolution.effects.iter().any(|effect| matches!(effect, AbilityEffectResolutionDto::TeleportAway { resisted, .. } if *resisted == expected)))));
        assert_eq!(game.entities[0].position == before, expected);
        if expected {
            assert_eq!(game.rng, oracle.rng);
        }
    }
}

#[test]
fn teleport_landing_rejects_traps_and_vaults_but_passive_pursuit_can_enter_deep_water() {
    let mut game = at_level(30);
    let position = Position { x: 80, y: 33 };
    let index = game.index(position).unwrap();
    replace_terrain(&mut game, position, "demo.terrain.warren-snare");
    assert!(!game.player_can_teleport_to(position, true));
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.vault_cells[index] = true;
    assert!(!game.player_can_teleport_to(position, true));
    game.vault_cells[index] = false;
    replace_terrain(&mut game, position, "demo.terrain.surface-water-deep");
    assert!(!game.player_can_teleport_to(position, false));
    assert!(game.player_can_teleport_to(position, true));
    weapon_passive(&mut game, EquipmentPassive::Levitation);
    assert!(game.player_can_teleport_to(position, false));
}

#[test]
fn disengage_excludes_even_the_sleeping_monkey_clone_that_follows_strafing() {
    let mut base = Game::new_with_build(923, BUILD).unwrap();
    super::super::support::descend_one_floor(&mut base);
    clear_monsters(&mut base);
    base.apply_player_experience(base.experience_required_for_level(24), &mut Vec::new());
    super::super::support::choose_human_talent_if_pending(&mut base);
    base.player.hp = base.effective_player_max_hp();
    base.terrain.fill("demo.terrain.floor".to_owned());
    base.player.position = Position {
        x: i32::from(base.width / 2),
        y: i32::from(base.height / 2),
    };
    let position = Position {
        x: base.player.position.x + 1,
        y: base.player.position.y,
    };
    let mut clone =
        base.generated_actor("test.clone".to_owned(), "demo.actor.monkey-clone", position);
    clone
        .statuses
        .push(monster_combat::melee_status(STATUS_SLEEP, 100, "test").status);
    base.entities.push(clone);
    base.duelist_target_id = Some("test.clone".to_owned());
    for slug in ["strafing", "disengage"] {
        let mut game = base.clone();
        cast(&mut game, slug);
        if slug == "disengage" {
            assert_eq!(game.entities[0].position, position);
            assert_eq!(game.duelist_target_id, None);
        } else {
            assert_ne!(game.entities[0].position, position);
            assert!(rfb_distance(game.entities[0].position, game.player.position) <= 2);
        }
    }
}

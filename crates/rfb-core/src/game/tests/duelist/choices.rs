// SPDX-License-Identifier: MPL-2.0
use super::*;
use rfb_protocol::{DuelistChoiceDto, DuelistContinuationDto, DuelistPromptDto};

fn confirm(accepted: bool) -> GameCommand {
    GameCommand::ResolveDuelistChoice {
        choice: DuelistChoiceDto::Confirm { accepted },
    }
}

fn choose(id: Option<&str>) -> GameCommand {
    GameCommand::ResolveDuelistChoice {
        choice: DuelistChoiceDto::Challenge {
            entity_id: id.map(str::to_owned),
        },
    }
}

fn cast(id: &str) -> GameCommand {
    GameCommand::CastAbility {
        ability_id: id.to_owned(),
        target: TargetSelection::SelfTarget,
    }
}

#[test]
fn over_range_confirmation_is_free_cancellable_and_round_trips_before_payment() {
    let mut game = at_level(8);
    target(&mut game, "test.far", 7);
    game.duelist_target_id = Some("test.far".to_owned());
    let before = (
        game.player.hp,
        game.world_tick,
        game.player.position,
        game.rng.clone(),
    );
    dispatch_next(&mut game, cast("demo.ability.duelist-charge"));
    assert!(matches!(
        game.duelist_prompt(),
        Some(DuelistPromptDto::Charge {
            distance: 7,
            range: 5,
            ..
        })
    ));
    assert_eq!(
        (
            game.player.hp,
            game.world_tick,
            game.player.position,
            game.rng.clone()
        ),
        before
    );
    let saved = game.to_save();
    let mut restored = Game::from_save(saved.clone()).unwrap();
    let invalid = game.dispatch(command(
        game.last_command_seq + 1,
        game.revision,
        choose(None),
    ));
    assert!(matches!(invalid, Err(CoreError::DuelistChoiceUnavailable)));
    assert_eq!(game.to_save(), saved);
    assert_eq!(
        dispatch_next(&mut game, confirm(false)),
        dispatch_next(&mut restored, confirm(false))
    );
    assert_eq!(
        (
            game.player.hp,
            game.world_tick,
            game.player.position,
            game.rng.clone()
        ),
        before
    );
    dispatch_next(&mut game, cast("demo.ability.duelist-charge"));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        dispatch_next(&mut game, confirm(true)),
        dispatch_next(&mut restored, confirm(true))
    );
    assert_eq!(game.player.position.x, before.2.x + 5);
    assert_eq!(game.player.hp, before.0 - 10);
    assert!(game.pending_duelist.is_none());
    assert_eq!(game.to_save(), restored.to_save());
}

#[test]
fn weapon_kill_waits_before_class_hp_and_world_then_free_challenge_resumes_once() {
    for next in [None, Some("test.next")] {
        let mut game = at_level(35);
        target(&mut game, "test.dying", 2);
        target(&mut game, "test.next", 4);
        if next.is_some() {
            game.entities[1].controller_id = Some(game.player.id.clone());
        }
        game.entities[0].hp = 1;
        game.duelist_target_id = Some("test.dying".to_owned());
        let hp = game.player.hp;
        let tick = game.world_tick;
        let start = dispatch_next(&mut game, cast("demo.ability.duelist-charge"));
        assert!(
            matches!(game.duelist_prompt(), Some(DuelistPromptDto::Challenge)),
            "{start:?}"
        );
        assert_eq!(game.player.hp, hp);
        assert_eq!(game.world_tick, tick);
        assert!(
            !start.events.iter().any(|event| matches!(
                event.outcome,
                Some(GameEventOutcomeDto::AbilityCast { .. })
            ))
        );
        let saved = game.to_save();
        let mut bad_cost = game.clone();
        if let Some(DuelistContinuationDto::ClassCast { hit_point_cost, .. }) = bad_cost
            .pending_duelist
            .as_mut()
            .unwrap()
            .continuations
            .iter_mut()
            .find(|frame| matches!(frame, DuelistContinuationDto::ClassCast { .. }))
        {
            *hit_point_cost += 1;
        }
        assert!(Game::from_save(bad_cost.to_save()).is_err());
        let mut restored = Game::from_save(saved.clone()).unwrap();
        assert!(
            game.dispatch(command(
                game.last_command_seq + 1,
                game.revision,
                GameCommand::Wait
            ))
            .is_err()
        );
        assert_eq!(game.to_save(), saved);
        let result = dispatch_next(&mut game, choose(next));
        assert_eq!(result, dispatch_next(&mut restored, choose(next)));
        assert_eq!(game.player.hp, hp - 10);
        assert!(game.world_tick > tick);
        assert_eq!(game.duelist_target_id.as_deref(), next);
        assert_eq!(game.to_save(), restored.to_save());
        assert!(game.pending_duelist.is_none());
        assert_eq!(
            result
                .events
                .iter()
                .filter(|event| event.kind == "ability.cast-success")
                .count(),
            1
        );
    }
}

#[test]
fn free_challenge_survives_town_actor_storage_during_charge_scroll() {
    let mut game = at_level(35);
    game.player.position = Position { x: 40, y: 20 };
    target(&mut game, "test.dying", 2);
    target(&mut game, "test.next", -2);
    game.entities[0].hp = 1;
    game.duelist_target_id = Some("test.dying".to_owned());
    dispatch_next(&mut game, cast("demo.ability.duelist-charge"));
    assert!(matches!(
        game.duelist_prompt(),
        Some(DuelistPromptDto::Challenge)
    ));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        dispatch_next(&mut game, choose(Some("test.next"))),
        dispatch_next(&mut restored, choose(Some("test.next")))
    );
    assert!(game.entities.iter().any(|actor| actor.id == "test.next"));
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.next"));
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn tampered_prompts_callers_and_costs_are_rejected_on_load() {
    let mut game = at_level(8);
    target(&mut game, "test.far", 7);
    game.duelist_target_id = Some("test.far".to_owned());
    dispatch_next(&mut game, cast("demo.ability.duelist-charge"));
    let mut invalid = game.clone();
    invalid.pending_duelist.as_mut().unwrap().prompt = Some(DuelistPromptDto::BlockTeleport {
        source_entity_id: "test.far".to_owned(),
    });
    assert!(Game::from_save(invalid.to_save()).is_err());
    let mut invalid = game.clone();
    invalid
        .pending_duelist
        .as_mut()
        .unwrap()
        .continuations
        .push(DuelistContinuationDto::PlayerAction {
            energy_cost: 0,
            recover_after_wait: false,
            pet_neglect_allowed: false,
            visible_auras_before: Vec::new(),
        });
    assert!(Game::from_save(invalid.to_save()).is_err());
    game.pending_duelist.as_mut().unwrap().command_completion = None;
    assert!(Game::from_save(game.to_save()).is_err());
}

fn teleport_caster(ability_id: &str) -> Game {
    let content = super::super::support::game_with_actor_definition(
        923,
        "demo.actor.agent-of-benedict",
        |actor| {
            actor.force_sleep = false;
            actor.monster_casting.as_mut().unwrap().frequency_percent = 100;
            let mut candidate = actor.monster_casting.as_ref().unwrap().abilities[0].clone();
            candidate.ability_id = ability_id.to_owned();
            actor.monster_casting.as_mut().unwrap().abilities = vec![candidate];
        },
    )
    .content;
    let mut game = Game::from_content_with_build(923, content, DEFAULT_WORLD_ID, BUILD).unwrap();
    clear_monsters(&mut game);
    game.player.position = Position { x: 77, y: 33 };
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.apply_player_experience(game.experience_required_for_level(30), &mut Vec::new());
    super::super::support::choose_human_talent_if_pending(&mut game);
    game.player.hp = game.effective_player_max_hp();
    let mut caster = game.generated_actor(
        "test.caster".to_owned(),
        "demo.actor.agent-of-benedict",
        Position { x: 81, y: 33 },
    );
    caster.energy_need = 0;
    caster.alerted = true;
    caster.nice = false;
    caster.statuses.clear();
    game.entities.push(caster);
    game.duelist_target_id = Some("test.caster".to_owned());
    game
}

#[test]
fn teleport_block_and_follow_pause_the_monster_pulse_and_restore_exactly() {
    let mut ordinary = Game::new_with_build(923, "demo.build.warrior").unwrap();
    clear_monsters(&mut ordinary);
    target(&mut ordinary, "test.caster", 2);
    ordinary
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.teleport".to_owned());
    assert!(
        !ordinary.duelist_can_follow_teleport(0, false),
        "closed Duelist UI must not block an existing class"
    );
    for ability in ["rfb-legacy.ability.banish", "rfb-legacy.ability.escape"] {
        let base = teleport_caster(ability);
        for accepted in [false, true] {
            let mut game = base.clone();
            target(&mut game, "test.z-after", -4);
            let follower_energy = game.entities[1].energy_need;
            let first = dispatch_next(&mut game, GameCommand::Wait);
            assert!(game.duelist_prompt().is_some(), "{ability}: {first:?}");
            assert_eq!(
                game.entities[1].energy_need, follower_energy,
                "later actors must wait"
            );
            assert!(
                game.pending_duelist
                    .as_ref()
                    .unwrap()
                    .continuations
                    .iter()
                    .any(|frame| matches!(frame, DuelistContinuationDto::WorldTick { .. }))
            );
            assert!(
                !first
                    .events
                    .iter()
                    .any(|event| event.kind == "monster.ability-cast")
            );
            let saved = game.to_save();
            let mut restored =
                Game::from_save_with_content(saved.clone(), game.content.clone()).unwrap();
            let mut invalid = game.clone();
            invalid.entities.retain(|actor| actor.id != "test.caster");
            assert!(
                Game::from_save_with_content(invalid.to_save(), invalid.content.clone()).is_err()
            );
            let result = dispatch_next(&mut game, confirm(accepted));
            assert_eq!(
                result,
                dispatch_next(&mut restored, confirm(accepted)),
                "{ability}, {accepted}"
            );
            assert_eq!(game.to_save(), restored.to_save());
            assert!(game.world_tick > saved.world_tick);
            if accepted {
                assert!(result.events.iter().any(|event| event.kind
                    == if ability.ends_with("banish") {
                        "duelist.block-teleport"
                    } else {
                        "duelist.follow-teleport"
                    }));
            }
        }
    }
}

#[test]
fn teleport_choices_keep_the_one_in_three_failure_and_charge_follow_even_when_anchored() {
    for ability in ["rfb-legacy.ability.banish", "rfb-legacy.ability.escape"] {
        let mut base = teleport_caster(ability);
        dispatch_next(&mut base, GameCommand::Wait);
        base.entities[0].casting_cooldown_remaining = 1000;
        for success in [false, true] {
            let mut game = base.clone();
            let seed = (0..100)
                .find(|seed| (crate::rng::RfbRng::seeded(*seed).bounded(3) != 0) == success)
                .unwrap();
            game.rng = crate::rng::RfbRng::seeded(seed);
            let from = game.player.position;
            if ability.ends_with("escape") {
                game.items
                    .iter_mut()
                    .find(|item| item.kind_id == "demo.item.rapier")
                    .unwrap()
                    .intrinsic_properties
                    .passives
                    .insert(EquipmentPassive::AntiTeleport);
            }
            let mut declined = game.clone();
            let mut ignored_events = Vec::new();
            declined
                .resolve_duelist_choice(
                    DuelistChoiceDto::Confirm { accepted: false },
                    &mut ignored_events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                    &mut 0,
                )
                .unwrap();
            let mut events = Vec::new();
            game.resolve_duelist_choice(
                DuelistChoiceDto::Confirm { accepted: true },
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
                &mut 0,
            )
            .unwrap();
            if ability.ends_with("banish") {
                assert!(events.iter().any(|event| matches!(event, DomainEvent::DuelistTeleportBlocked { succeeded, .. } if *succeeded == success)));
                assert_eq!(game.player.position == from, success);
            } else {
                assert!(events.iter().any(|event| matches!(event, DomainEvent::DuelistFollowedTeleport { succeeded, .. } if *succeeded == success)));
                assert_eq!(game.player.position, from);
                assert_eq!(game.world_tick, declined.world_tick + 10);
            }
        }
    }
}

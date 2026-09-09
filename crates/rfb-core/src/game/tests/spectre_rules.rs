// SPDX-License-Identifier: MPL-2.0
use super::spectre_supplies::birth;
use super::support::*;
use super::*;

const SPECTRE: &str = "rfb-legacy.race.spectre";
const HUMAN: &str = "demo.race.rfb-human";
const SCARE: &str = "rfb.ability.race.scare-monster";
const START: Position = Position { x: 48, y: 16 };
const EAST: Position = Position { x: 49, y: 16 };

fn form(game: &mut Game, race: &str) {
    let mut status =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.spectre").status;
    status.granted_race_id = Some(race.to_owned());
    game.player.statuses.push(status);
    game.refresh_player_resource_maxima();
}

fn ready() -> Game {
    let mut game = birth(83, "demo.build.high-mage-death");
    clear_monsters(&mut game);
    game.player.position = START;
    for y in 14..=18 {
        for x in 46..=52 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(4),
        &mut Vec::new(),
    );
    game.refresh_player_resource_maxima();
    game.player.hp = game.effective_player_max_hp();
    let mana = game.resources.get_mut("demo.resource.mana").unwrap();
    mana.current = mana.maximum;
    game
}

fn cast(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        SCARE,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

#[test]
fn spectre_six_classes_complete_absorb_level_scare_wall_and_save_sequence() {
    for build in [
        "demo.build.warrior",
        "demo.build.high-mage-death",
        "demo.build.archer",
        "demo.build.paladin-death",
        "demo.build.cavalry",
        "demo.build.sniper",
    ] {
        // Choose a normal seeded run with a successful fear result, without bypassing checks.
        let mut game = (0..100)
            .find_map(|seed| {
                let mut game = birth(seed, build);
                clear_monsters(&mut game);
                game.player.position = START;
                for y in 14..=18 {
                    for x in 46..=52 {
                        replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
                    }
                }
                let staff = game
                    .items
                    .iter()
                    .find(|item| item.kind_id == "demo.item.staff-of-nothing")
                    .unwrap()
                    .id
                    .clone();
                let nutrition = game.nutrition;
                dispatch_next(
                    &mut game,
                    GameCommand::AbsorbDevice {
                        item_id: staff.clone(),
                    },
                );
                assert_eq!(game.nutrition, nutrition + 5000);
                assert_eq!(
                    game.items
                        .iter()
                        .find(|item| item.id == staff)
                        .unwrap()
                        .charges
                        .unwrap()
                        .current,
                    20
                );
                // XP and a nearby sheep are explicit core-test preconditions, not desktop play.
                game.apply_unscaled_player_experience(
                    crate::stats::experience_required_for_level(4),
                    &mut Vec::new(),
                );
                assert_eq!(game.progress.level, 4);
                game.player.hp = game.effective_player_max_hp();
                game.push_generated_actor(
                    "test.spectre.target".to_owned(),
                    "demo.actor.sheep",
                    EAST,
                );
                let cast = dispatch_next(
                    &mut game,
                    GameCommand::CastAbility {
                        ability_id: SCARE.to_owned(),
                        target: TargetSelection::Direction {
                            direction: Direction::East,
                        },
                    },
                );
                let afraid = cast.events.iter().any(|event| {
                    let Some(GameEventOutcomeDto::AbilityEffects { resolution }) = &event.outcome
                    else {
                        return false;
                    };
                    resolution.effects.iter().any(|effect| {
                        matches!(effect,
                            rfb_protocol::AbilityEffectResolutionDto::ApplyStatus {
                                status_kind_id, applied_duration_ticks, ..
                            } if status_kind_id == STATUS_FEAR && *applied_duration_ticks > 0
                        )
                    })
                });
                afraid.then_some(game)
            })
            .unwrap_or_else(|| panic!("{build}: successful fear must be reachable"));
        clear_monsters(&mut game);
        replace_terrain(&mut game, EAST, "demo.terrain.wall");
        let before_tick = game.world_tick;
        let before_hp = game.player.hp;
        let entry = dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        assert_eq!(game.player.position, EAST);
        assert_eq!(game.world_tick - before_tick, 15);
        assert!(game.player.hp < before_hp);
        assert!(
            entry
                .events
                .iter()
                .any(|event| event.message_key == "player-wall-density")
        );
        let before_hp = game.player.hp;
        dispatch_next(&mut game, GameCommand::Wait);
        assert_eq!(game.player.hp, before_hp - 1);
        let leave = dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::West,
            },
        );
        assert_eq!(game.player.position, START);
        assert!(
            !leave
                .events
                .iter()
                .any(|event| event.message_key == "player-wall-density")
        );
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.snapshot(), game.snapshot());
        assert_eq!(
            dispatch_next(&mut restored, GameCommand::Wait),
            dispatch_next(&mut game, GameCommand::Wait)
        );
        assert_eq!(restored.state_hash(), game.state_hash());
    }
}

#[test]
fn spectre_scare_unlocks_at_four_uses_intelligence_and_keeps_charisma_effect_power() {
    let mut game = ready();
    let activation = game.race_ability_activation(SCARE).unwrap();
    assert_eq!(activation.base_failure_percent, 50);
    game.progress.level = 3;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|a| a.id == SCARE)
        .unwrap();
    assert_eq!(locked.minimum_level, 4);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (6, 6));
    assert!(!locked.can_cast);
    let mana = game.resources["demo.resource.mana"].current;
    assert!(
        cast(&mut game)
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastUnavailable { .. }))
    );
    assert_eq!(game.resources["demo.resource.mana"].current, mana);
    game.progress.level = 4;
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|a| a.id == SCARE)
        .unwrap();
    assert!(available.can_cast);
    let expected_power = (9 + crate::stats::original_save_adjustment(
        game.effective_player_attributes()
            .index(AttributeKind::Charisma),
    ))
    .max(1) as u16;
    assert!(
        matches!(available.effects.as_slice(), [AbilityEffectSpecDto::ApplyStatus {
        power: Some(power), duration_sides: 2, .. }] if *power == expected_power)
    );
    game.progress.attributes.intelligence = 118;
    let intelligent = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|a| a.id == SCARE)
        .unwrap();
    assert!(intelligent.failure_percent < available.failure_percent);
    assert_eq!(intelligent.effects, available.effects);
}

#[test]
fn spectre_scare_success_failure_and_sp_hp_payment_share_the_real_cast_pipeline() {
    let mut template = ready();
    template.push_generated_actor("test.spectre.target".to_owned(), "demo.actor.sheep", EAST);
    for success in [false, true] {
        let mut result = (0..1000)
            .find_map(|seed| {
                let mut game = template.clone();
                game.rng = RfbRng::seeded(seed);
                let before = game.resources["demo.resource.mana"].current;
                let events = cast(&mut game);
                let failed = events
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityCastFailed { .. }));
                let afraid = game.entities[0]
                    .statuses
                    .iter()
                    .any(|s| s.kind_id == STATUS_FEAR);
                ((success && afraid) || (!success && failed))
                    .then_some((game, events, before, seed))
            })
            .expect("both outcomes must be reachable without forcing cast success");
        assert_eq!(
            result.0.resources["demo.resource.mana"].current,
            result.2 - 6
        );
        assert_eq!(
            result.0.entities[0]
                .statuses
                .iter()
                .any(|s| s.kind_id == STATUS_FEAR),
            success
        );
        // The same cast pays missing SP with HP, including on failure.
        let mut hp_cast = template.clone();
        hp_cast
            .resources
            .get_mut("demo.resource.mana")
            .unwrap()
            .current = 2;
        hp_cast.player.hp = 10;
        hp_cast.rng = RfbRng::seeded(result.3);
        let events = cast(&mut hp_cast);
        assert_eq!(hp_cast.resources["demo.resource.mana"].current, 0);
        assert_eq!(hp_cast.player.hp, 6);
        assert_eq!(
            events
                .iter()
                .any(|e| matches!(e, DomainEvent::AbilityCastFailed { .. })),
            !success
        );
        result.0.entities.clear();
        let mut restored =
            Game::from_save_with_content(result.0.to_save(), result.0.content.clone()).unwrap();
        assert_eq!(cast(&mut restored), cast(&mut result.0));
        assert_eq!(restored.state_hash(), result.0.state_hash());
    }
    template
        .resources
        .get_mut("demo.resource.mana")
        .unwrap()
        .current = 0;
    template.player.hp = 5;
    assert!(
        cast(&mut template)
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastUnavailable { .. }))
    );
    assert_eq!(template.player.hp, 5);
}

#[test]
fn spectre_scare_respects_immunity_walls_and_current_form() {
    for immune in [false, true] {
        let mut game = ready();
        game.debug_set_ability_casts_succeed(true);
        game.push_generated_actor(
            "test.spectre.target".to_owned(),
            if immune {
                "demo.actor.metal-babble"
            } else {
                "demo.actor.sheep"
            },
            Position { x: 50, y: 16 },
        );
        if !immune {
            replace_terrain(&mut game, EAST, "demo.terrain.wall");
        }
        let events = cast(&mut game);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
        );
        assert!(
            game.entities[0]
                .statuses
                .iter()
                .all(|s| s.kind_id != STATUS_FEAR)
        );
        if immune {
            assert!(events.iter().any(|e| matches!(e,
            DomainEvent::AbilityEffectsResolved { resolution, .. } if resolution.effects.iter().any(|effect|
                matches!(effect, AbilityEffectResolutionDto::ApplyStatus { change: AbilityStatusChangeDto::Immune, .. })) )));
        }
    }
    let mut game = ready();
    form(&mut game, HUMAN);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|a| a.id != SCARE)
    );
    game.player.statuses.clear();
    game.build.as_mut().unwrap().race_id = HUMAN.to_owned();
    form(&mut game, SPECTRE);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .any(|a| a.id == SCARE && a.can_cast)
    );
    game.debug_set_ability_casts_succeed(true);
    assert!(
        cast(&mut game)
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
    );
    game.player.statuses.clear();
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|a| a.id != SCARE)
    );
}

#[test]
fn spectre_birth_virtue_is_unlife_and_kin_scroll_summons_current_glyph_without_rerolling_virtues() {
    for temporary in [false, true] {
        let mut game = birth(83, "demo.build.warrior");
        assert_eq!(game.virtues[2].kind, VirtueKindDto::Unlife);
        if temporary {
            game = Game::new_with_build(83, "demo.build.warrior").unwrap();
        }
        let virtues = game.virtues;
        if temporary {
            form(&mut game, SPECTRE);
        }
        assert_eq!(game.virtues, virtues);
        clear_monsters(&mut game);
        game.items.clear();
        game.gold_piles.clear();
        game.progress.level = 50;
        game.player.position = START;
        for y in 14..=18 {
            for x in 46..=50 {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
            }
        }
        give_inventory_item(
            &mut game,
            "test.spectre.kin",
            "demo.item.kin-summoning-scroll",
        );
        game.use_inventory_item(
            "test.spectre.kin",
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(!game.entities.is_empty());
        assert!(game.entities.iter().all(|actor| {
            game.content
                .actor(&actor.kind_id)
                .unwrap()
                .tags
                .iter()
                .any(|tag| tag == "kin-glyph-71")
                && game.actor_is_player_side(actor)
        }));
        assert!(game.items.iter().all(|item| item.id != "test.spectre.kin"));
    }
}

#[test]
fn spectre_nonliving_form_blocks_unlife_and_vampiric_healing_but_keeps_damage() {
    for vampiric in [false, true] {
        let effect = if vampiric {
            MeleeBlowEffectDefinition::Damage {
                chance_percent: None,
                damage_dice: 1,
                damage_sides: 4,
                damage_type: rfb_content::ActorDamageType::Physical,
                armor_mitigated: false,
                vampiric: true,
            }
        } else {
            MeleeBlowEffectDefinition::Unlife {
                chance_percent: None,
                amount_dice: 100,
                amount_sides: 1,
            }
        };
        let mut template = super::combat::monster_effect_game(0, effect);
        template.entities[0].hp = 1;
        template.entities[0].max_hp = 20;
        for spectre in [false, true] {
            let mut game = template.clone();
            if spectre {
                form(&mut game, SPECTRE);
            }
            let hp = game.player.hp;
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            if vampiric {
                assert!(game.player.hp < hp);
                assert_eq!(game.entities[0].hp == 1, spectre);
            } else {
                assert_eq!(game.progress.life_force, if spectre { 1000 } else { 900 });
                assert_eq!(
                    game.entities[0].power_per_mille,
                    if spectre { 1000 } else { 1100 }
                );
            }
        }
    }
}

#[test]
fn spectre_nonliving_form_ignores_salt_water_and_no_air() {
    for spectre in [false, true] {
        let mut game = Game::new_with_build(83, "demo.build.warrior").unwrap();
        clear_monsters(&mut game);
        if spectre {
            form(&mut game, SPECTRE);
        }
        game.nutrition = 9000;
        give_inventory_item(&mut game, "test.spectre.salt", "demo.item.salt-water");
        game.use_inventory_item(
            "test.spectre.salt",
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.nutrition, if spectre { 9000 } else { 99 });
        assert_eq!(game.player_has_status_kind(STATUS_PARALYSIS), !spectre);
        game.player.position = START;
        game.push_generated_actor(
            "test.spectre.vayu".to_owned(),
            "demo.actor.vayu-the-embodied-wind",
            EAST,
        );
        let ability = game
            .content
            .ability("rfb-legacy.ability.no-air-40")
            .unwrap()
            .clone();
        game.resolve_monster_player_effects(
            "test.spectre.vayu",
            "demo.actor.vayu-the-embodied-wind",
            &ability,
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );
        assert_eq!(game.player_has_status_kind(STATUS_NO_AIR), !spectre);
    }
}

#[test]
fn spectre_eldritch_extra_save_uses_current_undead_form_and_original_level_chance() {
    let mut game = ready();
    game.build.as_mut().unwrap().race_id = HUMAN.to_owned();
    form(&mut game, SPECTRE);
    game.push_generated_actor("test.spectre.ghast".to_owned(), "demo.actor.ghast", EAST);
    let power = 9;
    let normal_save =
        (game.player_derived_stats().saving_throw_skill.value - power).clamp(0, 100) as u64;
    for succeeds in [false, true] {
        let seed = (0..10000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(100) < power as u64
                    && rng.bounded(100) >= normal_save
                    && (rng.bounded(100) < 29) == succeeds
            })
            .unwrap();
        let mut candidate = game.clone();
        candidate.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        candidate.resolve_eldritch_horror(0, &mut events, &mut BTreeSet::new());
        assert_eq!(
            events.iter().any(|e| matches!(
                e,
                DomainEvent::EldritchHorror {
                    outcome: "unaffected",
                    ..
                }
            )),
            succeeds
        );
        assert_eq!(candidate.entities[0].eldritch_horror_triggered, !succeeds);
    }
}

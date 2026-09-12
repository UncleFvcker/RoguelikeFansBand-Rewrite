// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::dispatch_next;
use rfb_protocol::{AbilityDto, ResourceConversionResolutionDto};

const HP_TO_MP: &str = "demo.ability.warrior-mage-hp-to-sp";
const MP_TO_HP: &str = "demo.ability.warrior-mage-sp-to-hp";

fn power(game: &Game, id: &str) -> AbilityDto {
    game.snapshot()
        .player
        .abilities
        .into_iter()
        .find(|a| a.id == id)
        .unwrap()
}

fn cast(game: &mut Game, id: &str) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        id,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn conversion(events: &[DomainEvent]) -> &ResourceConversionResolutionDto {
    events
        .iter()
        .find_map(|e| match e {
            DomainEvent::AbilityResourceConverted { resolution, .. } => Some(resolution),
            _ => None,
        })
        .unwrap()
}

fn seed_cast(game: &mut Game, id: &str, succeeds: bool) {
    let failure = power(game, id).failure_percent;
    assert!((1..=95).contains(&failure));
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            (rng.bounded(100) >= u64::from(failure)) == succeeds
        })
        .unwrap();
}

#[test]
fn powers_unlock_at_twenty_five_use_intelligence_and_project_internal_costs() {
    let mut game = at_level(BUILD, 24);
    for id in [HP_TO_MP, MP_TO_HP] {
        assert_eq!(
            power(&game, id).unavailable_reason.as_deref(),
            Some("level-too-low")
        );
        let before = game.to_save();
        assert!(
            matches!(&cast(&mut game, id)[0], DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "level-too-low")
        );
        assert_eq!(game.to_save(), before);
    }
    for level in [25, 49, 50] {
        let mut game = at_level(BUILD, level);
        for id in [HP_TO_MP, MP_TO_HP] {
            let p = power(&game, id);
            assert_eq!(
                (p.source, p.resource_cost, p.can_cast),
                (AbilitySourceDto::Class, 0, true)
            );
            assert!(!game.ability_progress.contains_key(id));
        }
        assert!(
            matches!(power(&game, HP_TO_MP).effects.as_slice(), [AbilityEffectSpecDto::HealthToMana { hit_point_cost, mana_divisor: 5 }] if *hit_point_cost == u32::from(level))
        );
        assert!(
            matches!(power(&game, MP_TO_HP).effects.as_slice(), [AbilityEffectSpecDto::ManaToHealth { mana_cost, healing }] if *mana_cost == u32::from(level / 5) && *healing == u32::from(level))
        );
        let failure = power(&game, HP_TO_MP).failure_percent;
        game.progress.attributes.wisdom = 3;
        assert_eq!(power(&game, HP_TO_MP).failure_percent, failure);
        game.progress.attributes.intelligence = 3;
        assert!(power(&game, HP_TO_MP).failure_percent >= failure);
    }
}

#[test]
fn natural_failure_pays_no_internal_resources_and_success_survives_save_and_next_rng() {
    for id in [HP_TO_MP, MP_TO_HP] {
        for succeeds in [true, false] {
            let mut game = at_level(BUILD, 25);
            game.player.hp -= 50;
            game.resources.get_mut(MANA).unwrap().current = 10;
            seed_cast(&mut game, id, succeeds);
            let before = (game.player.hp, game.resources[MANA].current);
            let mut restored = Game::from_save(game.to_save()).unwrap();
            let mut expected_rng = game.rng.clone();
            expected_rng.bounded(100);
            let events = cast(&mut game, id);
            assert_eq!(events, cast(&mut restored, id));
            assert_eq!(game.rng, expected_rng);
            assert_eq!(game.state_hash(), restored.state_hash());
            let activation = events
                .iter()
                .find_map(|e| match e {
                    DomainEvent::AbilityCastSucceeded { resolution }
                    | DomainEvent::AbilityCastFailed { resolution } => Some(resolution),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                (
                    activation.succeeded,
                    activation.resource_cost,
                    activation.resource_paid,
                    activation.hp_paid
                ),
                (succeeds, 0, 0, 0)
            );
            let delta = if !succeeds {
                (0, 0)
            } else if id == HP_TO_MP {
                (-25, 5)
            } else {
                // The birth helper chooses Sacred Vitality: 25 base healing becomes 30.
                (30, -5)
            };
            assert_eq!(
                (
                    game.player.hp - before.0,
                    game.resources[MANA].current as i32 - before.1 as i32
                ),
                delta
            );
            assert_eq!(
                Game::from_save(game.to_save()).unwrap().state_hash(),
                game.state_hash()
            );
            assert_eq!(
                dispatch_next(&mut game, GameCommand::Wait).events,
                dispatch_next(&mut restored, GameCommand::Wait).events
            );
            assert_eq!(game.to_save(), restored.to_save());
        }
    }
}

#[test]
fn life_payment_ignores_full_mana_caps_gain_and_can_kill_below_zero() {
    for (hp, near_cap) in [
        (100, false),
        (100, true),
        (25, false),
        (24, false),
        (0, false),
    ] {
        let mut game = at_level(BUILD, 25);
        game.debug_set_ability_casts_succeed(true);
        game.player.hp = hp;
        let maximum = game.resources[MANA].maximum;
        let before = if near_cap { maximum - 2 } else { maximum };
        game.resources.get_mut(MANA).unwrap().current = before;
        let events = cast(&mut game, HP_TO_MP);
        let r = conversion(&events);
        assert_eq!(
            (r.hp_before, r.hp_after, r.resource_before, r.resource_after),
            (hp, hp - 25, before, maximum)
        );
        assert!(r.converted);
        assert_eq!(r.fatal, hp < 25);
        assert_eq!(game.player_is_dead(), hp < 25);
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn mana_payment_is_exact_even_at_full_health_and_shortfall_still_spends_turn() {
    for level in [25, 49, 50] {
        for full in [false, true] {
            let mut game = at_level(BUILD, level);
            game.debug_set_ability_casts_succeed(true);
            game.player.hp -= if full { 0 } else { 3 };
            game.resources.get_mut(MANA).unwrap().current = u32::from(level / 5);
            let r = conversion(&cast(&mut game, MP_TO_HP)).clone();
            assert!(r.converted);
            assert_eq!(r.resource_after, 0);
            assert_eq!(r.hp_after - r.hp_before, if full { 0 } else { 3 });
        }
    }
    for (id, succeeds, mana) in [
        (MP_TO_HP, true, 4),
        (MP_TO_HP, false, 10),
        (HP_TO_MP, false, 0),
    ] {
        let mut game = at_level(BUILD, 25);
        game.resources.get_mut(MANA).unwrap().current = mana;
        seed_cast(&mut game, id, succeeds);
        let tick = game.world_tick;
        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        assert!(game.world_tick > tick);
        if succeeds {
            assert!(update.events.iter().any(|e| matches!(&e.outcome, Some(GameEventOutcomeDto::ResourceConversion { resolution }) if !resolution.converted && resolution.resource_before == 4 && resolution.resource_after == 4 && resolution.hp_before == resolution.hp_after)));
        } else {
            assert!(
                update
                    .events
                    .iter()
                    .any(|e| e.kind == "ability.cast-failure")
            );
            assert!(
                !update
                    .events
                    .iter()
                    .any(|e| e.kind == "ability.resource-converted")
            );
        }
    }
}

#[test]
fn transcendence_absorbs_before_conversion_but_ordinary_invulnerability_does_not() {
    for mana in [0, 6, 23, 25] {
        let mut game = at_level(BUILD, 25);
        game.debug_set_ability_casts_succeed(true);
        game.resources.get_mut(MANA).unwrap().current = mana;
        for kind in [crate::effect::STATUS_TRANSCENDENCE, STATUS_INVULNERABILITY] {
            let mut status =
                super::super::monster_combat::melee_status(kind, 100, "test.conversion").status;
            if kind == STATUS_INVULNERABILITY {
                status.incoming_damage_percent = 0;
            }
            game.player.statuses.push(status);
        }
        let hp = game.player.hp;
        let r = conversion(&cast(&mut game, HP_TO_MP)).clone();
        assert_eq!(r.hp_after, hp - (25 - mana) as i32);
        assert_eq!(r.resource_after, (25 - mana) / 5);
        assert_eq!(r.converted, mana <= 20);
    }
}

#[test]
fn internal_conversion_ignores_spell_power_and_reduced_mana_but_uses_common_healing() {
    let mut game = at_level(BUILD, 25);
    game.debug_set_ability_casts_succeed(true);
    let item = game
        .items
        .iter_mut()
        .find(|i| matches!(i.location, ItemLocation::Equipped { .. }))
        .unwrap();
    item.intrinsic_properties
        .passives
        .insert(EquipmentPassive::ReducedManaCost);
    let mut status =
        super::super::monster_combat::melee_status("test.spell-power", 100, "test").status;
    status.granted_modifiers.spell_power_bonus = 3;
    game.player.statuses.push(status);
    game.refresh_player_ability_state();
    game.player.hp -= 100;
    game.resources.get_mut(MANA).unwrap().current = 20;
    let r = conversion(&cast(&mut game, HP_TO_MP)).clone();
    assert_eq!(
        (
            r.hp_before - r.hp_after,
            r.resource_after - r.resource_before
        ),
        (25, 5)
    );
    let mutation = game
        .content
        .mutations()
        .find(|m| m.healing_bonus_percent > 0)
        .unwrap()
        .clone();
    game.progress.active_mutation_ids.insert(mutation.id);
    let r = conversion(&cast(&mut game, MP_TO_HP)).clone();
    assert_eq!(r.resource_before - r.resource_after, 5);
    assert_eq!(
        r.hp_after - r.hp_before,
        25 + 25 * i32::from(mutation.healing_bonus_percent) / 100
    );
}

#[test]
fn powers_ignore_book_anti_magic_and_berserk_but_keep_confusion_and_stun_rules() {
    let mut game = at_level(BUILD, 25);
    game.items
        .iter_mut()
        .find(|i| matches!(i.location, ItemLocation::Equipped { .. }))
        .unwrap()
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::AntiMagic);
    game.apply_player_mental_status(crate::effect::STATUS_ANTI_MAGIC, 100, "test");
    game.apply_player_mental_status(STATUS_BERSERK, 100, "test");
    game.current_floor_id = "demo.floor.anti-magic-cave-depth-40".to_owned();
    assert!(game.dungeon_blocks_magic());
    for id in [HP_TO_MP, MP_TO_HP] {
        assert!(power(&game, id).can_cast);
    }
    game.apply_player_mental_status(STATUS_CONFUSION, 100, "test");
    let before = game.to_save();
    for id in [HP_TO_MP, MP_TO_HP] {
        assert!(!power(&game, id).can_cast);
        cast(&mut game, id);
        assert_eq!(game.to_save(), before);
    }
    let mut game = at_level(BUILD, 25);
    let base = power(&game, HP_TO_MP).failure_percent;
    game.items
        .iter_mut()
        .find(|i| matches!(i.location, ItemLocation::Equipped { .. }))
        .unwrap()
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::EasySpell);
    assert_eq!(power(&game, HP_TO_MP).failure_percent, base - 5);
    let mut stun = super::super::monster_combat::melee_status(STATUS_STUN, 100, "test").status;
    stun.intensity = 40;
    game.player.statuses.push(stun);
    assert_eq!(power(&game, HP_TO_MP).failure_percent, base - 5 + 20);
}

#[test]
fn einheri_conversion_uses_reduced_healing_without_reducing_mana_payment() {
    let mut game =
        Game::new_with_build_race_and_name(925, BUILD, "rfb-legacy.race.einheri", "Conversion")
            .unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(25), &mut Vec::new());
    game.debug_set_ability_casts_succeed(true);
    game.player.hp = game.effective_player_max_hp() - 100;
    game.resources.get_mut(MANA).unwrap().current = 5;
    let r = conversion(&cast(&mut game, MP_TO_HP)).clone();
    assert_eq!(
        (
            r.resource_before - r.resource_after,
            r.hp_after - r.hp_before
        ),
        (5, 12)
    );
}

#[test]
fn melee_conversion_book_cast_and_healing_continue_identically_after_loading() {
    let mut game = at_level(BUILD, 25);
    let spell = "demo.ability.arcane-detect-monsters";
    let book = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.cantrips-for-beginners")
        .unwrap()
        .id
        .clone();
    game.study_player_ability(&book, spell).unwrap();
    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 10, y: 10 };
    for x in 9..=12 {
        super::super::support::replace_terrain(
            &mut game,
            Position { x, y: 10 },
            "demo.terrain.floor",
        );
    }
    game.push_generated_actor(
        "test.opponent".to_owned(),
        "demo.actor.sheep",
        Position { x: 11, y: 10 },
    );
    game.entities[0].hp = 1;
    let melee = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(
        melee
            .events
            .iter()
            .any(|event| event.kind.starts_with("combat."))
    );
    game.resources.get_mut(MANA).unwrap().current = 0;
    let conversion = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: HP_TO_MP.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(conversion.events.iter().any(|e| matches!(&e.outcome, Some(GameEventOutcomeDto::ResourceConversion { resolution }) if resolution.hp_before - resolution.hp_after == 25 && resolution.resource_after == 5)));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for id in [spell, HP_TO_MP, MP_TO_HP] {
        let command = GameCommand::CastAbility {
            ability_id: id.to_owned(),
            target: TargetSelection::SelfTarget,
        };
        let update = dispatch_next(&mut game, command.clone());
        assert_eq!(update.events, dispatch_next(&mut restored, command).events);
        assert!(
            update
                .events
                .iter()
                .any(|e| e.kind == "ability.cast-success")
        );
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
    }
}

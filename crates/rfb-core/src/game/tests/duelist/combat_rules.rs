// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn automatic_weapon_challenge_spends_the_attack_and_retaliation_preserves_another_opponent() {
    let mut game = at_level(1);
    target(&mut game, "test.a", 1);
    target(&mut game, "test.b", -1);
    let hp = game.entities[0].hp;
    let mut events = Vec::new();
    game.resolve_player_melee(0, true, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.a"));
    assert_eq!(game.entities[0].hp, hp);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::DuelistChallengeIssued { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerMeleeHit { .. }))
    );
    events.clear();
    game.resolve_player_revenge_blow(1, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.a"));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::DuelistChallengeIssued { .. }))
    );
    game.duelist_target_id = None;
    assert!(!game.duelist_auto_challenge(0, false, false, &mut events));
    game.entities[0].max_hp = 101;
    assert!(game.duelist_auto_challenge(0, false, false, &mut events));
}

#[test]
fn careful_aim_does_not_draw_saves_before_level_fifteen() {
    for (level, expected) in [(9, 17), (10, 34), (14, 34)] {
        let mut game = at_level(level);
        target(&mut game, "test.a", 1);
        let rng = game.rng.clone();
        assert_eq!(game.duelist_strike_damage(0, 17), (expected, None));
        assert_eq!(game.rng, rng);
        assert!(game.entities[0].statuses.is_empty());
    }
}

#[test]
fn special_strikes_keep_initial_damage_current_hp_and_source_save_order() {
    let mut game = at_level(40);
    target(&mut game, "test.a", 1);
    game.entities[0].hp = 101;
    game.entities[0].max_hp = 10_000;
    game.entities[0]
        .statuses
        .push(monster_combat::melee_status(STATUS_STUN, 40, "test").status);
    let mut expected_rng = game.rng.clone();
    let power = (40
        + crate::stats::original_save_adjustment(
            game.effective_player_attributes()
                .index(AttributeKind::Dexterity),
        ))
    .max(1) as u64;
    let level = u64::from(game.content.actor("demo.actor.sheep").unwrap().level.max(1));
    let slow_saved = expected_rng.bounded(power) <= expected_rng.bounded(level);
    let stun_saved = expected_rng.bounded(power) <= expected_rng.bounded(level);
    let wound_saved = expected_rng.bounded(power) <= expected_rng.bounded(level);
    let mut damage = 20;
    let mut drain = None;
    if !wound_saved {
        damage += 20.min((expected_rng.bounded(3) as i32 + 1) * 10);
        drain = Some(damage);
    }
    let greater_saved = expected_rng.bounded(power) <= expected_rng.bounded(level);
    if !greater_saved {
        damage += 40.min((expected_rng.bounded(5) as i32 + 2) * 10);
        drain = Some(damage);
    }
    assert_eq!(game.duelist_strike_damage(0, 10), (damage, drain));
    assert_eq!(game.rng, expected_rng);
    assert_eq!(
        game.entities[0].hp, 101,
        "both wounds read HP before damage is committed"
    );
    assert_eq!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLOW),
        !slow_saved
    );
    assert_eq!(
        game.entities[0]
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_STUN)
            .unwrap()
            .remaining_ticks,
        if stun_saved { 40 } else { 43 }
    );
}

#[test]
fn defenses_use_instance_identity_and_fear_aura_does_not_get_the_spell_save_bonus() {
    let mut game = at_level(30);
    target(&mut game, "test.a", 1);
    target(&mut game, "test.b", -1);
    game.duelist_target_id = Some("test.a".to_owned());
    assert_eq!(game.duelist_reduce_damage("test.a", 5), 4);
    assert_eq!(game.duelist_reduce_damage("test.b", 5), 5);
    let mut other = game.clone();
    let mut a = Vec::new();
    let mut b = Vec::new();
    game.monster_saving_throw(Some("test.a"), "demo.actor.sheep", 100, &mut a);
    other.monster_saving_throw(Some("test.b"), "demo.actor.sheep", 100, &mut b);
    assert_ne!(a, b);
    assert_eq!(game.rng, other.rng);
    a.clear();
    b.clear();
    game.monster_fear_saving_throw(Some("test.a"), "demo.actor.sheep", 100, &mut a);
    other.monster_fear_saving_throw(Some("test.b"), "demo.actor.sheep", 100, &mut b);
    assert_eq!(a, b);
    assert_eq!(game.rng, other.rng);
    let hp = game.player.hp;
    game.resolve_monster_damage_to_player(
        "test.a",
        "demo.actor.sheep",
        "rfb-legacy.ability.bolt-physical-1d1-137",
        0,
        9,
        9,
        DamageType::Physical,
        &mut a,
    );
    other.resolve_monster_damage_to_player(
        "test.b",
        "demo.actor.sheep",
        "rfb-legacy.ability.bolt-physical-1d1-137",
        0,
        9,
        9,
        DamageType::Physical,
        &mut b,
    );
    assert_eq!(game.player.hp, hp - 6);
    assert_eq!(other.player.hp, hp - 9);
}

#[test]
fn natural_attack_can_spend_its_action_issuing_a_challenge_without_hitting() {
    let mut game = at_level(20);
    game.items.retain(|item| item.kind_id != "demo.item.rapier");
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.horns".to_owned());
    target(&mut game, "test.natural", 1);
    game.entities[0].hp = 2000;
    game.entities[0].max_hp = 2000;
    let mut events = Vec::new();
    game.resolve_player_melee(0, true, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert_eq!(game.entities[0].hp, 2000);
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.natural"));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::DuelistChallengeIssued { .. }))
    );
}

#[test]
fn fifth_level_fear_bonus_only_helps_attacks_on_the_challenged_instance() {
    let mut base = at_level(5);
    target(&mut base, "test.a", 1);
    target(&mut base, "test.b", -1);
    base.duelist_target_id = Some("test.a".to_owned());
    base.player
        .statuses
        .push(monster_combat::melee_status(STATUS_FEAR, 100, "test").status);
    let mut improved = false;
    for seed in 0..100 {
        let mut a = base.clone();
        a.rng = crate::rng::RfbRng::seeded(seed);
        let mut b = a.clone();
        let challenged_blocked = a.player_fear_blocks_melee(0);
        let other_blocked = b.player_fear_blocks_melee(1);
        assert_eq!(a.rng, b.rng);
        assert!(!challenged_blocked || other_blocked);
        improved |= !challenged_blocked && other_blocked;
        if improved {
            break;
        }
    }
    assert!(improved);
    assert!(base.player_has_status_kind(STATUS_FEAR));
}

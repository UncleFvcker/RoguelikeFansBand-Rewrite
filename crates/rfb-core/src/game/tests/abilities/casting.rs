// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::effect::STATUS_ANTI_MAGIC;

#[test]
fn spell_blocking_statuses_reject_without_spending_resources() {
    for (status_kind, expected_reason, equipment) in [
        (STATUS_ANTI_MAGIC, "anti-magic", false),
        (STATUS_BERSERK, "berserk", false),
        (STATUS_ANTI_MAGIC, "anti-magic", true),
    ] {
        let mut game = prepare_death_caster(7, 40, "demo.ability.death-berserk");
        if equipment {
            game.items
                .iter_mut()
                .find(|item| matches!(&item.location, ItemLocation::Equipped { .. }))
                .unwrap()
                .intrinsic_properties
                .passives
                .insert(rfb_content::EquipmentPassive::AntiMagic);
        } else {
            game.player
                .statuses
                .push(monster_combat::melee_status(status_kind, 5, "test.spell-blocker").status);
        }
        let mana_before = game.resources["demo.resource.mana"].current;
        let draws_before = game.rng_draw_counter();
        let mut events = Vec::new();

        game.resolve_player_ability(
            "demo.ability.death-berserk",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("spell rejection should resolve cleanly");

        assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == expected_reason
        ));
        assert!(
            !game
                .snapshot()
                .player
                .abilities
                .iter()
                .find(|ability| ability.id == "demo.ability.death-berserk")
                .expect("learned spell should remain projected")
                .can_cast
        );
    }
}

#[test]
fn race_ability_follows_the_effective_race_and_projects_its_source() {
    let mut game = Game::new_with_build_race_and_name(
        0,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human warrior should create");
    game.progress.level = 7;
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_BERSERK_ABILITY_ID)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.race-form").status;
    form.granted_race_id = Some("rfb-legacy.race.barbarian".to_owned());
    game.player.statuses.push(form);

    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
        .expect("the temporary Barbarian form should project its ability");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength)
    );
    assert_eq!(locked.minimum_level, 8);
    assert!(!locked.can_cast);

    game.progress.level = 8;
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
            .expect("race ability should remain projected")
            .can_cast
    );

    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 10, "test.confusion").status);
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
            .expect("race ability should remain projected while confused")
            .can_cast
    );
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_CONFUSION);

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_BERSERK_ABILITY_ID)
    );
}

#[test]
fn racial_cast_failures_pay_without_revealing_or_creating_items() {
    for (race_id, seed, level, ability_id, hp_cost, target) in [
        (
            "rfb-legacy.race.kobold",
            92,
            12,
            RACE_POISON_DART_ABILITY_ID,
            5,
            TargetSelection::Direction {
                direction: Direction::East,
            },
        ),
        (
            "rfb-legacy.race.dwarf",
            95,
            10,
            RACE_DETECT_TREASURE_ABILITY_ID,
            2,
            TargetSelection::SelfTarget,
        ),
        (
            "rfb-legacy.race.hobbit",
            88,
            15,
            RACE_CREATE_FOOD_ABILITY_ID,
            7,
            TargetSelection::SelfTarget,
        ),
    ] {
        let mut game = Game::new_with_build_race_and_name(
            seed,
            "demo.build.high-mage-death",
            race_id,
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("racial High-Mage should create");
        clear_monsters(&mut game);
        game.apply_unscaled_player_experience(
            crate::stats::experience_required_for_level(level),
            &mut Vec::new(),
        );
        game.resources
            .get_mut("demo.resource.mana")
            .expect("High-Mage should have mana")
            .current = 3;
        let blocker = Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        };
        let treasure = Position {
            x: game.player.position.x + 2,
            y: game.player.position.y,
        };
        replace_terrain(&mut game, blocker, "demo.terrain.wall");
        replace_terrain(&mut game, treasure, "demo.terrain.quartz-hidden-treasure");
        let treasure_index = game.index(treasure).expect("treasure cell");
        game.explored[treasure_index] = false;
        game.revealed_terrain.remove(&treasure);
        game.gold_piles = vec![GoldPile {
            id: "generated.gold.1".to_owned(),
            position: treasure,
            amount: 25,
            appearance: GoldAppearanceDto::Gold,
            discovered: false,
        }];
        game.next_gold_pile_serial = 2;
        let failure_percent = game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
            .expect("racial ability")
            .failure_percent;
        let failure_seed = (0..4_096)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(failure_percent))
            .expect("racial ability should have a failing seed");
        game.rng = RfbRng::seeded(failure_seed);
        let acquired = |game: &Game| {
            game.items
                .iter()
                .filter(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
                .count()
        };
        let acquired_before = acquired(&game);
        let serial_before = game.next_item_instance_serial;
        let hp_before = game.player.hp;
        let tick_before = game.world_tick;
        dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target,
            },
        );
        assert_eq!(
            (
                game.world_tick,
                game.resources["demo.resource.mana"].current,
                game.player.hp
            ),
            (tick_before + 10, 0, hp_before - hp_cost),
            "{race_id}"
        );
        assert_eq!(
            (game.next_item_instance_serial, acquired(&game)),
            (serial_before, acquired_before),
            "{race_id}"
        );
        assert!(
            !game.explored[treasure_index]
                && !game.revealed_terrain.contains(&treasure)
                && !game.gold_piles[0].discovered,
            "{race_id}"
        );
    }
}

#[test]
fn kobold_intrinsics_follow_the_effective_race() {
    let mut game = Game::new_with_build_race_and_name(
        93,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    game.progress.level = 12;
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Normal
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_POISON_DART_ABILITY_ID)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.kobold-form").status;
    form.granted_race_id = Some("rfb-legacy.race.kobold".to_owned());
    game.player.statuses.push(form);
    assert_eq!(game.player_infravision_range(), 3);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_POISON_DART_ABILITY_ID)
        .expect("temporary Kobold form should grant Poison Dart");
    assert_eq!(ability.source, AbilitySourceDto::Race);
    assert_eq!(
        ability.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Dexterity)
    );

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Normal
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_POISON_DART_ABILITY_ID)
    );
}

#[test]
fn dwarf_intrinsics_follow_the_effective_race_without_replacing_birth_rewards() {
    let mut game = Game::new_with_build_race_and_name(
        96,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    game.progress.level = 20;
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Normal
    );
    let pending_before = game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("birth Human should retain its level twenty reward");
    assert_eq!(pending_before.reward_id, "human-talent");

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.dwarf-form").status;
    form.granted_race_id = Some("rfb-legacy.race.dwarf".to_owned());
    game.player.statuses.push(form);
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Resistant
    );
    let snapshot = game.snapshot();
    for (ability_id, attribute) in [
        (
            RACE_DETECT_DOORS_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Wisdom,
        ),
        (
            RACE_DETECT_TREASURE_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Charisma,
        ),
    ] {
        let ability = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
            .expect("temporary Dwarf form should grant both detection powers");
        assert_eq!(ability.source, AbilitySourceDto::Race);
        assert_eq!(ability.governing_attribute, Some(attribute));
    }
    assert_eq!(
        snapshot
            .player
            .pending_race_mutation_choice
            .expect("effective race must not replace birth-race rewards")
            .reward_id,
        "human-talent"
    );
    assert!(game.progress.locked_mutation_ids.is_empty());

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Normal
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| !matches!(
                ability.id.as_str(),
                RACE_DETECT_DOORS_ABILITY_ID | RACE_DETECT_TREASURE_ABILITY_ID
            ))
    );
}

#[test]
fn hobbit_intrinsics_follow_the_effective_race() {
    let mut game = Game::new_with_build_race_and_name(
        89,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    game.progress.level = 15;
    assert_eq!(game.player_infravision_range(), 0);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_CREATE_FOOD_ABILITY_ID)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.hobbit-form").status;
    form.granted_race_id = Some("rfb-legacy.race.hobbit".to_owned());
    game.player.statuses.push(form);
    assert_eq!(game.player_infravision_range(), 4);
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_CREATE_FOOD_ABILITY_ID)
        .expect("temporary Hobbit form should grant Create Food");
    assert_eq!(ability.source, AbilitySourceDto::Race);
    assert_eq!(
        ability.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(game.player_infravision_range(), 0);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_CREATE_FOOD_ABILITY_ID)
    );
}

fn mutation_ability_catalog(
    minimum_level: u16,
    cost: u32,
    base_failure_percent: u8,
) -> Arc<rfb_content::ContentCatalog> {
    mutation_ability_catalog_with_effect(
        minimum_level,
        cost,
        base_failure_percent,
        AbilityEffectDefinition::NoOp {
            reason: "mutation-contract".to_owned(),
        },
    )
}

#[test]
fn active_mutation_projects_without_learning_progress_or_persistent_cooldown() {
    let catalog = mutation_ability_catalog(1, 7, 30);
    let mut game =
        Game::from_content_with_build(0, catalog.clone(), DEFAULT_WORLD_ID, "demo.build.warrior")
            .expect("warrior build should create");
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != MUTATION_CONTRACT_ABILITY_ID)
    );

    let mut events = Vec::new();
    assert!(game.gain_mutation(MUTATION_CONTRACT_ID, &mut events));
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == MUTATION_CONTRACT_ABILITY_ID)
        .expect("gaining the mutation should project its ability");
    assert_eq!(ability.source, AbilitySourceDto::Mutation);
    assert_eq!(ability.resource_id, None);
    assert_eq!(ability.base_resource_cost, 7);
    assert_eq!(ability.resource_cost, 7);
    assert_eq!(ability.proficiency, 0);
    assert_eq!(ability.proficiency_cap, 0);
    assert_eq!(ability.cast_count, 0);
    assert_eq!(ability.fail_count, 0);
    assert_eq!(ability.cooldown_remaining, 0);
    assert_eq!(ability.cooldown_turns, 0);
    assert!(!ability.learned);
    assert!(!ability.can_study);
    assert!(!ability.can_forget);
    assert!(ability.can_cast);
    assert!(
        !game
            .ability_progress
            .contains_key(MUTATION_CONTRACT_ABILITY_ID)
    );

    let mut restored = Game::from_save_with_content(game.to_save(), catalog)
        .expect("active mutation ability should restore from existing mutation state");
    assert!(
        restored
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == MUTATION_CONTRACT_ABILITY_ID)
    );
    events.clear();
    assert!(restored.lose_mutation(MUTATION_CONTRACT_ID, &mut events));
    assert!(
        restored
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != MUTATION_CONTRACT_ABILITY_ID)
    );
}

#[test]
fn fear_blocks_class_and_mutation_power_sources_without_cost_or_rng() {
    let catalog = mutation_ability_catalog(1, 7, 30);
    let mutation = mutation_ability_game(catalog.clone(), "demo.build.warrior");
    let class = Game::from_content_with_build(0, catalog, DEFAULT_WORLD_ID, "demo.build.archer")
        .expect("Archer build should create");

    for (mut game, ability_id, expected_source) in [
        (
            mutation,
            MUTATION_CONTRACT_ABILITY_ID,
            AbilitySourceDto::Mutation,
        ),
        (
            class,
            "demo.ability.archer-create-shots",
            AbilitySourceDto::Class,
        ),
    ] {
        game.player
            .statuses
            .push(monster_combat::melee_status(STATUS_FEAR, 5, "test.fear").status);
        let projected = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == ability_id)
            .expect("the power should remain projected while afraid");
        assert_eq!(projected.source, expected_source);
        assert!(!projected.can_cast);
        let hp_before = game.player.hp;
        let draws_before = game.rng_draw_counter();
        let mut events = Vec::new();

        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("fear rejection should resolve cleanly");

        assert_eq!(game.player.hp, hp_before);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "afraid"
        ));
    }
}

#[test]
fn mutation_cast_spills_sp_into_hp_and_keeps_rejections_atomic() {
    let catalog = mutation_ability_catalog(1, 7, 30);
    let mut mana = mutation_ability_game(catalog.clone(), "test.build.caster");
    mana.debug_set_ability_casts_succeed(true);
    let resource_id = mana
        .casting_profile()
        .expect("test caster should have a casting profile")
        .resource_id
        .clone();
    mana.resources
        .get_mut(&resource_id)
        .expect("test caster should have an SP pool")
        .current = 10;
    let hp_before = mana.player.hp;
    let mut events = Vec::new();
    mana.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("mutation ability should resolve");
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(
        resolution.resource_id.as_deref(),
        Some(resource_id.as_str())
    );
    assert_eq!(resolution.resource_before, 10);
    assert_eq!(resolution.resource_after, 3);
    assert_eq!(resolution.resource_paid, 7);
    assert_eq!(resolution.hp_paid, 0);
    assert_eq!(mana.player.hp, hp_before);
    assert!(
        !mana
            .ability_progress
            .contains_key(MUTATION_CONTRACT_ABILITY_ID)
    );

    let mut spill = mutation_ability_game(catalog.clone(), "test.build.caster");
    spill.debug_set_ability_casts_succeed(true);
    spill
        .resources
        .get_mut(&resource_id)
        .expect("test caster should have an SP pool")
        .current = 3;
    let hp_before = spill.player.hp;
    events.clear();
    spill
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("mutation ability should spill into HP");
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(resolution.resource_paid, 3);
    assert_eq!(resolution.hp_paid, 4);
    assert_eq!(spill.player.hp, hp_before - 4);

    let mut hp_only = mutation_ability_game(catalog.clone(), "demo.build.warrior");
    hp_only.debug_set_ability_casts_succeed(true);
    let hp_before = hp_only.player.hp;
    events.clear();
    hp_only
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("a build without SP should pay HP only");
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(resolution.resource_id, None);
    assert_eq!(resolution.resource_paid, 0);
    assert_eq!(resolution.hp_paid, 7);
    assert_eq!(hp_only.player.hp, hp_before - 7);

    let mut rejected = mutation_ability_game(catalog, "demo.build.warrior");
    rejected.player.hp = 6;
    let draws_before = rejected.rng_draw_counter();
    events.clear();
    rejected
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("insufficient mutation budget should reject cleanly");
    assert_eq!(rejected.player.hp, 6);
    assert_eq!(rejected.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "insufficient-resource"
    ));

    rejected.player.hp = 20;
    events.clear();
    rejected
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::North,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("invalid mutation target should reject cleanly");
    assert_eq!(rejected.player.hp, 20);
    assert_eq!(rejected.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityTargetUnavailable { .. }]
    ));
}

#[test]
fn mutation_level_and_failure_paths_do_not_create_ability_progress() {
    let low_catalog = mutation_ability_catalog(2, 1, 30);
    let mut low = mutation_ability_game(low_catalog, "demo.build.warrior");
    let draws_before = low.rng_draw_counter();
    let hp_before = low.player.hp;
    let mut events = Vec::new();
    low.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("low-level mutation ability should reject cleanly");
    assert_eq!(low.player.hp, hp_before);
    assert_eq!(low.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "level-too-low"
    ));

    let fail_catalog = mutation_ability_catalog(1, 1, 95);
    let mut failed = mutation_ability_game(fail_catalog, "demo.build.warrior");
    failed.player.hp = 20;
    failed.rng = RfbRng::seeded(0);
    events.clear();
    failed
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("mutation failure should resolve");
    assert!(matches!(
        events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(failed.player.hp, 19);
    assert!(
        !failed
            .ability_progress
            .contains_key(MUTATION_CONTRACT_ABILITY_ID)
    );
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(resolution.failure_percent, 92);
    assert_eq!(resolution.hp_paid, 1);
    assert_eq!(resolution.proficiency_before, 0);
    assert_eq!(resolution.proficiency_after, 0);
    assert_eq!(resolution.cast_count, 0);
    assert_eq!(resolution.fail_count, 0);
}

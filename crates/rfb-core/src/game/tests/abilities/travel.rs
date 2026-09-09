// SPDX-License-Identifier: MPL-2.0

use super::*;

const RACE_PHASE_DOOR_ABILITY_ID: &str = "rfb.ability.race.phase-door";

const RACE_AMBERITE_SHADOW_SHIFTING_ABILITY_ID: &str = "rfb.ability.race.amberite-shadow-shifting";

const RACE_AMBERITE_PATTERN_MINDWALK_ABILITY_ID: &str =
    "rfb.ability.race.amberite-pattern-mindwalk";

#[test]
fn formal_amberite_passives_and_powers_match_the_authoritative_behavior() {
    let mut shadow = amberite_game(412);
    clear_monsters(&mut shadow);
    assert!(shadow.player_sustains_attribute(AttributeKind::Constitution));
    assert_eq!(shadow.player_regeneration_rate_percent(), 200);

    let mut polymorph =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.amberite-form").status;
    polymorph.granted_race_id = Some("rfb-legacy.race.small-kobold".to_owned());
    shadow.player.statuses.push(polymorph);
    assert!(!shadow.player_sustains_attribute(AttributeKind::Constitution));
    assert_eq!(shadow.player_regeneration_rate_percent(), 100);
    shadow
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);

    shadow.progress.level = 29;
    let snapshot = shadow.snapshot();
    for ability_id in [
        RACE_AMBERITE_SHADOW_SHIFTING_ABILITY_ID,
        RACE_AMBERITE_PATTERN_MINDWALK_ABILITY_ID,
    ] {
        assert!(
            !snapshot
                .player
                .abilities
                .iter()
                .find(|ability| ability.id == ability_id)
                .expect("Amberite power should project before unlocking")
                .can_cast
        );
    }

    shadow.progress.level = 30;
    shadow.progress.max_level = 30;
    shadow.refresh_character_skills();
    shadow.player.hp = shadow.effective_player_max_hp();
    shadow.debug_set_ability_casts_succeed(true);
    let projected = shadow
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_AMBERITE_SHADOW_SHIFTING_ABILITY_ID)
        .expect("level-thirty Shadow Shifting");
    assert!(projected.can_cast);
    assert_eq!(projected.source, AbilitySourceDto::Race);
    assert_eq!(projected.minimum_level, 30);
    assert_eq!(
        (projected.base_resource_cost, projected.resource_cost),
        (50, 50)
    );
    assert_eq!(
        projected.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert!(matches!(
        projected.effects.as_slice(),
        [AbilityEffectSpecDto::AlterReality]
    ));

    let hp_before = shadow.player.hp;
    let mut replay = shadow.clone();
    for cast in [&mut shadow, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_AMBERITE_SHADOW_SHIFTING_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Shadow Shifting should resolve");
        assert!((15..=35).contains(&cast.reality_change_ticks));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::AbilityEffectsResolved { resolution, .. }
                if matches!(
                    resolution.effects.as_slice(),
                    [AbilityEffectResolutionDto::AlterReality {
                        ticks_before: 0,
                        ticks_after: 15..=35,
                        ..
                    }]
                )
        )));
    }
    assert_eq!(shadow.player.hp, hp_before - 50);
    assert_eq!(shadow.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(shadow.to_save(), shadow.content.clone())
        .expect("Shadow Shifting countdown should restore");
    assert_eq!(restored.reality_change_ticks, shadow.reality_change_ticks);
    assert_eq!(restored.state_hash(), shadow.state_hash());

    shadow
        .resolve_player_ability(
            RACE_AMBERITE_SHADOW_SHIFTING_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("a second Shadow Shifting cast should cancel the countdown");
    assert_eq!(shadow.reality_change_ticks, 0);

    let mut pattern = amberite_game(413);
    clear_monsters(&mut pattern);
    pattern.progress.level = 40;
    pattern.progress.max_level = 40;
    pattern.progress.attributes = AttributeSet {
        strength: 3,
        intelligence: 3,
        wisdom: 3,
        dexterity: 3,
        constitution: 3,
        charisma: 3,
    };
    pattern.progress.maximum_attributes = AttributeSet {
        strength: 18,
        intelligence: 18,
        wisdom: 18,
        dexterity: 18,
        constitution: 3,
        charisma: 18,
    };
    pattern.refresh_character_skills();
    let drained_experience = crate::stats::experience_required_for_level(40);
    let maximum_experience = crate::stats::experience_required_for_level(41).saturating_sub(1);
    pattern.progress.experience = drained_experience;
    pattern.progress.maximum_experience = maximum_experience;
    pattern.progress.life_force = 125;
    for status_kind_id in [
        STATUS_POISON,
        STATUS_HALLUCINATION,
        STATUS_STUN,
        STATUS_BLEEDING,
        STATUS_BLINDNESS,
    ] {
        pattern
            .player
            .statuses
            .push(monster_combat::melee_status(status_kind_id, 20, "test.pattern-mindwalk").status);
    }
    pattern.player.hp = pattern.effective_player_max_hp();
    pattern.debug_set_ability_casts_succeed(true);
    let projected = pattern
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_AMBERITE_PATTERN_MINDWALK_ABILITY_ID)
        .expect("level-forty Pattern Mindwalking");
    assert!(projected.can_cast);
    assert_eq!(projected.minimum_level, 40);
    assert_eq!(
        (projected.base_resource_cost, projected.resource_cost),
        (75, 75)
    );
    assert_eq!(
        projected.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert!(matches!(
        projected.effects.last(),
        Some(AbilityEffectSpecDto::RestoreVitality { life_force: 1000 })
    ));

    let mut replay = pattern.clone();
    for cast in [&mut pattern, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_AMBERITE_PATTERN_MINDWALK_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Pattern Mindwalking should resolve");
        assert_eq!(mutation_cast_resolution(&events).hp_paid, 75);
        assert!(!events.iter().any(|event| {
            match event {
                DomainEvent::AbilityHealed { .. } => true,
                DomainEvent::AbilityEffectsResolved { resolution, .. } => resolution
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, AbilityEffectResolutionDto::Heal { .. })),
                _ => false,
            }
        }));
        assert_eq!(cast.progress.attributes, cast.progress.maximum_attributes);
        assert_eq!(cast.progress.experience, maximum_experience);
        assert_eq!(cast.progress.life_force, 1_000);
        for status_kind_id in [
            STATUS_POISON,
            STATUS_HALLUCINATION,
            STATUS_STUN,
            STATUS_BLEEDING,
            STATUS_BLINDNESS,
        ] {
            assert!(!cast.player_has_status_kind(status_kind_id));
        }
    }
    assert!(pattern.player.hp < pattern.effective_player_max_hp());
    assert_eq!(pattern.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(pattern.to_save(), pattern.content.clone())
        .expect("Pattern Mindwalking result should restore");
    assert_eq!(restored.state_hash(), pattern.state_hash());
}

#[test]
fn formal_gnome_phase_door_is_distinct_from_the_sorcery_spell() {
    let mut game = Game::new_with_build_race_and_name(
        98,
        "demo.build.high-mage-sorcery",
        "rfb-legacy.race.gnome",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Gnome Sorcery High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 4);
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));

    let level_four_experience = crate::stats::experience_required_for_level(4);
    game.apply_unscaled_player_experience(level_four_experience, &mut Vec::new());
    let snapshot = game.snapshot();
    let racial = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_PHASE_DOOR_ABILITY_ID)
        .expect("Gnome racial Phase Door should be projected");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(racial.minimum_level, 5);
    assert_eq!(racial.base_resource_cost, 2);
    assert!(!racial.can_cast);
    assert_eq!(
        snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.sorcery-phase-door")
            .expect("Sorcery Phase Door should keep its separate identity")
            .source,
        AbilitySourceDto::Learned
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(5) - level_four_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let position_before = game.player.position;
    game.resolve_player_ability(
        RACE_PHASE_DOOR_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Gnome racial Phase Door should resolve");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 2
    );
    assert_ne!(game.player.position, position_before);
    assert!(game.player.position.x.abs_diff(position_before.x) <= 10);
    assert!(game.player.position.y.abs_diff(position_before.y) <= 10);
}

#[test]
fn mutation_telekinesis_and_swap_position_reuse_directional_targeting() {
    let mut fetch = active_source_mutation_game(17, "telekinesis", 9);
    fetch.items.clear();
    let origin = fetch.player.position;
    for step in 0..=3 {
        replace_terrain(
            &mut fetch,
            Position {
                x: origin.x + step,
                y: origin.y,
            },
            "demo.terrain.floor",
        );
    }
    give_inventory_item(
        &mut fetch,
        "test.item.fetch",
        "demo.item.detect-objects-staff",
    );
    fetch
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.fetch")
        .expect("fetched item")
        .location = ItemLocation::Ground(Position {
        x: origin.x + 3,
        y: origin.y,
    });
    let mut events = Vec::new();
    fetch
        .resolve_player_ability(
            "rfb.ability.mutation.telekinesis",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("telekinesis should resolve");
    assert!(
        matches!(
            fetch
                .items
                .iter()
                .find(|item| item.id == "test.item.fetch")
                .map(|item| &item.location),
            Some(ItemLocation::Ground(position)) if *position == origin
        ),
        "telekinesis events: {events:?}"
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::FetchItem { moved: true, .. }])
    )));

    let mut swap = active_source_mutation_game(19, "swap-pos", 15);
    let origin = swap.player.position;
    let target = Position {
        x: origin.x + 2,
        y: origin.y,
    };
    for step in 0..=2 {
        replace_terrain(
            &mut swap,
            Position {
                x: origin.x + step,
                y: origin.y,
            },
            "demo.terrain.floor",
        );
    }
    swap.entities.push(actor_from_runtime_spawn(
        "test.actor.swap",
        "demo.actor.gnome-mage",
        target,
        20,
        50,
        100,
        true,
    ));
    swap.resolve_player_ability(
        "rfb.ability.mutation.swap-pos",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("swap position should resolve");
    assert_eq!(swap.player.position, target);
    assert_eq!(swap.entities[0].position, origin);
}

#[test]
fn blink_other_moves_the_target_within_ten_tiles_using_one_destination_draw() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    let caster = Position {
        x: player.x + 1,
        y: player.y,
    };
    let landing = Position {
        x: player.x + 5,
        y: player.y,
    };
    for position in [player, caster, landing] {
        let index = game.index(position).expect("test cell should exist");
        game.terrain[index] = "demo.terrain.floor".to_owned();
    }
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.gnome-mage",
        "demo.actor.gnome-mage",
        caster,
        31,
        110,
        100,
        true,
    ));

    let ability = game
        .content
        .ability("rfb-legacy.ability.blink-other")
        .expect("P29 ability should compile")
        .clone();
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::BlinkTarget { radius: 10 }
    ));
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("adjacent player should be a valid blink target");
    let MonsterAbilityTargetPlan::BlinkTarget { destinations, .. } = &plan.target else {
        panic!("BLINK_OTHER should plan a target blink");
    };
    assert_eq!(destinations, &[landing]);
    assert!(destinations.iter().all(|position| {
        player
            .x
            .abs_diff(position.x)
            .max(player.y.abs_diff(position.y))
            <= 10
    }));

    let draws = game.rng_draw_counter();
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed_entities = Vec::new();
    game.resolve_monster_ability_plan(
        0,
        "demo.actor.gnome-mage",
        &plan,
        &mut events,
        &mut changed,
        &mut removed_entities,
    );
    assert_eq!(game.rng_draw_counter(), draws + 1);
    assert_eq!(game.player.position, landing);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterBlinkedTarget { resolution, .. }
            if resolution.from == player && resolution.to == landing
    )));
}

// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn default_character_build_preserves_the_v70_player_baseline() {
    let game = Game::new(42);
    let snapshot = game.snapshot();
    let build = snapshot
        .player
        .build
        .expect("default build should be projected");

    assert_eq!(build.build_id, "demo.build.explorer");
    assert_eq!(build.race_id, "demo.race.human");
    assert_eq!(build.class_id, "demo.class.explorer");
    assert_eq!(build.personality_id, "demo.personality.ordinary");
    assert_eq!(snapshot.player.max_hp, 10);
    assert_eq!(snapshot.player.melee_skill, 40);
    assert_eq!(snapshot.player.progress.skills.len(), 10);
    assert!(snapshot.inventory.is_empty());
    assert!(snapshot.equipment.is_empty());
}

#[test]
fn v70_save_migrates_default_build_and_skills_without_rng_drift() {
    let canonical = Game::new(42);
    let mut legacy = canonical.to_save();
    legacy.content_hash =
        "ad6b35c6e0ae8980a74fac51ea1e6597b09559541d4a85d598284dc2cb41d7e6".to_owned();
    legacy.player.build = None;
    legacy
        .player
        .progress
        .as_mut()
        .expect("v70 save should contain character progress")
        .skills
        .clear();
    let draw_counter = legacy.rng.draw_counter;

    let migrated = Game::from_save(legacy).expect("v70 save should migrate character build");
    let snapshot = migrated.snapshot();
    assert_eq!(
        snapshot
            .player
            .build
            .as_ref()
            .map(|build| build.build_id.as_str()),
        Some("demo.build.explorer")
    );
    assert_eq!(snapshot.player.progress.skills.len(), 10);
    assert_eq!(migrated.rng_draw_counter(), draw_counter);
    assert_eq!(migrated.state_hash(), canonical.state_hash());

    let restored = Game::from_save(migrated.to_save())
        .expect("migrated character build should survive another round trip");
    assert_eq!(restored.state_hash(), migrated.state_hash());
}

#[test]
fn representative_builds_merge_identity_skills_attributes_and_starting_gear() {
    let vanguard =
        Game::new_with_build(42, "demo.build.vanguard").expect("vanguard build should create");
    let snapshot = vanguard.snapshot();
    assert_eq!(snapshot.player.build.as_ref().unwrap().life_percent, 115);
    assert_eq!(snapshot.player.max_hp, 33);
    assert_eq!(snapshot.player.progress.attributes.strength.effective, 18);
    assert_eq!(
        snapshot
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == "demo.skill.melee")
            .map(|skill| skill.current),
        Some(78)
    );
    assert_eq!(snapshot.player.melee_skill, 88);
    assert_eq!(snapshot.inventory.len(), 2);
    assert_eq!(snapshot.equipment.len(), 1);
    assert_eq!(snapshot.equipment[0].kind_id, "demo.item.echo-blade");

    let scholar =
        Game::new_with_build(42, "demo.build.scholar").expect("scholar build should create");
    let scholar_snapshot = scholar.snapshot();
    assert_eq!(
        scholar_snapshot
            .player
            .build
            .as_ref()
            .unwrap()
            .experience_percent,
        156
    );
    assert!(
        scholar_snapshot
            .equipment
            .iter()
            .any(|item| item.kind_id == "demo.item.echo-charm")
    );

    let pathfinder =
        Game::new_with_build(42, "demo.build.pathfinder").expect("pathfinder build should create");
    assert!(pathfinder.snapshot().player.projectile_profile.is_some());

    let tinkerer =
        Game::new_with_build(42, "demo.build.tinkerer").expect("tinkerer build should create");
    assert!(
        tinkerer
            .snapshot()
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == "demo.skill.device")
            .is_some_and(|skill| skill.current > 60)
    );
    assert_eq!(vanguard.rng_draw_counter(), scholar.rng_draw_counter());
}

#[test]
fn build_skill_growth_experience_multiplier_and_save_identity_are_deterministic() {
    let mut vanguard =
        Game::new_with_build(17, "demo.build.vanguard").expect("vanguard build should create");
    vanguard.apply_player_experience(380, &mut Vec::new());
    assert_eq!(vanguard.progress.level, 10);
    assert_eq!(
        vanguard
            .progress
            .skill("demo.skill.melee")
            .map(|skill| skill.current),
        Some(105)
    );

    let mut scholar =
        Game::new_with_build(17, "demo.build.scholar").expect("scholar build should create");
    scholar.apply_player_experience(100, &mut Vec::new());
    assert_eq!(scholar.progress.experience, 156);

    let restored = Game::from_save(vanguard.to_save()).expect("build save should reload");
    assert_eq!(restored.build, vanguard.build);
    assert_eq!(restored.progress.skills, vanguard.progress.skills);
    assert_eq!(restored.snapshot(), vanguard.snapshot());
    assert!(matches!(
        Game::new_with_build(17, "demo.build.missing"),
        Err(CoreError::UnknownCharacterBuild(_))
    ));
}

#[test]
fn restore_life_uses_historical_experience_and_migrates_old_saves() {
    let mut game = prepare_death_caster(0, 42, "demo.ability.death-restore-life");
    game.progress.experience = 500;
    game.progress.maximum_experience = 900;
    game.progress.life_force = 125;
    game.debug_set_ability_casts_succeed(true);
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.death-restore-life",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Restore Life should resolve");
    assert_eq!(game.progress.experience, 900);
    assert_eq!(game.progress.maximum_experience, 900);
    assert_eq!(game.progress.life_force, 1_000);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RestoreVitality {
                    experience_before: 500,
                    experience_after: 900,
                    life_force_before: 125,
                    life_force_after: 1_000,
                    ..
                }]
            )
    )));

    let mut legacy = Game::new(0);
    legacy.apply_player_experience(10, &mut Vec::new());
    let expected = legacy.progress.experience;
    let mut payload = legacy.to_save();
    payload
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved")
        .maximum_experience = 0;
    let migrated = Game::from_save(payload).expect("old progress should migrate");
    assert_eq!(migrated.progress.maximum_experience, expected);
}

#[test]
fn attribute_history_migrates_old_saves_and_rejects_inverted_values() {
    let mut legacy = Game::new(0).to_save();
    let progress = legacy
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved");
    let strength = progress.attributes.strength;
    progress.maximum_attributes = None;
    let migrated = Game::from_save(legacy).expect("old progress should migrate");
    assert_eq!(migrated.progress.maximum_attributes.strength, strength);

    let mut invalid = migrated.to_save();
    let progress = invalid
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved");
    let mut maximum = progress.attributes;
    maximum.strength = progress.attributes.strength.saturating_sub(1);
    progress.maximum_attributes = Some(maximum);
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("player attribute state is invalid"))
    ));
}

#[test]
fn attribute_resource_refresh_scales_the_prechange_current_value_once() {
    let mut game =
        Game::new_with_build(96, "demo.build.scholar").expect("scholar build should create");
    let before = *game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should have mana");
    assert_eq!(before.current, before.maximum);

    assert!(game.resolve_item_drain_attribute(
        "demo.item.frailty-tonic",
        AttributeKind::Intelligence,
        &mut Vec::new(),
    ));
    let drained = *game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should retain mana");
    assert!(drained.maximum < before.maximum);
    assert_eq!(drained.current, drained.maximum);

    assert!(game.resolve_item_restore_attribute(
        "demo.item.intelligence-renewal-tonic",
        AttributeKind::Intelligence,
        &mut Vec::new(),
    ));
    let restored = game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should retain mana");
    assert_eq!(restored.maximum, before.maximum);
    assert_eq!(restored.current, before.current);
}

#[test]
fn nearby_genocide_filters_radius_resists_unique_and_is_deterministic() {
    let prepare = || {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        for (id, kind_id, position) in [
            (
                "test.actor.nearby",
                "demo.actor.ember-mote",
                Position { x: 4, y: 3 },
            ),
            (
                "test.actor.unique",
                "demo.actor.serpent-of-chaos",
                Position { x: 5, y: 3 },
            ),
            (
                "test.actor.distant",
                "demo.actor.ember-mote",
                Position { x: 19, y: 19 },
            ),
        ] {
            let definition = game.content.actor(kind_id).expect("demo target").clone();
            game.entities.push(actor_from_runtime_spawn(
                id,
                kind_id,
                position,
                definition.max_hp,
                definition.speed,
                100,
                true,
            ));
        }
        game
    };
    let mut left = prepare();
    let mut right = left.clone();
    let resolve = |game: &mut Game| {
        let mut events = Vec::new();
        let mut removed = Vec::new();
        game.resolve_ability_genocide(
            "demo.ability.death-mass-genocide",
            None,
            AbilityGenocideScopeDefinition::Nearby,
            1_000,
            2,
            &mut events,
            &mut BTreeSet::new(),
            &mut removed,
        );
        (events, removed)
    };
    let (events, removed) = resolve(&mut left);
    let (_, right_removed) = resolve(&mut right);
    assert_eq!(left.state_hash(), right.state_hash());
    assert_eq!(removed, right_removed);
    assert_eq!(removed, ["test.actor.nearby"]);
    assert!(
        left.entities
            .iter()
            .any(|entity| entity.id == "test.actor.unique")
    );
    assert!(
        left.entities
            .iter()
            .any(|entity| entity.id == "test.actor.distant")
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::Genocide {
                    radius: 2,
                    removed_entity_ids,
                    resisted_entity_ids,
                    ..
                }] if removed_entity_ids == &["test.actor.nearby".to_owned()]
                    && resisted_entity_ids == &["test.actor.unique".to_owned()]
            )
    )));
}

#[test]
fn wraithform_passes_walls_halves_spell_damage_and_expires_in_place() {
    let mut game = prepare_death_caster(31, 47, "demo.ability.death-wraithform");
    game.refresh_character_skills();
    game.refresh_player_resource_maxima();
    let wall = Position { x: 4, y: 3 };
    game.items.retain(
        |item| !matches!(item.location, ItemLocation::Ground(position) if position == wall),
    );
    replace_terrain(&mut game, wall, "demo.terrain.wall");
    let mut ability = game
        .content
        .ability("demo.ability.death-wraithform")
        .expect("Wraithform should exist")
        .clone();
    Game::apply_player_level_scaling(&mut ability, 47);
    game.resolve_ability_actor_effects(
        &ability.id,
        &ability.effect,
        AbilityTargetPlan::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Wraithform should resolve");
    assert!(game.player_can_pass_walls());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, wall);
    assert_eq!(game.terrain_at(wall), "demo.terrain.wall");

    let hp_before = game.player.hp;
    let damage = game.resolve_monster_damage_to_player(
        "test.actor.caster",
        "demo.actor.gloom-weaver",
        "demo.ability.resonant-bolt",
        0,
        9,
        9,
        DamageType::Fire,
        &mut Vec::new(),
    );
    assert!(matches!(
        damage,
        AbilityEffectResolutionDto::Damage {
            resolution: DamageResolutionDto {
                final_damage: 5,
                ..
            },
            ..
        }
    ));
    assert_eq!(game.player.hp, hp_before - 5);

    let mut restored = Game::from_save(game.to_save()).expect("wall-bound Wraithform should load");
    assert_eq!(restored.snapshot(), game.snapshot());
    restored
        .player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == "rfb.status.wraithform")
        .expect("Wraithform should remain active")
        .remaining_ticks = 1;
    restored
        .process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("Wraithform expiry should resolve");
    assert!(!restored.player_can_pass_walls());
    assert_eq!(restored.player.position, wall);
    assert_eq!(restored.terrain_at(wall), "demo.terrain.wall");
}

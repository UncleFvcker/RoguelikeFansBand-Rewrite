// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_protocol::AttributeKindDto;

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
fn representative_builds_merge_identity_skills_attributes_and_starting_gear() {
    let warrior = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrior journey should create");
    let snapshot = warrior.snapshot();
    assert_eq!(snapshot.player.build.as_ref().unwrap().life_percent, 115);
    assert_eq!(snapshot.player.max_hp, 33);
    assert_eq!(snapshot.player.progress.attributes.strength.effective, 17);
    assert_eq!(
        snapshot
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == "demo.skill.melee")
            .map(|skill| skill.current),
        Some(73)
    );
    assert_eq!(snapshot.player.melee_skill, 71);
    assert!((202..=800).contains(&snapshot.player.gold));
    assert_eq!(snapshot.player.carry_capacity_tenths_pound, 1200);
    assert_eq!(snapshot.player.carried_weight_tenths_pound, 714);
    assert_eq!(
        snapshot
            .body_slots
            .iter()
            .map(|slot| (slot.id.as_str(), slot.slot_type.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("right-hand", "weapon"),
            ("left-hand", "shield"),
            ("shooting", "launcher"),
            ("quiver", "quiver"),
            ("right-ring", "ring"),
            ("left-ring", "ring"),
            ("neck", "amulet"),
            ("light", "light"),
            ("body", "body"),
            ("cloak", "cloak"),
            ("head", "head"),
            ("hands", "gloves"),
            ("feet", "boots"),
            ("container", "container"),
            ("tool", "tool"),
        ]
    );
    assert_eq!(snapshot.inventory.len(), 8);
    assert!(
        snapshot
            .inventory
            .iter()
            .any(|item| { item.kind_id == "demo.item.arrow" && item.quantity == 22 })
    );
    assert!(
        snapshot
            .inventory
            .iter()
            .any(|item| { item.kind_id == "demo.item.ration-of-food" && item.quantity == 9 })
    );
    assert_eq!(
        snapshot
            .inventory
            .iter()
            .filter(|item| item.kind_id == "demo.item.wooden-torch")
            .count(),
        6
    );
    assert_eq!(snapshot.equipment.len(), 3);
    assert!(
        snapshot
            .equipment
            .iter()
            .any(|item| item.kind_id == "demo.item.broad-sword")
    );
    assert!(
        snapshot
            .equipment
            .iter()
            .any(|item| item.kind_id == "demo.item.chain-mail")
    );
    assert!(
        snapshot
            .equipment
            .iter()
            .any(|item| item.kind_id == "demo.item.short-bow")
    );

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
    assert!(warrior.rng_draw_counter() > 5);
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
fn attribute_increase_command_commits_growth_without_rng_or_world_progression() {
    let mut game =
        Game::new_with_build(96, "demo.build.scholar").expect("scholar build should create");
    game.apply_player_experience(100, &mut Vec::new());
    assert!(game.progress.pending_attribute_increases > 0);

    let resource = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana");
    resource.current = resource.maximum / 3;
    let resource_before = *resource;
    let natural_before = game.progress.attributes.intelligence;
    let pending_before = game.progress.pending_attribute_increases;
    let draws_before = game.rng_draw_counter();
    let world_tick_before = game.world_tick;
    let energy_before = game.player.energy_need;
    let turn_before = game.turn;

    let update = dispatch_next(
        &mut game,
        GameCommand::IncreaseAttribute {
            attribute: AttributeKindDto::Intelligence,
        },
    );

    let resource_after = game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should retain mana");
    assert!(game.progress.attributes.intelligence > natural_before);
    assert_eq!(
        game.progress.pending_attribute_increases,
        pending_before - 1
    );
    assert!(resource_after.maximum > resource_before.maximum);
    assert_eq!(
        resource_after.current,
        u32::try_from(
            u64::from(resource_before.current) * u64::from(resource_after.maximum)
                / u64::from(resource_before.maximum)
        )
        .expect("scaled resource value should fit u32")
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.world_tick, world_tick_before);
    assert_eq!(game.player.energy_need, energy_before);
    assert_eq!(game.turn, turn_before + 1);
    assert_eq!(update.events.len(), 1);
    assert_eq!(update.events[0].kind, "player.attribute-increased");
    assert_eq!(
        update.events[0].args.get("pendingAttributeIncreases"),
        Some(&game.progress.pending_attribute_increases.to_string())
    );
}

#[test]
fn unavailable_attribute_increase_rejects_without_mutation_or_rng() {
    let mut game = Game::new(42);
    assert_eq!(game.progress.pending_attribute_increases, 0);
    let progress_before = game.progress.clone();
    let resources_before = game.resources.clone();
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();
    let world_tick_before = game.world_tick;
    let energy_before = game.player.energy_need;

    let update = dispatch_next(
        &mut game,
        GameCommand::IncreaseAttribute {
            attribute: AttributeKindDto::Strength,
        },
    );

    assert_eq!(game.progress, progress_before);
    assert_eq!(game.resources, resources_before);
    assert_eq!(game.player.hp, hp_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.world_tick, world_tick_before);
    assert_eq!(game.player.energy_need, energy_before);
    assert!(update.changed_cells.is_empty());
    assert_eq!(update.events.len(), 1);
    assert_eq!(
        update.events[0].kind,
        "player.attribute-increase-unavailable"
    );
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
        let mut ability = game
            .content
            .ability("demo.ability.death-mass-genocide")
            .expect("mass genocide ability should exist")
            .clone();
        ability.effect = AbilityEffectDefinition::Genocide {
            scope: AbilityGenocideScopeDefinition::Nearby,
            power: 1_000,
            radius: 2,
        };
        game.resolve_player_genocide_effect(
            &ability,
            None,
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
    game.resolve_player_actor_status_effect(
        &ability,
        AbilityTargetPlan::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
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

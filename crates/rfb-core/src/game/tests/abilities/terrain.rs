// SPDX-License-Identifier: MPL-2.0

use super::*;

const RACE_PROBE_MONSTERS_ABILITY_ID: &str = "rfb.ability.race.probe-monsters";

const RACE_STONE_TO_MUD_ABILITY_ID: &str = "rfb.ability.race.stone-to-mud";

const RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID: &str =
    "rfb.ability.race.wood-elf-nature-awareness";

const RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID: &str = "rfb.ability.race.explosive-rune";

const EXPLOSIVE_RUNE_TERRAIN_ID: &str = "demo.terrain.explosive-rune";

#[test]
fn formal_ogre_sustains_intelligence_and_places_capped_explosive_runes() {
    let mut game = ogre_game(423);
    clear_monsters(&mut game);
    assert!(game.player_sustains_attribute(AttributeKind::Intelligence));
    assert!(
        game.virtues
            .iter()
            .any(|virtue| virtue.kind == VirtueKindDto::Temperance)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 20, "test.ogre-form").status;
    form.granted_race_id = Some("rfb-legacy.race.small-kobold".to_owned());
    game.player.statuses.push(form);
    assert!(!game.player_sustains_attribute(AttributeKind::Intelligence));
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);

    game.progress.level = 24;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID)
        .expect("Explosive Rune should project before unlocking");
    assert!(!locked.can_cast);
    assert_eq!(locked.minimum_level, 25);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (35, 35));
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert!(matches!(
        locked.effects.as_slice(),
        [AbilityEffectSpecDto::CreateCurrentTerrain {
            target_terrain_id,
            ..
        }] if target_terrain_id == EXPLOSIVE_RUNE_TERRAIN_ID
    ));

    game.progress.level = 25;
    game.progress.max_level = 25;
    game.refresh_character_skills();
    game.player.hp = game.effective_player_max_hp();
    let position = game.player.position;
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.floor_connections
        .retain(|connection| connection.position != position);
    game.items.retain(
        |item| !matches!(item.location, ItemLocation::Ground(ground) if ground == position),
    );
    game.gold_piles.retain(|pile| pile.position != position);

    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID)
        .expect("level-twenty-five Explosive Rune");
    assert!(available.can_cast);
    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(available.failure_percent)
        })
        .expect("Explosive Rune should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_hp = failed.player.hp;
    let mut failed_events = Vec::new();
    failed
        .resolve_player_ability(
            RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut failed_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed Explosive Rune should resolve");
    assert_eq!(failed.player.hp, failed_hp - 35);
    assert_eq!(failed.terrain_at(position), "demo.terrain.floor");
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));

    game.debug_set_ability_casts_succeed(true);
    let hp_before = game.player.hp;
    game.resolve_player_ability(
        RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Explosive Rune should resolve");
    assert_eq!(game.player.hp, hp_before - 35);
    assert_eq!(game.terrain_at(position), EXPLOSIVE_RUNE_TERRAIN_ID);
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("placed Explosive Rune should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.terrain_at(position), EXPLOSIVE_RUNE_TERRAIN_ID);

    let mut capped = ogre_game(424);
    let player_index = capped
        .index(capped.player.position)
        .expect("player position");
    let mut rune_count = 0;
    for (index, terrain_id) in capped.terrain.iter_mut().enumerate() {
        if rune_count < 11 && index != player_index {
            *terrain_id = EXPLOSIVE_RUNE_TERRAIN_ID.to_owned();
            rune_count += 1;
        }
    }
    capped.terrain[player_index] = "demo.terrain.floor".to_owned();
    assert_eq!(rune_count, 11);
    assert!(
        capped
            .current_terrain_creation_replacement(
                &["demo.terrain.floor".to_owned()],
                EXPLOSIVE_RUNE_TERRAIN_ID,
            )
            .is_none()
    );
}

#[test]
fn explosive_rune_step_explodes_or_is_destroyed_by_the_authoritative_roll() {
    let mut base = ogre_game(425);
    clear_monsters(&mut base);
    base.progress.level = 25;
    base.progress.max_level = 25;
    base.refresh_character_skills();
    let rune = Position { x: 10, y: 10 };
    let start = Position { x: 11, y: 10 };
    let bystander = Position { x: 10, y: 11 };
    let safe = Position { x: 5, y: 5 };
    for position in [rune, start, bystander, safe] {
        replace_terrain(&mut base, position, "demo.terrain.floor");
    }
    replace_terrain(&mut base, rune, EXPLOSIVE_RUNE_TERRAIN_ID);
    base.player.position = safe;
    base.push_generated_actor(
        "test.ogre-rune-stepper".to_owned(),
        "demo.actor.newt",
        start,
    );
    let monster_level = base
        .content
        .actor("demo.actor.newt")
        .expect("Newt definition")
        .level;
    let break_roll_sides = 299_u64 * u64::from(base.progress.level) / 50;
    let explode_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(break_roll_sides) + 1 > u64::from(monster_level)
        })
        .expect("Explosive Rune should have a triggering seed");
    let disarm_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(break_roll_sides) < u64::from(monster_level)
        })
        .expect("Explosive Rune should have a destruction seed");

    let restored = Game::from_save_with_content(base.to_save(), base.content.clone())
        .expect("armed Explosive Rune should restore");
    assert_eq!(restored.state_hash(), base.state_hash());

    let mut exploded = base.clone();
    exploded.push_generated_actor(
        "test.ogre-rune-bystander".to_owned(),
        "demo.actor.newt",
        bystander,
    );
    exploded.rng = RfbRng::seeded(explode_seed);
    let mut events = Vec::new();
    let mut removed = Vec::new();
    assert_eq!(
        exploded
            .move_entity(0, rune, &mut events, &mut BTreeSet::new(), &mut removed)
            .expect("monster rune step should resolve"),
        ActorStepOutcome::Removed
    );
    assert_eq!(exploded.terrain_at(rune), "demo.terrain.floor");
    assert!(removed.contains(&"test.ogre-rune-stepper".to_owned()));
    assert!(removed.contains(&"test.ogre-rune-bystander".to_owned()));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage {
            ability_id,
            resolution,
            ..
        } if ability_id == RACE_OGRE_EXPLOSIVE_RUNE_ABILITY_ID
            && resolution.center == rune
            && resolution.radius == 2
            && (64..=148).contains(&resolution.base_raw_damage)
            && resolution.target_count == 2
    )));

    let mut disarmed = base;
    disarmed.rng = RfbRng::seeded(disarm_seed);
    let hp_before = disarmed.entities[0].hp;
    let mut events = Vec::new();
    assert_eq!(
        disarmed
            .move_entity(0, rune, &mut events, &mut BTreeSet::new(), &mut Vec::new(),)
            .expect("monster rune destruction should resolve"),
        ActorStepOutcome::Moved
    );
    assert_eq!(disarmed.terrain_at(rune), "demo.terrain.floor");
    assert_eq!(disarmed.entities[0].hp, hp_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterTerrainDestroyed {
            terrain_kind_id,
            replacement_terrain_kind_id,
            position,
            ..
        } if terrain_kind_id == EXPLOSIVE_RUNE_TERRAIN_ID
            && replacement_terrain_kind_id == "demo.terrain.floor"
            && *position == rune
    )));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityAreaDamage { .. }))
    );
}

#[test]
fn formal_wood_elf_nature_awareness_unlocks_at_twenty_and_reuses_full_detection() {
    let mut game = wood_elf_game(385);
    clear_monsters(&mut game);
    game.progress.level = 19;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID)
        .expect("Wood-Elf Nature Awareness should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(locked.minimum_level, 20);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (15, 15));
    assert!(!locked.can_cast);

    game.progress.level = 20;
    game.progress.max_level = 20;
    game.refresh_character_skills();
    game.player.hp = game.effective_player_max_hp();
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID)
        .expect("level-twenty Wood-Elf Nature Awareness");
    assert!(available.can_cast);
    assert!(available.failure_percent >= 50);
    assert_eq!(available.effects.len(), 6);
    assert!(
        available
            .effects
            .iter()
            .all(|effect| matches!(effect, AbilityEffectSpecDto::Detect { radius: 30, .. }))
    );

    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    let trap = Position { x: 4, y: 3 };
    let door = Position { x: 5, y: 3 };
    let stairs_down = Position { x: 6, y: 3 };
    let stairs_up = Position { x: 7, y: 3 };
    let monster = Position { x: 3, y: 4 };
    for (position, terrain_id) in [
        (game.player.position, "demo.terrain.floor"),
        (trap, "demo.terrain.created-trap"),
        (door, "demo.terrain.door-secret"),
        (stairs_down, "demo.terrain.stairs-down"),
        (stairs_up, "demo.terrain.stairs-up"),
        (monster, "demo.terrain.floor"),
    ] {
        replace_terrain(&mut game, position, terrain_id);
    }
    for position in [trap, door, stairs_down, stairs_up] {
        let index = game.index(position).expect("detection target should exist");
        game.explored[index] = false;
        game.revealed_terrain.remove(&position);
    }
    game.push_generated_actor(
        "test.wood-elf-detection".to_owned(),
        "demo.actor.sheep",
        monster,
    );

    let hp_before = game.player.hp;
    let mut replay = game.clone();
    for cast in [&mut game, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Wood-Elf Nature Awareness should resolve");

        let detections = events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::AbilityDetected { resolution, .. } => Some(resolution),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(detections.len(), 6);
        for category in ["map", "trap", "door", "stairs-down", "stairs-up"] {
            assert!(
                detections
                    .iter()
                    .any(|detection| detection.category == category)
            );
        }
        assert!(detections.iter().any(|detection| {
            detection.category == "normal-monster"
                && detection
                    .detected_entity_ids
                    .iter()
                    .any(|id| id == "test.wood-elf-detection")
        }));
        assert!(cast.revealed_terrain.contains(&trap));
        assert!(cast.revealed_terrain.contains(&door));
        assert!(cast.explored[cast.index(stairs_down).expect("stairs should exist")]);
        assert!(cast.explored[cast.index(stairs_up).expect("stairs should exist")]);
    }
    assert_eq!(game.player.hp, hp_before - 15);
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Wood-Elf detection knowledge should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn formal_dwarf_detection_powers_reveal_original_terrain_categories_only() {
    let mut game = Game::new_with_build_race_and_name(
        94,
        "demo.build.high-mage-death",
        "rfb-legacy.race.dwarf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Dwarf High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Resistant
    );

    let level_four_experience = crate::stats::experience_required_for_level(4);
    game.apply_unscaled_player_experience(level_four_experience, &mut Vec::new());
    let snapshot = game.snapshot();
    let doors = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_DETECT_DOORS_ABILITY_ID)
        .expect("Dwarf door detection should be projected before it unlocks");
    assert_eq!(doors.source, AbilitySourceDto::Race);
    assert_eq!(
        doors.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(doors.minimum_level, 5);
    assert_eq!(doors.base_resource_cost, 5);
    assert!(!doors.can_cast);
    let treasure = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
        .expect("Dwarf treasure detection should be projected before it unlocks");
    assert_eq!(treasure.source, AbilitySourceDto::Race);
    assert_eq!(
        treasure.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Charisma)
    );
    assert_eq!(treasure.minimum_level, 10);
    assert_eq!(treasure.base_resource_cost, 5);
    assert!(!treasure.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(5) - level_four_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let snapshot = game.snapshot();
    assert!(
        snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_DOORS_ABILITY_ID)
            .expect("Dwarf door detection should remain projected")
            .can_cast
    );
    assert!(
        !snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should remain projected")
            .can_cast
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(9)
            - crate::stats::experience_required_for_level(5),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should retain mana");
    mana.current = mana.maximum;
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should remain projected")
            .can_cast
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10)
            - crate::stats::experience_required_for_level(9),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should retain mana");
    mana.current = mana.maximum;
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should unlock at level ten")
            .can_cast
    );

    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    let player = game.player.position;
    let blocker = Position { x: 4, y: 3 };
    let trap = Position { x: 5, y: 3 };
    let door = Position { x: 6, y: 3 };
    let stairs_down = Position { x: 7, y: 3 };
    let stairs_up = Position { x: 8, y: 3 };
    let magma = Position { x: 5, y: 4 };
    let quartz = Position { x: 6, y: 4 };
    let gold_blocker = Position { x: 4, y: 2 };
    let gold = Position { x: 5, y: 2 };
    for (position, terrain_id) in [
        (player, "demo.terrain.floor"),
        (blocker, "demo.terrain.wall"),
        (trap, "demo.terrain.created-trap"),
        (door, "demo.terrain.door-secret"),
        (stairs_down, "demo.terrain.stairs-down"),
        (stairs_up, "demo.terrain.stairs-up"),
        (magma, "demo.terrain.magma-hidden-treasure"),
        (quartz, "demo.terrain.quartz-hidden-treasure"),
        (gold_blocker, "demo.terrain.wall"),
        (gold, "demo.terrain.floor"),
    ] {
        replace_terrain(&mut game, position, terrain_id);
    }
    for position in [trap, door, stairs_down, stairs_up, magma, quartz] {
        let index = game.index(position).expect("detection target should exist");
        game.explored[index] = false;
        game.revealed_terrain.remove(&position);
    }
    game.gold_piles = vec![GoldPile {
        id: "generated.gold.1".to_owned(),
        position: gold,
        amount: 25,
        appearance: GoldAppearanceDto::Gold,
        discovered: false,
    }];
    game.next_gold_pile_serial = 2;
    let mana_before = game.resources["demo.resource.mana"].current;
    let mut replay = game.clone();

    for cast in [&mut game, &mut replay] {
        cast.resolve_player_ability(
            RACE_DETECT_DOORS_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dwarf door detection should resolve");
        assert!(cast.revealed_terrain.contains(&trap));
        assert!(cast.revealed_terrain.contains(&door));
        assert!(cast.explored[cast.index(stairs_down).expect("stairs should exist")]);
        assert!(cast.explored[cast.index(stairs_up).expect("stairs should exist")]);
        assert!(!cast.revealed_terrain.contains(&magma));
        assert!(!cast.revealed_terrain.contains(&quartz));

        cast.resolve_player_ability(
            RACE_DETECT_TREASURE_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dwarf treasure detection should resolve");
        assert!(cast.revealed_terrain.contains(&magma));
        assert!(cast.revealed_terrain.contains(&quartz));
        assert!(!cast.gold_piles[0].discovered);
    }

    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Dwarf detection knowledge should restore");
    for position in [trap, door, magma, quartz] {
        assert!(restored.revealed_terrain.contains(&position));
    }
    assert!(
        restored.explored[restored
            .index(stairs_down)
            .expect("restored stairs should exist")]
    );
    assert!(
        restored.explored[restored
            .index(stairs_up)
            .expect("restored stairs should exist")]
    );
    assert!(!restored.gold_piles[0].discovered);
}

#[test]
fn formal_nibelung_intrinsics_and_detection_powers_unlock_at_level_ten() {
    let mut game = Game::new_with_build_race_and_name(
        97,
        "demo.build.high-mage-death",
        "rfb-legacy.race.nibelung",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Nibelung High-Mage should create");
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Disenchant),
        ResistanceLevel::Resistant
    );

    let level_nine_experience = crate::stats::experience_required_for_level(9);
    game.apply_unscaled_player_experience(level_nine_experience, &mut Vec::new());
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
            .expect("Nibelung detection power should be projected");
        assert_eq!(ability.source, AbilitySourceDto::Race);
        assert_eq!(ability.governing_attribute, Some(attribute));
        assert_eq!(ability.minimum_level, 10);
        assert_eq!(ability.base_resource_cost, 5);
        assert!(!ability.can_cast);
    }

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10) - level_nine_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let snapshot = game.snapshot();
    for ability_id in [
        RACE_DETECT_DOORS_ABILITY_ID,
        RACE_DETECT_TREASURE_ABILITY_ID,
    ] {
        assert!(
            snapshot
                .player
                .abilities
                .iter()
                .find(|ability| ability.id == ability_id)
                .expect("Nibelung detection power should remain projected")
                .can_cast
        );
    }
}

#[test]
fn formal_half_giant_stone_to_mud_does_not_grant_mining_rewards() {
    let mut game = Game::new_with_build_race_and_name(
        99,
        "demo.build.high-mage-death",
        "rfb-legacy.race.half-giant",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Half-Giant High-Mage should create");
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    assert_eq!(game.player_infravision_range(), 3);
    assert!(game.player_sustains_attribute(AttributeKind::Strength));
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Shards),
        ResistanceLevel::Resistant
    );

    let level_nineteen_experience = crate::stats::experience_required_for_level(19);
    game.apply_unscaled_player_experience(level_nineteen_experience, &mut Vec::new());
    let racial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_STONE_TO_MUD_ABILITY_ID)
        .expect("Half-Giant Stone to Mud should be projected");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength)
    );
    assert_eq!(racial.minimum_level, 20);
    assert_eq!(racial.base_resource_cost, 10);
    assert!(!racial.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(20) - level_nineteen_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.quartz-vein");
    game.progress.mining_proficiency = 3_999;
    let materials_before = game.progress.materials.clone();
    let item_serial_before = game.next_item_instance_serial;

    game.resolve_player_ability(
        RACE_STONE_TO_MUD_ABILITY_ID,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Half-Giant Stone to Mud should resolve");

    assert_eq!(game.terrain_at(target), "demo.terrain.floor");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert_eq!(game.progress.mining_proficiency, 3_999);
    assert_eq!(game.progress.materials, materials_before);
    assert!(game.gold_piles.is_empty());
    assert!(game.items.is_empty());
    assert_eq!(game.next_item_instance_serial, item_serial_before);
}

#[test]
fn half_titan_probe_knowledge_survives_losing_the_race_power_and_reloading() {
    let mut game = Game::new_with_build_race_and_name(
        102,
        "demo.build.high-mage-death",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human High-Mage should create");
    clear_monsters(&mut game);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.half-titan-form").status;
    form.granted_race_id = Some("rfb-legacy.race.half-titan".to_owned());
    game.player.statuses.push(form);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Chaos),
        ResistanceLevel::Resistant
    );

    let level_fourteen_experience = crate::stats::experience_required_for_level(14);
    game.apply_unscaled_player_experience(level_fourteen_experience, &mut Vec::new());
    let racial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_PROBE_MONSTERS_ABILITY_ID)
        .expect("temporary Half-Titan form should grant monster probing");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(racial.minimum_level, 15);
    assert_eq!(racial.base_resource_cost, 10);
    assert!(!racial.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(15) - level_fourteen_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.floor");
    let target_index = game.index(target).expect("probe target should exist");
    game.glow[target_index] = true;
    game.push_generated_actor(
        "test.half-titan-probe".to_owned(),
        "demo.actor.sheep",
        target,
    );
    let mut events = Vec::new();
    game.resolve_player_ability(
        RACE_PROBE_MONSTERS_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Half-Titan monster probing should resolve");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityMonstersProbed { resolution, .. }
            if resolution.monsters.iter().any(|monster| monster.kind_id == "demo.actor.sheep")
    )));

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    game.refresh_player_resource_maxima();
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_PROBE_MONSTERS_ABILITY_ID)
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    let hash = game.state_hash();
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("probe knowledge should not require a current Sniper or Half-Titan source");
    assert!(restored.probed_actor_kind_ids.contains("demo.actor.sheep"));
    assert_eq!(restored.state_hash(), hash);
}

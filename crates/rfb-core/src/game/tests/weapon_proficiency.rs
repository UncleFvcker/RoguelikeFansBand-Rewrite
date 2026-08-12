// SPDX-License-Identifier: MPL-2.0

use super::*;

fn equipped_item_id(game: &Game, slot_type: &str) -> String {
    game.items
        .iter()
        .find(|item| {
            matches!(
                &item.location,
                ItemLocation::Equipped { slot_id }
                    if game.body_slot_type(slot_id) == Some(slot_type)
            )
        })
        .unwrap_or_else(|| panic!("test character should have an equipped {slot_type}"))
        .id
        .clone()
}

fn open_direction(game: &Game) -> (Direction, Position) {
    [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ]
    .into_iter()
    .find_map(|direction| {
        let (dx, dy) = direction.delta();
        let position = Position {
            x: game.player.position.x + dx,
            y: game.player.position.y + dy,
        };
        (game.index(position).is_some() && game.is_walkable(position))
            .then_some((direction, position))
    })
    .expect("player should have an adjacent open cell")
}

fn place_training_target(game: &mut Game) -> Direction {
    let (direction, position) = open_direction(game);
    let definition = game
        .content
        .actor_definitions()
        .filter(|definition| definition.role == ActorRole::Monster && definition.level >= 20)
        .min_by_key(|definition| (&definition.id, definition.level))
        .expect("demo content should contain a level-20 monster")
        .clone();
    game.entities.clear();
    let mut target = actor_from_runtime_spawn(
        "test.weapon-proficiency-target",
        &definition.id,
        position,
        1_000_000,
        definition.speed,
        INITIAL_MONSTER_ENERGY_NEED,
        true,
    );
    target.resistances = definition_resistance_profile(&definition);
    game.entities.push(target);
    direction
}

#[test]
fn growth_uses_original_gates_rng_remainders_and_bonus_notifications() {
    let mut game = Game::new(0x5052_4f46);
    let weapon_id = equipped_item_id(&game, "weapon");
    let before = game.rng.draw_counter;

    assert_eq!(game.train_weapon_proficiency(&weapon_id, 80), None);
    assert_eq!(
        game.progress
            .weapon_proficiencies
            .get("demo.item.broad-sword"),
        Some(&4_008)
    );
    assert_eq!(game.rng.draw_counter, before);

    game.progress
        .weapon_proficiencies
        .insert("demo.item.broad-sword".to_owned(), 4_199);
    let before_remainder = game.rng.draw_counter;
    assert_eq!(
        game.train_weapon_proficiency(&weapon_id, 80).as_deref(),
        Some("demo.item.broad-sword")
    );
    assert_eq!(game.rng.draw_counter, before_remainder + 1);

    game.progress.level = 50;
    game.progress
        .weapon_proficiencies
        .insert("demo.item.broad-sword".to_owned(), 4_500);
    let gated = game.rng.draw_counter;
    assert_eq!(game.train_weapon_proficiency(&weapon_id, 34), None);
    assert_eq!(game.rng.draw_counter, gated);

    game.progress.level = 1;
    game.progress
        .weapon_proficiencies
        .insert("demo.item.broad-sword".to_owned(), 5_000);
    assert_eq!(game.train_weapon_proficiency(&weapon_id, 20), None);
    assert_eq!(game.rng.draw_counter, gated);
}

#[test]
fn ordinary_melee_trains_once_before_multiple_missed_blows() {
    let mut game = Game::new(0x004d_454c_4545);
    place_training_target(&mut game);
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_STUN.to_owned(),
        intensity: 100,
        remaining_ticks: 100,
        source_id: Some("test.weapon-proficiency".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: rfb_protocol::EquipmentBonusesDto {
            melee_attacks: 2,
            ..Default::default()
        },
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let mut events = Vec::new();
    game.resolve_player_melee(0, true, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("ordinary melee should resolve");

    assert_eq!(
        game.progress
            .weapon_proficiencies
            .get("demo.item.broad-sword"),
        Some(&4_008)
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::PlayerMeleeMissed { .. }))
            .count(),
        3
    );
}

#[test]
fn projectile_collision_trains_but_an_empty_shot_does_not_touch_progress_or_rng() {
    let mut collision =
        Game::new_with_build(0x0042_4f57, "demo.build.archer").expect("Archer should create");
    let direction = place_training_target(&mut collision);
    collision
        .resolve_player_projectile(
            TargetSelection::Direction { direction },
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("colliding projectile should resolve");
    assert_eq!(
        collision
            .progress
            .weapon_proficiencies
            .get("demo.item.short-bow"),
        Some(&4_008)
    );

    let mut empty =
        Game::new_with_build(0x0045_4d50_5459, "demo.build.archer").expect("Archer should create");
    empty.entities.clear();
    empty
        .progress
        .weapon_proficiencies
        .insert("demo.item.short-bow".to_owned(), 7_999);
    let (direction, _) = open_direction(&empty);
    let before = empty.rng.draw_counter;
    empty
        .resolve_player_projectile(
            TargetSelection::Direction { direction },
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("empty projectile should resolve");
    assert_eq!(empty.rng.draw_counter, before);
    assert_eq!(
        empty
            .progress
            .weapon_proficiencies
            .get("demo.item.short-bow"),
        Some(&7_999)
    );
}

#[test]
fn artifact_training_uses_only_the_canonical_base_weapon_key() {
    let mut game = Game::new(0x4152_5449_4641_4354);
    let weapon_id = equipped_item_id(&game, "weapon");
    game.items
        .iter_mut()
        .find(|item| item.id == weapon_id)
        .expect("equipped weapon should remain present")
        .kind_id = "demo.item.crisdurian".to_owned();

    assert_eq!(game.train_weapon_proficiency(&weapon_id, 80), None);
    assert_eq!(
        game.progress
            .weapon_proficiencies
            .get("demo.item.executioners-sword"),
        Some(&4_008)
    );
    assert!(
        !game
            .progress
            .weapon_proficiencies
            .contains_key("demo.item.crisdurian")
    );
}

#[test]
fn progression_projection_lists_base_weapons_with_original_ranks_and_bonuses() {
    let snapshot = Game::new(0x0050_524f_4a45_4354).snapshot();
    let proficiencies = &snapshot.player.progress.weapon_proficiencies;
    assert!(
        proficiencies
            .iter()
            .all(|entry| entry.item_kind_id != "demo.item.crisdurian")
    );

    let executioner = proficiencies
        .iter()
        .find(|entry| entry.item_kind_id == "demo.item.executioners-sword")
        .expect("the canonical artifact base should be projected");
    assert_eq!(executioner.name_key, "item-demo-executioners-sword-name");
    assert_eq!(
        executioner.category,
        rfb_protocol::WeaponProficiencyCategoryDto::Melee
    );

    let bow = proficiencies
        .iter()
        .find(|entry| entry.item_kind_id == "demo.item.short-bow")
        .expect("short bow proficiency should be projected");
    assert_eq!(bow.current, 4_000);
    assert_eq!(bow.maximum, 7_000);
    assert_eq!(bow.hit_bonus, 0);
    assert_eq!(bow.rank, rfb_protocol::ProficiencyRankDto::Beginner);
    assert_eq!(
        bow.category,
        rfb_protocol::WeaponProficiencyCategoryDto::Launcher
    );

    let crossbow = proficiencies
        .iter()
        .find(|entry| entry.item_kind_id == "demo.item.light-crossbow")
        .expect("light crossbow proficiency should be projected");
    assert_eq!(crossbow.current, 4_000);
    assert_eq!(crossbow.hit_bonus, 10);
}

#[test]
fn sparse_weapon_progress_round_trips_and_rejects_noncanonical_or_out_of_range_entries() {
    let mut game = Game::new(0x5341_5645);
    game.progress
        .weapon_proficiencies
        .insert("demo.item.broad-sword".to_owned(), 4_008);
    let saved = game.to_save();
    let entries = &saved
        .player
        .progress
        .as_ref()
        .expect("new save should include character progress")
        .weapon_proficiencies;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].item_kind_id, "demo.item.broad-sword");
    let restored = Game::from_save(saved.clone()).expect("valid proficiency should round-trip");
    assert_eq!(
        restored.progress.weapon_proficiencies,
        game.progress.weapon_proficiencies
    );

    for (item_kind_id, current) in [
        ("demo.item.ration-of-food", 4_008),
        ("demo.item.unknown", 4_008),
        ("demo.item.broad-sword", 4_000),
        ("demo.item.broad-sword", 8_001),
        ("demo.item.crisdurian", 4_008),
    ] {
        let mut invalid = saved.clone();
        invalid
            .player
            .progress
            .as_mut()
            .expect("new save should include progress")
            .weapon_proficiencies = vec![rfb_protocol::WeaponProficiencySaveDto {
            item_kind_id: item_kind_id.to_owned(),
            current,
        }];
        assert!(matches!(
            Game::from_save(invalid),
            Err(CoreError::InvalidSave(
                "player weapon proficiency state is invalid"
            ))
        ));
    }

    let mut duplicate = saved.clone();
    duplicate
        .player
        .progress
        .as_mut()
        .expect("new save should include progress")
        .weapon_proficiencies = vec![
        rfb_protocol::WeaponProficiencySaveDto {
            item_kind_id: "demo.item.broad-sword".to_owned(),
            current: 4_008,
        },
        rfb_protocol::WeaponProficiencySaveDto {
            item_kind_id: "demo.item.broad-sword".to_owned(),
            current: 4_009,
        },
    ];
    assert!(matches!(
        Game::from_save(duplicate),
        Err(CoreError::InvalidSave(
            "player weapon proficiency state is invalid"
        ))
    ));

    let empty = Game::new(0x0045_4d50_5459).to_save();
    assert!(
        empty
            .player
            .progress
            .expect("new save should include progress")
            .weapon_proficiencies
            .is_empty()
    );
}

#[test]
fn weapon_proficiency_save_field_is_required_for_new_progress_payloads() {
    let mut value =
        serde_json::to_value(Game::new(0x5354_5249_4354).to_save()).expect("save should serialize");
    value["player"]["progress"]
        .as_object_mut()
        .expect("progress should be an object")
        .remove("weaponProficiencies");
    assert!(serde_json::from_value::<rfb_protocol::SavePayloadV1>(value).is_err());
}

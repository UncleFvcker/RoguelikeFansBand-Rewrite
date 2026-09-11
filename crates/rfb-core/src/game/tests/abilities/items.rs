// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::projectile_geometry::rfb_distance;

#[test]
fn death_weapon_branding_targets_plain_weapons_across_player_locations() {
    for (ability_id, expected_affix_id, location) in [
        (
            "demo.ability.death-poison-branding",
            "rfb-legacy.affix.slaying",
            ItemLocation::Inventory,
        ),
        (
            "demo.ability.death-vampiric-branding",
            "rfb-legacy.affix.death",
            ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
        ),
        (
            "demo.ability.death-vampiric-branding",
            "rfb-legacy.affix.death",
            ItemLocation::Ground(Position { x: 5, y: 5 }),
        ),
    ] {
        let level = if ability_id.ends_with("poison-branding") {
            30
        } else {
            40
        };
        let mut game = prepare_death_caster(7, level, ability_id);
        set_test_virtue(&mut game, 0, VirtueKindDto::Enchantment, 0);
        game.debug_set_ability_casts_succeed(true);
        game.player.position = Position { x: 5, y: 5 };
        game.items.retain(|item| {
            game.content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.ability_book_id.is_some())
        });
        give_inventory_item(&mut game, "test.brand-target", "demo.item.dagger");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.brand-target")
            .expect("branding target")
            .location = location;
        let mut events = Vec::new();

        game.resolve_player_ability(
            ability_id,
            TargetSelection::Item {
                item_id: "test.brand-target".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("legal branding should resolve");

        let item = game
            .items
            .iter()
            .find(|item| item.id == "test.brand-target")
            .expect("branded target");
        assert_eq!(item.affix_ids, [expected_affix_id], "{events:#?}");
        assert_eq!(item.origin_kind, Some(ItemOriginKindDto::PlayerMade));
        assert_eq!(item.discount_percent, 99);
        assert_eq!(item.quality, ItemQualityDto::Fine);
        assert_eq!(game.virtue_current(VirtueKindDto::Enchantment), 2);
        assert!((0..=6).contains(&item.enchantments.to_hit));
        assert!((0..=6).contains(&item.enchantments.to_damage));
        assert_eq!(item.rolled_affixes.len(), 1);
        if ability_id.ends_with("poison-branding") {
            assert!(
                !item.rolled_affixes[0].properties.slays.is_empty(),
                "explicit Slaying references must use the shared source-index 1 materializer"
            );
            assert!(
                item.rolled_affixes[0]
                    .properties
                    .brands
                    .contains(&WeaponBrand::Poison)
            );
            assert_eq!(
                item.rolled_affixes[0]
                    .properties
                    .resistances
                    .get(&ActorDamageType::Poison),
                Some(&ActorResistanceLevel::Resistant)
            );
        } else {
            assert_eq!(item.rolled_affixes[0].affix_id, expected_affix_id);
            assert!(
                game.content
                    .affix(expected_affix_id)
                    .expect("Death affix")
                    .passives
                    .contains(&EquipmentPassive::Vampiric)
            );
        }
        let expected_attempts = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    match resolution.effects.as_slice() {
                        [
                            AbilityEffectResolutionDto::BrandWeapon {
                                to_hit, to_damage, ..
                            },
                        ] => {
                            assert_eq!(to_hit.attempts, to_damage.attempts);
                            Some(to_hit.attempts)
                        }
                        _ => None,
                    }
                }
                _ => None,
            })
            .expect("branding resolution should be emitted");
        assert!((4..=6).contains(&expected_attempts));
        let knowledge = &game.item_property_knowledge["test.brand-target"];
        assert!(knowledge.discovered);
        assert!(knowledge.appraised);
        assert!(knowledge.identified);
        assert!(knowledge.known_affix_ids.contains(expected_affix_id));
    }

    let mut saved = Game::new(12);
    give_inventory_item(&mut saved, "test.saved-brand", "demo.item.dagger");
    let ability = saved
        .content
        .ability("demo.ability.death-vampiric-branding")
        .expect("vampiric branding content")
        .clone();
    saved.resolve_player_brand_weapon_effect(&ability, "test.saved-brand", &mut Vec::new());
    Game::from_save(saved.to_save()).expect("branded weapon should round-trip");
}

#[test]
fn death_weapon_branding_rejects_nonplain_or_unavailable_weapons_without_rng() {
    let mut game = prepare_death_caster(9, 40, "demo.ability.death-vampiric-branding");
    game.debug_set_ability_casts_succeed(true);
    game.items.retain(|item| {
        game.content
            .item(&item.kind_id)
            .is_some_and(|definition| definition.ability_book_id.is_some())
    });
    for (kind, ego) in [
        ("demo.item.dagger", true),
        ("demo.item.poison-needle", false),
    ] {
        game.items.retain(|item| item.id != "test.brand-target");
        give_inventory_item(&mut game, "test.brand-target", kind);
        if ego {
            game.items
                .iter_mut()
                .find(|item| item.id == "test.brand-target")
                .expect("branding target")
                .affix_ids
                .push("rfb-legacy.affix.slaying".to_owned());
        }
        let mana_before = game.resources["demo.resource.mana"].current;
        let draws_before = game.rng_draw_counter();
        let mut events = Vec::new();

        game.resolve_player_ability(
            "demo.ability.death-vampiric-branding",
            TargetSelection::Item {
                item_id: "test.brand-target".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("invalid branding target should be rejected cleanly");

        assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(
            matches!(
                events.as_slice(),
                [DomainEvent::AbilityTargetUnavailable { .. }]
            ),
            "{events:#?}"
        );
    }
}

fn formal_hobbit_high_mage(seed: u64, level: u16) -> Game {
    let mut game = Game::new_with_build_race_and_name(
        seed,
        "demo.build.high-mage-death",
        "rfb-legacy.race.hobbit",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Hobbit High-Mage should create");
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    game
}

#[test]
fn formal_hobbit_create_food_projects_and_round_trips_an_acquired_ration() {
    let locked = formal_hobbit_high_mage(87, 14)
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_CREATE_FOOD_ABILITY_ID)
        .expect("Hobbit Create Food should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(locked.minimum_level, 15);
    assert_eq!(locked.base_resource_cost, 10);
    assert_eq!(locked.failure_percent, 100);
    assert!(!locked.can_cast);

    let mut game = formal_hobbit_high_mage(87, 15);
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_CREATE_FOOD_ABILITY_ID)
        .expect("Hobbit Create Food should remain projected");
    assert!(available.can_cast);
    assert!(available.failure_percent < 100);
    game.debug_set_ability_casts_succeed(true);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 3;
    let mut replay = game.clone();
    let hp_before = game.player.hp;
    let tick_before = game.world_tick;
    let serial_before = game.next_item_instance_serial;
    for cast in [&mut game, &mut replay] {
        dispatch_next(
            cast,
            GameCommand::CastAbility {
                ability_id: RACE_CREATE_FOOD_ABILITY_ID.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
    }
    assert_eq!(game.state_hash(), replay.state_hash());
    assert_eq!(game.world_tick, tick_before + 10);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert_eq!(game.player.hp, hp_before - 7);
    assert_eq!(game.next_item_instance_serial, serial_before + 1);

    let created = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.ration-of-food"
                && item.origin_kind == Some(ItemOriginKindDto::Acquire)
        })
        .expect("Create Food should produce an acquired ration");
    let created_id = created.id.clone();
    assert_eq!(created.quantity, 1);
    assert_eq!(created.quality, rfb_protocol::ItemQualityDto::Ordinary);
    assert!(created.affix_ids.is_empty());
    assert!(created.curse.is_none());
    let ItemLocation::Ground(position) = created.location else {
        panic!("created ration should land on the ground");
    };
    assert!(game.is_walkable(position));
    assert!(rfb_distance(position, game.player.position) <= 3);

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Hobbit Create Food save should restore");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == created_id)
        .expect("created ration should survive the save round trip");
    assert_eq!(restored_item.kind_id, "demo.item.ration-of-food");
    assert_eq!(restored_item.quantity, 1);
    assert_eq!(restored_item.origin_kind, Some(ItemOriginKindDto::Acquire));
    assert_eq!(
        restored_item.quality,
        rfb_protocol::ItemQualityDto::Ordinary
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

fn create_item_ability_catalog(base_failure_percent: u8) -> Arc<rfb_content::ContentCatalog> {
    mutation_ability_catalog_with_effect(
        1,
        1,
        base_failure_percent,
        AbilityEffectDefinition::CreateItem {
            item_kind_id: "demo.item.ration-of-food".to_owned(),
            quantity: 1,
        },
    )
}

#[test]
fn create_item_ability_places_an_acquired_item_and_merges_repeated_casts() {
    let mut game = mutation_ability_game(create_item_ability_catalog(0), "demo.build.warrior");
    game.debug_set_ability_casts_succeed(true);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    let position = game.player.position;
    let serial_before = game.next_item_instance_serial;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    game.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .expect("plain item creation should resolve");

    let created = game
        .items
        .iter()
        .find(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
        .expect("a successful creation should place an acquired item");
    let created_id = created.id.clone();
    assert_eq!(created.kind_id, "demo.item.ration-of-food");
    assert_eq!(created.quantity, 1);
    assert_eq!(created.location, ItemLocation::Ground(position));
    assert_eq!(game.next_item_instance_serial, serial_before + 1);
    assert_eq!(changed, BTreeSet::from([position]));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::CreateItem {
                    item_kind_id,
                    quantity: 1,
                    position: effect_position,
                    destination_item_ids,
                    ..
                }] if item_kind_id == "demo.item.ration-of-food"
                    && *effect_position == position
                    && destination_item_ids == std::slice::from_ref(&created_id)
            )
    )));

    let serial_after_first = game.next_item_instance_serial;
    events.clear();
    changed.clear();
    game.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .expect("a repeated creation should resolve");
    let merged = game
        .items
        .iter()
        .find(|item| item.id == created_id)
        .expect("the original acquired stack should remain");
    assert_eq!(merged.quantity, 2);
    assert_eq!(game.next_item_instance_serial, serial_after_first);

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("an acquired item should survive a save round trip");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == created_id)
        .expect("the acquired item should be restored");
    assert_eq!(restored_item.quantity, 2);
    assert_eq!(restored_item.origin_kind, Some(ItemOriginKindDto::Acquire));
}

#[test]
fn create_item_ability_uses_rfb_nearby_scoring_and_failure_creates_nothing() {
    let prepare = |failure| {
        let mut game =
            mutation_ability_game(create_item_ability_catalog(failure), "demo.build.warrior");
        game.items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        game
    };

    let mut first = prepare(0);
    first.debug_set_ability_casts_succeed(true);
    let origin = first.player.position;
    give_inventory_item(&mut first, "test.drop-blocker", "demo.item.arrow");
    first
        .items
        .iter_mut()
        .find(|item| item.id == "test.drop-blocker")
        .expect("drop blocker should exist")
        .location = ItemLocation::Ground(origin);
    let mut second = first.clone();
    let cast = |game: &mut Game| {
        game.resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("nearby item creation should resolve");
        game.items
            .iter()
            .find(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
            .expect("created item should exist")
            .location
            .clone()
    };
    let first_location = cast(&mut first);
    let second_location = cast(&mut second);
    assert_eq!(first_location, second_location);
    let ItemLocation::Ground(created_position) = first_location else {
        panic!("created item should be on the ground");
    };
    assert_ne!(created_position, origin);
    assert_eq!(rfb_distance(created_position, origin), 1);

    let mut failed = prepare(95);
    failed.player.hp = 20;
    failed.rng = RfbRng::seeded(0);
    let serial_before = failed.next_item_instance_serial;
    let item_count_before = failed.items.len();
    let mut events = Vec::new();
    failed
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed item creation should resolve");
    assert!(matches!(
        events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(failed.next_item_instance_serial, serial_before);
    assert_eq!(failed.items.len(), item_count_before);
}

#[test]
fn mutation_eat_magic_and_weigh_magic_use_existing_device_and_status_state() {
    let capped_failure = active_source_mutation_game(51, "eat-magic", 100);
    let activation = capped_failure
        .content
        .mutation("rfb.mutation.eat-magic")
        .unwrap()
        .activation
        .clone()
        .unwrap();
    assert_eq!(capped_failure.innate_power_failure_percent(&activation), 11);

    let mut eater = active_source_mutation_game(53, "eat-magic", 17);
    super::super::dungeon_anti_magic::enter_context(&mut eater);
    give_inventory_item(
        &mut eater,
        "test.item.magic-food",
        "demo.item.detect-objects-staff",
    );
    let item = eater
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.magic-food")
        .unwrap();
    item.activation
        .as_mut()
        .expect("staff should have an activation")
        .device_check_difficulty = 40;
    item.charges
        .as_mut()
        .expect("staff should have charges")
        .current = 20;
    eater
        .resources
        .get_mut("demo.resource.mana")
        .unwrap()
        .current = 10;
    let mut events = Vec::new();
    // At level 17, power 47 and difficulty 40 give an internal failure odds of 5.
    eater.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(100);
            rng.bounded(5) != 0
        })
        .unwrap();
    eater
        .resolve_player_ability(
            "rfb.ability.mutation.eat-magic",
            TargetSelection::Item {
                item_id: "test.item.magic-food".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Eat Magic should resolve");
    assert_eq!(eater.resources["demo.resource.mana"].current, 29);
    assert_eq!(
        eater
            .items
            .iter()
            .find(|item| item.id == "test.item.magic-food")
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::DrainItemMagic {
                    drained: 20,
                    failed: false,
                    resource_before: 9,
                    resource_after: 29,
                    ..
                }]
            )
    )));

    let mut observer = active_source_mutation_game(59, "weigh-magic", 6);
    observer.player.statuses.push(StatusInstance {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 1,
        remaining_ticks: 20,
        source_id: Some("test.status".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    observer.recall = Some(RecallStateDto {
        dungeon_id: "demo.dungeon.warrens".to_owned(),
        floor_id: "demo.floor.warrens.1".to_owned(),
        remaining_turns: Some(9),
    });
    events.clear();
    observer
        .resolve_player_ability(
            "rfb.ability.mutation.weigh-magic",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Weigh Magic should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::ReportMagic { statuses, recall, .. }]
                    if statuses.iter().any(|status| status.kind_id == STATUS_HASTE)
                        && recall.as_ref().is_some_and(|state| state.remaining_turns == Some(9))
            )
    )));
}

fn apply_test_ground_projection(
    game: &mut Game,
    positions: &[Position],
    damage_type: DamageType,
) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_ground_item_projectile_effects(
        "test.ability.ground-items",
        positions,
        damage_type,
        true,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    events
}

#[test]
fn ground_item_elements_respect_ignore_flags_and_artifact_protection() {
    let mut game = Game::new(211);
    clear_monsters(&mut game);
    game.items.clear();
    let position = game.player.position;
    for (id, kind_id) in [
        ("test.arrow", "demo.item.arrow"),
        ("test.elven-cloak", "demo.item.elven-cloak"),
        ("test.artifact", "demo.item.pain"),
        ("test.adamantine", "demo.item.adamantine-bolt"),
        ("test.endurance", "demo.item.arrow"),
    ] {
        place_test_ground_item(&mut game, id, kind_id, position);
    }
    game.items
        .iter_mut()
        .find(|item| item.id == "test.endurance")
        .expect("Endurance ammunition")
        .affix_ids
        .push("rfb-legacy.affix.endurance".to_owned());

    apply_test_ground_projection(&mut game, &[position], DamageType::Fire);

    assert!(!game.items.iter().any(|item| item.id == "test.arrow"));
    for survivor in [
        "test.elven-cloak",
        "test.artifact",
        "test.adamantine",
        "test.endurance",
    ] {
        assert!(game.items.iter().any(|item| item.id == survivor));
    }
    apply_test_ground_projection(&mut game, &[position], DamageType::Mana);
    assert!(game.items.iter().any(|item| item.id == "test.artifact"));
    assert!(game.items.iter().any(|item| item.id == "test.endurance"));
}

#[test]
fn hell_fire_destroys_only_cursed_ground_items() {
    let mut game = Game::new(223);
    clear_monsters(&mut game);
    game.items.clear();
    let position = game.player.position;
    place_test_ground_item(&mut game, "test.clean", "demo.item.arrow", position);
    place_test_ground_item(&mut game, "test.cursed", "demo.item.arrow", position);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.cursed")
        .expect("cursed test item")
        .curse = Some(ItemCurseSeverityDto::Normal);

    apply_test_ground_projection(&mut game, &[position], DamageType::HellFire);

    assert!(game.items.iter().any(|item| item.id == "test.clean"));
    assert!(!game.items.iter().any(|item| item.id == "test.cursed"));
}

#[test]
fn ground_item_destruction_is_ordered_by_position_then_instance_id() {
    let mut game = Game::new(227);
    clear_monsters(&mut game);
    game.items.clear();
    let positions = [
        Position { x: 5, y: 2 },
        Position { x: 5, y: 1 },
        Position { x: 3, y: 1 },
    ];
    for (id, position) in [
        ("test.z", positions[1]),
        ("test.a", positions[1]),
        ("test.m", positions[2]),
        ("test.last", positions[0]),
    ] {
        place_test_ground_item(&mut game, id, "demo.item.arrow", position);
    }

    let events = apply_test_ground_projection(
        &mut game,
        &[positions[0], positions[1], positions[2]],
        DamageType::Fire,
    );
    let destroyed = events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::GroundItemDestroyedByAbility { item_id, .. } => Some(item_id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(destroyed, ["test.m", "test.a", "test.z", "test.last"]);
}

#[test]
fn shattered_potion_runs_its_area_program_after_removal() {
    let mut game = Game::new(229);
    clear_monsters(&mut game);
    game.items.clear();
    let position = game.player.position;
    place_test_ground_item(&mut game, "test.venom", "demo.item.venom-draught", position);
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();

    let events = apply_test_ground_projection(&mut game, &[position], DamageType::Cold);

    assert!(!game.items.iter().any(|item| item.id == "test.venom"));
    assert_eq!(game.player.hp, hp_before - 3);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityHit { ability_id, .. }
            if ability_id == "demo.item.venom-draught"
    )));
}

#[test]
fn shattered_potion_healing_uses_area_falloff() {
    let mut game = Game::new(233);
    clear_monsters(&mut game);
    let maximum = game.effective_player_max_hp();
    game.player.hp = 1;
    let center = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let shatter = ItemShatterEffectDefinition {
        radius: 2,
        effect: ItemUseEffectDefinition::Heal { amount: 100 },
    };

    game.resolve_ground_item_shatter_effect(
        "test.item.healing-potion",
        center,
        &shatter,
        true,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );

    assert_eq!(game.player.hp, (1 + 50).min(maximum));
}

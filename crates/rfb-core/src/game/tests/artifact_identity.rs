// SPDX-License-Identifier: MPL-2.0
//! Random artifact identity and its consumers, including natural generation.
use super::support::{dispatch_next, give_inventory_item};
use super::*;
use crate::game::inventory::{
    CurseEquippedItemRequest, EquippedItemCurseTarget, ItemEnchantmentRequest,
    ItemIdentificationRequest, RemoveEquippedCursesRequest,
};
use rfb_protocol::{ItemCurseEffectDto, MeleeDamageDiceDto};

const ID: &str = "test.random-artifact";
// Exact source literal from master spells3.c; not a new translated content name.
const NAME: &str = "(永恒蘑菇)";

fn game_with_artifact(kind: &str) -> Game {
    let mut game = Game::new_with_build(85, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    give_inventory_item(&mut game, ID, kind);
    let profile = &game
        .content
        .affix("rfb-legacy.affix.illumination-light")
        .unwrap()
        .device_generation
        .as_ref()
        .unwrap()
        .activations[0];
    let activation = ItemActivationDto {
        profile_id: profile.id.clone(),
        name_key: profile.name_key.clone(),
        power: profile.device_check_difficulty as u16,
        cost: profile.charges.cost,
        device_check_difficulty: profile.device_check_difficulty,
        target_spec: crate::game::ability_projection::target_spec_dto(&profile.target),
    };
    let item = game.items.last_mut().unwrap();
    item.artifact_name = Some(NAME.to_owned());
    item.quality = ItemQualityDto::Exceptional;
    item.intrinsic_weight_tenths_pound = Some(77);
    if kind == "demo.item.dagger" {
        item.intrinsic_melee_damage_dice = Some(MeleeDamageDiceDto { dice: 3, sides: 7 });
    }
    item.intrinsic_weapon_traits.insert(WeaponTraitDto::Blessed);
    item.intrinsic_properties.modifiers.strength = 2;
    item.intrinsic_properties.rfb_pval = Some(rfb_content::RfbPvalDefinition {
        value: 2,
        flags: BTreeSet::from([rfb_content::RfbPvalFlagDefinition::Strength]),
    });
    item.intrinsic_properties.resistances.insert(
        ActorDamageType::Fire,
        rfb_content::ActorResistanceLevel::Resistant,
    );
    item.intrinsic_properties
        .status_immunities
        .push(STATUS_PARALYSIS.to_owned());
    item.intrinsic_properties.brands.insert(WeaponBrand::Cold);
    item.enchantments = ItemEnchantmentsDto {
        to_hit: 7,
        to_damage: 9,
        to_armor: 0,
    };
    item.activation = Some(activation);
    item.charges = Some(ItemChargesDto {
        current: 1,
        maximum: 1,
    });
    game
}

fn round_trip(game: &Game) -> Game {
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.to_save(), game.to_save());
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    restored
}

#[test]
fn artifact_identity_keeps_base_properties_value_knowledge_and_equipment() {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        kind_id: String,
        object: crate::game::item_value::ValueObject,
        expected: i32,
    }
    #[derive(serde::Deserialize)]
    struct Reference {
        name: String,
        cases: Vec<Case>,
    }
    let reference: Reference =
        serde_json::from_str(include_str!("artifact-identity-reference.json")).unwrap();
    assert_eq!(reference.name, NAME);
    for case in reference.cases {
        let mut game = game_with_artifact(&case.kind_id);
        let generated = game.generated_artifact_ids.clone();
        let item = &game.items[0];
        assert!(item.is_artifact(&game.content));
        assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
        assert_eq!(game.item_instance_weight(item), 77);
        let value = crate::game::item_value::instance::value_object(&game.content, item).unwrap();
        let mut expected_object = case.object;
        // Source light kinds retain unused 1d1 fields; the rewrite has no light melee profile.
        if value.tval == 39 {
            expected_object.dd = 0;
            expected_object.ds = 0;
            expected_object.base_dd = 0;
            expected_object.base_ds = 0;
        }
        assert_eq!(value, expected_object);
        assert_eq!(
            crate::game::item_value::object_value(value),
            Some(case.expected)
        );
        assert_eq!(game.snapshot().inventory[0].artifact_name, None);
        assert_eq!(game.snapshot().inventory[0].modifiers.strength, 0);
        game.appraise_inventory_item(ID).unwrap();
        assert_eq!(game.snapshot().inventory[0].artifact_name, None);
        let mut game = round_trip(&game);
        game.identify_item_instance(ID, ItemIdentificationRequest::new(true));
        let known = &game.snapshot().inventory[0];
        assert_eq!(known.artifact_name.as_deref(), Some(NAME));
        assert_eq!(known.modifiers.strength, 2);
        assert!(known.known_properties.is_empty());
        assert!(game.equip_inventory_item(ID, None).is_some());
        assert_eq!(
            game.snapshot().equipment[0].artifact_name.as_deref(),
            Some(NAME)
        );
        assert_eq!(game.carried_weight_tenths_pound(), 77);
        assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
        if case.kind_id == "demo.item.dagger" {
            let melee = game.item_melee_profile(&game.items[0]).unwrap();
            assert_eq!((melee.damage.dice, melee.damage.sides), (3, 7));
            assert!(game.item_has_weapon_trait(&game.items[0], WeaponTraitDto::Blessed));
            let attack = game.player_melee_profile(&game.player_derived_stats());
            assert_eq!(attack.damage_dice, 3);
            assert_eq!(attack.damage_sides, 7);
            assert_eq!(attack.critical_weight_tenths_pound, Some(77));
        }
        game.register_generated_artifact(&case.kind_id);
        assert_eq!(game.generated_artifact_ids, generated);
        round_trip(&game);
    }
}

#[test]
fn artifact_identity_activation_and_recovery_need_no_ego_identity() {
    let mut game = game_with_artifact("demo.item.dagger");
    game.identify_item_instance(ID, ItemIdentificationRequest::new(true));
    assert!(!game.snapshot().inventory[0].usable);
    game.equip_inventory_item(ID, None).unwrap();
    assert!(game.snapshot().equipment[0].usable);
    let mut game = round_trip(&game);
    let mut events = Vec::new();
    for _ in 0..100 {
        game.use_inventory_item(
            ID,
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.items[0].charges.unwrap().current == 0 {
            break;
        }
    }
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityAreaDamage { .. }))
    );
    let mut game = round_trip(&game);
    for _ in 0..100 {
        game.world_tick += 1;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(game.items[0].charges.unwrap().current, 1);
    assert!(game.items[0].affix_ids.is_empty());
    round_trip(&game);
}

#[test]
fn artifact_identity_prevents_stacking_craft_and_destruction() {
    let mut game = game_with_artifact("demo.item.dagger");
    let mut duplicate = game.items[0].clone();
    duplicate.id = "test.second-artifact".to_owned();
    duplicate.location = ItemLocation::Ground(game.player.position);
    assert!(!crate::game::inventory::item_instances_stack_compatible(
        &game.content,
        &game.items[0],
        &duplicate
    ));
    game.items.push(duplicate);
    game.pick_up_item_at_player(Some("test.second-artifact"))
        .unwrap();
    assert_eq!(game.items.len(), 2);
    assert!(game.items.iter().all(|item| item.quantity == 1));
    assert_eq!(
        game.can_destroy_item(&game.items[0]),
        Err(crate::game::inventory::DestroyItemFailure::Artifact)
    );
    give_inventory_item(&mut game, "test.craft", "demo.item.crafting-scroll");
    let items = game.items.clone();
    let rng = game.rng.clone();
    let ticks = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.craft".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: ID.to_owned(),
            }),
        },
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
    assert_eq!(game.items, items);
    assert_eq!(game.rng, rng);
    assert_eq!(game.world_tick, ticks);
    // Artifact protection applies even without IGNORE_* flags on the instance.
    game.items.retain(|item| item.id == ID);
    let mut ordinary = game.items[0].clone();
    ordinary.id = "test.ordinary".to_owned();
    ordinary.artifact_name = None;
    game.items.push(ordinary);
    for item in &mut game.items {
        item.location = ItemLocation::Ground(game.player.position);
    }
    game.resolve_ground_item_projectile_effects(
        "test.mana",
        &[game.player.position],
        DamageType::Mana,
        false,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(game.items.len(), 1);
    assert_eq!(game.items[0].id, ID);
    game.items[0].location = ItemLocation::Inventory;
    round_trip(&game);
}

#[test]
fn artifact_identity_uses_artifact_enchantment_and_curse_rules() {
    let mut game = game_with_artifact("demo.item.dagger");
    assert!(game.equip_inventory_item(ID, None).is_some());
    let before = game.items[0].enchantments.to_hit;
    let mut expected = game.clone();
    let expected_result =
        expected.resolve_item_enchantment_component(before, 10, 1, false, true, false);
    let result = game.enchant_item_instance(ID, ItemEnchantmentRequest::new(10, 0, 0));
    assert_eq!(result.to_hit, expected_result);
    assert_eq!(game.rng, expected.rng);
    let mut saw_resist = false;
    let mut saw_blast = false;
    for seed in 0..32 {
        let mut trial = game.clone();
        trial.rng = RfbRng::seeded(seed);
        let mut source_rng = trial.rng.clone();
        let resisted = source_rng.bounded(100) < 50;
        let activation = trial.items[0].activation.clone();
        let outcome = trial.curse_equipped_item(
            CurseEquippedItemRequest::new(EquippedItemCurseTarget::Weapon).blasting(),
        );
        assert_eq!(outcome.resisted, resisted);
        assert_eq!(trial.items[0].artifact_name.as_deref(), Some(NAME));
        assert_eq!(trial.items[0].activation, activation);
        assert_eq!(trial.item_instance_weight(&trial.items[0]), 77);
        if resisted {
            saw_resist = true;
        } else {
            saw_blast = true;
            let dice = trial.item_melee_profile(&trial.items[0]).unwrap().damage;
            assert_eq!((dice.dice, dice.sides), (0, 0));
            assert!(trial.items[0].intrinsic_weapon_traits.is_empty());
        }
        round_trip(&trial);
    }
    assert!(saw_resist && saw_blast);
    game.items[0].curse = Some(ItemCurseSeverityDto::Heavy);
    game.items[0]
        .intrinsic_curse_effects
        .insert(ItemCurseEffectDto::LowMelee);
    assert!(game.player_has_equipped_curse_effect(ItemCurseEffectDto::LowMelee));
    let mut game = round_trip(&game);
    game.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
    assert!(!game.player_has_equipped_curse_effect(ItemCurseEffectDto::LowMelee));
    assert!(game.items[0].intrinsic_curse_effects.is_empty());
    assert_eq!(game.items[0].artifact_name.as_deref(), Some(NAME));
}

#[test]
fn artifact_identity_mundanity_removes_intrinsic_identity_properties_and_activation() {
    let mut game = game_with_artifact("demo.item.dagger");
    give_inventory_item(&mut game, "test.mundanity", "demo.item.mundanity-scroll");
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.mundanity".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: ID.to_owned(),
            }),
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-mundanity")
    );
    let item = game.items.iter().find(|item| item.id == ID).unwrap();
    assert_eq!(item.kind_id, "demo.item.dagger");
    assert!(!item.is_artifact(&game.content));
    assert!(item.intrinsic_weapon_traits.is_empty());
    assert!(item.activation.is_none() && item.charges.is_none());
    assert_eq!(item.intrinsic_properties, Default::default());
    assert_eq!(game.item_instance_weight(item), 12);
    let dice = game.item_melee_profile(item).unwrap().damage;
    assert_eq!((dice.dice, dice.sides), (1, 5));
    round_trip(&game);
}

#[test]
fn artifact_identity_invalid_save_state_is_rejected() {
    let game = game_with_artifact("demo.item.dagger");
    round_trip(&game);
    for invalid in 0..9 {
        let mut save = game.to_save();
        let item = &mut save.inventory[0];
        match invalid {
            0 => item.artifact_name = Some(" ".to_owned()),
            1 => item.artifact_name = Some("bad\nname".to_owned()),
            2 => item.artifact_name = Some("x".repeat(1024)),
            3 => item.quantity = 2,
            4 => item.kind_id = "demo.item.arkenstone-of-thrain".to_owned(),
            5 => item.intrinsic_melee_damage_dice = Some(MeleeDamageDiceDto { dice: 0, sides: 7 }),
            6 => item.intrinsic_weight_tenths_pound = Some(10_001),
            7 => item.intrinsic_weapon_traits = vec![WeaponTraitDto::Blessed; 2],
            8 => item.intrinsic_curse_effects = vec![ItemCurseEffectDto::LowMelee; 2],
            _ => unreachable!(),
        }
        assert!(Game::from_save(save).is_err(), "case {invalid}");
    }
}

#[test]
fn artifact_identity_mogaminator_matches_known_name_and_artifact_but_never_nameless() {
    let mut game = game_with_artifact("demo.item.dagger");
    game.interface_locale = rfb_protocol::LocaleDto::EnUs;
    let configure = |game: &mut Game, source: &str| {
        assert!(
            game.configure_mogaminator(
                true,
                false,
                rfb_protocol::AutoGetModeDto::Off,
                rfb_protocol::LocaleDto::EnUs,
                source.to_owned()
            )
            .is_empty()
        );
    };
    configure(&mut game, "artifact items");
    assert!(game.mogaminator_dto(Vec::new()).matches.is_empty());
    game.appraise_inventory_item(ID).unwrap();
    assert_eq!(game.mogaminator_dto(Vec::new()).matches.len(), 1);
    configure(&mut game, "nameless items");
    assert!(game.mogaminator_dto(Vec::new()).matches.is_empty());
    configure(&mut game, "items:永恒蘑菇");
    assert!(game.mogaminator_dto(Vec::new()).matches.is_empty());
    game.identify_item_instance(ID, ItemIdentificationRequest::new(true));
    assert_eq!(game.mogaminator_dto(Vec::new()).matches.len(), 1);
    configure(&mut game, "!artifact items");
    let items = game.items.clone();
    let outcomes = game
        .apply_mogaminator_to_items(vec![ID.to_owned()], false, true)
        .unwrap();
    assert!(
        matches!(outcomes.as_slice(), [crate::game::mogaminator::MogaminatorItemResolution::DestroyUnavailable { reason, .. }] if reason == "artifact")
    );
    assert_eq!(game.items, items);
    round_trip(&game);
}

#[test]
fn artifact_identity_home_and_shop_keep_separate_instances_and_save_all_properties() {
    let mut game = game_with_artifact("demo.item.dagger");
    game.identify_item_instance(ID, ItemIdentificationRequest::new(true));
    let second_id = "test.second-artifact";
    let mut duplicate = game.items[0].clone();
    duplicate.id = second_id.to_owned();
    game.items.push(duplicate);
    game.identify_item_instance(second_id, ItemIdentificationRequest::new(true));
    let home = game
        .content
        .town_facility("demo.town-facility.outpost-home")
        .unwrap()
        .clone();
    game.player.position = game
        .town_local_to_active_position(&home.town_id, position_from_content(home.entrance_position))
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    for id in [ID, second_id] {
        game.deposit_at_home(&home.id, id, 1).unwrap();
    }
    let home_dto = game
        .current_home_dtos()
        .into_iter()
        .find(|dto| dto.id == home.id)
        .unwrap();
    assert_eq!(home_dto.stored_items.len(), 2);
    assert!(home_dto.stored_items.iter().all(|item| item.quantity == 1
        && item.artifact_name.as_deref() == Some(NAME)
        && item.weight_tenths_pound == 77));
    game.reveal_current_visibility();
    let mut game = round_trip(&game);
    game.withdraw_from_home(&home.id, ID, 1).unwrap();
    assert_eq!(game.items.len(), 1);
    assert_eq!(game.items[0].artifact_name.as_deref(), Some(NAME));
    let shop = game
        .content
        .shop("demo.shop.outpost-weaponsmith")
        .unwrap()
        .clone();
    game.player.position = game
        .town_local_to_active_position(&shop.town_id, position_from_content(shop.entrance_position))
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    game.sell_to_shop(&shop.id, ID, 1).unwrap();
    let stock = game
        .current_shop_dtos()
        .into_iter()
        .find(|dto| dto.id == shop.id)
        .unwrap()
        .stock;
    let sold = stock
        .iter()
        .find(|item| item.artifact_name.as_deref() == Some(NAME))
        .unwrap();
    assert_eq!(sold.weight_tenths_pound, 77);
    assert!(sold.unit_price > 1_000);
    let sold_id = sold.id.clone();
    game.reveal_current_visibility();
    let mut game = round_trip(&game);
    game.gold = 100_000;
    game.buy_from_shop(&shop.id, &sold_id, 1).unwrap();
    assert_eq!(game.items[0].artifact_name.as_deref(), Some(NAME));
    assert_eq!(
        game.items[0].intrinsic_melee_damage_dice,
        Some(MeleeDamageDiceDto { dice: 3, sides: 7 })
    );
    round_trip(&game);
}

#[test]
fn artifact_identity_museum_accepts_random_but_rejects_fixed_identity() {
    let mut game = game_with_artifact("demo.item.dagger");
    game.identify_item_instance(ID, ItemIdentificationRequest::new(true));
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 17, y: 29 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let home = game
        .content
        .town_facility("demo.town-facility.thalos-museum")
        .unwrap()
        .clone();
    game.player.position = game
        .town_local_to_active_position(&home.town_id, position_from_content(home.entrance_position))
        .unwrap();
    game.mark_shop_visited_at_player().unwrap();
    give_inventory_item(&mut game, "test.fixed", "demo.item.arkenstone-of-thrain");
    game.register_generated_artifact("demo.item.arkenstone-of-thrain");
    let generated = game.generated_artifact_ids.clone();
    assert_eq!(
        game.deposit_at_home(&home.id, "test.fixed", 1),
        Err("artifact-rejected")
    );
    game.deposit_at_home(&home.id, ID, 1).unwrap();
    let stored = game
        .current_home_dtos()
        .into_iter()
        .find(|dto| dto.id == home.id)
        .unwrap();
    assert_eq!(stored.stored_items.len(), 1);
    assert_eq!(stored.stored_items[0].artifact_name.as_deref(), Some(NAME));
    assert_eq!(game.generated_artifact_ids, generated);
    game.reveal_current_visibility();
    round_trip(&game);
}

#[test]
fn artifact_identity_intrinsic_curses_generate_and_grow_without_ego_state() {
    let mut game = game_with_artifact("demo.item.dagger");
    ego::curses::curse_object(&game.content, &mut game.rng, &mut game.items[0]);
    assert!(game.items[0].curse.is_some());
    assert!(game.items[0].affix_ids.is_empty() && game.items[0].rolled_affixes.is_empty());
    round_trip(&game);

    let mut game = game_with_artifact("demo.item.dagger");
    game.equip_inventory_item(ID, None).unwrap();
    game.items[0].curse = Some(ItemCurseSeverityDto::Heavy);
    game.items[0]
        .intrinsic_curse_effects
        .insert(ItemCurseEffectDto::AddHeavyCurse);
    // Force the real 1/2000 periodic branch without running thousands of world turns.
    let seed = (0..100_000)
        .find(|&seed| RfbRng::seeded(seed).bounded(2000) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert!(game.items[0].intrinsic_curse_effects.len() > 1);
    assert!(game.items[0].affix_ids.is_empty() && game.items[0].rolled_affixes.is_empty());
    round_trip(&game);
}

#[test]
fn artifact_identity_inventory_damage_and_ground_save_keep_the_instance() {
    let mut game = game_with_artifact("demo.item.dagger");
    let original = game.items[0].clone();
    let mut ordinary = original.clone();
    ordinary.id = "test.ordinary".to_owned();
    ordinary.artifact_name = None;
    game.items.push(ordinary);
    for _ in 0..1000 {
        game.damage_player_inventory("test.acid", DamageType::Acid, false, 100, &mut Vec::new());
        if game.items.len() == 1 {
            break;
        }
    }
    assert_eq!(game.items, [original]);
    game.items[0].location = ItemLocation::Ground(game.player.position);
    game.identify_item_instance(ID, ItemIdentificationRequest::new(true));
    assert_eq!(
        game.snapshot().items[0].artifact_name.as_deref(),
        Some(NAME)
    );
    round_trip(&game);
}

#[test]
fn artifact_identity_natural_generation_preserves_existing_artifacts() {
    let game = game_with_artifact("demo.item.dagger");
    let mut random_count = 0;
    for seed in 0..16 {
        for mode in [
            ItemGenerationMode::Ordinary,
            ItemGenerationMode::Good,
            ItemGenerationMode::Great,
            ItemGenerationMode::Artifact {
                no_fixed_artifact: false,
            },
        ] {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(seed);
            let context = LootContext {
                table_id: "demo.loot-table.base-items".to_owned(),
                floor_id: trial.current_floor_id.clone(),
                depth: 80,
                source: LootSource::MonsterDeath {
                    actor_id: "test.source".to_owned(),
                },
            };
            let generated = trial
                .generate_loot_instances_internal(
                    &context,
                    ItemLocation::Inventory,
                    false,
                    Some(1),
                    mode,
                )
                .unwrap();
            for item in &generated {
                if let Some(name) = &item.artifact_name {
                    random_count += 1;
                    assert!(trial.random_artifact_names.contains(name));
                    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
                    assert!(!trial.generated_artifact_ids.contains(&item.kind_id));
                    if trial
                        .content
                        .item(&item.kind_id)
                        .unwrap()
                        .device_generation
                        .is_some()
                    {
                        assert!(item.activation.is_some(), "base activation on {seed}");
                        assert!(item.charges.is_some());
                    }
                }
            }
            assert_eq!(trial.items[0], game.items[0]);
            trial.items.extend(generated);
            round_trip(&trial);
        }
    }
    assert!(random_count > 0);
}

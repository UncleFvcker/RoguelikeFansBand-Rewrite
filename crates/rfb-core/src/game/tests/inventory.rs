// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_protocol::ItemFeelingDto;

#[test]
fn i6_pickup_blends_metadata_through_partial_stacks_split_destroy_and_save() {
    let mut game = Game::new(606);
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    give_inventory_item(&mut game, "test.carried", "demo.item.arrow");
    give_inventory_item(&mut game, "test.incoming", "demo.item.arrow");
    let maximum = game.content.item("demo.item.arrow").unwrap().max_stack;
    game.items[0].quantity = maximum - 1;
    game.items[1].quantity = 5;
    game.items[1].location = ItemLocation::Ground(game.player.position);
    game.items[1].origin_kind = Some(ItemOriginKindDto::PlayerMade);
    game.items[1].discount_percent = 99;
    game.items[1].inscription = Some("keep".into());
    assert!(matches!(
        game.pick_up_item_at_player(Some("test.incoming")).unwrap(),
        PickUpOutcome::Picked { quantity: 5, .. }
    ));
    let merged = &game.items[0];
    assert_eq!(merged.id, "test.carried");
    assert_eq!(merged.quantity, maximum);
    assert_eq!(merged.origin_kind, Some(ItemOriginKindDto::Mixed));
    assert_eq!(merged.discount_percent, 99);
    assert_eq!(merged.inscription.as_deref(), Some("keep"));
    assert_eq!(game.items[1].id, "test.incoming");
    assert_eq!(game.items[1].quantity, 4);
    assert_eq!(
        game.items[1].origin_kind,
        Some(ItemOriginKindDto::PlayerMade)
    );

    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for current in [&mut game, &mut restored] {
        current
            .drop_inventory_quantity("test.carried", 3)
            .unwrap()
            .unwrap();
        let split = current
            .items
            .iter()
            .find(|item| matches!(item.location, ItemLocation::Ground(_)))
            .unwrap();
        let split_id = split.id.clone();
        assert_eq!(split.quantity, 3);
        assert_eq!(split.origin_kind, Some(ItemOriginKindDto::Mixed));
        assert_eq!(split.discount_percent, 99);
        current.pick_up_item_at_player(Some(&split_id)).unwrap();
        assert!(!current.items.iter().any(|item| item.id == split_id));
        current.destroy_item("test.carried", 1).unwrap();
    }
    assert_eq!(game.items[0].quantity, maximum - 1);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    assert!(Game::from_save(game.to_save()).is_ok());
    game.items[0].discount_percent = 50;
    assert!(Game::from_save(game.to_save()).is_err());
}

#[test]
fn i6_conflicting_inscriptions_and_corpse_identities_do_not_combine() {
    let mut game = Game::new(606);
    game.items.clear();
    game.item_property_knowledge.clear();
    give_inventory_item(&mut game, "test.carried", "demo.item.arrow");
    give_inventory_item(&mut game, "test.incoming", "demo.item.arrow");
    game.items[0].inscription = Some("fire".into());
    game.items[1].inscription = Some("save".into());
    game.items[1].origin_kind = Some(ItemOriginKindDto::Acquire);
    game.items[1].location = ItemLocation::Ground(game.player.position);
    game.pick_up_item_at_player(Some("test.incoming")).unwrap();
    assert_eq!(game.items.len(), 2);
    assert_eq!(game.items[0].quantity, 1);
    assert_eq!(game.items[0].origin_kind, None);
    assert_eq!(game.items[1].inscription.as_deref(), Some("save"));

    let mut first = game.items[0].clone();
    first.kind_id = "demo.item.corpse-remains".into();
    first.origin_actor_kind_id = Some("demo.actor.goblin".into());
    first.inscription = None;
    let mut second = first.clone();
    second.id = "test.corpse".into();
    second.origin_actor_kind_id = Some("demo.actor.sheep".into());
    assert!(!super::super::inventory::item_instances_stack_compatible(
        &game.content,
        &first,
        &second
    ));
}

#[test]
fn i6_generated_piles_merge_metadata_without_allocating_another_id() {
    let mut game = Game::new(606);
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.gold_piles.clear();
    give_inventory_item(&mut game, "test.ground", "demo.item.arrow");
    game.items[0].quantity = 2;
    game.items[0].location = ItemLocation::Ground(game.player.position);
    game.items[0].origin_kind = Some(ItemOriginKindDto::PlayerMade);
    game.items[0].discount_percent = 99;
    game.items[0].inscription = Some("keep".into());
    let mut draft = super::super::loot::GeneratedItemDraft::from(game.items[0].clone());
    draft.origin_kind = None;
    draft.quantity = 3;
    let serial = game.next_item_instance_serial;
    let (position, ids) = game
        .drop_generated_item_near(draft.clone(), game.player.position)
        .unwrap()
        .unwrap();
    assert_eq!(position, game.player.position);
    assert_eq!(ids, ["test.ground"]);
    assert_eq!(game.next_item_instance_serial, serial);
    assert_eq!(game.items.len(), 1);
    assert_eq!(game.items[0].quantity, 5);
    assert_eq!(game.items[0].origin_kind, Some(ItemOriginKindDto::Mixed));
    assert_eq!(game.items[0].discount_percent, 99);
    assert_eq!(game.items[0].inscription.as_deref(), Some("keep"));
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for current in [&mut game, &mut restored] {
        current
            .drop_generated_item_near(draft.clone(), current.player.position)
            .unwrap();
    }
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

fn tomte_sensing_game(level: u16) -> Game {
    let mut game = Game::new_with_build_race_and_name(
        424,
        "demo.build.warrior",
        "rfb-legacy.race.high-elf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10_000, "test.tomte-sensing").status;
    form.granted_race_id = Some("rfb-legacy.race.tomte".to_owned());
    game.player.statuses.push(form);
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    game.refresh_player_ability_state();
    game.player.hp = game.effective_player_max_hp();
    game
}

#[test]
fn tomte_sensing_classifies_floor_items_without_identifying_their_properties() {
    let mut game = tomte_sensing_game(39);
    let artifact = game
        .content
        .item_definitions()
        .find(|item| {
            item.artifact_generation.is_some() && item.equipment_slot.as_deref() == Some("weapon")
        })
        .unwrap()
        .id
        .clone();
    let cases = [
        (
            "ordinary",
            "demo.item.dagger",
            false,
            false,
            0,
            Some(ItemFeelingDto::Average),
        ),
        (
            "good",
            "demo.item.dagger",
            false,
            false,
            2,
            Some(ItemFeelingDto::Good),
        ),
        (
            "ego",
            "demo.item.dagger",
            true,
            false,
            3,
            Some(ItemFeelingDto::Excellent),
        ),
        (
            "bad-ego",
            "demo.item.dagger",
            true,
            true,
            0,
            Some(ItemFeelingDto::Awful),
        ),
        (
            "artifact",
            artifact.as_str(),
            false,
            false,
            0,
            Some(ItemFeelingDto::Special),
        ),
        (
            "bad-artifact",
            artifact.as_str(),
            false,
            true,
            0,
            Some(ItemFeelingDto::Terrible),
        ),
        (
            "random-artifact",
            "demo.item.dagger",
            false,
            false,
            0,
            Some(ItemFeelingDto::Special),
        ),
        (
            "bad-random-artifact",
            "demo.item.dagger",
            false,
            true,
            0,
            Some(ItemFeelingDto::Terrible),
        ),
        (
            "cursed",
            "demo.item.dagger",
            false,
            true,
            0,
            Some(ItemFeelingDto::Bad),
        ),
        (
            "device",
            "demo.item.magic-missile-wand",
            false,
            false,
            0,
            Some(ItemFeelingDto::Average),
        ),
        ("potion", "demo.item.healing-potion", false, false, 0, None),
        ("food", "demo.item.ration-of-food", false, false, 0, None),
        ("capture", "demo.item.capture-ball", false, false, 0, None),
    ];
    for (id, kind, ego, cursed, bonus, _) in cases {
        give_inventory_item(&mut game, id, kind);
        let item = game.items.last_mut().unwrap();
        item.location = ItemLocation::Ground(game.player.position);
        item.enchantments.to_hit = bonus;
        item.intrinsic_properties.modifiers.intelligence = 4;
        if id.ends_with("random-artifact") {
            item.artifact_name = Some("(永恒蘑菇)".to_owned());
        }
        if ego {
            item.quality = ItemQualityDto::Fine;
            item.affix_ids.push("demo.affix.frost-hunter".to_owned());
        }
        if cursed {
            item.curse = Some(ItemCurseSeverityDto::Heavy);
        }
    }
    give_inventory_item(&mut game, "distant", "demo.item.dagger");
    game.items.last_mut().unwrap().location =
        ItemLocation::Ground(game.position_in_direction(Direction::East));
    let draws = game.rng_draw_counter();
    let mut world_map = game.clone();
    world_map.map_scale = MapScaleDto::World;
    world_map.apply_player_floor_item_knowledge();
    assert!(!world_map.item_property_knowledge.contains_key("ego"));
    game.apply_player_floor_item_knowledge();
    assert_eq!(game.rng_draw_counter(), draws);
    for (id, _, _, _, _, feeling) in cases {
        let item = game.items.iter().find(|item| item.id == id).unwrap();
        assert_eq!(game.item_feeling(item), feeling, "{id}");
        assert_eq!(
            game.item_identification(item),
            ItemIdentificationDto::Unexamined,
            "{id}"
        );
        assert!(game.known_item_properties(item).is_empty(), "{id}");
        assert_eq!(game.visible_item_modifiers(item).intelligence, 0, "{id}");
        assert_eq!(
            game.visible_item_enchantments(item),
            ItemEnchantmentsDto::default(),
            "{id}"
        );
        assert_eq!(game.visible_item_curse(item), None, "{id}");
        assert_eq!(game.visible_item_quality(item), None, "{id}");
        assert_eq!(game.visible_artifact_name(item), None, "{id}");
    }
    assert_eq!(game.item_feeling(game.items.last().unwrap()), None);
    assert_eq!(
        game.item_knowledge_dto("demo.item.magic-missile-wand"),
        ItemKnowledgeDto::Unknown
    );
    let hash = game.state_hash();
    game.apply_player_floor_item_knowledge();
    assert_eq!(game.state_hash(), hash, "sensing is idempotent");
    let mut unsensed = game.clone();
    unsensed
        .item_property_knowledge
        .get_mut("ego")
        .unwrap()
        .feeling = None;
    assert_ne!(
        unsensed.state_hash(),
        hash,
        "feelings participate in the state hash"
    );

    let mut kind = game.content.item("demo.item.dagger").unwrap().clone();
    for (tval, senses) in [
        (8, true),
        (50, true),
        (55, true),
        (66, true),
        (7, false),
        (10, false),
        (70, false),
        (75, false),
        (90, false),
    ] {
        kind.rfb_base_kind.as_mut().unwrap().tval = tval;
        assert_eq!(
            item_knowledge::item_can_be_sensed(&kind),
            senses,
            "tval {tval}"
        );
    }
}

#[test]
fn tomte_level_forty_and_headgear_gate_only_racial_identification() {
    let mut game = tomte_sensing_game(39);
    give_inventory_item(&mut game, "helmet", "demo.item.iron-helm");
    assert!(game.equip_inventory_item("helmet", Some("head")).is_some());
    give_inventory_item(&mut game, "blade", "demo.item.dagger");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.item_property_knowledge["blade"].feeling.is_none());
    dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: "head".to_owned(),
        },
    );
    assert_eq!(
        game.item_property_knowledge["blade"].feeling,
        Some(ItemFeelingDto::Average)
    );

    give_inventory_item(&mut game, "potion", "demo.item.healing-potion");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.item_knowledge_dto("demo.item.healing-potion"),
        ItemKnowledgeDto::Unknown
    );
    let gain = game.experience_required_for_level(40) - game.progress.experience;
    game.apply_player_experience(gain, &mut Vec::new());
    assert_eq!(game.progress.level, 40);
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.item_property_knowledge["potion"].appraised);
    assert_eq!(
        game.item_knowledge_dto("demo.item.healing-potion"),
        ItemKnowledgeDto::Aware
    );
    assert!(
        !game.item_property_knowledge["potion"].identified,
        "automatic ID uses normal identification"
    );

    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "helmet".to_owned(),
            slot_id: Some("head".to_owned()),
        },
    );
    give_inventory_item(&mut game, "new-blade", "demo.item.dagger");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(!game.item_property_knowledge["new-blade"].appraised);
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.draconian-lore".to_owned());
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.item_property_knowledge["new-blade"].appraised);
    game.progress
        .active_mutation_ids
        .remove("rfb.mutation.draconian-lore");
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert!(!game.player_has_tomte_item_sensing());
    assert!(!game.player_auto_identifies_items());
    assert_eq!(
        game.item_property_knowledge["blade"].feeling,
        Some(ItemFeelingDto::Average)
    );
}

#[test]
fn tomte_sensing_precedes_mogaminator_pickup_and_preserves_stack_knowledge() {
    let mut game = tomte_sensing_game(39);
    game.mogaminator.enabled = true;
    game.mogaminator.en_us_source = "ego items#keep".to_owned();
    game.mogaminator.zh_cn_source = "ego items#keep".to_owned();
    give_inventory_item(&mut game, "ego-arrows", "demo.item.arrow");
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.floor");
    let arrows = game.items.last_mut().unwrap();
    arrows.quantity = 4;
    arrows.quality = ItemQualityDto::Fine;
    arrows.affix_ids.push("demo.affix.frost-hunter".to_owned());
    arrows.location = ItemLocation::Ground(target);
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let arrows = game
        .items
        .iter()
        .find(|item| item.id == "ego-arrows")
        .unwrap();
    assert_eq!(arrows.location, ItemLocation::Inventory);
    assert_eq!(arrows.inscription.as_deref(), Some("keep"));
    assert_eq!(game.item_feeling(arrows), Some(ItemFeelingDto::Excellent));
    assert!(game.known_item_properties(arrows).is_empty());
    game.mogaminator.enabled = false;
    game.drop_inventory_quantity("ego-arrows", 2)
        .unwrap()
        .unwrap();
    let split = game
        .items
        .iter()
        .find(|item| matches!(item.location, ItemLocation::Ground(_)))
        .unwrap()
        .id
        .clone();
    assert_eq!(
        game.item_property_knowledge[&split],
        game.item_property_knowledge["ego-arrows"]
    );
    let save = game.to_save();
    let mut restored = Game::from_save(save.clone()).expect("sensed split stacks should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot(), game.snapshot());
    game.pick_up_item_at_player(Some(&split)).unwrap();
    restored.pick_up_item_at_player(Some(&split)).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "ego-arrows")
            .unwrap()
            .quantity,
        4
    );

    let mut different = inventory::ItemPropertyKnowledgeState::default();
    assert!(!crate::game::inventory::item_properties_match(
        Some(&different),
        game.item_property_knowledge.get("ego-arrows")
    ));
    different.feeling = Some(ItemFeelingDto::Excellent);
    assert!(crate::game::inventory::item_properties_match(
        Some(&different),
        game.item_property_knowledge.get("ego-arrows")
    ));
    let mut invalid = save;
    let knowledge = invalid
        .item_property_knowledge
        .iter_mut()
        .find(|entry| entry.item_id == "ego-arrows")
        .unwrap();
    knowledge.identified = true;
    knowledge.known_affix_ids = vec!["demo.affix.frost-hunter".to_owned()];
    assert!(Game::from_save(invalid).is_err());
}

#[test]
fn tomte_sensing_identifies_nameless_jewelry_but_ignores_glove_attack_bonuses() {
    let mut game = tomte_sensing_game(39);
    let mut kinds = ["ring", "amulet", "quiver"]
        .into_iter()
        .map(|slot| {
            game.content
                .item_definitions()
                .find(|item| {
                    item.equipment_slot.as_deref() == Some(slot)
                        && !item.tags.iter().any(|tag| tag == "artifact")
                })
                .unwrap()
                .id
                .clone()
        })
        .collect::<Vec<_>>();
    kinds.push("demo.item.feanorian-lamp".to_owned());
    for kind in kinds {
        give_inventory_item(&mut game, &kind, &kind);
        game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
        game.apply_player_floor_item_knowledge();
        assert!(game.item_property_knowledge[&kind].appraised, "{kind}");
        assert_eq!(game.item_property_knowledge[&kind].feeling, None, "{kind}");
        assert_eq!(game.item_knowledge_dto(&kind), ItemKnowledgeDto::Aware);
    }
    give_inventory_item(&mut game, "gloves", "demo.item.set-of-gauntlets");
    let gloves = game.items.last_mut().unwrap();
    gloves.location = ItemLocation::Ground(game.player.position);
    gloves.enchantments.to_hit = 5;
    gloves.enchantments.to_damage = 5;
    game.apply_player_floor_item_knowledge();
    assert_eq!(
        game.item_property_knowledge["gloves"].feeling,
        Some(ItemFeelingDto::Average)
    );
}

#[test]
fn fabric_bag_projects_four_non_ammunition_slots() {
    let mut game = Game::new(42);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.inventory.fabric-bag",
        "demo.item.fabric-bag",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.inventory.fabric-bag".to_owned(),
            slot_id: None,
        },
    );

    assert_eq!(update.player.inventory_used_slots, 0);
    assert_eq!(update.player.inventory_slot_capacity, 30);
    assert!(
        update
            .equipment
            .iter()
            .any(|item| { item.id == "test.inventory.fabric-bag" && item.slot_id == "container" })
    );
}

#[test]
fn armor_hit_modifier_only_changes_melee_skill() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    give_inventory_item(
        &mut game,
        "test.item.hard-leather-armour",
        "demo.item.hard-leather-armour",
    );
    game.items[0].location = ItemLocation::Equipped {
        slot_id: "body".to_owned(),
    };

    let equipped = game.player_derived_stats();
    assert_eq!(equipped.melee_skill.value, baseline.melee_skill.value - 1);
    assert_eq!(
        equipped.melee_damage_bonus.value,
        baseline.melee_damage_bonus.value
    );
    assert_eq!(equipped.ranged_skill.value, baseline.ranged_skill.value);
}

#[test]
fn gauntlets_add_their_hit_and_damage_modifiers_to_melee() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    give_inventory_item(
        &mut game,
        "test.item.set-of-gauntlets",
        "demo.item.set-of-gauntlets",
    );
    game.items[0].location = ItemLocation::Equipped {
        slot_id: "hands".to_owned(),
    };

    let equipped = game.player_derived_stats();
    assert_eq!(equipped.melee_skill.value, baseline.melee_skill.value + 1);
    assert_eq!(
        equipped.melee_damage_bonus.value,
        baseline.melee_damage_bonus.value + 1
    );
    assert_eq!(equipped.ranged_skill.value, baseline.ranged_skill.value);
}

#[test]
fn shovel_equips_as_a_tool_without_replacing_the_melee_weapon_profile() {
    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats();
    give_inventory_item(&mut game, "test.item.shovel", "demo.item.shovel");

    let update = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.item.shovel".to_owned(),
            slot_id: Some("tool".to_owned()),
        },
    );

    let equipped = game.player_derived_stats();
    let profile = game.player_melee_profile(&equipped);
    assert_eq!(update.equipment[0].slot_id, "tool");
    assert_eq!(equipped.dig_skill.value, baseline.dig_skill.value + 46);
    assert_eq!(equipped.attack.value, baseline.attack.value);
    assert_eq!(equipped.defense.value, baseline.defense.value);
    assert_eq!(equipped.melee_skill.value, baseline.melee_skill.value);
    assert_eq!(
        equipped.melee_damage_bonus.value,
        baseline.melee_damage_bonus.value
    );
    assert_eq!((profile.damage_dice, profile.damage_sides), (1, 2));
    assert_eq!(profile.source_item_id, None);
}

#[test]
fn original_diggers_use_weight_and_tunneling_pval_without_stacking_with_weapons() {
    for (kind_id, expected) in [
        ("demo.item.shovel", 46),
        ("demo.item.pick", 55),
        ("demo.item.gnomish-shovel", 66),
        ("demo.item.orcish-pick", 75),
        ("demo.item.mattock", 85),
    ] {
        let mut game = Game::new(42);
        game.items.clear();
        let baseline = game.player_derived_stats().dig_skill.value;
        give_inventory_item(&mut game, "test.digger", kind_id);
        dispatch_next(
            &mut game,
            GameCommand::Equip {
                item_id: "test.digger".to_owned(),
                slot_id: Some("tool".to_owned()),
            },
        );
        assert_eq!(
            game.player_derived_stats().dig_skill.value,
            baseline + expected
        );
    }

    let mut game = Game::new(42);
    game.items.clear();
    let baseline = game.player_derived_stats().dig_skill.value;
    give_inventory_item(&mut game, "test.weapon", "demo.item.broad-sword");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.weapon".to_owned(),
            slot_id: Some("right-hand".to_owned()),
        },
    );
    give_inventory_item(&mut game, "test.tool", "demo.item.orcish-pick");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.tool".to_owned(),
            slot_id: Some("tool".to_owned()),
        },
    );
    assert_eq!(game.player_derived_stats().dig_skill.value, baseline + 75);
}

#[test]
fn tool_rejects_an_unrelated_target_slot_without_changing_inventory() {
    let mut game = Game::new(42);
    game.items.clear();
    give_inventory_item(&mut game, "test.item.shovel", "demo.item.shovel");

    let update = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.item.shovel".to_owned(),
            slot_id: Some("body".to_owned()),
        },
    );

    assert!(update.equipment.is_empty());
    assert_eq!(update.inventory.len(), 1);
    assert_eq!(update.inventory[0].id, "test.item.shovel");
    assert_eq!(update.events[0].kind, "item.equip.none");
}

#[test]
fn sling_bow_and_crossbow_resolve_their_compatible_ammunition_profiles() {
    let cases = [
        (
            "demo.item.sling",
            "demo.item.mithril-shot",
            15,
            200,
            5,
            3,
            10,
        ),
        (
            "demo.item.long-bow",
            "demo.item.sheaf-arrow",
            16,
            300,
            4,
            4,
            20,
        ),
        (
            "demo.item.heavy-crossbow",
            "demo.item.adamantine-bolt",
            18,
            400,
            7,
            5,
            10,
        ),
    ];

    for (launcher_kind, ammo_kind, range, multiplier, dice, sides, break_chance) in cases {
        let mut game = Game::new(0);
        game.items.clear();
        give_inventory_item(&mut game, "test.item.launcher", launcher_kind);
        give_inventory_item(&mut game, "test.item.ammunition", ammo_kind);
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.launcher")
            .expect("test launcher should exist")
            .location = ItemLocation::Equipped {
            slot_id: "shooting".to_owned(),
        };

        let profile = game
            .player_projectile_profile()
            .expect("compatible ammunition should resolve a projectile profile");
        assert_eq!(profile.range, range);
        assert_eq!(profile.damage_multiplier_percent, multiplier);
        assert_eq!((profile.damage_dice, profile.damage_sides), (dice, sides));
        assert_eq!(profile.ammo_kind_id, ammo_kind);
        assert_eq!(profile.ammo_break_chance_percent, break_chance);
    }
}

#[test]
fn pickup_on_empty_ground_is_zero_time() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items.clear();
    let before = game.state_hash();
    let world_tick = game.world_tick;
    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("empty pickup should still execute");

    assert_eq!(update.turn, 1);
    assert_eq!(update.world_tick, world_tick);
    assert!(update.changed_cells.is_empty());
    assert!(update.inventory.is_empty());
    assert_eq!(update.events[0].message_key, "item-pickup-none");
    assert_ne!(update.state_hash, before);
}

#[test]
fn stale_revision_is_rejected_without_mutation() {
    let mut game = Game::new(1);
    let before = game.state_hash();
    let error = game
        .dispatch(command(1, 99, GameCommand::Wait))
        .expect_err("stale command should fail");
    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
    assert_eq!(game.state_hash(), before);
}

#[test]
fn inventory_item_missing_its_kind_is_an_invariant_error() {
    const ITEM_ID: &str = "test.item.missing-kind";
    let mut game = skill_check_game(1, "demo.build.warrior");
    give_inventory_item(&mut game, ITEM_ID, "demo.item.clarity-draught");
    game.items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("test item should exist")
        .kind_id = "test.item-kind.missing".to_owned();

    let result = game.dispatch(command(
        1,
        0,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    ));
    assert!(matches!(result, Err(CoreError::Invariant(_))));
}

#[test]
fn elemental_brand_is_suppressed_only_by_matching_immunity() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    let weapon_slot = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "weapon")
        .unwrap()
        .id
        .clone();
    game.items.retain(
        |item| !matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == &weapon_slot),
    );
    game.items.push(ItemInstance {
        previously_worn: false,
        book_counted: false,
        artifact_name: None,
        intrinsic_melee_damage_dice: None,
        intrinsic_weight_tenths_pound: None,
        intrinsic_weapon_traits: Default::default(),
        intrinsic_curse_effects: Default::default(),
        id: "test.item.ember-edge".to_owned(),
        kind_id: "demo.item.ember-edge".to_owned(),
        quantity: 1,
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        intrinsic_properties: Default::default(),
        permanent_destruction_immunities: Default::default(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        captured_actor: None,
        location: ItemLocation::Equipped {
            slot_id: weapon_slot,
        },
    });
    let profile = game.player_melee_profile(&game.player_derived_stats());
    let definition = game
        .content
        .actor("demo.actor.ash-drake")
        .expect("demo target")
        .clone();
    let mut target = actor_from_runtime_spawn(
        "test.actor.brand-target",
        &definition.id,
        Position { x: 4, y: 3 },
        definition.max_hp,
        definition.speed,
        0,
        true,
    );

    target
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Resistant);
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        24
    );
    target
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Immune);
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        10
    );
}

#[test]
fn offensive_flag_dto_hides_unknown_affix_contributions() {
    let mut game = Game::new(0);
    let item_id = "test.item.known-offense".to_owned();
    game.items.push(ItemInstance {
        previously_worn: false,
        book_counted: false,
        artifact_name: None,
        intrinsic_melee_damage_dice: None,
        intrinsic_weight_tenths_pound: None,
        intrinsic_weapon_traits: Default::default(),
        intrinsic_curse_effects: Default::default(),
        id: item_id.clone(),
        kind_id: "demo.item.ember-edge".to_owned(),
        quantity: 1,
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.frost-hunter".to_owned()],
        rolled_affixes: Vec::new(),
        intrinsic_properties: Default::default(),
        permanent_destruction_immunities: Default::default(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        captured_actor: None,
        location: ItemLocation::Inventory,
    });

    let hidden = game
        .inventory_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("test item");
    assert_eq!(hidden.brands, vec![WeaponBrandDto::Fire]);
    assert!(hidden.slays.is_empty());

    game.item_property_knowledge.insert(
        item_id.clone(),
        ItemPropertyKnowledgeState {
            discovered: true,
            appraised: true,
            identified: true,
            feeling: None,
            known_affix_ids: BTreeSet::from(["demo.affix.frost-hunter".to_owned()]),
        },
    );
    let visible = game
        .inventory_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("test item");
    assert_eq!(
        visible.brands,
        vec![WeaponBrandDto::Fire, WeaponBrandDto::Cold]
    );
    assert_eq!(
        visible.slays,
        vec![SlayDto {
            target: SlayTargetDto::Animal,
            level: SlayLevelDto::Slay,
        }]
    );
}

fn p88b_add_item(
    game: &mut Game,
    id: &str,
    kind_id: &str,
    quantity: u32,
    location: ItemLocation,
    affix_ids: &[&str],
) {
    give_inventory_item(game, id, kind_id);
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .expect("P88B item should exist");
    item.quantity = quantity;
    item.location = location;
    item.affix_ids = affix_ids.iter().map(|id| (*id).to_owned()).collect();
}

#[test]
fn p88b_protection_quiver_skips_quivered_ammunition_without_rng() {
    let mut game = Game::new(0);
    game.items.clear();
    p88b_add_item(
        &mut game,
        "p88b.quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut game,
        "p88b.arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    assert_eq!(game.inventory_used_slots(), 0);

    let draws = game.rng_draw_counter();
    let mut events = Vec::new();
    game.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut events);

    assert_eq!(game.rng_draw_counter(), draws);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "p88b.arrows")
            .unwrap()
            .quantity,
        60
    );
    assert!(events.is_empty());

    let mut expected = game.clone();
    let _nuke_resistance_roll = expected.rng.bounded(55);
    game.damage_player_inventory("test.nuke", DamageType::Nuke, false, 1, &mut events);
    assert_eq!(game.rng_draw_counter(), draws + 1);
    assert_eq!(game.rng.bounded(10_000), expected.rng.bounded(10_000));
    assert!(events.is_empty());
}

#[test]
fn ent_inventory_fire_saves_use_adjusted_resistance_and_original_random_bound() {
    let mut base = Game::new_with_build(427, "demo.build.warrior").unwrap();
    base.items.clear();
    p88b_add_item(
        &mut base,
        "test.ent.arrows",
        "demo.item.arrow",
        64,
        ItemLocation::Inventory,
        &[],
    );
    for (native, temporary) in [(false, false), (true, false), (false, true)] {
        for (damage_type, resistance, ent_percent, protected) in [
            (DamageType::Fire, ResistanceLevel::Vulnerable, 0, false),
            (DamageType::Fire, ResistanceLevel::Normal, 0, false),
            (DamageType::Fire, ResistanceLevel::Resistant, 35, false),
            (DamageType::Fire, ResistanceLevel::Strong, 45, false),
            (DamageType::Fire, ResistanceLevel::Immune, 100, false),
            (DamageType::Fire, ResistanceLevel::Resistant, 35, true),
            (DamageType::Acid, ResistanceLevel::Resistant, 50, false),
        ] {
            for seed in 0..16 {
                let mut game = base.clone();
                if native {
                    game.build.as_mut().unwrap().race_id = "rfb-legacy.race.ent".to_owned();
                }
                if temporary {
                    let mut form =
                        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.ent")
                            .status;
                    form.granted_race_id = Some("rfb-legacy.race.ent".to_owned());
                    game.player.statuses.push(form);
                }
                if protected {
                    game.player.statuses.push(
                        monster_combat::melee_status(
                            STATUS_INVENTORY_PROTECTION,
                            100,
                            "test.protection",
                        )
                        .status,
                    );
                }
                game.player.resistances.set(damage_type, resistance);
                game.rng = RfbRng::seeded(seed);
                let mut expected_rng = game.rng.clone();
                let ent_fire = (native || temporary) && damage_type == DamageType::Fire;
                let percent = if ent_fire {
                    ent_percent
                } else {
                    resistance.reduction_percent().max(0)
                };
                let mut destroyed = 0;
                for _ in 0..64 {
                    if expected_rng.bounded(100) < 3 {
                        if protected {
                            expected_rng.bounded(100);
                        } else if expected_rng.bounded(if ent_fire { 54 } else { 66 })
                            >= percent as u64
                        {
                            destroyed += 1;
                        }
                    }
                }
                game.damage_player_inventory(
                    "test.ent-fire",
                    damage_type,
                    false,
                    1,
                    &mut Vec::new(),
                );
                assert_eq!(
                    game.items[0].quantity,
                    64 - destroyed,
                    "native={native}, temporary={temporary}, {damage_type:?}, {resistance:?}, seed={seed}"
                );
                assert_eq!(game.rng, expected_rng);
            }
        }
    }
}

#[test]
fn p88b_quiver_overflow_remains_vulnerable_and_emits_partial_destruction() {
    let mut game = Game::new(0);
    game.items.clear();
    p88b_add_item(
        &mut game,
        "p88b.quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    for id in ["p88b.arrows-a", "p88b.arrows-b"] {
        p88b_add_item(
            &mut game,
            id,
            "demo.item.arrow",
            60,
            ItemLocation::Inventory,
            &[],
        );
    }
    assert_eq!(game.inventory_used_slots(), 1);

    let mut events = Vec::new();
    game.resolve_monster_damage_to_player(
        "test.monster",
        "test.monster-kind",
        "test.acid-bolt",
        0,
        1,
        1,
        DamageType::Acid,
        &mut events,
    );

    let protected = game
        .items
        .iter()
        .find(|item| item.id == "p88b.arrows-a")
        .expect("quivered arrows should remain");
    let overflow = game
        .items
        .iter()
        .find(|item| item.id == "p88b.arrows-b")
        .expect("partially destroyed overflow should remain");
    assert_eq!(protected.quantity, 60);
    assert!(overflow.quantity < 60);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::InventoryItemDestroyedByDamage {
            source_kind_id,
            target_kind_id,
            quantity,
        } if source_kind_id == "test.monster-kind"
            && target_kind_id == "demo.item.arrow"
            && *quantity == 60 - overflow.quantity
    )));
}

#[test]
fn p88b_protection_boundaries_preserve_other_destruction_rules() {
    let mut touch = Game::new(0);
    touch.items.clear();
    p88b_add_item(
        &mut touch,
        "p88b.touch-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    let draws = touch.rng_draw_counter();
    touch.damage_player_inventory("test.plasma", DamageType::Plasma, true, 1, &mut Vec::new());
    assert_eq!(touch.rng_draw_counter(), draws);

    let mut enduring = Game::new(0);
    enduring.items.clear();
    p88b_add_item(
        &mut enduring,
        "p88b.enduring-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &["rfb-legacy.affix.endurance"],
    );
    let draws = enduring.rng_draw_counter();
    enduring.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut Vec::new());
    assert_eq!(enduring.rng_draw_counter(), draws);

    let mut protected = Game::new(0);
    protected.items.clear();
    p88b_add_item(
        &mut protected,
        "p88b.protected-status-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    protected
        .player
        .statuses
        .push(monster_combat::melee_status(STATUS_INVENTORY_PROTECTION, 10, "test.status").status);
    let draws = protected.rng_draw_counter();
    protected.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut Vec::new());
    assert!(protected.rng_draw_counter() > draws);
    assert_eq!(protected.items[0].quantity, 60);

    let mut manual = Game::new(0);
    manual.items.clear();
    p88b_add_item(
        &mut manual,
        "p88b.manual-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut manual,
        "p88b.manual-arrows",
        "demo.item.arrow",
        10,
        ItemLocation::Inventory,
        &[],
    );
    assert_eq!(
        manual
            .destroy_item("p88b.manual-arrows", 1)
            .unwrap()
            .quantity,
        1
    );
}

#[test]
fn p88e_protection_quiver_does_not_cover_unequipped_fired_or_ground_ammunition() {
    let mut unequipped = Game::new(0);
    unequipped.items.clear();
    p88b_add_item(
        &mut unequipped,
        "p88e.unequipped-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Inventory,
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut unequipped,
        "p88e.unequipped-arrows",
        "demo.item.arrow",
        60,
        ItemLocation::Inventory,
        &[],
    );
    let draws = unequipped.rng_draw_counter();
    unequipped.damage_player_inventory("test.acid", DamageType::Acid, false, 1, &mut Vec::new());
    assert!(unequipped.rng_draw_counter() > draws);
    assert!(
        unequipped
            .items
            .iter()
            .find(|item| item.id == "p88e.unequipped-arrows")
            .is_none_or(|item| item.quantity < 60)
    );

    let mut fired = Game::new(0);
    fired.items.clear();
    p88b_add_item(
        &mut fired,
        "p88e.fired-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    p88b_add_item(
        &mut fired,
        "p88e.fired-arrows",
        "demo.item.arrow",
        10,
        ItemLocation::Inventory,
        &[],
    );
    let ammunition = fired
        .take_inventory_item("p88e.fired-arrows")
        .expect("taking ammunition should succeed")
        .expect("ammunition should exist");
    let landing = fired.player.position;
    let mut fired_events = Vec::new();
    fired.settle_projectile_ammunition(
        ammunition,
        landing,
        true,
        100,
        &mut fired_events,
        &mut BTreeSet::new(),
    );
    assert_eq!(
        fired
            .items
            .iter()
            .find(|item| item.id == "p88e.fired-arrows")
            .expect("the remaining stack should exist")
            .quantity,
        9
    );
    assert!(fired_events.iter().any(|event| matches!(
        event,
        DomainEvent::ProjectileAmmoBroken { ammo_kind_id }
            if ammo_kind_id == "demo.item.arrow"
    )));

    let mut ground = Game::new(0);
    ground.items.clear();
    p88b_add_item(
        &mut ground,
        "p88e.ground-quiver",
        "demo.item.quiver",
        1,
        ItemLocation::Equipped {
            slot_id: "quiver".to_owned(),
        },
        &["rfb-legacy.affix.quiver-protection"],
    );
    let ground_position = ground.player.position;
    p88b_add_item(
        &mut ground,
        "p88e.ground-arrows",
        "demo.item.arrow",
        10,
        ItemLocation::Ground(ground_position),
        &[],
    );
    let mut ground_events = Vec::new();
    ground.resolve_ground_item_projectile_effects(
        "test.fire",
        &[ground_position],
        DamageType::Fire,
        true,
        &mut ground_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(
        ground
            .items
            .iter()
            .all(|item| item.id != "p88e.ground-arrows")
    );
    assert!(ground_events.iter().any(|event| matches!(
        event,
        DomainEvent::GroundItemDestroyedByAbility {
            item_id,
            target_kind_id,
            quantity: 10,
            ..
        } if item_id == "p88e.ground-arrows" && target_kind_id == "demo.item.arrow"
    )));
}

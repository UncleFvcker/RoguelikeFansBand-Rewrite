// SPDX-License-Identifier: MPL-2.0
use super::*;

const N3: &[u32] = &[
    60, 74, 85, 92, 125, 139, 150, 153, 171, 172, 173, 175, 179, 195, 208, 270, 272, 275, 279, 294,
    334, 335, 341, 362, 381,
];

#[test]
#[ignore = "explicit N3 preparation for ordinary standalone UI acceptance"]
fn export_n3_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut base = Game::from_save(payload).unwrap();
    choose_human_talent_if_pending(&mut base);
    base.apply_player_experience(base.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut base);
    base.progress.attributes.strength = base.progress.attribute_potentials.strength;
    base.progress.attributes.dexterity = base.progress.attribute_potentials.dexterity;
    base.progress.maximum_attributes.strength = base.progress.attributes.strength;
    base.progress.maximum_attributes.dexterity = base.progress.attributes.dexterity;
    descend_one_floor(&mut base);
    base.apply_player_melee_status(STATUS_INVULNERABILITY, 200_000, "test.n3.desktop");
    let mut scenarios = Vec::new();
    for slug in [
        "eternal-blade",
        "moms-sniper-crossbow",
        "great-maul-of-vice",
        "cupids-arrow",
    ] {
        let mut game = base.clone();
        clear_monsters(&mut game);
        game.items.clear();
        game.gold = 20_000;
        game.player.position = Position { x: 10, y: 10 };
        for x in 10..=15 {
            replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
        }
        give_inventory_item(&mut game, "test.n3.light", "demo.item.wooden-torch");
        game.equip_inventory_item("test.n3.light", None).unwrap();
        let id = generate(&mut game, &format!("demo.item.{slug}"));
        game.items.iter_mut().find(|i| i.id == id).unwrap().location =
            ItemLocation::Ground(game.player.position);
        if slug == "moms-sniper-crossbow" {
            give_inventory_item(&mut game, "test.n3.ammo", "demo.item.arrow");
        }
        if slug == "cupids-arrow" {
            give_inventory_item(&mut game, "test.n3.bow", "demo.item.long-bow");
            game.equip_inventory_item("test.n3.bow", None).unwrap();
        }
        if slug != "great-maul-of-vice" {
            target(&mut game, "war-bear");
            game.entities[0]
                .statuses
                .push(monster_combat::melee_status(STATUS_SLEEP, 10_000, "test.n3.sleep").status);
        }
        game.refresh_player_resource_maxima();
        game.player.hp = game.effective_player_max_hp();
        game.reveal_current_visibility();
        let mut commands = vec![GameCommand::PickUp];
        if slug != "cupids-arrow" {
            commands.push(GameCommand::Equip {
                item_id: id.clone(),
                slot_id: None,
            });
        }
        commands.push(match slug {
            "eternal-blade" => GameCommand::Move {
                direction: Direction::East,
            },
            "moms-sniper-crossbow" | "cupids-arrow" => GameCommand::Fire {
                direction: Direction::East,
            },
            _ => GameCommand::UseItem {
                item_id: id.clone(),
                target: None,
            },
        });
        commands.push(GameCommand::Wait);
        let seed = (0..5000)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                for command in &commands {
                    if trial.player_is_dead() {
                        return false;
                    }
                    dispatch_next(&mut trial, command.clone());
                }
                if trial.player_is_dead() {
                    return false;
                }
                match slug {
                    "great-maul-of-vice" => {
                        trial
                            .items
                            .iter()
                            .find(|i| i.id == id)
                            .unwrap()
                            .charges
                            .unwrap()
                            .current
                            == 0
                            && trial.gold < 20_000
                    }
                    "cupids-arrow" => trial
                        .entities
                        .iter()
                        .any(|a| a.controller_id.as_deref() == Some(&trial.player.id)),
                    _ => trial.entities.first().is_none_or(|a| a.hp < a.max_hp),
                }
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let start = game.clone();
        let mut steps = Vec::new();
        for command in commands {
            let activation_slot = game.items.iter().find(|i| i.id == id).and_then(|i| {
                if let ItemLocation::Equipped { slot_id } = &i.location {
                    Some(slot_id.clone())
                } else {
                    None
                }
            });
            dispatch_next(&mut game, command.clone());
            resume(&game);
            steps.push(serde_json::json!({"command":command,"hash":game.state_hash(),"activationSlot":activation_slot}));
        }
        std::fs::write(
            directory.join(format!("{slug}.rfbsave")),
            rfb_save::encode(&header, &start.to_save()).unwrap(),
        )
        .unwrap();
        scenarios
            .push(serde_json::json!({"name":slug,"initialHash":start.state_hash(),"steps":steps}));
    }
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&scenarios).unwrap(),
    )
    .unwrap();
}

fn prepared_mage() -> Game {
    let mut game = super::prepared_mage();
    game.progress.attributes.strength = game.progress.attribute_potentials.strength;
    game.progress.attributes.dexterity = game.progress.attribute_potentials.dexterity;
    game.progress.maximum_attributes.strength = game.progress.attributes.strength;
    game.progress.maximum_attributes.dexterity = game.progress.attributes.dexterity;
    game.refresh_player_resource_maxima();
    game
}

fn strike(game: &mut Game) -> Vec<DomainEvent> {
    if game.entities.is_empty() {
        Vec::new()
    } else {
        super::strike(game)
    }
}

fn equip(game: &mut Game, slug: &str, slot: Option<&str>) -> String {
    let id = generate(game, &format!("demo.item.{slug}"));
    game.equip_inventory_item(&id, slot).unwrap();
    game.refresh_player_resource_maxima();
    id
}

fn target(game: &mut Game, slug: &str) {
    clear_monsters(game);
    game.push_generated_actor(
        "test.n3.target".into(),
        &format!("demo.actor.{slug}"),
        Position { x: 11, y: 10 },
    );
    game.entities[0].statuses.clear();
    game.reveal_current_visibility();
}

fn resume(game: &Game) -> Game {
    let loaded = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), loaded.state_hash());
    assert_eq!(game.rng, loaded.rng);
    loaded
}

#[test]
fn n3_all_25_generate_unknown_identify_equip_act_and_resume() {
    let prepared = prepared_mage();
    let definitions: Vec<_> = prepared
        .content
        .item_definitions()
        .filter(|item| {
            item.artifact_generation
                .as_ref()
                .is_some_and(|g| N3.contains(&g.source_index))
        })
        .cloned()
        .collect();
    assert_eq!(definitions.len(), 25);
    for definition in definitions {
        let mut game = prepared.clone();
        let id = generate(&mut game, &definition.id);
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|knowledge| knowledge.appraised)
        );
        game = resume(&game);
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        if definition.equipment_slot.is_some() {
            game.equip_inventory_item(&id, None).unwrap();
        }
        if definition.ammunition_profile.is_some() {
            give_inventory_item(&mut game, "test.n3.bow", "demo.item.long-bow");
            game.equip_inventory_item("test.n3.bow", None).unwrap();
        } else if let Some(profile) = &definition.projectile_profile {
            give_inventory_item(
                &mut game,
                "test.n3.ammo",
                if profile.ammunition_type == AmmunitionTypeDefinition::Bolt {
                    "demo.item.bolt"
                } else {
                    "demo.item.arrow"
                },
            );
        }
        game.refresh_player_resource_maxima();
        target(&mut game, "greater-titan");
        if definition.melee_profile.is_some() {
            strike(&mut game);
        } else if definition.projectile_profile.is_some() || definition.ammunition_profile.is_some()
        {
            shoot(&mut game);
        }
        let mut loaded = resume(&game);
        game.world_tick = 10;
        loaded.world_tick = 10;
        for trial in [&mut game, &mut loaded] {
            trial
                .process_equipped_curse_effects(
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
        }
        assert_eq!(game.state_hash(), loaded.state_hash(), "{}", definition.id);
    }
}

#[test]
fn n3_target_glyph_exceptions_change_actual_melee() {
    let base = prepared_mage();
    for (weapon, glyph, message) in [
        ("zantetsuken", "j", "item-zantetsuken-elastic"),
        ("skynail", "B", "item-skynail-refuses"),
    ] {
        let mut game = base.clone();
        equip(&mut game, weapon, None);
        let kind = game
            .content
            .actor_definitions()
            .find(|a| a.glyph == glyph && !a.tags.iter().any(|tag| tag == "unique"))
            .unwrap()
            .id
            .clone();
        target(&mut game, kind.strip_prefix("demo.actor.").unwrap());
        let mut observed = false;
        for seed in 0..100 {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(seed);
            let events = strike(&mut trial);
            assert_eq!(trial.entities[0].hp, game.entities[0].hp, "{weapon}");
            observed |= events.iter().any(|event| matches!(event, DomainEvent::ItemSpecialMessage {message_key} if message_key == message));
        }
        assert!(observed, "{weapon}");
        target(&mut game, "greater-titan");
        for _ in 0..20 {
            strike(&mut game);
        }
        assert!(game.entities.first().is_none_or(|a| a.hp < a.max_hp));
    }
}

#[test]
fn n3_ordered_dual_blades_and_littlethorn_revoke_when_reversed() {
    let mut game = prepared_mage();
    let right = equip(&mut game, "musashi-katana", Some("right-hand"));
    assert_eq!(game.dual_wielding_accuracy_per_mille(&right), 1000);
    let left = equip(&mut game, "musashi-wakizashi", Some("left-hand"));
    assert_eq!(game.dual_wielding_accuracy_per_mille(&right), 1000);
    assert_eq!(game.dual_wielding_accuracy_per_mille(&left), 1000);
    for item in &mut game.items {
        if item.id == right {
            item.location = ItemLocation::Equipped {
                slot_id: "left-hand".into(),
            };
        }
        if item.id == left {
            item.location = ItemLocation::Equipped {
                slot_id: "right-hand".into(),
            };
        }
    }
    assert!(game.dual_wielding_accuracy_per_mille(&right) < 1000);
    resume(&game);
    game.items.clear();
    let partner = game
        .content
        .item_definitions()
        .find(|item| {
            item.artifact_generation
                .as_ref()
                .is_some_and(|g| g.source_index == 174)
        })
        .unwrap()
        .id
        .clone();
    let right = generate(&mut game, &partner);
    game.equip_inventory_item(&right, Some("right-hand"))
        .unwrap();
    let before = game.player_derived_stats();
    let left = equip(&mut game, "littlethorn", Some("left-hand"));
    let paired = game.player_derived_stats();
    game.items
        .iter_mut()
        .find(|item| item.id == left)
        .unwrap()
        .location = ItemLocation::Inventory;
    assert_eq!(paired.speed.value - before.speed.value, 7);
    assert!(paired.armor_class.value >= before.armor_class.value + 10);
    assert_eq!(game.player_derived_stats().speed.value, before.speed.value);
}

#[test]
fn n3_nothung_and_bard_pair_reach_named_targets() {
    let mut game = prepared_mage();
    let sword = equip(&mut game, "nothung", None);
    target(&mut game, "fafner-the-dragon");
    let item = game.items.iter().find(|item| item.id == sword).unwrap();
    let monster = &game.entities[0];
    assert!(
        game.item_damage_multiplier(
            item,
            monster,
            game.actor_runtime_definition(monster).unwrap()
        ) >= 150
    );
    let events = strike(&mut game);
    assert!(!events.is_empty());
    game.items.clear();
    let bow = equip(&mut game, "bard-long-bow", None);
    generate(&mut game, "demo.item.bard-black-arrow");
    target(&mut game, "smaug-the-golden");
    let profile = game.player_projectile_profile().unwrap();
    let monster = &game.entities[0];
    let paired = game.player_projectile_damage_multiplier(
        &profile,
        monster,
        game.actor_runtime_definition(monster).unwrap(),
    );
    assert!(paired >= 250);
    let seed = (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            shoot(&mut trial);
            trial.entities.first().is_none_or(|a| a.hp < a.max_hp)
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    shoot(&mut game);
    assert!(game.entities.first().is_none_or(|a| a.hp < a.max_hp));
    game.items
        .iter_mut()
        .find(|item| item.id == bow)
        .unwrap()
        .location = ItemLocation::Inventory;
    resume(&game);
}

#[test]
fn n3_assassinator_kills_sleeping_nonunique_with_one_attempt() {
    let mut base = prepared_mage();
    equip(&mut base, "assassinator", None);
    give_inventory_item(&mut base, "test.n3.light", "demo.item.wooden-torch");
    base.equip_inventory_item("test.n3.light", None).unwrap();
    target(&mut base, "greater-titan");
    base.apply_actor_melee_status(0, STATUS_SLEEP, 100, "test.n3.sleep");
    let seed = (0..1000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            strike(&mut game);
            game.entities.is_empty()
        })
        .expect("sleeping assassination must kill beyond ordinary damage");
    let mut asleep = base.clone();
    asleep.rng = RfbRng::seeded(seed);
    let mut loaded = resume(&asleep);
    strike(&mut asleep);
    strike(&mut loaded);
    assert_eq!(asleep.state_hash(), loaded.state_hash());
    base.entities[0].statuses.clear();
    base.rng = RfbRng::seeded(seed);
    strike(&mut base);
    assert!(!base.entities.is_empty());
}

#[test]
fn n3_golden_hammer_transfers_real_held_stack_and_saves() {
    let mut game = prepared_mage();
    equip(&mut game, "golden-hammer", None);
    target(&mut game, "greater-titan");
    give_inventory_item(&mut game, "test.n3.held", "demo.item.arrow");
    let held = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.n3.held")
        .unwrap();
    held.quantity = 7;
    held.location = ItemLocation::CarriedBy {
        actor_id: "test.n3.target".into(),
    };
    for _ in 0..30 {
        strike(&mut game);
    }
    let held = game
        .items
        .iter()
        .find(|item| item.id == "test.n3.held")
        .unwrap();
    assert_eq!(held.location, ItemLocation::Inventory);
    assert_eq!(held.quantity, 7);
    resume(&game);
}

#[test]
fn n3_vice_drains_in_pack_exact_zero_survives_then_blasts() {
    let mut game = prepared_mage();
    let id = generate(&mut game, "demo.item.great-maul-of-vice");
    game.rng = RfbRng::seeded(17);
    let mut expected = game.rng.clone();
    let amount = expected.bounded(50) as u32 + 1;
    game.gold = amount;
    game.world_tick = 10;
    game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert_eq!(game.gold, 0);
    assert_eq!(game.rng, expected);
    assert!(
        game.items
            .iter()
            .any(|item| item.id == id && game.item_is_fixed_artifact(item, 279))
    );
    let mut loaded = resume(&game);
    for trial in [&mut game, &mut loaded] {
        trial.world_tick = 20;
        trial
            .process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
    }
    assert_eq!(game.state_hash(), loaded.state_hash());
    let blasted = game.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(blasted.kind_id, "demo.item.great-hammer");
    assert_eq!(game.gold, 0);
}

#[test]
fn n3_dragonchip_disables_both_invulnerabilities_and_revokes_on_removal() {
    let mut game = prepared_mage();
    target(&mut game, "greater-titan");
    game.apply_player_melee_status(STATUS_INVULNERABILITY, 100, "test.n3.invuln");
    game.apply_actor_melee_status(0, STATUS_INVULNERABILITY, 100, "test.n3.invuln");
    game.entities[0]
        .statuses
        .iter_mut()
        .find(|s| s.kind_id == STATUS_INVULNERABILITY)
        .unwrap()
        .incoming_damage_percent = 0;
    let id = equip(&mut game, "dragonchip", None);
    let before = game.rng.clone();
    assert_eq!(game.player_spell_damage_percent(DamageType::Fire, 100), 100);
    assert_eq!(game.actor_incoming_damage_percent(0, 100, false), 100);
    assert_eq!(game.rng, before);
    assert_eq!(game.player_equipment_bonuses().magic_resistance_percent, 15);
    game.player.hp = game.effective_player_max_hp();
    let before_hp = game.player.hp;
    game.resolve_monster_damage_to_player(
        "test.n3.target",
        "demo.actor.greater-titan",
        "rfb-legacy.ability.arrow-physical-2d7",
        0,
        100,
        100,
        DamageType::Physical,
        &mut Vec::new(),
    );
    assert_eq!(
        before_hp - game.player.hp,
        100,
        "arrows are innate, not magical resistance targets"
    );
    let before_hp = game.player.hp;
    game.resolve_monster_damage_to_player(
        "test.n3.target",
        "demo.actor.greater-titan",
        "rfb-legacy.ability.bolt-fire-9d8-10",
        0,
        100,
        100,
        DamageType::Fire,
        &mut Vec::new(),
    );
    assert_eq!(before_hp - game.player.hp, 85);

    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .location = ItemLocation::Inventory;
    assert_eq!(game.player_equipment_bonuses().magic_resistance_percent, 0);
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(13) != 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    assert_eq!(game.actor_incoming_damage_percent(0, 100, false), 0);
    resume(&game);
}

#[test]
fn n3_moms_crossbow_fires_all_three_real_ammunition_types() {
    let mut base = prepared_mage();
    equip(&mut base, "moms-sniper-crossbow", None);
    for slug in ["arrow", "bolt", "iron-shot"] {
        let mut game = base.clone();
        give_inventory_item(&mut game, "test.n3.ammo", &format!("demo.item.{slug}"));
        target(&mut game, "greater-titan");
        assert!(game.player_projectile_profile().is_some(), "{slug}");
        let seed = (0..100)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                shoot(&mut trial);
                trial.entities.first().is_none_or(|a| a.hp < a.max_hp)
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut loaded = resume(&game);
        shoot(&mut game);
        shoot(&mut loaded);
        assert_eq!(game.state_hash(), loaded.state_hash());
    }
}

#[test]
fn n3_cupid_can_make_a_pet_without_losing_arrow_and_unique_is_immune() {
    let mut base = prepared_mage();
    give_inventory_item(&mut base, "test.n3.bow", "demo.item.long-bow");
    base.equip_inventory_item("test.n3.bow", None).unwrap();
    let arrow = generate(&mut base, "demo.item.cupids-arrow");
    target(&mut base, "greater-titan");
    let seed = (0..1000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            shoot(&mut game);
            game.entities[0].controller_id.is_some()
        })
        .unwrap();
    base.rng = RfbRng::seeded(seed);
    let mut loaded = resume(&base);
    shoot(&mut base);
    shoot(&mut loaded);
    assert_eq!(base.state_hash(), loaded.state_hash());
    assert!(
        base.items
            .iter()
            .any(|item| item.id == arrow && matches!(item.location, ItemLocation::Ground(_)))
    );
    let mut unique = prepared_mage();
    give_inventory_item(&mut unique, "test.n3.bow", "demo.item.long-bow");
    unique.equip_inventory_item("test.n3.bow", None).unwrap();
    generate(&mut unique, "demo.item.cupids-arrow");
    target(&mut unique, "smaug-the-golden");
    unique.rng = RfbRng::seeded(seed);
    shoot(&mut unique);
    assert!(unique.entities[0].controller_id.is_none());
}

#[test]
fn n3_bard_arrow_sticks_to_a_survivor_and_drops_on_its_death() {
    let mut base = prepared_mage();
    give_inventory_item(&mut base, "test.n3.bow", "demo.item.long-bow");
    base.equip_inventory_item("test.n3.bow", None).unwrap();
    let arrow = generate(&mut base, "demo.item.bard-black-arrow");
    target(&mut base, "greater-titan");
    let seed = (0..1000)
        .find(|seed| {
            let mut trial = base.clone();
            trial.rng = RfbRng::seeded(*seed);
            shoot(&mut trial);
            trial.items.iter().any(|item| {
                item.id == arrow && matches!(item.location, ItemLocation::CarriedBy { .. })
            })
        })
        .unwrap();
    base.rng = RfbRng::seeded(seed);
    shoot(&mut base);
    let mut game = resume(&base);
    let position = game.entities[0].position;
    game.resolve_ability_damage_to_entity(
        0,
        "test.n3.kill",
        DamageType::Mana,
        200_000,
        crate::event::ProjectileTrace {
            origin: game.player.position,
            impact: position,
            landing: position,
            traversed: vec![position],
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        game.items
            .iter()
            .any(|item| item.id == arrow && matches!(item.location, ItemLocation::Ground(_)))
    );
    game.reveal_current_visibility();
    resume(&game);
}

#[test]
fn n3_zantetsuken_cuts_arrows_but_not_equal_damage_rocks_or_blind_shots() {
    let mut game = prepared_mage();
    equip(&mut game, "zantetsuken", None);
    target(&mut game, "black-orc");
    let mut events = Vec::new();
    let before = game.player.hp;
    game.resolve_monster_damage_to_player(
        "test.n3.target",
        "demo.actor.black-orc",
        "rfb-legacy.ability.arrow-physical-2d7",
        0,
        20,
        20,
        DamageType::Physical,
        &mut events,
    );
    assert_eq!(game.player.hp, before);
    game.resolve_monster_damage_to_player(
        "test.n3.target",
        "demo.actor.black-orc",
        "rfb-legacy.ability.bolt-physical-1d1-74",
        0,
        20,
        20,
        DamageType::Physical,
        &mut events,
    );
    assert!(game.player.hp < before);
    game.apply_player_melee_status(STATUS_BLINDNESS, 100, "test.n3.blind");
    let before = game.player.hp;
    game.resolve_monster_damage_to_player(
        "test.n3.target",
        "demo.actor.black-orc",
        "rfb-legacy.ability.arrow-physical-2d7",
        0,
        20,
        20,
        DamageType::Physical,
        &mut events,
    );
    assert!(game.player.hp < before);
}

#[test]
fn n3_feanor_rolls_extra_gate_only_after_rarity() {
    let mut game = prepared_mage();
    game.generated_artifact_ids = game
        .content
        .item_definitions()
        .filter(|item| item.artifact_generation.is_some() && item.id != "demo.item.feanor")
        .map(|item| item.id.clone())
        .collect();
    let rarity = game
        .content
        .item("demo.item.feanor")
        .unwrap()
        .artifact_generation
        .as_ref()
        .unwrap()
        .rarity_one_in;
    let mut successes = 0;
    for seed in 0..1000 {
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        let admitted = expected.bounded(u64::from(rarity)) == 0 && expected.bounded(3) == 0;
        let result = game.roll_fixed_artifact_kind_id(
            &context(),
            Some("demo.item.pair-of-hard-leather-boots"),
            false,
        );
        assert_eq!(result.is_some(), admitted);
        assert_eq!(game.rng, expected);
        successes += usize::from(admitted);
    }
    assert!(successes > 0 && successes < 1000);
}

#[test]
fn n3_silver_hammer_suppresses_phoenix_and_dawn_and_stuns_werewolves() {
    let mut base = prepared_mage();
    let hammer = equip(&mut base, "silver-hammer", None);
    target(&mut base, "werewolf");
    for _ in 0..30 {
        strike(&mut base);
        if base.entities.is_empty()
            || base.entities[0]
                .statuses
                .iter()
                .any(|s| s.kind_id == STATUS_STUN)
        {
            break;
        }
    }
    assert!(
        base.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );
    for slug in ["the-phoenix", "warrior-of-the-dawn"] {
        let mut protected = base.clone();
        target(&mut protected, slug);
        protected.entities[0].hp = 1;
        let mut ordinary = protected.clone();
        ordinary
            .items
            .iter_mut()
            .find(|item| item.id == hammer)
            .unwrap()
            .location = ItemLocation::Inventory;
        let kill = |game: &mut Game| {
            let position = game.entities[0].position;
            game.resolve_ability_damage_to_entity(
                0,
                "test.n3.kill",
                DamageType::Mana,
                200_000,
                crate::event::ProjectileTrace {
                    origin: game.player.position,
                    impact: position,
                    landing: position,
                    traversed: vec![position],
                },
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        };
        let seed = (0..100)
            .find(|seed| {
                let mut trial = ordinary.clone();
                trial.rng = RfbRng::seeded(*seed);
                kill(&mut trial);
                trial
                    .entities
                    .iter()
                    .any(|a| a.kind_id == format!("demo.actor.{slug}") && a.hp > 0)
            })
            .unwrap();
        protected.rng = RfbRng::seeded(seed);
        kill(&mut protected);
        assert!(
            !protected
                .entities
                .iter()
                .any(|a| a.kind_id == format!("demo.actor.{slug}") && a.hp > 0),
            "{slug}"
        );
        protected.reveal_current_visibility();
        resume(&protected);
    }
}

#[test]
fn n3_eternal_blade_deals_time_damage_and_iron_ball_changes_alignment() {
    let mut game = prepared_mage();
    equip(&mut game, "eternal-blade", None);
    target(&mut game, "greater-titan");
    let mut observed = false;
    for _ in 0..20 {
        observed |= strike(&mut game).iter().any(|event| matches!(event, DomainEvent::AbilityHit { ability_id, damage, .. } if ability_id == "demo.item.eternal-blade" && damage.applied > 0));
    }
    assert!(observed);
    resume(&game);
    game.items.clear();
    let before = game.player_alignment();
    let ball = equip(&mut game, "iron-ball", None);
    assert_eq!(game.player_alignment(), before - 1000);
    game.items
        .iter_mut()
        .find(|item| item.id == ball)
        .unwrap()
        .location = ItemLocation::Inventory;
    assert_eq!(game.player_alignment(), before);
}

#[test]
fn n3_four_activations_spend_equipped_charge_and_resume_recovery() {
    for slug in [
        "feanor",
        "aegis-fang",
        "kamikaze-warrior",
        "great-maul-of-vice",
    ] {
        let mut game = prepared_mage();
        let id = equip(&mut game, slug, None);
        game.gold = 1_000_000;
        let definition = game.content.item(&format!("demo.item.{slug}")).unwrap();
        let ticks = definition
            .device_generation
            .as_ref()
            .unwrap()
            .recovery
            .as_ref()
            .unwrap()
            .interval_ticks;
        game.rng = RfbRng::seeded(
            (0..1000)
                .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
                .unwrap(),
        );
        game.use_inventory_item(
            &id,
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            0,
            "{slug}"
        );
        let mut loaded = resume(&game);
        for trial in [&mut game, &mut loaded] {
            for _ in 0..ticks {
                trial.world_tick += 1;
                trial.process_inventory_device_recovery(&mut Vec::new());
            }
        }
        assert_eq!(game.state_hash(), loaded.state_hash());
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            1
        );
    }
}

#[test]
fn n3_winblows_and_edge_use_one_in_ten_priest_blessing() {
    for slug in ["winblows", "microsoft-edge"] {
        let mut game = Game::new_with_build(493, "demo.build.priest-life-sorcery").unwrap();
        choose_human_talent_if_pending(&mut game);
        game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
        choose_human_talent_if_pending(&mut game);
        game.refresh_player_resource_maxima();
        game.progress.attributes.wisdom = game.progress.attribute_potentials.wisdom;
        game.progress.maximum_attributes.wisdom = game.progress.attributes.wisdom;
        game.refresh_player_resource_maxima();
        game.debug_set_ability_casts_succeed(true);
        for pool in game.resources.values_mut() {
            pool.current = pool.maximum;
        }
        let id = generate(&mut game, &format!("demo.item.{slug}"));
        let index = game.items.iter().position(|item| item.id == id).unwrap();
        crate::game::inventory::clear_item_curse(&mut game.items[index]);
        let seed = (0..1000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(100);
                rng.bounded(10) == 0
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        expected.bounded(100);
        expected.bounded(10);
        let mut blessing_events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.priest-bless-weapon",
            TargetSelection::Item {
                item_id: id.clone(),
            },
            &mut blessing_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            game.item_has_weapon_trait(&game.items[index], rfb_protocol::WeaponTraitDto::Blessed),
            "{slug}: {blessing_events:?}"
        );
        assert_eq!(game.items[index].discount_percent, 99);
        assert_eq!(game.rng, expected);
        resume(&game);
        game.items[index]
            .intrinsic_weapon_traits
            .remove(&rfb_protocol::WeaponTraitDto::Blessed);
        assert!(
            Game::from_save(game.to_save()).is_err(),
            "discounted fixed weapon without blessing must be rejected"
        );
    }
}

#[test]
fn n3_excalibur_jr_halves_spider_damage_before_resistance() {
    let mut base = prepared_mage();
    equip(&mut base, "excalibur-jr", None);
    let mut spider = base.clone();
    target(&mut spider, "cave-spider");
    let animal = base
        .content
        .actor_definitions()
        .find(|a| a.glyph != "S" && a.tags.iter().any(|tag| tag == "animal") && a.level < 5)
        .unwrap()
        .id
        .clone();
    let mut other = base;
    target(&mut other, animal.strip_prefix("demo.actor.").unwrap());
    let first_hit = |game: &mut Game| {
        strike(game).into_iter().find_map(|event| {
            if let DomainEvent::PlayerMeleeHit { damage, .. } = event {
                Some(damage.raw)
            } else {
                None
            }
        })
    };
    let mut compared = false;
    for seed in 0..100 {
        let mut s = spider.clone();
        let mut o = other.clone();
        s.rng = RfbRng::seeded(seed);
        o.rng = RfbRng::seeded(seed);
        if let (Some(s), Some(o)) = (first_hit(&mut s), first_hit(&mut o))
            && s > 0
            && s == o / 2
        {
            compared = true;
            break;
        }
    }
    assert!(compared);
}

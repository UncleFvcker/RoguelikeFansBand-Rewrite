// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn activate(game: &mut Game, id: &str, target: Option<&TargetSelection>) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.use_inventory_item(
        id,
        target,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn charge(game: &Game, id: &str) -> u32 {
    game.items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .charges
        .unwrap()
        .current
}

fn ready_seed() -> u64 {
    (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
        .unwrap()
}

fn elemental_hit(game: &Game, damage_type: DamageType, expected: i32) {
    let mut trial = game.clone();
    let hp = trial.player.hp;
    trial.resolve_monster_damage_to_player(
        "test.dragon",
        "demo.actor.ogre",
        "test.breath",
        0,
        20,
        20,
        damage_type,
        &mut Vec::new(),
    );
    assert_eq!(hp - trial.player.hp, expected, "{damage_type:?}");
}

#[test]
fn c4a_seiryu_full_pool_equipment_and_opposition_reduce_damage_after_save() {
    let mut game = Game::new_with_build(504, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        // A controlled generation floor, distinct from the player's depth-one
        // map: known map IDs override context.depth for artifact OOD checks.
        floor_id: "test.floor.depth-85".into(),
        depth: 85,
        source: LootSource::MonsterDeath {
            actor_id: "test.c4a-drop".into(),
        },
    };
    let item = (0..300_000)
        .find_map(|_| {
            game.generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == "demo.item.seiryu")
        })
        .expect("Seiryu must be reachable through the complete ordinary pool");
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    assert!(game.visible_item_resistances(&game.items[0]).is_empty());
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    game = restored;
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&id, None).unwrap();
    assert_eq!(game.carried_weight_tenths_pound(), 300);
    assert_eq!(game.item_base_modifiers("demo.item.seiryu").defense, 70);
    assert_eq!(game.player_equipment_bonuses().melee_skill, -3);
    assert_eq!(game.player_see_invisible_sources(), 1);
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    let elements = [
        DamageType::Acid,
        DamageType::Electricity,
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Poison,
    ];
    for element in elements {
        assert_eq!(
            game.effective_player_resistances().level(element),
            ResistanceLevel::Resistant
        );
        elemental_hit(&game, element, 10);
    }
    game.rng = RfbRng::seeded(ready_seed());
    let mut expected_rng = game.rng.clone();
    assert!(expected_rng.bounded(100) < 5);
    let duration = 21 + expected_rng.bounded(20) as u32;
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        activate(&mut game, &id, None),
        activate(&mut restored, &id, None)
    );
    assert_eq!(
        game.rng, expected_rng,
        "one device check and one shared duration roll"
    );
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(charge(&game, &id), 0);
    let status = game
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_BASIC_RESISTANCE)
        .unwrap();
    assert_eq!(status.remaining_ticks, duration);
    assert_eq!(status.granted_resistances.len(), 5);
    for element in elements {
        assert_eq!(
            game.effective_player_resistances().level(element),
            ResistanceLevel::Strong
        );
        elemental_hit(&game, element, 7);
    }
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );
    // The same opposition from overlapping statuses contributes once. A racial
    // vulnerability cancels one tier; immunity still wins.
    let mut overlapping = game.clone();
    let mut other = status.clone();
    other.kind_id = STATUS_THERMAL_RESISTANCE.into();
    overlapping.player.statuses.push(other);
    overlapping.items[0].location = ItemLocation::Inventory;
    assert_eq!(
        overlapping
            .effective_player_resistances()
            .level(DamageType::Fire),
        ResistanceLevel::Resistant
    );
    overlapping
        .player
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Vulnerable);
    assert_eq!(
        overlapping
            .effective_player_resistances()
            .level(DamageType::Fire),
        ResistanceLevel::Normal
    );
    overlapping.equip_inventory_item(&id, None).unwrap();
    assert_eq!(
        overlapping
            .effective_player_resistances()
            .level(DamageType::Fire),
        ResistanceLevel::Resistant
    );
    overlapping
        .player
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Immune);
    elemental_hit(&overlapping, DamageType::Fire, 0);
    let before = game.rng.clone();
    assert!(activate(&mut game, &id, None).contains(&DomainEvent::ItemUseUnavailable));
    assert_eq!(game.rng, before);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for run in [&mut game, &mut restored] {
        for tick in 1..=1110 {
            run.world_tick += 1;
            run.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new(), true)
                .unwrap();
            run.process_inventory_device_recovery(&mut Vec::new());
            if tick == duration {
                assert!(!run.player_has_status_kind(STATUS_BASIC_RESISTANCE));
                elemental_hit(run, DamageType::Fire, 10);
            }
            if tick == 1109 {
                assert_eq!(charge(run, &id), 0);
            }
        }
        assert_eq!(charge(run, &id), 1);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
    assert_eq!(
        game.generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap()
    );
    assert_eq!(game.rng, restored.rng);
    assert_ne!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.blue-dragon-scale-mail"), false),
        Some("demo.item.seiryu".into())
    );
}

#[test]
fn c4a_midnight_brand_darkness_and_acid_cone_preserve_saved_cooldown() {
    let mut game = Game::new_with_build(505, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for x in 9..=29 {
        for y in 8..=12 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-85".into(),
        depth: 85,
        source: LootSource::MonsterDeath {
            actor_id: "test.c4a-drop".into(),
        },
    };
    let base = "demo.item.black-dragon-scale-mail";
    let ordinary = (0..20_000)
        .find_map(|_| {
            game.generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == base && item.artifact_name.is_none())
        })
        .expect("the complete ordinary pool must first reach black dragon scale mail");
    assert!(ordinary.activation.is_some());
    // Condition on that observed base; retain all artifact candidates and gates.
    let selected = (0..20_000)
        .find_map(|_| {
            game.roll_fixed_artifact_kind_id(&context, Some(base), false)
                .filter(|kind| kind == "demo.item.midnight-dragon-scale-mail")
        })
        .unwrap();
    let draft = game.fixed_item_draft(&context, selected);
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    assert_eq!(
        game.carried_weight_tenths_pound(),
        0,
        "source sscanf ignores the trailing W field"
    );
    assert_eq!(
        game.content
            .item("demo.item.midnight-dragon-scale-mail")
            .unwrap()
            .base_value,
        200
    );
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    give_inventory_item(&mut game, "test.lantern", "demo.item.brass-lantern");
    game.equip_inventory_item("test.lantern", None).unwrap();
    game.glow.fill(false);
    game.reveal_current_visibility();
    assert_eq!(game.player_light_radius(), Some(2));
    let armor = game.player_derived_stats().armor_class.value;
    game.equip_inventory_item(&id, None).unwrap();
    // Derived armor uses tenths of AC.
    assert_eq!(game.player_derived_stats().armor_class.value - armor, 600);
    assert_eq!(
        (
            game.player_equipment_bonuses().melee_skill,
            game.player_equipment_bonuses().melee_damage
        ),
        (4, 4)
    );
    assert_eq!(game.player_light_radius(), Some(1));
    assert!(!game.is_visible(Position { x: 12, y: 10 }));
    give_inventory_item(&mut game, "test.sword", "demo.item.dagger");
    game.equip_inventory_item("test.sword", Some("right-hand"))
        .unwrap();
    for (name, kind, position) in [
        ("center", "great-hell-wyrm", Position { x: 20, y: 10 }),
        ("side", "great-hell-wyrm", Position { x: 26, y: 11 }),
        ("immune", "great-bile-wyrm", Position { x: 22, y: 10 }),
    ] {
        game.push_generated_actor(
            format!("test.{name}"),
            &format!("demo.actor.{kind}"),
            position,
        );
    }
    let profile = &game.player_melee_profiles(&game.player_derived_stats())[0];
    for (index, multiplier) in [(0, 24), (2, 10)] {
        assert_eq!(
            game.player_melee_damage_multiplier(
                profile,
                &game.entities[index],
                game.content.actor(&game.entities[index].kind_id).unwrap()
            ),
            multiplier
        );
    }
    game.reveal_current_visibility();
    let east = TargetSelection::Direction {
        direction: Direction::East,
    };
    // Successful check followed by direction cancellation spends the check only.
    game.rng = RfbRng::seeded(ready_seed());
    let mut expected = game.rng.clone();
    expected.bounded(100);
    activate(&mut game, &id, None);
    assert_eq!(game.rng, expected);
    assert_eq!(charge(&game, &id), 1);
    let failure = (0..1000)
        .find(|seed| (5..10).contains(&RfbRng::seeded(*seed).bounded(100)))
        .unwrap();
    game.rng = RfbRng::seeded(failure);
    assert!(
        activate(&mut game, &id, Some(&east))
            .iter()
            .any(|event| matches!(
                event,
                DomainEvent::DeviceSkillChecked {
                    succeeded: false,
                    ..
                }
            ))
    );
    assert_eq!(charge(&game, &id), 1);
    game.rng = RfbRng::seeded(ready_seed());
    let hp: Vec<_> = game.entities.iter().map(|actor| actor.hp).collect();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let mut blocked = game.clone();
    replace_terrain(&mut blocked, Position { x: 15, y: 10 }, "demo.terrain.wall");
    activate(&mut blocked, &id, Some(&east));
    assert_eq!(
        blocked
            .entities
            .iter()
            .map(|actor| actor.hp)
            .collect::<Vec<_>>(),
        hp
    );
    let events = activate(&mut game, &id, Some(&east));
    assert_eq!(activate(&mut restored, &id, Some(&east)), events);
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityConeDamage { resolution, .. }
        if resolution.base_raw_damage == 120 && resolution.radius == 2 && resolution.damage_type == DamageTypeDto::Acid)));
    assert_eq!(
        game.entities
            .iter()
            .zip(hp)
            .map(|(actor, hp)| hp - actor.hp)
            .collect::<Vec<_>>(),
        [120, 60, 0]
    );
    assert_eq!(game.state_hash(), restored.state_hash());
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for run in [&mut game, &mut restored] {
        for _ in 0..399 {
            run.world_tick += 1;
            run.process_inventory_device_recovery(&mut Vec::new());
        }
        assert_eq!(charge(run, &id), 0);
        run.world_tick += 1;
        run.process_inventory_device_recovery(&mut Vec::new());
        assert_eq!(charge(run, &id), 1);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(
        game.generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap()
    );
    assert_eq!(game.rng, restored.rng);
    assert_ne!(
        game.roll_fixed_artifact_kind_id(&context, Some(base), false),
        Some("demo.item.midnight-dragon-scale-mail".into())
    );
}

#[test]
fn c4a_plain_and_ego_scales_keep_base_breath_and_replay_after_cancel() {
    for (slug, element, damage) in [
        ("black", DamageTypeDto::Acid, 150),
        ("blue", DamageTypeDto::Electricity, 150),
    ] {
        for ego in [false, true] {
            let mut game = Game::new_with_build(506, "demo.build.warrior").unwrap();
            choose_human_talent_if_pending(&mut game);
            clear_monsters(&mut game);
            game.items.clear();
            let kind = format!("demo.item.{slug}-dragon-scale-mail");
            give_inventory_item(&mut game, "test.scale", &kind);
            if ego {
                let rolled = super::super::ego::materialize_ego_with_rng(
                    false,
                    &game.content,
                    &mut game.rng,
                    &kind,
                    vec!["rfb-legacy.affix.breath".into()],
                    |_| 85,
                    85,
                    2,
                );
                rolled.apply_to(&mut game.items[0]);
                assert_eq!(game.items[0].affix_ids, ["rfb-legacy.affix.breath"]);
            }
            let source = &game
                .content
                .item(&kind)
                .unwrap()
                .device_generation
                .as_ref()
                .unwrap()
                .activations[0];
            assert_eq!(
                game.items[0].activation.as_ref().unwrap().profile_id,
                source.id
            );
            game.equip_inventory_item("test.scale", None).unwrap();
            game.identify_item_instance("test.scale", ItemIdentificationRequest::new(true));
            game.rng = RfbRng::seeded(ready_seed());
            let mut expected = game.rng.clone();
            expected.bounded(100);
            activate(&mut game, "test.scale", None);
            assert_eq!(game.rng, expected);
            assert_eq!(charge(&game, "test.scale"), 1);
            game.rng = RfbRng::seeded(ready_seed());
            let mut restored = Game::from_save(game.to_save()).unwrap();
            let east = TargetSelection::Direction {
                direction: Direction::East,
            };
            let events = activate(&mut game, "test.scale", Some(&east));
            assert_eq!(activate(&mut restored, "test.scale", Some(&east)), events);
            assert!(events.iter().any(
                |event| matches!(event, DomainEvent::AbilityConeDamage { resolution, .. }
                if resolution.base_raw_damage == damage && resolution.damage_type == element)
            ));
            assert_eq!(charge(&game, "test.scale"), 0);
            assert_eq!(game.state_hash(), restored.state_hash());
        }
    }
}

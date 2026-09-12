// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn artifact_loot_context(depth: u16) -> LootContext {
    LootContext {
        table_id: "demo.loot-table.paladin".to_owned(),
        floor_id: format!("test.floor.depth-{depth}"),
        depth,
        source: LootSource::ItemUse {
            item_id: "test.item-generation".to_owned(),
        },
    }
}

#[test]
fn a10_hell_beast_natural_entry_drops_zero_rarity_artifact_once_after_save() {
    const BEAST: &str = "demo.actor.greater-hell-beast";
    const SHIRT: &str = "demo.item.legendary-lost-treasure";
    // Seed 54 reaches GHB through the unmodified formal entrance/floor/ecology.
    // Combat is shortened below; this is not a natural leveling test.
    let mut game = Game::new_with_build(54, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");
    let context = artifact_loot_context(100);
    let rng = game.rng.clone();
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.t-shirt"), false),
        None
    );
    assert_eq!(
        game.rng, rng,
        "QUESTITEM must be excluded before the zero rarity gate"
    );
    let mut beast = game
        .entities
        .iter()
        .find(|a| a.kind_id == BEAST)
        .unwrap()
        .clone();
    game.entities.retain(|a| a.id == beast.id);
    game.items.clear();
    let position = (1..game.height - 1)
        .find_map(|y| {
            (1..game.width - 2)
                .map(|x| Position {
                    x: i32::from(x),
                    y: i32::from(y),
                })
                .find(|p| game.is_walkable(*p) && game.is_walkable(Position { x: p.x + 1, y: p.y }))
        })
        .unwrap();
    game.player.position = position;
    beast.position = Position {
        x: position.x + 1,
        y: position.y,
    };
    beast.hp = 1;
    beast.energy_need = STANDARD_ACTION_COST;
    beast.nice = true;
    game.entities[0] = beast.clone();
    game.reveal_current_visibility();
    game = Game::from_save(game.to_save()).unwrap();
    let prepared = game.clone();
    game = (0..256)
        .find_map(|seed| {
            let mut attempt = prepared.clone();
            attempt.rng = RfbRng::seeded(seed);
            dispatch_next(
                &mut attempt,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            attempt
                .items
                .iter()
                .any(|item| item.kind_id == SHIRT)
                .then_some(attempt)
        })
        .expect("actual melee death must drop the named reward");
    assert_eq!(game.actor_kind_available_instance_count(BEAST), 0);
    let reward = game
        .items
        .iter()
        .find(|item| item.kind_id == SHIRT)
        .unwrap()
        .clone();
    assert!(game.generated_artifact_ids.contains(SHIRT));
    let ItemLocation::Ground(position) = reward.location else {
        panic!("reward must drop on the floor")
    };
    game.player.position = position;
    game.pick_up_item_at_player(Some(&reward.id)).unwrap();
    assert!(
        !game
            .item_property_knowledge
            .get(&reward.id)
            .is_some_and(|knowledge| knowledge.identified)
    );
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    restored.identify_item_instance(&reward.id, ItemIdentificationRequest::new(true));
    let before = restored.player_derived_stats().armor_class.value;
    restored
        .equip_inventory_item(&reward.id, Some("body"))
        .unwrap();
    assert_eq!(
        restored.player_derived_stats().armor_class.value - before,
        10
    );
    restored.reveal_current_visibility();
    let mut resumed = Game::from_save(restored.to_save()).unwrap();
    assert_eq!(resumed.state_hash(), restored.state_hash());
    assert_eq!(resumed.actor_kind_available_instance_count(BEAST), 0);
    // Source xtra2.c skips an already-generated reward; it does not replace it.
    // Exercise the duplicate guard directly because this unique cannot respawn.
    assert!(resumed.generate_death_loot(&beast).unwrap().0.is_empty());
    assert!(restored.generate_death_loot(&beast).unwrap().0.is_empty());
    assert_eq!(resumed.rng, restored.rng);
    assert_eq!(
        resumed
            .items
            .iter()
            .filter(|item| item.kind_id == SHIRT)
            .count(),
        1
    );
}

#[test]
fn ordinary_heavy_armor_allocation_reaches_equipment_and_save() {
    let mut game = Game::new_with_build(409, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 35,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    };
    // Controlled depth and repeated drops; the formal pool, weights, quality
    // rolls and materialization remain intact. This is not a leveling test.
    let mut remaining = BTreeSet::from([272, 274, 276, 277, 278, 279]);
    for _ in 0..10_000 {
        let generated = game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap();
        for item in generated {
            let definition = game.content.item(&item.kind_id).unwrap();
            let Some(base) = definition.rfb_base_kind else {
                continue;
            };
            if item.quality != ItemQualityDto::Ordinary || !remaining.remove(&base.source_index) {
                continue;
            }
            let defense = definition.modifiers.defense;
            let hit = definition.equipment_bonuses.melee_skill;
            let weight = u32::from(definition.weight_tenths_pound);
            let baseline = game.player_derived_stats();
            let id = item.id.clone();
            game.items.push(item);
            game.pick_up_item_at_player(Some(&id)).unwrap();
            assert_eq!(game.carried_weight_tenths_pound(), weight);
            game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
            assert!(game.item_property_knowledge[&id].identified);
            game.equip_inventory_item(&id, Some("body")).unwrap();
            let equipped = game.player_derived_stats();
            assert_eq!(
                equipped.armor_class.value - baseline.armor_class.value,
                defense * 10
            );
            assert_eq!(equipped.melee_skill.value - baseline.melee_skill.value, hit);
            game.reveal_current_visibility();
            let mut restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(restored.rng, game.rng);
            assert_eq!(
                restored.player_derived_stats().armor_class,
                equipped.armor_class
            );
            assert_eq!(restored.carried_weight_tenths_pound(), weight);
            let next = game
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap();
            let replay = restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap();
            assert_eq!(next, replay);
            assert_eq!(restored.rng, game.rng);
            game.items.clear();
        }
        if remaining.is_empty() {
            return;
        }
    }
    panic!("ordinary heavy armor was not generated: {remaining:?}");
}

#[test]
fn hobbit_fixed_artifacts_generate_equip_and_preserve_uniqueness_after_save() {
    let mut game = Game::new_with_build(413, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-20".into(),
        depth: 20,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    };
    let mut remaining = BTreeSet::from(["demo.item.sam", "demo.item.merry", "demo.item.pippin"]);
    // Repeated drops at controlled depth; keep the formal base pool, quality
    // rolls, rarity and uniqueness bookkeeping in the production path.
    for _ in 0..50_000 {
        let generated = game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap();
        for item in generated {
            if !remaining.remove(item.kind_id.as_str()) {
                continue;
            }
            assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
            let id = item.id.clone();
            game.items.push(item);
            game.pick_up_item_at_player(Some(&id)).unwrap();
            game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
            assert!(game.item_property_knowledge[&id].identified);
        }
        if remaining.is_empty() {
            break;
        }
    }
    assert!(
        remaining.is_empty(),
        "artifacts never generated: {remaining:?}"
    );
    assert_eq!(game.carried_weight_tenths_pound(), 30);
    for (kind, defense) in [("sam", 9), ("merry", 8), ("pippin", 8)] {
        let id = game
            .items
            .iter()
            .find(|item| item.kind_id == format!("demo.item.{kind}"))
            .unwrap()
            .id
            .clone();
        let before = game.equipment_modifiers();
        let bonuses = game.player_equipment_bonuses();
        let stats = game.player_derived_stats();
        game.equip_inventory_item(&id, None).unwrap();
        let after = game.equipment_modifiers();
        let equipped = game.player_derived_stats();
        assert_eq!(after.strength, before.strength - 1);
        assert_eq!(after.dexterity, before.dexterity + 1);
        assert_eq!(equipped.speed.value, stats.speed.value + 1);
        assert_eq!(after.defense, before.defense + defense);
        assert_eq!(
            game.player_equipment_bonuses().melee_skill,
            bonuses.melee_skill + 1
        );
        assert_eq!(
            game.player_equipment_bonuses().melee_damage,
            bonuses.melee_damage + 1
        );
        if kind == "merry" {
            assert_eq!(equipped.stealth_skill.value, stats.stealth_skill.value + 1);
            assert_eq!(equipped.search_skill.value, stats.search_skill.value + 5);
            assert_eq!(
                equipped.perception_skill.value,
                stats.perception_skill.value + 5
            );
            assert_eq!(
                game.effective_player_resistances()
                    .level(DamageType::Confusion),
                ResistanceLevel::Resistant
            );
        }
    }
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    assert_eq!(
        restored.player_derived_stats().armor_class,
        game.player_derived_stats().armor_class
    );
    assert_eq!(
        restored.player_derived_stats().search_skill,
        game.player_derived_stats().search_skill
    );
    let next = game
        .generate_loot_instances(&context, ItemLocation::Inventory)
        .unwrap();
    assert_eq!(
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        next
    );
    assert_eq!(restored.rng, game.rng);
    for (kind, base) in [
        ("sam", "hard-leather-cap"),
        ("merry", "cloak"),
        ("pippin", "leather-gloves"),
    ] {
        assert!(
            restored
                .generated_artifact_ids
                .contains(&format!("demo.item.{kind}"))
        );
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(
                &context,
                Some(&format!("demo.item.{base}")),
                false
            ),
            Some(format!("demo.item.{kind}"))
        );
    }
}

#[test]
fn galadriel_instant_generation_lighting_activation_and_cooldown_survive_save() {
    let mut game = Game::new_with_build(421, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 7..=13 {
        for x in 7..=13 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.glow.fill(false);
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-30".into(),
        depth: 30,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    };
    assert!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.phial"), false)
            .is_none()
    );
    // Controlled depth; keep the ordinary pool and its real 1/1000 instant gate.
    let mut found = None;
    for _ in 0..50_000 {
        for item in game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap()
        {
            assert_ne!(item.kind_id, "demo.item.phial");
            if item.kind_id == "demo.item.galadriel" {
                found = Some(item);
            }
        }
        if found.is_some() {
            break;
        }
    }
    let item = found.expect("Galadriel must occur through ordinary instant generation");
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert!(item.fuel.is_none());
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&id, None).unwrap();
    assert_eq!(game.player_light_radius(), Some(3));
    assert_eq!(game.player_equipment_bonuses().search_skill, 25);
    assert_eq!(game.player_equipment_bonuses().perception_skill, 25);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );
    let mut events = Vec::new();
    game.world_tick = 10;
    game.process_equipped_light_fuel(&mut events);
    assert!(game.items[0].fuel.is_none());
    assert_eq!(game.player_light_radius(), Some(3));
    let target = Position { x: 11, y: 10 };
    game.push_generated_actor("test.phial-target".into(), "demo.actor.goblin", target);
    let target_hp = game.entities[0].hp;
    game.world_tick = 0;
    for _ in 0..100 {
        game.use_inventory_item(
            &id,
            Some(&TargetSelection::SelfTarget),
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
    assert_eq!(game.items[0].device_recovery_progress, 0);
    assert!(
        game.entities
            .iter()
            .find(|actor| actor.id == "test.phial-target")
            .is_none_or(|actor| actor.hp < target_hp),
        "illumination must damage a light-vulnerable target"
    );
    assert!(game.glow[game.index(game.player.position).unwrap()]);
    assert!(game.glow[game.index(target).unwrap()]);
    let rng = game.rng.clone();
    assert!(
        !game
            .use_inventory_item(
                &id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new()
            )
            .unwrap()
    );
    assert_eq!(game.rng, rng);
    for tick in 1..=75 {
        game.world_tick = tick;
        game.process_inventory_device_recovery(&mut events);
    }
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.player_light_radius(), Some(3));
    assert_eq!(restored.items[0].device_recovery_progress, 75);
    assert!(
        restored
            .generated_artifact_ids
            .contains("demo.item.galadriel")
    );
    assert!(
        restored
            .roll_fixed_artifact_kind_id(&context, Some("demo.item.phial"), true)
            .is_none()
    );
    for tick in 76..150 {
        restored.world_tick = tick;
        restored.process_inventory_device_recovery(&mut events);
    }
    assert_eq!(restored.items[0].charges.unwrap().current, 0);
    assert_eq!(restored.items[0].device_recovery_progress, 149);
    restored.world_tick = 150;
    restored.process_inventory_device_recovery(&mut events);
    assert_eq!(restored.items[0].charges.unwrap().current, 1);
    assert_eq!(restored.items[0].device_recovery_progress, 0);
    let mut continued = Game::from_save(restored.to_save()).unwrap();
    let next = restored
        .generate_loot_instances(&context, ItemLocation::Inventory)
        .unwrap();
    assert_eq!(
        continued
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        next
    );
    assert_eq!(continued.rng, restored.rng);
    for _ in 0..100 {
        continued
            .use_inventory_item(
                &id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        if continued.items[0].charges.unwrap().current == 0 {
            break;
        }
    }
    assert_eq!(continued.items[0].charges.unwrap().current, 0);
}

#[test]
fn a1_instant_lights_generate_activate_and_resume_after_save() {
    fn activate(game: &mut Game, id: &str) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        for _ in 0..100 {
            game.use_inventory_item(
                id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            if game
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current
                == 0
            {
                return events;
            }
        }
        panic!("activation never succeeded: {id}");
    }

    for (slug, base, cooldown) in [
        ("star-of-elendil", "star", 250),
        ("stone-of-lore", "stone", 50),
    ] {
        let mut game = Game::new_with_build(423, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        descend_one_floor(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        for y in 7..=13 {
            for x in 7..=16 {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
            }
        }
        game.glow.fill(false);
        let wall = Position { x: 14, y: 10 };
        replace_terrain(&mut game, wall, "demo.terrain.wall");
        let remote = Position { x: 16, y: 10 };
        replace_terrain(&mut game, remote, "demo.terrain.wall");
        let remote_index = game.index(remote).unwrap();
        game.explored[remote_index] = false;
        game.revealed_terrain.remove(&remote);
        let context = artifact_loot_context(50);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        assert!(
            game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                .is_none()
        );
        // Controlled base/depth and repeated real rarity rolls. The existing
        // Galadriel test separately exercises the complete ordinary instant gate.
        let selected = (0..5000).find_map(|seed| {
            game.rng = RfbRng::seeded(seed);
            game.roll_fixed_artifact_kind_id(&context, Some(&base), true)
        });
        assert_eq!(selected.as_ref(), Some(&kind));
        let draft = game.fixed_item_draft(&context, selected.unwrap());
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        assert!(item.fuel.is_none());
        assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
        let id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        game = restored;
        give_inventory_item(&mut game, "test.carried", "demo.item.dagger");
        let before = game.equipment_modifiers();
        game.equip_inventory_item(&id, None).unwrap();
        let after = game.equipment_modifiers();
        assert_eq!(game.player_light_radius(), Some(3));
        if slug == "star-of-elendil" {
            assert_eq!(after.constitution - before.constitution, 1);
            assert_eq!(after.charisma - before.charisma, 1);
            assert_eq!(after.speed - before.speed, 1);
            assert_eq!(game.player_see_invisible_sources(), 1);
            assert_eq!(game.player_hold_life_sources(), 1);
            assert!(
                !game
                    .item_property_knowledge
                    .get("test.carried")
                    .is_some_and(|k| k.appraised)
            );
        } else {
            assert_eq!(after.intelligence - before.intelligence, 2);
            assert_eq!(after.wisdom - before.wisdom, 2);
            assert!(game.item_property_knowledge["test.carried"].appraised);
            assert!(!game.item_property_knowledge["test.carried"].identified);
        }
        let target = Position { x: 11, y: 10 };
        game.push_generated_actor("test.light-target".into(), "demo.actor.goblin", target);
        let hp = game.entities[0].hp;
        game.push_generated_actor(
            "test.blocked".into(),
            "demo.actor.sheep",
            Position { x: 15, y: 10 },
        );
        game.world_tick = 0;
        let events = activate(&mut game, &id);
        if slug == "star-of-elendil" {
            assert!(
                game.explored[remote_index],
                "mapping must cross an intervening wall"
            );
            assert!(game.glow[game.index(target).unwrap()]);
            assert!(
                game.entities
                    .iter()
                    .find(|a| a.id == "test.light-target")
                    .is_none_or(|a| a.hp < hp)
            );
        } else {
            let report = events
                .iter()
                .find_map(|event| match event {
                    DomainEvent::AbilityMonstersProbed { resolution, .. } => Some(resolution),
                    _ => None,
                })
                .expect("stone activation must use the full monster probe consumer");
            assert_eq!(report.monsters.len(), 1);
            assert_eq!(report.monsters[0].entity_id, "test.light-target");
            assert_eq!(report.monsters[0].hp, hp);
            assert_eq!(report.monsters[0].max_hp, game.entities[0].max_hp);
            assert!(game.probed_actor_kind_ids.contains("demo.actor.goblin"));
            assert!(!game.probed_actor_kind_ids.contains("demo.actor.sheep"));
        }
        let rng = game.rng.clone();
        assert!(
            !game
                .use_inventory_item(
                    &id,
                    Some(&TargetSelection::SelfTarget),
                    None,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new()
                )
                .unwrap()
        );
        assert_eq!(game.rng, rng);
        clear_monsters(&mut game);
        game.items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        let middle = cooldown / 2;
        for tick in 1..=middle {
            game.world_tick = tick;
            game.process_equipped_light_fuel(&mut Vec::new());
            game.process_inventory_device_recovery(&mut Vec::new());
        }
        assert!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .fuel
                .is_none()
        );
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        assert_eq!(
            u32::from(
                restored
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .device_recovery_progress
            ),
            middle
        );
        assert_eq!(restored.probed_actor_kind_ids, game.probed_actor_kind_ids);
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert!(
            restored
                .roll_fixed_artifact_kind_id(&context, Some(&base), true)
                .is_none()
        );
        if slug == "stone-of-lore" {
            // Existing floor consumer must keep working after restoring LORE2.
            give_inventory_item(&mut restored, "test.floor", "demo.item.long-sword");
            restored.items.last_mut().unwrap().location =
                ItemLocation::Ground(restored.player.position);
            restored.apply_player_floor_item_knowledge();
            restored.pick_up_item_at_player(Some("test.floor")).unwrap();
            assert!(restored.item_property_knowledge["test.floor"].appraised);
        }
        for tick in middle + 1..cooldown {
            restored.world_tick = tick;
            restored.process_inventory_device_recovery(&mut Vec::new());
        }
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            0
        );
        assert_eq!(
            u32::from(
                restored
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .device_recovery_progress
            ),
            cooldown - 1
        );
        restored.world_tick = cooldown;
        restored.process_inventory_device_recovery(&mut Vec::new());
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            1
        );
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .device_recovery_progress,
            0
        );
        restored.reveal_current_visibility();
        let mut continued = Game::from_save(restored.to_save()).unwrap();
        assert_eq!(activate(&mut restored, &id), activate(&mut continued, &id));
        assert_eq!(continued.state_hash(), restored.state_hash());
        assert_eq!(continued.rng, restored.rng);
        assert_eq!(
            continued
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(continued.rng, restored.rng);
        if slug == "stone-of-lore" {
            let slot = match &restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .location
            {
                ItemLocation::Equipped { slot_id } => slot_id.clone(),
                _ => panic!("stone must remain equipped"),
            };
            restored.unequip_slot(&slot).unwrap();
            give_inventory_item(&mut restored, "test.after-removal", "demo.item.short-sword");
            restored.items.last_mut().unwrap().location =
                ItemLocation::Ground(restored.player.position);
            restored.apply_player_floor_item_knowledge();
            assert!(
                !restored
                    .item_property_knowledge
                    .get("test.after-removal")
                    .is_some_and(|k| k.appraised)
            );
            assert!(restored.item_property_knowledge["test.carried"].appraised);
        }
    }
}

#[test]
fn a7_instant_lights_activate_with_source_parameters_and_resume_after_save() {
    fn use_light(game: &mut Game, id: &str, target: Option<&TargetSelection>) -> Vec<DomainEvent> {
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
    let success_seed = (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
        .unwrap();
    let failure_seed = (0..1000)
        .find(|seed| (5..10).contains(&RfbRng::seeded(*seed).bounded(100)))
        .unwrap();

    for (slug, base, cooldown, power) in [
        ("laputa", "levitation-stone", 2500, 60),
        ("stone-of-war", "stone-621", 1000, 8),
    ] {
        let mut game = Game::new_with_build(447, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        descend_one_floor(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        for y in 7..=15 {
            for x in 7..=18 {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
            }
        }
        let context = artifact_loot_context(60);
        game.glow.fill(true);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        assert!(
            game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                .is_none()
        );
        let selected = (0..5000).find_map(|seed| {
            game.rng = RfbRng::seeded(seed);
            game.roll_fixed_artifact_kind_id(&context, Some(&base), true)
        });
        assert_eq!(selected.as_ref(), Some(&kind));
        let draft = game.fixed_item_draft(&context, selected.unwrap());
        let light = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        let id = light.id.clone();
        assert!(light.fuel.is_none());
        assert_eq!(light.activation.as_ref().unwrap().power, power);
        assert_eq!(
            light.activation.as_ref().unwrap().device_check_difficulty,
            i32::from(power)
        );
        game.items.push(light);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        game = restored;
        give_inventory_item(&mut game, "test.a7-bow", "demo.item.short-bow");
        game.equip_inventory_item("test.a7-bow", None).unwrap();
        let before = game.player_derived_stats();
        let before_modifiers = game.equipment_modifiers();
        let before_shot = game.player_projectile_profile().unwrap();
        game.equip_inventory_item(&id, None).unwrap();
        assert_eq!(game.player_light_radius(), Some(3));
        if slug == "laputa" {
            let after = game.equipment_modifiers();
            assert_eq!(after.intelligence - before_modifiers.intelligence, 2);
            assert_eq!(after.charisma - before_modifiers.charisma, 2);
            assert_eq!(after.speed - before_modifiers.speed, 2);
            assert!(game.player_levitates());
            assert_eq!(game.player_hold_life_sources(), 1);
        } else {
            let after = game.player_derived_stats();
            assert_eq!(after.melee_skill.value - before.melee_skill.value, 6);
            assert_eq!(
                after.melee_damage_bonus.value - before.melee_damage_bonus.value,
                6
            );
            let after_shot = game.player_projectile_profile().unwrap();
            assert_eq!(after_shot.to_hit - before_shot.to_hit, 6);
            assert_eq!(
                after_shot.launcher_to_damage - before_shot.launcher_to_damage,
                6
            );
            assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
        }
        let center = Position { x: 14, y: 10 };
        let target = if slug == "laputa" {
            TargetSelection::Position { position: center }
        } else {
            TargetSelection::SelfTarget
        };
        game.world_tick = 0;
        game.reveal_current_visibility();
        // Malformed target modes are rejected before any device check.
        let invalid = if slug == "laputa" {
            TargetSelection::SelfTarget
        } else {
            TargetSelection::Direction {
                direction: Direction::East,
            }
        };
        let hash = game.state_hash();
        let rng = game.rng.clone();
        use_light(&mut game, &id, Some(&invalid));
        assert_eq!(game.state_hash(), hash);
        assert_eq!(game.rng, rng);
        let mut failed = game.clone();
        failed.rng = RfbRng::seeded(failure_seed);
        let events = use_light(&mut failed, &id, Some(&target));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::DeviceSkillChecked {
                succeeded: false,
                ..
            }
        )));
        assert_eq!(
            failed
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            1
        );
        assert_eq!(
            failed
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .device_recovery_progress,
            0
        );
        assert!(
            failed
                .player
                .statuses
                .iter()
                .all(|status| status.kind_id != STATUS_BERSERK)
        );
        if slug == "laputa" {
            let mut cancelled = game.clone();
            cancelled.rng = RfbRng::seeded(success_seed);
            let rng = cancelled.rng.clone();
            let mut waiting = cancelled.clone();
            dispatch_next(&mut waiting, GameCommand::Wait);
            let update = dispatch_next(
                &mut cancelled,
                GameCommand::UseItem {
                    item_id: id.clone(),
                    target: None,
                },
            );
            assert_eq!(cancelled.world_tick, waiting.world_tick);
            assert_ne!(cancelled.rng, rng);
            assert_eq!(
                cancelled
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .charges
                    .unwrap()
                    .current,
                1
            );
            assert_eq!(
                cancelled
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .device_recovery_progress,
                0
            );
            assert!(update.events.iter().all(|event| !matches!(
                event.outcome,
                Some(GameEventOutcomeDto::AbilityAreaDamage { .. })
            )));
            for (actor_id, position) in [
                ("test.a7-center", center),
                ("test.a7-near", Position { x: 14, y: 11 }),
                ("test.a7-edge", Position { x: 14, y: 12 }),
                ("test.a7-outside", Position { x: 14, y: 13 }),
                ("test.a7-blocked", Position { x: 16, y: 10 }),
            ] {
                game.push_generated_actor(
                    actor_id.into(),
                    "demo.actor.ancient-multi-hued-dragon",
                    position,
                );
            }
            replace_terrain(&mut game, Position { x: 15, y: 10 }, "demo.terrain.wall");
            game.reveal_current_visibility();
            // All three supported target selections reach the same real blast.
            for selection in [
                target.clone(),
                TargetSelection::Direction {
                    direction: Direction::East,
                },
                TargetSelection::Entity {
                    entity_id: "test.a7-center".into(),
                },
            ] {
                let mut shot = game.clone();
                shot.rng = RfbRng::seeded(success_seed);
                let events = use_light(&mut shot, &id, Some(&selection));
                let blast = events
                    .iter()
                    .find_map(|event| match event {
                        DomainEvent::AbilityAreaDamage { resolution, .. } => Some(resolution),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(
                    (
                        blast.center,
                        blast.radius,
                        blast.base_raw_damage,
                        blast.damage_type
                    ),
                    (center, 2, 300, DamageTypeDto::Mana)
                );
                for (actor_id, damage) in [
                    ("test.a7-center", 300),
                    ("test.a7-near", 150),
                    ("test.a7-edge", 100),
                    ("test.a7-outside", 0),
                    ("test.a7-blocked", 0),
                ] {
                    let hp = game
                        .entities
                        .iter()
                        .find(|actor| actor.id == actor_id)
                        .unwrap()
                        .hp;
                    assert_eq!(
                        shot.entities
                            .iter()
                            .find(|actor| actor.id == actor_id)
                            .unwrap()
                            .hp,
                        hp - damage
                    );
                }
            }
            clear_monsters(&mut game);
        }
        let max_hp = game.player_derived_stats().max_hp.value;
        game.player.hp = (max_hp - 20).max(1);
        let hp = game.player.hp;
        game.rng = RfbRng::seeded(success_seed);
        use_light(&mut game, &id, Some(&target));
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            0
        );
        if slug == "stone-of-war" {
            assert_eq!(
                game.player.hp,
                (hp + 30).min(game.player_derived_stats().max_hp.value)
            );
            let berserk = game
                .player
                .statuses
                .iter()
                .find(|status| status.kind_id == STATUS_BERSERK)
                .unwrap();
            assert!((260..=500).contains(&berserk.remaining_ticks));
            assert!(game.player_status_immunities().contains(STATUS_FEAR));
        }
        let rng = game.rng.clone();
        use_light(&mut game, &id, Some(&target));
        assert_eq!(game.rng, rng);
        clear_monsters(&mut game);
        game.items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        let middle = cooldown / 2;
        for tick in 1..=middle {
            game.world_tick = tick;
            game.process_equipped_light_fuel(&mut Vec::new());
            game.process_inventory_device_recovery(&mut Vec::new());
        }
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        assert_eq!(restored.player.statuses, game.player.statuses);
        assert!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .fuel
                .is_none()
        );
        assert_eq!(
            u32::from(
                restored
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .device_recovery_progress
            ),
            middle
        );
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert!(
            restored
                .roll_fixed_artifact_kind_id(&context, Some(&base), true)
                .is_none()
        );
        for tick in middle + 1..cooldown {
            restored.world_tick = tick;
            restored.process_inventory_device_recovery(&mut Vec::new());
        }
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            0
        );
        assert_eq!(
            u32::from(
                restored
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .device_recovery_progress
            ),
            cooldown - 1
        );
        restored.world_tick = cooldown;
        restored.process_inventory_device_recovery(&mut Vec::new());
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            1
        );
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .device_recovery_progress,
            0
        );
        restored.reveal_current_visibility();
        restored.rng = RfbRng::seeded(success_seed);
        let mut continued = Game::from_save(restored.to_save()).unwrap();
        assert_eq!(
            use_light(&mut restored, &id, Some(&target)),
            use_light(&mut continued, &id, Some(&target))
        );
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .charges
                .unwrap()
                .current,
            0
        );
        assert_eq!(continued.state_hash(), restored.state_hash());
        assert_eq!(continued.rng, restored.rng);
        assert_eq!(
            continued
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(continued.rng, restored.rng);
    }
}

#[test]
fn a8_artifacts_generate_activate_and_resume_source_cooldowns_after_save() {
    fn activate(game: &mut Game, id: &str) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.use_inventory_item(
            id,
            Some(&TargetSelection::Direction {
                direction: Direction::East,
            }),
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        events
    }
    let success = (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
        .unwrap();
    let failure = (0..1000)
        .find(|seed| (5..10).contains(&RfbRng::seeded(*seed).bounded(100)))
        .unwrap();
    for (slug, base, cooldown, power, dice, sides, bonus, element, beam) in [
        (
            "cammithrim",
            "leather-gloves",
            20,
            10,
            4,
            6,
            0,
            DamageType::Physical,
            false,
        ),
        (
            "paurhach",
            "set-of-gauntlets",
            120,
            12,
            0,
            0,
            35,
            DamageType::Fire,
            true,
        ),
        (
            "pauraegen",
            "set-of-gauntlets",
            120,
            12,
            0,
            0,
            40,
            DamageType::Electricity,
            false,
        ),
        (
            "paurnen",
            "set-of-gauntlets",
            120,
            12,
            8,
            8,
            0,
            DamageType::Acid,
            false,
        ),
        (
            "narthanc",
            "dagger",
            120,
            4,
            6,
            8,
            0,
            DamageType::Fire,
            false,
        ),
        (
            "nimthanc",
            "dagger",
            120,
            3,
            6,
            8,
            0,
            DamageType::Cold,
            false,
        ),
        (
            "dethanc",
            "dagger",
            120,
            5,
            0,
            0,
            24,
            DamageType::Electricity,
            true,
        ),
    ] {
        let mut game = Game::new_with_build(448, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        descend_one_floor(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        for y in 7..=13 {
            for x in 7..=18 {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
            }
        }
        game.glow.fill(true);
        let context = artifact_loot_context(30);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        // Select the requested kind through real rarity rolls, not the first
        // result: the dagger and gauntlet bases have other artifact candidates.
        assert!((0..10_000).any(|seed| {
            game.rng = RfbRng::seeded(seed);
            game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                .as_ref()
                == Some(&kind)
        }));
        let draft = game.fixed_item_draft(&context, kind.clone());
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        let id = item.id.clone();
        assert_eq!(item.activation.as_ref().unwrap().power, power);
        assert_eq!(
            item.activation.as_ref().unwrap().device_check_difficulty,
            i32::from(power)
        );
        assert_eq!(
            item.rolled_affixes.len(),
            usize::from(base == "demo.item.dagger")
        );
        let rolled = item.rolled_affixes.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        game = restored;
        game.equip_inventory_item(&id, None).unwrap();
        assert_eq!(
            game.player_equipment_bonuses().light_radius,
            i32::from(matches!(slug, "cammithrim" | "narthanc" | "dethanc"))
        );
        if slug == "cammithrim" {
            assert_eq!(game.equipment_modifiers().defense, 11);
            assert!(game.player_sustains_attribute(AttributeKind::Constitution));
            assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
        } else if base == "demo.item.dagger" {
            let profile = game.item_throw_profile(&game.items[0]).unwrap();
            assert_eq!(
                (
                    profile.damage.dice,
                    profile.damage.sides,
                    profile.to_hit,
                    profile.to_damage
                ),
                (2, 5, 8, 12)
            );
            assert_eq!(game.equipment_modifiers().speed, 1);
            game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
            assert_eq!(
                game.visible_item_throw_profile(&game.items[0]),
                Some(profile)
            );
        } else {
            assert_eq!(game.equipment_modifiers().defense, 9);
        }
        game.world_tick = 0;
        // Explicit invalid modes cost nothing. Cancelled direction and failed
        // checks spend a turn but retain the charge and do not start recovery.
        game.reveal_current_visibility();
        let hash = game.state_hash();
        let rng = game.rng.clone();
        game.use_inventory_item(
            &id,
            Some(&TargetSelection::SelfTarget),
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.state_hash(), hash);
        assert_eq!(game.rng, rng);
        for (target, seed) in [
            (None, success),
            (
                Some(TargetSelection::Direction {
                    direction: Direction::East,
                }),
                failure,
            ),
        ] {
            let mut attempt = game.clone();
            attempt.rng = RfbRng::seeded(seed);
            let mut waiting = attempt.clone();
            waiting.rng.bounded(100);
            dispatch_next(&mut waiting, GameCommand::Wait);
            dispatch_next(
                &mut attempt,
                GameCommand::UseItem {
                    item_id: id.clone(),
                    target,
                },
            );
            assert_eq!(attempt.world_tick, waiting.world_tick);
            assert_eq!(attempt.rng, waiting.rng, "{slug} seed {seed}");
            assert_eq!(attempt.items[0].charges.unwrap().current, 1);
            assert_eq!(attempt.items[0].device_recovery_progress, 0);
        }
        for (id, x) in [
            ("test.a8-near", 12),
            ("test.a8-far", 14),
            ("test.a8-blocked", 16),
        ] {
            game.push_generated_actor(id.into(), "demo.actor.sheep", Position { x, y: 10 });
            let actor = game.entities.last_mut().unwrap();
            actor.hp = 10_000;
            actor.max_hp = 10_000;
        }
        replace_terrain(&mut game, Position { x: 15, y: 10 }, "demo.terrain.wall");
        game.rng = RfbRng::seeded(success);
        let mut expected = game.clone();
        expected.rng.bounded(100);
        let raw = expected.roll_damage(dice, sides) + bonus;
        let events = activate(&mut game, &id);
        if slug == "pauraegen" {
            assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityAreaDamage { resolution, .. } if resolution.radius == 2 && resolution.base_raw_damage == 40 && resolution.damage_type == DamageTypeDto::Electricity)));
            assert_eq!(game.entities[0].hp, 9960);
            assert_eq!(game.entities[1].hp, 9986); // (40 + distance 2) / 3
        } else {
            let hits = events
                .iter()
                .filter_map(|event| match event {
                    DomainEvent::ItemActivationHit { damage, .. } => Some(damage),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(hits.len(), if beam { 2 } else { 1 });
            assert!(
                hits.iter()
                    .all(|damage| damage.raw == raw && damage.damage_type == element)
            );
            assert_eq!(game.entities[1].hp < 10_000, beam);
        }
        assert_eq!(game.entities[2].hp, 10_000);
        assert_eq!(game.items[0].charges.unwrap().current, 0);
        let rng = game.rng.clone();
        activate(&mut game, &id);
        assert_eq!(game.rng, rng);
        clear_monsters(&mut game);
        for tick in 1..=cooldown / 2 {
            game.world_tick = tick;
            game.process_inventory_device_recovery(&mut Vec::new());
        }
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.items[0].rolled_affixes, rolled);
        assert_eq!(
            u32::from(restored.items[0].device_recovery_progress),
            cooldown / 2
        );
        for tick in cooldown / 2 + 1..cooldown {
            restored.world_tick = tick;
            restored.process_inventory_device_recovery(&mut Vec::new());
        }
        assert_eq!(restored.items[0].charges.unwrap().current, 0);
        restored.world_tick = cooldown;
        restored.process_inventory_device_recovery(&mut Vec::new());
        assert_eq!(restored.items[0].charges.unwrap().current, 1);
        assert_eq!(restored.items[0].device_recovery_progress, 0);
        restored.rng = RfbRng::seeded(success);
        restored.reveal_current_visibility();
        let mut continued = Game::from_save(restored.to_save()).unwrap();
        assert_eq!(activate(&mut restored, &id), activate(&mut continued, &id));
        assert_eq!(continued.items[0].charges.unwrap().current, 0);
        assert_eq!(continued.state_hash(), restored.state_hash());
        assert_eq!(continued.rng, restored.rng);
        assert_eq!(
            continued
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(continued.rng, restored.rng);
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(&context, Some(&base), false),
            Some(kind)
        );
    }
}

#[test]
fn a8_gauntlet_brands_reach_both_weapons_but_not_ammunition() {
    fn strike(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        events
    }
    for (slug, element) in [
        ("paurhach", DamageType::Fire),
        ("pauraegen", DamageType::Electricity),
        ("paurnen", DamageType::Acid),
        ("set-of-gauntlets-paurnimmen", DamageType::Cold),
    ] {
        let mut game = Game::new_with_build(449, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        replace_terrain(&mut game, Position { x: 10, y: 10 }, "demo.terrain.floor");
        let target = Position { x: 11, y: 10 };
        replace_terrain(&mut game, target, "demo.terrain.floor");
        for (id, kind, slot) in [
            ("test.a8-right", "dagger", "right-hand"),
            ("test.a8-left", "dagger", "left-hand"),
        ] {
            give_inventory_item(&mut game, id, &format!("demo.item.{kind}"));
            game.equip_inventory_item(id, Some(slot)).unwrap();
        }
        give_inventory_item(&mut game, "test.a8-bow", "demo.item.short-bow");
        game.equip_inventory_item("test.a8-bow", None).unwrap();
        let before = game.player_projectile_profile().unwrap();
        let kind = format!("demo.item.{slug}");
        give_inventory_item(&mut game, "test.a8-gloves", &kind);
        game.register_generated_artifact(&kind);
        game.equip_inventory_item("test.a8-gloves", None).unwrap();
        let after = game.player_projectile_profile().unwrap();
        assert_eq!(
            (
                after.to_hit - before.to_hit,
                after.launcher_to_damage - before.launcher_to_damage
            ),
            (2, 2)
        );
        game.push_generated_actor(
            "test.a8-target".into(),
            "demo.actor.greater-hell-beast",
            target,
        );
        game.entities[0].hp = 1500;
        game.entities[0].max_hp = 1500;
        game.entities[0]
            .resistances
            .set(element, ResistanceLevel::Normal);
        let profiles = game.player_melee_profiles(&game.player_derived_stats());
        assert_eq!(profiles.len(), 2);
        for profile in &profiles {
            assert_eq!(
                game.player_melee_damage_multiplier(
                    profile,
                    &game.entities[0],
                    game.content.actor("demo.actor.greater-hell-beast").unwrap()
                ),
                24
            );
        }
        assert_eq!(
            game.player_projectile_damage_multiplier(
                &after,
                &game.entities[0],
                game.content.actor("demo.actor.greater-hell-beast").unwrap()
            ),
            10
        );
        let seed = (0..1000)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                strike(&mut trial);
                trial.entities[0].hp < 1500
            })
            .unwrap();
        let mut damage = Vec::new();
        for resistance in [ResistanceLevel::Normal, ResistanceLevel::Immune] {
            let mut trial = game.clone();
            trial.entities[0].resistances.set(element, resistance);
            trial.rng = RfbRng::seeded(seed);
            trial.reveal_current_visibility();
            let mut restored = Game::from_save(trial.to_save()).unwrap();
            assert_eq!(strike(&mut restored), strike(&mut trial));
            assert_eq!(restored.state_hash(), trial.state_hash());
            assert_eq!(restored.rng, trial.rng);
            damage.push(1500 - trial.entities[0].hp);
        }
        assert!(damage[0] > damage[1] && damage[1] > 0);
    }
}

#[test]
fn a8_thrown_daggers_use_their_own_brands_and_keep_random_properties_after_pickup() {
    fn throw(game: &mut Game, id: &str) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.throw_inventory_item(
            id,
            Direction::East,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        events
    }
    let seed = (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
        .unwrap();
    for (slug, element, glove) in [
        ("narthanc", DamageType::Fire, "set-of-gauntlets-paurnimmen"),
        ("nimthanc", DamageType::Cold, "paurhach"),
        ("dethanc", DamageType::Electricity, "paurnen"),
    ] {
        let mut game = Game::new_with_build(450, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        let target = Position { x: 11, y: 10 };
        replace_terrain(&mut game, Position { x: 10, y: 10 }, "demo.terrain.floor");
        replace_terrain(&mut game, target, "demo.terrain.floor");
        let context = artifact_loot_context(30);
        let kind = format!("demo.item.{slug}");
        let draft = game.fixed_item_draft(&context, kind.clone());
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Inventory)
            .unwrap();
        let id = item.id.clone();
        assert_eq!(item.rolled_affixes.len(), 1);
        let rolled = item.rolled_affixes.clone();
        game.items.push(item);
        give_inventory_item(&mut game, "test.a8-ordinary", "demo.item.dagger");
        let ordinary = game.item_throw_parameters(&game.items[1]);
        let thrown = game.item_throw_parameters(&game.items[0]);
        assert!(thrown.0 > ordinary.0 && thrown.1 > ordinary.1);
        let glove_kind = format!("demo.item.{glove}");
        give_inventory_item(&mut game, "test.a8-gloves", &glove_kind);
        game.register_generated_artifact(&glove_kind);
        game.equip_inventory_item("test.a8-gloves", None).unwrap();
        game.push_generated_actor(
            "test.a8-target".into(),
            "demo.actor.greater-hell-beast",
            target,
        );
        game.entities[0].hp = 1500;
        game.entities[0].max_hp = 1500;
        for (resistance, multiplier) in
            [(ResistanceLevel::Normal, 24), (ResistanceLevel::Immune, 10)]
        {
            let mut trial = game.clone();
            trial.entities[0].resistances.set(element, resistance);
            trial.rng = RfbRng::seeded(seed);
            let mut expected = trial.clone();
            expected.rng.bounded(100);
            let raw = (expected.roll_damage(2, 5) * multiplier / 10 + 12) * thrown.1 / 100;
            trial.reveal_current_visibility();
            let mut restored = Game::from_save(trial.to_save()).unwrap();
            let events = throw(&mut trial, &id);
            assert_eq!(throw(&mut restored, &id), events);
            assert!(events.iter().any(|event| matches!(event, DomainEvent::ItemThrowHit { damage, .. } if damage.raw == raw)));
            assert_eq!(restored.state_hash(), trial.state_hash());
            assert_eq!(restored.rng, trial.rng);
            let thrown = restored.items.iter().find(|item| item.id == id).unwrap();
            assert_eq!(thrown.rolled_affixes, rolled);
            let ItemLocation::Ground(position) = thrown.location else {
                panic!("thrown artifact must land")
            };
            // Step onto the dropped dagger only after vacating the living target.
            restored.entities[0].position = restored.player.position;
            restored.player.position = position;
            restored.pick_up_item_at_player(Some(&id)).unwrap();
            restored.reveal_current_visibility();
            let recovered = Game::from_save(restored.to_save()).unwrap();
            assert_eq!(recovered.state_hash(), restored.state_hash());
            let item = recovered.items.iter().find(|item| item.id == id).unwrap();
            assert_eq!(item.rolled_affixes, rolled);
            assert_eq!(item.location, ItemLocation::Inventory);
            assert!(recovered.generated_artifact_ids.contains(&kind));
        }
    }
}

#[test]
fn a9_ordinary_identity_artifacts_generate_fight_and_preserve_their_scope_after_save() {
    fn strike(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        let target = game
            .entities
            .iter()
            .position(|actor| actor.id != "test.a9-mount")
            .unwrap();
        game.resolve_player_melee(
            target,
            false,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        events
    }
    for (slug, base, build, dice, sides, hit, damage, weight) in [
        ("xiaolong", "nunchaku", "warrior", 4, 4, 20, 10, 30),
        ("dragonlance", "heavy-lance", "cavalry", 6, 10, 2, 17, 500),
    ] {
        let mut game = Game::new_with_build(451, &format!("demo.build.{build}")).unwrap();
        choose_human_talent_if_pending(&mut game);
        descend_one_floor(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        let adjacent = Position { x: 11, y: 10 };
        replace_terrain(&mut game, Position { x: 10, y: 10 }, "demo.terrain.floor");
        replace_terrain(&mut game, adjacent, "demo.terrain.floor");
        game.glow.fill(true);
        let context = artifact_loot_context(60);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        let selected = (0..5000)
            .find_map(|seed| {
                game.rng = RfbRng::seeded(seed);
                game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                    .filter(|id| id == &kind)
            })
            .expect("ordinary class must generate the artifact through its real rarity gate");
        let draft = game.fixed_item_draft(&context, selected);
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
        assert!(item.activation.is_none() && item.charges.is_none() && item.curse.is_none());
        assert_eq!(game.item_instance_weight(&item), weight);
        let id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        game = restored;
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        game.equip_inventory_item(&id, Some("right-hand")).unwrap();
        let item_profile = game.item_melee_profile(&game.items[0]).unwrap();
        assert_eq!(
            (
                item_profile.damage.dice,
                item_profile.damage.sides,
                item_profile.to_hit,
                item_profile.to_damage
            ),
            (dice, sides, hit, damage)
        );
        assert_eq!(
            game.content
                .item(&kind)
                .unwrap()
                .weapon_proficiency_base_item_id
                .as_deref(),
            Some(base.as_str())
        );
        if slug == "xiaolong" {
            // Monk-only BLOWS is added by source artifact.c at generation time;
            // the ordinary warrior receives neither it nor extra-attack state.
            assert!(!game.item_has_rfb_flag(&game.items[0], "BLOWS"));
            let bonuses = game.player_equipment_bonuses();
            assert_eq!(
                (bonuses.melee_attacks, bonuses.melee_attacks_delta_percent),
                (0, 0)
            );
            assert_eq!(bonuses.stealth_skill, 3);
            let modifiers = game.equipment_modifiers();
            assert_eq!((modifiers.speed, modifiers.defense), (3, 10));
            for attribute in [
                AttributeKind::Strength,
                AttributeKind::Dexterity,
                AttributeKind::Constitution,
            ] {
                assert!(game.player_sustains_attribute(attribute));
            }
            assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
            assert_eq!(
                game.effective_player_resistances().level(DamageType::Fire),
                ResistanceLevel::Resistant
            );
            game.player.hp = 1;
            game.world_tick = 9;
            let update = dispatch_next(&mut game, GameCommand::Wait);
            assert!(
                update
                    .events
                    .iter()
                    .any(|event| event.message_key == "equipment-regenerated")
            );
            assert!(game.player.hp > 1);
        } else {
            // Dragon-pact-only additions must not leak to an ordinary Cavalry.
            for flag in ["SLAY_EVIL", "SLAY_DEMON", "SLAY_UNDEAD"] {
                assert!(!game.item_has_rfb_flag(&game.items[0], flag));
            }
            let modifiers = game.equipment_modifiers();
            assert_eq!((modifiers.strength, modifiers.charisma), (4, 4));
            for element in [
                DamageType::Acid,
                DamageType::Electricity,
                DamageType::Fire,
                DamageType::Cold,
                DamageType::Poison,
            ] {
                assert_eq!(
                    game.effective_player_resistances().level(element),
                    ResistanceLevel::Resistant
                );
            }
            assert!(game.item_has_weapon_trait(&game.items[0], WeaponTraitDto::Blessed));
            let mut cursed = game.clone();
            let seed = (0..10_000)
                .find(|seed| {
                    RfbRng::seeded(*seed).bounded(888) + 1 > 100
                        && RfbRng::seeded(*seed).bounded(100) >= 50
                })
                .unwrap();
            cursed.rng = RfbRng::seeded(seed);
            let mut expected = cursed.rng.clone();
            expected.bounded(888);
            assert!(
                cursed
                    .curse_equipped_item(CurseEquippedItemRequest::new(
                        EquippedItemCurseTarget::Weapon
                    ))
                    .resisted
            );
            assert_eq!(cursed.rng, expected);
            assert!(cursed.items[0].curse.is_none());
        }
        game.push_generated_actor(
            "test.a9-target".into(),
            "demo.actor.baby-blue-dragon",
            adjacent,
        );
        game.entities[0].hp = 10_000;
        game.entities[0].max_hp = 10_000;
        let on_foot = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!((on_foot.damage_dice, on_foot.damage_sides), (dice, sides));
        if slug == "dragonlance" {
            game.push_generated_actor(
                "test.a9-mount".into(),
                "demo.actor.horse",
                game.player.position,
            );
            game.entities[1].controller_id = Some(game.player.id.clone());
            game.riding_actor_id = Some("test.a9-mount".into());
            let mounted = game.player_melee_profile(&game.player_derived_stats());
            assert_eq!(mounted.to_hit, on_foot.to_hit + 15);
            assert_eq!((mounted.damage_dice, mounted.damage_sides), (8, 10));
        }
        let profile = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!(profile.source_item_id.as_deref(), Some(id.as_str()));
        assert_eq!(
            game.player_melee_damage_multiplier(
                &profile,
                &game.entities[0],
                game.content.actor("demo.actor.baby-blue-dragon").unwrap()
            ),
            if slug == "xiaolong" { 28 } else { 56 }
        );
        // Non-dragon targets distinguish the ordinary branch from the missing
        // pact slays; fire immunity isolates Xiaolong's brand from its slays.
        let mut ordinary = game.clone();
        clear_monsters(&mut ordinary);
        ordinary.riding_actor_id = None;
        ordinary.push_generated_actor("test.a9-ordinary".into(), "demo.actor.goblin", adjacent);
        ordinary.entities[0].hp = 10_000;
        ordinary.entities[0].max_hp = 10_000;
        let ordinary_profile = ordinary.player_melee_profile(&ordinary.player_derived_stats());
        assert_eq!(
            ordinary.player_melee_damage_multiplier(
                &ordinary_profile,
                &ordinary.entities[0],
                ordinary.content.actor("demo.actor.goblin").unwrap()
            ),
            if slug == "xiaolong" { 24 } else { 10 }
        );
        if slug == "xiaolong" {
            let seed = (0..1000)
                .find(|seed| {
                    let mut trial = ordinary.clone();
                    trial.rng = RfbRng::seeded(*seed);
                    strike(&mut trial);
                    trial.entities[0].hp < 10_000
                })
                .unwrap();
            let mut damages = Vec::new();
            for resistance in [ResistanceLevel::Normal, ResistanceLevel::Immune] {
                let mut trial = ordinary.clone();
                trial.entities[0]
                    .resistances
                    .set(DamageType::Fire, resistance);
                trial.rng = RfbRng::seeded(seed);
                strike(&mut trial);
                damages.push(10_000 - trial.entities[0].hp);
            }
            assert!(damages[0] > damages[1] && damages[1] > 0);
        }
        let seed = (0..1000)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                strike(&mut trial);
                trial.entities[0].hp < 10_000
            })
            .unwrap();
        // Restore the target's source HP bounds before exercising saved combat.
        game.entities[0].max_hp = game
            .content
            .actor(&game.entities[0].kind_id)
            .unwrap()
            .max_hp;
        game.entities[0].hp = 1;
        game.rng = RfbRng::seeded(seed);
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.riding_actor_id, game.riding_actor_id);
        assert_eq!(strike(&mut restored), strike(&mut game));
        assert!(
            restored
                .entities
                .iter()
                .all(|actor| actor.id != "test.a9-target")
        );
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        assert_eq!(
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            game.generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(restored.rng, game.rng);
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(&context, Some(&base), false),
            Some(kind)
        );
    }
}

#[test]
fn a2_armor_generation_equipment_consumers_and_uniqueness_survive_save() {
    for (slug, base, defense) in [
        ("thengel", "metal-cap", 15),
        ("perseus", "small-metal-shield", 20),
        ("bard", "soft-leather-boots", 20),
        ("fell-rider", "hard-leather-cap", 0),
        ("nightcap", "knit-cap", 7),
        ("four-winds", "knit-cap", 13),
    ] {
        let mut game = Game::new_with_build(424, "demo.build.mage-life-arcane").unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        // Controlled level gives Thengel a measurable INT-based mana baseline;
        // its WIS/CHR bonuses do not themselves change this Mage's maximum.
        game.apply_player_experience(game.experience_required_for_level(30), &mut Vec::new());
        game.refresh_player_resource_maxima();
        let mana_before = game.resources["demo.resource.mana"].maximum;
        let context = artifact_loot_context(30);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        // Keep source-ordered candidates and real rarity draws, controlling only
        // the base/depth. Complete ordinary-pool entry has existing shared tests.
        let selected = (0..5000)
            .find_map(|seed| {
                game.rng = RfbRng::seeded(seed);
                game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                    .filter(|id| id == &kind)
            })
            .expect("each A2 armor must occur in its ordinary fixed-artifact candidate pool");
        let draft = game.fixed_item_draft(&context, selected);
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
        assert_eq!(item.intrinsic_properties, Default::default());
        assert!(item.activation.is_none() && item.charges.is_none());
        assert!(
            item.curse.is_none(),
            "negative AC/CHR and darkness are not removable curses"
        );
        let id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        game = restored;
        game.equip_inventory_item(&id, None).unwrap();
        game.refresh_player_resource_maxima();
        let modifiers = game.equipment_modifiers();
        assert_eq!(modifiers.defense, defense);
        match slug {
            "thengel" => {
                assert_eq!((modifiers.wisdom, modifiers.charisma), (3, 3));
                assert!(mana_before > 0);
                assert_eq!(
                    game.resources["demo.resource.mana"].maximum,
                    mana_before * 115 / 100
                );
            }
            "perseus" => {
                assert_eq!(modifiers.strength, 2);
                assert!(game.player_reflects_bolts());
                let before = game.progress.attributes.strength;
                let rng = game.rng.clone();
                game.resolve_monster_attribute_drain(AttributeKind::Strength);
                assert_eq!(game.progress.attributes.strength, before);
                assert_eq!(game.rng, rng);
            }
            "bard" => {
                assert_eq!((modifiers.dexterity, modifiers.charisma), (4, 4));
                assert_eq!(game.player_equipment_bonuses().stealth_skill, 4);
                assert!(
                    game.player_status_immunities()
                        .contains("rfb.status.paralysis")
                );
                assert_eq!(
                    game.effective_player_resistances()
                        .level(DamageType::Poison),
                    ResistanceLevel::Resistant
                );
            }
            "fell-rider" => {
                assert_eq!((modifiers.wisdom, modifiers.charisma), (3, -3));
                let before = game.progress.attributes.intelligence;
                let rng = game.rng.clone();
                game.resolve_monster_attribute_drain(AttributeKind::Intelligence);
                assert_eq!(game.progress.attributes.intelligence, before);
                assert_eq!(game.rng, rng);
                assert_eq!(
                    game.effective_player_resistances().level(DamageType::Light),
                    ResistanceLevel::Resistant
                );
            }
            "nightcap" => {
                assert_eq!(game.player_equipment_bonuses().stealth_skill, 3);
                assert_eq!(game.player_equipment_bonuses().light_radius, -1);
                assert_eq!(game.player_hold_life_sources(), 1);
                assert_eq!(
                    game.effective_player_resistances().level(DamageType::Fear),
                    ResistanceLevel::Resistant
                );
            }
            "four-winds" => {
                for element in [
                    DamageType::Acid,
                    DamageType::Electricity,
                    DamageType::Fire,
                    DamageType::Cold,
                ] {
                    assert_eq!(
                        game.effective_player_resistances().level(element),
                        ResistanceLevel::Resistant
                    );
                }
            }
            _ => unreachable!(),
        }
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(
            restored.player_derived_stats().armor_class,
            game.player_derived_stats().armor_class
        );
        assert_eq!(
            restored.player_derived_stats().stealth_skill,
            game.player_derived_stats().stealth_skill
        );
        assert_eq!(
            restored.resources["demo.resource.mana"].maximum,
            game.resources["demo.resource.mana"].maximum
        );
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(&context, Some(&base), false),
            Some(kind)
        );
        let mut continued = Game::from_save(restored.to_save()).unwrap();
        assert_eq!(
            continued
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(continued.rng, restored.rng);
        let slot = match &restored.items[0].location {
            ItemLocation::Equipped { slot_id } => slot_id.clone(),
            _ => panic!("generated armor must remain equipped"),
        };
        restored.unequip_slot(&slot).unwrap();
        restored.refresh_player_resource_maxima();
        assert_eq!(restored.equipment_modifiers(), Default::default());
        if slug == "thengel" {
            assert_eq!(
                restored.resources["demo.resource.mana"].maximum,
                mana_before
            );
        }
        assert!(!restored.player_reflects_bolts());
    }
}

#[test]
fn a3_weapons_generate_fight_and_preserve_equipment_and_uniqueness_after_save() {
    fn strike(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        let target = game
            .entities
            .iter()
            .position(|actor| actor.id != "test.a3-mount")
            .unwrap();
        game.resolve_player_melee(
            target,
            false,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        events
    }

    for (slug, base, dice, sides, hit, damage, target_kind, multiplier) in [
        ("osondir", "halberd", 3, 6, 8, 12, "skeleton-human", 28),
        ("til-i-arc", "pike", 2, 6, 12, 15, "stone-troll", 28),
        ("eorlingas", "lance", 3, 10, 3, 21, "snaga", 28),
        ("barukkheled", "broad-axe", 2, 7, 13, 19, "stone-troll", 28),
        ("bloodspike", "morning-star", 2, 7, 8, 22, "sheep", 24),
        ("nar-i-vagil", "quarterstaff", 1, 10, 10, 20, "sheep", 24),
        ("samson", "club", 3, 5, 8, 10, "skeleton-human", 28),
        ("vagabond", "morning-star", 2, 7, 16, 15, "sheep", 10),
    ] {
        let mut game = Game::new_with_build(426, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        descend_one_floor(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        let adjacent = Position { x: 11, y: 10 };
        replace_terrain(&mut game, Position { x: 10, y: 10 }, "demo.terrain.floor");
        replace_terrain(&mut game, adjacent, "demo.terrain.floor");
        game.glow.fill(true);
        let context = artifact_loot_context(35);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        // Controlled base/depth, unchanged source-ordered candidates and rarity.
        // Existing fixed_weapon_pair tests exercise the complete ordinary pool.
        let selected = (0..5000)
            .find_map(|seed| {
                game.rng = RfbRng::seeded(seed);
                game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                    .filter(|id| id == &kind)
            })
            .expect("each A3 weapon must occur in its ordinary fixed-artifact pool");
        let draft = game.fixed_item_draft(&context, selected);
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
        assert_eq!(item.intrinsic_properties, Default::default());
        assert!(item.activation.is_none() && item.curse.is_none());
        let id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        game = restored;
        game.equip_inventory_item(&id, Some("right-hand")).unwrap();
        let profile = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!(profile.source_item_id.as_deref(), Some(id.as_str()));
        assert_eq!((profile.damage_dice, profile.damage_sides), (dice, sides));
        assert_eq!(profile.to_hit, hit);
        assert!(profile.to_damage >= damage);
        match slug {
            "osondir" => {
                assert_eq!(game.equipment_modifiers().charisma, 3);
                assert_eq!(game.player_light_radius(), Some(1));
                replace_terrain(&mut game, adjacent, "demo.terrain.dark-pit");
                dispatch_next(
                    &mut game,
                    GameCommand::Move {
                        direction: Direction::East,
                    },
                );
                assert_eq!(game.player.position, adjacent);
                game.player.position = Position { x: 10, y: 10 };
                replace_terrain(&mut game, adjacent, "demo.terrain.floor");
            }
            "til-i-arc" => {
                assert_eq!(game.equipment_modifiers().intelligence, 2);
                assert_eq!(game.equipment_modifiers().defense, 10);
                assert!(game.player_sustains_attribute(AttributeKind::Intelligence));
                assert!(game.player_slow_digestion());
                assert_eq!(game.player_light_radius(), Some(1));
            }
            "eorlingas" => {
                let stats = game.equipment_modifiers();
                assert_eq!((stats.strength, stats.dexterity, stats.charisma), (3, 3, 3));
                assert!(
                    game.player_status_immunities()
                        .contains("rfb.status.paralysis")
                );
            }
            "barukkheled" => assert_eq!(game.equipment_modifiers().constitution, 3),
            "bloodspike" => {
                assert_eq!(game.equipment_modifiers().strength, 4);
                assert_eq!(
                    game.effective_player_resistances().level(DamageType::Nexus),
                    ResistanceLevel::Resistant
                );
            }
            "nar-i-vagil" => {
                assert_eq!(game.equipment_modifiers().intelligence, 3);
                assert_eq!(game.player_light_radius(), Some(1));
            }
            "samson" => {
                let stats = game.equipment_modifiers();
                assert_eq!((stats.strength, stats.constitution), (3, 3));
            }
            "vagabond" => {
                assert_eq!(game.equipment_modifiers().constitution, 4);
                assert!(game.player_sustains_attribute(AttributeKind::Constitution));
                assert!(game.player_slow_digestion());
                assert_eq!(game.player_hold_life_sources(), 1);
                // Choose a draw that resists via BLESSED but would fail the
                // artifact-only 50% gate; this proves the extra consumer runs.
                let seed = (0..10_000)
                    .find(|seed| {
                        RfbRng::seeded(*seed).bounded(888) + 1 > 100
                            && RfbRng::seeded(*seed).bounded(100) >= 50
                    })
                    .unwrap();
                game.rng = RfbRng::seeded(seed);
                let mut expected = game.rng.clone();
                expected.bounded(888);
                assert!(
                    game.curse_equipped_item(CurseEquippedItemRequest::new(
                        EquippedItemCurseTarget::Weapon
                    ))
                    .resisted
                );
                assert_eq!(game.rng, expected);
                assert!(game.items[0].curse.is_none());
                game.player.hp = 1;
                game.world_tick = 0;
                let update = dispatch_next(&mut game, GameCommand::Wait);
                assert!(
                    update
                        .events
                        .iter()
                        .any(|e| e.message_key == "equipment-regenerated")
                );
                assert!(game.player.hp > 1);
            }
            _ => unreachable!(),
        }
        game.push_generated_actor(
            "test.a3-target".into(),
            &format!("demo.actor.{target_kind}"),
            adjacent,
        );
        game.entities[0].hp = 10_000;
        game.entities[0].max_hp = 10_000;
        if slug == "eorlingas" {
            // Controlled mount, using the real lance consumer: +15 hit and
            // exactly two extra dice, so this artifact becomes 5d10.
            game.push_generated_actor(
                "test.a3-mount".into(),
                "demo.actor.horse",
                game.player.position,
            );
            game.entities[1].controller_id = Some(game.player.id.clone());
            game.riding_actor_id = Some("test.a3-mount".into());
            let mounted = game.player_melee_profile(&game.player_derived_stats());
            assert_eq!(
                (mounted.to_hit, mounted.damage_dice, mounted.damage_sides),
                (18, 5, 10)
            );
        }
        let profile = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!(
            game.player_melee_damage_multiplier(
                &profile,
                &game.entities[0],
                game.content.actor(&game.entities[0].kind_id).unwrap()
            ),
            multiplier
        );
        if slug == "til-i-arc" {
            // Dual brands do not stack. One remaining nonimmune element is
            // sufficient; both immunities leave only the target's slay tier.
            let mut elemental = game.clone();
            clear_monsters(&mut elemental);
            elemental.push_generated_actor("test.elemental".into(), "demo.actor.goblin", adjacent);
            elemental.entities[0].hp = 10_000;
            elemental.entities[0].max_hp = 10_000;
            let mut damages = Vec::new();
            for (fire, cold, expected) in [
                (ResistanceLevel::Resistant, ResistanceLevel::Resistant, 24),
                (ResistanceLevel::Immune, ResistanceLevel::Resistant, 24),
                (ResistanceLevel::Resistant, ResistanceLevel::Immune, 24),
                (ResistanceLevel::Immune, ResistanceLevel::Immune, 10),
            ] {
                elemental.entities[0]
                    .resistances
                    .set(DamageType::Fire, fire);
                elemental.entities[0]
                    .resistances
                    .set(DamageType::Cold, cold);
                assert_eq!(
                    elemental.player_melee_damage_multiplier(
                        &profile,
                        &elemental.entities[0],
                        elemental.content.actor("demo.actor.goblin").unwrap()
                    ),
                    expected
                );
                let actual = (0..100)
                    .find_map(|seed| {
                        let mut trial = elemental.clone();
                        trial.rng = RfbRng::seeded(seed);
                        strike(&mut trial)
                            .into_iter()
                            .map(DomainEvent::into_dto)
                            .find_map(|event| match event.outcome {
                                Some(GameEventOutcomeDto::Damage { resolution }) => {
                                    assert!(trial.entities[0].hp < 10_000);
                                    Some(resolution.raw_damage)
                                }
                                _ => None,
                            })
                    })
                    .expect("seed range includes an elemental weapon hit");
                damages.push(actual);
            }
            assert_eq!(damages[0], damages[1]);
            assert_eq!(damages[1], damages[2]);
            assert!(damages[2] > damages[3]);
        }
        // Search a successful real hit, then replay that same attack after save.
        let seed = (0..100)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                strike(&mut trial);
                trial.entities[0].hp < 10_000
            })
            .expect("seed range must include a real weapon hit");
        // Restore the target's source HP bounds before exercising saved combat.
        game.entities[0].max_hp = game
            .content
            .actor(&game.entities[0].kind_id)
            .unwrap()
            .max_hp;
        game.entities[0].hp = 1;
        game.rng = RfbRng::seeded(seed);
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(strike(&mut restored), strike(&mut game));
        assert!(
            restored
                .entities
                .iter()
                .all(|actor| actor.id != "test.a3-target")
        );
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert_eq!(
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            game.generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(restored.rng, game.rng);
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(&context, Some(&base), false),
            Some(kind)
        );
    }
}

#[test]
fn terror_mask_generation_uses_current_build_and_preserves_identity_after_save() {
    use rfb_protocol::ItemCurseEffectDto;
    for (build, favored) in [
        ("warrior", true),
        ("cavalry", true),
        ("berserker", true),
        ("duelist", false),
        ("archer", false),
        ("sniper", false),
        ("high-mage-death", false),
        ("high-mage-craft", false),
        ("mage-life-arcane", false),
        ("ranger-nature-sorcery", false),
        ("paladin-death", false),
        ("mindcrafter", false),
    ] {
        let mut game = Game::new_with_build(422, &format!("demo.build.{build}")).unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: "test.floor.depth-50".into(),
            depth: 50,
            source: LootSource::MonsterDeath {
                actor_id: "test.ordinary-drop".into(),
            },
        };
        // Controlled depth, complete ordinary pool: profession never excludes the mask.
        let mut found = None;
        for _ in 0..50_000 {
            for item in game
                .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
                .unwrap()
            {
                if item.kind_id == "demo.item.terror-mask" {
                    found = Some(item);
                }
            }
            if found.is_some() {
                break;
            }
        }
        let item = found.unwrap_or_else(|| panic!("mask never generated for {build}"));
        assert_eq!(item.affix_ids.len(), usize::from(favored));
        assert_eq!(item.rolled_affixes.len(), usize::from(favored));
        assert_eq!(
            item.curse,
            (!favored).then_some(ItemCurseSeverityDto::Heavy)
        );
        assert_eq!(item.intrinsic_curse_effects.len(), usize::from(!favored));
        assert_eq!(game.item_has_rfb_flag(&item, "TY_CURSE"), !favored);
        assert_eq!(game.item_has_rfb_flag(&item, "AGGRAVATE"), !favored);
        let id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(game.visible_item_resistances(&game.items[0]).is_empty());
        game.reveal_current_visibility();
        let unknown = Game::from_save(game.to_save()).unwrap();
        assert_eq!(unknown.state_hash(), game.state_hash());
        let rolled = game.items[0].clone();
        let rng = game.rng.clone();
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        assert_eq!(game.rng, rng);
        game.equip_inventory_item(&id, None).unwrap();
        game.refresh_player_resource_maxima();
        assert_eq!(
            game.items[0].intrinsic_properties,
            rolled.intrinsic_properties
        );
        assert!(game.player_has_anti_magic());
        assert_eq!(game.player_has_equipped_aggravation(), !favored);
        assert!(game.player_has_equipped_curse_effect(ItemCurseEffectDto::Teleport));
        assert_eq!(
            game.player_has_equipped_curse_effect(ItemCurseEffectDto::TyCurse),
            !favored
        );
        if build == "berserker" {
            let before = game.items[0].clone();
            let rng = game.rng.clone();
            assert_eq!(
                game.inventory_item_dto(&before)
                    .use_unavailable_reason
                    .as_deref(),
                Some("berserker")
            );
            game.use_inventory_item(
                &id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(game.items[0], before);
            assert_eq!(game.rng, rng);
        }
        assert_eq!(game.equipment_modifiers().intelligence, -3);
        assert_eq!(game.equipment_modifiers().wisdom, -3);
        assert_eq!(game.equipment_modifiers().charisma, 3);
        assert_eq!(game.player_equipment_bonuses().melee_skill, 18);
        assert_eq!(game.player_equipment_bonuses().melee_damage, 18);
        // Duelist AC also changes with the mask's INT penalty; inspect the
        // actual item contribution separately from that class calculation.
        assert_eq!(
            game.player_derived_stats()
                .armor_class
                .contributions
                .iter()
                .filter(|entry| entry.source_id == id)
                .map(|entry| entry.amount)
                .sum::<i32>(),
            150
        );
        for element in [
            DamageType::Acid,
            DamageType::Cold,
            DamageType::Poison,
            DamageType::Disenchant,
        ] {
            assert_eq!(
                game.effective_player_resistances().level(element),
                ResistanceLevel::Resistant
            );
        }
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.items[0], game.items[0]);
        assert!(
            restored
                .generated_artifact_ids
                .contains("demo.item.terror-mask")
        );
        let next = game
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap();
        assert_eq!(
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            next
        );
        assert_eq!(restored.rng, game.rng);
        assert!(
            restored
                .roll_fixed_artifact_kind_id(&context, Some("demo.item.iron-helm"), false)
                .is_none()
        );
        let slot = match &restored.items[0].location {
            ItemLocation::Equipped { slot_id } => slot_id.clone(),
            _ => unreachable!(),
        };
        assert_eq!(restored.unequip_slot(&slot).is_some(), favored);
        if build == "archer" {
            // The actual cursed instance reaches the periodic TY_CURSE consumer.
            // Select its rare mana-blast boundary; ordinary curse scheduling stays intact.
            let mut doomed = restored.clone();
            let seed = (1..100_000)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    rng.bounded(200) == 0 && matches!(rng.bounded(34) + 1, 30 | 31)
                })
                .unwrap();
            doomed.rng = RfbRng::seeded(seed);
            doomed.player.hp = 1;
            doomed.world_tick = 10;
            doomed
                .process_equipped_curse_effects(
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            assert!(doomed.player_is_dead());
            restored.remove_equipped_curses(RemoveEquippedCursesRequest::new(false));
            assert_eq!(restored.items[0].curse, Some(ItemCurseSeverityDto::Heavy));
            restored.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
            assert!(restored.items[0].curse.is_none());
            // Dispel the detachable curse, not the artifact's intrinsic bad flags.
            assert!(restored.player_has_equipped_curse_effect(ItemCurseEffectDto::TyCurse));
            assert!(restored.player_has_equipped_aggravation());
            assert!(restored.unequip_slot(&slot).is_some());
            assert!(!restored.player_has_equipped_curse_effect(ItemCurseEffectDto::TyCurse));
            assert!(!restored.player_has_equipped_aggravation());
        }
        if build == "mage-life-arcane" {
            // Keep the generated, cursed mask; only level/book availability is controlled.
            restored.apply_player_experience(
                restored.experience_required_for_level(20),
                &mut Vec::new(),
            );
            give_inventory_item(
                &mut restored,
                "test.arcane-book",
                "demo.item.cantrips-for-beginners",
            );
            let spell = "demo.ability.arcane-detect-monsters";
            restored
                .study_player_ability("test.arcane-book", spell)
                .unwrap();
            let mana = "demo.resource.mana";
            let masked_maximum = restored.resources[mana].maximum;
            restored.resources.get_mut(mana).unwrap().current = masked_maximum;
            let mut caster = Game::from_save(restored.to_save()).unwrap();
            assert_eq!(caster.state_hash(), restored.state_hash());
            let before = caster.state_hash();
            let mut events = Vec::new();
            caster
                .resolve_player_ability(
                    spell,
                    TargetSelection::SelfTarget,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            assert!(matches!(events.as_slice(),
                [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "anti-magic"
            ));
            assert_eq!(
                caster.state_hash(),
                before,
                "rejection spends no mana or RNG"
            );
            assert!(
                caster
                    .snapshot()
                    .player
                    .abilities
                    .iter()
                    .any(|ability| { ability.id == spell && !ability.can_cast })
            );
            caster.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
            assert!(caster.player_has_anti_magic());
            assert!(caster.player_has_equipped_aggravation());
            assert!(caster.unequip_slot(&slot).is_some());
            caster.refresh_player_resource_maxima();
            assert!(!caster.player_has_anti_magic());
            assert!(!caster.player_has_equipped_aggravation());
            assert!(caster.resources[mana].maximum > masked_maximum);
            assert_eq!(caster.resources[mana].current, masked_maximum);
            assert!(
                caster
                    .snapshot()
                    .player
                    .abilities
                    .iter()
                    .any(|ability| { ability.id == spell && ability.can_cast })
            );
            caster.debug_ability_casts_succeed = true;
            events.clear();
            caster
                .resolve_player_ability(
                    spell,
                    TargetSelection::SelfTarget,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::AbilityDetected { .. }))
            );
            assert!(caster.resources[mana].current < masked_maximum);
        }
    }
}

#[test]
fn terror_mask_duplicate_power_and_resistance_do_not_reroll() {
    let mut game = Game::new_with_build(422, "demo.build.warrior").unwrap();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 50,
        source: LootSource::MonsterDeath {
            actor_id: "test.drop".into(),
        },
    };
    let seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            for _ in 0..3 {
                rng.bounded(1);
            } // Existing activation/profile initialization.
            rng.bounded(10) == 6 && rng.bounded(12) == 0
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut expected = game.rng.clone();
    for _ in 0..3 {
        expected.bounded(1);
    }
    expected.bounded(10);
    expected.bounded(12);
    let draft = game.fixed_item_draft(&context, "demo.item.terror-mask".into());
    assert_eq!(game.rng, expected);
    assert_eq!(
        draft.intrinsic_properties.status_immunities,
        ["rfb.status.paralysis"]
    );
    assert_eq!(draft.rolled_affixes[0].properties.resistances.len(), 1);
    assert_eq!(
        draft.rolled_affixes[0].properties.resistances[&ActorDamageType::Poison],
        rfb_content::ActorResistanceLevel::Resistant
    );
    assert!(draft.curse.is_none());
}

#[test]
fn terror_mask_melee_only_bonuses_and_fear_activation_restore() {
    let mut game = Game::new_with_build(423, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 9..=11 {
        for x in 9..=12 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.glow.fill(true);
    give_inventory_item(&mut game, "test.bow", "demo.item.short-bow");
    game.equip_inventory_item("test.bow", None).unwrap();
    let before = game.player_projectile_profile().unwrap();
    let melee_before = game.player_derived_stats();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 50,
        source: LootSource::MonsterDeath {
            actor_id: "test.drop".into(),
        },
    };
    // Focused equipment/activation check; natural generation is covered above.
    let draft = game.fixed_item_draft(&context, "demo.item.terror-mask".into());
    let mut item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    item.enchantments.to_hit = 1;
    item.enchantments.to_damage = 2;
    let id = item.id.clone();
    game.items.push(item);
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&id, None).unwrap();
    let after = game.player_projectile_profile().unwrap();
    assert_eq!(
        (after.to_hit, after.launcher_to_damage),
        (before.to_hit, before.launcher_to_damage)
    );
    assert_eq!(
        game.player_derived_stats().melee_skill.value - melee_before.melee_skill.value,
        19
    );
    assert_eq!(
        game.player_derived_stats().melee_damage_bonus.value
            - melee_before.melee_damage_bonus.value,
        20
    );
    game.push_generated_actor(
        "test.mask-target".into(),
        "demo.actor.goblin",
        Position { x: 11, y: 10 },
    );
    game.reveal_current_visibility();
    let mut events = Vec::new();
    game.world_tick = 0;
    for _ in 0..100 {
        game.use_inventory_item(
            &id,
            Some(&TargetSelection::SelfTarget),
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.items[1].charges.unwrap().current == 0 {
            break;
        }
    }
    assert_eq!(game.items[1].charges.unwrap().current, 0);
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == "rfb.status.fear")
    );
    let rng = game.rng.clone();
    assert!(
        !game
            .use_inventory_item(
                &id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new()
            )
            .unwrap()
    );
    assert_eq!(game.rng, rng);
    for tick in 1..=500 {
        game.world_tick = tick;
        game.process_inventory_device_recovery(&mut events);
    }
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for tick in 501..1000 {
        restored.world_tick = tick;
        restored.process_inventory_device_recovery(&mut events);
    }
    let item = restored.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(item.charges.unwrap().current, 0);
    assert_eq!(item.device_recovery_progress, 999);
    restored.world_tick = 1000;
    restored.process_inventory_device_recovery(&mut events);
    let item = restored.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(item.charges.unwrap().current, 1);
    assert_eq!(item.device_recovery_progress, 0);
}

#[test]
fn fixed_high_resistance_uses_one_source_roll_without_retrying_duplicates() {
    let original = Game::new_with_build(419, "demo.build.warrior").unwrap();
    let context = artifact_loot_context(40);
    // artifact.c::one_high_resistance: randint0(12), including existing flags.
    let source_order = [
        ActorDamageType::Poison,
        ActorDamageType::Light,
        ActorDamageType::Dark,
        ActorDamageType::Shards,
        ActorDamageType::Blindness,
        ActorDamageType::Confusion,
        ActorDamageType::Sound,
        ActorDamageType::Nether,
        ActorDamageType::Nexus,
        ActorDamageType::Chaos,
        ActorDamageType::Disenchant,
        ActorDamageType::Fear,
    ];
    for kind in ["rohirrim", "thorin", "celegorm", "anarion", "thror"] {
        for (index, element) in source_order.into_iter().enumerate() {
            let seed = (0..10_000)
                .find(|seed| RfbRng::seeded(*seed).bounded(12) == index as u64)
                .unwrap();
            let mut game = original.clone();
            game.rng = RfbRng::seeded(seed);
            let mut expected_rng = game.rng.clone();
            expected_rng.bounded(12);
            let kind_id = format!("demo.item.{kind}");
            let draft = game.fixed_item_draft(&context, kind_id.clone());
            assert_eq!(game.rng, expected_rng);
            assert_eq!(draft.rolled_affixes.len(), 1);
            let properties = &draft.rolled_affixes[0].properties;
            assert_eq!(properties.resistances.len(), 1);
            assert_eq!(
                properties.resistances[&element],
                rfb_content::ActorResistanceLevel::Resistant
            );
            if game
                .content
                .item(&kind_id)
                .unwrap()
                .resistances
                .contains_key(&element)
            {
                // A duplicate remains the same flag: no reroll or stronger resistance.
                game.items.clear();
                let item = game
                    .commit_generated_item_draft(draft, ItemLocation::Inventory)
                    .unwrap();
                let id = item.id.clone();
                game.items.push(item);
                game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
                game.equip_inventory_item(&id, None).unwrap();
                let item = &game.items[0];
                let visible = game.visible_item_resistances(item);
                assert_eq!(
                    visible
                        .iter()
                        .filter(|resistance| resistance.damage_type
                            == DamageTypeDto::from(DamageType::from(element)))
                        .count(),
                    1
                );
                assert_eq!(
                    game.effective_player_resistances().level(element.into()),
                    ResistanceLevel::Resistant
                );
            }
        }
    }
}

#[test]
fn a4_high_resistance_armor_generates_equips_and_preserves_rolls_after_save() {
    for (slug, base, weight) in [
        ("thorin", "small-metal-shield", 65),
        ("celegorm", "large-leather-shield", 60),
        ("anarion", "large-metal-shield", 120),
        ("thror", "mithril-shod-boots", 80),
    ] {
        let mut game = Game::new_with_build(426, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        let context = artifact_loot_context(70);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        // Control base/depth, retaining the ordinary source-ordered candidates
        // and real rarity draws. Full ordinary-pool generation is tested below.
        let selected = (0..5000)
            .find_map(|seed| {
                game.rng = RfbRng::seeded(seed);
                game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                    .filter(|id| id == &kind)
            })
            .expect("each A4 armor must occur in its ordinary fixed-artifact candidate pool");
        let draft = game.fixed_item_draft(&context, selected);
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        assert_eq!(
            item.affix_ids,
            ["rfb-legacy.affix.artifact-extra-high-resistance"]
        );
        assert_eq!(item.rolled_affixes.len(), 1);
        assert_eq!(item.rolled_affixes[0].properties.resistances.len(), 1);
        assert!(item.activation.is_none() && item.charges.is_none() && item.curse.is_none());
        let rolled = item.rolled_affixes.clone();
        let (&element, _) = rolled[0].properties.resistances.iter().next().unwrap();
        let id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert_eq!(game.carried_weight_tenths_pound(), weight);
        assert!(game.visible_item_resistances(&game.items[0]).is_empty());
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        assert_eq!(restored.items[0].rolled_affixes, rolled);
        game = restored;
        let rng = game.rng.clone();
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        assert_eq!(game.rng, rng);
        assert!(
            game.item_property_knowledge[&id]
                .known_affix_ids
                .contains(&rolled[0].affix_id)
        );
        assert!(
            game.visible_item_resistances(&game.items[0])
                .iter()
                .any(|r| r.damage_type == DamageTypeDto::from(DamageType::from(element)))
        );
        game.equip_inventory_item(&id, None).unwrap();
        assert_eq!(game.equipment_modifiers().defense, 26);
        assert_eq!(
            game.effective_player_resistances().level(element.into()),
            ResistanceLevel::Resistant
        );
        for (&damage_type, &level) in &game.content.item(&kind).unwrap().resistances {
            assert_eq!(
                game.effective_player_resistances()
                    .level(damage_type.into()),
                ResistanceLevel::from(level)
            );
        }
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.items[0].rolled_affixes, rolled);
        assert!(restored.generated_artifact_ids.contains(&kind));
        // Exercise the actual consumers after restoration, while the original
        // remains available to compare the next production generation and RNG.
        let mut equipped = restored.clone();
        let modifiers = equipped.equipment_modifiers();
        let slot = match &equipped.items[0].location {
            ItemLocation::Equipped { slot_id } => slot_id.clone(),
            _ => panic!("generated armor must remain equipped"),
        };
        match slug {
            "thorin" => {
                assert_eq!((modifiers.strength, modifiers.constitution), (4, 4));
                assert!(
                    equipped
                        .player_status_immunities()
                        .contains(STATUS_PARALYSIS)
                );
                let hp = equipped.player.hp;
                let result = equipped.resolve_monster_damage_to_player(
                    "test.acid",
                    "demo.actor.small-kobold",
                    "test.acid-bolt",
                    0,
                    30,
                    30,
                    DamageType::Acid,
                    &mut Vec::new(),
                );
                assert!(
                    matches!(result, AbilityEffectResolutionDto::Damage { resolution, .. }
                    if resolution.final_damage == 0)
                );
                assert_eq!(equipped.player.hp, hp);
                equipped.unequip_slot(&slot).unwrap();
                assert_eq!(
                    equipped
                        .effective_player_resistances()
                        .level(DamageType::Acid),
                    ResistanceLevel::Normal
                );
                assert!(
                    !equipped
                        .player_status_immunities()
                        .contains(STATUS_PARALYSIS)
                );
            }
            "celegorm" => {
                // RES_COLD is absent in the source and not a high-resistance candidate.
                assert_eq!(
                    equipped
                        .effective_player_resistances()
                        .level(DamageType::Cold),
                    ResistanceLevel::Normal
                );
            }
            "anarion" => {
                let attributes = equipped.progress.attributes;
                let rng = equipped.rng.clone();
                for attribute in [
                    AttributeKind::Strength,
                    AttributeKind::Intelligence,
                    AttributeKind::Wisdom,
                    AttributeKind::Dexterity,
                    AttributeKind::Constitution,
                    AttributeKind::Charisma,
                ] {
                    equipped.resolve_monster_attribute_drain(attribute);
                }
                assert_eq!(equipped.progress.attributes, attributes);
                assert_eq!(equipped.rng, rng);
            }
            "thror" => {
                assert_eq!(
                    (modifiers.strength, modifiers.constitution, modifiers.speed),
                    (3, 3, 3)
                );
                assert_eq!(equipped.player_equipment_bonuses().melee_damage, 2);
                give_inventory_item(&mut equipped, "test.bow", "demo.item.short-bow");
                equipped.equip_inventory_item("test.bow", None).unwrap();
                let shot = equipped.player_projectile_profile().unwrap();
                let melee = equipped.player_melee_profile(&equipped.player_derived_stats());
                equipped.unequip_slot(&slot).unwrap();
                let without = equipped.player_projectile_profile().unwrap();
                assert_eq!(
                    (shot.to_hit, shot.launcher_to_damage),
                    (without.to_hit, without.launcher_to_damage)
                );
                assert!(
                    melee.to_damage
                        >= equipped
                            .player_melee_profile(&equipped.player_derived_stats())
                            .to_damage
                            + 2
                );
                assert_eq!(equipped.player_equipment_bonuses().melee_damage, 0);
                assert_eq!(equipped.equipment_modifiers().speed, 0);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            game.generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(restored.rng, game.rng);
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(&context, Some(&base), false),
            Some(kind)
        );
    }
}

#[test]
fn fixed_high_resistance_armor_generates_reveals_equips_and_restores() {
    let mut game = Game::new_with_build(420, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-40".into(),
        depth: 40,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    };
    let mut remaining = BTreeSet::from([
        "rohirrim",
        "arvedui",
        "hithlomir",
        "thalkettoth",
        "thranduil",
    ]);
    // Repeated controlled-depth drops retain the complete ordinary production pool.
    for _ in 0..50_000 {
        for item in game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap()
        {
            if !remaining.remove(item.kind_id.strip_prefix("demo.item.").unwrap()) {
                continue;
            }
            assert_eq!(
                item.affix_ids,
                ["rfb-legacy.affix.artifact-extra-high-resistance"]
            );
            assert_eq!(item.rolled_affixes.len(), 1);
            assert_eq!(item.rolled_affixes[0].properties.resistances.len(), 1);
            assert!(item.activation.is_none() && item.curse.is_none());
            let id = item.id.clone();
            game.items.push(item);
            game.pick_up_item_at_player(Some(&id)).unwrap();
        }
        if remaining.is_empty() {
            break;
        }
    }
    assert!(
        remaining.is_empty(),
        "artifacts never generated: {remaining:?}"
    );
    assert_eq!(game.carried_weight_tenths_pound(), 575);
    game.reveal_current_visibility();
    let unknown = Game::from_save(game.to_save()).unwrap();
    assert_eq!(unknown.state_hash(), game.state_hash());
    for (kind, base, defense, hit) in [
        ("rohirrim", "metal-brigandine-armour", 34, 0),
        ("arvedui", "chain-mail", 29, -2),
        ("hithlomir", "soft-leather-armour", 24, 0),
        ("thalkettoth", "leather-scale-mail", 36, -1),
        ("thranduil", "hard-leather-cap", 12, 0),
    ] {
        let mut equipped = unknown.clone();
        let item = equipped
            .items
            .iter()
            .find(|item| item.kind_id == format!("demo.item.{kind}"))
            .unwrap();
        let id = item.id.clone();
        let rolled = item.rolled_affixes.clone();
        assert!(equipped.visible_item_resistances(item).is_empty());
        let rng = equipped.rng.clone();
        equipped.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        assert_eq!(equipped.rng, rng);
        let item = equipped.items.iter().find(|item| item.id == id).unwrap();
        let (&element, _) = rolled[0].properties.resistances.iter().next().unwrap();
        assert!(
            equipped.item_property_knowledge[&id]
                .known_affix_ids
                .contains(&rolled[0].affix_id)
        );
        assert!(
            equipped
                .visible_item_resistances(item)
                .iter()
                .any(|resistance| resistance.damage_type
                    == DamageTypeDto::from(DamageType::from(element)))
        );
        let before = equipped.player_derived_stats();
        equipped.equip_inventory_item(&id, None).unwrap();
        assert_eq!(
            equipped.player_derived_stats().armor_class.value,
            before.armor_class.value + defense * 10
        );
        assert_eq!(equipped.player_equipment_bonuses().melee_skill, hit);
        assert_eq!(
            equipped
                .effective_player_resistances()
                .level(element.into()),
            ResistanceLevel::Resistant
        );
        match kind {
            "rohirrim" => {
                assert_eq!(equipped.equipment_modifiers().strength, 2);
                assert_eq!(equipped.equipment_modifiers().dexterity, 2);
            }
            "arvedui" => {
                assert_eq!(equipped.equipment_modifiers().strength, 2);
                assert_eq!(equipped.equipment_modifiers().charisma, 2);
            }
            "hithlomir" => assert_eq!(equipped.player_equipment_bonuses().stealth_skill, 4),
            "thalkettoth" => {
                assert_eq!(equipped.equipment_modifiers().dexterity, 2);
                assert_eq!(equipped.equipment_modifiers().speed, 2);
            }
            "thranduil" => {
                assert_eq!(equipped.equipment_modifiers().intelligence, 2);
                assert_eq!(equipped.equipment_modifiers().wisdom, 2);
                assert!(
                    equipped
                        .player_equipment_passives()
                        .contains(&EquipmentPassive::Telepathy)
                );
            }
            _ => unreachable!(),
        }
        equipped.reveal_current_visibility();
        let mut restored = Game::from_save(equipped.to_save()).unwrap();
        assert_eq!(restored.state_hash(), equipped.state_hash());
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .rolled_affixes,
            rolled
        );
        assert!(
            restored
                .generated_artifact_ids
                .contains(&format!("demo.item.{kind}"))
        );
        let next = equipped
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap();
        assert_eq!(
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            next
        );
        assert_eq!(restored.rng, equipped.rng);
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(
                &context,
                Some(&format!("demo.item.{base}")),
                false
            ),
            Some(format!("demo.item.{kind}"))
        );
    }
}

#[test]
fn fixed_weapon_pair_generates_and_preserves_combat_and_passives_after_save() {
    let mut game = Game::new_with_build(418, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    let origin = game.player.position;
    let adjacent = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, adjacent, "demo.terrain.floor");
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-30".into(),
        depth: 30,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    };
    let mut remaining = BTreeSet::from(["demo.item.gondricam", "demo.item.forasgil"]);
    // Controlled depth and repeated production drops retain the full ordinary
    // pool, quality, rarity and unique-artifact registration.
    for _ in 0..50_000 {
        for item in game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap()
        {
            if !remaining.remove(item.kind_id.as_str()) {
                continue;
            }
            assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
            assert!(item.activation.is_none() && item.curse.is_none());
            let id = item.id.clone();
            game.items.push(item);
            game.pick_up_item_at_player(Some(&id)).unwrap();
            game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
            assert!(game.item_property_knowledge[&id].identified);
        }
        if remaining.is_empty() {
            break;
        }
    }
    assert!(
        remaining.is_empty(),
        "artifacts never generated: {remaining:?}"
    );
    assert_eq!(game.carried_weight_tenths_pound(), 150);
    for (kind, base, hit, damage, sides) in [
        ("gondricam", "cutlass", 10, 11, 9),
        ("forasgil", "rapier", 12, 19, 8),
    ] {
        let mut equipped = game.clone();
        let id = equipped
            .items
            .iter()
            .find(|item| item.kind_id == format!("demo.item.{kind}"))
            .unwrap()
            .id
            .clone();
        equipped
            .equip_inventory_item(&id, Some("right-hand"))
            .unwrap();
        let profile = equipped.player_melee_profile(&equipped.player_derived_stats());
        assert_eq!(profile.source_item_id.as_deref(), Some(id.as_str()));
        assert_eq!((profile.damage_dice, profile.damage_sides), (1, sides));
        assert_eq!(profile.to_hit, hit);
        assert!(profile.to_damage >= damage);
        if kind == "gondricam" {
            assert_eq!(equipped.equipment_modifiers().dexterity, 3);
            assert_eq!(equipped.player_equipment_bonuses().stealth_skill, 3);
            assert!(equipped.player_levitates());
            assert_eq!(equipped.player_see_invisible_sources(), 1);
            for element in [
                DamageType::Acid,
                DamageType::Electricity,
                DamageType::Fire,
                DamageType::Cold,
            ] {
                assert_eq!(
                    equipped.effective_player_resistances().level(element),
                    ResistanceLevel::Resistant
                );
            }
        } else {
            assert_eq!(equipped.equipment_modifiers().speed, 1);
            assert_eq!(equipped.player_equipment_bonuses().light_radius, 1);
            for element in [DamageType::Cold, DamageType::Light] {
                assert_eq!(
                    equipped.effective_player_resistances().level(element),
                    ResistanceLevel::Resistant
                );
            }
        }
        equipped.reveal_current_visibility();
        let mut restored = Game::from_save(equipped.to_save()).unwrap();
        assert_eq!(restored.state_hash(), equipped.state_hash());
        assert_eq!(
            restored
                .player_melee_profile(&restored.player_derived_stats())
                .to_damage,
            profile.to_damage
        );
        assert!(
            restored
                .generated_artifact_ids
                .contains(&format!("demo.item.{kind}"))
        );
        let next = equipped
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap();
        assert_eq!(
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            next
        );
        assert_eq!(restored.rng, equipped.rng);
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(
                &context,
                Some(&format!("demo.item.{base}")),
                false
            ),
            Some(format!("demo.item.{kind}"))
        );
        if kind == "gondricam" {
            // Exercise the existing flat equipment recovery, separately from
            // natural percentage regeneration; do not claim source regen parity.
            restored.player.hp = 1;
            restored.world_tick = 0;
            let update = dispatch_next(&mut restored, GameCommand::Wait);
            assert!(
                update
                    .events
                    .iter()
                    .any(|event| event.message_key == "equipment-regenerated")
            );
            assert!(restored.player.hp > 1);
            let pit = adjacent;
            replace_terrain(&mut restored, pit, "demo.terrain.dark-pit");
            let mut unarmed = restored.clone();
            unarmed
                .items
                .iter_mut()
                .find(|item| item.id == id)
                .unwrap()
                .location = ItemLocation::Inventory;
            dispatch_next(
                &mut unarmed,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            assert_eq!(unarmed.player.position, origin);
            dispatch_next(
                &mut restored,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            assert_eq!(restored.player.position, pit);
            restored.player.position = origin;
            replace_terrain(&mut restored, adjacent, "demo.terrain.floor");
        }
        {
            let mut damages = Vec::new();
            // Matching cold immunity suppresses only the brand. Animal slay
            // remains active; a matching slay and brand do not multiply together.
            let targets = if kind == "gondricam" {
                vec![("goblin", ResistanceLevel::Immune, 10)]
            } else {
                vec![
                    ("goblin", ResistanceLevel::Resistant, 24),
                    ("goblin", ResistanceLevel::Immune, 10),
                    ("sheep", ResistanceLevel::Immune, 24),
                    ("sheep", ResistanceLevel::Resistant, 24),
                ]
            };
            for (target_kind, cold, multiplier) in targets {
                let mut combat = restored.clone();
                combat.push_generated_actor(
                    "test.weapon-target".into(),
                    &format!("demo.actor.{target_kind}"),
                    adjacent,
                );
                let target = &mut combat.entities[0];
                target.hp = 10_000;
                target.max_hp = 10_000;
                target.resistances.set(DamageType::Cold, cold);
                let profile = combat.player_melee_profile(&combat.player_derived_stats());
                let target = &combat.entities[0];
                assert_eq!(
                    combat.player_melee_damage_multiplier(
                        &profile,
                        target,
                        combat.content.actor(&target.kind_id).unwrap()
                    ),
                    multiplier
                );
                let actual = (0..100)
                    .find_map(|seed| {
                        let mut trial = combat.clone();
                        trial.rng = RfbRng::seeded(seed);
                        let mut events = Vec::new();
                        trial
                            .resolve_player_melee(
                                0,
                                false,
                                &mut events,
                                &mut BTreeSet::new(),
                                &mut Vec::new(),
                            )
                            .unwrap();
                        events
                            .into_iter()
                            .map(DomainEvent::into_dto)
                            .find_map(|event| match event.outcome {
                                Some(GameEventOutcomeDto::Damage { resolution }) => {
                                    assert!(trial.entities[0].hp < 10_000);
                                    Some(resolution.raw_damage)
                                }
                                _ => None,
                            })
                    })
                    .expect("seed range includes a weapon hit");
                damages.push(actual);
            }
            if kind == "forasgil" {
                assert!(damages[0] > damages[1]);
                assert_eq!(damages[2], damages[3]);
            }
        }
    }
}

#[test]
fn fixed_armor_pair_generates_equips_and_preserves_consumers_after_save() {
    let mut game = Game::new_with_build(417, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-40".into(),
        depth: 40,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    };
    let mut remaining = BTreeSet::from(["demo.item.thorongil", "demo.item.cambeleg"]);
    // Controlled depth and repeated production drops, retaining the complete
    // ordinary pool, quality, rarity and uniqueness rules.
    for _ in 0..50_000 {
        for item in game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap()
        {
            if !remaining.remove(item.kind_id.as_str()) {
                continue;
            }
            assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
            assert!(item.activation.is_none() && item.curse.is_none());
            let id = item.id.clone();
            game.items.push(item);
            game.pick_up_item_at_player(Some(&id)).unwrap();
            game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
            assert!(game.item_property_knowledge[&id].identified);
        }
        if remaining.is_empty() {
            break;
        }
    }
    assert!(
        remaining.is_empty(),
        "artifacts never generated: {remaining:?}"
    );
    assert_eq!(game.carried_weight_tenths_pound(), 15);
    for (kind, defense) in [("thorongil", 11), ("cambeleg", 16)] {
        let id = game
            .items
            .iter()
            .find(|item| item.kind_id == format!("demo.item.{kind}"))
            .unwrap()
            .id
            .clone();
        let before = game.player_derived_stats();
        let modifiers = game.equipment_modifiers();
        let bonuses = game.player_equipment_bonuses();
        let see_invisible = game.player_see_invisible_sources();
        game.equip_inventory_item(&id, None).unwrap();
        let after = game.player_derived_stats();
        assert_eq!(
            after.armor_class.value,
            before.armor_class.value + defense * 10
        );
        assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
        if kind == "thorongil" {
            assert_eq!(game.player_see_invisible_sources(), see_invisible + 1);
            for element in [DamageType::Electricity, DamageType::Fire, DamageType::Cold] {
                assert_eq!(
                    game.effective_player_resistances().level(element),
                    ResistanceLevel::Resistant
                );
            }
        } else {
            assert_eq!(game.equipment_modifiers().strength, modifiers.strength + 3);
            assert_eq!(
                game.equipment_modifiers().constitution,
                modifiers.constitution + 3
            );
            assert_eq!(
                game.player_equipment_bonuses().melee_skill,
                bonuses.melee_skill + 8
            );
            assert_eq!(
                game.player_equipment_bonuses().melee_damage,
                bonuses.melee_damage + 8
            );
            assert!(after.melee_skill.value >= before.melee_skill.value + 8);
            assert!(after.melee_damage_bonus.value >= before.melee_damage_bonus.value + 8);
        }
    }
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let expected = game.player_derived_stats();
    let actual = restored.player_derived_stats();
    assert_eq!(actual.armor_class, expected.armor_class);
    assert_eq!(actual.melee_skill, expected.melee_skill);
    assert_eq!(actual.melee_damage_bonus, expected.melee_damage_bonus);
    assert_eq!(
        restored.player_see_invisible_sources(),
        game.player_see_invisible_sources()
    );
    assert!(
        restored
            .player_status_immunities()
            .contains(STATUS_PARALYSIS)
    );
    let next = game
        .generate_loot_instances(&context, ItemLocation::Inventory)
        .unwrap();
    assert_eq!(
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        next
    );
    assert_eq!(restored.rng, game.rng);
    for (kind, base) in [
        ("thorongil", "cloak"),
        ("cambeleg", "set-of-studded-leather-gloves"),
    ] {
        let kind = format!("demo.item.{kind}");
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(
                &context,
                Some(&format!("demo.item.{base}")),
                false
            ),
            Some(kind)
        );
    }
}

fn razorback_game() -> (Game, String) {
    let mut game = Game::new_with_build(129, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    game.gold_piles.clear();
    let context = artifact_loot_context(90);
    game.rng = RfbRng::seeded(
        (0..1_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(9) == 0)
            .unwrap(),
    );
    let kind = game
        .roll_fixed_artifact_kind_id(
            &context,
            Some("demo.item.multi-hued-dragon-scale-mail"),
            false,
        )
        .unwrap();
    assert_eq!(kind, "demo.item.razorback");
    let draft = game.fixed_item_draft(&context, kind);
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert_eq!(item.activation.as_ref().unwrap().power, 30);
    let id = item.id.clone();
    game.items.push(item);
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    (game, id)
}

#[test]
fn razorback_equipment_and_unique_identity_survive_save() {
    let (mut game, id) = razorback_game();
    for item in &mut game.items {
        if matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "body") {
            item.location = ItemLocation::Inventory;
        }
    }
    let before = game.equipment_modifiers();
    let bonuses = game.player_equipment_bonuses();
    let see_invisible = game.player_see_invisible_sources();
    assert!(!game.player_aggravates_monsters());
    assert!(game.equip_inventory_item(&id, None).is_some());
    assert_eq!(game.equipment_modifiers().defense, before.defense + 65);
    assert_eq!(
        game.player_equipment_bonuses().melee_skill,
        bonuses.melee_skill - 4
    );
    assert_eq!(
        game.player_equipment_bonuses().light_radius,
        bonuses.light_radius + 1
    );
    assert_eq!(game.player_see_invisible_sources(), see_invisible + 1);
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    assert!(game.player_aggravates_monsters());
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Electricity),
        ResistanceLevel::Immune
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .generated_artifact_ids
            .contains("demo.item.razorback")
    );
    assert!(
        restored
            .roll_fixed_artifact_kind_id(
                &artifact_loot_context(90),
                Some("demo.item.multi-hued-dragon-scale-mail"),
                false
            )
            .is_none()
    );
    restored
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .location = ItemLocation::Inventory;
    assert_eq!(restored.equipment_modifiers().defense, before.defense);
    assert!(!restored.player_aggravates_monsters());
    assert_ne!(
        restored
            .effective_player_resistances()
            .level(DamageType::Electricity),
        ResistanceLevel::Immune
    );
}

#[test]
fn artifact_inventory_drop_finds_distant_land_without_relaxing_ordinary_drops() {
    let (mut game, artifact_id) = razorback_game();
    game.player.position = Position { x: 99, y: 33 };
    for y in 29..=37 {
        for x in 95..=103 {
            replace_terrain(
                &mut game,
                Position { x, y },
                "demo.terrain.surface-water-deep",
            );
        }
    }
    give_inventory_item(&mut game, "test.ordinary-drop", "demo.item.iron-shot");
    assert!(
        game.drop_inventory_items(&[artifact_id.clone(), "test.ordinary-drop".to_owned()])
            .is_none()
    );
    assert!(
        game.drop_inventory_quantity("test.ordinary-drop", 1)
            .unwrap()
            .is_none()
    );
    let (_, _, landing) = game
        .drop_inventory_quantity(&artifact_id, 1)
        .unwrap()
        .unwrap();
    assert!(game.is_walkable(landing));
    assert!(
        game.items
            .iter()
            .any(|item| item.id == artifact_id && item.location == ItemLocation::Ground(landing))
    );
    assert!(game.items.iter().any(|item| item.id == "test.ordinary-drop" && item.location == ItemLocation::Inventory));
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn razorback_star_ball_and_full_10000_tick_cooldown_survive_save() {
    let (mut game, id) = razorback_game();
    assert!(game.equip_inventory_item(&id, None).is_some());
    game.player.position = Position { x: 99, y: 33 };
    for y in 29..=37 {
        for x in 95..=103 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.world_tick = 9_997;
    game.rng = RfbRng::seeded(
        (0..1_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
            .unwrap(),
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: id.clone(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    let blasts = update
        .events
        .iter()
        .filter_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::AbilityAreaDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!((5..=15).contains(&blasts.len()));
    assert!(blasts.iter().all(|blast| blast.radius == 3
        && blast.damage_type == DamageTypeDto::Electricity
        && blast.base_raw_damage == 150));
    assert_eq!(game.world_tick, 10_007);
    let mut events = Vec::new();
    for tick in 10_008..=14_997 {
        game.world_tick = tick;
        game.process_inventory_device_recovery(&mut events);
    }
    let item = game.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        (item.charges.unwrap().current, item.device_recovery_progress),
        (0, 5_000)
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for tick in 14_998..=19_996 {
        restored.world_tick = tick;
        restored.process_inventory_device_recovery(&mut events);
    }
    let item = restored.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        (item.charges.unwrap().current, item.device_recovery_progress),
        (0, 9_999)
    );
    restored.world_tick = 19_997;
    restored.process_inventory_device_recovery(&mut events);
    let item = restored.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        (item.charges.unwrap().current, item.device_recovery_progress),
        (1, 0)
    );
}

#[test]
fn razorback_guardian_reward_shares_natural_generation_uniqueness() {
    let pack =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack).unwrap();
    let world = artifact
        .content
        .worlds
        .iter_mut()
        .find(|world| world.id == DEFAULT_WORLD_ID)
        .unwrap();
    // Exercise the shared reward consumer without opening the Rlyeh dungeon in R2.
    let floor = world
        .procedural_floors
        .iter_mut()
        .find(|floor| {
            floor.guardian.as_ref().is_some_and(|guardian| {
                guardian.reward_artifact_item_kind_id.as_deref() == Some("demo.item.lotharang")
            })
        })
        .unwrap();
    let floor_id = floor.id.clone();
    let dungeon_id = floor.dungeon_id.clone().unwrap();
    let guardian = floor.guardian.as_mut().unwrap();
    guardian.reward_artifact_item_kind_id = Some("demo.item.razorback".to_owned());
    let guardian = guardian.clone();
    let content = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    let (mut game, _) = razorback_game();
    game.content = content;
    game.current_floor_id = floor_id;
    game.dungeon_states.get_mut(&dungeon_id).unwrap().suppressed = false;
    let mut actor = game.player.clone();
    actor.id = guardian.instance_id;
    actor.kind_id = guardian.actor_kind_id;
    let (replacement, _) = game.generate_death_loot(&actor).unwrap();
    assert!(
        replacement
            .iter()
            .all(|item| item.kind_id != "demo.item.razorback")
    );
    // The shared Artifact mode now materializes the replacement artifact.
    assert!(
        replacement
            .last()
            .is_some_and(|item| item.is_artifact(&game.content))
    );
    game.generated_artifact_ids.remove("demo.item.razorback");
    game.items
        .retain(|item| item.kind_id != "demo.item.razorback");
    let (reward, _) = game.generate_death_loot(&actor).unwrap();
    assert_eq!(
        reward
            .iter()
            .filter(|item| item.kind_id == "demo.item.razorback")
            .count(),
        1
    );
    assert!(game.generated_artifact_ids.contains("demo.item.razorback"));
    assert!(
        game.roll_fixed_artifact_kind_id(
            &artifact_loot_context(90),
            Some("demo.item.multi-hued-dragon-scale-mail"),
            false
        )
        .is_none()
    );
}

fn dr_jones_game() -> (Game, String) {
    let mut game = Game::new_with_build(162, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    game.gold_piles.clear();
    let context = artifact_loot_context(8);
    let seed = (0..1_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(5) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.whip"), false)
            .as_deref(),
        Some("demo.item.dr-jones-whip")
    );
    let draft = game.fixed_item_draft(&context, "demo.item.dr-jones-whip".to_owned());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert_eq!(item.activation.as_ref().unwrap().power, 25);
    assert_eq!(item.charges.unwrap().current, 1);
    let id = item.id.clone();
    game.item_property_knowledge.insert(
        id.clone(),
        ItemPropertyKnowledgeState {
            known_blessed: false,
            discovered: true,
            appraised: true,
            identified: true,
            known_affix_ids: BTreeSet::new(),
            feeling: None,
        },
    );
    game.items.push(item);
    (game, id)
}

#[test]
fn dr_jones_equipment_passives_and_unique_generation_survive_save() {
    let (mut game, id) = dr_jones_game();
    for item in &mut game.items {
        if matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "right-hand") {
            item.location = ItemLocation::Inventory;
        }
    }
    let before = game.equipment_modifiers();
    let see_invisible = game.player_see_invisible_sources();
    assert!(!game.player_levitates());
    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .location = ItemLocation::Equipped {
        slot_id: "right-hand".to_owned(),
    };
    assert_eq!(
        game.equipment_modifiers().intelligence,
        before.intelligence + 1
    );
    assert_eq!(game.equipment_modifiers().wisdom, before.wisdom + 1);
    assert!(game.player_levitates());
    assert_eq!(game.player_see_invisible_sources(), see_invisible + 1);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .generated_artifact_ids
            .contains("demo.item.dr-jones-whip")
    );
    let context = artifact_loot_context(8);
    assert!(
        restored
            .roll_fixed_artifact_kind_id(&context, Some("demo.item.whip"), false)
            .is_none()
    );
    restored
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .location = ItemLocation::Inventory;
    assert_eq!(
        restored.equipment_modifiers().intelligence,
        before.intelligence
    );
    assert_eq!(restored.equipment_modifiers().wisdom, before.wisdom);
    assert!(!restored.player_levitates());
    assert_eq!(restored.player_see_invisible_sources(), see_invisible);
}

#[test]
fn dr_jones_fetch_and_full_300_tick_cooldown_survive_save() {
    let (mut game, id) = dr_jones_game();
    let origin = game.player.position;
    let target = Position {
        x: origin.x + 3,
        y: origin.y,
    };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, target, "demo.terrain.floor");
    give_inventory_item(&mut game, "test.jones-stack", "demo.item.iron-shot");
    let stack = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.jones-stack")
        .unwrap();
    stack.location = ItemLocation::Ground(target);
    stack.quantity = 9;
    game.reveal_current_visibility();
    // Cross a global recovery boundary during activation; it must remain empty.
    game.world_tick = 297;
    game.rng = RfbRng::seeded(
        (0..1_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
            .unwrap(),
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: id.clone(),
            target: Some(TargetSelection::Position { position: target }),
        },
    );
    assert_eq!(game.world_tick, 307);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.jones-stack")
            .unwrap()
            .location,
        ItemLocation::Ground(origin)
    );
    let mut events = Vec::new();
    for tick in 308..=447 {
        game.world_tick = tick;
        game.process_inventory_device_recovery(&mut events);
    }
    let item = game.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        (item.charges.unwrap().current, item.device_recovery_progress),
        (0, 150)
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for tick in 448..=596 {
        restored.world_tick = tick;
        restored.process_inventory_device_recovery(&mut events);
    }
    let item = restored.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        (item.charges.unwrap().current, item.device_recovery_progress),
        (0, 299)
    );
    restored.world_tick = 597;
    restored.process_inventory_device_recovery(&mut events);
    let item = restored.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(
        (item.charges.unwrap().current, item.device_recovery_progress),
        (1, 0)
    );
}

#[test]
fn p90b_olog_hai_affix_materializes_and_runs_existing_berserk_activation() {
    let mut game =
        Game::new_with_build(90, "demo.build.warrior").expect("Olog-hai reward game should create");
    let original_content = game.content.clone();
    let original_floor_id = game.current_floor_id.clone();
    super::dungeon_anti_magic::enter_context(&mut game);
    clear_monsters(&mut game);
    let mut rewards = game
        .generate_loot_instances(
            &LootContext {
                table_id: "demo.loot-table.troll-cave-final-reward".to_owned(),
                floor_id: "demo.floor.troll-cave-depth-36".to_owned(),
                depth: 36,
                source: LootSource::FloorRoom {
                    room_id: "demo.guardian.troll-cave.1".to_owned(),
                    spawn_id: "demo.guardian.troll-cave.reward".to_owned(),
                },
            },
            ItemLocation::Inventory,
        )
        .expect("Troll cave reward should generate");
    let reward = rewards
        .pop()
        .expect("Troll cave reward should contain one armour");
    assert!(rewards.is_empty());
    assert_eq!(reward.kind_id, "demo.item.metal-lamellar-armour");
    assert_eq!(reward.quality, ItemQualityDto::Fine);
    assert_eq!(reward.affix_ids, ["rfb-legacy.affix.olog-hai"]);
    assert_eq!(reward.rolled_affixes.len(), 1);
    let activation = reward
        .activation
        .as_ref()
        .expect("Olog-hai reward should materialize its activation");
    assert_eq!(
        activation.profile_id,
        "rfb.device-activation.ego-72-berserk"
    );
    assert_eq!(activation.device_check_difficulty, 10);
    assert_eq!(activation.power, 10);
    assert_eq!(
        reward.charges,
        Some(ItemChargesDto {
            current: 1,
            maximum: 1
        })
    );

    let item_id = reward.id.clone();
    game.items.push(reward);
    let max_hp = game.player_derived_stats().max_hp.value;
    game.player.hp = (max_hp - 30).max(1);
    let hp_before = game.player.hp;
    game.world_tick = 0;
    let activation_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < 5
        })
        .expect("an automatic device success seed should exist");
    game.rng = RfbRng::seeded(activation_seed);

    let activated = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: item_id.clone(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert!(
        activated
            .events
            .iter()
            .any(|event| event.kind == "item.use-berserk-strength-applied")
    );
    assert!(
        activated
            .events
            .iter()
            .any(|event| event.kind == "item.use-heal")
    );
    assert_eq!(game.player.hp, (hp_before + 30).min(max_hp));
    let berserk = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_BERSERK)
        .expect("Olog-hai activation should apply Berserk");
    assert!((260..=500).contains(&berserk.remaining_ticks));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.charges)
            .expect("Olog-hai reward should retain charges")
            .current,
        0
    );

    // The activation above runs in NO_MAGIC; keep the existing independent
    // save/recharge fixture on its original floor and content catalog.
    clear_monsters(&mut game);
    game.content = original_content;
    game.current_floor_id = original_floor_id;
    let hash = game.state_hash();
    let mut restored = Game::from_save(game.to_save()).expect("Olog-hai reward should restore");
    assert_eq!(restored.state_hash(), hash);
    for _ in 0..50 {
        if restored
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.charges)
            .is_some_and(|charges| charges.current == 1)
        {
            break;
        }
        dispatch_next(&mut restored, GameCommand::Wait);
    }
    assert_eq!(
        restored
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.charges)
            .expect("Olog-hai reward should recover its charge")
            .current,
        1
    );
}

#[test]
fn p97e_multi_hued_dragon_breath_randomizes_five_elements_across_a_cone() {
    const ITEM_ID: &str = "test.item.multi-hued-dragon-scale-mail.1";
    let mut base =
        Game::new_with_build(197, "demo.build.warrior").expect("breath test game should create");
    clear_monsters(&mut base);
    base.terrain.fill("demo.terrain.floor".to_owned());
    base.player.position = Position { x: 10, y: 10 };
    give_inventory_item(&mut base, ITEM_ID, "demo.item.multi-hued-dragon-scale-mail");
    base.push_generated_actor(
        "test.actor.dragon-breath-center".to_owned(),
        "demo.actor.ancient-multi-hued-dragon",
        Position { x: 20, y: 10 },
    );
    base.push_generated_actor(
        "test.actor.dragon-breath-lateral".to_owned(),
        "demo.actor.ancient-multi-hued-dragon",
        Position { x: 27, y: 11 },
    );

    let mut observed = BTreeSet::new();
    for seed in 0..10_000 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: Some(TargetSelection::Direction {
                    direction: Direction::East,
                }),
            },
        );
        let hits = update
            .events
            .iter()
            .filter(|event| event.kind == "item.activation-hit")
            .filter_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::Damage { resolution }) => Some(resolution),
                _ => None,
            })
            .collect::<Vec<_>>();
        if hits.is_empty() {
            continue;
        }
        assert_eq!(hits.len(), 2);
        assert!(
            hits.iter()
                .all(|damage| damage.damage_type == hits[0].damage_type)
        );
        assert_eq!(
            hits.iter()
                .map(|damage| damage.raw_damage)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([125, 250])
        );
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == ITEM_ID)
                .and_then(|item| item.charges)
                .expect("activated scale mail should retain charge state")
                .current,
            0
        );
        observed.insert(hits[0].damage_type);
        if observed.len() == 5 {
            break;
        }
    }
    assert_eq!(
        observed,
        BTreeSet::from([
            DamageTypeDto::Acid,
            DamageTypeDto::Electricity,
            DamageTypeDto::Fire,
            DamageTypeDto::Cold,
            DamageTypeDto::Poison,
        ])
    );
}

#[test]
fn p99e_paurnimmen_cold_beam_hits_each_actor_before_the_wall() {
    const ITEM_ID: &str = "test.item.paurnimmen.1";
    const BLOCKED_ID: &str = "test.actor.paurnimmen-blocked";
    let mut base =
        Game::new_with_build(199, "demo.build.warrior").expect("beam test game should create");
    clear_monsters(&mut base);
    base.terrain.fill("demo.terrain.floor".to_owned());
    base.player.position = Position { x: 10, y: 10 };
    give_inventory_item(&mut base, ITEM_ID, "demo.item.set-of-gauntlets-paurnimmen");
    for (id, x) in [
        ("test.actor.paurnimmen-near", 14),
        ("test.actor.paurnimmen-far", 17),
        (BLOCKED_ID, 22),
    ] {
        base.push_generated_actor(
            id.to_owned(),
            "demo.actor.anti-paladin",
            Position { x, y: 10 },
        );
    }
    let wall = Position { x: 20, y: 10 };
    let wall_index = base.index(wall).expect("wall position should be in bounds");
    base.terrain[wall_index] = "demo.terrain.wall".to_owned();
    let blocked_hp = base
        .entities
        .iter()
        .find(|entity| entity.id == BLOCKED_ID)
        .expect("blocked actor should exist")
        .hp;

    let (game, resolutions) = (0..1_000)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let update = dispatch_next(
                &mut game,
                GameCommand::UseItem {
                    item_id: ITEM_ID.to_owned(),
                    target: Some(TargetSelection::Direction {
                        direction: Direction::East,
                    }),
                },
            );
            let resolutions = update
                .events
                .iter()
                .filter(|event| {
                    matches!(
                        event.kind.as_str(),
                        "item.activation-hit" | "item.activation-slew"
                    )
                })
                .filter_map(|event| match event.outcome.as_ref() {
                    Some(GameEventOutcomeDto::Damage { resolution }) => Some(*resolution),
                    _ => None,
                })
                .collect::<Vec<_>>();
            (!resolutions.is_empty()).then_some((game, resolutions))
        })
        .expect("one seed should pass Paurnimmen's device check");

    assert_eq!(resolutions.len(), 2);
    assert!(resolutions.iter().all(|resolution| {
        resolution.raw_damage == 40 && resolution.damage_type == DamageTypeDto::Cold
    }));
    assert_eq!(
        game.entities
            .iter()
            .find(|entity| entity.id == BLOCKED_ID)
            .expect("the wall should protect the blocked actor")
            .hp,
        blocked_hp
    );
}

#[test]
fn a6_aule_rolls_source_ability_before_high_resistance_once() {
    let original = Game::new_with_build(429, "demo.build.warrior").unwrap();
    let context = artifact_loot_context(80);
    let resistances = [
        ActorDamageType::Poison,
        ActorDamageType::Light,
        ActorDamageType::Dark,
        ActorDamageType::Shards,
        ActorDamageType::Blindness,
        ActorDamageType::Confusion,
        ActorDamageType::Sound,
        ActorDamageType::Nether,
        ActorDamageType::Nexus,
        ActorDamageType::Chaos,
        ActorDamageType::Disenchant,
        ActorDamageType::Fear,
    ];
    let low_esp = [
        EquipmentPassive::EspAnimal,
        EquipmentPassive::EspUndead,
        EquipmentPassive::EspDemon,
        EquipmentPassive::EspOrc,
        EquipmentPassive::EspTroll,
        EquipmentPassive::EspGiant,
        EquipmentPassive::EspDragon,
        EquipmentPassive::EspHuman,
        EquipmentPassive::EspGood,
    ];
    // Eight simple powers and both source branches into all nine low ESPs.
    let choices = (0..8)
        .map(|choice| (choice, None))
        .chain((8..10).flat_map(|choice| (0..9).map(move |esp| (choice, Some(esp)))));
    for (choice, esp) in choices {
        let resistance_index = (choice + esp.unwrap_or(0)) % 12;
        let seed = (0..100_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(10) == choice
                    && esp.is_none_or(|esp| rng.bounded(9) == esp)
                    && rng.bounded(12) == resistance_index
            })
            .expect("seed range must cover each source ability branch followed by resistance");
        let mut expected_rng = RfbRng::seeded(seed);
        assert_eq!(expected_rng.bounded(10), choice);
        if let Some(esp) = esp {
            assert_eq!(expected_rng.bounded(9), esp);
        }
        assert_eq!(expected_rng.bounded(12), resistance_index);
        let mut expected_power = AffixPropertyBundleDefinition::default();
        match choice {
            0 => {
                expected_power.passives.insert(EquipmentPassive::Levitation);
            }
            1 => {
                expected_power.rfb_flags.insert("LITE".into());
                expected_power.equipment_bonuses.light_radius = 1;
            }
            2 => {
                expected_power
                    .passives
                    .insert(EquipmentPassive::SeeInvisible);
            }
            3 => {
                expected_power.passives.insert(EquipmentPassive::Warning);
            }
            4 => {
                expected_power
                    .passives
                    .insert(EquipmentPassive::SlowDigestion);
            }
            5 => {
                expected_power
                    .passives
                    .insert(EquipmentPassive::Regeneration);
            }
            6 => expected_power
                .status_immunities
                .push(STATUS_PARALYSIS.into()),
            7 => {
                expected_power.passives.insert(EquipmentPassive::HoldLife);
            }
            _ => {
                expected_power
                    .passives
                    .insert(low_esp[esp.unwrap() as usize]);
            }
        }
        let mut game = original.clone();
        game.rng = RfbRng::seeded(seed);
        let draft = game.fixed_item_draft(&context, "demo.item.aule".into());
        assert_eq!(game.rng, expected_rng);
        assert_eq!(draft.intrinsic_properties, expected_power);
        assert_eq!(
            draft.affix_ids,
            ["rfb-legacy.affix.artifact-extra-high-resistance"]
        );
        let [rolled] = draft.rolled_affixes.as_slice() else {
            panic!("one high-resistance roll")
        };
        let expected_resistance = AffixPropertyBundleDefinition {
            resistances: BTreeMap::from([(
                resistances[resistance_index as usize],
                rfb_content::ActorResistanceLevel::Resistant,
            )]),
            ..Default::default()
        };
        assert_eq!(rolled.properties, expected_resistance);
        if resistance_index == 8 || matches!(choice, 2 | 6) {
            // Existing NEXUS/SEE_INVIS/FREE_ACT flags neither reroll nor stack.
            game.items.clear();
            let item = game
                .commit_generated_item_draft(draft, ItemLocation::Inventory)
                .unwrap();
            let id = item.id.clone();
            game.items.push(item);
            game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
            game.equip_inventory_item(&id, Some("right-hand")).unwrap();
            assert_eq!(game.player_see_invisible_sources(), 1);
            assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
            assert_eq!(
                game.effective_player_resistances().level(DamageType::Nexus),
                ResistanceLevel::Resistant
            );
            assert_eq!(
                game.visible_item_resistances(&game.items[0])
                    .iter()
                    .filter(|r| r.damage_type == DamageTypeDto::Nexus)
                    .count(),
                1
            );
            assert_eq!(game.rng, expected_rng);
        }
    }
    // Existing mage artifacts and the favored Terror Mask initialize activation
    // first; only Gandalf and this Mask branch then draw a power.
    for slug in ["gandalf", "saruman", "indra", "terror-mask"] {
        for seed in 0..16 {
            let mut game = original.clone();
            game.rng = RfbRng::seeded(seed);
            let mut expected_rng = game.rng.clone();
            let kind = format!("demo.item.{slug}");
            let (activation, charges) =
                initial_item_runtime_state(&game.content, &mut expected_rng, &kind, &[], 80);
            if matches!(slug, "gandalf" | "terror-mask") && expected_rng.bounded(10) >= 8 {
                expected_rng.bounded(9);
            }
            if slug == "terror-mask" {
                expected_rng.bounded(12);
            }
            let draft = game.fixed_item_draft(&context, kind);
            assert_eq!(game.rng, expected_rng, "unchanged RNG order for {slug}");
            assert_eq!(draft.activation, activation);
            assert_eq!(draft.charges, charges);
        }
    }
}

#[test]
fn a6_aule_generates_with_both_extras_and_replays_combat_after_save() {
    fn strike(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        events
    }
    let mut game = Game::new_with_build(430, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    let adjacent = Position { x: 11, y: 10 };
    replace_terrain(&mut game, Position { x: 10, y: 10 }, "demo.terrain.floor");
    replace_terrain(&mut game, adjacent, "demo.terrain.floor");
    game.glow.fill(true);
    let context = artifact_loot_context(80);
    let selected = (0..10_000)
        .find_map(|seed| {
            game.rng = RfbRng::seeded(seed);
            game.roll_fixed_artifact_kind_id(&context, Some("demo.item.great-hammer"), false)
                .filter(|kind| kind == "demo.item.aule")
        })
        .expect("Aule must occur with source rarity 75 at controlled base/depth 80");
    let draft = game.fixed_item_draft(&context, selected);
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    assert!(item.activation.is_none() && item.charges.is_none() && item.curse.is_none());
    let intrinsic = item.intrinsic_properties.clone();
    let rolled = item.rolled_affixes.clone();
    assert_ne!(intrinsic, AffixPropertyBundleDefinition::default());
    assert_eq!(rolled.len(), 1);
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    assert_eq!(game.carried_weight_tenths_pound(), 120);
    assert!(
        !game
            .item_property_knowledge
            .get(&id)
            .is_some_and(|k| k.appraised)
    );
    assert!(game.visible_item_passives(&game.items[0]).is_empty());
    assert!(game.visible_item_resistances(&game.items[0]).is_empty());
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    assert_eq!(restored.items[0].intrinsic_properties, intrinsic);
    assert_eq!(restored.items[0].rolled_affixes, rolled);
    game = restored;
    let rng = game.rng.clone();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&id, Some("right-hand")).unwrap();
    assert_eq!(game.rng, rng);
    assert!(
        game.item_property_knowledge[&id]
            .known_affix_ids
            .contains(&rolled[0].affix_id)
    );
    assert!(
        game.item_passives(&game.items[0])
            .is_superset(&intrinsic.passives)
    );
    let modifiers = game.equipment_modifiers();
    assert_eq!((modifiers.wisdom, modifiers.defense), (4, 5));
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    // A duplicate intrinsic SEE_INVIS is still one equipped source.
    assert_eq!(game.player_see_invisible_sources(), 1);
    for element in [
        DamageType::Acid,
        DamageType::Electricity,
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Nexus,
    ] {
        assert_eq!(
            game.effective_player_resistances().level(element),
            ResistanceLevel::Resistant
        );
    }
    let (&element, _) = rolled[0].properties.resistances.iter().next().unwrap();
    assert_eq!(
        game.effective_player_resistances().level(element.into()),
        ResistanceLevel::Resistant
    );
    let profile = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(profile.source_item_id.as_deref(), Some(id.as_str()));
    assert_eq!(
        (profile.damage_dice, profile.damage_sides, profile.to_hit),
        (5, 7, 19)
    );
    // Slays and the electricity brand take the strongest applicable multiplier.
    for (target, electricity, multiplier) in [
        ("baby-blue-dragon", ResistanceLevel::Immune, 56),
        ("skeleton-human", ResistanceLevel::Immune, 28),
        ("manes", ResistanceLevel::Immune, 28),
        ("sheep", ResistanceLevel::Normal, 24),
        ("sheep", ResistanceLevel::Immune, 10),
    ] {
        clear_monsters(&mut game);
        game.push_generated_actor(
            "test.aule-target".into(),
            &format!("demo.actor.{target}"),
            adjacent,
        );
        game.entities[0].hp = 10_000;
        game.entities[0].max_hp = 10_000;
        game.entities[0]
            .resistances
            .set(DamageType::Electricity, electricity);
        assert_eq!(
            game.player_melee_damage_multiplier(
                &profile,
                &game.entities[0],
                game.content.actor(&game.entities[0].kind_id).unwrap()
            ),
            multiplier
        );
    }
    let seed = (0..100)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            strike(&mut trial);
            trial.entities[0].hp < 10_000
        })
        .expect("controlled seed range includes a real Aule hit");
    // Restore the target's source HP bounds before exercising saved combat.
    game.entities[0].max_hp = game
        .content
        .actor(&game.entities[0].kind_id)
        .unwrap()
        .max_hp;
    game.entities[0].hp = 1;
    game.rng = RfbRng::seeded(seed);
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.items[0].intrinsic_properties, intrinsic);
    assert_eq!(restored.items[0].rolled_affixes, rolled);
    assert_eq!(strike(&mut restored), strike(&mut game));
    assert!(
        restored
            .entities
            .iter()
            .all(|actor| actor.id != "test.aule-target")
    );
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    assert!(restored.generated_artifact_ids.contains("demo.item.aule"));
    assert_eq!(
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        game.generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap()
    );
    assert_eq!(restored.rng, game.rng);
    assert_ne!(
        restored.roll_fixed_artifact_kind_id(&context, Some("demo.item.great-hammer"), false),
        Some("demo.item.aule".into())
    );
}

#[test]
fn a5_res_or_power_weighted_boundaries_keep_one_draw_and_duplicate_flags() {
    let original = Game::new_with_build(427, "demo.build.warrior").unwrap();
    let context = artifact_loot_context(35);
    let affix = original
        .content
        .affix("rfb-legacy.affix.artifact-extra-res-or-power")
        .unwrap();
    let candidates = &affix.roll_groups[0].candidates;
    // Existing content coverage checks 12*15 + 8*18 + 9*4 = 360. This is
    // distribution-equivalent to the source branches, not source RNG parity.
    for slug in [
        "maedhros",
        "glamdring",
        "orcrist",
        "gurthang",
        "azaghal",
        "soulsword",
    ] {
        let kind = format!("demo.item.{slug}");
        let mut start = 0;
        for candidate in candidates {
            let end = start + u64::from(candidate.weight);
            for raw in [start, end - 1] {
                let mut game = original.clone();
                game.items.clear();
                let seed = (0..10_000)
                    .find(|seed| RfbRng::seeded(*seed).bounded(360) == raw)
                    .unwrap();
                game.rng = RfbRng::seeded(seed);
                let mut expected_rng = game.rng.clone();
                expected_rng.bounded(360);
                let draft = game.fixed_item_draft(&context, kind.clone());
                assert_eq!(game.rng, expected_rng);
                assert_eq!(draft.rolled_affixes.len(), 1);
                let definition = game.content.item(&kind).unwrap();
                let mut expected = candidate.properties.clone();
                let light = definition
                    .equipment_bonuses
                    .light_radius
                    .max(expected.equipment_bonuses.light_radius);
                if definition.equipment_bonuses.light_radius == 1
                    && expected.equipment_bonuses.light_radius == 1
                {
                    expected.equipment_bonuses.light_radius = 0;
                    expected.rfb_flags.insert("LITE".into());
                }
                assert_eq!(draft.rolled_affixes[0].properties, expected);
                let item = game
                    .commit_generated_item_draft(draft, ItemLocation::Inventory)
                    .unwrap();
                let id = item.id.clone();
                game.items.push(item);
                game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
                game.equip_inventory_item(&id, Some("right-hand")).unwrap();
                assert_eq!(
                    game.item_equipment_bonuses(&game.items[0]).light_radius,
                    light
                );
                assert_eq!(
                    game.visible_item_equipment_bonuses(&game.items[0])
                        .light_radius,
                    light
                );
                for &element in expected.resistances.keys() {
                    assert_eq!(
                        game.effective_player_resistances().level(element.into()),
                        ResistanceLevel::Resistant
                    );
                    assert_eq!(
                        game.visible_item_resistances(&game.items[0])
                            .iter()
                            .filter(
                                |r| r.damage_type == DamageTypeDto::from(DamageType::from(element))
                            )
                            .count(),
                        1
                    );
                }
                assert_eq!(game.rng, expected_rng);
            }
            start = end;
        }
        assert_eq!(start, 360);
    }
}

#[test]
fn a5_weapons_generate_fight_and_keep_random_properties_after_save() {
    fn strike(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        events
    }
    for (slug, base, raw, target_kind, multiplier) in [
        ("maedhros", "main-gauche", 336, "stone-troll", 28),
        ("glamdring", "broad-sword", 198, "snaga", 56),
        ("orcrist", "broad-sword", 198, "snaga", 56),
        ("gurthang", "two-handed-sword", 288, "baby-blue-dragon", 56),
        ("azaghal", "main-gauche", 0, "baby-blue-dragon", 56),
    ] {
        let mut game = Game::new_with_build(428, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        descend_one_floor(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        let adjacent = Position { x: 11, y: 10 };
        replace_terrain(&mut game, Position { x: 10, y: 10 }, "demo.terrain.floor");
        replace_terrain(&mut game, adjacent, "demo.terrain.floor");
        game.glow.fill(true);
        let context = artifact_loot_context(35);
        let kind = format!("demo.item.{slug}");
        let base = format!("demo.item.{base}");
        let selected = (0..5000)
            .find_map(|seed| {
                game.rng = RfbRng::seeded(seed);
                game.roll_fixed_artifact_kind_id(&context, Some(&base), false)
                    .filter(|id| id == &kind)
            })
            .expect("A5 ordinary candidate pool must include each weapon at controlled base/depth");
        // Control the extra roll to cover duplicate ESP, LITE, FREE_ACT and a
        // resistance. Selection above still uses real source rarity draws.
        game.rng = RfbRng::seeded(
            (0..10_000)
                .find(|seed| RfbRng::seeded(*seed).bounded(360) == raw)
                .unwrap(),
        );
        let draft = game.fixed_item_draft(&context, selected);
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        assert!(item.activation.is_none() && item.curse.is_none());
        let rolled = item.rolled_affixes.clone();
        let id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        assert!(game.known_item_properties(&game.items[0]).is_empty());
        game.reveal_current_visibility();
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.items[0].rolled_affixes, rolled);
        game = restored;
        let rng = game.rng.clone();
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        assert_eq!(game.rng, rng);
        assert!(
            game.item_property_knowledge[&id]
                .known_affix_ids
                .contains(&rolled[0].affix_id)
        );
        game.equip_inventory_item(&id, Some("right-hand")).unwrap();
        match slug {
            "maedhros" => {
                let modifiers = game.equipment_modifiers();
                assert_eq!(
                    (modifiers.intelligence, modifiers.dexterity, modifiers.speed),
                    (3, 3, 3)
                );
                assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
                assert_eq!(game.player_see_invisible_sources(), 1);
            }
            "glamdring" | "orcrist" => {
                assert_eq!(game.player_light_radius(), Some(1));
                assert_eq!(
                    game.visible_item_equipment_bonuses(&game.items[0])
                        .light_radius,
                    1
                );
                assert!(rolled[0].properties.rfb_flags.contains("LITE"));
                assert!(game.player_slow_digestion());
                if slug == "glamdring" {
                    assert_eq!(game.player_equipment_bonuses().search_skill, 15);
                    assert_eq!(game.player_equipment_bonuses().perception_skill, 15);
                    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
                } else {
                    assert_eq!(game.player_equipment_bonuses().stealth_skill, 3);
                }
            }
            "gurthang" => {
                assert_eq!(game.equipment_modifiers().strength, 2);
                assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
                assert!(game.player_slow_digestion());
                assert!(
                    game.player_equipment_passives()
                        .contains(&EquipmentPassive::Regeneration)
                );
            }
            "azaghal" => {
                assert_eq!(
                    game.effective_player_resistances().level(DamageType::Fire),
                    ResistanceLevel::Immune
                );
                assert_eq!(
                    game.effective_player_resistances()
                        .level(DamageType::Poison),
                    ResistanceLevel::Resistant
                );
                let hp = game.player.hp;
                game.resolve_monster_damage_to_player(
                    "test.fire",
                    "demo.actor.small-kobold",
                    "test.fire-bolt",
                    0,
                    30,
                    30,
                    DamageType::Fire,
                    &mut Vec::new(),
                );
                assert_eq!(game.player.hp, hp);
            }
            _ => unreachable!(),
        }
        game.push_generated_actor(
            "test.a5-target".into(),
            &format!("demo.actor.{target_kind}"),
            adjacent,
        );
        game.entities[0].hp = 10_000;
        game.entities[0].max_hp = 10_000;
        if slug != "gurthang" {
            assert!(game.entity_is_visible_by_telepathy(&game.entities[0]));
        }
        let profile = game.player_melee_profile(&game.player_derived_stats());
        assert_eq!(profile.source_item_id.as_deref(), Some(id.as_str()));
        assert_eq!(
            game.player_melee_damage_multiplier(
                &profile,
                &game.entities[0],
                game.content.actor(&game.entities[0].kind_id).unwrap()
            ),
            multiplier
        );
        if matches!(slug, "glamdring" | "orcrist") {
            // A neutral target isolates the brand from the stronger orc kill.
            let mut elemental = game.clone();
            clear_monsters(&mut elemental);
            elemental.push_generated_actor("test.brand".into(), "demo.actor.sheep", adjacent);
            elemental.entities[0].hp = 10_000;
            elemental.entities[0].max_hp = 10_000;
            let element = if slug == "glamdring" {
                DamageType::Fire
            } else {
                DamageType::Cold
            };
            let seed = (0..100)
                .find(|seed| {
                    let mut trial = elemental.clone();
                    trial.rng = RfbRng::seeded(*seed);
                    strike(&mut trial);
                    trial.entities[0].hp < 10_000
                })
                .unwrap();
            let mut hits = Vec::new();
            for (resistance, expected) in
                [(ResistanceLevel::Normal, 24), (ResistanceLevel::Immune, 10)]
            {
                let mut trial = elemental.clone();
                trial.entities[0].resistances.set(element, resistance);
                trial.rng = RfbRng::seeded(seed);
                assert_eq!(
                    trial.player_melee_damage_multiplier(
                        &profile,
                        &trial.entities[0],
                        trial.content.actor("demo.actor.sheep").unwrap()
                    ),
                    expected
                );
                strike(&mut trial);
                hits.push(10_000 - trial.entities[0].hp);
            }
            assert!(hits[0] > hits[1] && hits[1] > 0);
        }
        let seed = (0..100)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                strike(&mut trial);
                trial.entities[0].hp < 10_000
            })
            .expect("controlled seed range includes a real weapon hit");
        // Restore the target's source HP bounds before exercising saved combat.
        game.entities[0].max_hp = game
            .content
            .actor(&game.entities[0].kind_id)
            .unwrap()
            .max_hp;
        game.entities[0].hp = 1;
        game.rng = RfbRng::seeded(seed);
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.items[0].rolled_affixes, rolled);
        assert_eq!(strike(&mut restored), strike(&mut game));
        assert!(
            restored
                .entities
                .iter()
                .all(|actor| actor.id != "test.a5-target")
        );
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
        assert!(restored.generated_artifact_ids.contains(&kind));
        assert_eq!(
            restored
                .generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap(),
            game.generate_loot_instances(&context, ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(restored.rng, game.rng);
        assert_ne!(
            restored.roll_fixed_artifact_kind_id(&context, Some(&base), false),
            Some(kind)
        );
    }
}

#[test]
fn p100e_soulsword_rolls_and_persists_one_extra_power_and_increases_life() {
    let mut game =
        Game::new_with_build(100, "demo.build.warrior").expect("Soulsword game should create");
    clear_monsters(&mut game);
    for item in game.items.iter_mut().filter(
        |item| matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "right-hand"),
    ) {
        item.location = ItemLocation::Inventory;
    }
    let base_max_hp = game.player_derived_stats().max_hp.value;
    let context = LootContext {
        table_id: "demo.loot-table.graveyard-final-replacement".to_owned(),
        floor_id: "demo.floor.graveyard-depth-70".to_owned(),
        depth: 70,
        source: LootSource::FloorRoom {
            room_id: "demo.guardian.graveyard.1".to_owned(),
            spawn_id: "demo.guardian.graveyard.reward".to_owned(),
        },
    };
    game.rng = RfbRng::seeded(100);
    let draft = game.fixed_item_draft(&context, "demo.item.soulsword".to_owned());
    assert_eq!(
        draft.affix_ids,
        ["rfb-legacy.affix.artifact-extra-res-or-power"]
    );
    let [rolled] = draft.rolled_affixes.as_slice() else {
        panic!("Soulsword should roll exactly one affix bundle");
    };
    assert_eq!(
        rolled.affix_id,
        "rfb-legacy.affix.artifact-extra-res-or-power"
    );
    assert_ne!(rolled.properties, AffixPropertyBundleDefinition::default());

    let rolled_properties = rolled.properties.clone();
    let item = game
        .commit_generated_item_draft(
            draft,
            ItemLocation::Equipped {
                slot_id: "right-hand".to_owned(),
            },
        )
        .expect("Soulsword draft should commit");
    let item_id = item.id.clone();
    game.item_property_knowledge.insert(
        item_id.clone(),
        ItemPropertyKnowledgeState {
            known_blessed: false,
            discovered: true,
            appraised: true,
            identified: true,
            feeling: None,
            known_affix_ids: BTreeSet::from([
                "rfb-legacy.affix.artifact-extra-res-or-power".to_owned()
            ]),
        },
    );
    game.items.push(item);
    assert_eq!(
        game.player_derived_stats().max_hp.value,
        base_max_hp.saturating_mul(109).saturating_div(100)
    );

    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Soulsword should restore");
    assert_eq!(restored.state_hash(), hash);
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == item_id)
        .expect("restored Soulsword");
    assert_eq!(
        restored_item.rolled_affixes[0].properties,
        rolled_properties
    );
    assert!(
        restored
            .generated_artifact_ids
            .contains("demo.item.soulsword")
    );
}

#[test]
fn p100e_soulsword_warning_reveals_and_stops_before_a_hidden_trap() {
    let mut game =
        Game::new_with_build(101, "demo.build.warrior").expect("warning game should create");
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    let start = game.player.position;
    for item in game.items.iter_mut().filter(
        |item| matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "right-hand"),
    ) {
        item.location = ItemLocation::Inventory;
    }

    let warning_seed = (0..10_000)
        .find(|seed| {
            let roll = RfbRng::seeded(*seed).bounded(360);
            (234..252).contains(&roll)
        })
        .expect("a warning-property seed should exist");
    game.rng = RfbRng::seeded(warning_seed);
    let context = artifact_loot_context(70);
    let draft = game.fixed_item_draft(&context, "demo.item.soulsword".to_owned());
    assert!(
        draft.rolled_affixes[0]
            .properties
            .passives
            .contains(&EquipmentPassive::Warning)
    );
    let item = game
        .commit_generated_item_draft(
            draft,
            ItemLocation::Equipped {
                slot_id: "right-hand".to_owned(),
            },
        )
        .expect("warning Soulsword should commit");
    game.items.push(item);

    let trap = Position {
        x: start.x + 1,
        y: start.y,
    };
    replace_terrain(&mut game, trap, "demo.terrain.warren-snare");
    game.revealed_terrain.remove(&trap);
    let warning_roll_seed = (0..1_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(13) != 0)
        .expect("a warning success seed should exist");
    game.rng = RfbRng::seeded(warning_roll_seed);

    let warned = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, start);
    assert!(game.revealed_terrain.contains(&trap));
    assert!(
        warned
            .events
            .iter()
            .any(|event| event.kind == "item.warning-trap")
    );

    let moved = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, trap);
    assert!(
        moved
            .events
            .iter()
            .any(|event| event.kind == "terrain.trap-triggered")
    );
}

#[test]
fn booze_applies_original_confusion_hallucination_and_blackout_ranges() {
    let mut saw_hallucination = false;
    let mut saw_clear_head = false;
    let mut saw_blackout = false;
    for seed in 0..256 {
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        clear_monsters(&mut game);
        game.rng = RfbRng::seeded(seed);
        game.explored.fill(true);
        let origin = game.player.position;
        game.resolve_item_booze(
            "demo.item.booze-potion",
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );

        let confusion = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_CONFUSION)
            .expect("an unresisted drink should always confuse a Warrior");
        assert!((160..=350).contains(&confusion.remaining_ticks));
        if let Some(hallucination) = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_HALLUCINATION)
        {
            assert!((260..=500).contains(&hallucination.remaining_ticks));
            saw_hallucination = true;
        } else {
            saw_clear_head = true;
        }
        if game.player.position != origin {
            assert!(game.explored.iter().all(|explored| !explored));
            saw_blackout = true;
        }
    }
    assert!(saw_hallucination && saw_clear_head && saw_blackout);
}

#[test]
fn booze_keeps_a_longer_existing_confusion_duration() {
    let mut game =
        Game::new_with_build(0, "demo.build.warrior").expect("Warrens journey should create");
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_CONFUSION.to_owned(),
        remaining_ticks: 10_000,
        intensity: 1,
        source_id: Some("test.existing-confusion".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });

    game.resolve_item_booze(
        "demo.item.booze-potion",
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );

    let confusion = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_CONFUSION)
        .expect("existing confusion should remain");
    assert_eq!(confusion.remaining_ticks, 10_000);
    assert_eq!(
        confusion.source_id.as_deref(),
        Some("test.existing-confusion")
    );
}

#[test]
fn booze_refreshes_existing_statuses_without_identifying_itself() {
    let mut verified = false;
    for seed in 0..256 {
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        clear_monsters(&mut game);
        game.rng = RfbRng::seeded(seed);
        game.mark_item_tried("demo.item.booze-potion");
        for status_kind_id in [STATUS_CONFUSION, STATUS_HALLUCINATION] {
            game.player.statuses.push(StatusInstance {
                kind_id: status_kind_id.to_owned(),
                remaining_ticks: 10,
                intensity: 1,
                source_id: Some("test.existing-status".to_owned()),
                granted_resistances: BTreeMap::new(),
                granted_brands: BTreeSet::new(),
                granted_modifiers: StatModifiersDto::default(),
                granted_equipment_bonuses: EquipmentBonusesDto::default(),
                granted_status_immunities: BTreeSet::new(),
                granted_race_id: None,
                grants_wall_passage: false,
                incoming_damage_percent: 100,
            });
        }
        let mut events = Vec::new();
        game.resolve_item_booze("demo.item.booze-potion", &mut events, &mut BTreeSet::new());
        if events
            .iter()
            .any(|event| matches!(event, DomainEvent::ItemTeleported { .. }))
        {
            continue;
        }

        assert!(
            game.player
                .statuses
                .iter()
                .find(|status| status.kind_id == STATUS_CONFUSION)
                .is_some_and(|status| status.remaining_ticks > 10)
        );
        assert!(
            events.iter().all(|event| !matches!(
                event,
                DomainEvent::ItemStatusResolved { noticed: true, .. }
            ))
        );
        assert_eq!(
            game.item_knowledge_dto("demo.item.booze-potion"),
            ItemKnowledgeDto::Tried
        );
        verified = true;
        break;
    }
    assert!(verified, "a non-blackout booze seed should be available");
}

#[test]
fn restorative_item_sequence_recovers_resource_then_removes_status() {
    const ITEM_ID: &str = "test.item.clarity-draught.1";
    let mut game = test_caster_game(19);
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana")
        .current = 0;
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_CONFUSION.to_owned(),
        remaining_ticks: 20,
        intensity: 1,
        source_id: Some("test".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(&mut game, ITEM_ID, "demo.item.clarity-draught");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert!(game.resources["demo.resource.mana"].current > 0);
    assert!(!game.player_has_status_kind(STATUS_CONFUSION));
    let effect_events = update
        .events
        .iter()
        .filter(|event| event.kind.starts_with("item.use-"))
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        effect_events,
        vec![
            "item.use-resource-restored",
            "item.use-status-removed",
            "item.use-food"
        ]
    );
    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_BIRTH + 100);
}

#[test]
fn resource_restorative_preserves_awareness_order_and_missing_resource_semantics() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    for (case, seed, has_mana, has_berserk) in [
        ("restoration-and-cure", 23, true, true),
        ("restoration-before-no-effect", 27, true, false),
        ("missing-resource", 29, false, false),
    ] {
        let mut game = if has_mana {
            test_caster_game(seed)
        } else {
            skill_check_game(seed, "demo.build.warrior")
        };
        clear_monsters(&mut game);
        let before = u32::from(has_berserk);
        let maximum = if has_mana {
            let mana = game
                .resources
                .get_mut("demo.resource.mana")
                .expect("caster mana");
            mana.current = before;
            mana.maximum
        } else {
            assert!(!game.resources.contains_key("demo.resource.mana"), "{case}");
            0
        };
        if has_berserk {
            game.player
                .statuses
                .push(monster_combat::melee_status(STATUS_BERSERK, 20, "test").status);
        }
        give_inventory_item(&mut game, ITEM_ID, "demo.item.perfect-focus-elixir");
        let draws_before = game.rng_draw_counter();
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: None,
            },
        );
        assert!(!game.items.iter().any(|item| item.id == ITEM_ID), "{case}");
        assert_eq!(game.rng_draw_counter(), draws_before, "{case}");
        assert!(!game.player_has_status_kind(STATUS_BERSERK), "{case}");
        assert!(
            update.events.iter().any(|event| matches!(&event.outcome,
            Some(GameEventOutcomeDto::ResourceRecovery { resolution })
                if resolution.before == before && resolution.after == maximum
                    && resolution.recovered == maximum - before)),
            "{case}"
        );
        if has_mana {
            assert_eq!(
                game.resources["demo.resource.mana"].current, maximum,
                "{case}"
            );
            if has_berserk {
                let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
                    .expect("restored resource state should reload");
                assert_eq!(restored.snapshot(), game.snapshot(), "{case}");
            } else {
                let event = update
                    .events
                    .iter()
                    .find(|event| event.kind == "item.use-status-no-effect")
                    .expect("absent berserk should report no effect");
                assert_eq!(
                    event.args["nameKey"], "item-demo-perfect-focus-elixir-name",
                    "{case}"
                );
            }
        } else {
            assert!(
                update
                    .events
                    .iter()
                    .any(|event| event.kind == "item.use-resource-no-effect"),
                "{case}"
            );
            assert!(
                game.item_knowledge
                    .get("demo.item.perfect-focus-elixir")
                    .is_some_and(|knowledge| knowledge.tried && !knowledge.aware),
                "{case}"
            );
        }
    }
}

#[test]
fn identify_scroll_rejects_missing_and_self_targets_before_consumption() {
    const SCROLL_ID: &str = "test.item.invalid-identify-scroll.1";
    let mut game = skill_check_game(41, "demo.build.warrior");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.appraisal-scroll");

    for target_item_id in ["missing.item", SCROLL_ID] {
        let draws_before = game.rng_draw_counter();
        let tick_before = game.world_tick;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::Item {
                    item_id: target_item_id.to_owned(),
                }),
            },
        );
        assert_eq!(update.events[0].kind, "item.use-unavailable");
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert_eq!(game.world_tick, tick_before);
        assert!(game.items.iter().any(|item| item.id == SCROLL_ID));
        assert_eq!(
            game.item_knowledge_dto("demo.item.appraisal-scroll"),
            ItemKnowledgeDto::Unknown
        );
    }
}

#[test]
fn enchantment_artifact_and_ammunition_pile_gates_follow_original_order() {
    let artifact_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.warrior");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 1, false, false, false);
        let mut artifact = skill_check_game(*seed, "demo.build.warrior");
        artifact.rng = RfbRng::seeded(*seed);
        let artifact = artifact.resolve_item_enchantment_component(0, 1, 1, false, true, false);
        ordinary.successes == 1 && artifact.successes == 0
    });
    assert_eq!(artifact_seed, Some(0));

    let ammunition_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.warrior");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 20, false, false, false);
        let mut ammunition = skill_check_game(*seed, "demo.build.warrior");
        ammunition.rng = RfbRng::seeded(*seed);
        let ammunition =
            ammunition.resolve_item_enchantment_component(0, 1, 20, true, false, false);
        ordinary.successes == 0 && ammunition.successes == 1
    });
    assert_eq!(ammunition_seed, Some(0));
}

#[test]
fn curse_scroll_lands_on_equipped_weapon_and_artifact_can_resist() {
    fn run(resisted: bool) -> (Game, GameUpdate, u64) {
        const SCROLL_ID: &str = "test.item.weapon-blight-scroll.1";
        const WEAPON_ID: &str = "test.item.relic-blade.1";
        let mut game = skill_check_game(61, "demo.build.warrior");
        for item in game
            .items
            .iter_mut()
            .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        {
            item.location = ItemLocation::Inventory;
        }
        give_inventory_item(&mut game, SCROLL_ID, "demo.item.weapon-blight-scroll");
        give_inventory_item(&mut game, WEAPON_ID, "demo.item.soulsword");
        game.items
            .iter_mut()
            .find(|item| item.id == WEAPON_ID)
            .expect("relic blade should exist")
            .location = ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        };
        game.debug_set_item_curses_land(!resisted);
        game.debug_set_item_curses_resisted(resisted);
        let draws_before = game.rng_draw_counter();
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::SelfTarget),
            },
        );
        (game, update, draws_before)
    }

    let (landed, update, draws_before) = run(false);
    assert_eq!(landed.rng_draw_counter(), draws_before + 4);
    let blasted = landed
        .items
        .iter()
        .find(|item| item.id == "test.item.relic-blade.1")
        .unwrap();
    assert_eq!(blasted.kind_id, "demo.item.scimitar");
    assert_eq!(blasted.affix_ids, ["rfb-legacy.affix.blasted"]);
    assert_eq!(update.events[0].kind, "item.use-cursed");
    assert_eq!(
        landed
            .items
            .iter()
            .find(|item| item.id == "test.item.relic-blade.1")
            .expect("relic blade should remain equipped")
            .curse,
        Some(ItemCurseSeverityDto::Normal)
    );
    assert_eq!(
        landed.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Aware
    );
    assert!(update.events.iter().any(|event| {
        matches!(
            &event.outcome,
            Some(GameEventOutcomeDto::ItemCurse { resolution })
                if resolution.item_id.as_deref() == Some("test.item.relic-blade.1")
                    && resolution.before.is_none()
                    && resolution.after == Some(ItemCurseSeverityDto::Normal)
                    && !resolution.resisted
        )
    }));

    let (resisted, update, draws_before) = run(true);
    assert_eq!(resisted.rng_draw_counter(), draws_before);
    assert_eq!(update.events[0].kind, "item.use-curse-resisted");
    assert_eq!(
        resisted
            .items
            .iter()
            .find(|item| item.id == "test.item.relic-blade.1")
            .expect("relic blade should remain equipped")
            .curse,
        None
    );
    assert_eq!(
        resisted.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Aware
    );
}

#[test]
fn curse_scroll_without_a_matching_equipped_item_consumes_without_rng_or_awareness() {
    const SCROLL_ID: &str = "test.item.weapon-blight-scroll.no-target";
    let mut game = skill_check_game(67, "demo.build.warrior");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.weapon-blight-scroll");
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    assert_eq!(update.events[0].kind, "item.use-curse-no-target");
    assert_eq!(
        game.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Tried
    );
}

#[test]
fn cleansing_scrolls_respect_heavy_and_permanent_curse_boundaries() {
    const NORMAL_ID: &str = "test.item.normal-curse";
    const HEAVY_ID: &str = "test.item.heavy-curse";
    const PERMANENT_ID: &str = "test.item.permanent-curse";
    let mut game = skill_check_game(71, "demo.build.warrior");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    for (id, kind_id, slot_id, curse) in [
        (
            NORMAL_ID,
            "demo.item.relic-blade",
            "right-hand",
            ItemCurseSeverityDto::Normal,
        ),
        (
            HEAVY_ID,
            "demo.item.burdened-mail",
            "body",
            ItemCurseSeverityDto::Heavy,
        ),
        (
            PERMANENT_ID,
            "demo.item.sealed-amulet",
            "neck",
            ItemCurseSeverityDto::Permanent,
        ),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("curse test item should exist");
        item.location = ItemLocation::Equipped {
            slot_id: slot_id.to_owned(),
        };
        item.curse = Some(curse);
    }
    give_inventory_item(
        &mut game,
        "test.item.cleansing-scroll.1",
        "demo.item.cleansing-scroll",
    );
    let ordinary = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.cleansing-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert_eq!(ordinary.events[0].kind, "item.use-curses-removed");
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == NORMAL_ID)
            .unwrap()
            .curse,
        None
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == HEAVY_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Heavy)
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == PERMANENT_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Permanent)
    );

    give_inventory_item(
        &mut game,
        "test.item.greater-cleansing-scroll.1",
        "demo.item.greater-cleansing-scroll",
    );
    let greater = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.greater-cleansing-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert_eq!(greater.events[0].kind, "item.use-curses-removed");
    let resolution = greater
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::ItemCurseRemoval { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("greater cleansing should emit a structured resolution");
    assert_eq!(resolution.removed_item_ids, [HEAVY_ID]);
    assert_eq!(resolution.retained_permanent_item_ids, [PERMANENT_ID]);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == HEAVY_ID)
            .unwrap()
            .curse,
        None
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == PERMANENT_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Permanent)
    );
    let saved = game.to_save();
    let restored = Game::from_save(saved.clone()).expect("curse severities should round-trip");
    for (item_id, expected) in [
        (HEAVY_ID, None),
        (PERMANENT_ID, Some(ItemCurseSeverityDto::Permanent)),
    ] {
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == item_id)
                .unwrap()
                .curse,
            expected
        );
    }
}

#[test]
fn spell_scroll_increases_only_eligible_learning_capacity_without_rng() {
    const ITEM_ID: &str = "test.item.spell-scroll";
    const KIND_ID: &str = "demo.item.spell-scroll";

    let mut caster = test_caster_game(17);
    clear_monsters(&mut caster);
    give_inventory_item(&mut caster, ITEM_ID, KIND_ID);
    let capacity_before = caster
        .snapshot()
        .player
        .ability_learning
        .expect("test caster should expose learning capacity")
        .capacity;
    let draws_before = caster.rng_draw_counter();
    let update = dispatch_next(
        &mut caster,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(caster.rng_draw_counter(), draws_before);
    assert_eq!(caster.bonus_spell_learning_capacity, 1);
    assert_eq!(
        caster
            .snapshot()
            .player
            .ability_learning
            .expect("test caster should retain learning capacity")
            .capacity,
        capacity_before + 1
    );
    let capacity_before_arg = capacity_before.to_string();
    let capacity_after_arg = capacity_before.saturating_add(1).to_string();
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-spell-learning-capacity-increased"
            && event.args.get("before").map(String::as_str) == Some(capacity_before_arg.as_str())
            && event.args.get("after").map(String::as_str) == Some(capacity_after_arg.as_str())
    }));
    assert_eq!(caster.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    let restored = Game::from_save_with_content(caster.to_save(), caster.content.clone())
        .expect("spell bonus should round trip");
    assert_eq!(restored.state_hash(), caster.state_hash());

    let mut warrior =
        Game::new_with_build(17, "demo.build.warrior").expect("Warrior build should create");
    clear_monsters(&mut warrior);
    give_inventory_item(&mut warrior, ITEM_ID, KIND_ID);
    let draws_before = warrior.rng_draw_counter();
    let tick_before = warrior.world_tick;
    let update = dispatch_next(
        &mut warrior,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(warrior.rng_draw_counter(), draws_before);
    assert_eq!(warrior.world_tick, tick_before + 10);
    assert_eq!(warrior.bonus_spell_learning_capacity, 0);
    assert!(!warrior.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(warrior.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    assert!(
        update
            .events
            .iter()
            .any(|event| { event.kind == "item.use-spell-learning-capacity-no-effect" })
    );

    let mut invalid = warrior.to_save();
    invalid.player.bonus_spell_learning_capacity = 1;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave(
            "bonus spell learning capacity is invalid"
        ))
    ));
}

#[test]
fn veil_draught_awareness_and_rng_follow_existing_blindness_and_immunity() {
    const ITEM_ID: &str = "test.item.veil-draught";
    const KIND_ID: &str = "demo.item.veil-draught";

    for (existing_blindness, expected_draws, expected_event) in [
        (true, 2, "item.use-blindness-no-new-effect"),
        (false, 1, "item.use-blindness-resisted"),
    ] {
        let mut game = Game::new(94);
        clear_monsters(&mut game);
        give_inventory_item(&mut game, ITEM_ID, KIND_ID);
        game.player.statuses.push(StatusInstance {
            kind_id: if existing_blindness {
                STATUS_BLINDNESS.to_owned()
            } else {
                "test.status.blindness-immunity".to_owned()
            },
            intensity: 1,
            remaining_ticks: 20,
            source_id: Some(if existing_blindness {
                "test.existing-blindness".to_owned()
            } else {
                "test.blindness-immunity".to_owned()
            }),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: if existing_blindness {
                BTreeSet::new()
            } else {
                BTreeSet::from([STATUS_BLINDNESS.to_owned()])
            },
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let draws_before = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: None,
            },
        );

        assert_eq!(game.rng_draw_counter(), draws_before + expected_draws);
        assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
        assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Tried);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == expected_event)
        );
        let blindness = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_BLINDNESS);
        if existing_blindness {
            let blindness = blindness.expect("the blindness duration should extend");
            assert!((110..=209).contains(&blindness.remaining_ticks));
            assert_eq!(
                blindness.source_id.as_deref(),
                Some("test.existing-blindness")
            );
        } else {
            assert!(blindness.is_none());
        }
    }
}

#[test]
fn fury_draught_awareness_depends_on_new_berserk_or_actual_healing() {
    const ITEM_ID: &str = "test.item.fury-draught";
    const KIND_ID: &str = "demo.item.fury-draught";

    for (damage, expected_knowledge, expected_heal_kind) in [
        (10, ItemKnowledgeDto::Aware, "item.use-heal"),
        (0, ItemKnowledgeDto::Tried, "item.use-no-effect"),
    ] {
        let mut game = Game::new(90);
        clear_monsters(&mut game);
        give_inventory_item(&mut game, ITEM_ID, KIND_ID);
        game.player.statuses.push(StatusInstance {
            kind_id: "rfb.status.berserk".to_owned(),
            intensity: 1,
            remaining_ticks: 20,
            source_id: Some("test.existing-berserk".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto {
                defense: -10,
                max_hp: 30,
                ..StatModifiersDto::default()
            },
            granted_equipment_bonuses: EquipmentBonusesDto {
                melee_skill: 12,
                melee_damage: 3,
                ranged_skill: -12,
                throwing_skill: -20,
                device_skill: -20,
                saving_throw_skill: -30,
                stealth_skill: -7,
                search_skill: -15,
                perception_skill: -15,
                digging_skill: 30,
                ..EquipmentBonusesDto::default()
            },
            granted_status_immunities: BTreeSet::from([STATUS_FEAR.to_owned()]),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        game.player.hp = game.effective_player_max_hp() - damage;
        let expected_hp = game.effective_player_max_hp();
        let draws_before = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: None,
            },
        );

        assert_eq!(game.rng_draw_counter(), draws_before + 1);
        assert_eq!(game.player.hp, expected_hp);
        assert_eq!(game.item_knowledge_dto(KIND_ID), expected_knowledge);
        assert_eq!(
            update
                .events
                .iter()
                .take(2)
                .map(|event| event.kind.as_str())
                .collect::<Vec<_>>(),
            [
                "item.use-berserk-strength-no-new-effect",
                expected_heal_kind
            ]
        );
        let berserk = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == "rfb.status.berserk")
            .expect("the draught should extend Berserk");
        assert!((36..=60).contains(&berserk.remaining_ticks));
    }
}

#[test]
fn renewal_tonic_awareness_depends_on_either_restoration() {
    const ITEM_ID: &str = "test.item.renewal-tonic";
    const KIND_ID: &str = "demo.item.renewal-tonic";

    for (
        experience,
        maximum_experience,
        life_force,
        level,
        expected_life_force,
        expected_knowledge,
        expected_event_kind,
    ) in [
        (
            5,
            25,
            1_000,
            1,
            1_000,
            ItemKnowledgeDto::Aware,
            "item.use-restore-life-levels",
        ),
        (
            25,
            25,
            900,
            3,
            1_000,
            ItemKnowledgeDto::Aware,
            "item.use-restore-life-levels",
        ),
        (
            25,
            25,
            1_000,
            3,
            1_000,
            ItemKnowledgeDto::Tried,
            "item.use-restore-life-levels-no-effect",
        ),
    ] {
        let mut game = Game::new(93);
        clear_monsters(&mut game);
        game.progress.experience = experience;
        game.progress.maximum_experience = maximum_experience;
        game.progress.life_force = life_force;
        game.progress.level = level;
        game.progress.max_level = level;
        give_inventory_item(&mut game, ITEM_ID, KIND_ID);
        let draws_before = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: None,
            },
        );

        assert_eq!(game.progress.experience, maximum_experience);
        assert_eq!(game.progress.life_force, expected_life_force);
        assert_eq!(game.item_knowledge_dto(KIND_ID), expected_knowledge);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == expected_event_kind)
        );
    }
}

#[test]
fn shatterburst_draught_uses_damage_scaling_and_existing_status_stacking() {
    const ITEM_ID: &str = "test.item.shatterburst-draught";
    const KIND_ID: &str = "demo.item.shatterburst-draught";

    let mut game = Game::new(95);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    game.player.hp = 10_000;
    for (kind_id, intensity, remaining_ticks, source_id, incoming_damage_percent) in [
        (STATUS_STUN, 2, 100, "test.existing-stun", 100),
        (STATUS_BLEEDING, 2, 20, "test.existing-bleeding", 100),
        (
            "test.status.half-damage",
            1,
            100,
            "test.detonation-guard",
            50,
        ),
    ] {
        game.player.statuses.push(StatusInstance {
            kind_id: kind_id.to_owned(),
            intensity,
            remaining_ticks,
            source_id: Some(source_id.to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent,
        });
    }
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    let damage = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-detonation")
        .and_then(|event| match &event.outcome {
            Some(GameEventOutcomeDto::Damage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("detonation should report its nonfatal damage");
    assert_eq!(game.rng_draw_counter(), draws_before + 50);
    assert_eq!(damage.armor_reduction, 0);
    assert_eq!(damage.resistance, ResistanceLevelDto::Normal);
    assert_eq!(damage.final_damage, (damage.raw_damage + 1) / 2);
    let stun = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_STUN)
        .expect("detonation should retain existing stun");
    assert_eq!(stun.intensity, 2);
    assert_eq!(stun.remaining_ticks, 90);
    assert_eq!(stun.source_id.as_deref(), Some("test.existing-stun"));
    let bleeding = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_BLEEDING)
        .expect("detonation should extend existing bleeding");
    assert_eq!(bleeding.intensity, 2);
    assert_eq!(bleeding.remaining_ticks, 5_010);
    assert_eq!(
        bleeding.source_id.as_deref(),
        Some("test.existing-bleeding")
    );
}

#[test]
fn mortal_draught_life_loss_bypasses_incoming_damage_reduction_without_rng() {
    const ITEM_ID: &str = "test.item.mortal-draught";
    const KIND_ID: &str = "demo.item.mortal-draught";

    let mut game = Game::new(83);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    game.player.statuses.push(StatusInstance {
        kind_id: "test.status.half-damage".to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.life-loss-guard".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 50,
    });
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(game.player.hp, hp_before.saturating_sub(5000));
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn friendly_item_summons_are_permanent_controlled_and_round_trip() {
    let mut game = skill_check_game(68, "demo.build.warrior");
    give_inventory_item(
        &mut game,
        "test.item.pet-summoning-scroll.1",
        "demo.item.pet-summoning-scroll",
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.pet-summoning-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::ItemSummon { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("pet scroll should emit a summon resolution");
    assert!(!resolution.entity_ids.is_empty());
    assert!(!resolution.hostile);
    assert_eq!(resolution.duration_turns, 0);
    let summoned_ids = resolution.entity_ids.clone();
    assert!(summoned_ids.iter().all(|entity_id| {
        game.entities
            .iter()
            .find(|entity| entity.id == *entity_id)
            .is_some_and(|entity| {
                entity.controller_id.as_deref() == Some(game.player.id.as_str())
                    && entity.summon.is_none()
            })
    }));
    assert_eq!(
        game.item_knowledge_dto("demo.item.pet-summoning-scroll"),
        ItemKnowledgeDto::Aware
    );
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "test.item.pet-summoning-scroll.1")
    );

    let saved = game.to_save();
    let restored = Game::from_save(saved).expect("controlled item summons should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(summoned_ids.iter().all(|entity_id| {
        restored
            .entities
            .iter()
            .find(|entity| entity.id == *entity_id)
            .is_some_and(|entity| {
                entity.controller_id.as_deref() == Some(restored.player.id.as_str())
                    && entity.summon.is_none()
            })
    }));
}

#[test]
fn visible_actor_scrolls_consume_empty_results_without_rng_or_awareness() {
    for (seed, item_id, kind_id) in [
        (
            73,
            "test.item.empty-dispel-undead-scroll.1",
            "demo.item.dispel-undead-scroll",
        ),
        (
            74,
            "test.item.empty-banishment-scroll.1",
            "demo.item.banishment-scroll",
        ),
    ] {
        let mut game = skill_check_game(seed, "demo.build.warrior");
        super::dungeon_anti_magic::enter_context(&mut game);
        give_inventory_item(&mut game, item_id, kind_id);
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.use_inventory_item(
            item_id,
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("empty visible actor effect should resolve");
        assert_eq!(game.rng_draw_counter(), 0);
        assert!(!game.items.iter().any(|item| item.id == item_id));
        assert_eq!(game.item_knowledge_dto(kind_id), ItemKnowledgeDto::Tried);
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::ItemDispelNoEffect { .. }] | [DomainEvent::ItemBanishmentNoEffect { .. }]
        ));
    }
}

#[test]
fn mass_genocide_scroll_consumes_empty_result_with_awareness_and_zero_rng() {
    const ITEM_ID: &str = "test.item.severance-scroll.1";
    const KIND_ID: &str = "demo.item.severance-scroll";
    let mut game = skill_check_game(75, "demo.build.warrior");
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("empty mass genocide should resolve");

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.player.hp, hp_before);
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ItemMassGenocide {
            removed_count: 0,
            resisted_count: 0,
            fatigue_damage: 0,
            ..
        }]
    ));
}

#[test]
fn genocide_scroll_rejects_invalid_glyphs_and_consumes_an_empty_selection_without_rng() {
    const ITEM_ID: &str = "test.item.glyph-severance-scroll.1";
    const KIND_ID: &str = "demo.item.glyph-severance-scroll";
    let mut game = skill_check_game(79, "demo.build.warrior");
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    assert!(
        game.inventory_dto()
            .iter()
            .find(|item| item.id == ITEM_ID)
            .is_some_and(|item| item.requires_target_glyph)
    );
    let world_tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();

    for command in [
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
        GameCommand::UseItemByGlyph {
            item_id: ITEM_ID.to_owned(),
            glyph: "oo".to_owned(),
        },
    ] {
        let update = dispatch_next(&mut game, command);
        assert_eq!(update.world_tick, world_tick_before);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(game.items.iter().any(|item| item.id == ITEM_ID));
        assert_eq!(update.events[0].kind, "item.use-unavailable");
    }

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItemByGlyph {
            item_id: ITEM_ID.to_owned(),
            glyph: "x".to_owned(),
        },
    );
    assert!(update.world_tick > world_tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    assert_eq!(update.events[0].kind, "item.use-genocide");
    assert_eq!(
        update.events[0].args.get("glyph").map(String::as_str),
        Some("x")
    );
    assert_eq!(
        update.events[0].args.get("removed").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        update.events[0].args.get("resisted").map(String::as_str),
        Some("0")
    );
}

#[test]
fn adjacent_terrain_creation_consumes_empty_result_as_tried_without_rng() {
    const ITEM_ID: &str = "test.item.stone-ring-scroll.1";
    const KIND_ID: &str = "demo.item.stone-ring-scroll";
    let mut game = skill_check_game(76, "demo.build.warrior");
    let player_index = game
        .index(game.player.position)
        .expect("player position should be in bounds");
    game.terrain.fill("demo.terrain.wall".to_owned());
    game.terrain[player_index] = "demo.terrain.floor".to_owned();
    let item_position = Position { x: 4, y: 3 };
    let connection_position = Position { x: 3, y: 4 };
    replace_terrain(&mut game, item_position, "demo.terrain.floor");
    replace_terrain(&mut game, connection_position, "demo.terrain.floor");
    give_inventory_item(
        &mut game,
        "test.item.ground-blocker",
        "demo.item.ration-of-food",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.ground-blocker")
        .expect("ground blocker should exist")
        .location = ItemLocation::Ground(item_position);
    assert!(
        game.items
            .iter()
            .any(|item| item.location == ItemLocation::Ground(item_position))
    );
    game.floor_connections.push(FloorConnectionState {
        id: "test.connection.protected-floor".to_owned(),
        position: connection_position,
        target_floor_id: None,
        target_connection_id: None,
    });
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    let before = game.snapshot();
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(update.world_tick, before.world_tick + 10);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Tried);
    assert_eq!(update.events.len(), 1);
    assert_eq!(
        update.events[0].kind,
        "item.use-create-adjacent-terrain-no-effect"
    );
    assert_eq!(update.events[0].args["count"], "0");
}

#[test]
fn p3_4_light_and_darkness_reuse_persisted_floor_glow() {
    let mut game = skill_check_game(206, "demo.build.warrior");
    game.glow.fill(false);
    give_inventory_item(&mut game, "test.item.light.1", "demo.item.light-scroll");
    let hash_before = game.state_hash();
    let mut events = Vec::new();
    game.use_inventory_item(
        "test.item.light.1",
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("light scroll should resolve");
    assert!(game.glow.iter().any(|glow| *glow));
    assert_ne!(game.state_hash(), hash_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ItemFloorGlowChanged {
            glow: true,
            affected_positions,
            ..
        }] if !affected_positions.is_empty()
    ));

    let restored = Game::from_save(game.to_save()).expect("lit floor should reload");
    assert_eq!(restored.glow, game.glow);
    assert_eq!(restored.state_hash(), game.state_hash());

    give_inventory_item(
        &mut game,
        "test.item.darkness.1",
        "demo.item.darkness-scroll",
    );
    let mut darkness_events = Vec::new();
    game.use_inventory_item(
        "test.item.darkness.1",
        None,
        None,
        &mut darkness_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("darkness scroll should resolve");
    assert!(game.glow.iter().all(|glow| !*glow));
    assert!(darkness_events.iter().any(|event| matches!(
        event,
        DomainEvent::ItemFloorGlowChanged {
            glow: false,
            affected_positions,
            ..
        } if !affected_positions.is_empty()
    )));
}

#[test]
fn p3_4_rune_requires_clean_floor_and_uses_original_break_threshold() {
    let mut blocked = skill_check_game(207, "demo.build.warrior");
    let blocked_position = blocked.player.position;
    replace_terrain(&mut blocked, blocked_position, "demo.terrain.floor");
    blocked.gold_piles.push(GoldPile {
        id: "test.gold.rune-blocker".to_owned(),
        position: blocked.player.position,
        amount: 1,
        appearance: GoldAppearanceDto::Copper,
        discovered: true,
    });
    give_inventory_item(
        &mut blocked,
        "test.item.rune.blocked",
        "demo.item.rune-of-protection-scroll",
    );
    let mut blocked_events = Vec::new();
    blocked
        .use_inventory_item(
            "test.item.rune.blocked",
            None,
            None,
            &mut blocked_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("blocked rune should still consume the scroll");
    assert_eq!(
        blocked.terrain_at(blocked.player.position),
        "demo.terrain.floor"
    );
    assert!(matches!(
        blocked_events.as_slice(),
        [DomainEvent::ItemCreatedCurrentTerrain {
            affected_position: None,
            ..
        }]
    ));

    let mut game = game_with_actor_definition(208, "demo.actor.dread-vampire", |actor| {
        actor.level = 400;
    });
    clear_monsters(&mut game);
    let player_position = game.player.position;
    replace_terrain(&mut game, player_position, "demo.terrain.floor");
    give_inventory_item(
        &mut game,
        "test.item.rune.legal",
        "demo.item.rune-of-protection-scroll",
    );
    game.use_inventory_item(
        "test.item.rune.legal",
        None,
        None,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("legal rune should resolve");
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.warding-glyph"
    );
    let monster_position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, monster_position, "demo.terrain.floor");
    game.push_generated_actor(
        "test.actor.rune-breaker".to_owned(),
        "demo.actor.dread-vampire",
        monster_position,
    );
    let mut events = Vec::new();
    assert_eq!(
        game.try_monster_break_warding_glyph(
            0,
            game.player.position,
            &mut events,
            &mut BTreeSet::new(),
        ),
        Some(true)
    );
    assert_eq!(game.terrain_at(game.player.position), "demo.terrain.floor");
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::WardingGlyphBroken { .. }]
    ));
}

#[test]
fn vengeance_retaliates_against_monster_spells_but_not_after_player_death() {
    fn vengeance_status() -> StatusInstance {
        StatusInstance {
            kind_id: STATUS_VENGEANCE.to_owned(),
            intensity: 1,
            remaining_ticks: 100,
            source_id: Some("demo.item.reprisal-scroll".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        }
    }

    fn cinder_game(player_hp: i32) -> Game {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.wall".to_owned());
        let player = game.player.position;
        for step in 0..=3 {
            replace_terrain(
                &mut game,
                Position {
                    x: player.x + step,
                    y: player.y,
                },
                "demo.terrain.floor",
            );
        }
        game.player.hp = player_hp;
        game.player.statuses.push(vengeance_status());
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.vengeance-cinder",
            "demo.actor.cinder-adept",
            Position {
                x: player.x + 3,
                y: player.y,
            },
            20,
            100,
            100,
            true,
        ));
        game
    }

    fn first_cast(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        for _ in 0..100 {
            if game.resolve_monster_ability(0, &mut events) {
                return events;
            }
        }
        panic!("cinder adept should cast within 100 attempts");
    }

    let mut surviving = cinder_game(100);
    let events = first_cast(&mut surviving);
    let cast_damage = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::MonsterAbilityCast { resolution, .. } => {
                resolution.effects.iter().find_map(|effect| match effect {
                    AbilityEffectResolutionDto::Damage { resolution, .. } => {
                        Some(resolution.final_damage)
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("damaging monster cast should expose applied damage");
    let retaliation_damage = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::VengeanceHit { damage, .. }
            | DomainEvent::VengeanceSlew { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .expect("surviving player should retaliate");
    assert_eq!(retaliation_damage, cast_damage);
    assert_eq!(
        surviving
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_VENGEANCE)
            .expect("vengeance should remain active")
            .remaining_ticks,
        95
    );

    let mut dying = cinder_game(1);
    let source_hp = dying.entities[0].hp;
    let events = first_cast(&mut dying);
    assert!(dying.player_is_dead());
    assert_eq!(dying.entities[0].hp, source_hp);
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::VengeanceHit { .. } | DomainEvent::VengeanceSlew { .. }
    )));
    assert_eq!(dying.player.statuses[0].remaining_ticks, 100);
}

#[test]
fn travel_scroll_random_teleport_is_deterministic_and_rejects_without_space_atomically() {
    let prepare = || {
        let mut game = Game::new(64);
        clear_monsters(&mut game);
        give_inventory_item(
            &mut game,
            "test.item.flicker-scroll.1",
            "demo.item.flicker-scroll",
        );
        game
    };
    let mut first = prepare();
    let mut second = prepare();
    let first_update = dispatch_next(
        &mut first,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    let second_update = dispatch_next(
        &mut second,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(first.snapshot(), second.snapshot());
    assert_eq!(first_update.events, second_update.events);
    assert_ne!(first.player.position, Position { x: 3, y: 3 });
    assert!(first_update.events.iter().any(|event| {
        event.kind == "item.use-teleported"
            && matches!(
                event.outcome,
                Some(GameEventOutcomeDto::AbilityTeleport { .. })
            )
    }));

    let mut blocked = prepare();
    let player_index = blocked
        .index(blocked.player.position)
        .expect("player position should be in bounds");
    blocked.terrain.fill("demo.terrain.wall".to_owned());
    blocked.terrain[player_index] = "demo.terrain.floor".to_owned();
    let before = blocked.snapshot();
    let draw_counter = blocked.rng_draw_counter();
    let update = dispatch_next(
        &mut blocked,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(update.world_tick, before.world_tick);
    assert_eq!(blocked.rng_draw_counter(), draw_counter);
    assert!(
        blocked
            .items
            .iter()
            .any(|item| item.id == "test.item.flicker-scroll.1")
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
}

#[test]
fn p3_2_refreshments_are_deliberate_no_numeric_effects() {
    let mut game = Game::new(201);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.item.water.1", "demo.item.water-potion");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.water.1".to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == "test.item.water.1"));
    assert_eq!(
        game.item_knowledge_dto("demo.item.water-potion"),
        ItemKnowledgeDto::Aware
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-no-effect")
    );
}

#[test]
fn p3_2_lose_memories_preserves_historical_experience() {
    let mut game = Game::new(202);
    clear_monsters(&mut game);
    game.progress
        .gain_experience(1_000, game.character_experience_percent(), false);
    let maximum = game.progress.maximum_experience;
    give_inventory_item(
        &mut game,
        "test.item.lose-memories.1",
        "demo.item.lose-memories-potion",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.lose-memories.1".to_owned(),
            target: None,
        },
    );

    assert_eq!(game.progress.experience, 750);
    assert_eq!(game.progress.maximum_experience, maximum);
    let event = update
        .events
        .iter()
        .find(|event| event.kind == "item.experience-lost")
        .expect("experience loss should be projected");
    assert_eq!(event.args["amount"], "250");
    assert_eq!(event.args["remaining"], "750");
}

#[test]
fn p3_2_invulnerability_and_giant_strength_reuse_status_payloads() {
    let mut game = Game::new(204);
    clear_monsters(&mut game);
    let honour_before = game.virtue_current(VirtueKindDto::Honour);
    let valour_before = game.virtue_current(VirtueKindDto::Valour);
    give_inventory_item(
        &mut game,
        "test.item.invulnerability.1",
        "demo.item.invulnerability-potion",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.invulnerability.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(game.player_incoming_damage_percent(), 0);
    assert_eq!(
        game.virtue_current(VirtueKindDto::Honour),
        honour_before - 2
    );
    assert_eq!(
        game.virtue_current(VirtueKindDto::Valour),
        valour_before - 5
    );
    assert_eq!(
        game.reduce_player_damage(resolve_damage(
            DamagePacket::new(100, DamageType::Physical),
            ResistanceLevel::Normal,
        ))
        .applied,
        0
    );

    give_inventory_item(
        &mut game,
        "test.item.giant-strength.1",
        "demo.item.giant-strength-potion",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.giant-strength.1".to_owned(),
            target: None,
        },
    );
    let giant = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_GIANT_STRENGTH)
        .expect("giant strength should remain active");
    assert_eq!(giant.granted_modifiers.max_hp, 10);
    assert_eq!(giant.granted_equipment_bonuses.melee_skill, 1);
}

#[test]
fn p3_7_experience_potion_uses_unscaled_relative_gain_and_level_cap() {
    let mut game =
        Game::new_with_build(701, "demo.build.warrior").expect("Warrior build should create");
    clear_monsters(&mut game);
    game.apply_player_experience(100, &mut Vec::new());
    assert_eq!(game.progress.experience, 100);
    give_inventory_item(
        &mut game,
        "test.item.experience.1",
        "demo.item.experience-potion",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.experience.1".to_owned(),
            target: None,
        },
    );

    assert_eq!(game.progress.experience, 160);
    assert!(update.events.iter().any(|event| {
        event.kind == "player.experience-gained"
            && event.args.get("amount").map(String::as_str) == Some("60")
    }));

    let mut capped = Game::new(704);
    clear_monsters(&mut capped);
    capped.apply_player_experience(4_500_000, &mut Vec::new());
    assert_eq!(capped.progress.level, 50);
    choose_human_talent_if_pending(&mut capped);
    give_inventory_item(
        &mut capped,
        "test.item.experience.2",
        "demo.item.experience-potion",
    );
    dispatch_next(
        &mut capped,
        GameCommand::UseItem {
            item_id: "test.item.experience.2".to_owned(),
            target: None,
        },
    );
    assert_eq!(capped.progress.experience, 4_600_000);
    assert_eq!(capped.progress.level, 50);
}

#[test]
fn p3_7_neo_tsuyoshi_round_trips_and_crashes_on_expiry() {
    let mut game = Game::new(702);
    clear_monsters(&mut game);
    game.player.statuses.push(StatusInstance {
        kind_id: crate::effect::STATUS_HALLUCINATION.to_owned(),
        intensity: 1,
        remaining_ticks: 50,
        source_id: Some("test.hallucination".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(
        &mut game,
        "test.item.neo-tsuyoshi.1",
        "demo.item.neo-tsuyoshi-special",
    );

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.neo-tsuyoshi.1".to_owned(),
            target: None,
        },
    );

    assert!(!game.player_has_status_kind(crate::effect::STATUS_HALLUCINATION));
    let tsuyoshi = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_TSUYOSHI)
        .expect("Neo-Tsuyoshi should remain active");
    assert_eq!(tsuyoshi.granted_modifiers.strength, 4);
    assert_eq!(tsuyoshi.granted_modifiers.constitution, 4);
    assert_eq!(tsuyoshi.granted_modifiers.max_hp, 50);
    assert!((91..=190).contains(&tsuyoshi.remaining_ticks));
    let restored = Game::from_save(game.to_save()).expect("Tsuyoshi status should round trip");
    assert_eq!(restored.snapshot(), game.snapshot());

    game.progress.attributes.strength = 118;
    game.progress.maximum_attributes.strength = 118;
    game.progress.attributes.constitution = 118;
    game.progress.maximum_attributes.constitution = 118;
    game.player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == STATUS_TSUYOSHI)
        .expect("Tsuyoshi should remain active")
        .remaining_ticks = 1;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut Vec::new(), true)
        .expect("Tsuyoshi expiry should resolve");

    assert!(!game.player_has_status_kind(STATUS_TSUYOSHI));
    assert!(game.progress.maximum_attributes.strength < 118);
    assert!(game.progress.maximum_attributes.constitution < 118);
    assert_eq!(
        game.progress.attributes.strength,
        game.progress.maximum_attributes.strength
    );
    assert_eq!(
        game.progress.attributes.constitution,
        game.progress.maximum_attributes.constitution
    );
    assert_eq!(game.rng_draw_counter(), draws_before + 4);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerStatusExpired { status_kind_id }
            if status_kind_id == STATUS_TSUYOSHI
    )));
}

#[test]
fn p3_7_tsuyoshi_special_triggers_the_same_permanent_crash_immediately() {
    let mut game = Game::new(703);
    clear_monsters(&mut game);
    game.progress.attributes.strength = 18;
    game.progress.maximum_attributes.strength = 18;
    game.progress.attributes.constitution = 18;
    game.progress.maximum_attributes.constitution = 18;
    give_inventory_item(
        &mut game,
        "test.item.tsuyoshi.1",
        "demo.item.tsuyoshi-special",
    );

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.tsuyoshi.1".to_owned(),
            target: None,
        },
    );

    assert_eq!(game.progress.attributes.strength, 17);
    assert_eq!(game.progress.maximum_attributes.strength, 17);
    assert_eq!(game.progress.attributes.constitution, 17);
    assert_eq!(game.progress.maximum_attributes.constitution, 17);
    assert!(!game.player_has_status_kind(STATUS_TSUYOSHI));
    assert!(game.player_has_status_kind(crate::effect::STATUS_HALLUCINATION));
}

#[test]
fn p3_3_treasure_detection_reports_stable_gold_pile_ids() {
    let mut game = Game::new(205);
    clear_monsters(&mut game);
    let gold_position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    game.gold_piles = vec![
        GoldPile {
            id: "generated.gold.zeta".to_owned(),
            position: gold_position,
            amount: 20,
            appearance: GoldAppearanceDto::Gold,
            discovered: false,
        },
        GoldPile {
            id: "generated.gold.alpha".to_owned(),
            position: gold_position,
            amount: 10,
            appearance: GoldAppearanceDto::Copper,
            discovered: false,
        },
    ];
    give_inventory_item(
        &mut game,
        "test.item.treasure-detection.1",
        "demo.item.treasure-detection-scroll",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.treasure-detection.1".to_owned(),
            target: None,
        },
    );

    let detection = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("treasure detection should expose detected gold piles");
    assert_eq!(detection.subject, AbilityDetectSubjectDto::Gold);
    assert_eq!(detection.category, "gold");
    assert_eq!(
        detection.detected_positions,
        vec![gold_position, gold_position]
    );
    assert_eq!(
        detection.detected_entity_ids,
        vec!["generated.gold.alpha", "generated.gold.zeta"]
    );
    assert!(game.gold_piles.iter().all(|pile| pile.discovered));
    assert_eq!(update.gold_piles.len(), 2);
    assert!(update.changed_cells.iter().any(|cell| {
        cell.position == gold_position && cell.item_id.as_deref() == Some("generated.gold.alpha")
    }));
}

#[test]
fn tomte_tailored_acquirement_filters_headgear_by_birth_race_only() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut content = rfb_content::compile_pack_dir(&path).unwrap().content;
    let table = content
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap();
    let kinds = [
        "demo.item.knit-cap",
        "demo.item.iron-helm",
        "demo.item.iron-crown",
        "demo.item.dagger",
    ];
    table
        .entries
        .retain(|entry| kinds.contains(&entry.item_kind_id.as_str()));
    assert_eq!(table.entries.len(), kinds.len());
    for entry in &mut table.entries {
        entry.weight = 1;
        entry.min_depth = 0;
    }
    let content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(content).unwrap(),
    ));
    let mut template = Game::new_with_build(83, "demo.build.warrior").unwrap();
    clear_monsters(&mut template);
    template.content = content;
    for birth_tomte in [false, true] {
        let mut base = template.clone();
        base.build.as_mut().unwrap().race_id = if birth_tomte {
            "rfb-legacy.race.tomte"
        } else {
            "demo.race.rfb-human"
        }
        .to_owned();
        let mut form =
            monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10_000, "test.tailored").status;
        form.granted_race_id = Some(
            if birth_tomte {
                "demo.race.rfb-human"
            } else {
                "rfb-legacy.race.tomte"
            }
            .to_owned(),
        );
        base.player.statuses.push(form);
        for scroll in [
            "demo.item.acquirement-scroll",
            "demo.item.star-acquirement-scroll",
        ] {
            let mut seen = BTreeSet::new();
            for seed in 0..256 {
                let mut game = base.clone();
                game.items.clear();
                give_inventory_item(&mut game, "test.acquirement", scroll);
                game.rng = RfbRng::seeded(seed);
                dispatch_next(
                    &mut game,
                    GameCommand::UseItem {
                        item_id: "test.acquirement".to_owned(),
                        target: None,
                    },
                );
                // Empty categories retry; every accepted reward uses drop_near.
                for item in &game.items {
                    assert!(
                        matches!(item.location, ItemLocation::Ground(at) if (at.x-game.player.position.x).pow(2) + (at.y-game.player.position.y).pow(2) <= 10 && game.can_drop_item_at(at))
                    );
                    assert_eq!(item.quality, ItemQualityDto::Exceptional);
                    seen.insert(item.kind_id.clone());
                }
            }
            let expected = kinds
                .iter()
                .filter(|kind| {
                    !birth_tomte || matches!(**kind, "demo.item.knit-cap" | "demo.item.dagger")
                })
                .map(|kind| (*kind).to_owned())
                .collect::<BTreeSet<_>>();
            assert_eq!(seen, expected, "{scroll}: birth Tomte {birth_tomte}");
        }
        let context = LootContext {
            table_id: "demo.loot-table.base-items".to_owned(),
            floor_id: base.current_floor_id.clone(),
            depth: 20,
            source: LootSource::MonsterDeath {
                actor_id: "test.drop".to_owned(),
            },
        };
        let mut ordinary = BTreeSet::new();
        for seed in 0..256 {
            base.rng = RfbRng::seeded(seed);
            if let Some(draft) = base.generate_one_loot_draft(&context, ItemGenerationMode::Great) {
                let item = base.content.item(&draft.kind_id).unwrap();
                ordinary.insert(item.artifact_generation.as_ref().map_or_else(
                    || item.id.clone(),
                    |artifact| artifact.base_item_kind_id.clone(),
                ));
            }
        }
        assert_eq!(
            ordinary,
            kinds.iter().map(|kind| (*kind).to_owned()).collect()
        );
    }
}

#[test]
fn b4_tailored_glove_egos_share_casting_encumbrance_and_rejection_keeps_rng() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut source = rfb_content::compile_pack_dir(&path).unwrap().content;
    let template = source
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .unwrap()
        .clone();
    for ego in ["protection", "wizard-gloves", "free-action"] {
        let mut table = template.clone();
        table.id = format!("test.loot-table.tailored-{ego}");
        table.kind_selection = None;
        table.rfb_ego_policy = None;
        table.quality_policy = None;
        table.quality_weights = vec![rfb_content::LootQualityWeightDefinition {
            quality: rfb_content::ItemQuality::Exceptional,
            weight: 1,
        }];
        table
            .entries
            .retain(|entry| entry.item_kind_id == "demo.item.leather-gloves");
        table.affix_weights = vec![rfb_content::LootAffixWeightDefinition {
            affix_id: Some(format!("rfb-legacy.affix.{ego}")),
            weight: 1,
        }];
        source.loot_tables.push(table);
    }
    let content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(source).unwrap(),
    ));
    let warrior_mage_builds = super::support::warrior_mage_build_ids();
    for (build, ego) in [
        ("high-mage-death", "protection"),
        ("high-mage-death", "wizard-gloves"),
        ("high-mage-death", "free-action"),
        ("mage-death-sorcery", "protection"),
        ("mage-death-sorcery", "wizard-gloves"),
        ("mage-death-sorcery", "free-action"),
        ("berserker", "protection"),
        ("mindcrafter", "protection"),
    ]
    .into_iter()
    .chain(warrior_mage_builds.iter().flat_map(|id| {
        ["protection", "wizard-gloves", "free-action"]
            .map(|ego| (id.strip_prefix("demo.build.").unwrap(), ego))
    })) {
        let mut game = Game::new_with_build(425, &format!("demo.build.{build}")).unwrap();
        game.content = content.clone();
        clear_monsters(&mut game);
        choose_human_talent_if_pending(&mut game);
        game.progress.level = 30;
        game.progress.max_level = 30;
        game.refresh_character_skills();
        choose_human_talent_if_pending(&mut game);
        game.refresh_player_resource_maxima();
        let baseline_mana = game.resources.get("demo.resource.mana").map(|r| r.maximum);
        assert_eq!(baseline_mana.is_some(), build != "berserker");
        let baseline_armor = game.player_derived_stats().armor_class.value;
        let encumbers = (matches!(build, "high-mage-death" | "mage-death-sorcery")
            || build.starts_with("warrior-mage-"))
            && ego == "protection";
        let context = LootContext {
            table_id: format!("test.loot-table.tailored-{ego}"),
            floor_id: game.current_floor_id.clone(),
            depth: 30,
            source: LootSource::ItemUse {
                item_id: "test.reward".into(),
            },
        };
        let mut control = game.clone();
        let great = control
            .generate_one_loot_draft(&context, ItemGenerationMode::Great)
            .unwrap();
        assert_eq!(great.affix_ids, [format!("rfb-legacy.affix.{ego}")]);
        let items_before = game.items.clone();
        let knowledge_before = game.item_knowledge.clone();
        let tailored =
            game.generate_loot_draft_attempt(&context, ItemGenerationMode::TailoredGreat);
        assert_eq!(tailored.is_some(), !encumbers);
        if let Some(tailored) = tailored {
            assert_eq!(tailored, great);
        }
        assert_eq!(game.items, items_before);
        assert_eq!(game.item_knowledge, knowledge_before);
        assert_eq!(
            game.rng, control.rng,
            "rejected completed glove retains all materialization draws"
        );
        assert_eq!(game.rng.bounded(1_000), control.rng.bounded(1_000));

        // The same completed object drives actual equipment and mana. Inspecting
        // it during generation must not reveal hidden properties to the player.
        let item = game
            .commit_generated_item_draft(great, ItemLocation::Ground(game.player.position))
            .unwrap();
        let id = item.id.clone();
        assert!(!game.item_is_icky(&item, false));
        assert_eq!(game.item_is_icky(&item, true), encumbers);
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        game.equip_inventory_item(&id, Some("hands")).unwrap();
        game.refresh_player_resource_maxima();
        let mana = game.resources.get("demo.resource.mana").map(|r| r.maximum);
        if encumbers {
            assert_eq!(mana, baseline_mana.map(|m| m * 3 / 4));
        } else {
            assert!(mana >= baseline_mana);
        }
        if ego == "protection" {
            assert!(game.player_derived_stats().armor_class.value > baseline_armor);
        }
        game.identify_item_instance(&id, ItemIdentificationRequest::new(false));
        assert_eq!(
            game.item_is_icky(game.items.iter().find(|item| item.id == id).unwrap(), false),
            encumbers
        );
        game.reveal_current_visibility();
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(
            restored
                .resources
                .get("demo.resource.mana")
                .map(|r| r.maximum),
            mana
        );
        if !(matches!(build, "high-mage-death" | "mage-death-sorcery")
            || build.starts_with("warrior-mage-"))
        {
            let actual = game.generate_one_loot_draft(&context, ItemGenerationMode::TailoredGreat);
            let replay =
                restored.generate_one_loot_draft(&context, ItemGenerationMode::TailoredGreat);
            assert!(actual.is_some());
            assert_eq!(actual, replay);
            assert_eq!(game.rng, restored.rng);
        }
    }
}

#[test]
fn b4_glove_exemptions_use_flags_and_final_shared_pval_not_net_stat_bonus() {
    use rfb_content::{RfbPvalDefinition, RfbPvalFlagDefinition};
    let mut game = Game::new_with_build(426, "demo.build.high-mage-death").unwrap();
    give_inventory_item(&mut game, "test.gloves", "demo.item.leather-gloves");
    let mut item = game.items.last().unwrap().clone();
    for (flag, pval, encumbers) in [
        (RfbPvalFlagDefinition::Dexterity, -1, true),
        (RfbPvalFlagDefinition::Dexterity, 0, true),
        (RfbPvalFlagDefinition::Dexterity, 1, false),
        (RfbPvalFlagDefinition::Mastery, 0, false),
        (RfbPvalFlagDefinition::Mastery, -1, false),
    ] {
        item.intrinsic_properties.rfb_pval = Some(RfbPvalDefinition {
            value: pval,
            flags: [flag].into(),
        });
        assert_eq!(game.item_has_glove_encumbrance(&item), encumbers);
    }
    item.intrinsic_properties.rfb_pval = None;
    item.intrinsic_properties
        .rfb_flags
        .insert("FREE_ACT".into());
    assert!(!game.item_has_glove_encumbrance(&item));
    let paladin = Game::new_with_build(426, "demo.build.paladin-death").unwrap();
    item.intrinsic_properties = Default::default();
    assert!(!paladin.item_has_glove_encumbrance(&item));
}

fn b4_pick_up_tailored_kind(game: &mut Game, kind: &str) -> String {
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 60,
        source: LootSource::ItemUse {
            item_id: "test.tailored-reward".into(),
        },
    };
    let found = game
        .item_knowledge
        .get(kind)
        .map_or(0, |state| state.found_count);
    let draft = (0..512)
        .find_map(|_| {
            game.generate_one_loot_draft(&context, ItemGenerationMode::TailoredGreat)
                .filter(|draft| draft.kind_id == kind)
        })
        .unwrap_or_else(|| panic!("tailored kind unreachable: {kind}"));
    assert_eq!(
        game.item_knowledge
            .get(kind)
            .map_or(0, |state| state.found_count),
        found
    );
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    id
}

#[test]
fn all_priest_builds_generate_tailored_hafted_weapons_equip_and_resume_generation() {
    let builds = super::support::priest_build_ids();
    assert_eq!(builds.len(), 24);
    for build in builds
        .into_iter()
        .chain(super::support::warrior_mage_build_ids())
    {
        let mut game = Game::new_with_build(427, &build).unwrap();
        clear_monsters(&mut game);
        choose_human_talent_if_pending(&mut game);
        game.items.clear();
        let kind = if game.player_is_warrior_mage() {
            "demo.item.dagger"
        } else {
            "demo.item.mace"
        };
        let id = b4_pick_up_tailored_kind(&mut game, kind);
        assert!(game.equip_inventory_item(&id, None).is_some());
        game.refresh_player_resource_maxima();
        game.refresh_player_ability_state();
        assert!(!game.item_is_icky(&game.items[0], false));
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            b4_pick_up_tailored_kind(&mut game, kind),
            b4_pick_up_tailored_kind(&mut restored, kind)
        );
        assert_eq!(game.to_save(), restored.to_save());
    }
}

#[test]
fn tailored_duelist_weapon_equips_and_preserves_continued_generation_after_save() {
    let mut game = Game::new_with_build(427, "demo.build.duelist").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    let id = b4_pick_up_tailored_kind(&mut game, "demo.item.dagger");
    assert!(game.equip_inventory_item(&id, None).is_some());
    assert!(game.duelist_equipment_error().is_none());
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        b4_pick_up_tailored_kind(&mut game, "demo.item.dagger"),
        b4_pick_up_tailored_kind(&mut restored, "demo.item.dagger")
    );
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn b4_tailored_launchers_equip_shoot_and_restore_for_archer_and_sniper() {
    for (build, kind, ammunition) in [
        ("archer", "demo.item.short-bow", "demo.item.arrow"),
        ("sniper", "demo.item.light-crossbow", "demo.item.bolt"),
    ] {
        let mut game = Game::new_with_build(427, &format!("demo.build.{build}")).unwrap();
        clear_monsters(&mut game);
        choose_human_talent_if_pending(&mut game);
        let id = b4_pick_up_tailored_kind(&mut game, kind);
        game.equip_inventory_item(&id, None).unwrap();
        give_inventory_item(&mut game, "test.tailored-ammo", ammunition);
        let profile = game.player_projectile_profile().unwrap();
        assert_eq!(profile.source_item_id, id);
        assert_eq!(profile.ammo_kind_id, ammunition);
        let update = dispatch_next(
            &mut game,
            GameCommand::Fire {
                direction: Direction::East,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "combat.projectile-landed"),
            "{build}: {:?}",
            update.events
        );
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(
            restored.player_projectile_profile().unwrap().source_item_id,
            id
        );
    }
}

#[test]
fn b4_tailored_lance_keeps_riding_bonus_in_actual_mounted_combat() {
    let mut game = Game::new_with_build(428, "demo.build.cavalry").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    let id = b4_pick_up_tailored_kind(&mut game, "demo.item.lance");
    game.equip_inventory_item(&id, None).unwrap();
    let on_foot = game.player_melee_profile(&game.player_derived_stats());
    let start = game.player.position;
    let mount_position = Position {
        x: start.x + 1,
        y: start.y,
    };
    replace_terrain(&mut game, mount_position, "demo.terrain.floor");
    game.push_generated_actor("test.mount".into(), "demo.actor.horse", mount_position);
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.resolve_riding(Direction::East, &mut Vec::new(), &mut BTreeSet::new());
    assert_eq!(game.riding_actor_id.as_deref(), Some("test.mount"));
    let mounted = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(mounted.to_hit, on_foot.to_hit + 15);
    assert_eq!(mounted.damage_dice, on_foot.damage_dice + 2);
    let enemy_position = Position {
        x: mount_position.x + 1,
        y: mount_position.y,
    };
    replace_terrain(&mut game, enemy_position, "demo.terrain.floor");
    game.push_generated_actor(
        "test.target".into(),
        "demo.actor.novice-warrior",
        enemy_position,
    );
    let mut events = Vec::new();
    game.resolve_player_melee(1, true, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert!(!events.is_empty());
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.riding_actor_id, game.riding_actor_id);
}

#[test]
fn b4_tailored_death_books_are_counted_studied_cast_and_restored() {
    for build in ["high-mage-death", "paladin-death"] {
        let mut game = Game::new_with_build(429, &format!("demo.build.{build}")).unwrap();
        clear_monsters(&mut game);
        choose_human_talent_if_pending(&mut game);
        game.progress.level = 50;
        game.progress.max_level = 50;
        game.refresh_character_skills();
        choose_human_talent_if_pending(&mut game);
        game.refresh_player_resource_maxima();
        for resource in game.resources.values_mut() {
            resource.current = resource.maximum;
        }
        let id = b4_pick_up_tailored_kind(&mut game, "demo.item.black-channels");
        assert_eq!(
            game.item_knowledge["demo.item.black-channels"].found_count,
            1
        );
        let ability = "demo.ability.death-berserk";
        if build == "high-mage-death" {
            game.study_player_ability(&id, ability).unwrap();
        } else {
            for _ in 0..8 {
                game.study_random_player_ability(&id).unwrap();
                if game.learned_abilities.contains(ability) {
                    break;
                }
            }
        }
        assert!(game.learned_abilities.contains(ability));
        game.debug_set_ability_casts_succeed(true);
        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: ability.into(),
                target: TargetSelection::SelfTarget,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "ability.cast-success")
        );
        assert!(
            game.player
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_BERSERK)
        );
        assert_eq!(
            game.item_knowledge["demo.item.black-channels"].found_count,
            1
        );
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert!(restored.learned_abilities.contains(ability));
    }
}

#[test]
fn b4_tailored_high_mage_device_is_usable_and_restores_its_charges() {
    let mut game = Game::new_with_build(430, "demo.build.high-mage-death").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    let id = b4_pick_up_tailored_kind(&mut game, "demo.item.magic-missile-wand");
    let before = game
        .items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .charges
        .unwrap();
    // Existing device consumer's automatic-success branch; generation itself
    // above uses the formal pool, class hooks and device materialization.
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 5)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: id.clone(),
            target: Some(TargetSelection::Direction {
                direction: Direction::East,
            }),
        },
    );
    let charges = game
        .items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .charges
        .unwrap();
    assert!(charges.current < before.current);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .charges,
        Some(charges)
    );
}

#[test]
fn p3_5_acquirement_uses_stable_ids_current_position_and_exact_rng_draws() {
    let mut single = Game::new(503);
    clear_monsters(&mut single);
    // Bound this fixture to item use, independently of birth RNG consumption.
    single.rng = RfbRng::seeded(503);
    give_inventory_item(
        &mut single,
        "test.item.acquirement.1",
        "demo.item.acquirement-scroll",
    );
    let position = single.player.position;
    let ids_before = single
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let draws_before = single.rng_draw_counter();
    let update = dispatch_next(
        &mut single,
        GameCommand::UseItem {
            item_id: "test.item.acquirement.1".to_owned(),
            target: None,
        },
    );
    let generated = single
        .items
        .iter()
        .filter(|item| !ids_before.contains(&item.id))
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 1);
    assert_eq!(generated[0].location, ItemLocation::Ground(position));
    assert_eq!(generated[0].quality, ItemQualityDto::Exceptional);
    assert!(generated[0].id.starts_with("generated.item."));
    // Fixed action seed includes generation plus drop_near's disabled-breakage
    // roll and tied-grid roll; it does not include character creation.
    assert_eq!(single.rng_draw_counter(), draws_before + 641);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-acquirement"
            && event.args.get("count").map(String::as_str) == Some("1")
    }));

    let mut multiple = Game::new(509);
    clear_monsters(&mut multiple);
    multiple.rng = RfbRng::seeded(509);
    give_inventory_item(
        &mut multiple,
        "test.item.star-acquirement.1",
        "demo.item.star-acquirement-scroll",
    );
    let before_count = multiple.items.len();
    let draws_before = multiple.rng_draw_counter();
    dispatch_next(
        &mut multiple,
        GameCommand::UseItem {
            item_id: "test.item.star-acquirement.1".to_owned(),
            target: None,
        },
    );
    let generated_count = multiple.items.len() - (before_count - 1);
    assert!((2..=3).contains(&generated_count));
    // Generation and placement interleave, changing the next item's RNG branch.
    assert_eq!(multiple.rng_draw_counter(), draws_before + 50);
}

#[test]
fn p3_5_mundanity_preserves_stack_identity_and_rejects_unmapped_demo_artifacts() {
    let mut game = Game::new(521);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.item.mundanity.1",
        "demo.item.mundanity-scroll",
    );
    give_inventory_item(&mut game, "test.item.mundane-target.1", "demo.item.arrow");
    let target = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.mundane-target.1")
        .expect("target stack should exist");
    target.quantity = 3;
    target.quality = ItemQualityDto::Exceptional;
    target.affix_ids = vec!["demo.affix.frost-hunter".to_owned()];
    target.enchantments.to_hit = 2;
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.mundanity.1".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.mundane-target.1".to_owned(),
            }),
        },
    );
    let mundane = game
        .items
        .iter()
        .find(|item| item.id == "test.item.mundane-target.1")
        .unwrap();
    assert_eq!(mundane.quantity, 3);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.arrow")
            .count(),
        1
    );
    assert_eq!(mundane.quality, ItemQualityDto::Ordinary);
    assert!(mundane.affix_ids.is_empty());
    assert!(mundane.enchantments.is_empty());
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-mundanity"
            && event.args.get("split").map(String::as_str) == Some("false")
            && event.args.get("targetId") == Some(&mundane.id)
    }));
    Game::from_save(game.to_save()).expect("mundane ammunition stack should round-trip");

    give_inventory_item(
        &mut game,
        "test.item.mundanity.2",
        "demo.item.mundanity-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.fixed-artifact.1",
        "demo.item.relic-blade",
    );
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.mundanity.2".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.fixed-artifact.1".to_owned(),
            }),
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.item.mundanity.2")
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
}

#[test]
fn e6_crafting_keeps_ammunition_stack_identifies_ego_and_cancels_invalid_targets() {
    let mut game = Game::new(523);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.item.crafting.1",
        "demo.item.crafting-scroll",
    );
    give_inventory_item(&mut game, "test.item.crafting-target.1", "demo.item.arrow");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.crafting-target.1")
        .expect("ammunition should exist")
        .quantity = 3;
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.crafting.1".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.crafting-target.1".to_owned(),
            }),
        },
    );
    assert!(game.rng_draw_counter() > draws_before + 1);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.crafting-target.1")
            .expect("crafted stack should exist")
            .quantity,
        3
    );
    let crafted = game
        .items
        .iter()
        .find(|item| item.id == "test.item.crafting-target.1")
        .expect("the entire stack should be crafted in place");
    assert_eq!(crafted.quantity, 3);
    assert_eq!(crafted.quality, ItemQualityDto::Exceptional);
    assert_eq!(crafted.origin_kind, Some(ItemOriginKindDto::PlayerMade));
    assert_eq!(crafted.discount_percent, 99);
    assert_eq!(crafted.affix_ids.len(), 1);
    let knowledge = &game.item_property_knowledge[&crafted.id];
    assert!(knowledge.appraised && knowledge.identified);
    assert!(knowledge.known_affix_ids.contains(&crafted.affix_ids[0]));
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-crafting"
            && event.args.get("targetId") == Some(&crafted.id)
            && event.args.get("split").map(String::as_str) == Some("false")
    }));
    Game::from_save(game.to_save()).expect("crafted ammunition should round-trip");

    give_inventory_item(
        &mut game,
        "test.item.crafting.2",
        "demo.item.crafting-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.invalid-crafting-target.1",
        "demo.item.ration-of-food",
    );
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.crafting.2".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.invalid-crafting-target.1".to_owned(),
            }),
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.item.crafting.2")
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
}

fn artifact_creation_command(quantity: u32, name: Option<&str>) -> GameCommand {
    GameCommand::UseItem {
        item_id: "test.artifact-scroll".into(),
        target: Some(TargetSelection::ArtifactCreationItem {
            item_id: "test.artifact-target".into(),
            quantity,
            name: name.map(str::to_owned),
        }),
    }
}

#[test]
fn artifact_scroll_keeps_selected_equipment_identity_properties_and_saved_name() {
    let priest_builds = super::support::priest_build_ids();
    let warrior_mage_builds = super::support::warrior_mage_build_ids();
    for (build, equipped) in [
        "warrior",
        "archer",
        "sniper",
        "cavalry",
        "high-mage-death",
        "paladin-death",
        "mindcrafter",
        "duelist",
        "mage-death-sorcery",
        "ranger-nature-sorcery",
        "ranger-nature-death",
        "ranger-nature-arcane",
        "ranger-nature-daemon",
    ]
    .into_iter()
    .chain(
        priest_builds
            .iter()
            .chain(&warrior_mage_builds)
            .map(|id| id.strip_prefix("demo.build.").unwrap()),
    )
    .flat_map(|build| [false, true].map(|equipped| (build, equipped)))
    {
        let mut game = Game::new_with_build(637, &format!("demo.build.{build}")).unwrap();
        choose_human_talent_if_pending(&mut game);
        clear_monsters(&mut game);
        game.items.clear();
        give_inventory_item(
            &mut game,
            "test.artifact-scroll",
            "demo.item.artifact-creation-scroll",
        );
        give_inventory_item(&mut game, "test.artifact-target", "demo.item.dagger");
        if equipped {
            game.equip_inventory_item("test.artifact-target", None)
                .unwrap();
        }
        let location = game.items[1].location.clone();
        game.items[1].enchantments.to_damage = 5;
        game.items[1]
            .intrinsic_properties
            .rfb_flags
            .insert("RES_FIRE".into());
        let serial = game.next_item_instance_serial;
        let tick = game.world_tick;
        Game::from_save(game.to_save())
            .unwrap_or_else(|error| panic!("{build}, equipped={equipped}: before scroll: {error}"));
        assert!(
            game.snapshot()
                .inventory
                .iter()
                .find(|item| item.id == "test.artifact-scroll")
                .unwrap()
                .artifact_creation_targets
                .as_ref()
                .unwrap()
                .contains(&"test.artifact-target".to_owned())
        );
        let update = dispatch_next(&mut game, artifact_creation_command(1, Some("圆月")));
        if build == "mage-death-sorcery" {
            assert_eq!(game.virtue_current(VirtueKindDto::Enchantment), 6);
        }
        let target = game
            .items
            .iter()
            .find(|item| item.id == "test.artifact-target")
            .unwrap();
        assert_eq!(target.location, location);
        assert_eq!(target.kind_id, "demo.item.dagger");
        assert_eq!(target.artifact_name.as_deref(), Some("'圆月'"));
        assert_eq!(
            target.origin_kind,
            Some(ItemOriginKindDto::ArtifactCreation)
        );
        assert!(target.intrinsic_properties.rfb_flags.contains("RES_FIRE"));
        assert!(target.enchantments.to_damage >= 5);
        assert_eq!(
            game.item_identification(target),
            ItemIdentificationDto::Identified
        );
        assert_eq!(game.next_item_instance_serial, serial);
        assert!(game.world_tick > tick);
        assert!(
            !game
                .items
                .iter()
                .any(|item| item.id == "test.artifact-scroll")
        );
        assert!(update.events.iter().any(
            |event| event.kind == "item.artifact-creation" && event.args["succeeded"] == "true"
        ));
        let saved_target = target.clone();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == saved_target.id),
            Some(&saved_target)
        );
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.random_artifact_names, game.random_artifact_names);
        let a = dispatch_next(&mut game, GameCommand::Wait);
        let b = dispatch_next(&mut restored, GameCommand::Wait);
        assert_eq!(a.events, b.events);
        assert_eq!(game.state_hash(), restored.state_hash());
    }
}

#[test]
fn artifact_scroll_destroys_extra_ground_ammunition_and_uses_generated_dice_for_shooting() {
    let mut game = Game::new(4);
    choose_human_talent_if_pending(&mut game);
    let definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.castle-depth-55")
        .unwrap()
        .clone();
    let floor = game
        .generate_procedural_floor(&definition, Some("demo.dungeon.castle.instance.1".into()))
        .unwrap();
    game.dungeon_states
        .get_mut("demo.dungeon.castle")
        .unwrap()
        .next_instance_ordinal = 1;
    game.activate_floor(floor, Vec::new());
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.artifact-scroll",
        "demo.item.artifact-creation-scroll",
    );
    give_inventory_item(&mut game, "test.artifact-target", "demo.item.arrow");
    game.items[1].quantity = 4;
    game.items[1].location = ItemLocation::Ground(game.player.position);
    game.rng = RfbRng::seeded(4);
    let serial = game.next_item_instance_serial;
    let update = dispatch_next(&mut game, artifact_creation_command(4, None));
    let target = game
        .items
        .iter()
        .find(|item| item.id == "test.artifact-target")
        .unwrap();
    let dice = target.intrinsic_melee_damage_dice.unwrap();
    assert!(dice.dice > 1 || dice.sides > 4);
    assert_eq!(target.quantity, 1);
    assert!(target.activation.is_none());
    assert_eq!(target.location, ItemLocation::Ground(game.player.position));
    assert_eq!(game.next_item_instance_serial, serial);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.artifact-creation"
                && event.args["destroyedQuantity"] == "3")
    );
    game.pick_up_item_at_player(Some("test.artifact-target"))
        .unwrap();
    give_inventory_item(&mut game, "test.artifact-bow", "demo.item.short-bow");
    game.equip_inventory_item("test.artifact-bow", None)
        .unwrap();
    let profile = game.player_projectile_profile().unwrap();
    assert_eq!(
        (profile.damage_dice, profile.damage_sides),
        (dice.dice, dice.sides)
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        restored.player_projectile_profile().unwrap().damage_sides,
        dice.sides
    );
    assert_eq!(game.state_hash(), restored.state_hash());
    let shot = dispatch_next(
        &mut game,
        GameCommand::Fire {
            direction: Direction::East,
        },
    );
    let replay = dispatch_next(
        &mut restored,
        GameCommand::Fire {
            direction: Direction::East,
        },
    );
    assert_eq!(shot.events, replay.events);
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn artifact_scroll_zero_value_failure_keeps_generated_item_rng_and_scroll_without_time() {
    let mut game = Game::new(640);
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.artifact-scroll",
        "demo.item.artifact-creation-scroll",
    );
    give_inventory_item(&mut game, "test.artifact-target", "demo.item.dagger");
    game.items[1].enchantments = ItemEnchantmentsDto {
        to_hit: -255,
        to_damage: -255,
        to_armor: -255,
    };
    for progress in game.ability_progress.values_mut() {
        progress.cooldown_remaining = 3;
    }
    let ability_progress = game.ability_progress.clone();
    let draws = game.rng_draw_counter();
    let tick = game.world_tick;
    let update = dispatch_next(&mut game, artifact_creation_command(1, Some("零")));
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.artifact-creation"
                && event.args["succeeded"] == "false")
    );
    assert_eq!(game.world_tick, tick);
    assert_eq!(game.ability_progress, ability_progress);
    assert!(game.rng_draw_counter() > draws);
    assert_eq!(game.items[0].id, "test.artifact-scroll");
    assert_eq!(game.items[1].artifact_name.as_deref(), Some("'零'"));
    assert_eq!(game.items[1].origin_kind, None);
    assert_eq!(
        game.item_identification(&game.items[1]),
        ItemIdentificationDto::Identified
    );
    assert_eq!(
        game.state_hash(),
        Game::from_save(game.to_save()).unwrap().state_hash()
    );
}

#[test]
fn artifact_scroll_rejects_stale_known_or_illegal_targets_without_mutation() {
    let mut base = Game::new(638);
    choose_human_talent_if_pending(&mut base);
    clear_monsters(&mut base);
    base.items.clear();
    give_inventory_item(
        &mut base,
        "test.artifact-scroll",
        "demo.item.artifact-creation-scroll",
    );
    give_inventory_item(&mut base, "test.artifact-target", "demo.item.dagger");
    for case in 0..9 {
        let mut game = base.clone();
        let mut command = artifact_creation_command(1, None);
        match case {
            0 => {
                command = GameCommand::UseItem {
                    item_id: "test.artifact-scroll".into(),
                    target: None,
                }
            }
            1 => command = artifact_creation_command(2, None),
            2 => game.items[1].artifact_name = Some("'Known artifact'".into()),
            3 => game.items[1].affix_ids = vec!["demo.affix.frost-hunter".into()],
            4 => game.items[1].kind_id = "demo.item.poison-needle".into(),
            5 => {
                game.items[1].location = ItemLocation::Ground(Position {
                    x: game.player.position.x + 1,
                    y: game.player.position.y,
                })
            }
            6 => command = artifact_creation_command(1, Some(&"长".repeat(27))),
            7 => command = artifact_creation_command(1, Some("bad\nname")),
            _ => game.items[1].kind_id = "demo.item.cure-poison-mushroom".into(),
        }
        game.identify_item_instance("test.artifact-target", ItemIdentificationRequest::new(true));
        let items = game.items.clone();
        let rng = game.rng.clone();
        let tick = game.world_tick;
        let update = dispatch_next(&mut game, command);
        assert_eq!(game.items, items, "case {case}");
        assert_eq!(game.rng, rng);
        assert_eq!(game.world_tick, tick);
        assert_eq!(update.events[0].kind, "item.use-unavailable");
    }
    for no_remove in [false, true] {
        let mut game = base.clone();
        if no_remove {
            game.items[1]
                .intrinsic_properties
                .rfb_flags
                .insert("NO_REMOVE".into());
        } else {
            game.items[1].affix_ids = vec!["demo.affix.frost-hunter".into()];
        }
        let items = game.items.clone();
        let rng = game.rng.clone();
        let tick = game.world_tick;
        let update = dispatch_next(&mut game, artifact_creation_command(1, None));
        assert_eq!(game.items, items);
        assert_eq!(game.rng, rng);
        assert_eq!(game.world_tick, tick);
        assert_eq!(
            game.item_knowledge_dto("demo.item.artifact-creation-scroll"),
            ItemKnowledgeDto::Aware
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "item.artifact-creation"
                    && event.args["succeeded"] == "false")
        );
    }
}

#[test]
fn artifact_scroll_snotling_mushroom_is_reusable_and_restores_its_cooldown() {
    let mut game = snotling_game(639);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.artifact-scroll",
        "demo.item.artifact-creation-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.artifact-target",
        "demo.item.cure-poison-mushroom",
    );
    game.items[1].quantity = 2;
    dispatch_next(&mut game, artifact_creation_command(2, None));
    assert_eq!(game.items[0].artifact_name.as_deref(), Some("(永恒蘑菇)"));
    assert_eq!(game.items[0].quantity, 1);
    let eat = GameCommand::UseItem {
        item_id: "test.artifact-target".into(),
        target: None,
    };
    dispatch_next(&mut game, eat.clone());
    assert_eq!(game.items[0].quantity, 1);
    assert!(
        (1..crate::state::ARTIFACT_MUSHROOM_COOLDOWN_TICKS)
            .contains(&game.items[0].device_recovery_progress)
    );
    assert!(!game.snapshot().inventory[0].usable);
    assert!(game.player_has_status_kind(STATUS_HASTE));
    let tick = game.world_tick;
    let update = dispatch_next(&mut game, eat.clone());
    assert_eq!(update.events[0].kind, "item.use-unavailable");
    assert_eq!(game.world_tick, tick);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    let remaining = game.items[0].device_recovery_progress;
    for _ in 0..remaining {
        game.process_inventory_device_recovery(&mut Vec::new());
        restored.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(game.items[0].device_recovery_progress, 0);
    assert!(game.snapshot().inventory[0].usable);
    let a = dispatch_next(&mut game, eat.clone());
    let b = dispatch_next(&mut restored, eat);
    assert_eq!(a.events, b.events);
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.items[0].quantity, 1);
}

#[test]
fn e6_crafting_stack_needs_no_new_instance_allocation() {
    let mut game = Game::new(524);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.item.crafting.failure",
        "demo.item.crafting-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.crafting-target.failure",
        "demo.item.arrow",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.crafting-target.failure")
        .expect("ammunition should exist")
        .quantity = 3;
    game.next_item_instance_serial = u64::MAX;

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.crafting.failure".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.crafting-target.failure".to_owned(),
            }),
        },
    );

    assert_eq!(game.next_item_instance_serial, u64::MAX);
    let target = game
        .items
        .iter()
        .find(|item| item.id == "test.item.crafting-target.failure")
        .unwrap();
    assert_eq!(target.quantity, 3);
    assert_eq!(target.affix_ids.len(), 1);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-crafting")
    );
}

#[test]
fn e6_crafting_materializes_player_made_armor() {
    let mut game = Game::new(525);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.item.crafting.armor",
        "demo.item.crafting-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.crafting-target.armor",
        "demo.item.chain-mail",
    );

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.crafting.armor".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.crafting-target.armor".to_owned(),
            }),
        },
    );

    let crafted = game
        .items
        .iter()
        .find(|item| item.id == "test.item.crafting-target.armor")
        .expect("crafted armour should remain in inventory");
    assert_eq!(crafted.affix_ids.len(), 1);
    assert!(
        game.content
            .affix(&crafted.affix_ids[0])
            .unwrap()
            .rfb_ego
            .is_some()
    );
    assert_eq!(crafted.origin_kind, Some(ItemOriginKindDto::PlayerMade));
    assert_eq!(crafted.discount_percent, 99);
    Game::from_save(game.to_save()).expect("player-made armour should round-trip");
}

fn crafting_game(kind_id: &str, quantity: u32) -> Game {
    let mut game = Game::new(523);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.crafting-source",
        "demo.item.crafting-scroll",
    );
    give_inventory_item(&mut game, "test.crafting-target", kind_id);
    game.items[1].quantity = quantity;
    let virtue_index = game
        .virtues
        .iter()
        .position(|virtue| virtue.kind == VirtueKindDto::Enchantment)
        .unwrap_or(0);
    game.virtues[virtue_index] = rfb_protocol::VirtueDto {
        kind: VirtueKindDto::Enchantment,
        value: 0,
    };
    game
}

#[test]
fn e6_crafting_invalid_and_unconfirmed_targets_cost_no_use_time_or_rng() {
    let base = crafting_game("demo.item.arrow", 31);
    for case in 0..10 {
        let mut game = base.clone();
        let mut target = Some(TargetSelection::Item {
            item_id: "test.crafting-target".to_owned(),
        });
        match case {
            0 => target = None,
            1 => {} // 31 projectiles require explicit confirmation.
            2 => {
                target = Some(TargetSelection::CraftingItem {
                    item_id: "test.crafting-target".to_owned(),
                    quantity: 30,
                })
            }
            3 => game.items[1].quantity = 60,
            4 => {
                game.items[1].kind_id = "demo.item.chain-mail".to_owned();
                game.items[1].quantity = 2;
            }
            5 => game.items[1].affix_ids = vec!["rfb-legacy.affix.slaying-180".to_owned()],
            6 => {
                game.items[1].kind_id = "demo.item.ration-of-food".to_owned();
                game.items[1].quantity = 1;
            }
            7 => {
                game.items[1].kind_id = "demo.item.relic-blade".to_owned();
                game.items[1].quantity = 1;
            }
            8 => {
                game.items[1].quantity = 1;
                game.items[1].location = ItemLocation::Ground(Position {
                    x: game.player.position.x + 1,
                    y: game.player.position.y,
                });
            }
            9 => game.items[1].quantity = 0,
            _ => unreachable!(),
        }
        let items = game.items.clone();
        let rng = game.rng.clone();
        let tick = game.world_tick;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.crafting-source".to_owned(),
                target,
            },
        );
        assert_eq!(game.items, items, "case {case}");
        assert_eq!(game.rng, rng, "case {case}");
        assert_eq!(game.world_tick, tick, "case {case}");
        assert_eq!(update.events[0].kind, "item.use-unavailable", "case {case}");
    }
}

#[test]
fn e6_crafting_ammunition_roll_and_failure_virtue_match_source() {
    let base = crafting_game("demo.item.arrow", 1);
    let mut saw_failure = false;
    let mut saw_success = false;
    for quantity in [1, 30, 31, 59] {
        for seed in 0..32 {
            let mut game = base.clone();
            game.items[1].quantity = quantity;
            let original = game.items[1].clone();
            game.rng = RfbRng::seeded(seed);
            let mut expected_rng = game.rng.clone();
            let succeeds = expected_rng.bounded(30) as i32 + 1 > quantity as i32 - 30;
            let failure_virtue = if succeeds {
                0
            } else {
                -i16::from(expected_rng.bounded(3) == 0)
            };
            let update = dispatch_next(
                &mut game,
                GameCommand::UseItem {
                    item_id: "test.crafting-source".to_owned(),
                    target: Some(TargetSelection::CraftingItem {
                        item_id: original.id.clone(),
                        quantity,
                    }),
                },
            );
            assert!(
                !game
                    .items
                    .iter()
                    .any(|item| item.id == "test.crafting-source")
            );
            let crafted = game
                .items
                .iter()
                .find(|item| item.id == original.id)
                .unwrap();
            assert_eq!(crafted.quantity, quantity);
            if succeeds {
                saw_success = true;
                assert_eq!(crafted.affix_ids.len(), 1);
                assert_eq!(game.virtue_current(VirtueKindDto::Enchantment), 1);
                assert!(
                    update
                        .events
                        .iter()
                        .any(|event| event.kind == "item.use-crafting")
                );
            } else {
                saw_failure = true;
                assert_eq!(crafted, &original);
                assert_eq!(
                    game.virtue_current(VirtueKindDto::Enchantment),
                    failure_virtue
                );
                assert!(
                    update
                        .events
                        .iter()
                        .any(|event| event.kind == "item.use-crafting-failed")
                );
                assert!(
                    game.item_property_knowledge
                        .get(&original.id)
                        .is_none_or(|knowledge| !knowledge.identified
                            && knowledge.known_affix_ids.is_empty())
                );
            }
        }
    }
    assert!(saw_failure && saw_success);
}

#[test]
fn e6_crafting_uses_shared_weighted_materialization_at_player_level() {
    use crate::game::ego::roll_and_materialize_rfb_ego_from_affixes_with_rng;
    for kind_id in [
        "demo.item.long-sword",
        "demo.item.mattock",
        "demo.item.long-bow",
        "demo.item.harp",
        "demo.item.chain-mail",
        "demo.item.elven-cloak",
        "demo.item.multi-hued-dragon-scale-mail",
    ] {
        let mut game = crafting_game(kind_id, 1);
        game.apply_player_experience(100_000, &mut Vec::new());
        while game.pending_race_mutation_choice().is_some() {
            choose_human_talent_if_pending(&mut game);
        }
        assert!(game.progress.level > game.floor_depth(&game.current_floor_id));
        game.items[1].quality = ItemQualityDto::Fine;
        game.items[1].enchantments = ItemEnchantmentsDto {
            to_hit: 3,
            to_damage: 4,
            to_armor: 5,
        };
        if kind_id == "demo.item.harp" {
            game.items[1].intrinsic_properties.modifiers.charisma = 2;
        }
        if kind_id == "demo.item.elven-cloak" {
            game.items[1]
                .intrinsic_properties
                .equipment_bonuses
                .stealth_skill = 2;
            game.items[1]
                .intrinsic_properties
                .equipment_bonuses
                .search_skill = 10;
            game.items[1]
                .intrinsic_properties
                .equipment_bonuses
                .perception_skill = 10;
        }
        game.rng = RfbRng::seeded(7);
        let mut expected = game.items[1].clone();
        let materialized = roll_and_materialize_rfb_ego_from_affixes_with_rng(
            false,
            rfb_protocol::ItemEnchantmentsDto::default(),
            &mut game.rng.clone(),
            game.content.item(kind_id).unwrap(),
            game.content.affix_definitions(),
            game.progress.level,
            Some(&expected.intrinsic_properties),
        )
        .unwrap();
        materialized.apply_to(&mut expected);
        expected.quality = ItemQualityDto::Exceptional;
        expected.origin_kind = Some(ItemOriginKindDto::PlayerMade);
        expected.discount_percent = 99;
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.crafting-source".to_owned(),
                target: Some(TargetSelection::Item {
                    item_id: expected.id.clone(),
                }),
            },
        );
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == expected.id)
                .unwrap(),
            &expected,
            "{kind_id}"
        );
        assert!(
            !game
                .content
                .item(&expected.kind_id)
                .unwrap()
                .tags
                .iter()
                .any(|tag| tag == "artifact")
        );
        assert_eq!(game.virtue_current(VirtueKindDto::Enchantment), 1);
        Game::from_save(game.to_save()).unwrap_or_else(|error| panic!("{kind_id}: {error:?}"));
    }
}

#[test]
fn e6_crafting_failed_materialization_preserves_all_target_state() {
    let mut game = crafting_game("demo.item.long-sword", 1);
    game.items[1].enchantments = ItemEnchantmentsDto {
        to_hit: 255,
        to_damage: 255,
        to_armor: 0,
    };
    let original = game.items[1].clone();
    game.rng = RfbRng::seeded(0);
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.crafting-source".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: original.id.clone(),
            }),
        },
    );
    assert_eq!(
        game.items.iter().find(|item| item.id == original.id),
        Some(&original)
    );
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "test.crafting-source")
    );
    assert!(
        game.item_property_knowledge
            .get(&original.id)
            .is_none_or(|knowledge| !knowledge.identified && knowledge.known_affix_ids.is_empty())
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-crafting-failed")
    );
}

#[test]
fn p3_5_rumour_is_localized_without_core_rng() {
    let mut game = Game::new(541);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.item.rumour.1", "demo.item.rumour-scroll");
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.rumour.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    let event = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-rumour")
        .expect("rumour should emit a localized message reference");
    assert_eq!(
        event.args.get("rumourKey").map(String::as_str),
        Some("rumour-demo-warrens-depths")
    );
}

#[test]
fn fixed_artifact_selection_uses_source_order_ood_rarity_and_uniqueness() {
    let context = artifact_loot_context(60);
    let mut instant = Game::new(1);
    let instant_rejection_seed = (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(10) != 0)
        .expect("an instant-artifact gate rejection seed should exist");
    instant.rng = RfbRng::seeded(instant_rejection_seed);
    assert_eq!(
        instant.roll_instant_fixed_artifact_kind_id(&context, 10),
        None
    );
    assert_eq!(instant.rng_draw_counter(), 1);

    let crisdurian_seed = (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(15) == 0)
        .expect("a Crisdurian rarity seed should exist");
    let mut game = Game::new(1);
    game.rng = RfbRng::seeded(crisdurian_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,)
            .as_deref(),
        Some("demo.item.crisdurian")
    );
    assert_eq!(game.rng_draw_counter(), 1);

    game.generated_artifact_ids
        .insert("demo.item.crisdurian".to_owned());
    let slayer_seed = (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(60) == 0)
        .expect("a Slayer rarity seed should exist");
    game.rng = RfbRng::seeded(slayer_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,)
            .as_deref(),
        Some("demo.item.slayer")
    );
    assert_eq!(game.rng_draw_counter(), 1);

    game.generated_artifact_ids
        .insert("demo.item.slayer".to_owned());
    game.rng = RfbRng::seeded(0);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,),
        None
    );
    assert_eq!(game.rng_draw_counter(), 0);

    let rarity_rejection_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(15) != 0 && rng.bounded(60) != 0
        })
        .expect("a double rarity rejection seed should exist");
    game.generated_artifact_ids.clear();
    game.rng = RfbRng::seeded(rarity_rejection_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,),
        None
    );
    assert_eq!(game.rng_draw_counter(), 2);

    let ood_rejection_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(4) != 0 && rng.bounded(4) != 0
        })
        .expect("a double OOD rejection seed should exist");
    game.rng = RfbRng::seeded(ood_rejection_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(
            &artifact_loot_context(58),
            Some("demo.item.executioners-sword"),
            false,
        ),
        None
    );
    assert_eq!(game.rng_draw_counter(), 2);
}

#[test]
fn item_generation_modes_keep_drafts_unallocated_until_commit() {
    let context = artifact_loot_context(60);

    let mut good = Game::new(2);
    good.rng = RfbRng::seeded(7);
    let good_draft = good
        .generate_one_loot_draft(&context, ItemGenerationMode::Good)
        .expect("Good generation should produce a draft");
    assert!(matches!(
        good_draft.quality,
        ItemQualityDto::Fine | ItemQualityDto::Exceptional
    ));

    let mut great = Game::new(2);
    great.rng = RfbRng::seeded(7);
    let great_draft = great
        .generate_one_loot_draft(&context, ItemGenerationMode::Great)
        .expect("Great generation should produce a draft");
    assert_eq!(great_draft.quality, ItemQualityDto::Exceptional);

    let mut artifact = Game::new(2);
    let serial_before = artifact.next_item_instance_serial;
    let fallback = (0..10_000).find_map(|seed| {
        artifact.rng = RfbRng::seeded(seed);
        let draft = artifact.generate_one_loot_draft(
            &context,
            ItemGenerationMode::Artifact {
                no_fixed_artifact: false,
            },
        )?;
        artifact
            .content
            .item(&draft.kind_id)
            .is_some_and(|item| item.artifact_generation.is_none())
            .then_some(draft)
    });
    let fallback = fallback.expect("an Artifact request fallback should exist");
    assert_eq!(fallback.quality, ItemQualityDto::Exceptional);
    assert_eq!(artifact.next_item_instance_serial, serial_before);
    let committed = artifact
        .commit_generated_item_draft(fallback, ItemLocation::Ground(artifact.player.position))
        .expect("an accepted draft should receive an instance ID");
    assert_eq!(artifact.next_item_instance_serial, serial_before + 1);
    assert!(
        artifact
            .content
            .item(&committed.kind_id)
            .is_some_and(|item| item.artifact_generation.is_none())
    );

    let mut fixed = Game::new(3);
    fixed.rng = RfbRng::seeded(crisdurian_seed_for_test());
    let kind_id = fixed
        .roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false)
        .expect("Crisdurian should pass its rarity gate");
    let draft = fixed.fixed_item_draft(&context, kind_id);
    assert_eq!(draft.quality, ItemQualityDto::Ordinary);
    let item = fixed
        .commit_generated_item_draft(draft, ItemLocation::Ground(fixed.player.position))
        .expect("fixed artifact draft should commit");
    assert_eq!(item.kind_id, "demo.item.crisdurian");
    assert!(
        fixed
            .generated_artifact_ids
            .contains("demo.item.crisdurian")
    );
}

#[test]
fn p103d_mana_storm_staff_hits_radius_five_without_backlash() {
    const ITEM_ID: &str = "test.item.mana-storm-staff.1";
    let mut game =
        Game::new_with_build(203, "demo.build.warrior").expect("Mana Storm test should create");
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.player.position = Position { x: 10, y: 10 };
    give_inventory_item(&mut game, ITEM_ID, "demo.item.mana-storm-staff");
    let item = game
        .items
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("Mana Storm staff should be granted");
    assert_eq!(item.charges.expect("Mana Storm staff charges").maximum, 5);

    for (id, x) in [("near", 11), ("edge", 15), ("outside", 16)] {
        game.push_generated_actor(
            format!("test.actor.mana-storm-{id}"),
            "demo.actor.ancient-multi-hued-dragon",
            Position { x, y: 10 },
        );
        let actor = game.entities.last_mut().expect("Mana Storm target");
        actor.hp = 1_000;
        actor.max_hp = 1_000;
    }
    let hp_before = game.player.hp;
    let mut domain_events = Vec::new();
    game.resolve_item_elemental_blast(
        "demo.item.mana-storm-staff",
        792,
        DamageType::Mana,
        5,
        0,
        0,
        DamageType::Mana,
        false,
        &mut domain_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Mana Storm should resolve");
    let events = domain_events
        .into_iter()
        .map(DomainEvent::into_dto)
        .collect::<Vec<_>>();

    let resolutions = events
        .iter()
        .filter_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::Damage { resolution })
            | Some(GameEventOutcomeDto::Death { resolution })
                if matches!(
                    event.kind.as_str(),
                    "item.use-elemental-blast-hit" | "item.use-elemental-blast-slay"
                ) =>
            {
                Some(resolution)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(resolutions.len(), 2);
    assert_eq!(
        resolutions
            .iter()
            .map(|resolution| resolution.raw_damage)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([132, 396])
    );
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.id == "test.actor.mana-storm-outside")
    );
    assert_eq!(game.player.hp, hp_before);
    assert!(!events.iter().any(|event| {
        matches!(
            event.kind.as_str(),
            "item.use-elemental-backlash" | "item.use-elemental-backlash-death"
        )
    }));
}

#[test]
fn p107e_frost_ball_and_confusing_light_reuse_area_and_status_resolvers() {
    const FROST_ITEM_ID: &str = "test.item.frost-ball-wand.1";
    let mut frost =
        Game::new_with_build(207, "demo.build.warrior").expect("Frost Ball test should create");
    super::dungeon_anti_magic::enter_context(&mut frost);
    clear_monsters(&mut frost);
    frost.terrain.fill("demo.terrain.floor".to_owned());
    frost.player.position = Position { x: 10, y: 10 };
    give_inventory_item(&mut frost, FROST_ITEM_ID, "demo.item.frost-ball-wand");
    for (id, position) in [
        ("center", Position { x: 18, y: 10 }),
        ("edge", Position { x: 18, y: 12 }),
        ("outside", Position { x: 18, y: 13 }),
    ] {
        frost.push_generated_actor(
            format!("test.actor.frost-ball-{id}"),
            "demo.actor.blubbering-idiot",
            position,
        );
        let actor = frost.entities.last_mut().expect("Frost Ball target");
        actor.hp = 1_000;
        actor.max_hp = 1_000;
    }
    let frost = (0..1_000)
        .find_map(|seed| {
            let mut game = frost.clone();
            game.rng = RfbRng::seeded(seed);
            dispatch_next(
                &mut game,
                GameCommand::UseItem {
                    item_id: FROST_ITEM_ID.to_owned(),
                    target: Some(TargetSelection::Direction {
                        direction: Direction::East,
                    }),
                },
            );
            game.entities
                .iter()
                .any(|actor| actor.id == "test.actor.frost-ball-center" && actor.hp < 1_000)
                .then_some(game)
        })
        .expect("Frost Ball should pass its device check");
    for id in ["center", "edge"] {
        assert!(
            frost
                .entities
                .iter()
                .find(|actor| actor.id == format!("test.actor.frost-ball-{id}"))
                .expect("affected Frost Ball target")
                .hp
                < 1_000
        );
    }
    assert_eq!(
        frost
            .entities
            .iter()
            .find(|actor| actor.id == "test.actor.frost-ball-outside")
            .expect("outside Frost Ball target")
            .hp,
        1_000
    );

    const LIGHT_ITEM_ID: &str = "test.item.confusing-light-staff.1";
    let mut light = Game::new_with_build(208, "demo.build.warrior")
        .expect("Confusing Light test should create");
    clear_monsters(&mut light);
    light.terrain.fill("demo.terrain.floor".to_owned());
    light.player.position = Position { x: 10, y: 10 };
    give_inventory_item(&mut light, LIGHT_ITEM_ID, "demo.item.confusing-light-staff");
    light.push_generated_actor(
        "test.actor.confusing-light".to_owned(),
        "demo.actor.blubbering-idiot",
        Position { x: 12, y: 10 },
    );
    light.reveal_current_visibility();
    let light = (0..1_000)
        .find_map(|seed| {
            let mut game = light.clone();
            game.rng = RfbRng::seeded(seed);
            game.use_inventory_item(
                LIGHT_ITEM_ID,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("Confusing Light should resolve");
            let status_ids = game.entities[0]
                .statuses
                .iter()
                .map(|status| status.kind_id.as_str())
                .collect::<BTreeSet<_>>();
            (status_ids.len() == 5).then_some(game)
        })
        .expect("Confusing Light should apply all five statuses");
    assert_eq!(
        light.entities[0]
            .statuses
            .iter()
            .map(|status| status.kind_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            STATUS_CONFUSION,
            STATUS_FEAR,
            STATUS_PARALYSIS,
            STATUS_SLOW,
            STATUS_STUN,
        ])
    );
}

#[test]
fn p107f_diamond_edge_vorpal_flag_multiplies_regular_melee_blows() {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let mut plain_content = artifact.content.clone();
    plain_content
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.diamond-edge")
        .expect("Diamond Edge should exist")
        .vorpal = false;
    let catalog = |content| {
        std::sync::Arc::new(rfb_content::ContentCatalog::from_artifact(
            rfb_content::encode_content(content).expect("custom content should encode"),
        ))
    };
    let mut vorpal = Game::from_content_with_build(
        0,
        catalog(artifact.content),
        DEFAULT_WORLD_ID,
        "demo.build.warrior",
    )
    .expect("Vorpal Diamond Edge game should create");
    let mut plain = Game::from_content_with_build(
        0,
        catalog(plain_content),
        DEFAULT_WORLD_ID,
        "demo.build.warrior",
    )
    .expect("plain Diamond Edge game should create");
    for game in [&mut vorpal, &mut plain] {
        clear_monsters(game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 10, y: 10 };
        game.items
            .iter_mut()
            .find(|item| {
                matches!(
                    &item.location,
                    ItemLocation::Equipped { slot_id } if slot_id == "right-hand"
                )
            })
            .expect("warrior should have an equipped weapon")
            .kind_id = "demo.item.diamond-edge".to_owned();
        game.push_generated_actor(
            "test.actor.diamond-edge".to_owned(),
            "demo.actor.blubbering-idiot",
            Position { x: 11, y: 10 },
        );
        let actor = game.entities.last_mut().expect("Diamond Edge target");
        actor.hp = 100_000;
        actor.max_hp = 100_000;
    }

    let first_melee_damage = |game: &mut Game, seed| {
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .expect("Diamond Edge melee should resolve");
        events
            .into_iter()
            .map(DomainEvent::into_dto)
            .find_map(|event| match event.outcome {
                Some(GameEventOutcomeDto::Damage { resolution }) => Some(resolution.raw_damage),
                _ => None,
            })
    };
    let observed = (0..10_000).find_map(|seed| {
        let mut vorpal_game = vorpal.clone();
        let mut plain_game = plain.clone();
        let vorpal_damage = first_melee_damage(&mut vorpal_game, seed)?;
        let plain_damage = first_melee_damage(&mut plain_game, seed)?;
        (vorpal_damage > plain_damage).then_some((plain_damage, vorpal_damage))
    });
    let (plain_damage, vorpal_damage) = observed.expect("a Vorpal trigger seed should exist");
    assert!(vorpal_damage >= plain_damage.saturating_mul(2));
}

fn crisdurian_seed_for_test() -> u64 {
    (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(15) == 0)
        .expect("a Crisdurian rarity seed should exist")
}

#[test]
fn status_item_refresh_preserves_source_and_does_not_make_the_kind_aware() {
    let mut base = Game::new(85);
    clear_monsters(&mut base);
    let duration_seed = (0..512)
        .find(|seed| RfbRng::seeded(*seed).bounded(25) == 24)
        .expect("a maximum slowness duration roll should exist");
    for (kind_id, status_id, ticks, source, seed, event, resistances) in [
        (
            "demo.item.slowness-potion",
            STATUS_SLOW,
            1,
            "test.existing-slow",
            duration_seed,
            "item.use-slowness-no-effect",
            BTreeMap::new(),
        ),
        (
            "demo.item.temperate-tonic",
            STATUS_THERMAL_RESISTANCE,
            3,
            "test.existing-thermal-resistance",
            85,
            "item.use-thermal-resistance-no-effect",
            BTreeMap::from([
                (DamageType::Fire, ResistanceLevel::Resistant),
                (DamageType::Cold, ResistanceLevel::Resistant),
            ]),
        ),
    ] {
        let mut game = base.clone();
        give_inventory_item(&mut game, "test.refresh-item", kind_id);
        game.player.statuses.push(StatusInstance {
            kind_id: status_id.to_owned(),
            intensity: 1,
            remaining_ticks: ticks,
            source_id: Some(source.to_owned()),
            granted_resistances: resistances.clone(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        // Preserve the tonic's post-initialization RNG; force the slowness maximum roll.
        if status_id == STATUS_SLOW {
            game.rng = RfbRng::seeded(seed);
        }
        let draws_before = game.rng_draw_counter();
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.refresh-item".to_owned(),
                target: None,
            },
        );
        assert_eq!(game.rng_draw_counter(), draws_before + 1, "{kind_id}");
        let status = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == status_id)
            .unwrap_or_else(|| panic!("{kind_id} should refresh {status_id}"));
        assert!(status.remaining_ticks > ticks, "{kind_id}");
        if status_id == STATUS_THERMAL_RESISTANCE {
            assert!((4..=13).contains(&status.remaining_ticks), "{kind_id}");
        }
        assert_eq!(status.source_id.as_deref(), Some(source), "{kind_id}");
        for (damage, resistance) in resistances {
            assert_eq!(
                game.effective_player_resistances().level(damage),
                resistance,
                "{kind_id}"
            );
        }
        assert!(
            !game.items.iter().any(|item| item.id == "test.refresh-item"),
            "{kind_id}"
        );
        assert_eq!(
            game.item_knowledge_dto(kind_id),
            ItemKnowledgeDto::Tried,
            "{kind_id}"
        );
        assert!(
            update.events.iter().any(|entry| entry.kind == event),
            "{kind_id}"
        );
    }
}

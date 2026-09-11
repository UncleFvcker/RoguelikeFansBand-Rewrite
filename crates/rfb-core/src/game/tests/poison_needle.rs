// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn needle_game(build: &str, race: &str) -> Game {
    let mut game = Game::new_with_build_race_and_name(414, build, race, "Needle").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    give_inventory_item(&mut game, "test.needle", "demo.item.poison-needle");
    game.identify_item_instance("test.needle", ItemIdentificationRequest::new(true));
    game.equip_inventory_item("test.needle", None).unwrap();
    game
}

fn target(game: &mut Game, kind: &str) {
    game.entities.clear();
    game.push_generated_actor(
        "test.needle-target".into(),
        kind,
        Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        },
    );
}

fn attack(game: &mut Game) -> (u16, Vec<DomainEvent>) {
    let mut events = Vec::new();
    let outcome = game
        .resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    (outcome.attacks_used, events)
}

fn hits(events: &[DomainEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::PlayerMeleeHit { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .collect()
}

#[test]
fn ordinary_generation_reaches_pickup_equipment_restrictions_and_saved_combat() {
    let mut game = needle_game("demo.build.warrior", "demo.race.rfb-human");
    game.items.clear();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-30".into(),
        depth: 30,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    };
    // Keep the full source allocation, quality and materialization path.
    let mut generated = None;
    for _ in 0..20_000 {
        let items = game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap();
        if let Some(item) = items
            .into_iter()
            .find(|item| item.kind_id == "demo.item.poison-needle")
        {
            generated = Some(item);
            break;
        }
    }
    let item = generated.expect("formal 30/6 allocation must generate Poison Needle");
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert_eq!(item.enchantments, ItemEnchantmentsDto::default());
    assert!(!item.is_artifact(&game.content));
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&id, None).unwrap();
    assert!(game.item_resists_enchantment(&game.items[0]));
    give_inventory_item(&mut game, "test.craft", "demo.item.crafting-scroll");
    let before = (game.items.clone(), game.rng.clone(), game.world_tick);
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.craft".into(),
            target: Some(TargetSelection::Item {
                item_id: id.clone(),
            }),
        },
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
    assert_eq!(
        (game.items.clone(), game.rng.clone(), game.world_tick),
        before
    );
    target(&mut game, "demo.actor.warrens-keeper");
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored.snapshot().player.melee_profile,
        game.snapshot().player.melee_profile
    );
    assert_eq!(attack(&mut restored), attack(&mut game));
    assert_eq!(restored.rng, game.rng);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored
            .generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
        game.generate_loot_instances(&context, ItemLocation::Inventory)
            .unwrap(),
    );
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn extra_blows_tonberry_berserker_and_duelist_keep_one_point_preview_and_one_blow() {
    for (build, race) in [
        ("demo.build.warrior", "demo.race.rfb-human"),
        ("demo.build.warrior", "rfb-legacy.race.tonberry"),
        ("demo.build.berserker", "demo.race.rfb-human"),
        ("demo.build.duelist", "demo.race.rfb-human"),
    ] {
        let mut game = needle_game(build, race);
        game.progress.level = 50;
        // A controlled equipment bonus exercises extra and fractional blows;
        // it is not an Ego or enchantment applied to the unmodifiable needle.
        give_inventory_item(&mut game, "test.extra-blows", "demo.item.ring");
        let bonuses = &mut game
            .items
            .last_mut()
            .unwrap()
            .intrinsic_properties
            .equipment_bonuses;
        bonuses.melee_attacks = 3;
        bonuses.melee_attacks_delta_percent = 75;
        bonuses.melee_damage = 100;
        game.identify_item_instance("test.extra-blows", ItemIdentificationRequest::new(true));
        game.equip_inventory_item("test.extra-blows", None).unwrap();
        let stats = game.player_derived_stats();
        let profile = game.player_melee_profile(&stats);
        assert_eq!(
            (profile.attacks, profile.extra_attack_chance_percent),
            (1, 0),
            "{build}/{race}"
        );
        assert_eq!(profile.to_dto().to_damage, 0);
        let details = game.character_trait_details(&stats);
        assert_eq!(details.melee_damage[0].base_damage, Some([1, 1]));
        assert_eq!(details.melee_damage[0].damage_percent, 100);
        let blows = details
            .stats
            .iter()
            .find(|stat| stat.id == "melee-attacks-hundredths")
            .unwrap();
        assert_eq!(blows.value, Some(100));
        assert_eq!(
            blows
                .sources
                .iter()
                .map(|source| source.amount)
                .sum::<i32>(),
            100
        );
        target(&mut game, "demo.actor.warrens-keeper");
        let (used, events) = attack(&mut game);
        assert_eq!(used, 1);
        assert_eq!(hits(&events), [1]);
        if build == "demo.build.duelist" {
            assert_eq!(
                game.duelist_equipment_error(),
                Some("duelist-poison-needle")
            );
            assert!(game.duelist_target_id.is_none());
        }
    }
}

#[test]
fn vital_points_respect_unique_and_unique2_with_metal_babble_exception() {
    let mut base = needle_game("demo.build.warrior", "demo.race.rfb-human");
    let mut penalty = monster_combat::melee_status(STATUS_STUN, 10, "test.hit-penalty").status;
    penalty.granted_equipment_bonuses.melee_skill = -100_000;
    base.player.statuses.push(penalty);
    assert_eq!(
        base.player_melee_profile(&base.player_derived_stats())
            .melee_skill
            .value,
        0
    );
    for (kind, can_kill) in [
        ("demo.actor.sheep", true),
        ("demo.actor.warrens-keeper", false),
        ("demo.actor.silver-angel", false),
        ("demo.actor.metal-babble-unique", true),
    ] {
        target(&mut base, kind);
        // Controlled HP distinguishes the vital kill from ordinary damage;
        // this boundary test does not save the modified monster.
        base.entities[0].hp = 100_000;
        base.entities[0].max_hp = 100_000;
        // The needle replaces damage after both resistance and invulnerability.
        base.entities[0]
            .resistances
            .set(DamageType::Physical, ResistanceLevel::Immune);
        let mut shield =
            monster_combat::melee_status(STATUS_INVULNERABILITY, 50, "test.shield").status;
        shield.incoming_damage_percent = 0;
        base.entities[0].statuses.push(shield);
        let mut saw_kill = false;
        let mut saw_one = false;
        for seed in 0..128 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let (used, events) = attack(&mut game);
            assert_eq!(used, 1);
            let damage = hits(&events);
            assert_eq!(damage.len(), 1, "needle ignores normal AC and hit skill");
            match damage[0] {
                1 => {
                    saw_one = true;
                    assert_eq!(game.entities[0].hp, 99_999);
                }
                100_001 => {
                    assert!(can_kill, "{kind}");
                    saw_kill = true;
                    assert!(game.entities.is_empty());
                    assert!(
                        events
                            .iter()
                            .any(|event| matches!(event, DomainEvent::PlayerSlew { .. }))
                    );
                    if kind == "demo.actor.metal-babble-unique" {
                        assert!(!game.unique_actor_kind_is_available(kind));
                    }
                }
                damage => panic!("unexpected needle damage {damage} against {kind}"),
            }
        }
        assert!(saw_one);
        assert_eq!(saw_kill, can_kill, "{kind}");
    }
}

#[test]
fn dual_needles_use_weapon_count_and_skip_normal_hit_critical_and_fractional_rolls() {
    let mut base = needle_game("demo.build.warrior", "demo.race.rfb-human");
    give_inventory_item(&mut base, "test.offhand", "demo.item.poison-needle");
    let slot = base
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "shield")
        .unwrap()
        .id
        .clone();
    base.equip_inventory_item("test.offhand", Some(&slot))
        .unwrap();
    target(&mut base, "demo.actor.warrens-keeper");
    let mut counts = BTreeSet::new();
    for seed in 0..64 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        let mut count = 0;
        for _ in 0..2 {
            if expected.bounded(2) == 0 {
                count += 1;
                expected.bounded(1); // Existing shared 1d1 damage roll.
                expected.bounded(6); // RFB randint1(randint1(7/7)+5); unique still rolls.
            }
        }
        let (used, events) = attack(&mut game);
        assert_eq!(used, 2);
        assert_eq!(hits(&events), vec![1; count]);
        assert_eq!(game.rng, expected);
        counts.insert(count);
    }
    assert_eq!(counts, BTreeSet::from([0, 1, 2]));
}

#[test]
fn vampiric_armor_uses_ordinary_drain_and_vital_kills_do_not_heal() {
    let mut base = needle_game("demo.build.warrior", "demo.race.rfb-human");
    // Controlled armor properties exercise the shared vampiric consumer;
    // the needle itself remains plain and unbrandable.
    give_inventory_item(
        &mut base,
        "test.vampiric-gloves",
        "demo.item.leather-gloves",
    );
    let properties = &mut base.items.last_mut().unwrap().intrinsic_properties;
    properties.passives.insert(EquipmentPassive::Vampiric);
    properties.equipment_bonuses.melee_damage = 12;
    base.equip_inventory_item("test.vampiric-gloves", None)
        .unwrap();
    base.player.hp = 1;
    target(&mut base, "demo.actor.sheep");
    base.entities[0].hp = 100_000;
    base.entities[0].max_hp = 100_000;
    let mut results = BTreeSet::new();
    for seed in 0..64 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let (_, events) = attack(&mut game);
        let killed = game.entities.is_empty();
        results.insert(killed);
        if killed {
            assert_eq!(game.player.hp, 1);
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::PlayerVampiricHealed { .. }))
            );
        } else {
            assert_eq!(hits(&events), [1]);
            assert!(game.player.hp > 1);
        }
    }
    assert_eq!(results, BTreeSet::from([false, true]));
}

// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

const REWARDS: [(&str, &str, u16); 4] = [
    ("fang-farmer-maggots-dog", "dog-collar-of-fang", 2),
    ("wolf-farmer-maggots-dog", "dog-collar-of-wolf", 2),
    ("grip-farmer-maggots-dog", "dog-collar-of-grip", 2),
    ("the-multi-hued-centipede", "multi-hued-centipede", 30),
];

const Q2_REWARDS: [(&str, &str); 15] = [
    ("ubbo-sathla-the-unbegotten-source", "ubbo-sathla"),
    ("glaurung-father-of-the-dragons", "dragonkind"),
    ("vecna-the-emperor-lich", "emperor-lich"),
    ("carcharoth-the-jaws-of-thirst", "dog-collar-of-carcharoth"),
    ("ymir-the-ice-giant", "ymir"),
    ("ariel-queen-of-air", "ariel"),
    ("moire-queen-of-rebma", "moire"),
    ("quaker-master-of-earth", "quaker"),
    ("the-emperor-quylthulg", "emperor-quylthulg"),
    ("oremorj-the-cyberdemon-lord", "cyberdemon-lord"),
    ("ulik-the-troll", "ulik"),
    ("omarax-the-eye-tyrant", "eyes"),
    ("kundry-queen-of-the-lost-haven", "kundry"),
    ("loge-spirit-of-fire", "loge"),
    ("jack-of-lanterns", "pumpkin-lamp-of-jack-of-lanterns"),
];

const Q3_REWARDS: [(&str, &str); 11] = [
    ("master-tonberry", "master-tonberry"),
    ("surtur-the-giant-fire-demon", "twilight"),
    ("the-stormbringer", "stormbringer"),
    ("gothmog-the-high-captain-of-balrogs", "gothmog"),
    ("ungoliant-the-unlight", "devouring-darkness"),
    ("ungoliant-the-unlight", "unlight-cloak-of-ungoliant"),
    ("mephistopheles-lord-of-hell", "mephistopheles"),
    ("typhoeus-the-storm-giant", "typhoeus"),
    ("kronos-lord-of-the-titans", "kronos"),
    ("the-lernean-hydra", "eye-of-the-hydra"),
    ("atlas-the-titan", "atlas"),
];

#[test]
fn q2_q3_named_death_equipment_and_saved_continuation() {
    let mut fresh = Game::new_with_build(523, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut fresh);
    for (actor, slug) in Q2_REWARDS.into_iter().chain(Q3_REWARDS) {
        let kind = format!("demo.item.{slug}");
        let actor_kind = format!("demo.actor.{actor}");
        let mut game = if slug == "master-tonberry" {
            Game::new_with_build_race_and_name(523, "demo.build.warrior",
                "rfb-legacy.race.tonberry", Game::DEFAULT_PLAYER_NAME).unwrap()
        } else { fresh.clone() };
        if game.actor_kind_is_dungeon_guardian(&actor_kind) {
            let floor = game.content.world(&game.world_id).unwrap().procedural_floors.iter()
                .find(|floor| floor.guardian.as_ref().is_some_and(|guardian| guardian.actor_kind_id == actor_kind)
                    && floor.dungeon_id.as_ref().is_some_and(|id| game.dungeon_is_active(id)))
                .unwrap().id.clone();
            // Prepare the actual active final floor, retaining its guardian reward.
            assert!(game.transition_floor(floor, None, None, false).unwrap().is_some());
        }
        // Prepared adjacent source actor and successful seed, not natural leveling.
        prepare_combat(&mut game, &actor_kind);
        game = successful_kill_start(&game, &kind);
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        let attack = GameCommand::Move { direction: Direction::East };
        assert_eq!(dispatch_next(&mut game, attack.clone()).events,
            dispatch_next(&mut loaded, attack).events, "{slug}");
        assert_eq!(game.state_hash(), loaded.state_hash(), "{slug}");
        assert_eq!(game.actor_kind_available_instance_count(&actor_kind), 0);
        let reward = game.items.iter().find(|item| item.kind_id == kind).unwrap().clone();
        assert!(game.generated_artifact_ids.contains(&kind));
        if let Some(guardian_reward) = game.dungeon_guardian_floor_for_actor(&actor_kind)
            .and_then(|floor| floor.guardian.as_ref())
            .and_then(|guardian| guardian.reward_artifact_item_kind_id.as_ref()) {
            assert!(game.items.iter().any(|item| &item.kind_id == guardian_reward),
                "named reward must supplement the existing dungeon reward");
        }
        let ItemLocation::Ground(position) = reward.location else { panic!("ground reward") };
        game.player.position = position;
        game.pick_up_item_at_player(Some(&reward.id)).unwrap();
        game.reveal_current_visibility();
        game = Game::from_save(game.to_save()).unwrap();
        game.identify_item_instance(&reward.id, ItemIdentificationRequest::new(true));
        game.equip_inventory_item(&reward.id, None).unwrap();
        let properties = game.equipment_modifiers();
        let bonuses = game.player_equipment_bonuses();
        match slug {
            "ymir" => {
                assert_eq!((properties.strength, properties.dexterity, properties.constitution), (3, -3, 3));
                let resistances = game.effective_player_resistances();
                assert_eq!(resistances.level(DamageType::Cold), ResistanceLevel::Immune);
                assert_eq!(resistances.level(DamageType::Fire), ResistanceLevel::Vulnerable);
                assert_eq!(reward.curse, None, "negative DEX does not invent a curse");
            }
            "cyberdemon-lord" => {
                assert_eq!(properties.defense, 90);
                assert_eq!((bonuses.melee_skill, bonuses.melee_damage), (-5, 15));
            }
            "ariel" => {
                assert_eq!((properties.speed, properties.dexterity), (5, 5));
                assert!(game.player_equipment_passives().contains(&EquipmentPassive::Levitation));
            }
            "eyes" => assert_eq!((bonuses.search_skill, bonuses.perception_skill), (15, 15)),
            "kundry" => assert_eq!((bonuses.device_skill, bonuses.magic_resistance_percent), (16, 10)),
            "pumpkin-lamp-of-jack-of-lanterns" => {
                assert_eq!(game.player_light_radius(), Some(4));
                assert_eq!(reward.fuel, None);
                assert!(game.player_equipment_passives().contains(&EquipmentPassive::EspHuman));
            }
            "master-tonberry" => {
                assert_eq!(bonuses.melee_attacks_delta_percent, -200);
                assert_eq!(properties.speed, -2);
                assert_eq!(reward.curse, None);
            }
            "twilight" | "stormbringer" | "gothmog" => {
                assert_eq!(reward.curse, Some(ItemCurseSeverityDto::Heavy));
                if slug != "gothmog" {
                    assert!(reward.intrinsic_properties.rfb_flags.contains("AGGRAVATE"));
                }
                if slug == "twilight" {
                    assert!(reward.intrinsic_properties.rfb_flags.contains("TY_CURSE"));
                    assert_eq!(reward.intrinsic_curse_effects.len(), 1);
                }
                if slug == "stormbringer" {
                    assert!(reward.intrinsic_properties.rfb_flags.contains("DRAIN_EXP"));
                }
            }
            "unlight-cloak-of-ungoliant" => {
                assert_eq!(game.player_light_radius(), None);
                assert_eq!((bonuses.search_skill, bonuses.perception_skill), (30, 30));
                assert!(game.player_equipment_passives().contains(&EquipmentPassive::EspAnimal));
            }
            "eye-of-the-hydra" => {
                assert_eq!(game.player_light_radius(), None);
                assert_eq!(reward.fuel, None);
            }
            _ => {}
        }
        if matches!(slug, "dragonkind" | "emperor-lich" | "emperor-quylthulg") {
            assert_eq!(reward.rolled_affixes.len(), 1);
            assert!(!reward.rolled_affixes[0].properties.resistances.is_empty());
        }
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        assert_eq!(game.state_hash(), loaded.state_hash());
        assert_eq!(dispatch_next(&mut game, GameCommand::Wait).events,
            dispatch_next(&mut loaded, GameCommand::Wait).events);
        assert_eq!(game.state_hash(), loaded.state_hash());
        let slot = match &loaded.items.iter().find(|item| item.id == reward.id).unwrap().location {
            ItemLocation::Equipped { slot_id } => slot_id.clone(),
            _ => panic!("equipped reward"),
        };
        if reward.curse.is_some() {
            assert!(loaded.unequip_slot(&slot).is_none(), "heavy curse survives loading");
            continue;
        }
        loaded.unequip_slot(&slot).unwrap();
        assert_eq!(loaded.equipment_modifiers(), Default::default());
        assert_eq!(loaded.player_equipment_bonuses(), Default::default());
    }
}

#[test]
fn q2_q3_dungeon_rewards_remain_in_complete_allocation() {
    let mut game = Game::new_with_build(524, "demo.build.warrior").unwrap();
    // R'lyeh reaches depth96 and admits both preferred and other source actors.
    // Moire is wild-only; Vecna/Kundry retain their dedicated guardian floors.
    let floor = "demo.floor.rlyeh-depth-96";
    let policy = game.content.encounter_table("demo.encounter-table.rlyeh")
        .unwrap().global_allocation.clone().unwrap();
    for actor in ["demo.actor.vecna-the-emperor-lich", "demo.actor.kundry-queen-of-the-lost-haven"] {
        assert!(game.content.world(&game.world_id).unwrap().procedural_floors.iter()
            .any(|floor| floor.guardian.as_ref().is_some_and(|guardian| guardian.actor_kind_id == actor)));
    }
    let mut remaining = Q2_REWARDS.iter().chain(Q3_REWARDS.iter())
        .filter(|(actor, slug)| *slug != "moire"
            && !game.actor_kind_is_dungeon_guardian(&format!("demo.actor.{actor}")))
        .map(|(actor, _)| format!("demo.actor.{actor}")).collect::<BTreeSet<_>>();
    for _ in 0..100_000 {
        if let Some(actor) = game.select_original_allocated_monster(
            floor, &policy, 96, 96, None, &[], None, None,
        ) {
            remaining.remove(&actor);
        }
        if remaining.is_empty() { break; }
    }
    assert!(remaining.is_empty(), "unreachable Q2/Q3 actors: {remaining:?}");
}

#[test]
fn q2_eyes_and_kundry_activate_and_restore_partial_recovery() {
    for slug in ["eyes", "kundry"] {
        let mut game = Game::new_with_build(525, "demo.build.mage-arcane-sorcery").unwrap();
        choose_human_talent_if_pending(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        clear_monsters(&mut game);
        game.items.clear();
        game.progress.level = 30;
        game.refresh_player_resource_maxima();
        let id = "test.q2.activation";
        give_inventory_item(&mut game, id, &format!("demo.item.{slug}"));
        game.identify_item_instance(id, ItemIdentificationRequest::new(true));
        game.equip_inventory_item(id, None).unwrap();
        game.resources.get_mut("demo.resource.mana").unwrap().current = 0;
        game.glow.fill(false);
        assert!(game.resources["demo.resource.mana"].maximum > 15);
        let activate = |game: &mut Game| {
            let mut events = Vec::new();
            game.use_inventory_item(id, Some(&TargetSelection::SelfTarget), None,
                &mut events, &mut BTreeSet::new(), &mut Vec::new()).unwrap();
            events
        };
        let seed = (0..1000).find(|seed| RfbRng::seeded(*seed).bounded(100) < 5).unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        assert_eq!(activate(&mut game), activate(&mut loaded));
        assert_eq!(game.state_hash(), loaded.state_hash());
        assert_eq!(game.items[0].charges.unwrap().current, 0);
        if slug == "kundry" {
            assert_eq!(game.resources["demo.resource.mana"].current, 15);
        } else {
            assert!(game.glow.iter().all(|glow| *glow));
            assert!(!game.player_has_status_kind("rfb.status.telepathy"));
        }
        let start = game.world_tick;
        for tick in 1..=150 {
            game.world_tick = start + tick;
            game.process_inventory_device_recovery(&mut Vec::new());
        }
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        for tick in 151..=300 {
            for game in [&mut game, &mut loaded] {
                game.world_tick = start + tick;
                game.process_inventory_device_recovery(&mut Vec::new());
                assert_eq!(game.items[0].charges.unwrap().current, u32::from(tick == 300));
            }
        }
        assert_eq!(game.state_hash(), loaded.state_hash());
    }
}

#[test]
#[ignore = "explicit Q1 combat preparation for ordinary Tauri standalone acceptance"]
fn export_q1_desktop_save() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut game = Game::from_save(payload).unwrap();
    choose_human_talent_if_pending(&mut game);
    prepare_combat(&mut game, "demo.actor.fang-farmer-maggots-dog");
    let start = successful_kill_start(&game, "demo.item.dog-collar-of-fang");
    let mut game = start.clone();
    let mut steps = Vec::new();
    let attack = GameCommand::Move {
        direction: Direction::East,
    };
    dispatch_next(&mut game, attack.clone());
    steps.push(serde_json::json!({"command":attack,"hash":game.state_hash()}));
    assert!(
        !game
            .entities
            .iter()
            .any(|a| a.kind_id == "demo.actor.fang-farmer-maggots-dog")
    );
    let reward = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.dog-collar-of-fang")
        .unwrap()
        .id
        .clone();
    let move_to_reward = GameCommand::Move {
        direction: Direction::East,
    };
    dispatch_next(&mut game, move_to_reward.clone());
    steps.push(serde_json::json!({"command":move_to_reward,"hash":game.state_hash()}));
    if matches!(
        game.items
            .iter()
            .find(|item| item.id == reward)
            .unwrap()
            .location,
        ItemLocation::Ground(_)
    ) {
        dispatch_next(&mut game, GameCommand::PickUp);
        steps.push(serde_json::json!({"command":GameCommand::PickUp,"hash":game.state_hash()}));
    }
    for command in [
        GameCommand::Equip {
            item_id: reward.clone(),
            slot_id: None,
        },
        GameCommand::Wait,
    ] {
        dispatch_next(&mut game, command.clone());
        steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
    }
    assert!(matches!(
        game.items
            .iter()
            .find(|item| item.id == reward)
            .unwrap()
            .location,
        ItemLocation::Equipped { .. }
    ));
    assert_eq!(game.equipment_modifiers().strength, 1);
    assert_eq!(
        Game::from_save(start.to_save()).unwrap().state_hash(),
        start.state_hash()
    );
    std::fs::write(
        directory.join("q1-fang.rfbsave"),
        rfb_save::encode(&header, &start.to_save()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&serde_json::json!([
            {"name":"q1-fang","initialHash":start.state_hash(),"steps":steps}
        ]))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn q3_tonberry_drop_tests_permanent_identity_before_both_probability_rolls() {
    let mut base = Game::new_with_build_race_and_name(526, "demo.build.warrior",
        "rfb-legacy.race.tonberry", Game::DEFAULT_PLAYER_NAME).unwrap();
    prepare_combat(&mut base, "demo.actor.master-tonberry");
    let target = base.entities[0].clone();
    let kind = "demo.item.master-tonberry";
    for (gate, roll, bad_luck, expected) in [
        (1, 0, false, false), (0, 98, false, true), (0, 99, false, false),
        (0, 74, true, true), (0, 75, true, false),
    ] {
        let seed = (0..100_000).find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(14) == gate && rng.bounded(100) == roll
        }).unwrap();
        let mut game = base.clone();
        if bad_luck { game.progress.active_mutation_ids.insert("rfb.mutation.bad-luck".into()); }
        game.rng = RfbRng::seeded(seed);
        let (drops, _) = game.generate_death_loot(&target).unwrap();
        assert_eq!(drops.iter().any(|item| item.kind_id == kind), expected);
        if expected {
            // A temporary Tonberry form must not qualify a human birth identity.
            let mut human = Game::new_with_build(526, "demo.build.warrior").unwrap();
            choose_human_talent_if_pending(&mut human);
            prepare_combat(&mut human, "demo.actor.master-tonberry");
            let mut form = monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.q3.form").status;
            form.granted_race_id = Some("rfb-legacy.race.tonberry".into());
            human.player.statuses.push(form);
            human.rng = RfbRng::seeded(seed);
            assert!(!human.generate_death_loot(&target).unwrap().0.iter().any(|item| item.kind_id == kind));
        }
    }
}

#[test]
fn q3_ungoliant_selects_once_before_chance_and_never_replaces_a_generated_choice() {
    let mut base = Game::new_with_build(527, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut base);
    prepare_combat(&mut base, "demo.actor.ungoliant-the-unlight");
    let target = base.entities[0].clone();
    let kinds = ["demo.item.devouring-darkness", "demo.item.unlight-cloak-of-ungoliant"];
    for choice in 0..2 {
        for roll in [4, 5] {
            let seed = (0..10_000).find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(2) == choice as u64 && rng.bounded(100) == roll
            }).unwrap();
            for generated in [false, true] {
                let mut game = base.clone();
                if generated { game.generated_artifact_ids.insert(kinds[choice].into()); }
                game.rng = RfbRng::seeded(seed);
                let drops = game.generate_death_loot(&target).unwrap().0;
                let rewards = drops.iter().filter(|item| kinds.contains(&item.kind_id.as_str())).collect::<Vec<_>>();
                assert_eq!(rewards.len(), usize::from(roll == 4 && !generated));
                if let Some(reward) = rewards.first() { assert_eq!(reward.kind_id, kinds[choice]); }
                assert!(!game.generated_artifact_ids.contains(kinds[1 - choice]));
            }
        }
    }
}

#[test]
fn q3_playable_balrog_keeps_ordinary_gothmog_construction() {
    let mut game = Game::new_with_build_race_and_name(528, "demo.build.warrior",
        "rfb-legacy.race.balrog", Game::DEFAULT_PLAYER_NAME).unwrap();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(), floor_id: game.current_floor_id.clone(), depth: 95,
        source: LootSource::MonsterDeath { actor_id: "test.q3.gothmog".into() },
    };
    let draft = game.fixed_item_draft(&context, "demo.item.gothmog".into());
    let item = game.commit_generated_item_draft(draft, ItemLocation::Inventory).unwrap();
    assert_eq!(item.curse, Some(ItemCurseSeverityDto::Heavy));
    game.items.push(item);
    let loaded = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), loaded.state_hash());
}

#[test]
fn q3_hydra_eye_heals_cures_source_statuses_and_preserves_recovery() {
    let mut game = Game::new_with_build(529, "demo.build.mage-arcane-sorcery").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.progress.level = 50;
    game.refresh_player_resource_maxima();
    let id = "test.q3.eye";
    give_inventory_item(&mut game, id, "demo.item.eye-of-the-hydra");
    game.identify_item_instance(id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(id, None).unwrap();
    assert_eq!(game.player_light_radius(), None);
    give_inventory_item(&mut game, "test.q3.glow", "demo.item.leather-gloves");
    game.items.iter_mut().find(|item| item.id == "test.q3.glow").unwrap()
        .intrinsic_properties.equipment_bonuses.light_radius = 8;
    game.equip_inventory_item("test.q3.glow", None).unwrap();
    assert_eq!(game.player_light_radius(), Some(1), "eye subtracts seven, not generic DARKNESS three");
    for status in [STATUS_BLINDNESS, STATUS_BLEEDING, STATUS_CONFUSION, STATUS_STUN, STATUS_BERSERK, STATUS_POISON] {
        game.player.statuses.push(monster_combat::melee_status(status, 100, "test.q3.healing").status);
    }
    game.player.hp = 1;
    let seed = (0..1000).find(|seed| RfbRng::seeded(*seed).bounded(100) < 5).unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut loaded = Game::from_save(game.to_save()).unwrap();
    let activate = |game: &mut Game| {
        let mut events = Vec::new();
        game.use_inventory_item(id, Some(&TargetSelection::SelfTarget), None,
            &mut events, &mut BTreeSet::new(), &mut Vec::new()).unwrap();
        events
    };
    assert_eq!(activate(&mut game), activate(&mut loaded));
    assert_eq!(game.state_hash(), loaded.state_hash());
    assert_eq!(game.player.hp, 701.min(game.effective_player_max_hp()));
    for status in [STATUS_BLINDNESS, STATUS_BLEEDING, STATUS_CONFUSION, STATUS_STUN, STATUS_BERSERK] {
        assert!(!game.player_has_status_kind(status));
    }
    assert!(game.player_has_status_kind(STATUS_POISON), "HEAL_CURING does not cure poison");
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    let start = game.world_tick;
    for tick in 1..=1500 {
        game.world_tick = start + tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    let mut loaded = Game::from_save(game.to_save()).unwrap();
    for tick in 1501..=3000 {
        for game in [&mut game, &mut loaded] {
            game.world_tick = start + tick;
            game.process_inventory_device_recovery(&mut Vec::new());
            assert_eq!(game.items[0].charges.unwrap().current, u32::from(tick == 3000));
        }
    }
    assert_eq!(game.state_hash(), loaded.state_hash());
}

#[test]
fn q3_stormbringer_ally_strike_boundary_and_hostile_rng_match_saved_continuation() {
    let mut base = Game::new_with_build(530, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut base);
    prepare_combat(&mut base, "demo.actor.warrens-keeper");
    base.entities[0].hp = 100_000;
    base.entities[0].max_hp = 100_000;
    give_inventory_item(&mut base, "test.q3.black-blade", "demo.item.stormbringer");
    base.equip_inventory_item("test.q3.black-blade", None).unwrap();
    let step = |game: &mut Game| {
        let mut events = Vec::new();
        let outcome = game.resolve_local_player_step(Direction::East, false, &mut events,
            &mut BTreeSet::new(), &mut Vec::new()).unwrap();
        (outcome.melee.is_some(), events)
    };
    for roll in [665, 666] {
        let seed = (0..10_000).find(|seed| RfbRng::seeded(*seed).bounded(1000) == roll).unwrap();
        let mut game = base.clone();
        game.entities[0].controller_id = Some(game.player.id.clone());
        assert!(game.entity_is_visible_to_player(&game.entities[0]));
        game.rng = RfbRng::seeded(seed);
        let mut loaded = Game::from_save(game.to_save()).unwrap();
        let result = step(&mut game);
        assert_eq!(result.0, roll == 666);
        assert_eq!(result, step(&mut loaded));
        assert_eq!(game.state_hash(), loaded.state_hash());
        if roll == 665 { assert_eq!(game.rng.draw_counter, 1); }
    }
    // Moving into a hostile actor must not consume the ally-selection draw.
    let mut moved = base.clone();
    let mut direct = base;
    let movement = step(&mut moved);
    let mut events = Vec::new();
    direct.resolve_player_melee(0, true, &mut events, &mut BTreeSet::new(), &mut Vec::new()).unwrap();
    assert_eq!(movement.1, events);
    assert_eq!(moved.rng, direct.rng);
}

fn prepare_combat(game: &mut Game, actor_kind: &str) {
    clear_monsters(game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 9..=11 {
        for x in 9..=13 {
            replace_terrain(game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.push_generated_actor(
        "test.q1.target".into(),
        actor_kind,
        Position { x: 11, y: 10 },
    );
    // Shortened real melee: preserve source maximum HP and all actor rules.
    game.entities[0].hp = 1;
    game.entities[0].nice = true;
    game.entities[0].energy_need = STANDARD_ACTION_COST;
    game.player.hp = game.effective_player_max_hp();
    game.reveal_current_visibility();
}

fn successful_kill_start(base: &Game, item_kind: &str) -> Game {
    (0..1024)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let before = game.clone();
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            game.items
                .iter()
                .any(|item| item.kind_id == item_kind)
                .then_some(before)
        })
        .expect("a real melee death with the source reward probability")
}

#[test]
fn q1_global_allocation_death_equipment_and_saved_continuation() {
    let mut fresh = Game::new_with_build(521, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut fresh);
    let surface = fresh.clone();
    place_player_on_terrain(&mut fresh, "demo.terrain.stairs-down");
    dispatch_next(&mut fresh, GameCommand::TraverseStairs);
    assert_eq!(fresh.current_floor_id, "demo.floor.warrens-depth-1");
    for (actor, slug, depth) in REWARDS {
        let mut game = if depth == 30 {
            surface.clone()
        } else {
            fresh.clone()
        };
        let dungeon = if depth == 2 { "warrens" } else { "orc-cave" };
        let floor_id = format!("demo.floor.{dungeon}-depth-{depth}");
        // Explicit depth preparation; selection retains the complete formal pool.
        assert!(
            game.transition_floor(floor_id.clone(), None, None, false)
                .unwrap()
                .is_some()
        );
        clear_monsters(&mut game);
        let policy = game
            .content
            .encounter_table(&format!("demo.encounter-table.{dungeon}"))
            .unwrap()
            .global_allocation
            .clone()
            .unwrap();
        let kind = format!("demo.actor.{actor}");
        let selected = (0..20_000)
            .find_map(|_| {
                game.select_original_allocated_monster(
                    &floor_id,
                    &policy,
                    depth,
                    depth,
                    None,
                    &[],
                    None,
                    None,
                )
                .filter(|id| id == &kind)
            })
            .expect("Q1 actor must be reachable in the unchanged global allocation");
        prepare_combat(&mut game, &selected);
        let item_kind = format!("demo.item.{slug}");
        game = successful_kill_start(&game, &item_kind);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(game.state_hash(), restored.state_hash());
        // Loading publishes a full snapshot; its subsequent visual deltas need
        // not match a session whose last publication predates test preparation.
        assert_eq!(
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East
                }
            )
            .events,
            dispatch_next(
                &mut restored,
                GameCommand::Move {
                    direction: Direction::East
                }
            )
            .events
        );
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.actor_kind_available_instance_count(&kind), 0);
        let reward = game
            .items
            .iter()
            .find(|item| item.kind_id == item_kind)
            .unwrap()
            .clone();
        let ItemLocation::Ground(position) = reward.location else {
            panic!("ground reward")
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
        game = Game::from_save(game.to_save()).unwrap();
        game.identify_item_instance(&reward.id, ItemIdentificationRequest::new(true));
        game.equip_inventory_item(&reward.id, None).unwrap();
        let properties = game.equipment_modifiers();
        let bonuses = game.player_equipment_bonuses();
        match slug {
            "dog-collar-of-fang" => {
                assert_eq!(properties.strength, 1);
                assert_eq!((bonuses.melee_skill, bonuses.melee_damage), (2, 3));
                assert!(game.player_status_immunities().contains("rfb.status.fear"));
            }
            "dog-collar-of-wolf" => {
                assert_eq!((properties.constitution, properties.defense), (1, 7));
                assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
            }
            "dog-collar-of-grip" => assert_eq!((properties.dexterity, properties.speed), (1, 1)),
            _ => {
                assert_eq!(
                    (
                        properties.strength,
                        properties.dexterity,
                        properties.constitution,
                        properties.speed,
                        properties.defense
                    ),
                    (1, 1, 1, 1, 12)
                );
                assert_eq!(
                    (
                        bonuses.melee_skill,
                        bonuses.melee_damage,
                        bonuses.stealth_skill
                    ),
                    (2, 3, 1)
                );
                assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
                for element in [
                    DamageType::Acid,
                    DamageType::Electricity,
                    DamageType::Fire,
                    DamageType::Cold,
                    DamageType::Poison,
                ] {
                    assert_eq!(game.player_resistance_percent(element), 50);
                }
            }
        }
        game.reveal_current_visibility();
        restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            dispatch_next(&mut restored, GameCommand::Wait).events,
            dispatch_next(&mut game, GameCommand::Wait).events
        );
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
        assert!(restored.generated_artifact_ids.contains(&item_kind));
        let ItemLocation::Equipped { slot_id } = restored
            .items
            .iter()
            .find(|item| item.id == reward.id)
            .unwrap()
            .location
            .clone()
        else {
            panic!("equipped reward")
        };
        assert!(restored.unequip_slot(&slot_id).is_some());
        assert_eq!(restored.equipment_modifiers(), Default::default());
        assert_eq!(restored.player_equipment_bonuses(), Default::default());
    }
}

#[test]
fn q1_named_drop_boundaries_pet_unique_and_normal_pool_exclusion() {
    let mut base = Game::new_with_build(522, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut base);
    for (actor, slug, depth) in REWARDS {
        let mut prepared = base.clone();
        prepare_combat(&mut prepared, &format!("demo.actor.{actor}"));
        let target = prepared.entities[0].clone();
        let item_kind = format!("demo.item.{slug}");
        let chance = if depth == 2 { 10 } else { 5 };
        for bad_luck in [false, true] {
            let threshold = if bad_luck {
                chance - chance / 4
            } else {
                chance
            };
            for roll in [threshold - 1, threshold] {
                let mut game = prepared.clone();
                if bad_luck {
                    game.progress
                        .active_mutation_ids
                        .insert("rfb.mutation.bad-luck".into());
                }
                let seed = (0..10_000)
                    .find(|seed| RfbRng::seeded(*seed).bounded(100) == roll)
                    .unwrap();
                game.rng = RfbRng::seeded(seed);
                let (drops, _) = game.generate_death_loot(&target).unwrap();
                assert_eq!(
                    drops.iter().any(|item| item.kind_id == item_kind),
                    roll < threshold
                );
                if roll < threshold {
                    let mut already = prepared.clone();
                    already.generated_artifact_ids.insert(item_kind.clone());
                    already.rng = RfbRng::seeded(seed);
                    let draws = already.rng.draw_counter;
                    assert!(
                        already
                            .generate_death_loot(&target)
                            .unwrap()
                            .0
                            .iter()
                            .all(|item| item.kind_id != item_kind)
                    );
                    assert!(
                        already.rng.draw_counter > draws,
                        "already generated still rolls"
                    );
                }
            }
        }
        let mut pet = target;
        pet.controller_id = Some(prepared.player.id.clone());
        assert!(
            prepared
                .generate_death_loot(&pet)
                .unwrap()
                .0
                .iter()
                .all(|item| item.kind_id != item_kind)
        );
        if slug == "dog-collar-of-fang" {
            pet.controller_id = None;
            prepared.terrain.fill("demo.terrain.wall".into());
            let seed = (0..10_000)
                .find(|seed| RfbRng::seeded(*seed).bounded(100) == 0)
                .unwrap();
            prepared.rng = RfbRng::seeded(seed);
            assert!(
                prepared
                    .generate_death_loot(&pet)
                    .unwrap()
                    .0
                    .iter()
                    .all(|item| item.kind_id != item_kind)
            );
            assert!(
                !prepared.generated_artifact_ids.contains(&item_kind),
                "no legal floor must not reserve uniqueness"
            );
        }
    }
    // Remove all ordinary candidates via the existing uniqueness gate, leaving
    // the four new QUESTITEM definitions ungenerated. Neither base can roll them.
    base.generated_artifact_ids = base
        .content
        .item_definitions()
        .filter(|item| {
            item.artifact_generation.is_some()
                && !REWARDS
                    .iter()
                    .any(|(_, slug, _)| item.id == format!("demo.item.{slug}"))
        })
        .map(|item| item.id.clone())
        .collect();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: base.current_floor_id.clone(),
        depth: 100,
        source: LootSource::MonsterDeath {
            actor_id: "test.pool".into(),
        },
    };
    for kind in ["demo.item.amulet", "demo.item.soft-leather-boots"] {
        assert_eq!(
            base.roll_fixed_artifact_kind_id(&context, Some(kind), false),
            None
        );
    }
}

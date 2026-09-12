// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::stats::AttributeSet;

const KIND: &str = "demo.item.darnya";
const POWER: &str = "demo.item.one-ring";
const START: Position = Position { x: 99, y: 33 };
const EAST: TargetSelection = TargetSelection::Direction {
    direction: Direction::East,
};
// One successful device check followed by each source d10 result, in order.
const SEEDS: [u64; 10] = [183, 32, 167, 273, 86, 452, 95, 122, 71, 602];

fn context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-110".into(),
        depth: 110,
        source: LootSource::MonsterDeath {
            actor_id: "test.c5a".into(),
        },
    }
}

fn prepare(build: &str) -> Game {
    let mut game = Game::new_with_build(511, build).unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = START;
    for y in 27..=39 {
        for x in 95..=120 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.reveal_current_visibility();
    game
}

fn give_ring(game: &mut Game, kind: &str) -> String {
    let draft = game.fixed_item_draft(&context(), kind.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(START))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    id
}

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

#[test]
#[ignore = "prepares a bound One Ring for the focused standalone reading scenario"]
fn export_c5b_desktop_save() {
    let input = std::path::PathBuf::from(std::env::var("C5B_DESKTOP_INPUT").unwrap());
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut game = Game::from_save(payload).unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = START;
    replace_terrain(&mut game, START, "demo.terrain.floor");
    give_ring(&mut game, POWER);
    game.items[0].charges.as_mut().unwrap().current = 0;
    game.items[0].device_recovery_progress = 123;
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    std::fs::write(input.with_file_name("prepared.hash"), game.state_hash()).unwrap();
    std::fs::write(
        input.with_file_name("prepared.rfbsave"),
        rfb_save::encode(&header, &game.to_save()).unwrap(),
    )
    .unwrap();
}

#[test]
fn c5a_darnya_ordinary_generation_permanent_curse_cancel_failure_and_cooldown_survive_save() {
    let mut game = prepare("demo.build.warrior");
    let mut probe = game.clone();
    (0..20_000)
        .find_map(|_| {
            probe
                .generate_loot_instances(&context(), ItemLocation::Inventory)
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == "demo.item.ring" && item.artifact_name.is_none())
        })
        .expect("the complete ordinary pool must produce the ring base");
    let kind = (0..10_000)
        .find_map(|_| {
            game.roll_fixed_artifact_kind_id(&context(), Some("demo.item.ring"), false)
                .filter(|kind| kind == KIND)
        })
        .expect("the observed base must reach Darnya through level, rarity and unique gates");
    assert_eq!(kind, KIND);
    let id = give_ring(&mut game, KIND);
    assert_eq!(
        game.visible_item_modifiers(&game.items[0]),
        StatModifiersDto::default()
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
    assert!(!game.inventory_item_dto(&game.items[0]).usable);
    assert!(activate(&mut restored, &id, Some(&EAST)).contains(&DomainEvent::ItemUseUnavailable));
    game.equip_inventory_item(&id, None).unwrap();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    assert!(
        game.equipment_dto()
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .usable
    );
    let modifiers = game.item_base_modifiers(KIND);
    assert_eq!(
        [
            modifiers.strength,
            modifiers.intelligence,
            modifiers.wisdom,
            modifiers.dexterity,
            modifiers.constitution,
            modifiers.charisma,
            modifiers.speed
        ],
        [-5; 7]
    );
    let bonuses = game.player_equipment_bonuses();
    assert_eq!((bonuses.melee_skill, bonuses.melee_damage), (-15, -15));
    assert_eq!(game.items[0].curse, Some(ItemCurseSeverityDto::Permanent));
    for effect in [
        ItemCurseEffectDto::Aggravate,
        ItemCurseEffectDto::DrainExperience,
        ItemCurseEffectDto::TyCurse,
    ] {
        assert!(game.player_has_equipped_curse_effect(effect));
    }
    assert!(game.item_has_heavy_curse(&game.items[0]));
    let slot = match &game.items[0].location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => unreachable!(),
    };
    give_inventory_item(
        &mut game,
        "test.cleanse",
        "demo.item.greater-cleansing-scroll",
    );
    activate(&mut game, "test.cleanse", None);
    assert_eq!(game.items[0].curse, Some(ItemCurseSeverityDto::Permanent));
    assert!(game.unequip_slot(&slot).is_none());
    give_inventory_item(&mut game, "test.replacement", "demo.item.ring");
    assert!(
        game.equip_inventory_item("test.replacement", Some(&slot))
            .is_none()
    );

    let failure = (0..1000)
        .find(|seed| (5..10).contains(&RfbRng::seeded(*seed).bounded(100)))
        .unwrap();
    for (seed, target, succeeds) in [(SEEDS[0], None, true), (failure, Some(&EAST), false)] {
        game.rng = RfbRng::seeded(seed);
        let mut expected_rng = game.rng.clone();
        expected_rng.bounded(100);
        let before = game.progress.clone();
        let mut saved = Game::from_save(game.to_save()).unwrap();
        let events = activate(&mut game, &id, target);
        assert_eq!(events, activate(&mut saved, &id, target));
        assert!(events.iter().any(|event| matches!(event, DomainEvent::DeviceSkillChecked { succeeded, .. } if *succeeded == succeeds)));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { .. }))
        );
        assert_eq!(game.progress, before);
        assert_eq!(
            game.rng, expected_rng,
            "cancel/failure must not draw the random branch"
        );
        assert_eq!(game.state_hash(), saved.state_hash());
        assert_eq!(charge(&game, &id), 1);
    }
    // Cancelling from the command entry still spends its normal action time.
    game.rng = RfbRng::seeded(SEEDS[0]);
    let tick = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: id.clone(),
            target: None,
        },
    );
    assert!(game.world_tick > tick);
    assert_eq!(charge(&game, &id), 1);
    game.rng = RfbRng::seeded(SEEDS[6]);
    activate(&mut game, &id, Some(&EAST));
    assert_eq!(charge(&game, &id), 0);
    let mut saved = Game::from_save(game.to_save()).unwrap();
    for run in [&mut game, &mut saved] {
        for tick in 1..=5000 {
            run.world_tick += 1;
            run.process_inventory_device_recovery(&mut Vec::new());
            if tick == 4999 {
                assert_eq!(charge(run, &id), 0);
            }
        }
        assert_eq!(charge(run, &id), 1);
        assert_ne!(
            run.roll_fixed_artifact_kind_id(&context(), Some("demo.item.ring"), false),
            Some(KIND.into())
        );
    }
    assert_eq!(game.state_hash(), saved.state_hash());
    assert_eq!(
        game.generate_loot_instances(&context(), ItemLocation::Inventory)
            .unwrap(),
        saved
            .generate_loot_instances(&context(), ItemLocation::Inventory)
            .unwrap()
    );
    assert_eq!(game.rng, saved.rng);
}

#[test]
fn c5a_one_ring_all_ten_rolls_keep_source_weights_permanent_costs_and_saved_rng() {
    let mut base = prepare("demo.build.high-mage-death");
    let id = give_ring(&mut base, KIND);
    base.equip_inventory_item(&id, None).unwrap();
    base.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    base.apply_player_experience(100, &mut Vec::new());
    base.progress.attributes = AttributeSet {
        strength: 3,
        intelligence: 4,
        wisdom: 18,
        dexterity: 19,
        constitution: 118,
        charisma: 80,
    };
    base.progress.maximum_attributes = AttributeSet {
        strength: 3,
        intelligence: 18,
        wisdom: 18,
        dexterity: 19,
        constitution: 118,
        charisma: 120,
    };
    use rfb_protocol::VirtueKindDto::*;
    base.virtues = [
        Sacrifice,
        Enlightenment,
        Honour,
        Valour,
        Justice,
        Harmony,
        Compassion,
        Temperance,
    ]
    .map(|kind| rfb_protocol::VirtueDto {
        kind,
        value: match kind {
            Sacrifice => 52,
            Enlightenment => -52,
            _ => 0,
        },
    });
    for kind in [
        crate::effect::STATUS_SUSTAIN_STRENGTH,
        crate::effect::STATUS_SUSTAIN_INTELLIGENCE,
        crate::effect::STATUS_SUSTAIN_WISDOM,
        crate::effect::STATUS_SUSTAIN_DEXTERITY,
        crate::effect::STATUS_SUSTAIN_CONSTITUTION,
        crate::effect::STATUS_SUSTAIN_CHARISMA,
        crate::effect::STATUS_HOLD_LIFE,
    ] {
        base.player
            .statuses
            .push(monster_combat::melee_status(kind, 6000, "test.c5a").status);
    }
    let mut power = monster_combat::melee_status(STATUS_BERSERK, 6000, "test.c5a").status;
    power.granted_modifiers.device_power_bonus = 5;
    base.player.statuses.push(power);
    base.player
        .statuses
        .sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
    base.refresh_player_resource_maxima();
    base.player.hp = base.effective_player_max_hp();
    base.reveal_current_visibility();
    for (index, seed) in SEEDS.into_iter().enumerate() {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        let events = activate(&mut game, &id, Some(&EAST));
        assert_eq!(events, activate(&mut restored, &id, Some(&EAST)));
        let expected_branch = match index {
            0..=1 => 0,
            2 => 1,
            3..=5 => 2,
            _ => 3,
        };
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::RandomChoice { roll, branch_index, .. }]
                if *roll == index as i32 + 1 && *branch_index == expected_branch))));
        if index < 2 {
            assert_eq!(
                game.progress.attributes,
                AttributeSet {
                    strength: 3,
                    intelligence: 3,
                    wisdom: 16,
                    dexterity: 17,
                    constitution: 93,
                    charisma: 55
                }
            );
            assert_eq!(
                game.progress.maximum_attributes,
                AttributeSet {
                    strength: 3,
                    intelligence: 16,
                    wisdom: 16,
                    dexterity: 17,
                    constitution: 93,
                    charisma: 95
                }
            );
            assert_eq!(
                (game.progress.experience, game.progress.maximum_experience),
                (75, 82)
            );
            assert_eq!(
                (
                    game.virtue_current(rfb_protocol::VirtueKindDto::Sacrifice),
                    game.virtue_current(rfb_protocol::VirtueKindDto::Enlightenment)
                ),
                if index == 0 { (53, -56) } else { (55, -54) }
            );
            assert_eq!(
                game.rng.draw_counter, 15,
                "including virtue rolls between current and maximum drains"
            );
            let mut healed = Game::from_save(game.to_save()).unwrap();
            // Ordinary restoration can only reach the newly reduced maxima.
            healed.restore_all_player_attributes();
            healed.restore_player_experience_and_life_force(0, &mut Vec::new());
            assert_eq!(healed.progress.attributes, game.progress.maximum_attributes);
            assert_eq!(
                (
                    healed.progress.experience,
                    healed.progress.maximum_experience
                ),
                (82, 82)
            );
        } else {
            assert_eq!(game.progress.attributes, base.progress.attributes);
            assert_eq!(
                (game.progress.experience, game.progress.maximum_experience),
                (100, 100)
            );
            assert_eq!(game.rng.draw_counter, 2);
            match index {
                2 => assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityVisibleDamage { resolution, .. } if resolution.base_raw_damage == 1000))),
                3..=5 => assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityAreaDamage { resolution, .. } if resolution.base_raw_damage == 600 && resolution.radius == 3 && resolution.damage_type == DamageTypeDto::Mana))),
                _ => assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityLanded { .. }))),
            }
        }
        assert_eq!(charge(&game, &id), 0);
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(
            game.state_hash(),
            Game::from_save(game.to_save()).unwrap().state_hash()
        );
    }
}

#[test]
fn c5a_one_ring_dispel_ball_and_bolt_hit_real_targets_without_device_boost() {
    let mut base = prepare("demo.build.warrior");
    let id = give_ring(&mut base, KIND);
    base.equip_inventory_item(&id, None).unwrap();
    base.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    let mut power = monster_combat::melee_status(STATUS_BERSERK, 6000, "test.c5a").status;
    power.granted_modifiers.device_power_bonus = 5;
    base.player.statuses.push(power);
    for (actor_id, x, y) in [
        ("test.center", 102, 33),
        ("test.neighbor", 102, 34),
        ("test.behind-wall", 97, 31),
        ("test.far", 118, 33),
    ] {
        base.push_generated_actor(
            actor_id.into(),
            "demo.actor.great-hell-wyrm",
            Position { x, y },
        );
        let actor = base.entities.last_mut().unwrap();
        actor
            .resistances
            .set(DamageType::Physical, ResistanceLevel::Immune);
    }
    base.push_generated_actor(
        "test.resist-all".into(),
        "demo.actor.metal-babble",
        Position { x: 100, y: 31 },
    );
    replace_terrain(&mut base, Position { x: 98, y: 32 }, "demo.terrain.wall");
    base.player
        .statuses
        .push(monster_combat::melee_status(STATUS_BLINDNESS, 6000, "test.c5a").status);
    base.reveal_current_visibility();
    assert!(!base.entity_is_visible_to_player(&base.entities[0]));
    for (index, expected) in [
        (2, [1000, 1000, 0, 0]),
        (3, [600, 300, 0, 0]),
        (6, [500, 0, 0, 0]),
    ] {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(SEEDS[index]);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        let hp = game
            .entities
            .iter()
            .map(|actor| (actor.id.clone(), actor.hp))
            .collect::<BTreeMap<_, _>>();
        let events = activate(&mut game, &id, Some(&EAST));
        assert_eq!(events, activate(&mut restored, &id, Some(&EAST)));
        for (actor_id, expected) in [
            "test.center",
            "test.neighbor",
            "test.behind-wall",
            "test.far",
        ]
        .into_iter()
        .zip(expected)
        {
            let actor = game
                .entities
                .iter()
                .find(|actor| actor.id == actor_id)
                .unwrap();
            assert_eq!(
                hp[actor_id] - actor.hp,
                expected,
                "branch {index}: {actor_id}"
            );
        }
        assert_eq!(
            game.entities
                .iter()
                .find(|actor| actor.id == "test.resist-all")
                .unwrap()
                .hp,
            hp["test.resist-all"]
        );
        assert_eq!(game.state_hash(), restored.state_hash());
    }
}

#[test]
fn c5a_backlash_recalculates_lost_levels_and_resources_once_and_respects_lower_bounds() {
    let mut base = prepare("demo.build.high-mage-death");
    let id = give_ring(&mut base, KIND);
    base.equip_inventory_item(&id, None).unwrap();
    base.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    for amount in [0, 3, base.experience_required_for_level(40)] {
        let mut game = base.clone();
        game.apply_player_experience(amount, &mut Vec::new());
        if amount <= 3 {
            let low = AttributeSet {
                strength: 3,
                intelligence: 3,
                wisdom: 3,
                dexterity: 3,
                constitution: 3,
                charisma: 3,
            };
            game.progress.attributes = low;
            game.progress.maximum_attributes = low;
        }
        game.refresh_player_resource_maxima();
        let old_max_hp = game.effective_player_max_hp();
        game.player.hp = old_max_hp / 2;
        let old_hp = game.player.hp;
        let mana = game.resources.get_mut("demo.resource.mana").unwrap();
        mana.current = mana.maximum / 2;
        let (old_mana, old_max_mana) = (mana.current, mana.maximum);
        let old_level = game.progress.level;
        game.reveal_current_visibility();
        game.rng = RfbRng::seeded(SEEDS[0]);
        let mut saved = Game::from_save(game.to_save()).unwrap();
        let events = activate(&mut game, &id, Some(&EAST));
        assert_eq!(events, activate(&mut saved, &id, Some(&EAST)));
        let current = amount - amount / 4;
        assert_eq!(
            (game.progress.experience, game.progress.maximum_experience),
            (current, amount - current / 4)
        );
        if amount <= 3 {
            assert_eq!(game.progress.attributes, game.progress.maximum_attributes);
            assert_eq!(game.progress.attributes.strength, 3);
            assert_eq!(game.rng.draw_counter, 2);
        } else {
            assert!(game.progress.level < old_level);
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::PlayerLevelLost { .. }))
            );
            let new_max = game.effective_player_max_hp();
            assert!(new_max < old_max_hp);
            assert_eq!(
                game.player.hp,
                (i64::from(old_hp) * i64::from(new_max) / i64::from(old_max_hp)) as i32
            );
            let mana = &game.resources["demo.resource.mana"];
            assert!(mana.maximum < old_max_mana);
            assert_eq!(
                mana.current,
                (u64::from(old_mana) * u64::from(mana.maximum) / u64::from(old_max_mana)) as u32
            );
        }
        assert_eq!(game.state_hash(), saved.state_hash());
        assert_eq!(
            game.state_hash(),
            Game::from_save(game.to_save()).unwrap().state_hash()
        );
    }
}

#[test]
fn c5b_one_ring_ordinary_generation_permanent_properties_and_shared_activation_survive_save() {
    let mut game = prepare("demo.build.warrior");
    let mut probe = game.clone();
    (0..20_000)
        .find_map(|_| {
            probe
                .generate_loot_instances(&context(), ItemLocation::Inventory)
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == "demo.item.ring")
        })
        .expect("ordinary ring base");
    let selected = (0..20_000)
        .find_map(|_| {
            game.roll_fixed_artifact_kind_id(&context(), Some("demo.item.ring"), false)
                .filter(|kind| kind == POWER)
        })
        .expect("source rarity127 in the complete ring artifact candidate set");
    assert_eq!(selected, POWER);
    // Real one_ability precedes one_high_resistance; HoldLife is an extra,
    // not an intrinsic flag of the One Ring.
    let seed = (0..1000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(10) == 7 && rng.bounded(12) == 0
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut expected_rng = game.rng.clone();
    assert_eq!(expected_rng.bounded(10), 7);
    assert_eq!(expected_rng.bounded(12), 0);
    // Existing profile/maximum/current initialization draws, each with bound1.
    for _ in 0..3 {
        assert_eq!(expected_rng.bounded(1), 0);
    }
    let id = give_ring(&mut game, POWER);
    assert_eq!(game.rng, expected_rng);
    assert!(
        game.items[0]
            .intrinsic_properties
            .passives
            .contains(&EquipmentPassive::HoldLife)
    );
    assert_eq!(game.items[0].rolled_affixes.len(), 1);
    assert_eq!(
        game.items[0].rolled_affixes[0]
            .properties
            .resistances
            .get(&ActorDamageType::Poison),
        Some(&rfb_content::ActorResistanceLevel::Resistant)
    );
    assert!(game.visible_item_passives(&game.items[0]).is_empty());
    let mut loaded = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), loaded.state_hash());
    let before = loaded.effective_player_attributes();
    loaded.equip_inventory_item(&id, None).unwrap();
    loaded.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    assert!(loaded.effective_player_attributes().strength > before.strength);
    let modifiers = loaded.equipment_modifiers();
    assert_eq!(
        [
            modifiers.strength,
            modifiers.intelligence,
            modifiers.wisdom,
            modifiers.dexterity,
            modifiers.constitution,
            modifiers.charisma,
            modifiers.speed
        ],
        [5; 7]
    );
    for attribute in [
        AttributeKind::Strength,
        AttributeKind::Intelligence,
        AttributeKind::Wisdom,
        AttributeKind::Dexterity,
        AttributeKind::Constitution,
        AttributeKind::Charisma,
    ] {
        assert!(loaded.player_sustains_attribute(attribute));
    }
    for passive in [
        EquipmentPassive::SeeInvisible,
        EquipmentPassive::Regeneration,
        EquipmentPassive::EspDemon,
        EquipmentPassive::EspUndead,
        EquipmentPassive::HoldLife,
    ] {
        assert!(loaded.player_equipment_passives().contains(&passive));
    }
    for damage_type in [
        DamageType::Acid,
        DamageType::Electricity,
        DamageType::Fire,
        DamageType::Cold,
    ] {
        assert_eq!(
            loaded.effective_player_resistances().level(damage_type),
            ResistanceLevel::Immune
        );
        let hp = loaded.player.hp;
        loaded.resolve_monster_damage_to_player(
            "test.monster",
            "demo.actor.ogre",
            "test.element",
            0,
            20,
            20,
            damage_type,
            &mut Vec::new(),
        );
        assert_eq!(loaded.player.hp, hp);
    }
    assert_eq!(loaded.items[0].curse, Some(ItemCurseSeverityDto::Permanent));
    assert!(
        loaded
            .content
            .item(POWER)
            .unwrap()
            .rfb_value
            .as_ref()
            .unwrap()
            .flags
            .contains("FIXED_ART")
    );
    let slot = match &loaded.items[0].location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => unreachable!(),
    };
    give_inventory_item(
        &mut loaded,
        "test.cleanse",
        "demo.item.greater-cleansing-scroll",
    );
    activate(&mut loaded, "test.cleanse", None);
    assert!(loaded.unequip_slot(&slot).is_none());
    give_inventory_item(&mut loaded, "test.replacement", "demo.item.ring");
    assert!(
        loaded
            .equip_inventory_item("test.replacement", Some(&slot))
            .is_none()
    );
    loaded.apply_player_experience(100, &mut Vec::new());
    let maximum_before = loaded.progress.maximum_attributes;
    loaded.rng = RfbRng::seeded(SEEDS[0]);
    let mut saved = Game::from_save(loaded.to_save()).unwrap();
    let events = activate(&mut loaded, &id, Some(&EAST));
    assert_eq!(events, activate(&mut saved, &id, Some(&EAST)));
    assert_eq!(
        (
            loaded.progress.experience,
            loaded.progress.maximum_experience
        ),
        (75, 82)
    );
    assert!(loaded.progress.maximum_attributes.strength < maximum_before.strength);
    assert_eq!(charge(&loaded, &id), 0);
    assert!(!loaded.inventory_item_dto(&loaded.items[0]).readable);
    for run in [&mut loaded, &mut saved] {
        for tick in 1..=5000 {
            run.world_tick += 1;
            run.process_inventory_device_recovery(&mut Vec::new());
            if tick == 4999 {
                assert_eq!(charge(run, &id), 0);
            }
        }
        assert_eq!(charge(run, &id), 1);
        assert!(run.generated_artifact_ids.contains(POWER));
        assert_ne!(
            run.roll_fixed_artifact_kind_id(&context(), Some("demo.item.ring"), false),
            Some(POWER.into())
        );
    }
    assert_eq!(loaded.state_hash(), saved.state_hash());
    assert_eq!(
        loaded
            .generate_loot_instances(&context(), ItemLocation::Inventory)
            .unwrap(),
        saved
            .generate_loot_instances(&context(), ItemLocation::Inventory)
            .unwrap()
    );
    assert_eq!(loaded.rng, saved.rng);
}

#[test]
fn c5b_one_ring_reading_uses_pack_or_floor_and_reading_energy_without_activation() {
    for (build, speed_reader) in [
        ("demo.build.warrior", false),
        ("demo.build.warrior", true),
        ("demo.build.berserker", true),
    ] {
        let mut base = prepare(build);
        let id = give_ring(&mut base, POWER);
        if speed_reader {
            assert!(base.gain_mutation("rfb.mutation.speed-reader", &mut Vec::new()));
        }
        base.items[0].charges.as_mut().unwrap().current = 0;
        base.items[0].device_recovery_progress = 123;
        let action = GameAction::UseItem {
            item_id: id.clone(),
            target: None,
            target_glyph: None,
        };
        assert_eq!(
            base.player_mutation_action_energy_cost(&action, STANDARD_ACTION_COST),
            if speed_reader { 50 } else { 100 }
        );
        for location in [ItemLocation::Inventory, ItemLocation::Ground(START)] {
            let mut game = base.clone();
            game.items[0].location = location.clone();
            game.reveal_current_visibility();
            let can_read = !game.player_is_berserker();
            assert_eq!(game.item_inscription_is_readable(&game.items[0]), can_read);
            if location == ItemLocation::Inventory {
                let projection = game.inventory_item_dto(&game.items[0]);
                assert_eq!(projection.usable, can_read);
                assert_eq!(projection.readable, can_read);
                assert!(projection.use_target_spec.is_none());
            } else {
                assert_eq!(
                    game.items_dto()
                        .iter()
                        .find(|item| item.id == id)
                        .unwrap()
                        .readable,
                    can_read
                );
            }
            let before_item = game.items[0].clone();
            let before_knowledge = game.item_property_knowledge.clone();
            let rng = game.rng.clone();
            // Even a supplied activation direction cannot activate a carried ring.
            let events = activate(&mut game, &id, Some(&EAST));
            assert_eq!(
                events.contains(&DomainEvent::OneRingInscriptionRead),
                can_read
            );
            assert_eq!(events.contains(&DomainEvent::ItemUseUnavailable), !can_read);
            assert_eq!(game.rng, rng);
            assert_eq!(game.items[0], before_item);
            assert_eq!(game.item_property_knowledge, before_knowledge);
            let mut saved = Game::from_save(game.to_save()).unwrap();
            let tick = game.world_tick;
            let command = GameCommand::UseItem {
                item_id: id.clone(),
                target: None,
            };
            let read = dispatch_next(&mut game, command.clone());
            assert_eq!(read.events, dispatch_next(&mut saved, command).events);
            assert!(game.world_tick > tick); // Illiteracy also spends reading energy.
            assert_eq!(game.items[0].device_recovery_progress, 123);
            assert_eq!(charge(&game, &id), 0);
            assert_eq!(game.state_hash(), saved.state_hash());
            for status in [STATUS_BLINDNESS, STATUS_CONFUSION] {
                let mut blocked = base.clone();
                blocked
                    .player
                    .statuses
                    .push(monster_combat::melee_status(status, 100, "test.c5b").status);
                let tick = blocked.world_tick;
                let rng = blocked.rng.clone();
                let event = dispatch_next(
                    &mut blocked,
                    GameCommand::UseItem {
                        item_id: id.clone(),
                        target: None,
                    },
                );
                assert!(
                    !event
                        .events
                        .iter()
                        .any(|event| event.kind == "item.one-ring-inscription-read")
                );
                assert_eq!(blocked.world_tick, tick);
                assert_eq!(blocked.rng, rng);
            }
        }
        let mut dark = base;
        descend_one_floor(&mut dark);
        clear_monsters(&mut dark);
        dark.glow.fill(false);
        dark.items.retain(|item| item.id == id);
        assert!(!dark.position_is_lit(dark.player.position));
        let tick = dark.world_tick;
        dispatch_next(
            &mut dark,
            GameCommand::UseItem {
                item_id: id,
                target: None,
            },
        );
        assert_eq!(dark.world_tick, tick);
    }
}

#[test]
fn c5b_one_ring_mastery_and_brands_follow_weapon_hands_and_innate_attacks_after_save() {
    let mut base = prepare("demo.build.warrior");
    let id = give_ring(&mut base, POWER);
    assert!(base.gain_mutation("rfb.mutation.horns", &mut Vec::new()));
    let ring_slot = base
        .body_slots
        .iter()
        .filter(|slot| slot.slot_type == "ring")
        .nth(1)
        .unwrap()
        .id
        .clone();
    base.equip_inventory_item(&id, Some(&ring_slot)).unwrap();
    let offhand_slot = base
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "shield")
        .unwrap()
        .id
        .clone();
    for (weapon_kind, offhand_kind, main_bonus, offhand_bonus, innate_bonus) in [
        ("demo.item.dagger", None, 0, 0, 5),
        ("demo.item.long-sword", None, 5, 0, 0),
        ("demo.item.dagger", Some("demo.item.dagger"), 0, 5, 0),
        (
            "demo.item.dagger",
            Some("demo.item.small-metal-shield"),
            0,
            0,
            5,
        ),
    ] {
        let mut game = base.clone();
        give_inventory_item(&mut game, "test.main", weapon_kind);
        game.equip_inventory_item("test.main", None).unwrap();
        if let Some(kind) = offhand_kind {
            give_inventory_item(&mut game, "test.offhand", kind);
            game.equip_inventory_item("test.offhand", Some(&offhand_slot))
                .unwrap();
        }
        game.push_generated_actor(
            "test.target".into(),
            "demo.actor.great-hell-wyrm",
            Position {
                x: START.x + 1,
                y: START.y,
            },
        );
        game = Game::from_save(game.to_save()).unwrap();
        let stats = game.player_derived_stats();
        let profiles = game.player_melee_profiles(&stats);
        for profile in &profiles {
            let (kind, bonus) = if profile.source_item_id.as_deref() == Some("test.main") {
                (weapon_kind, main_bonus)
            } else {
                (offhand_kind.unwrap(), offhand_bonus)
            };
            let dice = game
                .content
                .item(kind)
                .unwrap()
                .melee_profile
                .as_ref()
                .unwrap()
                .damage_dice;
            assert_eq!(profile.damage_dice, dice + bonus);
            for nonimmune in [
                DamageType::Acid,
                DamageType::Electricity,
                DamageType::Fire,
                DamageType::Cold,
            ] {
                for element in [
                    DamageType::Acid,
                    DamageType::Electricity,
                    DamageType::Fire,
                    DamageType::Cold,
                ] {
                    game.entities[0].resistances.set(
                        element,
                        if element == nonimmune {
                            ResistanceLevel::Normal
                        } else {
                            ResistanceLevel::Immune
                        },
                    );
                }
                assert_eq!(
                    game.player_melee_damage_multiplier(
                        profile,
                        &game.entities[0],
                        game.content.actor("demo.actor.great-hell-wyrm").unwrap()
                    ),
                    if bonus == 5 { 24 } else { 10 }
                );
            }
        }
        let innate = game.player_mutation_innate_attack_profiles(&stats);
        assert_eq!(innate[0].damage_dice, 2 + innate_bonus);
        assert_eq!(
            game.player_melee_damage_multiplier(
                &innate[0],
                &game.entities[0],
                game.content.actor("demo.actor.great-hell-wyrm").unwrap()
            ),
            10
        );
        game.rng = RfbRng::seeded(SEEDS[0]);
        let mut saved = Game::from_save(game.to_save()).unwrap();
        let mut events = Vec::new();
        let hp = game.entities[0].hp;
        game.resolve_player_melee(0, true, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        let mut resumed_events = Vec::new();
        saved
            .resolve_player_melee(
                0,
                true,
                &mut resumed_events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(events, resumed_events);
        assert!(
            game.entities[0].hp < hp,
            "{weapon_kind}/{offhand_kind:?}: {events:?}"
        );
        assert_eq!(game.state_hash(), saved.state_hash());
    }
}

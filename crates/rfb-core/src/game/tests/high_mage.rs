// SPDX-License-Identifier: MPL-2.0

use super::support::{
    clear_monsters, descend_one_floor, dispatch_next, give_inventory_item, replace_terrain,
};
use super::*;

const HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-death";
const ARCANE_HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-arcane";

fn high_mage_game(seed: u64) -> Game {
    Game::new_with_build(seed, HIGH_MAGE_BUILD_ID).expect("Death High-Mage build should create")
}

fn arcane_high_mage_game(seed: u64, level: u16, ability_ids: &[&str]) -> Game {
    let mut game = Game::new_with_build(seed, ARCANE_HIGH_MAGE_BUILD_ID)
        .expect("Arcane High-Mage build should create");
    game.progress.level = level;
    game.progress.max_level = level;
    game.learned_abilities
        .extend(ability_ids.iter().map(|id| (*id).to_owned()));
    give_inventory_item(&mut game, "test.minor-arcana", "demo.item.minor-arcana");
    give_inventory_item(&mut game, "test.major-arcana", "demo.item.major-arcana");
    give_inventory_item(
        &mut game,
        "test.manual-of-mastery",
        "demo.item.manual-of-mastery",
    );
    game.refresh_player_resource_maxima();
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should have mana")
        .current = 100;
    game.debug_ability_casts_succeed = true;
    game
}

#[test]
fn arcane_high_mage_birth_keeps_only_the_first_book_and_is_isolated_from_death() {
    let game = Game::new_with_build(0x4152_4341_4e45, ARCANE_HIGH_MAGE_BUILD_ID)
        .expect("Arcane High-Mage build should create");
    let carried = game
        .items
        .iter()
        .filter(|item| {
            matches!(
                item.location,
                ItemLocation::Inventory | ItemLocation::Equipped { .. }
            )
        })
        .map(|item| item.kind_id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(carried.contains("demo.item.cantrips-for-beginners"));
    assert!(!carried.contains("demo.item.minor-arcana"));
    assert!(!carried.contains("demo.item.major-arcana"));
    assert!(!carried.contains("demo.item.manual-of-mastery"));
    assert!(!carried.contains("demo.item.black-prayers"));

    let learned = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .filter(|ability| ability.source == AbilitySourceDto::Learned)
        .collect::<Vec<_>>();
    assert_eq!(learned.len(), 31);
    assert!(
        learned
            .iter()
            .all(|ability| ability.id.starts_with("demo.ability.arcane-"))
    );
    let zap = learned
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-zap")
        .expect("Zap should be projected");
    assert_eq!(zap.minimum_level, 1);
    assert_eq!(zap.base_resource_cost, 1);
}

#[test]
fn arcane_phlogiston_adds_half_capacity_and_caps_an_equipped_light() {
    let mut game = arcane_high_mage_game(
        0x5048_4c4f_4749_5354,
        11,
        &["demo.ability.arcane-phlogiston"],
    );
    give_inventory_item(&mut game, "test.phlogiston-torch", "demo.item.wooden-torch");
    let torch = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.phlogiston-torch")
        .expect("test torch should exist");
    torch.location = ItemLocation::Equipped {
        slot_id: "light".to_owned(),
    };
    torch
        .fuel
        .as_mut()
        .expect("torch should carry fuel")
        .current = 1_000;

    for expected in [3_500, 5_000] {
        game.resources
            .get_mut("demo.resource.mana")
            .expect("Arcane High-Mage should retain mana")
            .current = 100;
        game.resolve_player_ability(
            "demo.ability.arcane-phlogiston",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Phlogiston should resolve");
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == "test.phlogiston-torch")
                .and_then(|item| item.fuel)
                .expect("torch should retain fuel")
                .current,
            expected
        );
    }
}

#[test]
fn arcane_cure_poison_uses_the_original_fractional_reduction() {
    let mut game = arcane_high_mage_game(
        0x4355_5245_504f_4953,
        11,
        &["demo.ability.arcane-cure-poison"],
    );
    game.player.statuses.push(StatusInstance {
        kind_id: "rfb.status.poison".to_owned(),
        intensity: 1,
        remaining_ticks: 1_000,
        source_id: Some("test.poison".to_owned()),
        granted_modifiers: StatModifiersDto::default(),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });

    game.resolve_player_ability(
        "demo.ability.arcane-cure-poison",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Cure Poison should resolve");
    assert_eq!(game.player.statuses[0].remaining_ticks, 800);

    game.player.statuses[0].remaining_ticks = 80;
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should retain mana")
        .current = 100;
    game.resolve_player_ability(
        "demo.ability.arcane-cure-poison",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Cure Poison should resolve low-level poisoning");
    assert!(
        game.player
            .statuses
            .iter()
            .all(|status| status.kind_id != "rfb.status.poison")
    );
}

#[test]
fn arcane_resist_cold_and_fire_create_independent_spell_powered_statuses() {
    let mut game = arcane_high_mage_game(
        0x5245_5349_5354_3139,
        11,
        &[
            "demo.ability.arcane-resist-cold",
            "demo.ability.arcane-resist-fire",
        ],
    );
    for ability_id in [
        "demo.ability.arcane-resist-cold",
        "demo.ability.arcane-resist-fire",
    ] {
        game.resources
            .get_mut("demo.resource.mana")
            .expect("Arcane High-Mage should retain mana")
            .current = 100;
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap_or_else(|error| panic!("{ability_id} should resolve: {error:?}"));
    }

    for (status_kind_id, damage_type) in [
        ("rfb.status.resist-cold", DamageType::Cold),
        ("rfb.status.resist-fire", DamageType::Fire),
    ] {
        let status = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == status_kind_id)
            .unwrap_or_else(|| panic!("{status_kind_id} should be active"));
        assert!((21..=40).contains(&status.remaining_ticks));
        assert_eq!(
            status.granted_resistances.get(&damage_type),
            Some(&ResistanceLevel::Resistant)
        );
    }
}

#[test]
fn arcane_magic_item_detection_uses_instance_identity_and_enchantment() {
    let mut game = arcane_high_mage_game(0x4445_5445_4354_3139, 11, &[]);
    let position = Position {
        x: game.player.position.x + 2,
        y: game.player.position.y,
    };
    for (id, kind_id) in [
        ("test.magic-potion", "demo.item.antidote-potion"),
        ("test.plain-dagger", "demo.item.dagger"),
        ("test.enchanted-dagger", "demo.item.dagger"),
        ("test.ego-dagger", "demo.item.dagger"),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        game.items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("test item should exist")
            .location = ItemLocation::Ground(position);
    }
    game.items
        .iter_mut()
        .find(|item| item.id == "test.enchanted-dagger")
        .expect("enchanted dagger should exist")
        .enchantments
        .to_hit = 1;
    game.items
        .iter_mut()
        .find(|item| item.id == "test.ego-dagger")
        .expect("ego dagger should exist")
        .affix_ids
        .push("rfb-legacy.affix.slaying".to_owned());

    let (_, ids) = game.detect_item_positions("magic-item", 30, true);
    assert!(ids.contains(&"test.magic-potion".to_owned()));
    assert!(ids.contains(&"test.enchanted-dagger".to_owned()));
    assert!(ids.contains(&"test.ego-dagger".to_owned()));
    assert!(!ids.contains(&"test.plain-dagger".to_owned()));
}

#[test]
fn arcane_door_trap_detection_remembers_stairs_through_walls() {
    let mut game = arcane_high_mage_game(
        0x444f_4f52_5452_4150,
        11,
        &["demo.ability.arcane-detect-doors-traps"],
    );
    let wall = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let stairs = Position {
        x: game.player.position.x + 2,
        y: game.player.position.y,
    };
    let wall_index = game.index(wall).expect("wall position should exist");
    let stairs_index = game.index(stairs).expect("stairs position should exist");
    game.terrain[wall_index] = "demo.terrain.wall".to_owned();
    game.terrain[stairs_index] = "demo.terrain.stairs-down".to_owned();
    game.explored[stairs_index] = false;
    assert!(!game.is_visible(stairs));

    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.arcane-detect-doors-traps",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Detect Doors & Traps should resolve");
    assert!(game.explored[stairs_index], "events: {events:#?}");
}

#[test]
fn arcane_first_book_jams_and_destroys_doors_and_cures_light_wounds() {
    let mut game = arcane_high_mage_game(
        0x4152_4341_4e45_3138,
        5,
        &[
            "demo.ability.arcane-wizard-lock",
            "demo.ability.arcane-trap-door-destruction",
            "demo.ability.arcane-cure-light-wounds",
        ],
    );
    let door = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let door_index = game.index(door).expect("adjacent door cell should exist");
    game.terrain[door_index] = "demo.terrain.door-closed".to_owned();

    for expected in ["demo.terrain.door-jammed-1", "demo.terrain.door-jammed-2"] {
        game.resolve_player_ability(
            "demo.ability.arcane-wizard-lock",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Wizard Lock should resolve");
        assert_eq!(game.terrain[door_index], expected);
    }

    game.resolve_player_ability(
        "demo.ability.arcane-trap-door-destruction",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Trap & Door Destruction should resolve");
    assert_eq!(game.terrain[door_index], "demo.terrain.door-broken");

    game.player.hp = 1;
    game.player.statuses.push(StatusInstance {
        kind_id: "rfb.status.bleeding".to_owned(),
        intensity: 1,
        remaining_ticks: 20,
        source_id: Some("test.wound".to_owned()),
        granted_modifiers: StatModifiersDto::default(),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let experience_before = game.progress.experience;
    game.resolve_player_ability(
        "demo.ability.arcane-cure-light-wounds",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Cure Light Wounds should resolve");
    assert!(game.player.hp > 1);
    assert_eq!(game.player.statuses[0].remaining_ticks, 10);
    assert_eq!(
        game.progress.experience - experience_before,
        33,
        "the original 25-point spell reward uses the High-Mage 130% experience factor"
    );

    game.resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should retain mana")
        .current = 100;
    let experience_before = game.progress.experience;
    game.resolve_player_ability(
        "demo.ability.arcane-cure-light-wounds",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("repeated Cure Light Wounds should resolve");
    assert_eq!(game.progress.experience, experience_before);
}

#[test]
fn astral_guide_reduces_successful_arcane_blink_energy_to_one_third() {
    let mut ordinary = arcane_high_mage_game(0x4153_5452_414c, 5, &["demo.ability.arcane-blink"]);
    let mut guided = ordinary.clone();
    guided
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.astral-guide".to_owned());
    let ordinary_tick = ordinary.world_tick;
    let guided_tick = guided.world_tick;

    dispatch_next(
        &mut ordinary,
        GameCommand::CastAbility {
            ability_id: "demo.ability.arcane-blink".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    dispatch_next(
        &mut guided,
        GameCommand::CastAbility {
            ability_id: "demo.ability.arcane-blink".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );

    assert_eq!(ordinary.world_tick - ordinary_tick, 10);
    assert_eq!(guided.world_tick - guided_tick, 4);
}

#[test]
fn arcane_cure_medium_wounds_uses_spell_powered_healing_and_original_bleeding_formula() {
    let mut game = arcane_high_mage_game(
        0x4355_5245_4d45_4449,
        22,
        &["demo.ability.arcane-cure-medium-wounds"],
    );
    game.player.hp = 1;
    game.player.statuses.push(StatusInstance {
        kind_id: "rfb.status.bleeding".to_owned(),
        intensity: 1,
        remaining_ticks: 300,
        source_id: Some("test.medium-wound".to_owned()),
        granted_modifiers: StatModifiersDto::default(),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });

    game.resolve_player_ability(
        "demo.ability.arcane-cure-medium-wounds",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Cure Medium Wounds should resolve");

    assert!((5..=32).contains(&game.player.hp));
    assert_eq!(game.player.statuses[0].remaining_ticks, 100);
}

#[test]
fn arcane_satisfy_hunger_sets_nutrition_to_original_maximum_minus_one() {
    let mut game = arcane_high_mage_game(
        0x5341_5449_5346_5932,
        22,
        &["demo.ability.arcane-satisfy-hunger"],
    );
    game.nutrition = 1;
    game.resolve_player_ability(
        "demo.ability.arcane-satisfy-hunger",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Satisfy Hunger should resolve");
    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
}

#[test]
fn arcane_identify_performs_basic_identification_without_an_extra_rng_roll() {
    let mut game =
        arcane_high_mage_game(0x4944_454e_5449_4659, 22, &["demo.ability.arcane-identify"]);
    give_inventory_item(&mut game, "test.identify-target", "demo.item.dagger");
    let draws_before = game.rng_draw_counter();
    game.resolve_player_ability(
        "demo.ability.arcane-identify",
        TargetSelection::Item {
            item_id: "test.identify-target".to_owned(),
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Identify should resolve");

    let target = game
        .items
        .iter()
        .find(|item| item.id == "test.identify-target")
        .expect("identify target should remain");
    assert_eq!(
        game.item_identification(target),
        ItemIdentificationDto::Appraised
    );
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
}

#[test]
fn arcane_stone_to_mud_uses_the_rock_power_roll_and_preserves_permanent_walls() {
    let mut game = arcane_high_mage_game(
        0x5354_4f4e_454d_5544,
        22,
        &["demo.ability.arcane-stone-to-mud"],
    );
    let actor_position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let target = Position {
        x: game.player.position.x + 2,
        y: game.player.position.y,
    };
    let actor_index = game
        .index(actor_position)
        .expect("adjacent terrain should exist");
    game.terrain[actor_index] = "demo.terrain.floor".to_owned();
    let mut rock_actor = actor_from_runtime_spawn(
        "test.adobe-golem",
        "demo.actor.adobe-golem",
        actor_position,
        100,
        100,
        100,
        true,
    );
    rock_actor
        .resistances
        .set(DamageType::Disintegrate, ResistanceLevel::Vulnerable);
    game.entities.push(rock_actor);
    let target_index = game.index(target).expect("adjacent terrain should exist");
    game.terrain[target_index] = "demo.terrain.quartz-vein".to_owned();
    game.resolve_player_ability(
        "demo.ability.arcane-stone-to-mud",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Stone to Mud should resolve against ordinary rock");
    assert_eq!(game.terrain[target_index], "demo.terrain.floor");
    assert!((50..=79).contains(&game.entities[0].hp));

    game.terrain[target_index] = "demo.terrain.permanent-wall".to_owned();
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should retain mana")
        .current = 100;
    game.resolve_player_ability(
        "demo.ability.arcane-stone-to-mud",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Stone to Mud should resolve against permanent rock");
    assert_eq!(game.terrain[target_index], "demo.terrain.permanent-wall");
}

#[test]
fn astral_guide_reduces_successful_arcane_long_teleport_energy_to_one_third() {
    let mut ordinary =
        arcane_high_mage_game(0x4153_5452_414c_3230, 22, &["demo.ability.arcane-teleport"]);
    let mut guided = ordinary.clone();
    guided
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.astral-guide".to_owned());
    let ordinary_tick = ordinary.world_tick;
    let guided_tick = guided.world_tick;

    dispatch_next(
        &mut ordinary,
        GameCommand::CastAbility {
            ability_id: "demo.ability.arcane-teleport".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    dispatch_next(
        &mut guided,
        GameCommand::CastAbility {
            ability_id: "demo.ability.arcane-teleport".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );

    assert_eq!(ordinary.world_tick - ordinary_tick, 10);
    assert_eq!(guided.world_tick - guided_tick, 4);
}

#[test]
fn arcane_fourth_book_statuses_keep_see_invisible_separate_from_sight() {
    let mut game = arcane_high_mage_game(
        0x4152_4341_4e45_3231,
        30,
        &[
            "demo.ability.arcane-see-invisible",
            "demo.ability.arcane-resist-poison",
        ],
    );
    assert_eq!(game.player_see_invisible_sources(), 0);
    assert_eq!(game.player_infravision_range(), 0);

    for ability_id in [
        "demo.ability.arcane-see-invisible",
        "demo.ability.arcane-resist-poison",
    ] {
        game.resources
            .get_mut("demo.resource.mana")
            .expect("Arcane High-Mage should retain mana")
            .current = 100;
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("fourth-book status spell should resolve");
    }

    assert!(game.player_has_status_kind(STATUS_SEE_INVISIBLE));
    assert!(!game.player_has_status_kind(STATUS_SIGHT));
    assert_eq!(game.player_see_invisible_sources(), 1);
    assert_eq!(game.player_infravision_range(), 0);
    assert!(game.player_has_status_kind("rfb.status.resist-poison"));
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );
}

#[test]
fn arcane_teleport_away_beams_through_monsters_and_honors_original_resistance() {
    let mut game = arcane_high_mage_game(
        0x5445_4c45_4157_4159,
        50,
        &["demo.ability.arcane-teleport-away"],
    );
    clear_monsters(&mut game);
    let origin = game.player.position;
    for step in 1..=8 {
        replace_terrain(
            &mut game,
            Position {
                x: origin.x + step,
                y: origin.y,
            },
            "demo.terrain.floor",
        );
    }
    let ordinary_from = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    let unique_from = Position {
        x: origin.x + 2,
        y: origin.y,
    };
    game.entities.push(actor_from_runtime_spawn(
        "test.teleport-away.ordinary",
        "demo.actor.small-kobold",
        ordinary_from,
        5,
        100,
        100,
        true,
    ));
    game.entities.push(actor_from_runtime_spawn(
        "test.teleport-away.unique",
        "demo.actor.alberich-the-nibelung-king",
        unique_from,
        40,
        100,
        100,
        true,
    ));

    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    game.resolve_player_ability(
        "demo.ability.arcane-teleport-away",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .expect("Teleport Away should resolve");

    let ordinary_after = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.teleport-away.ordinary")
        .expect("ordinary target should remain")
        .position;
    let unique_after = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.teleport-away.unique")
        .expect("unique target should remain")
        .position;
    assert_ne!(ordinary_after, ordinary_from);
    assert_eq!(unique_after, unique_from);
    assert!(changed.contains(&ordinary_from));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if resolution.effects.iter().any(|effect| matches!(
                effect,
                AbilityEffectResolutionDto::TeleportAway {
                    target_entity_id,
                    resisted: true,
                    ..
                } if target_entity_id == "test.teleport-away.unique"
            )) && resolution.effects.iter().any(|effect| matches!(
                effect,
                AbilityEffectResolutionDto::TeleportAway {
                    target_entity_id,
                    resisted: false,
                    to: Some(_),
                    ..
                } if target_entity_id == "test.teleport-away.ordinary"
            ))
    )));
}

#[test]
fn arcane_recharging_is_atomic_and_keeps_player_failure_separate_from_device_explosion() {
    let mut base = arcane_high_mage_game(
        0x5245_4348_4152_4745,
        40,
        &["demo.ability.arcane-recharging"],
    );
    give_inventory_item(
        &mut base,
        "test.recharge-target",
        "demo.item.detect-objects-staff",
    );
    let target = base
        .items
        .iter_mut()
        .find(|item| item.id == "test.recharge-target")
        .expect("recharge target should exist");
    target
        .activation
        .as_mut()
        .expect("staff should have an activation")
        .device_check_difficulty = 120;
    target.charges = Some(ItemChargesDto {
        current: 10,
        maximum: 100,
    });
    let mut recharge_ability = base
        .content
        .ability("demo.ability.arcane-recharging")
        .expect("Recharging should exist")
        .clone();
    Game::apply_player_level_scaling(&mut recharge_ability, 40);
    Game::apply_player_spell_power(
        &mut recharge_ability,
        base.effective_player_spell_power_bonus(),
    );
    assert!(matches!(
        recharge_ability.effect,
        AbilityEffectDefinition::RechargeFromPlayer { power: 60 }
    ));

    let mut cancelled = base.clone();
    let cancelled_rng = cancelled.rng.clone();
    let cancelled_mana = cancelled.resources["demo.resource.mana"].current;
    cancelled
        .resolve_player_ability(
            "demo.ability.arcane-recharging",
            TargetSelection::Item {
                item_id: "missing-item".to_owned(),
            },
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("cancelled recharge target should be rejected");
    assert_eq!(cancelled.rng, cancelled_rng);
    assert_eq!(
        cancelled.resources["demo.resource.mana"].current,
        cancelled_mana
    );

    let failed_cast = (0..128_u64)
        .find_map(|seed| {
            let mut game = base.clone();
            game.debug_ability_casts_succeed = false;
            game.rng = RfbRng::seeded(seed);
            let mut expected_rng = game.rng.clone();
            let _ = expected_rng.bounded(100);
            let mut events = Vec::new();
            game.resolve_player_ability(
                "demo.ability.arcane-recharging",
                TargetSelection::Item {
                    item_id: "test.recharge-target".to_owned(),
                },
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("failed Recharging cast should resolve atomically");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastFailed { .. }))
                .then_some((game, events, expected_rng))
        })
        .expect("a bounded seed should fail the Recharging cast");
    assert_eq!(failed_cast.0.rng, failed_cast.2);
    assert_eq!(failed_cast.0.resources["demo.resource.mana"].current, 55);
    assert_eq!(
        failed_cast
            .0
            .items
            .iter()
            .find(|item| item.id == "test.recharge-target")
            .and_then(|item| item.charges)
            .expect("failed-cast target should retain charges")
            .current,
        10
    );
    assert!(
        !failed_cast
            .1
            .iter()
            .any(|event| matches!(event, DomainEvent::DeviceRechargeResolved { .. }))
    );

    let mut success = base.clone();
    success.debug_recharge_attempts_succeed = true;
    let mut success_rng = success.rng.clone();
    let _ = success_rng.bounded(100);
    let mut success_events = Vec::new();
    success
        .resolve_player_ability(
            "demo.ability.arcane-recharging",
            TargetSelection::Item {
                item_id: "test.recharge-target".to_owned(),
            },
            &mut success_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("successful player recharge should resolve");
    assert_eq!(success.resources["demo.resource.mana"].current, 0);
    assert_eq!(
        success
            .items
            .iter()
            .find(|item| item.id == "test.recharge-target")
            .and_then(|item| item.charges)
            .expect("target should retain charges")
            .current,
        65
    );
    assert_eq!(success.rng, success_rng);

    let mut failure = base;
    failure.debug_recharge_attempts_fail = true;
    failure
        .items
        .iter_mut()
        .find(|item| item.id == "test.recharge-target")
        .expect("failure target should exist")
        .location = ItemLocation::Ground(failure.player.position);
    let mut failure_rng = failure.rng.clone();
    let _ = failure_rng.bounded(100);
    let mut failure_events = Vec::new();
    failure
        .resolve_player_ability(
            "demo.ability.arcane-recharging",
            TargetSelection::Item {
                item_id: "test.recharge-target".to_owned(),
            },
            &mut failure_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed player recharge should resolve");
    assert_eq!(failure.resources["demo.resource.mana"].current, 0);
    assert_eq!(failure.rng, failure_rng);
    assert_eq!(
        failure
            .items
            .iter()
            .find(|item| item.id == "test.recharge-target")
            .and_then(|item| item.charges)
            .expect("failed target should retain charge state")
            .current,
        0
    );
    assert!(failure_events.iter().any(|event| matches!(
        event,
        DomainEvent::DeviceRechargeResolved {
            source_is_item: false,
            succeeded: false,
            failure_roll: None,
            source_destroyed: false,
            ..
        }
    )));
    assert!(
        failure
            .items
            .iter()
            .any(|item| item.id == "test.recharge-target")
    );
}

#[test]
fn arcane_detection_recall_and_level_teleport_reuse_existing_transactions() {
    let mut game = arcane_high_mage_game(
        0x4445_5445_4354_3231,
        50,
        &[
            "demo.ability.arcane-detection",
            "demo.ability.arcane-word-of-recall",
            "demo.ability.arcane-teleport-level",
        ],
    );
    let detection = game
        .content
        .ability("demo.ability.arcane-detection")
        .expect("Detection should exist");
    assert_eq!(detection.effect.ordered_effects().len(), 8);
    let mut detection_events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.arcane-detection",
        TargetSelection::SelfTarget,
        &mut detection_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Detection should resolve");
    assert_eq!(
        detection_events
            .iter()
            .filter(|event| matches!(event, DomainEvent::AbilityDetected { resolution, .. } if resolution.radius == 30))
            .count(),
        8
    );

    descend_one_floor(&mut game);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should retain mana")
        .current = 100;
    game.debug_recall_delay_turns = Some(27);
    game.recall = Some(RecallStateDto {
        dungeon_id: "demo.dungeon.warrens".to_owned(),
        floor_id: "demo.floor.warrens-depth-1".to_owned(),
        remaining_turns: None,
    });
    game.resolve_player_ability(
        "demo.ability.arcane-word-of-recall",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Word of Recall should resolve");
    assert_eq!(
        game.recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns),
        Some(28)
    );

    game.resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should retain mana")
        .current = 100;
    let from_floor = game.current_floor_id.clone();
    let (upward, downward) = game.teleport_level_targets();
    if !upward.is_empty() || !downward.is_empty() {
        game.resolve_player_ability(
            "demo.ability.arcane-teleport-level",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Teleport Level should resolve");
        assert_ne!(game.current_floor_id, from_floor);
    }
}

#[test]
fn death_high_mage_birth_uses_the_original_class_identity_and_kit() {
    let game = high_mage_game(0x4849_4748_4d41_4745);
    let snapshot = game.snapshot();
    let build = snapshot
        .player
        .build
        .expect("High-Mage should project its build");

    assert_eq!(build.build_id, HIGH_MAGE_BUILD_ID);
    assert_eq!(build.class_id, "demo.class.high-mage");
    assert_eq!(build.life_percent, 94);
    assert_eq!(build.experience_percent, 130);
    assert_eq!(snapshot.player.kind_id, "demo.actor.high-mage-player");
    assert_eq!(
        snapshot.player.progress.attributes.intelligence.effective, 17,
        "base 13 Intelligence should receive the original +4 class modifier"
    );

    for kind_id in [
        "demo.item.dagger",
        "demo.item.robe",
        "demo.item.magic-missile-wand",
        "demo.item.black-prayers",
    ] {
        assert!(
            game.items.iter().any(|item| item.kind_id == kind_id),
            "birth kit should contain {kind_id}"
        );
    }
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.dagger" && matches!(item.location, ItemLocation::Equipped { .. })
    }));
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.robe" && matches!(item.location, ItemLocation::Equipped { .. })
    }));
    let clarity = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.clarity-draught")
        .expect("High-Mage should start with Clarity draughts");
    assert!((10..=20).contains(&clarity.quantity));
}

#[test]
fn death_high_mage_projects_original_mana_and_spell_table() {
    let game = high_mage_game(7);
    let snapshot = game.snapshot();
    let mana = snapshot
        .player
        .resources
        .iter()
        .find(|resource| resource.id == "demo.resource.mana")
        .expect("High-Mage should have Mana");
    assert_eq!((mana.current, mana.maximum), (11, 11));
    assert_eq!(
        (mana.wait_recovery_amount, mana.rest_recovery_amount),
        (2, 6)
    );
    assert_eq!(
        snapshot.player.ability_learning,
        Some(rfb_protocol::AbilityLearningDto {
            learned_count: 0,
            capacity: 1,
            remaining_slots: 1,
            study_mode: rfb_protocol::AbilityStudyModeDto::Chosen,
        })
    );

    let learned = snapshot
        .player
        .abilities
        .iter()
        .filter(|ability| ability.source == AbilitySourceDto::Learned)
        .collect::<Vec<_>>();
    assert_eq!(learned.len(), 32);
    assert!(
        learned
            .iter()
            .all(|ability| ability.book_name_key.is_some())
    );
    let detect_unlife = learned
        .iter()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("first Death spell should be projected");
    assert_eq!(
        detect_unlife.book_name_key.as_deref(),
        Some("ability-book-demo-black-prayers-name")
    );
    assert_eq!(detect_unlife.book_rank, Some(1));
    assert_eq!(detect_unlife.minimum_level, 1);
    assert_eq!(detect_unlife.base_resource_cost, 1);
    assert_eq!(
        detect_unlife.resource_cost, 2,
        "unskilled spells retain the RFB surcharge"
    );
    assert_eq!(detect_unlife.failure_percent, 17);
    assert!(detect_unlife.can_study);

    let wraithform = learned
        .iter()
        .find(|ability| ability.id == "demo.ability.death-wraithform")
        .expect("last Death spell should be projected");
    assert_eq!(
        wraithform.book_name_key.as_deref(),
        Some("ability-book-demo-necronomicon-name")
    );
    assert_eq!(wraithform.book_rank, Some(4));
    assert_eq!(
        (
            wraithform.minimum_level,
            wraithform.base_resource_cost,
            wraithform.failure_percent
        ),
        (45, 75, 95)
    );

    let eat_magic = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.high-mage-eat-magic")
        .expect("High-Mage class power should be projected");
    assert_eq!(eat_magic.source, AbilitySourceDto::Class);
    assert_eq!(eat_magic.minimum_level, 25);
    assert_eq!(eat_magic.resource_cost, 1);
    assert!(!eat_magic.can_cast);
    assert_eq!(eat_magic.book_name_key, None);
}

#[test]
fn death_high_mage_damage_bonus_and_level_twenty_five_power_are_active() {
    let mut game = high_mage_game(11);
    game.progress.level = 25;
    game.progress.max_level = 25;
    game.refresh_player_resource_maxima();
    let snapshot = game.snapshot();

    let malediction = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("Malediction should be projected");
    let damage_bonus = malediction
        .effects
        .iter()
        .find_map(|effect| match effect {
            AbilityEffectSpecDto::Damage { damage_bonus, .. } => Some(*damage_bonus),
            _ => None,
        })
        .expect("Malediction should contain damage");
    assert_eq!(
        damage_bonus, 10,
        "High-Mage gains +5 + level/5 spell damage"
    );

    let eat_magic = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.high-mage-eat-magic")
        .expect("High-Mage class power should remain projected");
    assert!(eat_magic.can_cast);
    assert_eq!(eat_magic.target_spec.modes, vec![TargetModeDto::Item]);

    give_inventory_item(
        &mut game,
        "test.item.high-mage-magic-food",
        "demo.item.detect-objects-staff",
    );
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.high-mage-magic-food")
        .expect("test device should exist");
    item.activation
        .as_mut()
        .expect("staff should have an activation")
        .device_check_difficulty = 120;
    item.charges
        .as_mut()
        .expect("staff should have charges")
        .current = 20;
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have Mana");
    mana.maximum = 100;
    mana.current = 10;
    game.debug_ability_casts_succeed = true;
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.high-mage-eat-magic",
        TargetSelection::Item {
            item_id: "test.item.high-mage-magic-food".to_owned(),
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("High-Mage Eat Magic should resolve");
    assert_eq!(game.resources["demo.resource.mana"].current, 29);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.high-mage-magic-food")
            .expect("test device should remain")
            .charges
            .expect("staff should retain charge state")
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
}

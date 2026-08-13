// SPDX-License-Identifier: MPL-2.0

use super::support::{
    clear_monsters, descend_one_floor, dispatch_next, give_inventory_item, replace_terrain,
};
use super::*;

const HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-death";
const ARCANE_HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-arcane";
const SORCERY_HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-sorcery";

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

fn sorcery_high_mage_game(seed: u64, level: u16, ability_ids: &[&str]) -> Game {
    let mut game = Game::new_with_build(seed, SORCERY_HIGH_MAGE_BUILD_ID)
        .expect("Sorcery High-Mage build should create");
    game.progress.level = level;
    game.progress.max_level = level;
    game.learned_abilities
        .extend(ability_ids.iter().map(|id| (*id).to_owned()));
    give_inventory_item(
        &mut game,
        "test.master-sorcerers-handbook",
        "demo.item.master-sorcerers-handbook",
    );
    give_inventory_item(
        &mut game,
        "test.pattern-sorcery",
        "demo.item.pattern-sorcery",
    );
    give_inventory_item(
        &mut game,
        "test.grimoire-of-power",
        "demo.item.grimoire-of-power",
    );
    game.refresh_player_resource_maxima();
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Sorcery High-Mage should have mana")
        .current = 100;
    game.debug_ability_casts_succeed = true;
    game
}

fn grant_spell_power(game: &mut Game, bonus: i32) {
    game.player.statuses.push(StatusInstance {
        kind_id: "test.status.spell-power".to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.sorcery-spell-power".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto {
            spell_power_bonus: bonus,
            ..StatModifiersDto::default()
        },
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
}

#[test]
fn sorcery_high_mage_birth_keeps_only_the_first_book_and_realm() {
    let game = Game::new_with_build(0x534f_5243_4552_5932, SORCERY_HIGH_MAGE_BUILD_ID)
        .expect("Sorcery High-Mage build should create");
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
    assert!(carried.contains("demo.item.beginners-handbook"));
    assert!(!carried.contains("demo.item.master-sorcerers-handbook"));
    assert!(!carried.contains("demo.item.pattern-sorcery"));
    assert!(!carried.contains("demo.item.grimoire-of-power"));
    assert!(!carried.contains("demo.item.cantrips-for-beginners"));
    assert!(!carried.contains("demo.item.black-prayers"));

    let learned = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .filter(|ability| ability.source == AbilitySourceDto::Learned)
        .collect::<Vec<_>>();
    assert_eq!(learned.len(), 32);
    assert!(
        learned
            .iter()
            .all(|ability| ability.id.starts_with("demo.ability.sorcery-"))
    );
}

#[test]
fn sorcery_identify_and_mass_sleep_switch_at_the_original_levels() {
    let projected = |level| {
        sorcery_high_mage_game(
            0x534f_5243_4552_5900 + u64::from(level),
            level,
            &[
                "demo.ability.sorcery-identify",
                "demo.ability.sorcery-mass-sleep",
            ],
        )
        .snapshot()
        .player
        .abilities
    };
    let level_29 = projected(29);
    let identify_29 = level_29
        .iter()
        .find(|ability| ability.id == "demo.ability.sorcery-identify")
        .expect("Identify should be projected");
    assert_eq!(identify_29.name_key, "ability-demo-sorcery-identify-name");
    assert!(matches!(
        identify_29.effects.as_slice(),
        [AbilityEffectSpecDto::IdentifyItem { .. }]
    ));
    let sleep_29 = level_29
        .iter()
        .find(|ability| ability.id == "demo.ability.sorcery-mass-sleep")
        .expect("Mass Sleep should be projected");
    assert_eq!(sleep_29.name_key, "ability-demo-sorcery-mass-sleep-name");
    assert!(matches!(
        sleep_29.effects.as_slice(),
        [AbilityEffectSpecDto::VisibleApplyStatus {
            status_kind_id,
            power: Some(96),
            ..
        }] if status_kind_id == STATUS_SLEEP
    ));

    let level_30 = projected(30);
    let identify_30 = level_30
        .iter()
        .find(|ability| ability.id == "demo.ability.sorcery-identify")
        .expect("Mass Identify should be projected");
    assert_eq!(
        identify_30.name_key,
        "ability-demo-sorcery-mass-identify-name"
    );
    assert!(matches!(
        identify_30.effects.as_slice(),
        [AbilityEffectSpecDto::MassIdentify]
    ));

    let level_35 = projected(35);
    let stasis_35 = level_35
        .iter()
        .find(|ability| ability.id == "demo.ability.sorcery-mass-sleep")
        .expect("Mass Stasis should be projected");
    assert_eq!(stasis_35.name_key, "ability-demo-sorcery-mass-stasis-name");
    assert!(matches!(
        stasis_35.effects.as_slice(),
        [AbilityEffectSpecDto::VisibleApplyStatus {
            status_kind_id,
            duration_ticks: 20,
            power: Some(81),
            ..
        }] if status_kind_id == STATUS_PARALYSIS
    ));
}

#[test]
fn sorcery_mass_identify_appraises_all_carried_items() {
    let mut game = sorcery_high_mage_game(
        0x4d41_5353_4944_454e,
        30,
        &["demo.ability.sorcery-identify"],
    );
    for (id, kind_id) in [
        ("test.mass-identify.dagger", "demo.item.dagger"),
        ("test.mass-identify.potion", "demo.item.antidote-potion"),
    ] {
        give_inventory_item(&mut game, id, kind_id);
    }

    game.resolve_player_ability(
        "demo.ability.sorcery-identify",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Mass Identify should resolve without an item target");

    for id in ["test.mass-identify.dagger", "test.mass-identify.potion"] {
        let knowledge = &game.item_property_knowledge[id];
        assert!(knowledge.discovered && knowledge.appraised);
        assert!(!knowledge.identified);
    }
}

#[test]
fn sorcery_mass_stasis_suspends_visible_non_unique_monsters_only() {
    let mut game = sorcery_high_mage_game(
        0x5354_4153_4953_3335,
        35,
        &["demo.ability.sorcery-mass-sleep"],
    );
    clear_monsters(&mut game);
    let origin = game.player.position;
    let ordinary_position = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    let unique_position = Position {
        x: origin.x + 2,
        y: origin.y,
    };
    replace_terrain(&mut game, ordinary_position, "demo.terrain.floor");
    replace_terrain(&mut game, unique_position, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.stasis.ordinary",
        "demo.actor.small-kobold",
        ordinary_position,
        5,
        100,
        100,
        true,
    ));
    game.entities.push(actor_from_runtime_spawn(
        "test.stasis.unique",
        "demo.actor.alberich-the-nibelung-king",
        unique_position,
        40,
        100,
        100,
        true,
    ));

    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.sorcery-mass-sleep",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Mass Stasis should resolve");

    let ordinary = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.stasis.ordinary")
        .expect("ordinary target should remain");
    assert!(ordinary.statuses.iter().any(|status| {
        status.kind_id == STATUS_PARALYSIS && (20..=30).contains(&status.remaining_ticks)
    }));
    let unique = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.stasis.unique")
        .expect("unique target should remain");
    assert!(
        unique
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_PARALYSIS)
    );
    assert!(events.iter().all(|event| !matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if resolution.target_entity_id.as_deref() == Some("test.stasis.unique")
    )));
}

#[test]
fn sorcery_third_book_statuses_use_the_original_spell_powered_durations() {
    let mut game = sorcery_high_mage_game(
        0x534f_5243_4552_5933,
        12,
        &[
            "demo.ability.sorcery-inventory-protection",
            "demo.ability.sorcery-esp",
        ],
    );
    for ability_id in [
        "demo.ability.sorcery-inventory-protection",
        "demo.ability.sorcery-esp",
    ] {
        game.resources
            .get_mut("demo.resource.mana")
            .expect("Sorcery High-Mage should retain mana")
            .current = 100;
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("third-book status spell should resolve");
    }
    let protection = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_INVENTORY_PROTECTION)
        .expect("Inventory Protection should apply");
    assert!((31..=60).contains(&protection.remaining_ticks));
    let telepathy = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_TELEPATHY)
        .expect("ESP should apply telepathy");
    assert!((26..=55).contains(&telepathy.remaining_ticks));
}

#[test]
fn sorcery_self_knowledge_reuses_the_read_only_character_report() {
    let mut game = sorcery_high_mage_game(
        0x5345_4c46_4b4e_4f57,
        15,
        &["demo.ability.sorcery-self-knowledge"],
    );
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.sorcery-self-knowledge",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Self Knowledge should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilitySelfKnowledge { ability_id, report, .. }
            if ability_id == "demo.ability.sorcery-self-knowledge"
                && report.level == 15
                && report.max_hp == game.effective_player_max_hp()
    )));
}

#[test]
fn sorcery_teleport_town_lists_only_visited_destinations_and_moves_without_a_fare() {
    let mut game = sorcery_high_mage_game(
        0x5445_4c45_544f_574e,
        15,
        &["demo.ability.sorcery-teleport-town"],
    );
    game.town_states
        .insert("demo.town.anambar".to_owned(), TownState { visited: true });
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.sorcery-teleport-town")
        .expect("Teleport Town should be projected");
    assert_eq!(ability.target_spec.modes, [TargetModeDto::Town]);
    assert_eq!(
        ability
            .town_targets
            .iter()
            .map(|target| target.town_id.as_str())
            .collect::<Vec<_>>(),
        ["demo.town.anambar"]
    );
    let gold = game.gold;
    game.resolve_player_ability(
        "demo.ability.sorcery-teleport-town",
        TargetSelection::Town {
            town_id: "demo.town.anambar".to_owned(),
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Teleport Town should resolve");
    assert_eq!(
        game.current_town().map(|town| town.id.as_str()),
        Some("demo.town.anambar")
    );
    assert_eq!(game.gold, gold);
}

#[test]
fn sorcery_dimension_door_cancellation_is_atomic_and_failed_steps_cost_extra_energy() {
    let mut cancelled = sorcery_high_mage_game(
        0x4449_4d45_4e53_494f,
        36,
        &["demo.ability.sorcery-dimension-door"],
    );
    let mana = cancelled.resources["demo.resource.mana"].current;
    let draws = cancelled.rng_draw_counter();
    cancelled
        .resolve_player_ability(
            "demo.ability.sorcery-dimension-door",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("a cancelled Dimension Door target should be rejected cleanly");
    assert_eq!(cancelled.resources["demo.resource.mana"].current, mana);
    assert_eq!(cancelled.rng_draw_counter(), draws);

    let mut ordinary = sorcery_high_mage_game(
        0x4449_4d45_4e53_4941,
        36,
        &["demo.ability.sorcery-dimension-door"],
    );
    clear_monsters(&mut ordinary);
    let invalid = Position {
        x: ordinary.player.position.x + 1,
        y: ordinary.player.position.y,
    };
    replace_terrain(&mut ordinary, invalid, "demo.terrain.permanent-wall");
    let mut guided = ordinary.clone();
    guided
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.astral-guide".to_owned());
    let ordinary_tick = ordinary.world_tick;
    let guided_tick = guided.world_tick;
    for game in [&mut ordinary, &mut guided] {
        dispatch_next(
            game,
            GameCommand::CastAbility {
                ability_id: "demo.ability.sorcery-dimension-door".to_owned(),
                target: TargetSelection::Position { position: invalid },
            },
        );
    }
    assert_eq!(ordinary.world_tick - ordinary_tick, 15);
    assert_eq!(guided.world_tick - guided_tick, 10);
    assert_ne!(ordinary.player.position, invalid);
    assert_ne!(guided.player.position, invalid);
}

#[test]
fn sorcery_dimension_door_success_uses_one_failure_roll_and_one_extra_energy_charge() {
    let mut ordinary = (0_u64..100)
        .find_map(|offset| {
            let game = sorcery_high_mage_game(
                0x4449_4d45_4e53_5000 + offset,
                36,
                &["demo.ability.sorcery-dimension-door"],
            );
            let mut probe = game.rng.clone();
            let _ability_roll = probe.bounded(100);
            (probe.bounded(13) != 0).then_some(game)
        })
        .expect("a successful Dimension Door seed should be available");
    clear_monsters(&mut ordinary);
    let target = Position {
        x: ordinary.player.position.x + 1,
        y: ordinary.player.position.y,
    };
    replace_terrain(&mut ordinary, target, "demo.terrain.floor");
    let mut guided = ordinary.clone();
    guided
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.astral-guide".to_owned());
    let ordinary_tick = ordinary.world_tick;
    let guided_tick = guided.world_tick;
    for game in [&mut ordinary, &mut guided] {
        dispatch_next(
            game,
            GameCommand::CastAbility {
                ability_id: "demo.ability.sorcery-dimension-door".to_owned(),
                target: TargetSelection::Position { position: target },
            },
        );
        assert_eq!(game.player.position, target);
    }
    assert_eq!(ordinary.world_tick - ordinary_tick, 13);
    assert_eq!(guided.world_tick - guided_tick, 10);
}

#[test]
fn sorcery_create_stair_respects_surface_and_permanent_terrain() {
    let mut surface = sorcery_high_mage_game(
        0x5354_4149_5253_5552,
        8,
        &["demo.ability.sorcery-create-stair"],
    );
    let before = surface.terrain[surface
        .index(surface.player.position)
        .expect("player should stand in bounds")]
    .clone();
    surface
        .resolve_player_ability(
            "demo.ability.sorcery-create-stair",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("surface Create Stair should resolve without changing terrain");
    assert_eq!(
        surface.terrain[surface
            .index(surface.player.position)
            .expect("player should remain in bounds")],
        before
    );

    let mut dungeon = sorcery_high_mage_game(
        0x5354_4149_5244_554e,
        8,
        &["demo.ability.sorcery-create-stair"],
    );
    descend_one_floor(&mut dungeon);
    let position = dungeon.player.position;
    replace_terrain(&mut dungeon, position, "demo.terrain.floor");

    let mut task = dungeon.clone();
    task.current_floor_id = "demo.floor.trouble-at-home".to_owned();
    let task_draws = task.rng_draw_counter();
    task.resolve_player_ability(
        "demo.ability.sorcery-create-stair",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("task-floor Create Stair should resolve without changing terrain");
    assert_eq!(
        task.terrain[task
            .index(position)
            .expect("player should remain in bounds")],
        "demo.terrain.floor"
    );
    assert_eq!(task.rng_draw_counter(), task_draws + 1);

    let mut permanent = dungeon.clone();
    replace_terrain(&mut permanent, position, "demo.terrain.permanent-wall");
    permanent
        .resolve_player_ability(
            "demo.ability.sorcery-create-stair",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("permanent-terrain Create Stair should resolve without changing terrain");
    assert_eq!(
        permanent.terrain[permanent
            .index(position)
            .expect("player should remain in bounds")],
        "demo.terrain.permanent-wall"
    );

    dungeon
        .resolve_player_ability(
            "demo.ability.sorcery-create-stair",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("dungeon Create Stair should resolve");
    assert!(matches!(
        dungeon.terrain[dungeon
            .index(position)
            .expect("player should remain in bounds")]
        .as_str(),
        "demo.terrain.stairs-up" | "demo.terrain.stairs-down"
    ));
}

#[test]
fn sorcery_fourth_book_projection_uses_level_and_spell_power() {
    let ability_ids = [
        "demo.ability.sorcery-fetch",
        "demo.ability.sorcery-clairvoyance",
        "demo.ability.sorcery-device-mastery",
        "demo.ability.sorcery-banish",
        "demo.ability.sorcery-invulnerability",
    ];
    let projected = |bonus| {
        let mut game = sorcery_high_mage_game(0x504f_5745_525f_3530, 50, &ability_ids);
        grant_spell_power(&mut game, bonus);
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>()
    };

    let positive = projected(7);
    assert!(matches!(
        positive["demo.ability.sorcery-fetch"].effects.as_slice(),
        [AbilityEffectSpecDto::FetchItem {
            maximum_weight_tenths_pound: 1_153
        }]
    ));
    assert!(matches!(
        positive["demo.ability.sorcery-clairvoyance"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Clairvoyance {
            telepathy_duration_ticks: 25,
            telepathy_duration_dice: 1,
            telepathy_duration_sides: 46,
        }]
    ));
    assert!(matches!(
        positive["demo.ability.sorcery-device-mastery"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::DeviceMastery {
            duration_base: 7,
            device_power_bonus: 5,
        }]
    ));
    assert!(matches!(
        positive["demo.ability.sorcery-banish"].effects.as_slice(),
        [AbilityEffectSpecDto::Banish {
            maximum_distance: 307
        }]
    ));
    assert!(matches!(
        positive["demo.ability.sorcery-invulnerability"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Invulnerability {
            duration_dice: 1,
            duration_sides: 4,
            duration_bonus: 4,
            duration_spell_power_bonus: Some(7),
        }]
    ));

    let negative = projected(-20);
    assert!(matches!(
        negative["demo.ability.sorcery-fetch"].effects.as_slice(),
        [AbilityEffectSpecDto::FetchItem {
            maximum_weight_tenths_pound: 0
        }]
    ));
    assert!(matches!(
        negative["demo.ability.sorcery-clairvoyance"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Clairvoyance {
            telepathy_duration_sides: 0,
            ..
        }]
    ));
    assert!(matches!(
        negative["demo.ability.sorcery-banish"].effects.as_slice(),
        [AbilityEffectSpecDto::Banish {
            maximum_distance: 0
        }]
    ));

    let mut suppressed = sorcery_high_mage_game(
        0x504f_5745_525f_4e45,
        50,
        &[
            "demo.ability.sorcery-clairvoyance",
            "demo.ability.sorcery-device-mastery",
        ],
    );
    grant_spell_power(&mut suppressed, -20);
    suppressed
        .resources
        .get_mut("demo.resource.mana")
        .expect("Sorcery High-Mage should retain mana")
        .current = 1_000;
    suppressed
        .resolve_player_ability(
            "demo.ability.sorcery-clairvoyance",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("negative spell power Clairvoyance should resolve");
    assert_eq!(
        suppressed
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_TELEPATHY)
            .expect("Clairvoyance should retain its fixed duration")
            .remaining_ticks,
        25
    );
    suppressed
        .resolve_player_ability(
            "demo.ability.sorcery-device-mastery",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("zero-duration Device Mastery should resolve");
    assert!(!suppressed.player_has_status_kind(STATUS_DEVICE_MASTERY));
}

#[test]
fn sorcery_probe_reveals_true_identity_and_create_door_uses_only_empty_floor() {
    let mut probe =
        sorcery_high_mage_game(0x5052_4f42_455f_3038, 8, &["demo.ability.sorcery-probe"]);
    clear_monsters(&mut probe);
    let target = Position {
        x: probe.player.position.x + 1,
        y: probe.player.position.y,
    };
    replace_terrain(&mut probe, target, "demo.terrain.floor");
    let mut actor = actor_from_runtime_spawn(
        "test.sorcery.probe",
        "demo.actor.small-kobold",
        target,
        8,
        110,
        100,
        true,
    );
    actor.hp = 5;
    actor.appearance_kind_id = Some("demo.actor.large-kobold".to_owned());
    probe.entities.push(actor);
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    probe
        .resolve_player_ability(
            "demo.ability.sorcery-probe",
            TargetSelection::SelfTarget,
            &mut events,
            &mut changed,
            &mut Vec::new(),
        )
        .expect("Probe should resolve");
    assert_eq!(probe.entities[0].appearance_kind_id, None);
    assert!(changed.contains(&target));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityProbed { report, .. }
            if report.entity_id == "test.sorcery.probe"
                && report.target_kind_id == "demo.actor.small-kobold"
                && report.hp == 5
                && report.max_hp == 8
                && report.speed == 110
                && report.alignment == AbilityProbeAlignmentDto::Evil
                && report.faction == EntityFactionDto::Hostile
    )));

    let mut doors = sorcery_high_mage_game(
        0x444f_4f52_535f_3138,
        18,
        &["demo.ability.sorcery-create-door"],
    );
    clear_monsters(&mut doors);
    let origin = doors.player.position;
    let surroundings = TERRAIN_INTERACTION_DIRECTIONS
        .into_iter()
        .map(|direction| doors.position_in_direction(direction))
        .collect::<Vec<_>>();
    for position in surroundings {
        replace_terrain(&mut doors, position, "demo.terrain.floor");
    }
    let occupied = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    let permanent = Position {
        x: origin.x - 1,
        y: origin.y,
    };
    give_inventory_item(&mut doors, "test.door.blocker", "demo.item.dagger");
    doors
        .items
        .iter_mut()
        .find(|item| item.id == "test.door.blocker")
        .expect("door blocker should exist")
        .location = ItemLocation::Ground(occupied);
    replace_terrain(&mut doors, permanent, "demo.terrain.permanent-wall");
    doors
        .resolve_player_ability(
            "demo.ability.sorcery-create-door",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Create Door should resolve");
    assert_eq!(
        doors.terrain[doors.index(occupied).unwrap()],
        "demo.terrain.floor"
    );
    assert_eq!(
        doors.terrain[doors.index(permanent).unwrap()],
        "demo.terrain.permanent-wall"
    );
    let created = Position {
        x: origin.x,
        y: origin.y - 1,
    };
    assert_eq!(
        doors.terrain[doors.index(created).unwrap()],
        "demo.terrain.door-closed"
    );
}

#[test]
fn sorcery_device_mastery_banish_and_invulnerability_commit_shared_rules() {
    let mut mastery = sorcery_high_mage_game(
        0x4445_5649_4345_3530,
        50,
        &["demo.ability.sorcery-device-mastery"],
    );
    grant_spell_power(&mut mastery, 7);
    mastery
        .resolve_player_ability(
            "demo.ability.sorcery-device-mastery",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Device Mastery should resolve");
    let status = mastery
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_DEVICE_MASTERY)
        .expect("Device Mastery should apply its status");
    assert!((8..=14).contains(&status.remaining_ticks));
    assert_eq!(status.granted_modifiers.device_power_bonus, 5);
    assert_eq!(mastery.effective_player_device_power_bonus(), 5);
    assert_eq!(device_power_value(100, 5), 125);

    clear_monsters(&mut mastery);
    let target_position = Position {
        x: mastery.player.position.x + 1,
        y: mastery.player.position.y,
    };
    replace_terrain(&mut mastery, target_position, "demo.terrain.floor");
    mastery.entities.push(actor_from_runtime_spawn(
        "test.device-mastery.target",
        "demo.actor.small-kobold",
        target_position,
        100,
        110,
        100_000,
        true,
    ));
    let wand_id = mastery
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.magic-missile-wand")
        .map(|item| {
            item.activation
                .as_mut()
                .expect("starting wand should carry its activation")
                .device_check_difficulty = 0;
            item.id.clone()
        })
        .expect("Sorcery High-Mage should retain the shared starting wand");
    let mut plain = mastery.clone();
    plain
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_DEVICE_MASTERY);
    mastery.rng = RfbRng::seeded(32);
    plain.rng = RfbRng::seeded(32);
    for game in [&mut mastery, &mut plain] {
        dispatch_next(
            game,
            GameCommand::UseItem {
                item_id: wand_id.clone(),
                target: Some(TargetSelection::Direction {
                    direction: Direction::East,
                }),
            },
        );
    }
    let remaining_hp = |game: &Game| {
        game.entities
            .iter()
            .find(|actor| actor.id == "test.device-mastery.target")
            .expect("device target should survive the comparison")
            .hp
    };
    assert!(
        remaining_hp(&mastery) < remaining_hp(&plain),
        "Device Mastery should increase actual device damage"
    );

    let mut banish =
        sorcery_high_mage_game(0x4241_4e49_5348_3431, 41, &["demo.ability.sorcery-banish"]);
    clear_monsters(&mut banish);
    let origin = banish.player.position;
    let ordinary = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    let guardian = Position {
        x: origin.x + 2,
        y: origin.y,
    };
    replace_terrain(&mut banish, ordinary, "demo.terrain.floor");
    replace_terrain(&mut banish, guardian, "demo.terrain.floor");
    banish.entities.push(actor_from_runtime_spawn(
        "test.banish.ordinary",
        "demo.actor.small-kobold",
        ordinary,
        8,
        110,
        100,
        true,
    ));
    banish.entities.push(actor_from_runtime_spawn(
        "test.banish.guardian",
        "demo.actor.warrens-keeper",
        guardian,
        100,
        110,
        100,
        true,
    ));
    let mut events = Vec::new();
    banish
        .resolve_player_ability(
            "demo.ability.sorcery-banish",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Banish should resolve");
    let ordinary_after = banish
        .entities
        .iter()
        .find(|actor| actor.id == "test.banish.ordinary")
        .unwrap()
        .position;
    let guardian_after = banish
        .entities
        .iter()
        .find(|actor| actor.id == "test.banish.guardian")
        .unwrap()
        .position;
    assert_ne!(ordinary_after, ordinary);
    assert_eq!(guardian_after, guardian);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::Banish { targets, .. }]
                if targets.iter().any(|target| target.entity_id == "test.banish.guardian" && target.resisted))
    )));

    let mut invulnerable = sorcery_high_mage_game(
        0x494e_5655_4c4e_3432,
        42,
        &["demo.ability.sorcery-invulnerability"],
    );
    for (slot, kind) in [
        VirtueKindDto::Unlife,
        VirtueKindDto::Honour,
        VirtueKindDto::Sacrifice,
        VirtueKindDto::Valour,
    ]
    .into_iter()
    .enumerate()
    {
        invulnerable.virtues[slot] = VirtueDto { kind, value: 0 };
    }
    invulnerable
        .resources
        .get_mut("demo.resource.mana")
        .expect("Sorcery High-Mage should retain mana")
        .current = 1_000;
    let mut invulnerability_events = Vec::new();
    invulnerable
        .resolve_player_ability(
            "demo.ability.sorcery-invulnerability",
            TargetSelection::SelfTarget,
            &mut invulnerability_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Invulnerability should resolve");
    assert!(
        invulnerable.player_has_status_kind(STATUS_INVULNERABILITY),
        "{invulnerability_events:?}"
    );
    assert_eq!(invulnerable.virtue_current(VirtueKindDto::Unlife), -2);
    assert_eq!(invulnerable.virtue_current(VirtueKindDto::Honour), -2);
    assert_eq!(invulnerable.virtue_current(VirtueKindDto::Sacrifice), -3);
    assert_eq!(invulnerable.virtue_current(VirtueKindDto::Valour), -5);
    let status = invulnerable
        .player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == STATUS_INVULNERABILITY)
        .expect("Invulnerability should apply its shared status");
    assert_eq!(status.incoming_damage_percent, 0);
    assert!((5..=8).contains(&status.remaining_ticks));
    status.remaining_ticks = 1;
    let energy_before = invulnerable.player.energy_need;
    invulnerable
        .process_status_tick(
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
            false,
        )
        .expect("Invulnerability expiration should resolve");
    assert_eq!(
        invulnerable.player.energy_need,
        energy_before + STANDARD_ACTION_COST
    );
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
    assert_eq!(learned.len(), 32);
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
    let clairvoyance = learned
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-clairvoyance")
        .expect("Clairvoyance should complete the fourth book");
    assert_eq!(clairvoyance.book_rank, Some(4));
    assert_eq!(clairvoyance.minimum_level, 46);
    assert_eq!(clairvoyance.base_resource_cost, 80);
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
fn arcane_clairvoyance_maps_lights_reveals_and_grants_conditional_telepathy() {
    let mut game = arcane_high_mage_game(
        0x434c_4149_5256_4f59,
        46,
        &["demo.ability.arcane-clairvoyance"],
    );
    descend_one_floor(&mut game);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should retain mana");
    mana.current = 1_000;
    mana.maximum = 1_000;
    game.ability_progress
        .get_mut("demo.ability.arcane-clairvoyance")
        .expect("Clairvoyance progress should exist")
        .proficiency = SPELL_EXP_MASTER;
    for virtue in &mut game.virtues {
        if matches!(
            virtue.kind,
            VirtueKindDto::Knowledge | VirtueKindDto::Enlightenment
        ) {
            virtue.value = 0;
        }
    }
    game.explored.fill(false);
    game.glow.fill(false);
    give_inventory_item(&mut game, "test.clairvoyance-item", "demo.item.dagger");
    let item_position = Position { x: 0, y: 0 };
    game.items
        .iter_mut()
        .find(|item| item.id == "test.clairvoyance-item")
        .expect("Clairvoyance test item should exist")
        .location = ItemLocation::Ground(item_position);
    let knowledge_before = game.virtue_current(VirtueKindDto::Knowledge);
    let enlightenment_before = game.virtue_current(VirtueKindDto::Enlightenment);
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    game.resolve_player_ability(
        "demo.ability.arcane-clairvoyance",
        TargetSelection::SelfTarget,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .expect("Clairvoyance should resolve");

    assert!(
        game.explored.iter().all(|explored| *explored),
        "floor {}x{} has {} terrain cells, {} explored cells and {} unexplored cells; events: {events:#?}",
        game.width,
        game.height,
        game.terrain.len(),
        game.explored.len(),
        game.explored.iter().filter(|explored| !**explored).count(),
    );
    assert!(game.glow.iter().all(|glow| *glow));
    assert!(game.item_is_discovered("test.clairvoyance-item"));
    assert!(changed.contains(&item_position));
    assert_eq!(
        game.virtue_current(VirtueKindDto::Knowledge),
        knowledge_before + 1
    );
    assert_eq!(
        game.virtue_current(VirtueKindDto::Enlightenment),
        enlightenment_before + 1
    );
    let telepathy = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_TELEPATHY)
        .expect("Clairvoyance should grant temporary telepathy");
    assert!((26..=55).contains(&telepathy.remaining_ticks));
    assert_eq!(game.rng_draw_counter(), draws_before + 2);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::AbilityDetected { .. }))
            .count(),
        2
    );

    let mut permanent = arcane_high_mage_game(
        0x5045_524d_4145_5350,
        46,
        &["demo.ability.arcane-clairvoyance"],
    );
    let mana = permanent
        .resources
        .get_mut("demo.resource.mana")
        .expect("Arcane High-Mage should retain mana");
    mana.current = 1_000;
    mana.maximum = 1_000;
    permanent
        .ability_progress
        .get_mut("demo.ability.arcane-clairvoyance")
        .expect("Clairvoyance progress should exist")
        .proficiency = SPELL_EXP_MASTER;
    for virtue in &mut permanent.virtues {
        if matches!(
            virtue.kind,
            VirtueKindDto::Knowledge | VirtueKindDto::Enlightenment
        ) {
            virtue.value = 0;
        }
    }
    permanent
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.esp".to_owned());
    let permanent_draws = permanent.rng_draw_counter();
    permanent
        .resolve_player_ability(
            "demo.ability.arcane-clairvoyance",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("permanent ESP Clairvoyance should resolve");
    assert!(permanent.player_has_permanent_telepathy());
    assert!(!permanent.player_has_status_kind(STATUS_TELEPATHY));
    assert_eq!(permanent.rng_draw_counter(), permanent_draws + 1);
}

#[test]
fn death_high_mage_cannot_study_the_arcane_fourth_book() {
    let mut game = high_mage_game(0x4445_4154_4841_5243);
    game.progress.level = 100;
    game.progress.max_level = 100;
    give_inventory_item(
        &mut game,
        "test.foreign-manual",
        "demo.item.manual-of-mastery",
    );
    assert_eq!(
        game.study_player_ability("test.foreign-manual", "demo.ability.arcane-clairvoyance"),
        Err("ability-not-supported")
    );
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

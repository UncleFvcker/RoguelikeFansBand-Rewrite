// SPDX-License-Identifier: MPL-2.0

use super::support::{
    choose_human_talent_if_pending, clear_monsters, descend_one_floor, dispatch_next,
    give_inventory_item, replace_terrain,
};
use super::*;
use rfb_protocol::{DamageTypeDto, ItemDestructionElementDto};

const HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-death";
const ARCANE_HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-arcane";
const SORCERY_HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-sorcery";
const ARMAGEDDON_HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-armageddon";
const NATURE_HIGH_MAGE_BUILD_ID: &str = "demo.build.high-mage-nature";

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

fn armageddon_high_mage_game(seed: u64, level: u16) -> Game {
    let mut game = Game::new_with_build(seed, ARMAGEDDON_HIGH_MAGE_BUILD_ID)
        .expect("Armageddon High-Mage build should create");
    game.progress.level = level;
    game.progress.max_level = level;
    game.learned_abilities.extend(
        [
            "demo.ability.armageddon-shard-bolt",
            "demo.ability.armageddon-gravity-bolt",
            "demo.ability.armageddon-plasma-bolt",
            "demo.ability.armageddon-meteor",
            "demo.ability.armageddon-thunderclap",
            "demo.ability.armageddon-windblast",
            "demo.ability.armageddon-hellstorm",
            "demo.ability.armageddon-rocket",
            "demo.ability.armageddon-ice-bolt",
            "demo.ability.armageddon-water-ball",
            "demo.ability.armageddon-breathe-lightning",
            "demo.ability.armageddon-breathe-frost",
            "demo.ability.armageddon-breathe-fire",
            "demo.ability.armageddon-breathe-acid",
            "demo.ability.armageddon-breathe-plasma",
            "demo.ability.armageddon-breathe-gravity",
            "demo.ability.armageddon-mana-bolt",
            "demo.ability.armageddon-plasma-ball",
            "demo.ability.armageddon-mana-ball",
            "demo.ability.armageddon-breathe-sound",
            "demo.ability.armageddon-breathe-inertia",
            "demo.ability.armageddon-breathe-disintegration",
            "demo.ability.armageddon-breathe-mana",
            "demo.ability.armageddon-breathe-shards",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    give_inventory_item(
        &mut game,
        "test.earth-wind-and-fire",
        "demo.item.earth-wind-and-fire",
    );
    give_inventory_item(
        &mut game,
        "test.path-of-destruction",
        "demo.item.path-of-destruction",
    );
    give_inventory_item(
        &mut game,
        "test.day-of-ragnarok",
        "demo.item.day-of-ragnarok",
    );
    game.refresh_player_resource_maxima();
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Armageddon High-Mage should have mana")
        .current = 1_000;
    game.debug_ability_casts_succeed = true;
    game
}

fn nature_high_mage_game(seed: u64, level: u16) -> Game {
    let mut game = Game::new_with_build(seed, NATURE_HIGH_MAGE_BUILD_ID)
        .expect("Nature High-Mage build should create");
    game.progress.level = level;
    game.progress.max_level = level;
    game.learned_abilities.extend(
        [
            "demo.ability.nature-detect-creatures",
            "demo.ability.nature-lightning",
            "demo.ability.nature-detect-doors-and-traps",
            "demo.ability.nature-produce-food",
            "demo.ability.nature-daylight",
            "demo.ability.nature-wind-walker",
            "demo.ability.nature-resist-environment",
            "demo.ability.nature-cure-wounds-and-poison",
            "demo.ability.nature-stone-to-mud",
            "demo.ability.nature-frost-bolt",
            "demo.ability.nature-awareness",
            "demo.ability.nature-fire-bolt",
            "demo.ability.nature-ray-of-sunlight",
            "demo.ability.nature-entangle",
            "demo.ability.nature-natures-gate",
            "demo.ability.nature-herbal-healing",
            "demo.ability.nature-stair-building",
            "demo.ability.nature-stone-skin",
            "demo.ability.nature-resistance-true",
            "demo.ability.nature-forest-creation",
            "demo.ability.nature-stone-tell",
            "demo.ability.nature-wall-of-stone",
            "demo.ability.nature-protect-from-corrosion",
            "demo.ability.nature-call-sunlight",
            "demo.ability.nature-earthquake",
            "demo.ability.nature-fire-storm",
            "demo.ability.nature-blizzard",
            "demo.ability.nature-lightning-storm",
            "demo.ability.nature-whirlpool",
            "demo.ability.nature-ice-bolt",
            "demo.ability.nature-gravity-storm",
            "demo.ability.nature-natures-wrath",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    give_inventory_item(&mut game, "test.nature-mastery", "demo.item.nature-mastery");
    give_inventory_item(&mut game, "test.natures-gifts", "demo.item.natures-gifts");
    give_inventory_item(&mut game, "test.natures-wrath", "demo.item.natures-wrath");
    game.refresh_player_resource_maxima();
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Nature High-Mage should have mana")
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

fn nature_wrath_seed(branch: u64) -> u64 {
    (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            let _ = rng.bounded(100);
            rng.bounded(6) + 1 == branch
        })
        .expect("a bounded seed should select every Nature's Wrath branch")
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
fn armageddon_high_mage_birth_keeps_the_common_kit_and_only_its_first_book() {
    let game = Game::new_with_build(0x4152_4d41_4745_4444, ARMAGEDDON_HIGH_MAGE_BUILD_ID)
        .expect("Armageddon High-Mage build should create");
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
    for expected in [
        "demo.item.book-of-elements",
        "demo.item.dagger",
        "demo.item.robe",
        "demo.item.clarity-draught",
        "demo.item.magic-missile-wand",
    ] {
        assert!(carried.contains(expected));
    }
    assert!(!carried.contains("demo.item.black-prayers"));
    assert!(!carried.contains("demo.item.cantrips-for-beginners"));
    assert!(!carried.contains("demo.item.beginners-handbook"));
    assert!(!carried.contains("demo.item.earth-wind-and-fire"));
    assert!(!carried.contains("demo.item.path-of-destruction"));
    assert!(!carried.contains("demo.item.day-of-ragnarok"));

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
            .all(|ability| ability.id.starts_with("demo.ability.armageddon-"))
    );
}

#[test]
fn nature_high_mage_birth_keeps_the_common_kit_and_only_its_first_book() {
    let game = Game::new_with_build(0x4e41_5455_5245_3031, NATURE_HIGH_MAGE_BUILD_ID)
        .expect("Nature High-Mage build should create");
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
    for expected in [
        "demo.item.call-of-the-wild",
        "demo.item.dagger",
        "demo.item.robe",
        "demo.item.clarity-draught",
        "demo.item.magic-missile-wand",
    ] {
        assert!(carried.contains(expected));
    }
    for excluded in [
        "demo.item.black-prayers",
        "demo.item.cantrips-for-beginners",
        "demo.item.beginners-handbook",
        "demo.item.book-of-elements",
        "demo.item.nature-mastery",
        "demo.item.natures-gifts",
        "demo.item.natures-wrath",
    ] {
        assert!(!carried.contains(excluded));
    }

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
            .all(|ability| ability.id.starts_with("demo.ability.nature-"))
    );
}

#[test]
fn nature_first_book_projects_level_and_spell_power_formulas() {
    for (level, dice, range, light_sides, light_radius) in
        [(1, 3, 2, 0, 1), (25, 7, 6, 12, 3), (50, 12, 10, 25, 6)]
    {
        let projected = nature_high_mage_game(0x4e41_5455_5245_1000 + u64::from(level), level)
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            projected["demo.ability.nature-lightning"].target_spec.range,
            range
        );
        assert!(matches!(
            projected["demo.ability.nature-lightning"].effects.as_slice(),
            [AbilityEffectSpecDto::BeamDamage {
                damage_dice,
                damage_sides: 4,
                ..
            }] if *damage_dice == dice
        ));
        assert!(matches!(
            projected["demo.ability.nature-daylight"].effects.as_slice(),
            [AbilityEffectSpecDto::LightArea {
                damage_dice: 2,
                damage_sides,
                radius,
            }] if *damage_sides == light_sides && *radius == light_radius
        ));
    }

    let mut powered = nature_high_mage_game(0x4e41_5455_5245_5057, 50);
    grant_spell_power(&mut powered, 7);
    let lightning = powered
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.nature-lightning")
        .expect("Lightning should be projected");
    assert_eq!(lightning.target_spec.range, 15);
    assert!(matches!(
        lightning.effects.as_slice(),
        [AbilityEffectSpecDto::BeamDamage {
            final_damage_spell_power_bonus: Some(7),
            ..
        }]
    ));
}

#[test]
fn nature_first_book_applies_food_levitation_environment_and_curing() {
    let mut game = nature_high_mage_game(0x4e41_5455_5245_4546, 10);
    let position = game.player.position;
    game.resolve_player_ability(
        "demo.ability.nature-produce-food",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Produce Food should resolve");
    let ration = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.ration-of-food"
                && item.location == ItemLocation::Ground(position)
        })
        .expect("Produce Food should create a ration at the player's feet");
    assert_eq!(
        ration.origin_kind,
        Some(rfb_protocol::ItemOriginKindDto::Acquire)
    );

    game.resolve_player_ability(
        "demo.ability.nature-wind-walker",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Wind Walker should resolve");
    assert!(game.player_levitates());

    game.resolve_player_ability(
        "demo.ability.nature-resist-environment",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Resist Environment should resolve");
    for damage_type in [DamageType::Fire, DamageType::Cold, DamageType::Electricity] {
        assert_eq!(
            game.effective_player_resistances().level(damage_type),
            ResistanceLevel::Resistant
        );
    }
    assert_eq!(
        game.player
            .statuses
            .iter()
            .filter(|status| status.kind_id == "rfb.status.resist-environment")
            .count(),
        1
    );

    let status = |kind_id: &str, remaining_ticks| StatusInstance {
        kind_id: kind_id.to_owned(),
        intensity: 1,
        remaining_ticks,
        source_id: Some("test.nature".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    };
    game.player.statuses.push(status(STATUS_BLEEDING, 50));
    game.player.statuses.push(status(STATUS_POISON, 600));
    game.player.hp = game.player.hp.saturating_sub(20);
    let hp_before = game.player.hp;
    game.resolve_player_ability(
        "demo.ability.nature-cure-wounds-and-poison",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Cure Wounds & Poison should resolve");
    assert!(game.player.hp > hp_before);
    assert!(!game.player_has_status_kind(STATUS_BLEEDING));
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_POISON)
            .map(|status| status.remaining_ticks),
        Some(400)
    );
}

#[test]
fn nature_daylight_burns_an_unprotected_vampire_form() {
    let mut game = nature_high_mage_game(0x4e41_5455_5245_5355, 10);
    game.player.statuses.push(StatusInstance {
        kind_id: "test.status.vampire-form".to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.nature".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: Some("demo.race.vampire-lord".to_owned()),
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let hp_before = game.player.hp;
    game.resolve_player_ability(
        "demo.ability.nature-daylight",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Daylight should resolve");
    assert!((2..=4).contains(&hp_before.saturating_sub(game.player.hp)));
}

#[test]
fn nature_second_book_projects_bolts_entangle_and_fixed_healing() {
    for (level, frost_dice, fire_dice, beam_chance) in
        [(5, 3, 5, 5), (25, 8, 10, 25), (50, 14, 16, 50)]
    {
        let projected = nature_high_mage_game(0x4e41_5455_5245_2000 + u64::from(level), level)
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();
        for (id, expected_dice, damage_type) in [
            (
                "demo.ability.nature-frost-bolt",
                frost_dice,
                DamageTypeDto::Cold,
            ),
            (
                "demo.ability.nature-fire-bolt",
                fire_dice,
                DamageTypeDto::Fire,
            ),
        ] {
            assert!(matches!(
                projected[id].effects.as_slice(),
                [AbilityEffectSpecDto::BoltOrBeamDamage {
                    damage_dice,
                    damage_sides: 8,
                    damage_type: actual_type,
                    beam_chance_percent: actual_chance,
                    ..
                }] if *damage_dice == expected_dice
                    && *actual_type == damage_type
                    && *actual_chance == beam_chance
            ));
        }
        assert!(matches!(
            projected["demo.ability.nature-entangle"].effects.as_slice(),
            [AbilityEffectSpecDto::Entangle {
                power,
                duration_ticks: 50,
            }] if *power == level * 2
        ));
    }

    let mut powered = nature_high_mage_game(0x4e41_5455_5245_4850, 50);
    grant_spell_power(&mut powered, 7);
    let projected = powered
        .snapshot()
        .player
        .abilities
        .into_iter()
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();
    assert!(matches!(
        projected["demo.ability.nature-entangle"].effects.as_slice(),
        [AbilityEffectSpecDto::Entangle { power: 153, .. }]
    ));
    assert!(matches!(
        projected["demo.ability.nature-herbal-healing"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Heal { amount: 769 }, ..]
    ));
}

#[test]
fn nature_entangle_uses_the_original_unique_immunity_and_old_slow_save() {
    let mut game = nature_high_mage_game(0x4e41_5455_5245_534c, 50);
    clear_monsters(&mut game);
    let origin = game.player.position;
    for (id, kind_id, offset) in [
        ("test.entangle-normal", "demo.actor.small-kobold", 1),
        ("test.entangle-unique", "demo.actor.serpent-of-chaos", 2),
    ] {
        let definition = game.content.actor(kind_id).expect("test actor").clone();
        let position = Position {
            x: origin.x + offset,
            y: origin.y,
        };
        replace_terrain(&mut game, position, "demo.terrain.floor");
        game.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    }
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.nature-entangle",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Entangle should resolve");

    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLOW && status.remaining_ticks == 50)
    );
    assert!(
        game.entities[1]
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_SLOW)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if resolution.target_entity_id.as_deref() == Some("test.entangle-unique")
                && matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::ApplyStatus {
                    change: AbilityStatusChangeDto::Immune,
                    ..
                }])
    )));
}

#[test]
fn nature_gate_uses_all_three_level_bands_and_creates_upkeep_pets() {
    let cast = |seed, level| {
        let mut game = nature_high_mage_game(seed, level);
        clear_monsters(&mut game);
        let mut events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.nature-natures-gate",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Nature's Gate should resolve");
        let resolution = events
            .into_iter()
            .find_map(|event| match event {
                DomainEvent::AbilitySummoned { resolution, .. } => Some(resolution),
                _ => None,
            })
            .expect("Nature's Gate should report its summon result");
        (game, resolution)
    };

    let (game, resolution) = cast(0x4e41_5455_5245_2911, 29);
    assert_eq!(resolution.actor_kind_id, "animal-ranger");
    assert_eq!(resolution.entity_ids.len(), 1);
    assert_eq!(game.pet_upkeep().controlled_pets, 1);

    let mut middle_categories = BTreeSet::new();
    for seed in 0..96 {
        let (game, resolution) = cast(0x4e41_5455_3500_0000 + seed, 35);
        if !resolution.entity_ids.is_empty() {
            assert!(game.entities.iter().all(|entity| {
                entity.controller_id.as_deref() == Some(game.player.id.as_str())
            }));
            middle_categories.insert(resolution.actor_kind_id);
        }
        if middle_categories.len() == 3 {
            break;
        }
    }
    assert_eq!(
        middle_categories,
        BTreeSet::from([
            "animal-ranger".to_owned(),
            "hound".to_owned(),
            "hydra".to_owned(),
        ])
    );

    let mut saw_no_reinforcements = false;
    let mut saw_ent = false;
    for seed in 0..96 {
        let (game, resolution) = cast(0x4e41_5455_4700_0000 + seed, 47);
        if resolution.actor_kind_id == "nature-gate" {
            saw_no_reinforcements = true;
        } else if resolution.actor_kind_id == "ent" && !resolution.entity_ids.is_empty() {
            saw_ent = true;
            assert_eq!(resolution.summoned_kind_ids, ["demo.actor.ent"]);
            assert_eq!(game.pet_upkeep().controlled_pets, 1);
        }
        if saw_no_reinforcements && saw_ent {
            break;
        }
    }
    assert!(saw_no_reinforcements && saw_ent);
}

#[test]
fn nature_herbal_healing_scales_fixed_healing_and_cures_statuses() {
    let mut game = nature_high_mage_game(0x4e41_5455_5245_4848, 50);
    grant_spell_power(&mut game, 7);
    game.progress.hp_progression.fill(1_000);
    game.player.max_hp = 1_000;
    game.player.hp = 100;
    let status = |kind_id: &str, remaining_ticks| StatusInstance {
        kind_id: kind_id.to_owned(),
        intensity: 1,
        remaining_ticks,
        source_id: Some("test.nature".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    };
    game.player.statuses.push(status(STATUS_STUN, 40));
    game.player.statuses.push(status(STATUS_BLEEDING, 80));
    game.player.statuses.push(status(STATUS_POISON, 1_000));
    game.resolve_player_ability(
        "demo.ability.nature-herbal-healing",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Herbal Healing should resolve");
    assert_eq!(game.player.hp, 869);
    assert!(!game.player_has_status_kind(STATUS_STUN));
    assert!(!game.player_has_status_kind(STATUS_BLEEDING));
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_POISON)
            .map(|status| status.remaining_ticks),
        Some(500)
    );
}

#[test]
fn commit32_nature_third_book_projects_and_applies_stone_skin_and_shared_resistance() {
    for (level, defense) in [(8, 16), (25, 30), (50, 50)] {
        let projected = nature_high_mage_game(0x4e41_5455_5245_3200 + u64::from(level), level)
            .snapshot()
            .player
            .abilities;
        let stone_skin = projected
            .iter()
            .find(|ability| ability.id == "demo.ability.nature-stone-skin")
            .expect("Stone Skin should be projected");
        assert!(matches!(
            stone_skin.effects.as_slice(),
            [AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 20,
                duration_dice: 1,
                duration_sides: 30,
                granted_modifiers,
                ..
            }] if granted_modifiers.defense == defense
        ));
    }

    let mut game = nature_high_mage_game(0x4e41_5455_5245_3250, 50);
    grant_spell_power(&mut game, 7);
    let projected = game.snapshot().player.abilities;
    let stone_skin = projected
        .iter()
        .find(|ability| ability.id == "demo.ability.nature-stone-skin")
        .expect("Stone Skin should remain projected");
    assert!(matches!(
        stone_skin.effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 30,
            duration_sides: 46,
            granted_modifiers,
            ..
        }] if granted_modifiers.defense == 50
    ));
    let resistance = projected
        .iter()
        .find(|ability| ability.id == "demo.ability.nature-resistance-true")
        .expect("Resistance True should be projected");
    assert!(matches!(
        resistance.effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 30,
            duration_dice: 1,
            duration_sides: 30,
            granted_resistances,
            ..
        }] if granted_resistances.len() == 5
    ));

    for ability_id in [
        "demo.ability.nature-stone-skin",
        "demo.ability.nature-resistance-true",
    ] {
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap_or_else(|error| panic!("{ability_id} should resolve: {error:?}"));
    }
    let stone_skin = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == "rfb.status.stone-skin")
        .expect("Stone Skin should create one status");
    assert!((31..=76).contains(&stone_skin.remaining_ticks));
    assert_eq!(stone_skin.granted_modifiers.defense, 50);
    let resistance = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == "rfb.status.resistance-true")
        .expect("Resistance True should create one shared status");
    assert!((31..=60).contains(&resistance.remaining_ticks));
    for damage_type in [
        DamageType::Acid,
        DamageType::Electricity,
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Poison,
    ] {
        assert_eq!(
            resistance.granted_resistances.get(&damage_type),
            Some(&ResistanceLevel::Resistant)
        );
    }
}

#[test]
fn commit32_nature_forest_and_stone_wall_share_adjacent_terrain_rules() {
    let mut game = nature_high_mage_game(0x4e41_5455_5245_5445, 50);
    clear_monsters(&mut game);
    game.gold_piles.clear();
    game.floor_connections.clear();
    game.terrain.fill("demo.terrain.floor".to_owned());
    let expected = game
        .adjacent_terrain_creation_replacements(
            &["demo.terrain.floor".to_owned()],
            "demo.terrain.surface-tree",
        )
        .len();
    let mut changed = BTreeSet::new();
    game.resolve_player_ability(
        "demo.ability.nature-forest-creation",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut changed,
        &mut Vec::new(),
    )
    .expect("Forest Creation should resolve");
    assert_eq!(changed.len(), expected);
    assert!(expected > 0);
    assert_eq!(
        game.terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.surface-tree")
            .count(),
        expected
    );

    game.terrain.fill("demo.terrain.floor".to_owned());
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Nature High-Mage should retain mana")
        .current = 100;
    let expected = game
        .adjacent_terrain_creation_replacements(
            &["demo.terrain.floor".to_owned()],
            "demo.terrain.wall",
        )
        .len();
    let mut changed = BTreeSet::new();
    game.resolve_player_ability(
        "demo.ability.nature-wall-of-stone",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut changed,
        &mut Vec::new(),
    )
    .expect("Wall of Stone should resolve");
    assert_eq!(changed.len(), expected);
    assert!(expected > 0);
    assert_eq!(
        game.terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.wall")
            .count(),
        expected
    );
}

#[test]
fn commit32_nature_corrosion_protection_is_permanent_visible_and_location_agnostic() {
    let mut game = nature_high_mage_game(0x4e41_5455_5245_4143, 50);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("Nature High-Mage should retain mana");
    mana.current = 1_000;
    mana.maximum = 1_000;
    give_inventory_item(
        &mut game,
        "test.corrosion.inventory",
        "demo.item.hard-leather-armour",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.corrosion.inventory")
        .expect("inventory armor should exist")
        .enchantments
        .to_armor = -3;
    let hash_before = game.state_hash();
    game.resolve_player_ability(
        "demo.ability.nature-protect-from-corrosion",
        TargetSelection::Item {
            item_id: "test.corrosion.inventory".to_owned(),
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("inventory armor should accept corrosion protection");
    let protected = game
        .items
        .iter()
        .find(|item| item.id == "test.corrosion.inventory")
        .expect("protected armor should remain");
    assert_eq!(protected.enchantments.to_armor, 0);
    assert!(
        protected
            .permanent_destruction_immunities
            .contains(&ItemDestructionElement::Acid)
    );
    assert_ne!(game.state_hash(), hash_before);
    assert_eq!(
        game.inventory_dto()
            .into_iter()
            .find(|item| item.id == "test.corrosion.inventory")
            .expect("protected armor should be projected")
            .permanent_destruction_immunities,
        vec![ItemDestructionElementDto::Acid]
    );

    let player_position = game.player.position;
    game.items
        .iter_mut()
        .find(|item| item.id == "test.corrosion.inventory")
        .expect("protected armor should remain")
        .location = ItemLocation::Ground(player_position);
    game.resolve_ground_item_projectile_effects(
        "test.ability.acid",
        &[player_position],
        DamageType::Acid,
        true,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.corrosion.inventory")
    );

    give_inventory_item(
        &mut game,
        "test.corrosion.equipped",
        "demo.item.hard-leather-armour",
    );
    let equipped = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.corrosion.equipped")
        .expect("equipped armor should exist");
    equipped.location = ItemLocation::Equipped {
        slot_id: "body".to_owned(),
    };
    equipped.enchantments.to_armor = -2;
    equipped.curse = Some(ItemCurseSeverityDto::Normal);
    game.resolve_player_ability(
        "demo.ability.nature-protect-from-corrosion",
        TargetSelection::Item {
            item_id: "test.corrosion.equipped".to_owned(),
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("equipped armor should accept corrosion protection");
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.corrosion.equipped")
            .expect("equipped armor should remain")
            .enchantments
            .to_armor,
        -2
    );

    give_inventory_item(
        &mut game,
        "test.corrosion.floor",
        "demo.item.hard-leather-armour",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.corrosion.floor")
        .expect("floor armor should exist")
        .location = ItemLocation::Ground(player_position);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("Nature High-Mage should retain mana")
        .current = 100;
    game.resolve_player_ability(
        "demo.ability.nature-protect-from-corrosion",
        TargetSelection::Item {
            item_id: "test.corrosion.floor".to_owned(),
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("floor armor should accept corrosion protection");
    assert!(
        game.items
            .iter()
            .find(|item| item.id == "test.corrosion.floor")
            .expect("floor armor should remain")
            .permanent_destruction_immunities
            .contains(&ItemDestructionElement::Acid)
    );

    give_inventory_item(&mut game, "test.corrosion.dagger", "demo.item.dagger");
    let hash_before_invalid_target = game.state_hash();
    let draws_before_invalid_target = game.rng_draw_counter();
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.nature-protect-from-corrosion",
        TargetSelection::Item {
            item_id: "test.corrosion.dagger".to_owned(),
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("a non-armor target should be rejected without an execution error");
    assert_eq!(game.state_hash(), hash_before_invalid_target);
    assert_eq!(game.rng_draw_counter(), draws_before_invalid_target);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityTargetUnavailable { ability_id }]
            if ability_id == "demo.ability.nature-protect-from-corrosion"
    ));

    let mut persisted = Game::new(0x4143_4944);
    give_inventory_item(
        &mut persisted,
        "test.corrosion.persisted",
        "demo.item.hard-leather-armour",
    );
    persisted
        .items
        .iter_mut()
        .find(|item| item.id == "test.corrosion.persisted")
        .expect("persisted armor should exist")
        .permanent_destruction_immunities
        .insert(ItemDestructionElement::Acid);
    let restored = Game::from_save_with_content(persisted.to_save(), persisted.content.clone())
        .expect("corrosion protection should round-trip");
    assert_eq!(restored.state_hash(), persisted.state_hash());
    assert!(
        restored
            .items
            .iter()
            .find(|item| item.id == "test.corrosion.persisted")
            .expect("restored armor should remain")
            .permanent_destruction_immunities
            .contains(&ItemDestructionElement::Acid)
    );
}

#[test]
fn commit32_nature_call_sunlight_maps_lights_reveals_without_esp_and_burns_vampires() {
    let mut game = nature_high_mage_game(0x4e41_5455_5245_5355, 50);
    clear_monsters(&mut game);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("Nature High-Mage should retain mana");
    mana.current = 1_000;
    mana.maximum = 1_000;
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
    give_inventory_item(&mut game, "test.sunlight-item", "demo.item.dagger");
    let item_position = Position { x: 0, y: 0 };
    game.items
        .iter_mut()
        .find(|item| item.id == "test.sunlight-item")
        .expect("sunlight test item should exist")
        .location = ItemLocation::Ground(item_position);
    game.player.statuses.push(StatusInstance {
        kind_id: "test.status.vampire-form".to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.nature".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: Some("demo.race.vampire-lord".to_owned()),
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let hp_before = game.player.hp;
    game.resolve_player_ability(
        "demo.ability.nature-call-sunlight",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Call Sunlight should resolve");

    assert_eq!(game.player.hp, hp_before - 50);
    assert!(game.explored.iter().all(|explored| *explored));
    assert!(game.glow.iter().all(|glow| *glow));
    assert!(game.item_is_discovered("test.sunlight-item"));
    assert_eq!(game.virtue_current(VirtueKindDto::Knowledge), 1);
    assert_eq!(game.virtue_current(VirtueKindDto::Enlightenment), 1);
    assert!(!game.player_has_status_kind(STATUS_TELEPATHY));
}

#[test]
fn commit33_nature_fourth_book_projects_original_damage_radius_and_spell_power() {
    for (level, storm_bonus, radius, ice_dice, ice_bonus) in
        [(1, 66, 1, 5, 5), (25, 119, 3, 12, 10), (50, 174, 5, 20, 15)]
    {
        let projected = nature_high_mage_game(0x4e41_5455_5245_3300 + u64::from(level), level)
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();
        assert!(matches!(
            projected["demo.ability.nature-earthquake"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::Earthquake { radius: 10, .. }]
        ));
        assert!(matches!(
            projected["demo.ability.nature-fire-storm"].effects.as_slice(),
            [AbilityEffectSpecDto::AreaDamage {
                damage_dice: 1,
                damage_sides: 1,
                damage_bonus,
                radius: actual_radius,
                final_damage_spell_power_bonus: None,
                ..
            }] if *damage_bonus == storm_bonus && *actual_radius == radius
        ));
        assert!(matches!(
            projected["demo.ability.nature-ice-bolt"].effects.as_slice(),
            [AbilityEffectSpecDto::Damage {
                damage_dice,
                damage_sides: 15,
                damage_bonus,
                final_damage_spell_power_bonus: None,
                ..
            }] if *damage_dice == ice_dice && *damage_bonus == ice_bonus
        ));
        assert!(matches!(
            projected["demo.ability.nature-natures-wrath"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::NatureWrath]
        ));
    }

    let mut powered = nature_high_mage_game(0x4e41_5455_5245_3350, 50);
    grant_spell_power(&mut powered, 7);
    let projected = powered
        .snapshot()
        .player
        .abilities
        .into_iter()
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();
    assert!(matches!(
        projected["demo.ability.nature-earthquake"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Earthquake { radius: 15, .. }]
    ));
    assert!(matches!(
        projected["demo.ability.nature-fire-storm"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_bonus: 174,
            radius: 5,
            final_damage_spell_power_bonus: Some(7),
            ..
        }]
    ));

    let mut events = Vec::new();
    let target = powered
        .open_positions_around(powered.player.position, 1)
        .into_iter()
        .next()
        .expect("Fire Storm should have an adjacent target");
    powered
        .resolve_player_ability(
            "demo.ability.nature-fire-storm",
            TargetSelection::Position { position: target },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("powered Fire Storm should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage { resolution, .. }
            if resolution.base_raw_damage == 269 && resolution.radius == 5
    )));
}

#[test]
fn commit33_natures_wrath_selects_all_six_branches_and_orders_the_elemental_storms() {
    for branch in 1..=6_u64 {
        let mut game = nature_high_mage_game(0x4e41_5455_5245_3300 + branch, 50);
        clear_monsters(&mut game);
        let mana = game
            .resources
            .get_mut("demo.resource.mana")
            .expect("Nature High-Mage should have mana");
        mana.current = mana.maximum;
        game.ability_progress
            .get_mut("demo.ability.nature-natures-wrath")
            .expect("Nature's Wrath should have progress")
            .proficiency = SPELL_EXP_MASTER;
        game.rng = RfbRng::seeded(nature_wrath_seed(branch));
        let mut events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.nature-natures-wrath",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Nature's Wrath branch should resolve");
        assert!(
            events.iter().any(|event| matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::RandomChoice {
                            roll,
                            maximum_roll: 6,
                            ..
                        }] if *roll == branch as i32
                    )
            )),
            "branch {branch} events: {events:#?}"
        );
        if branch == 5 {
            let storms = events
                .iter()
                .filter_map(|event| match event {
                    DomainEvent::AbilityAreaDamage { resolution, .. } => Some((
                        resolution.damage_type,
                        resolution.base_raw_damage,
                        resolution.radius,
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                storms,
                vec![
                    (DamageTypeDto::Fire, 370, 5),
                    (DamageTypeDto::Cold, 370, 5),
                    (DamageTypeDto::Electricity, 370, 5),
                ]
            );
        }
    }
}

#[test]
fn commit33_natures_wrath_direction_prompt_is_atomic_cancelable_and_persistent() {
    let mut cancelled = nature_high_mage_game(0x4e41_5455_5245_3343, 50);
    choose_human_talent_if_pending(&mut cancelled);
    cancelled.learned_abilities.clear();
    cancelled
        .learned_abilities
        .insert("demo.ability.nature-natures-wrath".to_owned());
    let mana = cancelled
        .resources
        .get_mut("demo.resource.mana")
        .expect("Nature High-Mage should have mana");
    mana.current = mana.maximum;
    cancelled
        .ability_progress
        .get_mut("demo.ability.nature-natures-wrath")
        .expect("Nature's Wrath should have progress")
        .proficiency = SPELL_EXP_MASTER;
    cancelled.refresh_character_skills();
    cancelled.rng = RfbRng::seeded(nature_wrath_seed(2));
    let mana_before = cancelled.resources["demo.resource.mana"].current;
    let world_tick_before = cancelled.world_tick;
    dispatch_next(
        &mut cancelled,
        GameCommand::CastAbility {
            ability_id: "demo.ability.nature-natures-wrath".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(
        cancelled
            .pending_ability_direction
            .as_ref()
            .map(|pending| pending.branch_roll),
        Some(2)
    );
    assert_eq!(
        cancelled.resources["demo.resource.mana"].current,
        mana_before
    );
    assert_eq!(cancelled.world_tick, world_tick_before);
    assert_eq!(
        cancelled.ability_progress["demo.ability.nature-natures-wrath"].cast_count,
        0
    );
    let restored = Game::from_save(cancelled.to_save()).expect("pending direction should reload");
    assert_eq!(
        restored
            .pending_ability_direction
            .as_ref()
            .map(|pending| (pending.ability_id.as_str(), pending.branch_roll)),
        Some(("demo.ability.nature-natures-wrath", 2))
    );

    dispatch_next(&mut cancelled, GameCommand::CancelAbilityDirection);
    assert!(cancelled.pending_ability_direction.is_none());
    assert_eq!(
        cancelled.resources["demo.resource.mana"].current,
        mana_before
    );
    assert_eq!(cancelled.world_tick, world_tick_before);
    assert_eq!(
        cancelled.ability_progress["demo.ability.nature-natures-wrath"].cast_count,
        0
    );

    let mut resolved = nature_high_mage_game(0x4e41_5455_5245_3352, 50);
    choose_human_talent_if_pending(&mut resolved);
    resolved.learned_abilities.clear();
    resolved
        .learned_abilities
        .insert("demo.ability.nature-natures-wrath".to_owned());
    let mana = resolved
        .resources
        .get_mut("demo.resource.mana")
        .expect("Nature High-Mage should have mana");
    mana.current = mana.maximum;
    resolved
        .ability_progress
        .get_mut("demo.ability.nature-natures-wrath")
        .expect("Nature's Wrath should have progress")
        .proficiency = SPELL_EXP_MASTER;
    resolved.rng = RfbRng::seeded(nature_wrath_seed(6));
    let mana_before = resolved.resources["demo.resource.mana"].current;
    dispatch_next(
        &mut resolved,
        GameCommand::CastAbility {
            ability_id: "demo.ability.nature-natures-wrath".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    let resource_paid = resolved
        .pending_ability_direction
        .as_ref()
        .expect("the shard branch should request a direction")
        .cast_resolution
        .resource_paid;
    let mut events = Vec::new();
    resolved
        .resolve_pending_ability_direction(
            Direction::East,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("the deferred shard branch should resolve");
    assert!(resolved.pending_ability_direction.is_none());
    assert_eq!(
        resolved.resources["demo.resource.mana"].current,
        mana_before - resource_paid
    );
    assert_eq!(
        resolved.ability_progress["demo.ability.nature-natures-wrath"].cast_count,
        1
    );
    let shard_balls = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                DomainEvent::AbilityAreaDamage { resolution, .. }
                    if resolution.damage_type == DamageTypeDto::Shards
                        && resolution.base_raw_damage == 135
                        && resolution.radius == 1
            )
        })
        .count();
    assert_eq!(shard_balls, 3);
}

#[test]
fn armageddon_first_book_projects_original_level_beam_and_damage_formulas() {
    let ability_ids = [
        "demo.ability.armageddon-lightning-bolt",
        "demo.ability.armageddon-frost-bolt",
        "demo.ability.armageddon-fire-bolt",
        "demo.ability.armageddon-acid-bolt",
        "demo.ability.armageddon-lightning-ball",
        "demo.ability.armageddon-frost-ball",
        "demo.ability.armageddon-fire-ball",
        "demo.ability.armageddon-acid-ball",
    ];
    for (level, bolt_dice, spell_damage_bonus, beam_chance, ball_bonuses) in [
        (1, [3, 4, 5, 5], 5, 11, [25, 30, 35, 40]),
        (25, [9, 10, 11, 11], 10, 35, [66, 71, 76, 81]),
        (50, [15, 16, 17, 17], 15, 60, [109, 114, 119, 124]),
    ] {
        let projected = armageddon_high_mage_game(0x454c_454d_454e_5453, level)
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();
        for (id, damage_dice) in ability_ids[..4].iter().zip(bolt_dice) {
            assert!(matches!(
                projected[*id].effects.as_slice(),
                [AbilityEffectSpecDto::BoltOrBeamDamage {
                    damage_dice: actual_dice,
                    damage_sides: 8,
                    damage_bonus,
                    beam_chance_percent: actual_beam_chance,
                    final_damage_spell_power_bonus: None,
                    ..
                }] if *actual_dice == damage_dice
                    && *damage_bonus == spell_damage_bonus
                    && *actual_beam_chance == beam_chance
            ));
        }
        for (id, damage_bonus) in ability_ids[4..].iter().zip(ball_bonuses) {
            assert!(matches!(
                projected[*id].effects.as_slice(),
                [AbilityEffectSpecDto::AreaDamage {
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_bonus: actual_bonus,
                    radius: 2,
                    final_damage_spell_power_bonus: None,
                    ..
                }] if *actual_bonus == damage_bonus
            ));
        }
    }

    for bonus in [7, -20] {
        let mut game = armageddon_high_mage_game(0x5350_454c_4c50_4f57, 50);
        grant_spell_power(&mut game, bonus);
        let projected = game.snapshot().player.abilities;
        for id in ability_ids {
            let ability = projected
                .iter()
                .find(|ability| ability.id == id)
                .unwrap_or_else(|| panic!("{id} should be projected"));
            assert!(matches!(
                ability.effects.as_slice(),
                [AbilityEffectSpecDto::BoltOrBeamDamage {
                    final_damage_spell_power_bonus: Some(actual),
                    ..
                } | AbilityEffectSpecDto::AreaDamage {
                    final_damage_spell_power_bonus: Some(actual),
                    ..
                }] if *actual == bonus
            ));
        }
    }
}

#[test]
fn armageddon_second_book_projects_original_level_beam_and_damage_formulas() {
    for (level, beam_chance, bolt_dice, area_bonuses, thunder_radius) in [
        (15, 25, [10, 8, 14], [82, 125, 62, 195, 127], 3),
        (25, 35, [13, 11, 17], [94, 149, 74, 319, 169], 4),
        (50, 60, [19, 17, 23], [124, 209, 104, 629, 274], 7),
    ] {
        let projected = armageddon_high_mage_game(0x4541_5254_4857_0000 + u64::from(level), level)
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();
        for (id, damage_dice) in [
            "demo.ability.armageddon-shard-bolt",
            "demo.ability.armageddon-gravity-bolt",
            "demo.ability.armageddon-plasma-bolt",
        ]
        .into_iter()
        .zip(bolt_dice)
        {
            assert!(matches!(
                projected[id].effects.as_slice(),
                [AbilityEffectSpecDto::BoltOrBeamDamage {
                    damage_dice: actual_dice,
                    damage_sides: 8,
                    beam_chance_percent: actual_beam_chance,
                    ..
                }] if *actual_dice == damage_dice && *actual_beam_chance == beam_chance
            ));
        }
        for (id, damage_bonus) in [
            "demo.ability.armageddon-meteor",
            "demo.ability.armageddon-thunderclap",
            "demo.ability.armageddon-windblast",
            "demo.ability.armageddon-hellstorm",
            "demo.ability.armageddon-rocket",
        ]
        .into_iter()
        .zip(area_bonuses)
        {
            assert!(matches!(
                projected[id].effects.as_slice(),
                [AbilityEffectSpecDto::AreaDamage {
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_bonus: actual_bonus,
                    radius,
                    ..
                }] if *actual_bonus == damage_bonus
                    && (*radius == thunder_radius
                        || id != "demo.ability.armageddon-thunderclap")
            ));
        }
    }
}

#[test]
fn armageddon_special_projectiles_share_original_resistance_status_and_cell_rules() {
    let mut game = armageddon_high_mage_game(0x5350_4543_4941_4c53, 50);
    clear_monsters(&mut game);
    let origin = game.player.position;
    let target_position = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    for y in 0..game.height {
        for x in 0..game.width {
            replace_terrain(
                &mut game,
                Position {
                    x: i32::from(x),
                    y: i32::from(y),
                },
                "demo.terrain.floor",
            );
        }
    }
    game.entities.push(actor_from_runtime_spawn(
        "test.armageddon-special",
        "demo.actor.small-kobold",
        target_position,
        1_000,
        100,
        100,
        true,
    ));
    let trace = ProjectileTrace {
        origin,
        impact: target_position,
        landing: target_position,
        traversed: vec![target_position],
    };

    game.entities[0]
        .resistances
        .set(DamageType::Shards, ResistanceLevel::Resistant);
    let rocket = game
        .resolve_ability_damage_to_entity(
            0,
            "test.rocket",
            DamageType::Rocket,
            100,
            trace.clone(),
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("rocket damage should resolve");
    assert_eq!(rocket.applied, 50);

    game.entities[0].resistances = ResistanceProfile::default();
    game.resolve_ability_damage_to_entity(
        0,
        "test.gravity",
        DamageType::Gravity,
        100,
        trace,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("gravity damage should resolve");
    assert_ne!(game.entities[0].position, target_position);
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLOW)
    );
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    game.entities[0].kind_id = "demo.actor.quartz-vein".to_owned();
    game.entities[0].position = target_position;
    game.entities[0].statuses.clear();
    game.resolve_ability_damage_to_entity(
        0,
        "test.plasma",
        DamageType::Plasma,
        100,
        ProjectileTrace {
            origin,
            impact: target_position,
            landing: target_position,
            traversed: vec![target_position],
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("plasma damage should resolve");
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_STUN)
    );

    game.entities[0].kind_id = "demo.actor.small-kobold".to_owned();
    game.entities[0].statuses.clear();
    game.resolve_ability_damage_to_entity(
        0,
        "test.plasma",
        DamageType::Plasma,
        100,
        ProjectileTrace {
            origin,
            impact: target_position,
            landing: target_position,
            traversed: vec![target_position],
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("ordinary plasma target should resolve");
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    game.entities[0].hp = 1_000;
    game.entities[0].position = target_position;
    game.entities[0].statuses.clear();
    game.resolve_ability_damage_to_entity(
        0,
        "test.telekinesis",
        DamageType::Telekinesis,
        100,
        ProjectileTrace {
            origin,
            impact: target_position,
            landing: target_position,
            traversed: vec![target_position],
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("telekinesis damage should resolve");
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    let tree = Position {
        x: origin.x + 2,
        y: origin.y,
    };
    replace_terrain(&mut game, tree, "demo.terrain.surface-tree");
    game.resolve_projectile_terrain_effects(&[tree], DamageType::Shards, &mut BTreeSet::new());
    assert_eq!(
        game.terrain[game.index(tree).expect("tree should remain in bounds")],
        "demo.terrain.surface-grass"
    );

    give_inventory_item(&mut game, "test.meteor-scroll", "demo.item.accuracy-scroll");
    give_inventory_item(&mut game, "test.meteor-potion", "demo.item.antidote-potion");
    for item in game.items.iter_mut().filter(|item| {
        matches!(
            item.id.as_str(),
            "test.meteor-scroll" | "test.meteor-potion"
        )
    }) {
        item.location = ItemLocation::Ground(origin);
    }
    game.resolve_ground_item_projectile_effects(
        "test.meteor",
        &[origin],
        DamageType::Meteor,
        true,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(game.items.iter().all(|item| !matches!(
        item.id.as_str(),
        "test.meteor-scroll" | "test.meteor-potion"
    )));
}

#[test]
fn armageddon_third_book_projects_original_formulas_and_breath_radius_boundary() {
    for (level, radius, ice_dice, water_bonus, cone_bonuses) in [
        (40, 2, 15, 122, [192, 192, 212, 212, 232, 172]),
        (41, 3, 15, 124, [196, 196, 217, 217, 237, 176]),
    ] {
        let projected = armageddon_high_mage_game(0x5041_5448_4000 + u64::from(level), level)
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();
        assert!(matches!(
            projected["demo.ability.armageddon-ice-bolt"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::Damage {
                damage_dice: actual_dice,
                damage_sides: 15,
                damage_bonus: 13,
                damage_type: DamageTypeDto::Ice,
                ..
            }] if *actual_dice == ice_dice
        ));
        assert!(matches!(
            projected["demo.ability.armageddon-water-ball"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::AreaDamage {
                damage_dice: 1,
                damage_sides: 1,
                damage_bonus: actual_bonus,
                damage_type: DamageTypeDto::Water,
                radius: 2,
                ..
            }] if *actual_bonus == water_bonus
        ));
        for (id, damage_type, damage_bonus) in [
            (
                "demo.ability.armageddon-breathe-lightning",
                DamageTypeDto::Electricity,
                cone_bonuses[0],
            ),
            (
                "demo.ability.armageddon-breathe-frost",
                DamageTypeDto::Cold,
                cone_bonuses[1],
            ),
            (
                "demo.ability.armageddon-breathe-fire",
                DamageTypeDto::Fire,
                cone_bonuses[2],
            ),
            (
                "demo.ability.armageddon-breathe-acid",
                DamageTypeDto::Acid,
                cone_bonuses[3],
            ),
            (
                "demo.ability.armageddon-breathe-plasma",
                DamageTypeDto::Plasma,
                cone_bonuses[4],
            ),
            (
                "demo.ability.armageddon-breathe-gravity",
                DamageTypeDto::Gravity,
                cone_bonuses[5],
            ),
        ] {
            assert!(matches!(
                projected[id].effects.as_slice(),
                [AbilityEffectSpecDto::ConeDamage {
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_bonus: actual_bonus,
                    damage_type: actual_type,
                    radius: actual_radius,
                    ..
                }] if *actual_bonus == damage_bonus
                    && *actual_type == damage_type
                    && *actual_radius == radius
            ));
        }
    }
}

#[test]
fn armageddon_breath_damage_matches_projection_and_affects_items_and_terrain() {
    for (bonus, expected_damage) in [(7, 335), (-7, 101)] {
        let seed = 0x4252_4541_5448_u64.wrapping_add_signed(i64::from(bonus));
        let mut game = armageddon_high_mage_game(seed, 41);
        grant_spell_power(&mut game, bonus);
        clear_monsters(&mut game);
        let origin = game.player.position;
        for y in 0..game.height {
            for x in 0..game.width {
                replace_terrain(
                    &mut game,
                    Position {
                        x: i32::from(x),
                        y: i32::from(y),
                    },
                    "demo.terrain.floor",
                );
            }
        }
        let target = Position {
            x: origin.x + 1,
            y: origin.y,
        };
        game.entities.push(actor_from_runtime_spawn(
            "test.breath-target",
            "demo.actor.small-kobold",
            target,
            1_000,
            100,
            100,
            true,
        ));
        let tree = Position {
            x: origin.x + 7,
            y: origin.y + 1,
        };
        replace_terrain(&mut game, tree, "demo.terrain.surface-tree");
        give_inventory_item(&mut game, "test.breath-scroll", "demo.item.accuracy-scroll");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.breath-scroll")
            .expect("breath test scroll should exist")
            .location = ItemLocation::Ground(tree);

        let projected = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == "demo.ability.armageddon-breathe-fire")
            .expect("fire breath should project");
        assert!(matches!(
            projected.effects.as_slice(),
            [AbilityEffectSpecDto::ConeDamage {
                damage_dice: 1,
                damage_sides: 1,
                damage_bonus: 217,
                final_damage_spell_power_bonus: Some(actual_bonus),
                radius: 3,
                ..
            }] if *actual_bonus == bonus
        ));

        let draws = game.rng_draw_counter();
        game.resolve_player_ability(
            "demo.ability.armageddon-breathe-fire",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("fire breath should resolve");
        assert_eq!(1_000 - game.entities[0].hp, expected_damage);
        assert_eq!(game.rng_draw_counter(), draws + 2);
        assert_eq!(
            game.terrain[game.index(tree).expect("tree should remain in bounds")],
            "demo.terrain.surface-grass"
        );
        assert!(
            game.items
                .iter()
                .all(|item| item.id != "test.breath-scroll")
        );
    }
}

#[test]
fn armageddon_ice_and_water_use_original_resistance_and_stun_rules() {
    let mut game = armageddon_high_mage_game(0x4943_455f_5741_5445, 41);
    clear_monsters(&mut game);
    let origin = game.player.position;
    let target = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    game.entities.push(actor_from_runtime_spawn(
        "test.ice-water-target",
        "demo.actor.small-kobold",
        target,
        1_000,
        100,
        100,
        true,
    ));
    game.entities[0]
        .resistances
        .set(DamageType::Ice, ResistanceLevel::Immune);
    game.entities[0]
        .resistances
        .set(DamageType::Cold, ResistanceLevel::Resistant);
    let trace = ProjectileTrace {
        origin,
        impact: target,
        landing: target,
        traversed: vec![target],
    };
    let ice = game
        .resolve_ability_damage_to_entity(
            0,
            "test.ice",
            DamageType::Ice,
            100,
            trace.clone(),
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("ice damage should resolve");
    assert_eq!(ice.applied, 50);
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN && (1..=15).contains(&status.intensity))
    );

    game.entities[0].hp = 1_000;
    game.entities[0].statuses.clear();
    game.entities[0].resistances = ResistanceProfile::default();
    let water = game
        .resolve_ability_damage_to_entity(
            0,
            "test.water",
            DamageType::Water,
            100,
            trace,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("water damage should resolve");
    assert_eq!(water.applied, 100);
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );
}

#[test]
fn armageddon_fourth_book_projects_original_formulas_and_breath_radius_boundary() {
    for (level, radius, mana_sides, ball_bonuses, cone_bonuses) in [
        (40, 2, 200, [182, 272], [252, 212, 292, 372, 412]),
        (41, 3, 205, [184, 276], [258, 217, 299, 381, 422]),
    ] {
        let projected = armageddon_high_mage_game(0x5241_474e_4152_0000 + u64::from(level), level)
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();
        assert!(matches!(
            projected["demo.ability.armageddon-mana-bolt"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::Damage {
                damage_dice: 1,
                damage_sides,
                damage_bonus: 63,
                damage_type: DamageTypeDto::Mana,
                ..
            }] if *damage_sides == mana_sides
        ));
        for (id, damage_type, damage_bonus) in [
            (
                "demo.ability.armageddon-plasma-ball",
                DamageTypeDto::Plasma,
                ball_bonuses[0],
            ),
            (
                "demo.ability.armageddon-mana-ball",
                DamageTypeDto::Mana,
                ball_bonuses[1],
            ),
        ] {
            assert!(matches!(
                projected[id].effects.as_slice(),
                [AbilityEffectSpecDto::AreaDamage {
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_bonus: actual_bonus,
                    damage_type: actual_type,
                    radius: 3,
                    ..
                }] if *actual_bonus == damage_bonus && *actual_type == damage_type
            ));
        }
        for (id, damage_type, damage_bonus) in [
            (
                "demo.ability.armageddon-breathe-sound",
                DamageTypeDto::Sound,
                cone_bonuses[0],
            ),
            (
                "demo.ability.armageddon-breathe-inertia",
                DamageTypeDto::Inertia,
                cone_bonuses[1],
            ),
            (
                "demo.ability.armageddon-breathe-disintegration",
                DamageTypeDto::Disintegrate,
                cone_bonuses[2],
            ),
            (
                "demo.ability.armageddon-breathe-mana",
                DamageTypeDto::Mana,
                cone_bonuses[3],
            ),
            (
                "demo.ability.armageddon-breathe-shards",
                DamageTypeDto::Shards,
                cone_bonuses[4],
            ),
        ] {
            assert!(matches!(
                projected[id].effects.as_slice(),
                [AbilityEffectSpecDto::ConeDamage {
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_bonus: actual_bonus,
                    damage_type: actual_type,
                    radius: actual_radius,
                    ..
                }] if *actual_bonus == damage_bonus
                    && *actual_type == damage_type
                    && *actual_radius == radius
            ));
        }
    }

    for bonus in [7, -7] {
        let mut game = armageddon_high_mage_game(0x5350_504f_5745_5200, 50);
        grant_spell_power(&mut game, bonus);
        for ability in game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .filter(|ability| {
                matches!(
                    ability.id.as_str(),
                    "demo.ability.armageddon-mana-bolt"
                        | "demo.ability.armageddon-plasma-ball"
                        | "demo.ability.armageddon-mana-ball"
                        | "demo.ability.armageddon-breathe-sound"
                        | "demo.ability.armageddon-breathe-inertia"
                        | "demo.ability.armageddon-breathe-disintegration"
                        | "demo.ability.armageddon-breathe-mana"
                        | "demo.ability.armageddon-breathe-shards"
                )
            })
        {
            assert!(matches!(
                ability.effects.as_slice(),
                [AbilityEffectSpecDto::Damage {
                    final_damage_spell_power_bonus: Some(actual),
                    ..
                } | AbilityEffectSpecDto::AreaDamage {
                    final_damage_spell_power_bonus: Some(actual),
                    ..
                } | AbilityEffectSpecDto::ConeDamage {
                    final_damage_spell_power_bonus: Some(actual),
                    ..
                }] if *actual == bonus
            ));
        }
    }
}

#[test]
fn armageddon_sound_and_inertia_use_distinct_original_monster_riders() {
    let mut game = armageddon_high_mage_game(0x534f_554e_445f_494e, 50);
    clear_monsters(&mut game);
    let target = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    game.entities.push(actor_from_runtime_spawn(
        "test.sound-inertia-target",
        "demo.actor.small-kobold",
        target,
        1_000,
        100,
        100,
        true,
    ));
    let trace = ProjectileTrace {
        origin: game.player.position,
        impact: target,
        landing: target,
        traversed: vec![target],
    };

    game.resolve_ability_damage_to_entity(
        0,
        "test.sound",
        DamageType::Sound,
        100,
        trace.clone(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("sound damage should resolve");
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    game.entities[0].hp = 1_000;
    game.entities[0].statuses.clear();
    game.resolve_ability_damage_to_entity(
        0,
        "test.inertia",
        DamageType::Inertia,
        100,
        trace.clone(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("first inertia damage should resolve");
    game.entities[0].hp = 1_000;
    game.resolve_ability_damage_to_entity(
        0,
        "test.inertia",
        DamageType::Inertia,
        100,
        trace,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("second inertia damage should resolve");
    assert_eq!(game.entities[0].minor_slow, 10);
    let projected = game
        .snapshot()
        .entities
        .into_iter()
        .find(|entity| entity.id == "test.sound-inertia-target")
        .expect("minor-slow target should remain visible");
    assert_eq!((projected.minor_slow, projected.speed), (10, 90));

    let mut save_game = Game::new_with_build(0x4d49_4e4f_525f_534c, ARMAGEDDON_HIGH_MAGE_BUILD_ID)
        .expect("Armageddon High-Mage build should create");
    let definition = save_game
        .content
        .actor("demo.actor.small-kobold")
        .cloned()
        .expect("small kobold should exist");
    let saved_actor = spawn_actor_from_definition(
        &mut save_game.rng,
        &definition,
        "test.saved-minor-slow",
        target,
        INITIAL_MONSTER_ENERGY_NEED,
        true,
    );
    save_game.entities.push(saved_actor);
    let unslowed_hash = save_game.state_hash();
    save_game.entities[0].minor_slow = 10;
    assert_ne!(save_game.state_hash(), unslowed_hash);
    let save = save_game.to_save();
    let restored = Game::from_save_with_content(save.clone(), save_game.content.clone())
        .expect("monster minor slow should round-trip");
    assert_eq!(restored.entities[0].minor_slow, 10);
    let mut invalid = save;
    invalid.entities[0].minor_slow = 11;
    assert!(matches!(
        Game::from_save_with_content(invalid, save_game.content.clone()),
        Err(CoreError::InvalidSave("entity minor slow is invalid"))
    ));

    let seed = (0..10_000)
        .find(|seed| {
            let mut ordinary = RfbRng::seeded(*seed);
            let mut regenerating = RfbRng::seeded(*seed);
            ordinary.bounded(100) >= 10 && regenerating.bounded(50) < 10
        })
        .expect("a recovery seed should exist");
    let mut ordinary = game.clone();
    ordinary.entities[0].minor_slow = 10;
    ordinary.rng = RfbRng::seeded(seed);
    ordinary.process_monster_minor_slow_recovery(0);
    assert_eq!(ordinary.entities[0].minor_slow, 10);
    let mut regenerating = game;
    regenerating.entities[0].kind_id = "demo.actor.forest-troll".to_owned();
    regenerating.entities[0].minor_slow = 10;
    regenerating.rng = RfbRng::seeded(seed);
    regenerating.process_monster_minor_slow_recovery(0);
    assert_eq!(regenerating.entities[0].minor_slow, 9);
}

#[test]
fn armageddon_disintegration_cone_crosses_destructible_terrain_but_stops_at_permanent_terrain() {
    let mut game = armageddon_high_mage_game(0x4449_5349_4e54_4547, 50);
    clear_monsters(&mut game);
    let origin = game.player.position;
    for y in 0..game.height {
        for x in 0..game.width {
            replace_terrain(
                &mut game,
                Position {
                    x: i32::from(x),
                    y: i32::from(y),
                },
                "demo.terrain.floor",
            );
        }
    }
    let wall = Position {
        x: origin.x + 2,
        y: origin.y,
    };
    let before_permanent = Position {
        x: origin.x + 4,
        y: origin.y,
    };
    let permanent = Position {
        x: origin.x + 5,
        y: origin.y,
    };
    let after_permanent = Position {
        x: origin.x + 6,
        y: origin.y,
    };
    replace_terrain(&mut game, wall, "demo.terrain.wall");
    replace_terrain(&mut game, permanent, "demo.terrain.permanent-wall");
    let quartz = game
        .content
        .actor("demo.actor.quartz-vein")
        .cloned()
        .expect("quartz vein should exist");
    let mut quartz = spawn_actor_from_definition(
        &mut game.rng,
        &quartz,
        "test.disintegration-before",
        before_permanent,
        INITIAL_MONSTER_ENERGY_NEED,
        true,
    );
    quartz.hp = 1_000;
    quartz.max_hp = 1_000;
    game.entities.push(quartz);
    game.entities.push(actor_from_runtime_spawn(
        "test.disintegration-after",
        "demo.actor.small-kobold",
        after_permanent,
        1_000,
        100,
        100,
        true,
    ));
    for (id, kind_id) in [
        ("test.disintegration-scroll", "demo.item.accuracy-scroll"),
        ("test.disintegration-artifact", "demo.item.pain"),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        game.items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("disintegration test item should exist")
            .location = ItemLocation::Ground(before_permanent);
    }

    game.resolve_player_ability(
        "demo.ability.armageddon-breathe-disintegration",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("disintegration breath should resolve");

    assert_eq!(
        game.terrain[game.index(wall).expect("wall should remain in bounds")],
        "demo.terrain.floor"
    );
    assert_eq!(
        game.terrain[game
            .index(permanent)
            .expect("permanent wall should remain in bounds")],
        "demo.terrain.permanent-wall"
    );
    assert_eq!(game.entities[0].hp, 453);
    assert_eq!(game.entities[1].hp, 1_000);
    assert!(
        game.items
            .iter()
            .all(|item| item.id != "test.disintegration-scroll")
    );
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.disintegration-artifact")
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
    choose_human_talent_if_pending(&mut ordinary);
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
    choose_human_talent_if_pending(&mut ordinary);
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
    choose_human_talent_if_pending(&mut mastery);
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
    choose_human_talent_if_pending(&mut ordinary);
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

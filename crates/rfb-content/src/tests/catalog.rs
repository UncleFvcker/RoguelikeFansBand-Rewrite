use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[test]
fn compiled_catalog_indexes_current_rfb_content() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

    assert_eq!(catalog.pack_id(), "rfb.demo.original-v1");
    assert_eq!(catalog.pack_version(), "1.376.0");
    assert_eq!(catalog.races().count(), 57);
    let human_weakness = catalog
        .race("demo.race.rfb-human")
        .expect("formal Human race should exist")
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "human-weakness")
        .expect("Human should receive the level 35 weakness");
    assert_eq!(human_weakness.minimum_level, 35);
    let RaceMutationSelectionDefinition::CastingAttribute {
        default_mutation_id,
        mutation_ids_by_attribute,
    } = &human_weakness.selection
    else {
        panic!("Human weakness should follow the class casting attribute");
    };
    assert_eq!(default_mutation_id, "rfb.mutation.human-str");
    assert_eq!(
        mutation_ids_by_attribute,
        &BTreeMap::from([
            (
                CastingAttribute::Intelligence,
                "rfb.mutation.human-int".to_owned(),
            ),
            (
                CastingAttribute::Wisdom,
                "rfb.mutation.human-wis".to_owned(),
            ),
            (
                CastingAttribute::Dexterity,
                "rfb.mutation.human-dex".to_owned(),
            ),
            (
                CastingAttribute::Constitution,
                "rfb.mutation.human-con".to_owned(),
            ),
            (
                CastingAttribute::Charisma,
                "rfb.mutation.human-chr".to_owned(),
            ),
        ])
    );
    assert_eq!(
        catalog
            .race_by_legacy_index(6)
            .expect("P62 Snotling form")
            .id,
        "rfb-legacy.race.snotling"
    );
    assert!(
        catalog
            .race_by_legacy_index(36)
            .expect("P62 Android immunity profile")
            .tags
            .iter()
            .any(|tag| tag == "polymorph-immune")
    );
    assert!(catalog.mutation("rfb.mutation.spit-acid").is_some());
    assert!(
        catalog
            .ability("demo.ability.warrens-scare")
            .is_some_and(|ability| ability.player.is_none())
    );
    assert!(
        catalog
            .ability("demo.ability.death-dark-bolt")
            .is_some_and(|ability| ability.player.is_some())
    );
    assert_eq!(
        catalog
            .item("demo.item.black-prayers")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.black-prayers")
    );
    assert_eq!(
        catalog
            .class("demo.class.warrior")
            .expect("Warrior class")
            .pet_upkeep_divisor,
        40
    );
    assert_eq!(
        catalog
            .class("demo.class.high-mage")
            .expect("High-Mage class")
            .pet_upkeep_divisor,
        25
    );
    assert!(catalog.build("demo.build.high-mage-death").is_some());
    assert!(catalog.build("demo.build.high-mage-arcane").is_some());
    assert!(catalog.build("demo.build.high-mage-sorcery").is_some());
    assert!(catalog.build("demo.build.high-mage-armageddon").is_some());
    assert!(catalog.build("demo.build.high-mage-nature").is_some());
    assert!(catalog.build("demo.build.high-mage-life").is_some());
    assert_eq!(
        catalog
            .item("demo.item.cantrips-for-beginners")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.cantrips-for-beginners")
    );
    assert_eq!(
        catalog
            .item("demo.item.minor-arcana")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.minor-arcana")
    );
    assert_eq!(
        catalog
            .item("demo.item.manual-of-mastery")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.manual-of-mastery")
    );
    assert_eq!(
        catalog
            .item("demo.item.beginners-handbook")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.beginners-handbook")
    );
    assert_eq!(
        catalog
            .item("demo.item.master-sorcerers-handbook")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.master-sorcerers-handbook")
    );
    assert_eq!(
        catalog
            .item("demo.item.pattern-sorcery")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.pattern-sorcery")
    );
    assert_eq!(
        catalog
            .item("demo.item.book-of-elements")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.book-of-elements")
    );
    assert_eq!(
        catalog
            .class("demo.class.archer")
            .expect("Archer class")
            .pet_upkeep_divisor,
        40
    );
    assert!(catalog.build("demo.build.archer").is_some());
    assert!(catalog.class("demo.class.paladin").is_some());
    assert!(catalog.build("demo.build.paladin-death").is_some());
    let cavalry = catalog.class("demo.class.cavalry").expect("Cavalry class");
    assert_eq!(cavalry.pet_upkeep_divisor, 35);
    assert!(cavalry.riding_combat_expert);
    assert_eq!(cavalry.mounted_non_arrow_base_shot_cap, Some(100));
    assert!(catalog.build("demo.build.cavalry").is_some());
    let sniper = catalog.class("demo.class.sniper").expect("Sniper class");
    assert_eq!(sniper.pet_upkeep_divisor, 40);
    assert!(sniper.sniping_profile.is_some());
    assert!(catalog.build("demo.build.sniper").is_some());
    for (class_id, initial, maximum) in [
        ("demo.class.warrior", 0, 6_000),
        ("demo.class.high-mage", 0, 0),
        ("demo.class.archer", 0, 4_000),
        ("demo.class.paladin", 0, 6_000),
        ("demo.class.cavalry", 2_000, 8_000),
        ("demo.class.sniper", 0, 0),
    ] {
        let riding = catalog
            .class(class_id)
            .expect("formal class")
            .riding_proficiency;
        assert_eq!((riding.initial, riding.maximum), (initial, maximum));
    }
    let warrior_proficiency = catalog
        .class("demo.class.warrior")
        .and_then(|class| class.weapon_proficiency.as_ref())
        .expect("Warrior weapon proficiency");
    assert_eq!(warrior_proficiency.default_initial, 4_000);
    assert_eq!(warrior_proficiency.default_maximum, 8_000);
    assert_eq!(
        warrior_proficiency.overrides["demo.item.short-bow"].maximum,
        7_000
    );
    assert_eq!(
        catalog
            .class("demo.class.high-mage")
            .and_then(|class| class.weapon_proficiency.as_ref())
            .expect("High-Mage weapon proficiency")
            .overrides["demo.item.dagger"]
            .maximum,
        8_000
    );
    assert_eq!(
        catalog
            .class("demo.class.archer")
            .and_then(|class| class.weapon_proficiency.as_ref())
            .expect("Archer weapon proficiency")
            .overrides["demo.item.short-bow"]
            .maximum,
        8_000
    );
    let paladin = catalog.class("demo.class.paladin").expect("Paladin class");
    assert_eq!(
        paladin
            .weapon_proficiency
            .as_ref()
            .expect("Paladin weapon proficiency")
            .overrides["demo.item.broad-sword"]
            .initial,
        4_000
    );
    assert!(
        paladin
            .abilities
            .iter()
            .any(|ability| ability.ability_id == "demo.ability.paladin-hell-lance")
    );
    assert_eq!(paladin.level_resistances[0].minimum_level, 40);
    assert_eq!(
        cavalry
            .weapon_proficiency
            .as_ref()
            .expect("Cavalry weapon proficiency")
            .overrides["demo.item.short-bow"]
            .initial,
        4_000
    );
    let cavalry_weapon_proficiency = cavalry
        .weapon_proficiency
        .as_ref()
        .expect("Cavalry weapon proficiency");
    assert_eq!(cavalry_weapon_proficiency.default_initial, 2_000);
    assert_eq!(cavalry_weapon_proficiency.default_maximum, 7_000);
    assert_eq!(
        (
            cavalry_weapon_proficiency.overrides["demo.item.heavy-lance"].initial,
            cavalry_weapon_proficiency.overrides["demo.item.heavy-lance"].maximum,
        ),
        (4_000, 8_000)
    );
    assert!(
        cavalry
            .abilities
            .iter()
            .any(|ability| ability.ability_id == "demo.ability.cavalry-rodeo")
    );
    let sniper_weapon_proficiency = sniper
        .weapon_proficiency
        .as_ref()
        .expect("Sniper weapon proficiency");
    assert_eq!(sniper_weapon_proficiency.default_initial, 2_000);
    assert_eq!(sniper_weapon_proficiency.default_maximum, 4_000);
    assert_eq!(
        (
            sniper_weapon_proficiency.overrides["demo.item.light-crossbow"].initial,
            sniper_weapon_proficiency.overrides["demo.item.light-crossbow"].maximum,
        ),
        (4_000, 8_000)
    );
    assert_eq!(sniper.abilities.len(), 17);
    assert!(
        catalog
            .class("demo.class.archer")
            .expect("Archer class")
            .abilities
            .iter()
            .all(|ability| ability.ui_group_name_key.as_deref()
                == Some("ability-group-demo-archer-create-ammo-name"))
    );
    assert!(catalog.item("demo.item.quiver").is_some());
    assert!(catalog.item("demo.item.shard-of-pottery").is_some());
    assert!(catalog.item("demo.item.broken-stick").is_some());
    assert_eq!(
        catalog
            .item("demo.item.crisdurian")
            .and_then(|item| item.weapon_proficiency_base_item_id.as_deref()),
        Some("demo.item.executioners-sword")
    );
    assert!(catalog.affix("rfb-legacy.affix.slaying").is_some());
    assert!(catalog.affix("rfb-legacy.affix.protection").is_some());
    assert!(catalog.affix("demo.affix.ammo-elemental").is_some());
    assert!(catalog.class("demo.class.mage").is_none());
    let world = catalog
        .world("demo.world.middle-earth")
        .expect("Middle-earth should be indexed");
    assert_eq!(world.initial_floor_id, "demo.floor.surface");
    assert!(
        world
            .dungeons
            .iter()
            .any(|dungeon| dungeon.id == "demo.dungeon.warrens")
    );
    assert!(
        world
            .procedural_floors
            .iter()
            .any(|floor| floor.id == "demo.floor.warrens-depth-1")
    );
    assert!(
        catalog
            .encounter_table("demo.encounter-table.warrens")
            .is_some()
    );
    assert!(catalog.loot_table("demo.loot-table.base-items").is_some());
    assert!(catalog.item("demo.item.magic-missile-wand").is_some());
    assert!(catalog.affix("demo.affix.regeneration").is_some());
}

#[test]
fn formal_human_matches_rfb_static_profile() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let human = catalog
        .race("demo.race.rfb-human")
        .expect("formal Human race");

    assert_eq!(human.modifiers, StatModifiers::default());
    assert_eq!(human.life_percent, 100);
    assert_eq!(human.experience_percent, 100);
    assert_eq!(human.shop_adjust_percent, 100);
    assert_eq!(human.base_hp, 20);
    assert!(human.tags.iter().any(|tag| tag == "standard-body"));
    assert_eq!(human.kin_category.as_deref(), Some("kin-glyph-112"));

    let human_skills = catalog
        .skill_set(&human.skill_set_id)
        .expect("formal Human skill set");
    assert_eq!(human_skills.entries.len(), 1);
    assert_eq!(human_skills.entries[0].skill_id, "demo.skill.perception");
    assert_eq!(human_skills.entries[0].base, 10);
    assert_eq!(human_skills.entries[0].growth_per_ten_levels, 0);

    for build_id in [
        "demo.build.warrior",
        "demo.build.high-mage-death",
        "demo.build.high-mage-arcane",
        "demo.build.high-mage-sorcery",
        "demo.build.high-mage-armageddon",
        "demo.build.archer",
        "demo.build.paladin-death",
        "demo.build.cavalry",
        "demo.build.sniper",
    ] {
        assert_eq!(
            catalog.build(build_id).expect("formal build").race_id,
            human.id,
            "{build_id} must continue to use the formal Human race"
        );
    }
}

#[test]
fn formal_half_orc_matches_rfb_profile_and_talent_pool() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let half_orc = catalog
        .race("rfb-legacy.race.half-orc")
        .expect("formal Half-Orc race");

    assert_eq!(half_orc.modifiers.strength, 2);
    assert_eq!(half_orc.modifiers.intelligence, -1);
    assert_eq!(half_orc.modifiers.wisdom, 0);
    assert_eq!(half_orc.modifiers.dexterity, 0);
    assert_eq!(half_orc.modifiers.constitution, 1);
    assert_eq!(half_orc.modifiers.charisma, -1);
    assert_eq!(half_orc.life_percent, 103);
    assert_eq!(half_orc.base_hp, 20);
    assert_eq!(half_orc.experience_percent, 110);
    assert_eq!(half_orc.shop_adjust_percent, 120);
    assert_eq!(half_orc.infravision, 3);
    assert_eq!(
        half_orc.resistances.get(&ActorDamageType::Dark),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(half_orc.tags.iter().any(|tag| tag == "rfb-compatibility"));

    let half_orc_skills = catalog
        .skill_set(&half_orc.skill_set_id)
        .expect("formal Half-Orc skill set");
    assert_eq!(
        half_orc_skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", -3),
            ("demo.skill.disarming", -3),
            ("demo.skill.melee", 20),
            ("demo.skill.perception", 5),
            ("demo.skill.ranged", -5),
            ("demo.skill.saving-throw", -1),
            ("demo.skill.search", -1),
            ("demo.skill.stealth", -2),
        ]
    );

    let human_talent = catalog
        .race("demo.race.rfb-human")
        .expect("formal Human race")
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "human-talent")
        .expect("Human talent");
    let half_orc_talent = half_orc
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "half-orc-talent")
        .expect("Half-Orc talent");
    assert_eq!(half_orc_talent.minimum_level, 30);
    assert_eq!(half_orc_talent.selection, human_talent.selection);
}

#[test]
fn formal_hobbit_matches_rfb_profile_and_create_food_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let hobbit = catalog
        .race("rfb-legacy.race.hobbit")
        .expect("formal Hobbit race");

    assert_eq!(
        [
            hobbit.modifiers.strength,
            hobbit.modifiers.intelligence,
            hobbit.modifiers.wisdom,
            hobbit.modifiers.dexterity,
            hobbit.modifiers.constitution,
            hobbit.modifiers.charisma,
        ],
        [-2, 1, 1, 3, 2, 1]
    );
    assert_eq!(hobbit.life_percent, 92);
    assert_eq!(hobbit.base_hp, 14);
    assert_eq!(hobbit.experience_percent, 120);
    assert_eq!(hobbit.shop_adjust_percent, 100);
    assert_eq!(hobbit.infravision, 4);
    assert_eq!(hobbit.kin_category.as_deref(), Some("kin-glyph-104"));
    assert!(hobbit.resistances.is_empty());
    assert!(hobbit.level_mutation_rewards.is_empty());
    assert_eq!(
        hobbit.abilities,
        [InnatePowerDefinition {
            minimum_level: 15,
            governing_attribute: TechniqueAttribute::Intelligence,
            cost: 10,
            cost_scaling: None,
            base_failure_percent: 50,
            minimum_failure_percent: None,
            ability_id: "rfb.ability.race.create-food".to_owned(),
        }]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(hobbit.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&hobbit.skill_set_id)
        .expect("formal Hobbit skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 8),
            ("demo.skill.disarming", 15),
            ("demo.skill.melee", -10),
            ("demo.skill.perception", 15),
            ("demo.skill.ranged", 10),
            ("demo.skill.saving-throw", 10),
            ("demo.skill.search", 12),
            ("demo.skill.stealth", 5),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.create-food")
        .expect("Hobbit Create Food ability");
    assert!(matches!(
        &ability.effect,
        AbilityEffectDefinition::CreateItem {
            item_kind_id,
            quantity: 1,
        } if item_kind_id == "demo.item.ration-of-food"
    ));
}

#[test]
fn formal_kobold_matches_rfb_profile_and_poison_dart_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let kobold = catalog
        .race("rfb-legacy.race.kobold")
        .expect("formal Kobold race");

    assert_eq!(
        [
            kobold.modifiers.strength,
            kobold.modifiers.intelligence,
            kobold.modifiers.wisdom,
            kobold.modifiers.dexterity,
            kobold.modifiers.constitution,
            kobold.modifiers.charisma,
        ],
        [1, -1, 0, 1, 0, -2]
    );
    assert_eq!(kobold.life_percent, 98);
    assert_eq!(kobold.base_hp, 19);
    assert_eq!(kobold.experience_percent, 90);
    assert_eq!(kobold.shop_adjust_percent, 120);
    assert_eq!(kobold.infravision, 3);
    assert_eq!(kobold.kin_category.as_deref(), Some("kin-glyph-107"));
    assert_eq!(
        kobold.resistances.get(&ActorDamageType::Poison),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(kobold.level_mutation_rewards.is_empty());
    assert_eq!(
        kobold.abilities,
        [InnatePowerDefinition {
            minimum_level: 12,
            governing_attribute: TechniqueAttribute::Dexterity,
            cost: 8,
            cost_scaling: None,
            base_failure_percent: 50,
            minimum_failure_percent: None,
            ability_id: "rfb.ability.race.poison-dart".to_owned(),
        }]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(kobold.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&kobold.skill_set_id)
        .expect("formal Kobold skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", -2),
            ("demo.skill.disarming", -2),
            ("demo.skill.melee", 10),
            ("demo.skill.perception", 8),
            ("demo.skill.ranged", 3),
            ("demo.skill.saving-throw", -1),
            ("demo.skill.search", 1),
            ("demo.skill.stealth", -1),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.poison-dart")
        .expect("Kobold Poison Dart ability");
    assert_eq!(ability.target.range, 18);
    assert!(!ability.affects_ground_items);
    assert!(ability.spell_power_fields.is_empty());
    assert_eq!(ability.level_scaling.len(), 1);
    assert_eq!(
        ability.level_scaling[0].field,
        AbilityLevelScalingField::DamageBonus
    );
    assert_eq!(ability.level_scaling[0].level_offset, 1);
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 1,
            damage_type: ActorDamageType::Poison,
            beam_chance_percent: 0,
            ..
        }
    ));
}

#[test]
fn formal_dwarf_matches_rfb_profile_and_detection_powers() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let dwarf = catalog
        .race("rfb-legacy.race.dwarf")
        .expect("formal Dwarf race");

    assert_eq!(
        [
            dwarf.modifiers.strength,
            dwarf.modifiers.intelligence,
            dwarf.modifiers.wisdom,
            dwarf.modifiers.dexterity,
            dwarf.modifiers.constitution,
            dwarf.modifiers.charisma,
        ],
        [2, -2, 2, -2, 2, 1]
    );
    assert_eq!(dwarf.life_percent, 103);
    assert_eq!(dwarf.base_hp, 22);
    assert_eq!(dwarf.experience_percent, 135);
    assert_eq!(dwarf.shop_adjust_percent, 115);
    assert_eq!(dwarf.infravision, 5);
    assert_eq!(dwarf.kin_category.as_deref(), Some("kin-glyph-104"));
    assert_eq!(
        dwarf.resistances.get(&ActorDamageType::Blindness),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(dwarf.level_mutation_rewards.is_empty());
    assert_eq!(
        dwarf.abilities,
        [
            InnatePowerDefinition {
                minimum_level: 5,
                governing_attribute: TechniqueAttribute::Wisdom,
                cost: 5,
                cost_scaling: None,
                base_failure_percent: 50,
                minimum_failure_percent: None,
                ability_id: "rfb.ability.race.detect-doors-stairs-traps".to_owned(),
            },
            InnatePowerDefinition {
                minimum_level: 10,
                governing_attribute: TechniqueAttribute::Charisma,
                cost: 5,
                cost_scaling: None,
                base_failure_percent: 50,
                minimum_failure_percent: None,
                ability_id: "rfb.ability.race.detect-treasure".to_owned(),
            },
        ]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(dwarf.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&dwarf.skill_set_id)
        .expect("formal Dwarf skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 5),
            ("demo.skill.disarming", 2),
            ("demo.skill.melee", 15),
            ("demo.skill.perception", 10),
            ("demo.skill.saving-throw", 6),
            ("demo.skill.search", 7),
            ("demo.skill.stealth", -1),
        ]
    );

    let doors = catalog
        .ability("rfb.ability.race.detect-doors-stairs-traps")
        .expect("Dwarf door and trap detection ability");
    let door_effects = doors.effect.ordered_effects();
    assert_eq!(door_effects.len(), 4);
    for (effect, expected_category) in
        door_effects
            .iter()
            .zip(["trap", "door", "stairs-down", "stairs-up"])
    {
        assert!(matches!(
            effect,
            AbilityEffectDefinition::Detect {
                subject: AbilityDetectSubjectDefinition::Terrain,
                category,
                radius: 30,
                persistent: true,
                through_walls: true,
            } if category == expected_category
        ));
    }

    let treasure = catalog
        .ability("rfb.ability.race.detect-treasure")
        .expect("Dwarf treasure detection ability");
    assert!(matches!(
        treasure.effect,
        AbilityEffectDefinition::Detect {
            subject: AbilityDetectSubjectDefinition::Terrain,
            ref category,
            radius: 30,
            persistent: true,
            through_walls: true,
        } if category == "treasure"
    ));
}

#[test]
fn formal_nibelung_matches_rfb_profile_and_reuses_detection_powers() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let nibelung = catalog
        .race("rfb-legacy.race.nibelung")
        .expect("formal Nibelung race");

    assert_eq!(
        [
            nibelung.modifiers.strength,
            nibelung.modifiers.intelligence,
            nibelung.modifiers.wisdom,
            nibelung.modifiers.dexterity,
            nibelung.modifiers.constitution,
            nibelung.modifiers.charisma,
        ],
        [0, 1, 0, 1, 1, -2]
    );
    assert_eq!(nibelung.life_percent, 101);
    assert_eq!(nibelung.base_hp, 21);
    assert_eq!(nibelung.experience_percent, 150);
    assert_eq!(nibelung.shop_adjust_percent, 115);
    assert_eq!(nibelung.infravision, 5);
    assert_eq!(nibelung.kin_category.as_deref(), Some("kin-glyph-104"));
    assert_eq!(
        nibelung.resistances.get(&ActorDamageType::Dark),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        nibelung.resistances.get(&ActorDamageType::Disenchant),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(nibelung.level_mutation_rewards.is_empty());
    assert_eq!(
        nibelung.abilities,
        [
            InnatePowerDefinition {
                minimum_level: 10,
                governing_attribute: TechniqueAttribute::Wisdom,
                cost: 5,
                cost_scaling: None,
                base_failure_percent: 50,
                minimum_failure_percent: None,
                ability_id: "rfb.ability.race.detect-doors-stairs-traps".to_owned(),
            },
            InnatePowerDefinition {
                minimum_level: 10,
                governing_attribute: TechniqueAttribute::Charisma,
                cost: 5,
                cost_scaling: None,
                base_failure_percent: 50,
                minimum_failure_percent: None,
                ability_id: "rfb.ability.race.detect-treasure".to_owned(),
            },
        ]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(nibelung.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&nibelung.skill_set_id)
        .expect("formal Nibelung skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 3),
            ("demo.skill.disarming", 3),
            ("demo.skill.melee", 10),
            ("demo.skill.perception", 10),
            ("demo.skill.saving-throw", 6),
            ("demo.skill.search", 5),
            ("demo.skill.stealth", 1),
        ]
    );

    for ability_id in [
        "rfb.ability.race.detect-doors-stairs-traps",
        "rfb.ability.race.detect-treasure",
    ] {
        assert!(catalog.ability(ability_id).is_some());
    }
}

#[test]
fn formal_gnome_matches_rfb_profile_and_uses_a_distinct_race_phase_door() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let gnome = catalog
        .race("rfb-legacy.race.gnome")
        .expect("formal Gnome race");

    assert_eq!(
        [
            gnome.modifiers.strength,
            gnome.modifiers.intelligence,
            gnome.modifiers.wisdom,
            gnome.modifiers.dexterity,
            gnome.modifiers.constitution,
            gnome.modifiers.charisma,
        ],
        [-1, 2, -1, 2, 1, -1]
    );
    assert_eq!(gnome.life_percent, 95);
    assert_eq!(gnome.base_hp, 16);
    assert_eq!(gnome.experience_percent, 115);
    assert_eq!(gnome.shop_adjust_percent, 115);
    assert_eq!(gnome.infravision, 4);
    assert_eq!(gnome.kin_category.as_deref(), Some("kin-glyph-104"));
    assert_eq!(gnome.status_immunities, ["rfb.status.paralysis"]);
    assert!(gnome.level_mutation_rewards.is_empty());
    assert_eq!(
        gnome.abilities,
        [InnatePowerDefinition {
            minimum_level: 5,
            governing_attribute: TechniqueAttribute::Intelligence,
            cost: 2,
            cost_scaling: None,
            base_failure_percent: 50,
            minimum_failure_percent: None,
            ability_id: "rfb.ability.race.phase-door".to_owned(),
        }]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(gnome.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&gnome.skill_set_id)
        .expect("formal Gnome skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 6),
            ("demo.skill.disarming", 10),
            ("demo.skill.melee", -8),
            ("demo.skill.perception", 13),
            ("demo.skill.ranged", 8),
            ("demo.skill.saving-throw", 7),
            ("demo.skill.search", 6),
            ("demo.skill.stealth", 3),
        ]
    );

    let phase_door = catalog
        .ability("rfb.ability.race.phase-door")
        .expect("Gnome race Phase Door ability");
    assert_ne!(phase_door.id, "demo.ability.sorcery-phase-door");
    assert!(matches!(
        phase_door.effect,
        AbilityEffectDefinition::BlinkSelf { radius: 10 }
    ));
}

#[test]
fn formal_half_giant_matches_rfb_profile_and_reuses_stone_to_mud() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let half_giant = catalog
        .race("rfb-legacy.race.half-giant")
        .expect("formal Half-Giant race");

    assert_eq!(
        [
            half_giant.modifiers.strength,
            half_giant.modifiers.intelligence,
            half_giant.modifiers.wisdom,
            half_giant.modifiers.dexterity,
            half_giant.modifiers.constitution,
            half_giant.modifiers.charisma,
        ],
        [4, -2, -2, -2, 3, 0]
    );
    assert_eq!(half_giant.life_percent, 108);
    assert_eq!(half_giant.base_hp, 26);
    assert_eq!(half_giant.experience_percent, 150);
    assert_eq!(half_giant.shop_adjust_percent, 125);
    assert_eq!(half_giant.infravision, 3);
    assert_eq!(half_giant.kin_category.as_deref(), Some("kin-glyph-80"));
    assert_eq!(
        half_giant.resistances.get(&ActorDamageType::Shards),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        half_giant
            .attribute_sustains
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [ItemAttributeDefinition::Strength]
    );
    assert!(half_giant.level_mutation_rewards.is_empty());
    assert_eq!(
        half_giant.abilities,
        [InnatePowerDefinition {
            minimum_level: 20,
            governing_attribute: TechniqueAttribute::Strength,
            cost: 10,
            cost_scaling: None,
            base_failure_percent: 70,
            minimum_failure_percent: None,
            ability_id: "rfb.ability.race.stone-to-mud".to_owned(),
        }]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(half_giant.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&half_giant.skill_set_id)
        .expect("formal Half-Giant skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", -5),
            ("demo.skill.disarming", -6),
            ("demo.skill.melee", 25),
            ("demo.skill.perception", 5),
            ("demo.skill.saving-throw", -3),
            ("demo.skill.search", -1),
            ("demo.skill.stealth", -2),
        ]
    );

    let stone_to_mud = catalog
        .ability("rfb.ability.race.stone-to-mud")
        .expect("Half-Giant Stone to Mud ability");
    assert!(matches!(
        stone_to_mud.effect,
        AbilityEffectDefinition::TerrainBeam {
            operation: AbilityTerrainBeamOperationDefinition::StoneToMud,
        }
    ));
}

#[test]
fn formal_half_troll_matches_rfb_profile_and_reuses_racial_berserk() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let half_troll = catalog
        .race("rfb-legacy.race.half-troll")
        .expect("formal Half-Troll race");

    assert_eq!(
        [
            half_troll.modifiers.strength,
            half_troll.modifiers.intelligence,
            half_troll.modifiers.wisdom,
            half_troll.modifiers.dexterity,
            half_troll.modifiers.constitution,
            half_troll.modifiers.charisma,
        ],
        [4, -4, -1, -3, 3, -2]
    );
    assert_eq!(half_troll.life_percent, 107);
    assert_eq!(half_troll.base_hp, 25);
    assert_eq!(half_troll.experience_percent, 150);
    assert_eq!(half_troll.shop_adjust_percent, 135);
    assert_eq!(half_troll.infravision, 3);
    assert_eq!(half_troll.kin_category.as_deref(), Some("kin-glyph-84"));
    assert_eq!(half_troll.regeneration_rate_modifier_percent, 100);
    assert_eq!(
        half_troll
            .attribute_sustains
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [ItemAttributeDefinition::Strength]
    );
    assert!(half_troll.level_mutation_rewards.is_empty());
    assert_eq!(
        half_troll.abilities,
        [InnatePowerDefinition {
            minimum_level: 10,
            governing_attribute: TechniqueAttribute::Strength,
            cost: 12,
            cost_scaling: None,
            base_failure_percent: 50,
            minimum_failure_percent: None,
            ability_id: "rfb.ability.race.berserk".to_owned(),
        }]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(half_troll.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&half_troll.skill_set_id)
        .expect("formal Half-Troll skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", -6),
            ("demo.skill.disarming", -5),
            ("demo.skill.melee", 20),
            ("demo.skill.perception", 5),
            ("demo.skill.ranged", -6),
            ("demo.skill.saving-throw", -5),
            ("demo.skill.search", -1),
            ("demo.skill.stealth", -2),
        ]
    );
}

#[test]
fn formal_half_titan_matches_rfb_profile_and_reuses_monster_probe() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let half_titan = catalog
        .race("rfb-legacy.race.half-titan")
        .expect("formal Half-Titan race");

    assert_eq!(
        [
            half_titan.modifiers.strength,
            half_titan.modifiers.intelligence,
            half_titan.modifiers.wisdom,
            half_titan.modifiers.dexterity,
            half_titan.modifiers.constitution,
            half_titan.modifiers.charisma,
        ],
        [5, 1, 2, -2, 3, 3]
    );
    assert_eq!(half_titan.life_percent, 110);
    assert_eq!(half_titan.base_hp, 28);
    assert_eq!(half_titan.experience_percent, 200);
    assert_eq!(half_titan.shop_adjust_percent, 90);
    assert_eq!(half_titan.infravision, 0);
    assert_eq!(half_titan.kin_category.as_deref(), Some("kin-glyph-80"));
    assert_eq!(
        half_titan.resistances.get(&ActorDamageType::Chaos),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(half_titan.level_mutation_rewards.is_empty());
    assert_eq!(
        half_titan.abilities,
        [InnatePowerDefinition {
            minimum_level: 15,
            governing_attribute: TechniqueAttribute::Intelligence,
            cost: 10,
            cost_scaling: None,
            base_failure_percent: 60,
            minimum_failure_percent: None,
            ability_id: "rfb.ability.race.probe-monsters".to_owned(),
        }]
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(half_titan.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&half_titan.skill_set_id)
        .expect("formal Half-Titan skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 3),
            ("demo.skill.disarming", -5),
            ("demo.skill.melee", 25),
            ("demo.skill.perception", 8),
            ("demo.skill.saving-throw", 1),
            ("demo.skill.search", 1),
            ("demo.skill.stealth", -2),
        ]
    );

    assert!(matches!(
        catalog
            .ability("rfb.ability.race.probe-monsters")
            .expect("Half-Titan monster probe ability")
            .effect,
        AbilityEffectDefinition::ProbeMonsters
    ));
}

#[test]
fn formal_cyclops_matches_rfb_profile_and_throw_boulder_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let cyclops = catalog
        .race("rfb-legacy.race.cyclops")
        .expect("formal Cyclops race");

    assert_eq!(
        [
            cyclops.modifiers.strength,
            cyclops.modifiers.intelligence,
            cyclops.modifiers.wisdom,
            cyclops.modifiers.dexterity,
            cyclops.modifiers.constitution,
            cyclops.modifiers.charisma,
        ],
        [4, -3, -2, -3, 4, -1]
    );
    assert_eq!(cyclops.life_percent, 108);
    assert_eq!(cyclops.base_hp, 24);
    assert_eq!(cyclops.experience_percent, 155);
    assert_eq!(cyclops.shop_adjust_percent, 135);
    assert_eq!(cyclops.infravision, 1);
    assert_eq!(cyclops.kin_category.as_deref(), Some("kin-glyph-80"));
    assert_eq!(
        cyclops.resistances.get(&ActorDamageType::Sound),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(cyclops.level_mutation_rewards.is_empty());
    assert_eq!(cyclops.abilities.len(), 1);
    let activation = &cyclops.abilities[0];
    assert_eq!(activation.minimum_level, 20);
    assert_eq!(activation.governing_attribute, TechniqueAttribute::Strength);
    assert_eq!(activation.cost, 0);
    assert_eq!(activation.base_failure_percent, 50);
    assert_eq!(activation.ability_id, "rfb.ability.race.throw-boulder");
    assert_eq!(
        activation.cost_scaling,
        Some(InnatePowerCostScalingDefinition {
            curve: InnatePowerCostScalingCurveDefinition::Prorated,
            start_level: 1,
            level_interval: 1,
            amount: 250,
            divisor: 7,
            round_up: true,
            linear_weight: 2,
            quadratic_weight: 1,
            cubic_weight: 2,
        })
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(cyclops.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&cyclops.skill_set_id)
        .expect("formal Cyclops skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", -3),
            ("demo.skill.disarming", -4),
            ("demo.skill.melee", 20),
            ("demo.skill.perception", 5),
            ("demo.skill.ranged", 10),
            ("demo.skill.saving-throw", -3),
            ("demo.skill.search", -2),
            ("demo.skill.stealth", -2),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.throw-boulder")
        .expect("Cyclops boulder ability");
    assert!(ability.affects_ground_items);
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 0,
            damage_type: ActorDamageType::Rock,
            beam_chance_percent: 0,
            ..
        }
    ));
    assert_eq!(ability.level_scaling.len(), 1);
    let scaling = &ability.level_scaling[0];
    assert_eq!(scaling.curve, AbilityLevelScalingCurveDefinition::Prorated);
    assert_eq!(scaling.linear_weight, 2);
    assert_eq!(scaling.quadratic_weight, 1);
    assert_eq!(scaling.cubic_weight, 2);
    assert_eq!(scaling.multiplier, 250);
}

#[test]
fn formal_yeek_matches_rfb_profile_acid_growth_and_scare_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let yeek = catalog
        .race("rfb-legacy.race.yeek")
        .expect("formal Yeek race");

    assert_eq!(
        [
            yeek.modifiers.strength,
            yeek.modifiers.intelligence,
            yeek.modifiers.wisdom,
            yeek.modifiers.dexterity,
            yeek.modifiers.constitution,
            yeek.modifiers.charisma,
        ],
        [-2, 1, -2, 1, -2, -4]
    );
    assert_eq!(
        (
            yeek.life_percent,
            yeek.base_hp,
            yeek.experience_percent,
            yeek.shop_adjust_percent,
            yeek.infravision,
        ),
        (92, 14, 70, 105, 2)
    );
    assert_eq!(yeek.kin_category.as_deref(), Some("kin-glyph-121"));
    assert_eq!(
        yeek.resistances.get(&ActorDamageType::Acid),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(yeek.level_resistances.len(), 1);
    assert_eq!(yeek.level_resistances[0].minimum_level, 20);
    assert_eq!(
        yeek.level_resistances[0]
            .resistances
            .get(&ActorDamageType::Acid),
        Some(&ActorResistanceLevel::Immune)
    );
    assert!(yeek.level_mutation_rewards.is_empty());
    assert_eq!(yeek.abilities.len(), 1);
    let activation = &yeek.abilities[0];
    assert_eq!(activation.minimum_level, 15);
    assert_eq!(activation.governing_attribute, TechniqueAttribute::Wisdom);
    assert_eq!(activation.cost, 15);
    assert_eq!(activation.base_failure_percent, 50);
    assert_eq!(activation.ability_id, "rfb.ability.race.scare-monster");
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(yeek.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&yeek.skill_set_id)
        .expect("formal Yeek skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 3),
            ("demo.skill.disarming", 2),
            ("demo.skill.melee", -5),
            ("demo.skill.perception", 15),
            ("demo.skill.ranged", -3),
            ("demo.skill.saving-throw", 6),
            ("demo.skill.search", 5),
            ("demo.skill.stealth", 3),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.scare-monster")
        .expect("Yeek scare ability");
    assert_eq!(
        ability.status_power_attribute,
        Some(ItemAttributeDefinition::Charisma)
    );
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::ApplyStatus {
            ref status_kind_id,
            intensity: 1,
            duration_ticks: 1,
            duration_dice: 3,
            duration_sides: 1,
            stacking: AbilityStatusStackingDefinition::Extend,
            power: Some(5),
            ..
        } if status_kind_id == "rfb.status.fear"
    ));
    assert_eq!(ability.level_scaling.len(), 3);
}

#[test]
fn formal_klackon_matches_rfb_profile_speed_and_acid_spit() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let klackon = catalog
        .race("rfb-legacy.race.klackon")
        .expect("formal Klackon race");

    assert_eq!(
        [
            klackon.modifiers.strength,
            klackon.modifiers.intelligence,
            klackon.modifiers.wisdom,
            klackon.modifiers.dexterity,
            klackon.modifiers.constitution,
            klackon.modifiers.charisma,
        ],
        [2, -1, -1, 1, 2, 1]
    );
    assert_eq!(
        (
            klackon.life_percent,
            klackon.base_hp,
            klackon.experience_percent,
            klackon.shop_adjust_percent,
            klackon.infravision,
        ),
        (105, 23, 170, 115, 2)
    );
    assert_eq!(
        klackon.level_stat_scalings,
        [RaceLevelStatScalingDefinition {
            stat: RaceLevelStatDefinition::Speed,
            multiplier: 1,
            divisor: 10,
        }]
    );
    assert_eq!(klackon.kin_category.as_deref(), Some("kin-glyph-75"));
    assert_eq!(
        klackon.resistances.get(&ActorDamageType::Acid),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        klackon.resistances.get(&ActorDamageType::Confusion),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(klackon.level_mutation_rewards.is_empty());
    assert_eq!(klackon.abilities.len(), 1);
    let activation = &klackon.abilities[0];
    assert_eq!(activation.minimum_level, 9);
    assert_eq!(
        activation.governing_attribute,
        TechniqueAttribute::Dexterity
    );
    assert_eq!(activation.cost, 9);
    assert_eq!(activation.base_failure_percent, 50);
    assert_eq!(activation.ability_id, "rfb.ability.race.spit-acid");
    assert_eq!(
        activation.cost_scaling,
        Some(InnatePowerCostScalingDefinition {
            curve: InnatePowerCostScalingCurveDefinition::Step,
            start_level: 5,
            level_interval: 5,
            amount: 1,
            divisor: 1,
            round_up: false,
            linear_weight: 1,
            quadratic_weight: 0,
            cubic_weight: 0,
        })
    );
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(klackon.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&klackon.skill_set_id)
        .expect("formal Klackon skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", -2),
            ("demo.skill.disarming", 10),
            ("demo.skill.melee", 5),
            ("demo.skill.perception", 10),
            ("demo.skill.ranged", 3),
            ("demo.skill.saving-throw", 3),
            ("demo.skill.search", -1),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.spit-acid")
        .expect("Klackon acid-spit ability");
    assert!(ability.affects_ground_items);
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::BoltOrAreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 1,
            damage_type: ActorDamageType::Acid,
            area_from_level: 25,
            radius: 2,
            ..
        }
    ));
    assert_eq!(ability.level_scaling.len(), 1);
}

#[test]
fn formal_golem_has_authoritative_level_scaled_intrinsics() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let golem = catalog
        .race("rfb-legacy.race.golem")
        .expect("formal Golem race");

    assert_eq!(golem.armor_class, 10);
    assert_eq!(golem.infravision, 4);
    assert!(golem.see_invisible);
    assert_eq!(golem.hold_life_minimum_level, Some(35));
    assert_eq!(
        golem.level_stat_scalings,
        [
            RaceLevelStatScalingDefinition {
                stat: RaceLevelStatDefinition::ArmorClass,
                multiplier: 2,
                divisor: 5,
            },
            RaceLevelStatScalingDefinition {
                stat: RaceLevelStatDefinition::Speed,
                multiplier: -1,
                divisor: 16,
            },
        ]
    );
    assert_eq!(
        golem.resistances.get(&ActorDamageType::Poison),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        golem.status_immunities,
        ["rfb.status.paralysis", "rfb.status.stun"]
    );
    assert!(golem.tags.iter().any(|tag| tag == "rfb-compatibility"));
}

#[test]
fn formal_golem_declares_construct_metabolism_tags() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let golem = catalog
        .race("rfb-legacy.race.golem")
        .expect("formal Golem race");

    assert_eq!(golem.food_nutrition_divisor, 20);
    for tag in ["device-eater", "nonliving", "slow-digestion"] {
        assert!(golem.tags.iter().any(|candidate| candidate == tag));
    }
}

#[test]
fn formal_golem_completes_the_authoritative_profile_stone_skin_and_birth_staff() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let golem = catalog
        .race("rfb-legacy.race.golem")
        .expect("formal Golem race");

    assert_eq!(
        [
            golem.modifiers.strength,
            golem.modifiers.intelligence,
            golem.modifiers.wisdom,
            golem.modifiers.dexterity,
            golem.modifiers.constitution,
            golem.modifiers.charisma,
        ],
        [4, -5, -5, -2, 4, 0]
    );
    assert_eq!(
        (
            golem.life_percent,
            golem.base_hp,
            golem.experience_percent,
            golem.infravision,
            golem.shop_adjust_percent,
        ),
        (105, 23, 185, 4, 120)
    );
    assert!(golem.tags.iter().any(|tag| tag == "rfb-compatibility"));
    let [activation] = golem.abilities.as_slice() else {
        panic!("Golem should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.golem-stone-skin",
            20,
            TechniqueAttribute::Constitution,
            20,
            50,
        )
    );
    let ability = catalog
        .ability(&activation.ability_id)
        .expect("Golem Stone Skin ability");
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::ApplyStatus {
            ref status_kind_id,
            intensity: 1,
            duration_ticks: 20,
            duration_dice: 1,
            duration_sides: 30,
            ref granted_modifiers,
            ..
        } if status_kind_id == "rfb.status.stone-skin" && granted_modifiers.defense == 10
    ));
    assert!(ability.spell_power_fields.is_empty());
    assert!(matches!(
        ability.level_scaling.as_slice(),
        [AbilityLevelScalingDefinition {
            effect_index: 0,
            field: AbilityLevelScalingField::StatusDefense,
            multiplier: 40,
            divisor: 50,
            ..
        }]
    ));

    let [starting_item] = golem.starting_items.as_slice() else {
        panic!("Golem should start with one race-specific item");
    };
    assert_eq!(starting_item.item_kind_id, "demo.item.staff-of-nothing");
    assert_eq!(starting_item.quantity, 1);
    assert!(!starting_item.equipped);
    assert!(starting_item.fully_charged);
    let staff = catalog
        .item(&starting_item.item_kind_id)
        .expect("Golem birth staff");
    let generation = staff
        .device_generation
        .as_ref()
        .expect("birth staff should use the device lifecycle");
    assert_eq!(
        generation.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 10,
            energy_per_mille: 10,
        })
    );
    let [staff_activation] = generation.activations.as_slice() else {
        panic!("birth staff should have one activation");
    };
    assert_eq!(
        staff_activation.charges,
        ItemDeviceChargeRangeDefinition {
            minimum: 21,
            maximum: 21,
            cost: 1,
        }
    );
    assert!(matches!(
        staff_activation.effect,
        ItemUseEffectDefinition::NoNumericEffect
    ));
}

#[test]
fn formal_zombie_completes_the_authoritative_profile_and_restore_life_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let zombie = catalog
        .race("rfb-legacy.race.zombie")
        .expect("formal Zombie race");

    assert_eq!(
        [
            zombie.modifiers.strength,
            zombie.modifiers.intelligence,
            zombie.modifiers.wisdom,
            zombie.modifiers.dexterity,
            zombie.modifiers.constitution,
            zombie.modifiers.charisma,
        ],
        [2, -6, -6, 1, 4, -3]
    );
    assert_eq!(
        (
            zombie.life_percent,
            zombie.base_hp,
            zombie.experience_percent,
            zombie.infravision,
            zombie.shop_adjust_percent,
        ),
        (108, 24, 180, 2, 140)
    );
    assert!(zombie.see_invisible);
    assert_eq!(zombie.hold_life_minimum_level, Some(1));
    assert_eq!(zombie.food_nutrition_divisor, 20);
    assert_eq!(
        zombie.resistances.get(&ActorDamageType::Nether),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        zombie.resistances.get(&ActorDamageType::Poison),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(zombie.level_resistances.len(), 1);
    assert_eq!(zombie.level_resistances[0].minimum_level, 5);
    assert_eq!(
        zombie.level_resistances[0]
            .resistances
            .get(&ActorDamageType::Cold),
        Some(&ActorResistanceLevel::Resistant)
    );
    for tag in [
        "device-eater",
        "night-start",
        "nonliving",
        "rfb-compatibility",
        "slow-digestion",
        "undead",
    ] {
        assert!(
            zombie.tags.iter().any(|candidate| candidate == tag),
            "{tag}"
        );
    }

    let skills = catalog
        .skill_set(&zombie.skill_set_id)
        .expect("formal Zombie skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [-5, -5, 5, -1, -1, 5, 15, 0]
    );

    let [activation] = zombie.abilities.as_slice() else {
        panic!("Zombie should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.restore-life",
            30,
            TechniqueAttribute::Wisdom,
            30,
            70,
        )
    );
    let ability = catalog
        .ability(&activation.ability_id)
        .expect("Zombie Restore Life ability");
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::RestoreVitality {
            life_force: 150,
            restore_attributes: false,
        }
    ));

    let [starting_item] = zombie.starting_items.as_slice() else {
        panic!("Zombie should start with one race-specific item");
    };
    assert_eq!(starting_item.item_kind_id, "demo.item.staff-of-nothing");
    assert_eq!(starting_item.quantity, 1);
    assert!(!starting_item.equipped);
    assert!(starting_item.fully_charged);
}

#[test]
fn formal_skeleton_completes_the_authoritative_profile_and_restore_life_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let skeleton = catalog
        .race("rfb-legacy.race.skeleton")
        .expect("formal Skeleton race");

    assert_eq!(
        [
            skeleton.modifiers.strength,
            skeleton.modifiers.intelligence,
            skeleton.modifiers.wisdom,
            skeleton.modifiers.dexterity,
            skeleton.modifiers.constitution,
            skeleton.modifiers.charisma,
        ],
        [0, 1, -2, 0, 1, 1]
    );
    assert_eq!(
        (
            skeleton.life_percent,
            skeleton.base_hp,
            skeleton.experience_percent,
            skeleton.infravision,
            skeleton.shop_adjust_percent,
        ),
        (100, 21, 115, 2, 125)
    );
    assert!(skeleton.see_invisible);
    assert_eq!(skeleton.hold_life_minimum_level, Some(1));
    assert_eq!(skeleton.food_nutrition_divisor, 20);
    assert_eq!(
        skeleton.resistances.get(&ActorDamageType::Shards),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        skeleton.resistances.get(&ActorDamageType::Poison),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(skeleton.level_resistances.len(), 1);
    assert_eq!(skeleton.level_resistances[0].minimum_level, 10);
    assert_eq!(
        skeleton.level_resistances[0]
            .resistances
            .get(&ActorDamageType::Cold),
        Some(&ActorResistanceLevel::Resistant)
    );
    for tag in [
        "device-eater",
        "night-start",
        "nonliving",
        "rfb-compatibility",
        "slow-digestion",
        "undead",
    ] {
        assert!(
            skeleton.tags.iter().any(|candidate| candidate == tag),
            "{tag}"
        );
    }

    let skills = catalog
        .skill_set(&skeleton.skill_set_id)
        .expect("formal Skeleton skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [-5, 0, 3, -1, -1, 8, 10, 0]
    );

    let [activation] = skeleton.abilities.as_slice() else {
        panic!("Skeleton should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.restore-life",
            30,
            TechniqueAttribute::Wisdom,
            30,
            70,
        )
    );
    let [starting_item] = skeleton.starting_items.as_slice() else {
        panic!("Skeleton should start with one race-specific item");
    };
    assert_eq!(starting_item.item_kind_id, "demo.item.staff-of-nothing");
    assert_eq!(starting_item.quantity, 1);
    assert!(!starting_item.equipped);
    assert!(starting_item.fully_charged);
}

#[test]
fn formal_wood_elf_completes_the_authoritative_profile_and_nature_awareness_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let wood_elf = catalog
        .race("rfb-legacy.race.wood-elf")
        .expect("formal Wood-Elf race");

    assert_eq!(
        [
            wood_elf.modifiers.strength,
            wood_elf.modifiers.intelligence,
            wood_elf.modifiers.wisdom,
            wood_elf.modifiers.dexterity,
            wood_elf.modifiers.constitution,
            wood_elf.modifiers.charisma,
        ],
        [-1, 1, 2, 1, -1, 1]
    );
    assert_eq!(
        (
            wood_elf.life_percent,
            wood_elf.base_hp,
            wood_elf.experience_percent,
            wood_elf.infravision,
            wood_elf.shop_adjust_percent,
        ),
        (97, 16, 125, 3, 95)
    );
    for tag in [
        "forest-adapted",
        "humanoid",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(
            wood_elf.tags.iter().any(|candidate| candidate == tag),
            "{tag}"
        );
    }

    let skills = catalog
        .skill_set(&wood_elf.skill_set_id)
        .expect("formal Wood-Elf skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [5, 4, 4, 3, 8, 12, -5, 12]
    );

    let [activation] = wood_elf.abilities.as_slice() else {
        panic!("Wood-Elf should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.wood-elf-nature-awareness",
            20,
            TechniqueAttribute::Wisdom,
            15,
            50,
        )
    );
    let ability = catalog
        .ability(&activation.ability_id)
        .expect("Wood-Elf Nature Awareness ability");
    assert!(ability.tags.iter().any(|tag| tag == "nature"));
    assert!(wood_elf.starting_items.is_empty());
}

#[test]
fn formal_archon_completes_the_authoritative_profile_and_static_passives() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let archon = catalog
        .race("rfb-legacy.race.archon")
        .expect("formal Archon race");

    assert_eq!(
        [
            archon.modifiers.strength,
            archon.modifiers.intelligence,
            archon.modifiers.wisdom,
            archon.modifiers.dexterity,
            archon.modifiers.constitution,
            archon.modifiers.charisma,
        ],
        [2, 0, 4, 1, 2, 3]
    );
    assert_eq!(
        (
            archon.life_percent,
            archon.base_hp,
            archon.experience_percent,
            archon.infravision,
            archon.shop_adjust_percent,
        ),
        (103, 22, 200, 3, 90)
    );
    assert!(archon.levitation);
    assert!(archon.see_invisible);
    assert_eq!(archon.body_slots.len(), 15);
    for tag in ["angel", "rfb-compatibility", "standard-body"] {
        assert!(
            archon.tags.iter().any(|candidate| candidate == tag),
            "{tag}"
        );
    }
    assert!(!archon.tags.iter().any(|tag| tag == "good"));

    let skills = catalog
        .skill_set(&archon.skill_set_id)
        .expect("formal Archon skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [0, 8, 8, 2, 2, 11, 10, 7]
    );
    assert!(archon.abilities.is_empty());
    assert!(archon.starting_items.is_empty());
}

#[test]
fn formal_sprite_completes_the_authoritative_profile_and_sleeping_dust_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let sprite = catalog
        .race("rfb-legacy.race.sprite")
        .expect("formal Sprite race");

    assert_eq!(
        [
            sprite.modifiers.strength,
            sprite.modifiers.intelligence,
            sprite.modifiers.wisdom,
            sprite.modifiers.dexterity,
            sprite.modifiers.constitution,
            sprite.modifiers.charisma,
        ],
        [-4, 3, 3, 3, -2, -2],
    );
    assert_eq!(
        (
            sprite.life_percent,
            sprite.base_hp,
            sprite.experience_percent,
            sprite.infravision,
            sprite.shop_adjust_percent,
        ),
        (92, 14, 135, 4, 90),
    );
    assert!(sprite.levitation);
    assert_eq!(
        sprite.resistances.get(&ActorDamageType::Light),
        Some(&ActorResistanceLevel::Resistant),
    );
    assert_eq!(
        sprite.level_stat_scalings,
        [RaceLevelStatScalingDefinition {
            stat: RaceLevelStatDefinition::Speed,
            multiplier: 1,
            divisor: 10,
        }],
    );
    assert_eq!(sprite.body_slots.len(), 15);
    for tag in [
        "humanoid",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(
            sprite.tags.iter().any(|candidate| candidate == tag),
            "{tag}"
        );
    }

    let skills = catalog
        .skill_set(&sprite.skill_set_id)
        .expect("formal Sprite skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [10, 6, 6, 4, 10, 10, -12, 0],
    );

    let [activation] = sprite.abilities.as_slice() else {
        panic!("Sprite should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.sleeping-dust",
            12,
            TechniqueAttribute::Intelligence,
            12,
            50,
        ),
    );
    let ability = catalog
        .ability(&activation.ability_id)
        .expect("Sprite Sleeping Dust ability");
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::SleepingDust {
            visible_at_level: 25,
        }
    ));
    assert!(sprite.starting_items.is_empty());
}

#[test]
fn formal_snotling_completes_the_authoritative_profile_power_and_birth_mushrooms() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let snotling = catalog
        .race("rfb-legacy.race.snotling")
        .expect("formal Snotling race");

    assert_eq!(
        [
            snotling.modifiers.strength,
            snotling.modifiers.intelligence,
            snotling.modifiers.wisdom,
            snotling.modifiers.dexterity,
            snotling.modifiers.constitution,
            snotling.modifiers.charisma,
        ],
        [-2, -2, -2, -2, -2, -5],
    );
    assert_eq!(
        (
            snotling.life_percent,
            snotling.base_hp,
            snotling.experience_percent,
            snotling.infravision,
            snotling.shop_adjust_percent,
        ),
        (85, 10, 45, 2, 125),
    );
    assert_eq!(snotling.body_slots.len(), 15);
    for tag in [
        "humanoid",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(
            snotling.tags.iter().any(|candidate| candidate == tag),
            "{tag}",
        );
    }

    let skills = catalog
        .skill_set(&snotling.skill_set_id)
        .expect("formal Snotling skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [-3, -2, -2, 2, 0, 7, -10, -5],
    );

    let [activation] = snotling.abilities.as_slice() else {
        panic!("Snotling should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.devour-flesh",
            1,
            TechniqueAttribute::Charisma,
            0,
            0,
        ),
    );
    let ability = catalog
        .ability(&activation.ability_id)
        .expect("Snotling Devour Flesh ability");
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::DevourFlesh {
            maximum_hp_divisor: 3,
            bleeding_amount: 100,
        }
    ));
    assert!(
        ability
            .tags
            .iter()
            .any(|tag| tag == "usable-while-confused")
    );
    assert_eq!(
        snotling.starting_items,
        [StartingItemDefinition {
            item_kind_id: "demo.item.fast-recovery-mushroom".to_owned(),
            quantity: 1,
            maximum_quantity: Some(3),
            equipped: false,
            fully_charged: false,
        }],
    );
}

#[test]
fn formal_boit_completes_the_authoritative_profile_throwing_bonus_and_vomit_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let boit = catalog
        .race("rfb-legacy.race.boit")
        .expect("formal Boit race");

    assert_eq!(
        [
            boit.modifiers.strength,
            boit.modifiers.intelligence,
            boit.modifiers.wisdom,
            boit.modifiers.dexterity,
            boit.modifiers.constitution,
            boit.modifiers.charisma,
        ],
        [-1, -2, -2, -2, 0, -2],
    );
    assert_eq!(boit.modifiers.speed, 2);
    assert_eq!(
        (
            boit.life_percent,
            boit.base_hp,
            boit.experience_percent,
            boit.infravision,
            boit.shop_adjust_percent,
        ),
        (95, 15, 80, 1, 105),
    );
    assert_eq!(boit.body_slots.len(), 15);
    for tag in [
        "humanoid",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(boit.tags.iter().any(|candidate| candidate == tag), "{tag}");
    }

    let skills = catalog
        .skill_set(&boit.skill_set_id)
        .expect("formal Boit skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
            skills.get("demo.skill.throwing").copied().unwrap_or(0),
        ],
        [2, -5, -1, 0, 0, 10, -8, -8, 25],
    );

    let [activation] = boit.abilities.as_slice() else {
        panic!("Boit should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.vomit",
            1,
            TechniqueAttribute::Strength,
            0,
            0,
        ),
    );
    let ability = catalog
        .ability(&activation.ability_id)
        .expect("Boit Vomit ability");
    assert!(matches!(ability.effect, AbilityEffectDefinition::Vomit));
    for tag in ["usable-while-afraid", "usable-while-confused"] {
        assert!(
            ability.tags.iter().any(|candidate| candidate == tag),
            "{tag}",
        );
    }
    assert!(boit.starting_items.is_empty());
}

#[test]
fn formal_einheri_matches_the_authoritative_profile_healing_penalty_and_talent_pool() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let einheri = catalog
        .race("rfb-legacy.race.einheri")
        .expect("formal Einheri race");

    assert_eq!(
        [
            einheri.modifiers.strength,
            einheri.modifiers.intelligence,
            einheri.modifiers.wisdom,
            einheri.modifiers.dexterity,
            einheri.modifiers.constitution,
            einheri.modifiers.charisma,
        ],
        [2, 0, 0, 2, 1, 1],
    );
    assert_eq!(
        (
            einheri.life_percent,
            einheri.base_hp,
            einheri.experience_percent,
            einheri.infravision,
            einheri.shop_adjust_percent,
            einheri.hold_life_minimum_level,
            einheri.regeneration_rate_modifier_percent,
            einheri.healing_received_percent,
        ),
        (113, 44, 160, 3, 100, Some(1), 100, 50),
    );
    assert_eq!(einheri.body_slots.len(), 15);
    for tag in [
        "nonliving",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
        "undead",
    ] {
        assert!(
            einheri.tags.iter().any(|candidate| candidate == tag),
            "{tag}",
        );
    }

    let skills = catalog
        .skill_set(&einheri.skill_set_id)
        .expect("formal Einheri skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [5, 3, 8, -1, 7, 10, 22, 8],
    );

    let [activation] = einheri.abilities.as_slice() else {
        panic!("Einheri should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.berserk",
            1,
            TechniqueAttribute::Strength,
            10,
            50,
        ),
    );
    let talent = einheri
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "einheri-talent")
        .expect("Einheri talent");
    let human_talent = catalog
        .race("demo.race.rfb-human")
        .expect("formal Human race")
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "human-talent")
        .expect("Human talent");
    assert_eq!(talent.minimum_level, 30);
    assert_eq!(talent.selection, human_talent.selection);
    assert!(einheri.starting_items.is_empty());
}

#[test]
fn formal_kutar_matches_the_authoritative_profile_and_expansion_power() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let kutar = catalog
        .race("rfb-legacy.race.kutar")
        .expect("formal Kutar race");

    assert_eq!(
        [
            kutar.modifiers.strength,
            kutar.modifiers.intelligence,
            kutar.modifiers.wisdom,
            kutar.modifiers.dexterity,
            kutar.modifiers.constitution,
            kutar.modifiers.charisma,
        ],
        [0, -1, -1, 1, 2, 2],
    );
    assert_eq!(
        (
            kutar.life_percent,
            kutar.base_hp,
            kutar.experience_percent,
            kutar.infravision,
            kutar.shop_adjust_percent,
        ),
        (102, 21, 175, 0, 95),
    );
    assert_eq!(
        kutar.resistances.get(&ActorDamageType::Confusion),
        Some(&ActorResistanceLevel::Resistant),
    );
    assert_eq!(kutar.body_slots.len(), 15);
    for tag in [
        "humanoid",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(kutar.tags.iter().any(|candidate| candidate == tag), "{tag}");
    }

    let skills = catalog
        .skill_set(&kutar.skill_set_id)
        .expect("formal Kutar skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [-2, 3, 5, 5, -2, 6, 0, -3],
    );

    let [activation] = kutar.abilities.as_slice() else {
        panic!("Kutar should have one racial power");
    };
    assert_eq!(
        (
            activation.ability_id.as_str(),
            activation.minimum_level,
            activation.governing_attribute,
            activation.cost,
            activation.base_failure_percent,
        ),
        (
            "rfb.ability.race.kutar-expand",
            20,
            TechniqueAttribute::Charisma,
            15,
            70,
        ),
    );
    let ability = catalog
        .ability(&activation.ability_id)
        .expect("Kutar Expand Horizontally ability");
    assert!(matches!(
        &ability.effect,
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            duration_ticks: 30,
            duration_dice: 1,
            duration_sides: 20,
            granted_modifiers,
            granted_equipment_bonuses,
            ..
        } if status_kind_id == "rfb.status.kutar-expand"
            && granted_modifiers.defense == 35
            && granted_equipment_bonuses.saving_throw_skill_override == Some(10)
    ));
    assert!(kutar.starting_items.is_empty());
}

#[test]
fn formal_amberite_matches_the_authoritative_profile_and_powers() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let amberite = catalog
        .race("rfb-legacy.race.amberite")
        .expect("formal Amberite race");

    assert_eq!(
        [
            amberite.modifiers.strength,
            amberite.modifiers.intelligence,
            amberite.modifiers.wisdom,
            amberite.modifiers.dexterity,
            amberite.modifiers.constitution,
            amberite.modifiers.charisma,
        ],
        [1, 2, 2, 2, 3, 0],
    );
    assert_eq!(
        (
            amberite.life_percent,
            amberite.base_hp,
            amberite.experience_percent,
            amberite.infravision,
            amberite.shop_adjust_percent,
            amberite.regeneration_rate_modifier_percent,
        ),
        (100, 20, 190, 0, 100, 100),
    );
    assert!(
        amberite
            .attribute_sustains
            .contains(&ItemAttributeDefinition::Constitution)
    );
    assert_eq!(amberite.body_slots.len(), 15);
    for tag in [
        "humanoid",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(
            amberite.tags.iter().any(|candidate| candidate == tag),
            "{tag}",
        );
    }

    let skills = catalog
        .skill_set(&amberite.skill_set_id)
        .expect("formal Amberite skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [4, 3, 3, 2, 3, 13, 15, 7],
    );

    let [shadow_shifting, pattern_mindwalk] = amberite.abilities.as_slice() else {
        panic!("Amberite should have two racial powers");
    };
    assert_eq!(
        (
            shadow_shifting.ability_id.as_str(),
            shadow_shifting.minimum_level,
            shadow_shifting.governing_attribute,
            shadow_shifting.cost,
            shadow_shifting.base_failure_percent,
        ),
        (
            "rfb.ability.race.amberite-shadow-shifting",
            30,
            TechniqueAttribute::Intelligence,
            50,
            70,
        ),
    );
    assert_eq!(
        (
            pattern_mindwalk.ability_id.as_str(),
            pattern_mindwalk.minimum_level,
            pattern_mindwalk.governing_attribute,
            pattern_mindwalk.cost,
            pattern_mindwalk.base_failure_percent,
        ),
        (
            "rfb.ability.race.amberite-pattern-mindwalk",
            40,
            TechniqueAttribute::Wisdom,
            75,
            75,
        ),
    );
    assert!(matches!(
        catalog
            .ability(&shadow_shifting.ability_id)
            .expect("Amberite Shadow Shifting ability")
            .effect,
        AbilityEffectDefinition::AlterReality
    ));
    let pattern = catalog
        .ability(&pattern_mindwalk.ability_id)
        .expect("Amberite Pattern Mindwalking ability");
    let AbilityEffectDefinition::Sequence { effects } = &pattern.effect else {
        panic!("Pattern Mindwalking should be an ordered restoration sequence");
    };
    assert_eq!(effects.len(), 7);
    assert!(matches!(
        effects.last(),
        Some(AbilityEffectDefinition::RestoreVitality {
            life_force: 1000,
            restore_attributes: true,
        })
    ));
    assert!(amberite.starting_items.is_empty());
}

#[test]
fn formal_beastman_matches_the_authoritative_static_profile() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let beastman = catalog
        .race("rfb-legacy.race.beastman")
        .expect("formal Beastman race");

    assert_eq!(
        [
            beastman.modifiers.strength,
            beastman.modifiers.intelligence,
            beastman.modifiers.wisdom,
            beastman.modifiers.dexterity,
            beastman.modifiers.constitution,
            beastman.modifiers.charisma,
        ],
        [2, -2, -1, -1, 2, 1],
    );
    assert_eq!(
        (
            beastman.life_percent,
            beastman.base_hp,
            beastman.experience_percent,
            beastman.infravision,
            beastman.shop_adjust_percent,
        ),
        (102, 22, 150, 0, 130),
    );
    assert_eq!(
        beastman.resistances.get(&ActorDamageType::Confusion),
        Some(&ActorResistanceLevel::Resistant),
    );
    assert_eq!(
        beastman.resistances.get(&ActorDamageType::Sound),
        Some(&ActorResistanceLevel::Resistant),
    );
    assert_eq!(beastman.body_slots.len(), 15);
    for tag in [
        "humanoid",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(
            beastman.tags.iter().any(|candidate| candidate == tag),
            "{tag}",
        );
    }

    let skills = catalog
        .skill_set(&beastman.skill_set_id)
        .expect("formal Beastman skill set")
        .entries
        .iter()
        .map(|entry| (entry.skill_id.as_str(), entry.base))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        [
            skills.get("demo.skill.disarming").copied().unwrap_or(0),
            skills.get("demo.skill.device").copied().unwrap_or(0),
            skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
            skills.get("demo.skill.stealth").copied().unwrap_or(0),
            skills.get("demo.skill.search").copied().unwrap_or(0),
            skills.get("demo.skill.perception").copied().unwrap_or(0),
            skills.get("demo.skill.melee").copied().unwrap_or(0),
            skills.get("demo.skill.ranged").copied().unwrap_or(0),
        ],
        [-5, -1, -1, -1, -1, 5, 12, 3],
    );
    assert!(beastman.abilities.is_empty());
    assert!(beastman.starting_items.is_empty());
}

#[test]
fn formal_dark_elf_matches_rfb_profile_passives_and_magic_missile() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let dark_elf = catalog
        .race("rfb-legacy.race.dark-elf")
        .expect("formal Dark-Elf race");

    assert_eq!(
        [
            dark_elf.modifiers.strength,
            dark_elf.modifiers.intelligence,
            dark_elf.modifiers.wisdom,
            dark_elf.modifiers.dexterity,
            dark_elf.modifiers.constitution,
            dark_elf.modifiers.charisma,
        ],
        [-1, 3, 2, 2, -2, 3]
    );
    assert_eq!(
        (
            dark_elf.life_percent,
            dark_elf.base_hp,
            dark_elf.experience_percent,
            dark_elf.shop_adjust_percent,
            dark_elf.infravision,
            dark_elf.see_invisible_minimum_level,
            dark_elf.spell_capacity_bonus,
        ),
        (97, 18, 155, 120, 5, Some(20), 3)
    );
    assert_eq!(dark_elf.kin_category.as_deref(), Some("kin-glyph-104"));
    assert_eq!(
        dark_elf.resistances.get(&ActorDamageType::Dark),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(dark_elf.level_mutation_rewards.is_empty());
    assert_eq!(dark_elf.abilities.len(), 1);
    let activation = &dark_elf.abilities[0];
    assert_eq!(activation.minimum_level, 1);
    assert_eq!(
        activation.governing_attribute,
        TechniqueAttribute::Intelligence
    );
    assert_eq!(activation.cost, 2);
    assert_eq!(activation.base_failure_percent, 30);
    assert_eq!(activation.ability_id, "rfb.ability.race.magic-missile");
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(dark_elf.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&dark_elf.skill_set_id)
        .expect("formal Dark-Elf skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 7),
            ("demo.skill.disarming", 5),
            ("demo.skill.melee", -5),
            ("demo.skill.perception", 12),
            ("demo.skill.ranged", 6),
            ("demo.skill.saving-throw", 12),
            ("demo.skill.search", 8),
            ("demo.skill.stealth", 3),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.magic-missile")
        .expect("Dark-Elf magic missile ability");
    assert!(ability.affects_ground_items);
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice: 3,
            damage_sides: 4,
            damage_bonus: 0,
            damage_type: ActorDamageType::Physical,
            beam_chance_percent: 0,
            beam_chance_modifier: 0,
        }
    ));
    assert_eq!(ability.level_scaling.len(), 2);
    assert_eq!(ability.spell_power_fields.len(), 1);
    assert!(
        ability
            .tags
            .iter()
            .any(|tag| tag == "uses-casting-profile-offense")
    );
}

#[test]
fn formal_mindflayer_matches_rfb_profile_senses_and_mind_blast() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let mindflayer = catalog
        .race("rfb-legacy.race.mindflayer")
        .expect("formal Mindflayer race");

    assert_eq!(
        [
            mindflayer.modifiers.strength,
            mindflayer.modifiers.intelligence,
            mindflayer.modifiers.wisdom,
            mindflayer.modifiers.dexterity,
            mindflayer.modifiers.constitution,
            mindflayer.modifiers.charisma,
        ],
        [-3, 4, 4, 0, -2, -1]
    );
    assert_eq!(
        (
            mindflayer.life_percent,
            mindflayer.base_hp,
            mindflayer.experience_percent,
            mindflayer.shop_adjust_percent,
            mindflayer.infravision,
            mindflayer.see_invisible_minimum_level,
            mindflayer.telepathy_minimum_level,
        ),
        (97, 18, 150, 115, 4, Some(15), Some(30))
    );
    assert_eq!(mindflayer.kin_category.as_deref(), Some("kin-glyph-104"));
    assert_eq!(
        mindflayer
            .attribute_sustains
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [
            ItemAttributeDefinition::Intelligence,
            ItemAttributeDefinition::Wisdom,
        ]
    );
    assert!(mindflayer.level_mutation_rewards.is_empty());
    assert_eq!(mindflayer.abilities.len(), 1);
    let activation = &mindflayer.abilities[0];
    assert_eq!(activation.minimum_level, 5);
    assert_eq!(
        activation.governing_attribute,
        TechniqueAttribute::Intelligence
    );
    assert_eq!(activation.cost, 3);
    assert_eq!(activation.base_failure_percent, 50);
    assert_eq!(activation.ability_id, "rfb.ability.race.mind-blast");
    for tag in [
        "humanoid",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(mindflayer.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&mindflayer.skill_set_id)
        .expect("formal Mindflayer skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 11),
            ("demo.skill.disarming", 10),
            ("demo.skill.melee", -10),
            ("demo.skill.perception", 12),
            ("demo.skill.ranged", -5),
            ("demo.skill.saving-throw", 9),
            ("demo.skill.search", 5),
            ("demo.skill.stealth", 2),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.mind-blast")
        .expect("Mindflayer mind-blast ability");
    assert!(ability.affects_ground_items);
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::Damage {
            damage_dice: 3,
            damage_sides: 3,
            damage_bonus: 0,
            damage_type: ActorDamageType::Psi,
        }
    ));
    assert_eq!(ability.level_scaling.len(), 1);
    assert_eq!(ability.spell_power_fields.len(), 1);
    assert!(
        ability
            .tags
            .iter()
            .any(|tag| tag == "uses-casting-profile-offense")
    );
}

#[test]
fn formal_imp_matches_rfb_profile_demon_identity_and_fire_upgrade() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let imp = catalog
        .race("rfb-legacy.race.imp")
        .expect("formal Imp race");

    assert_eq!(
        [
            imp.modifiers.strength,
            imp.modifiers.intelligence,
            imp.modifiers.wisdom,
            imp.modifiers.dexterity,
            imp.modifiers.constitution,
            imp.modifiers.charisma,
        ],
        [0, -1, -1, 1, 2, -1]
    );
    assert_eq!(
        (
            imp.life_percent,
            imp.base_hp,
            imp.experience_percent,
            imp.shop_adjust_percent,
            imp.infravision,
            imp.see_invisible_minimum_level,
        ),
        (99, 19, 90, 120, 3, Some(10))
    );
    assert_eq!(imp.kin_category.as_deref(), Some("kin-glyph-117"));
    assert_eq!(
        imp.resistances.get(&ActorDamageType::Fire),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(imp.abilities.len(), 1);
    let activation = &imp.abilities[0];
    assert_eq!(activation.minimum_level, 9);
    assert_eq!(
        activation.governing_attribute,
        TechniqueAttribute::Intelligence
    );
    assert_eq!((activation.cost, activation.base_failure_percent), (8, 50));
    assert_eq!(activation.ability_id, "rfb.ability.race.imp-fire");
    assert_eq!(
        activation.cost_scaling,
        Some(InnatePowerCostScalingDefinition {
            curve: InnatePowerCostScalingCurveDefinition::Step,
            start_level: 30,
            level_interval: 100,
            amount: 7,
            divisor: 1,
            round_up: false,
            linear_weight: 1,
            quadratic_weight: 0,
            cubic_weight: 0,
        })
    );
    for tag in [
        "demon",
        "legacy-import",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(imp.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&imp.skill_set_id)
        .expect("formal Imp skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 1),
            ("demo.skill.disarming", -3),
            ("demo.skill.melee", 5),
            ("demo.skill.perception", 10),
            ("demo.skill.ranged", -3),
            ("demo.skill.saving-throw", -1),
            ("demo.skill.search", -1),
            ("demo.skill.stealth", 1),
        ]
    );

    let ability = catalog
        .ability("rfb.ability.race.imp-fire")
        .expect("Imp fire ability");
    assert!(ability.affects_ground_items);
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::BoltOrAreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 0,
            damage_type: ActorDamageType::Fire,
            area_from_level: 30,
            area_damage_multiplier: 2,
            radius: 2,
        }
    ));
    assert_eq!(ability.level_scaling.len(), 1);
    assert_eq!(ability.spell_power_fields.len(), 1);
    assert!(
        !ability
            .tags
            .iter()
            .any(|tag| tag == "uses-casting-profile-offense")
    );
}

#[test]
fn draconian_subraces_match_rfb_profiles_and_breath_bindings() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let profiles = [
        (
            "red",
            [3, 0, 0, 1, 2, 2],
            (104, 190, 115, 2),
            ActorDamageType::Fire,
            (25, 1, 2_500, 500),
            [-2, -2, 2, -1, 1, 10, 15, 1],
            (0, None),
        ),
        (
            "white",
            [3, 0, 0, 1, 2, 2],
            (104, 190, 115, 2),
            ActorDamageType::Cold,
            (25, 1, 2_500, 500),
            [-2, -2, 2, -1, 1, 10, 14, 1],
            (0, None),
        ),
        (
            "blue",
            [2, 1, 1, 1, 2, 2],
            (103, 190, 110, 2),
            ActorDamageType::Electricity,
            (25, 1, 2_500, 500),
            [-2, -1, 2, 0, 1, 10, 12, 1],
            (0, None),
        ),
        (
            "black",
            [2, 1, 1, 1, 2, 2],
            (103, 190, 110, 2),
            ActorDamageType::Acid,
            (25, 1, 2_500, 500),
            [-2, -1, 2, 0, 1, 10, 13, 1],
            (0, None),
        ),
        (
            "green",
            [1, 1, 1, 1, 2, 2],
            (101, 205, 105, 2),
            ActorDamageType::Poison,
            (25, 1, 2_500, 500),
            [-2, 1, 2, 1, 1, 10, 5, 1],
            (0, None),
        ),
        (
            "bronze",
            [1, 2, 1, 1, 2, 2],
            (101, 215, 100, 2),
            ActorDamageType::Confusion,
            (17, 25, 125_000, 300),
            [-2, 6, 3, 1, 1, 10, 3, 1],
            (0, None),
        ),
        (
            "crystal",
            [1, -1, 1, 0, 3, 2],
            (103, 250, 105, 2),
            ActorDamageType::Shards,
            (20, 30, 125_000, 350),
            [-2, -2, 2, 0, 1, 10, 12, 1],
            (10, Some(40)),
        ),
        (
            "gold",
            [1, 1, 2, 1, 2, 2],
            (102, 220, 95, 2),
            ActorDamageType::Sound,
            (20, 30, 125_000, 350),
            [-2, 4, 5, 1, 1, 10, 5, 1],
            (0, None),
        ),
        (
            "shadow",
            [0, 1, 1, 3, 2, 2],
            (100, 225, 105, 4),
            ActorDamageType::Nether,
            (20, 35, 125_000, 400),
            [-2, 3, 2, 4, 1, 10, 0, 1],
            (0, None),
        ),
    ];

    for (suffix, modifiers, profile, damage_type, breath, expected_skills, passives) in profiles {
        let race_id = format!("rfb-legacy.race.draconian-{suffix}");
        let race = catalog
            .race(&race_id)
            .unwrap_or_else(|| panic!("{race_id}"));
        assert_eq!(
            [
                race.modifiers.strength,
                race.modifiers.intelligence,
                race.modifiers.wisdom,
                race.modifiers.dexterity,
                race.modifiers.constitution,
                race.modifiers.charisma,
            ],
            modifiers,
            "{suffix}"
        );
        assert_eq!(
            (
                race.life_percent,
                race.experience_percent,
                race.shop_adjust_percent,
                race.infravision
            ),
            profile,
            "{suffix}"
        );
        assert_eq!(race.base_hp, 22, "{suffix}");
        assert_eq!(race.kin_category.as_deref(), Some("kin-glyph-100"));
        assert_eq!(
            race.resistances,
            BTreeMap::from([(damage_type, ActorResistanceLevel::Resistant)]),
            "{suffix}"
        );
        assert!(race.levitation, "{suffix}");
        assert_eq!(race.armor_class, passives.0, "{suffix}");
        assert_eq!(race.reflects_bolts_minimum_level, passives.1, "{suffix}");
        assert_eq!(race.body_slots.len(), 15, "{suffix}");
        assert_eq!(race.level_mutation_rewards.len(), 1, "{suffix}");
        assert!(race.tags.iter().any(|tag| tag == "draconian"));
        assert!(race.tags.iter().any(|tag| tag == "rfb-compatibility"));
        assert!(!race.tags.iter().any(|tag| tag == "polymorph-candidate"));

        let [activation] = race.abilities.as_slice() else {
            panic!("{suffix} should have exactly one breath power");
        };
        assert_eq!(activation.minimum_level, 1, "{suffix}");
        assert_eq!(
            activation.governing_attribute,
            TechniqueAttribute::Constitution,
            "{suffix}"
        );
        assert_eq!((activation.cost, activation.base_failure_percent), (0, 70));
        assert_eq!(
            activation.cost_scaling,
            Some(InnatePowerCostScalingDefinition {
                curve: InnatePowerCostScalingCurveDefinition::Prorated,
                start_level: 1,
                level_interval: 1,
                amount: 40,
                divisor: 1,
                round_up: false,
                linear_weight: 5,
                quadratic_weight: 3,
                cubic_weight: 0,
            }),
            "{suffix}"
        );
        assert_eq!(
            activation.ability_id,
            format!("rfb.ability.race.draconian-{suffix}-breath")
        );

        let ability = catalog
            .ability(&activation.ability_id)
            .unwrap_or_else(|| panic!("{}", activation.ability_id));
        assert!(ability.affects_ground_items, "{suffix}");
        let AbilityEffectDefinition::DraconianBreathDamage {
            base_hp_percent,
            level_cubic_percent_numerator,
            level_cubic_percent_divisor,
            max_damage,
            damage_type: actual_damage_type,
            enhancing_mutation_id,
        } = &ability.effect
        else {
            panic!("{suffix} should use dynamic Draconian breath damage");
        };
        assert_eq!(
            (
                *base_hp_percent,
                *level_cubic_percent_numerator,
                *level_cubic_percent_divisor,
                *max_damage
            ),
            breath,
            "{suffix}"
        );
        assert_eq!(*actual_damage_type, damage_type, "{suffix}");
        assert_eq!(enhancing_mutation_id, "rfb.mutation.draconian-breath");

        let skills = catalog
            .skill_set(&race.skill_set_id)
            .unwrap_or_else(|| panic!("{}", race.skill_set_id))
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            [
                skills.get("demo.skill.disarming").copied().unwrap_or(0),
                skills.get("demo.skill.device").copied().unwrap_or(0),
                skills.get("demo.skill.saving-throw").copied().unwrap_or(0),
                skills.get("demo.skill.stealth").copied().unwrap_or(0),
                skills.get("demo.skill.search").copied().unwrap_or(0),
                skills.get("demo.skill.perception").copied().unwrap_or(0),
                skills.get("demo.skill.melee").copied().unwrap_or(0),
                skills.get("demo.skill.ranged").copied().unwrap_or(0),
            ],
            expected_skills,
            "{suffix}"
        );
    }

    assert_eq!(
        catalog
            .race_by_legacy_index(20)
            .expect("legacy Draconian index")
            .id,
        "rfb-legacy.race.draconian-red"
    );
    assert!(
        [
            "white", "blue", "black", "green", "bronze", "crystal", "gold", "shadow"
        ]
        .iter()
        .all(|suffix| catalog
            .race(&format!("rfb-legacy.race.draconian-{suffix}"))
            .is_some_and(|race| race.legacy_index.is_none()))
    );
}

#[test]
fn draconian_level_35_power_matrix_includes_completed_metamorphosis() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );
    let mutation_ids = [
        "rfb.mutation.draconian-shield",
        "rfb.mutation.draconian-magic-res",
        "rfb.mutation.draconian-strike",
        "rfb.mutation.draconian-breath",
        "rfb.mutation.draconian-regen",
        "rfb.mutation.draconian-kin",
        "rfb.mutation.draconian-lore",
        "rfb.mutation.draconian-resistance",
        "rfb.mutation.draconian-metamorphosis",
    ];
    let profiles = [
        (
            "red",
            15,
            Some(ActorDamageType::Fire),
            ActorDamageType::Fire,
            15,
            DraconianStrikeModeDefinition::Fire,
        ),
        (
            "white",
            15,
            Some(ActorDamageType::Cold),
            ActorDamageType::Cold,
            15,
            DraconianStrikeModeDefinition::Cold,
        ),
        (
            "blue",
            15,
            Some(ActorDamageType::Electricity),
            ActorDamageType::Electricity,
            15,
            DraconianStrikeModeDefinition::Electricity,
        ),
        (
            "black",
            25,
            None,
            ActorDamageType::Acid,
            15,
            DraconianStrikeModeDefinition::Acid,
        ),
        (
            "green",
            25,
            None,
            ActorDamageType::Poison,
            15,
            DraconianStrikeModeDefinition::Poison,
        ),
        (
            "bronze",
            25,
            None,
            ActorDamageType::Confusion,
            20,
            DraconianStrikeModeDefinition::Confusion,
        ),
        (
            "crystal",
            10,
            Some(ActorDamageType::Shards),
            ActorDamageType::Shards,
            12,
            DraconianStrikeModeDefinition::Vorpal,
        ),
        (
            "gold",
            25,
            None,
            ActorDamageType::Sound,
            20,
            DraconianStrikeModeDefinition::Stun,
        ),
        (
            "shadow",
            25,
            None,
            ActorDamageType::Nether,
            7,
            DraconianStrikeModeDefinition::Vampiric,
        ),
    ];

    for (suffix, armor_class, aura, resistance, cost, mode) in profiles {
        let race = catalog
            .race(&format!("rfb-legacy.race.draconian-{suffix}"))
            .unwrap_or_else(|| panic!("{suffix}"));
        let [reward] = race.level_mutation_rewards.as_slice() else {
            panic!("{suffix} should have one level-35 reward");
        };
        assert_eq!(
            (reward.id.as_str(), reward.minimum_level),
            ("draconian-power", 35)
        );
        let RaceMutationSelectionDefinition::Choice {
            mutation_ids: actual_ids,
        } = &reward.selection
        else {
            panic!("{suffix} should expose a manual power choice");
        };
        assert_eq!(
            actual_ids.iter().map(String::as_str).collect::<Vec<_>>(),
            mutation_ids
        );
        for class_id in [
            "demo.class.archer",
            "demo.class.cavalry",
            "demo.class.sniper",
        ] {
            assert_eq!(
                race.mutation_choice_exclusions_by_class[class_id],
                BTreeSet::from(["rfb.mutation.draconian-metamorphosis".to_owned()]),
                "{suffix}:{class_id}"
            );
        }

        let shield = &race.mutation_overrides["rfb.mutation.draconian-shield"];
        assert_eq!(
            (shield.armor_class, shield.contact_aura),
            (Some(armor_class), aura)
        );
        let resistance_override = &race.mutation_overrides["rfb.mutation.draconian-resistance"];
        assert_eq!(
            resistance_override.resistances,
            Some(BTreeMap::from([
                (resistance, ActorResistanceLevel::Strong,)
            ]))
        );
        let strike = &race.mutation_overrides["rfb.mutation.draconian-strike"];
        let activation = strike.activation.as_ref().expect("strike activation");
        assert_eq!(activation.minimum_level, 30);
        assert_eq!(
            activation.governing_attribute,
            TechniqueAttribute::Dexterity
        );
        assert_eq!(
            (activation.cost, activation.base_failure_percent),
            (cost, 0)
        );
        assert_eq!(
            activation.ability_id,
            format!("rfb.ability.mutation.draconian-strike-{suffix}")
        );
        assert!(matches!(
            catalog
                .ability(&activation.ability_id)
                .expect("strike ability")
                .effect,
            AbilityEffectDefinition::DraconianStrike { mode: actual } if actual == mode
        ));
    }

    let kin = catalog
        .mutation("rfb.mutation.draconian-kin")
        .expect("Draconian kin mutation")
        .activation
        .as_ref()
        .expect("Draconian kin activation");
    assert_eq!(kin.minimum_level, 30);
    assert_eq!(kin.governing_attribute, TechniqueAttribute::Charisma);
    assert_eq!((kin.cost, kin.base_failure_percent), (30, 70));
    let AbilityEffectDefinition::SummonCategory {
        category,
        maximum_level,
        count_dice,
        count_sides,
        friendly_group_chance_percent,
        group_count_dice,
        group_count_sides,
        group_count_bonus,
        radius,
        duration_turns,
        ..
    } = &catalog
        .ability(&kin.ability_id)
        .expect("Draconian kin ability")
        .effect
    else {
        panic!("Draconian kin should use the shared category summon");
    };
    assert_eq!(category, "kin-glyph-100");
    assert_eq!((*maximum_level, *count_dice, *count_sides), (0, 1, 1));
    assert_eq!(*friendly_group_chance_percent, 50);
    assert_eq!(
        (*group_count_dice, *group_count_sides, *group_count_bonus),
        (1, 3, 1)
    );
    assert_eq!((*radius, *duration_turns), (2, 0));

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let entries = ledger["mutations"].as_array().expect("mutation entries");
    for id in mutation_ids {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
    let metamorphosis = catalog
        .mutation("rfb.mutation.draconian-metamorphosis")
        .expect("Draconian metamorphosis mutation");
    assert_eq!(metamorphosis.name, "变形");
    assert_eq!(metamorphosis.description, "你变形成了一条龙！");
    assert_eq!(metamorphosis.random_weight, 0);
}

#[test]
fn formal_barbarian_matches_rfb_profile_power_and_talent_pool() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let barbarian = catalog
        .race("rfb-legacy.race.barbarian")
        .expect("formal Barbarian race");

    assert_eq!(
        [
            barbarian.modifiers.strength,
            barbarian.modifiers.intelligence,
            barbarian.modifiers.wisdom,
            barbarian.modifiers.dexterity,
            barbarian.modifiers.constitution,
            barbarian.modifiers.charisma,
        ],
        [3, -2, -1, 1, 2, 2]
    );
    assert_eq!(barbarian.life_percent, 103);
    assert_eq!(barbarian.base_hp, 22);
    assert_eq!(barbarian.experience_percent, 135);
    assert_eq!(barbarian.shop_adjust_percent, 120);
    assert_eq!(barbarian.infravision, 0);
    assert_eq!(
        barbarian.resistances.get(&ActorDamageType::Fear),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        barbarian.abilities,
        [InnatePowerDefinition {
            minimum_level: 8,
            governing_attribute: TechniqueAttribute::Strength,
            cost: 10,
            cost_scaling: None,
            base_failure_percent: 30,
            minimum_failure_percent: None,
            ability_id: "rfb.ability.race.berserk".to_owned(),
        }]
    );
    for tag in [
        "humanoid",
        "polymorph-candidate",
        "rfb-compatibility",
        "standard-body",
    ] {
        assert!(barbarian.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&barbarian.skill_set_id)
        .expect("formal Barbarian skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", -7),
            ("demo.skill.disarming", -2),
            ("demo.skill.melee", 12),
            ("demo.skill.perception", 7),
            ("demo.skill.ranged", 6),
            ("demo.skill.saving-throw", 2),
            ("demo.skill.search", 1),
            ("demo.skill.stealth", -1),
        ]
    );

    let talent = barbarian
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "barbarian-talent")
        .expect("Barbarian talent");
    let human_talent = catalog
        .race("demo.race.rfb-human")
        .expect("formal Human race")
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "human-talent")
        .expect("Human talent");
    assert_eq!(talent.minimum_level, 30);
    assert_eq!(talent.selection, human_talent.selection);
}

#[test]
fn formal_high_elf_matches_the_original_profile() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let high_elf = catalog
        .race("rfb-legacy.race.high-elf")
        .expect("formal High-Elf race");

    assert_eq!(
        [
            high_elf.modifiers.strength,
            high_elf.modifiers.intelligence,
            high_elf.modifiers.wisdom,
            high_elf.modifiers.dexterity,
            high_elf.modifiers.constitution,
            high_elf.modifiers.charisma,
        ],
        [1, 3, -1, 3, 1, 1]
    );
    assert_eq!(high_elf.life_percent, 99);
    assert_eq!(high_elf.base_hp, 19);
    assert_eq!(high_elf.experience_percent, 190);
    assert_eq!(high_elf.shop_adjust_percent, 90);
    assert_eq!(high_elf.infravision, 4);
    assert!(high_elf.see_invisible);
    assert_eq!(
        high_elf.resistances.get(&ActorDamageType::Light),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(high_elf.level_mutation_rewards.is_empty());
    for tag in [
        "humanoid",
        "rfb-compatibility",
        "snow-adapted",
        "standard-body",
    ] {
        assert!(high_elf.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&high_elf.skill_set_id)
        .expect("formal High-Elf skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 9),
            ("demo.skill.disarming", 4),
            ("demo.skill.melee", 10),
            ("demo.skill.perception", 14),
            ("demo.skill.ranged", 15),
            ("demo.skill.saving-throw", 12),
            ("demo.skill.search", 3),
            ("demo.skill.stealth", 4),
        ]
    );
}

#[test]
fn formal_dunadan_matches_the_original_profile_and_talent_pool() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let dunadan = catalog
        .race("rfb-legacy.race.dunadan")
        .expect("formal Dunadan race");

    assert_eq!(
        [
            dunadan.modifiers.strength,
            dunadan.modifiers.intelligence,
            dunadan.modifiers.wisdom,
            dunadan.modifiers.dexterity,
            dunadan.modifiers.constitution,
            dunadan.modifiers.charisma,
        ],
        [1, 2, 2, 2, 3, 0]
    );
    assert_eq!(dunadan.life_percent, 100);
    assert_eq!(dunadan.base_hp, 20);
    assert_eq!(dunadan.experience_percent, 160);
    assert_eq!(dunadan.shop_adjust_percent, 100);
    assert_eq!(dunadan.infravision, 0);
    assert_eq!(
        dunadan
            .attribute_sustains
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [ItemAttributeDefinition::Constitution]
    );
    for tag in ["humanoid", "rfb-compatibility", "standard-body"] {
        assert!(dunadan.tags.iter().any(|candidate| candidate == tag));
    }

    let skills = catalog
        .skill_set(&dunadan.skill_set_id)
        .expect("formal Dunadan skill set");
    assert_eq!(
        skills
            .entries
            .iter()
            .map(|entry| (entry.skill_id.as_str(), entry.base))
            .collect::<Vec<_>>(),
        [
            ("demo.skill.device", 3),
            ("demo.skill.disarming", 4),
            ("demo.skill.melee", 15),
            ("demo.skill.perception", 13),
            ("demo.skill.ranged", 7),
            ("demo.skill.saving-throw", 3),
            ("demo.skill.search", 3),
            ("demo.skill.stealth", 2),
        ]
    );

    let talent = dunadan
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "dunadan-talent")
        .expect("Dunadan should choose a level 30 talent");
    let human_talent = catalog
        .race("demo.race.rfb-human")
        .expect("formal Human race")
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "human-talent")
        .expect("Human talent");
    assert_eq!(talent.minimum_level, 30);
    assert_eq!(talent.selection, human_talent.selection);
}

#[test]
fn p76_complex_monsters_and_their_shared_mechanisms_compile_into_the_pack() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let actors = [
        (822, "qlzqqlzuup-the-lord-of-flesh"),
        (844, "kaschei-the-immortal"),
        (848, "shub-niggurath-black-goat-of-the-woods"),
        (849, "nodens-lord-of-the-great-abyss"),
        (859, "the-unicorn-of-order"),
        (861, "morgoth-lord-of-darkness"),
        (1098, "hades-ruler-of-the-underworld"),
        (1099, "athena-the-goddess-of-wisdom"),
        (1100, "ares-the-god-of-war"),
        (1102, "apollo-the-sun-god"),
        (1103, "artemis-the-moon-goddess"),
        (1104, "hephaestus-the-smith-god"),
        (1105, "hera-queen-of-the-gods"),
        (1107, "aphrodite-the-goddess-of-love"),
        (1161, "atropos-the-sister-of-fate"),
        (1180, "tik-srvzllat"),
        (1259, "osiris-the-reborn"),
        (1265, "ptah-the-divine-craftsman"),
        (1281, "aijem-the-walrus"),
        (1365, "vayu-the-embodied-wind"),
        (1372, "vishnu-the-preserver"),
        (1373, "lakshmi-the-goddess-of-prosperity"),
        (1380, "shiva-the-destroyer"),
        (1382, "parvati-the-goddess-of-hidden-power"),
    ];
    for (legacy_index, id) in actors {
        let actor = catalog
            .actor(&format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P76 actor {id} should compile"));
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "{id} should retain its authoritative source index"
        );
    }

    let has_ability = |actor_id: &str, ability_id: &str| {
        catalog
            .actor(actor_id)
            .and_then(|actor| actor.monster_casting.as_ref())
            .is_some_and(|casting| {
                casting
                    .abilities
                    .iter()
                    .any(|candidate| candidate.ability_id == ability_id)
            })
    };
    assert!(has_ability(
        "demo.actor.morgoth-lord-of-darkness",
        "rfb-legacy.ability.summon-unique-l100-1d2"
    ));
    assert!(has_ability(
        "demo.actor.osiris-the-reborn",
        "rfb-legacy.ability.summon-family-osiris-the-reborn"
    ));
    assert!(has_ability(
        "demo.actor.vayu-the-embodied-wind",
        "rfb-legacy.ability.breath-air-17-250-r3"
    ));
    assert!(has_ability(
        "demo.actor.vayu-the-embodied-wind",
        "rfb-legacy.ability.no-air-40"
    ));
    assert!(has_ability(
        "demo.actor.aijem-the-walrus",
        "rfb-legacy.ability.chicken-1d1-199"
    ));

    let kaschei = catalog
        .actor("demo.actor.kaschei-the-immortal")
        .expect("Kaschei should compile");
    assert!(matches!(
        kaschei.contact_effects.as_slice(),
        [MeleeBlowEffectDefinition::Unlife {
            amount_dice: 2,
            amount_sides: 6,
            chance_percent: Some(50)
        }]
    ));
    let unicorn = catalog
        .actor("demo.actor.the-unicorn-of-order")
        .expect("Unicorn of Order should compile");
    assert!(
        unicorn
            .contact_auras
            .iter()
            .any(|aura| { aura.damage_type == ActorDamageType::Time && aura.ravages_time })
    );
}

#[test]
fn p77_location_bound_monsters_and_resurrection_mechanism_compile_into_the_pack() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    for id in [
        "godzilla",
        "greater-cyber-wyrm-angel-daemon-lich",
        "sauron-the-sorcerer",
        "oberon-king-of-amber",
        "the-serpent-of-chaos",
        "the-resurrection-machine",
    ] {
        assert!(
            catalog.actor(&format!("demo.actor.{id}")).is_some(),
            "P77 actor {id} should compile"
        );
    }

    let godzilla = catalog.actor("demo.actor.godzilla").expect("Godzilla");
    let godzilla_allocation = godzilla.allocation.as_ref().expect("ocean allocation");
    assert_eq!(godzilla_allocation.legacy_index, 832);
    assert!(godzilla_allocation.wild_only);
    assert_eq!(godzilla_allocation.habitats, [ActorHabitat::Ocean]);

    let wyrm = catalog
        .actor("demo.actor.greater-cyber-wyrm-angel-daemon-lich")
        .expect("Greater Cyber Wyrm Angel Daemon Lich");
    let wyrm_allocation = wyrm.allocation.as_ref().expect("wilderness allocation");
    assert_eq!(wyrm_allocation.legacy_index, 1337);
    assert!(wyrm_allocation.habitats.contains(&ActorHabitat::All));
    assert!(wyrm_allocation.habitats.contains(&ActorHabitat::Ocean));
    assert_eq!(
        wyrm.monster_casting
            .as_ref()
            .expect("the full original casting profile")
            .abilities
            .len(),
        94
    );

    for id in [
        "sauron-the-sorcerer",
        "oberon-king-of-amber",
        "the-serpent-of-chaos",
        "the-resurrection-machine",
    ] {
        assert!(
            catalog
                .actor(&format!("demo.actor.{id}"))
                .expect("fixed identity actor")
                .allocation
                .is_none(),
            "{id} should require explicit placement"
        );
    }

    let serpent = catalog
        .actor("demo.actor.the-serpent-of-chaos")
        .expect("Serpent of Chaos");
    assert!(serpent.tags.iter().any(|tag| tag == "guardian"));
    assert!(serpent.contact_auras.iter().any(|aura| {
        aura.damage_type == ActorDamageType::Chaos && aura.chance_percent == Some(20)
    }));
    assert!(serpent.contact_auras.iter().any(|aura| {
        aura.damage_type == ActorDamageType::Disenchant && aura.chance_percent == Some(10)
    }));

    let resurrection = catalog
        .ability("rfb-legacy.ability.summon-dead-unique-l100-1d2")
        .expect("S_DEAD_UNIQ should compile");
    assert!(
        resurrection
            .tags
            .iter()
            .any(|tag| tag == "monster-dead-unique-summon")
    );
    assert!(matches!(
        &resurrection.effect,
        AbilityEffectDefinition::SummonCategory {
            category,
            count_dice: 1,
            count_sides: 2,
            maximum_level: 100,
            ..
        } if category == "unique"
    ));
}

#[test]
fn p78_direct_monsters_compile_with_original_levels_and_existing_mechanics() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    for (id, level, legacy_index) in [
        ("warrens-keeper", 7, 135),
        ("swamp-rat", 10, 1297),
        ("plague-monk", 14, 1293),
        ("skaven-assassin", 14, 1294),
        ("clay-golem", 15, 261),
        ("magic-mushroom-patch", 15, 267),
        ("rat-ogre", 15, 1295),
        ("master-rogue", 23, 376),
        ("mummified-human", 24, 390),
        ("samurai", 25, 901),
        ("black-knight", 28, 442),
        ("trap-master", 28, 1036),
        ("nekhbet-the-vulture-mother", 57, 1258),
        ("thoth-the-voice-of-ra", 60, 1246),
        ("loki-the-trickster", 85, 835),
        ("shuma-gorath", 88, 841),
        ("pandemonium", 94, 1200),
        ("zombified-serpent-of-chaos", 127, 883),
    ] {
        let actor = catalog
            .actor(&format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P78 actor {id} should compile"));
        assert_eq!(actor.level, level, "P78 actor {id} level");
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index),
            "P78 actor {id} source index"
        );
        let allocation_tag = if id == "warrens-keeper" {
            "warrens"
        } else {
            "orc-cave"
        };
        assert!(actor.tags.iter().any(|tag| tag == allocation_tag));
    }

    let mushroom = catalog
        .actor("demo.actor.magic-mushroom-patch")
        .expect("Magic mushroom patch");
    assert!(
        mushroom
            .monster_casting
            .as_ref()
            .expect("original casting profile")
            .abilities
            .iter()
            .any(|ability| ability.ability_id == "rfb-legacy.ability.polymorph-target")
    );

    let serpent = catalog
        .actor("demo.actor.zombified-serpent-of-chaos")
        .expect("Zombified Serpent of Chaos");
    assert!(serpent.tags.iter().any(|tag| tag == "unique2"));
    assert!(serpent.contact_auras.iter().any(|aura| {
        aura.damage_type == ActorDamageType::Shards && aura.chance_percent.is_none()
    }));
    assert!(serpent.contact_auras.iter().any(|aura| {
        aura.damage_type == ActorDamageType::Chaos && aura.chance_percent == Some(40)
    }));
    assert!(serpent.contact_auras.iter().any(|aura| {
        aura.damage_type == ActorDamageType::Disenchant && aura.chance_percent == Some(20)
    }));
}

#[test]
fn p79_norse_and_olympian_summoners_compile_with_original_retinues() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    for (id, level, legacy_index) in [
        ("einheri-berserker", 65, 1344),
        ("hermes-the-messenger-god", 86, 1101),
        ("zeus-king-of-the-olympians", 90, 1096),
        ("odin-the-all-father", 90, 1343),
    ] {
        let actor = catalog
            .actor(&format!("demo.actor.{id}"))
            .unwrap_or_else(|| panic!("P79 actor {id} should compile"));
        assert_eq!(actor.level, level);
        assert_eq!(
            actor
                .allocation
                .as_ref()
                .map(|allocation| allocation.legacy_index),
            Some(legacy_index)
        );
    }

    let einheri = catalog
        .actor("demo.actor.einheri-berserker")
        .expect("Einheri berserker");
    assert_eq!(
        einheri
            .allocation
            .as_ref()
            .expect("Asgard allocation")
            .legacy_dungeon_indices,
        [39]
    );
    assert!(einheri.tags.iter().any(|tag| tag == "asgard"));

    for (ability_id, target_id, count_sides) in [
        (
            "rfb-legacy.ability.summon-magic-mushroom-patch-l15-1d16",
            "demo.actor.magic-mushroom-patch",
            16,
        ),
        (
            "rfb-legacy.ability.summon-shambler-l67-1d4",
            "demo.actor.shambler",
            4,
        ),
    ] {
        let ability = catalog.ability(ability_id).expect("P79 fixed retinue");
        assert!(matches!(
            &ability.effect,
            AbilityEffectDefinition::SummonCategory {
                count_dice: 1,
                count_sides: sides,
                count_bonus: 0,
                maximum_count: None,
                batch_candidates,
                ..
            } if *sides == count_sides
                && batch_candidates.as_slice() == [AbilitySummonCandidateDefinition {
                    actor_kind_id: target_id.to_owned(),
                    weight: 1,
                }]
        ));
    }

    let odin = catalog
        .ability("rfb-legacy.ability.summon-odin-retinue-1d4-max1")
        .expect("Odin retinue");
    assert!(matches!(
        &odin.effect,
        AbilityEffectDefinition::SummonCategory {
            count_dice: 1,
            count_sides: 4,
            count_bonus: 0,
            maximum_count: Some(1),
            batch_candidates,
            ..
        } if batch_candidates.as_slice() == [
            AbilitySummonCandidateDefinition {
                actor_kind_id: "demo.actor.einheri-berserker".to_owned(),
                weight: 1,
            },
            AbilitySummonCandidateDefinition {
                actor_kind_id: "demo.actor.valkyrie".to_owned(),
                weight: 1,
            },
        ]
    ));
}

#[test]
fn p80_variant_maintainer_compiles_with_software_bug_summon() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");
    let actor = catalog
        .actor("demo.actor.the-variant-maintainer")
        .expect("Variant Maintainer should compile");
    assert_eq!(actor.level, 14);
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(1094)
    );
    assert!(actor.tags.iter().any(|tag| tag == "unique"));
    let casting = actor
        .monster_casting
        .as_ref()
        .expect("Variant Maintainer casting profile");
    assert_eq!(casting.frequency_percent, 33);
    assert_eq!(
        casting
            .abilities
            .iter()
            .map(|entry| entry.ability_id.as_str())
            .collect::<Vec<_>>(),
        [
            "rfb-legacy.ability.summon-software-bug-l14-1d3-1",
            "rfb-legacy.ability.polymorph-target",
        ]
    );

    let summon = catalog
        .ability("rfb-legacy.ability.summon-software-bug-l14-1d3-1")
        .expect("software bug summon should compile");
    assert!(matches!(
        &summon.effect,
        AbilityEffectDefinition::SummonCategory {
            count_dice: 1,
            count_sides: 3,
            count_bonus: 1,
            batch_candidates,
            ..
        } if batch_candidates.as_slice() == [AbilitySummonCandidateDefinition {
            actor_kind_id: "demo.actor.software-bug".to_owned(),
            weight: 1,
        }]
    ));
}

#[test]
fn elemental_ground_item_rules_compile_as_explicit_content() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

    assert!(
        catalog
            .ability("rfb-legacy.ability.bolt-fire-9d8-5")
            .expect("fire bolt")
            .affects_ground_items
    );
    assert!(
        !catalog
            .ability("demo.ability.death-berserk")
            .expect("self buff")
            .affects_ground_items
    );
    let arrow = catalog.item("demo.item.arrow").expect("arrow");
    assert!(
        arrow
            .elemental_destruction_vulnerabilities
            .contains(&ItemDestructionElement::Fire)
    );
    let protection = catalog
        .affix("rfb-legacy.affix.protection")
        .expect("Protection affix");
    assert!(
        protection
            .elemental_destruction_immunities
            .contains(&ItemDestructionElement::Acid)
    );
    let endurance = catalog
        .affix("rfb-legacy.affix.endurance")
        .expect("Endurance affix");
    assert!(endurance.resists_projection_destruction);
    assert!(endurance.resists_monster_destruction);
    assert!(affix_is_compatible_with_item(endurance, arrow, 40));

    let venom = catalog
        .item("demo.item.venom-draught")
        .expect("venom potion");
    assert!(matches!(
        venom
            .use_action
            .as_ref()
            .map(|use_action| &use_action.effect),
        Some(ItemUseEffectDefinition::ApplyPoison { .. })
    ));
    assert!(matches!(
        venom.shatter_effect.as_ref().map(|shatter| &shatter.effect),
        Some(ItemUseEffectDefinition::Damage {
            damage_type: ActorDamageType::Poison,
            ..
        })
    ));
}

#[test]
fn m6_a_periodic_mutations_are_random_candidates_with_typed_effects() {
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&original_pack_path()).expect("original pack should compile"),
    );
    for id in [
        "rfb.mutation.bers-rage",
        "rfb.mutation.cowardice",
        "rfb.mutation.alcohol",
        "rfb.mutation.hallucination",
        "rfb.mutation.prod-mana",
        "rfb.mutation.speed-flux",
        "rfb.mutation.invuln",
        "rfb.mutation.sp-to-hp",
        "rfb.mutation.hp-to-sp",
        "rfb.mutation.hypochondria",
    ] {
        let mutation = catalog.mutation(id).expect("M6-A mutation should exist");
        assert!(
            mutation.random_selection_enabled,
            "{id} should be selectable"
        );
        assert!(
            mutation.periodic_effect.is_some(),
            "{id} should be periodic"
        );
    }
}

#[test]
fn m6_b_periodic_mutations_are_random_candidates_with_typed_effects() {
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&original_pack_path()).expect("original pack should compile"),
    );
    for id in [
        "rfb.mutation.teleport-rnd",
        "rfb.mutation.banish-all-rnd",
        "rfb.mutation.shadow-walk",
        "rfb.mutation.fumbling",
    ] {
        let mutation = catalog.mutation(id).expect("M6-B mutation should exist");
        assert!(
            mutation.random_selection_enabled,
            "{id} should be selectable"
        );
        assert!(
            mutation.periodic_effect.is_some(),
            "{id} should be periodic"
        );
    }
}

#[test]
fn m6_c_periodic_mutations_are_random_candidates_with_typed_effects() {
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&original_pack_path()).expect("original pack should compile"),
    );
    for id in [
        "rfb.mutation.flatulent",
        "rfb.mutation.attract-demon",
        "rfb.mutation.eat-light",
        "rfb.mutation.attract-animal",
        "rfb.mutation.raw-chaos",
        "rfb.mutation.attract-dragon",
    ] {
        let mutation = catalog.mutation(id).expect("M6-C mutation should exist");
        assert!(
            mutation.random_selection_enabled,
            "{id} should be selectable"
        );
        assert!(
            mutation.periodic_effect.is_some(),
            "{id} should be periodic"
        );
    }
}

#[test]
fn m6_d_periodic_mutations_are_random_candidates_with_typed_effects() {
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&original_pack_path()).expect("original pack should compile"),
    );
    for id in [
        "rfb.mutation.normality",
        "rfb.mutation.wraith",
        "rfb.mutation.poly-wound",
        "rfb.mutation.wasting",
        "rfb.mutation.random-telepathy",
        "rfb.mutation.nausea",
        "rfb.mutation.warning",
    ] {
        let mutation = catalog.mutation(id).expect("M6-D mutation should exist");
        assert!(
            mutation.random_selection_enabled,
            "{id} should be selectable"
        );
        assert!(
            mutation.periodic_effect.is_some(),
            "{id} should be periodic"
        );
    }
}

#[test]
fn mutation_definitions_match_the_frozen_legacy_ledger() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );
    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let mutations = ledger["mutations"]
        .as_array()
        .expect("ledger should contain mutations");
    assert_eq!(mutations.len(), catalog.mutations().count());
    for expected in mutations {
        let id = expected["id"].as_str().expect("mutation id");
        let actual = catalog.mutation(id).expect("mutation definition");
        assert_eq!(actual.name, expected["nameZh"].as_str().unwrap());
        assert_eq!(
            actual.description,
            expected["descriptionZh"].as_str().unwrap()
        );
        assert_eq!(
            serde_json::to_value(actual.rating).unwrap(),
            expected["rating"]
        );
        assert_eq!(
            u64::from(actual.source_index),
            expected["sourceIndex"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(actual.random_weight),
            expected["randomWeight"].as_u64().unwrap()
        );
        assert_eq!(
            serde_json::to_value(&actual.removes_on_gain).unwrap(),
            expected["removesOnGain"]
        );
    }
    assert_eq!(
        catalog
            .mutations()
            .filter(|mutation| mutation.random_weight > 0)
            .count(),
        104
    );
    assert_eq!(
        catalog
            .mutations()
            .filter(|mutation| mutation.random_weight > 0 && mutation.random_selection_enabled)
            .count(),
        104
    );
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| mutation["status"] == "active")
            .count(),
        144
    );
}

#[test]
fn first_passive_mutation_batch_keeps_original_attribute_speed_and_armor_bonuses() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );
    let expected = [
        (
            "rfb.mutation.hyper-str",
            StatModifiers {
                strength: 4,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.puny",
            StatModifiers {
                strength: -4,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.hyper-int",
            StatModifiers {
                intelligence: 4,
                wisdom: 4,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.moronic",
            StatModifiers {
                intelligence: -4,
                wisdom: -4,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.pultitis",
            StatModifiers {
                intelligence: -3,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.resilient",
            StatModifiers {
                constitution: 4,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.xtra-fat",
            StatModifiers {
                constitution: 2,
                speed: -2,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.albino",
            StatModifiers {
                constitution: -4,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.silly-voice",
            StatModifiers {
                charisma: -4,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.blank-face",
            StatModifiers {
                charisma: -1,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.xtra-legs",
            StatModifiers {
                speed: 3,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.short-leg",
            StatModifiers {
                speed: -3,
                ..StatModifiers::default()
            },
            0,
        ),
        (
            "rfb.mutation.warts",
            StatModifiers {
                charisma: -2,
                ..StatModifiers::default()
            },
            5,
        ),
        (
            "rfb.mutation.scales",
            StatModifiers {
                charisma: -1,
                ..StatModifiers::default()
            },
            10,
        ),
        (
            "rfb.mutation.steel-skin",
            StatModifiers {
                dexterity: -1,
                ..StatModifiers::default()
            },
            25,
        ),
    ];
    let active_expected = expected
        .iter()
        .map(|(id, _, _)| *id)
        .collect::<std::collections::BTreeSet<_>>();
    for (id, modifiers, armor_class) in expected {
        let mutation = catalog
            .mutation(id)
            .unwrap_or_else(|| panic!("{id} should exist"));
        assert_eq!(mutation.modifiers, modifiers, "{id} modifiers");
        assert_eq!(mutation.armor_class, armor_class, "{id} armor class");
    }

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let active = ledger["mutations"]
        .as_array()
        .expect("ledger should contain mutations")
        .iter()
        .filter(|entry| entry["status"] == "active")
        .map(|entry| entry["id"].as_str().expect("mutation id"))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(active.is_superset(&active_expected));
}

#[test]
fn second_passive_mutation_batch_keeps_resistance_sense_and_levitation_semantics() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );
    let expected_active = [
        "rfb.mutation.magic-res",
        "rfb.mutation.wings",
        "rfb.mutation.fearless",
        "rfb.mutation.weird-mind",
        "rfb.mutation.draconian-magic-res",
        "rfb.mutation.draconian-resistance",
        "rfb.mutation.sensitive-eyes",
        "rfb.mutation.no-inhibitions",
        "rfb.mutation.infravision",
        "rfb.mutation.vuln-elem",
        "rfb.mutation.waybread-into",
        "rfb.mutation.ill-norm",
        "rfb.mutation.esp",
    ];

    for id in ["rfb.mutation.magic-res", "rfb.mutation.draconian-magic-res"] {
        let mutation = catalog.mutation(id).unwrap_or_else(|| panic!("{id}"));
        assert_eq!(mutation.saving_throw_skill, 15);
        assert_eq!(mutation.saving_throw_skill_per_five_levels, 1);
    }
    assert!(catalog.mutation("rfb.mutation.wings").unwrap().levitation);
    assert!(
        catalog
            .mutation("rfb.mutation.waybread-into")
            .unwrap()
            .levitation
    );
    assert!(catalog.mutation("rfb.mutation.esp").unwrap().telepathy);
    assert!(
        catalog
            .mutation("rfb.mutation.ill-norm")
            .unwrap()
            .normal_appearance
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.fearless")
            .unwrap()
            .resistances
            .get(&ActorDamageType::Fear),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.sensitive-eyes")
            .unwrap()
            .resistances
            .get(&ActorDamageType::Blindness),
        Some(&ActorResistanceLevel::Vulnerable)
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.sensitive-eyes")
            .unwrap()
            .infravision,
        4
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.infravision")
            .unwrap()
            .infravision,
        3
    );
    let elemental_vulnerability = &catalog
        .mutation("rfb.mutation.vuln-elem")
        .unwrap()
        .resistances;
    for damage_type in [
        ActorDamageType::Acid,
        ActorDamageType::Cold,
        ActorDamageType::Electricity,
        ActorDamageType::Fire,
    ] {
        assert_eq!(
            elemental_vulnerability.get(&damage_type),
            Some(&ActorResistanceLevel::Vulnerable),
            "{damage_type:?} vulnerability"
        );
    }
    assert_eq!(
        catalog
            .mutation("rfb.mutation.weird-mind")
            .unwrap()
            .status_immunities,
        ["rfb.status.eldritch-horror", "rfb.status.hallucination"]
    );

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let entries = ledger["mutations"].as_array().expect("mutation entries");
    for id in expected_active {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
}

#[test]
fn third_passive_mutation_batch_keeps_regeneration_aura_and_light_semantics() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );

    for (id, modifier) in [
        ("rfb.mutation.flesh-rot", -80),
        ("rfb.mutation.regen", 100),
        ("rfb.mutation.draconian-regen", 150),
    ] {
        assert_eq!(
            catalog
                .mutation(id)
                .unwrap_or_else(|| panic!("{id}"))
                .regeneration_rate_modifier_percent,
            modifier,
            "{id} regeneration modifier"
        );
    }
    let flesh_rot = catalog.mutation("rfb.mutation.flesh-rot").unwrap();
    assert_eq!(flesh_rot.modifiers.constitution, -2);
    assert_eq!(flesh_rot.modifiers.charisma, -1);
    assert_eq!(
        catalog
            .mutation("rfb.mutation.fire-aura")
            .unwrap()
            .contact_aura,
        Some(ActorDamageType::Fire)
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.fire-aura")
            .unwrap()
            .light_radius,
        1
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.elec-aura")
            .unwrap()
            .contact_aura,
        Some(ActorDamageType::Electricity)
    );

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let entries = ledger["mutations"].as_array().expect("mutation entries");
    for id in [
        "rfb.mutation.flesh-rot",
        "rfb.mutation.elec-aura",
        "rfb.mutation.fire-aura",
        "rfb.mutation.regen",
        "rfb.mutation.draconian-regen",
        "rfb.mutation.draconian-shield",
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
}

#[test]
fn fourth_passive_mutation_batch_keeps_innate_attack_and_combat_semantics() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );

    for (id, name, dice, sides, damage_type, weight) in [
        (
            "rfb.mutation.scorpion-tail",
            "尾巴",
            3,
            7,
            ActorDamageType::Poison,
            50,
        ),
        (
            "rfb.mutation.horns",
            "长角",
            2,
            6,
            ActorDamageType::Physical,
            150,
        ),
        (
            "rfb.mutation.beak",
            "鸟喙",
            2,
            4,
            ActorDamageType::Physical,
            30,
        ),
        (
            "rfb.mutation.trunk",
            "象鼻",
            1,
            4,
            ActorDamageType::Physical,
            200,
        ),
        (
            "rfb.mutation.tentacles",
            "触手",
            2,
            5,
            ActorDamageType::Physical,
            50,
        ),
    ] {
        let attack = catalog
            .mutation(id)
            .unwrap_or_else(|| panic!("{id}"))
            .innate_attack
            .as_ref()
            .unwrap_or_else(|| panic!("{id} innate attack"));
        assert_eq!(attack.name, name, "{id}");
        assert_eq!(
            (attack.damage_dice, attack.damage_sides),
            (dice, sides),
            "{id}"
        );
        assert_eq!(attack.damage_type, damage_type, "{id}");
        assert_eq!(attack.weight_tenths_pound, weight, "{id}");
        assert_eq!((attack.to_hit, attack.to_damage), (0, 0), "{id}");
    }
    assert!(
        catalog
            .mutation("rfb.mutation.launcher")
            .unwrap()
            .mighty_throw
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.limber")
            .unwrap()
            .modifiers
            .dexterity,
        3
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.arthritis")
            .unwrap()
            .modifiers
            .dexterity,
        -3
    );
    let motion = catalog.mutation("rfb.mutation.motion").unwrap();
    assert_eq!(motion.stealth_skill, 1);
    assert_eq!(motion.status_immunities, ["rfb.status.paralysis"]);
    assert_eq!(
        catalog
            .mutation("rfb.mutation.untouchable")
            .unwrap()
            .armor_class,
        20
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.tread-softly")
            .unwrap()
            .stealth_skill,
        3
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.human-int")
            .unwrap()
            .resistances
            .get(&ActorDamageType::Fear),
        Some(&ActorResistanceLevel::Vulnerable)
    );
    assert!(matches!(
        catalog
            .mutation("rfb.mutation.human-con")
            .unwrap()
            .periodic_effect,
        Some(MutationPeriodicEffectDefinition::ApplyStatus {
            trigger_one_in: 200,
            skip_if_present: true,
            ref status_kind_id,
            duration_ticks: 50,
            ..
        }) if status_kind_id == "rfb.status.unwell"
    ));
    let human_charisma = catalog.mutation("rfb.mutation.human-chr").unwrap();
    assert_eq!(
        (
            human_charisma.device_skill,
            human_charisma.melee_skill,
            human_charisma.ranged_skill,
            human_charisma.spell_failure_modifier_percent,
        ),
        (-10, -16, -10, 10)
    );

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let entries = ledger["mutations"].as_array().expect("mutation entries");
    for id in [
        "rfb.mutation.launcher",
        "rfb.mutation.scorpion-tail",
        "rfb.mutation.horns",
        "rfb.mutation.beak",
        "rfb.mutation.trunk",
        "rfb.mutation.tentacles",
        "rfb.mutation.limber",
        "rfb.mutation.arthritis",
        "rfb.mutation.motion",
        "rfb.mutation.untouchable",
        "rfb.mutation.tread-softly",
        "rfb.mutation.human-str",
        "rfb.mutation.human-int",
        "rfb.mutation.human-wis",
        "rfb.mutation.human-dex",
        "rfb.mutation.human-con",
        "rfb.mutation.human-chr",
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
    let id = "rfb.mutation.vortex-melee";
    let entry = entries
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap_or_else(|| panic!("{id}"));
    assert_eq!(entry["status"], "blocked", "{id}");
    assert_eq!(
        entry["blockers"],
        serde_json::json!(["vortex-race-innate-attack-identity"]),
        "{id}"
    );
}

#[test]
fn fifth_passive_mutation_batch_keeps_cross_system_semantics_explicit() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );

    let eyes = catalog.mutation("rfb.mutation.xtra-eyes").unwrap();
    assert_eq!((eyes.search_skill, eyes.perception_skill), (15, 15));
    assert_eq!(
        catalog
            .mutation("rfb.mutation.xtra-noise")
            .unwrap()
            .stealth_skill,
        -3
    );
    let learner = catalog.mutation("rfb.mutation.fast-learner").unwrap();
    assert_eq!(learner.kill_experience_bonus_percent, 20);
    assert_eq!(
        learner.relative_experience_multiplier,
        Some(MutationRatioDefinition {
            numerator: 5,
            denominator: 3,
        })
    );
    assert!(
        catalog
            .mutation("rfb.mutation.loremaster")
            .unwrap()
            .auto_identify_items
    );
    assert!(
        catalog
            .mutation("rfb.mutation.draconian-lore")
            .unwrap()
            .auto_identify_items
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.arcane-mastery")
            .unwrap()
            .spell_failure_modifier_percent,
        -3
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.one-with-magic")
            .unwrap()
            .dispel_resistance_percent,
        77
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.fleet-of-foot")
            .unwrap()
            .movement_energy_multiplier,
        Some(MutationRatioDefinition {
            numerator: 3,
            denominator: 5,
        })
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.limp")
            .unwrap()
            .movement_energy_multiplier,
        Some(MutationRatioDefinition {
            numerator: 10,
            denominator: 9,
        })
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.speed-reader")
            .unwrap()
            .scroll_energy_multiplier,
        Some(MutationRatioDefinition {
            numerator: 1,
            denominator: 2,
        })
    );
    assert!(
        catalog
            .mutation("rfb.mutation.black-marketeer")
            .unwrap()
            .black_market_standard_prices
    );
    assert!(
        catalog
            .mutation("rfb.mutation.strong-mind")
            .unwrap()
            .resource_drain_immunity
    );

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let entries = ledger["mutations"].as_array().expect("mutation entries");
    for id in [
        "rfb.mutation.xtra-eyes",
        "rfb.mutation.xtra-noise",
        "rfb.mutation.fast-learner",
        "rfb.mutation.loremaster",
        "rfb.mutation.arcane-mastery",
        "rfb.mutation.one-with-magic",
        "rfb.mutation.merchants-friend",
        "rfb.mutation.fleet-of-foot",
        "rfb.mutation.black-marketeer",
        "rfb.mutation.speed-reader",
        "rfb.mutation.draconian-lore",
        "rfb.mutation.strong-mind",
        "rfb.mutation.limp",
        "rfb.mutation.bad-luck",
        "rfb.mutation.good-luck",
        "rfb.mutation.easy-tiring",
        "rfb.mutation.impotence",
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
    for (id, blocker) in [
        (
            "rfb.mutation.astral-guide",
            "player-teleport-energy-cost-across-abilities-and-items",
        ),
        (
            "rfb.mutation.easy-tiring2",
            "fatigue-minislow-state-recovery-and-magic-ranged-consumers",
        ),
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "blocked", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([blocker]), "{id}");
    }
}

#[test]
fn sixth_passive_mutation_batch_completes_current_consumers() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );

    assert_eq!(
        catalog
            .mutation("rfb.mutation.unyielding")
            .unwrap()
            .max_hp_per_level,
        1
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.potion-chugger")
            .unwrap()
            .potion_energy_multiplier,
        Some(MutationRatioDefinition {
            numerator: 1,
            denominator: 2,
        })
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.sacred-vitality")
            .unwrap()
            .healing_bonus_percent,
        20
    );
    let fell_sorcery = catalog.mutation("rfb.mutation.fell-sorcery").unwrap();
    assert_eq!(
        (
            fell_sorcery.modifiers.strength,
            fell_sorcery.modifiers.dexterity,
            fell_sorcery.modifiers.constitution,
            fell_sorcery.modifiers.spell_power_bonus,
        ),
        (-1, -1, -1, 2)
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.weapon-skills")
            .unwrap()
            .weapon_proficiency_maximum,
        Some(8_000)
    );
    assert!(
        catalog
            .mutation("rfb.mutation.infernal-deal")
            .unwrap()
            .infernal_deal
    );
    assert!(
        catalog
            .mutation("rfb.mutation.demonic-grasp")
            .unwrap()
            .device_charge_drain_immunity
    );

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let entries = ledger["mutations"].as_array().expect("mutation entries");
    for id in [
        "rfb.mutation.fast-learner",
        "rfb.mutation.untouchable",
        "rfb.mutation.loremaster",
        "rfb.mutation.arcane-mastery",
        "rfb.mutation.one-with-magic",
        "rfb.mutation.merchants-friend",
        "rfb.mutation.fleet-of-foot",
        "rfb.mutation.weird-mind",
        "rfb.mutation.black-marketeer",
        "rfb.mutation.speed-reader",
        "rfb.mutation.tread-softly",
        "rfb.mutation.strong-mind",
        "rfb.mutation.unyielding",
        "rfb.mutation.potion-chugger",
        "rfb.mutation.sacred-vitality",
        "rfb.mutation.fell-sorcery",
        "rfb.mutation.weapon-skills",
        "rfb.mutation.infernal-deal",
        "rfb.mutation.demonic-grasp",
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
}

#[test]
fn active_demigod_talents_preserve_original_hooks_and_activation_parameters() {
    let pack = original_pack_path();
    let catalog = ContentCatalog::from_artifact(
        compile_pack_dir(&pack).expect("original pack should compile"),
    );
    assert!(
        catalog
            .mutation("rfb.mutation.subtle-casting")
            .unwrap()
            .suppresses_distant_spell_anger
    );
    assert!(
        catalog
            .mutation("rfb.mutation.peerless-sniper")
            .unwrap()
            .suppresses_distant_projectile_anger
    );
    assert!(
        catalog
            .mutation("rfb.mutation.evasion")
            .unwrap()
            .evades_innate_monster_attacks
    );
    assert!(
        catalog
            .mutation("rfb.mutation.cult-of-personality")
            .unwrap()
            .cult_of_personality
    );
    assert!(
        catalog
            .mutation("rfb.mutation.fantastic-frenzy")
            .unwrap()
            .preserves_melee_energy_on_kill
    );
    for (mutation_id, level, attribute, cost, failure, ability_id) in [
        (
            "rfb.mutation.peerless-tracker",
            20,
            TechniqueAttribute::Wisdom,
            25,
            40,
            "rfb.ability.mutation.peerless-tracker",
        ),
        (
            "rfb.mutation.fantastic-frenzy",
            40,
            TechniqueAttribute::Strength,
            50,
            80,
            "rfb.ability.mutation.fantastic-frenzy",
        ),
    ] {
        let activation = catalog
            .mutation(mutation_id)
            .and_then(|mutation| mutation.activation.as_ref())
            .unwrap_or_else(|| panic!("{mutation_id}"));
        assert_eq!(activation.minimum_level, level, "{mutation_id}");
        assert_eq!(activation.governing_attribute, attribute, "{mutation_id}");
        assert_eq!(activation.cost, cost, "{mutation_id}");
        assert_eq!(activation.base_failure_percent, failure, "{mutation_id}");
        assert_eq!(activation.ability_id, ability_id, "{mutation_id}");
    }

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    for id in [
        "rfb.mutation.subtle-casting",
        "rfb.mutation.peerless-sniper",
        "rfb.mutation.evasion",
        "rfb.mutation.peerless-tracker",
        "rfb.mutation.cult-of-personality",
        "rfb.mutation.fantastic-frenzy",
    ] {
        let entry = ledger["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
    let human = catalog.race("demo.race.rfb-human").unwrap();
    let talent = human
        .level_mutation_rewards
        .iter()
        .find(|reward| reward.id == "human-talent")
        .expect("Human should choose a demigod talent at level 20");
    assert_eq!(talent.minimum_level, 20);
    let RaceMutationSelectionDefinition::Choice { mutation_ids } = &talent.selection else {
        panic!("Human talent should be a choice");
    };
    assert_eq!(
        mutation_ids.iter().map(String::as_str).collect::<Vec<_>>(),
        [
            "rfb.mutation.fast-learner",
            "rfb.mutation.weapon-skills",
            "rfb.mutation.subtle-casting",
            "rfb.mutation.peerless-sniper",
            "rfb.mutation.unyielding",
            "rfb.mutation.untouchable",
            "rfb.mutation.loremaster",
            "rfb.mutation.arcane-mastery",
            "rfb.mutation.evasion",
            "rfb.mutation.potion-chugger",
            "rfb.mutation.one-with-magic",
            "rfb.mutation.peerless-tracker",
            "rfb.mutation.infernal-deal",
            "rfb.mutation.fell-sorcery",
            "rfb.mutation.sacred-vitality",
            "rfb.mutation.cult-of-personality",
            "rfb.mutation.fleet-of-foot",
            "rfb.mutation.demonic-grasp",
            "rfb.mutation.weird-mind",
            "rfb.mutation.fantastic-frenzy",
        ]
    );
    assert!(mutation_ids.iter().all(|id| {
        ledger["mutations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == id.as_str() && entry["status"] == "active")
    }));
}

#[test]
fn mutation_transaction_metadata_rejects_duplicate_order_and_invalid_removals() {
    let pack = original_pack_path();
    let artifact = compile_pack_dir(&pack).expect("original pack should compile");
    let first_id = artifact.content.mutations[0].id.clone();

    let mut duplicate_order = artifact.content.clone();
    duplicate_order.mutations[1].source_index = duplicate_order.mutations[0].source_index;
    assert!(matches!(
        encode_content(duplicate_order),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut self_removal = artifact.content.clone();
    self_removal.mutations[0].removes_on_gain = vec![first_id];
    assert!(matches!(
        encode_content(self_removal),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut dangling = artifact.content.clone();
    dangling.mutations[0].removes_on_gain = vec!["rfb.mutation.unknown".to_owned()];
    assert!(matches!(
        encode_content(dangling),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut invalid_bonus = artifact.content;
    invalid_bonus.mutations[0].armor_class = 1_000_001;
    assert!(matches!(
        encode_content(invalid_bonus),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut invalid_sense = compile_pack_dir(&pack)
        .expect("original pack should compile")
        .content;
    invalid_sense.mutations[0].infravision = 65;
    assert!(matches!(
        encode_content(invalid_sense),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut invalid_regeneration = compile_pack_dir(&pack)
        .expect("original pack should compile")
        .content;
    invalid_regeneration.mutations[0].regeneration_rate_modifier_percent = 1_001;
    assert!(matches!(
        encode_content(invalid_regeneration),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut invalid_innate = compile_pack_dir(&pack)
        .expect("original pack should compile")
        .content;
    invalid_innate.mutations[0].innate_attack = Some(MutationInnateAttackDefinition {
        name: String::new(),
        to_hit: 0,
        to_damage: 0,
        damage_dice: 1,
        damage_sides: 1,
        damage_type: ActorDamageType::Physical,
        weight_tenths_pound: 1,
    });
    assert!(matches!(
        encode_content(invalid_innate),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut invalid_ratio = compile_pack_dir(&pack)
        .expect("original pack should compile")
        .content;
    invalid_ratio.mutations[0].movement_energy_multiplier = Some(MutationRatioDefinition {
        numerator: 1,
        denominator: 0,
    });
    assert!(matches!(
        encode_content(invalid_ratio),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut invalid_periodic = compile_pack_dir(&pack)
        .expect("original pack should compile")
        .content;
    invalid_periodic.mutations[0].periodic_effect =
        Some(MutationPeriodicEffectDefinition::ApplyStatus {
            trigger_one_in: 0,
            skip_if_present: false,
            status_kind_id: "rfb.status.test".to_owned(),
            intensity: 1,
            duration_ticks: 1,
            duration_dice: 0,
            duration_sides: 0,
            stacking: AbilityStatusStackingDefinition::Replace,
        });
    assert!(matches!(
        encode_content(invalid_periodic),
        Err(ContentError::InvalidMutation(_))
    ));
}

#[test]
fn mutation_activation_is_bounded_unique_and_uses_an_unowned_ability() {
    let pack = original_pack_path();
    let artifact = compile_pack_dir(&pack).expect("original pack should compile");
    let mutation_id = artifact.content.mutations[0].id.clone();
    let activation = InnatePowerDefinition {
        minimum_level: 1,
        governing_attribute: TechniqueAttribute::Constitution,
        cost: 0,
        cost_scaling: None,
        base_failure_percent: 30,
        minimum_failure_percent: None,
        ability_id: "demo.ability.warrens-scare".to_owned(),
    };

    let mut valid = artifact.content.clone();
    valid.mutations[0].activation = Some(activation.clone());
    encode_content(valid).expect("a mutation may grant an otherwise unowned ability");

    let mut invalid_level = artifact.content.clone();
    invalid_level.mutations[0].activation = Some(InnatePowerDefinition {
        minimum_level: 0,
        ..activation.clone()
    });
    assert!(matches!(
        encode_content(invalid_level),
        Err(ContentError::InvalidMutation(id)) if id == mutation_id
    ));

    let mut invalid_minimum = artifact.content.clone();
    invalid_minimum.mutations[0].activation = Some(InnatePowerDefinition {
        minimum_failure_percent: Some(31),
        ..activation.clone()
    });
    assert!(matches!(
        encode_content(invalid_minimum),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut dangling = artifact.content.clone();
    dangling.mutations[0].activation = Some(InnatePowerDefinition {
        ability_id: "rfb.ability.missing".to_owned(),
        ..activation.clone()
    });
    assert!(matches!(
        encode_content(dangling),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut ordinary = artifact.content.clone();
    ordinary.mutations[0].activation = Some(InnatePowerDefinition {
        ability_id: "demo.ability.death-dark-bolt".to_owned(),
        ..activation.clone()
    });
    assert!(matches!(
        encode_content(ordinary),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut duplicate = artifact.content;
    duplicate.mutations[0].activation = Some(activation.clone());
    duplicate.mutations[1].activation = Some(activation);
    assert!(matches!(
        encode_content(duplicate),
        Err(ContentError::InvalidMutation(_))
    ));
}

#[test]
fn race_abilities_are_bounded_unique_and_use_unowned_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let race_id = "demo.race.rfb-human";
    let activation = InnatePowerDefinition {
        minimum_level: 8,
        governing_attribute: TechniqueAttribute::Strength,
        cost: 10,
        cost_scaling: None,
        base_failure_percent: 30,
        minimum_failure_percent: None,
        ability_id: "rfb.ability.race.berserk".to_owned(),
    };

    let mut valid = artifact.content.clone();
    valid
        .races
        .iter_mut()
        .find(|race| race.id == race_id)
        .expect("Human race should exist")
        .abilities
        .push(activation.clone());
    encode_content(valid).expect("a race may grant an otherwise unowned ability");

    for invalid_activation in [
        InnatePowerDefinition {
            minimum_level: 0,
            ..activation.clone()
        },
        InnatePowerDefinition {
            minimum_failure_percent: Some(31),
            ..activation.clone()
        },
        InnatePowerDefinition {
            ability_id: "rfb.ability.missing".to_owned(),
            ..activation.clone()
        },
        InnatePowerDefinition {
            ability_id: "demo.ability.death-dark-bolt".to_owned(),
            ..activation.clone()
        },
    ] {
        let mut invalid = artifact.content.clone();
        invalid
            .races
            .iter_mut()
            .find(|race| race.id == race_id)
            .expect("Human race should exist")
            .abilities
            .push(invalid_activation);
        assert!(matches!(
            encode_content(invalid),
            Err(ContentError::InvalidCharacterSource(id)) if id == race_id
        ));
    }

    let mut duplicate = artifact.content;
    let race = duplicate
        .races
        .iter_mut()
        .find(|race| race.id == race_id)
        .expect("Human race should exist");
    race.abilities = vec![activation.clone(), activation];
    assert!(matches!(
        encode_content(duplicate),
        Err(ContentError::InvalidCharacterSource(id)) if id == race_id
    ));
}

#[test]
fn active_mutation_batches_are_bound_to_authoritative_abilities() {
    let pack = original_pack_path();
    let artifact = compile_pack_dir(&pack).expect("original pack should compile");
    let catalog = ContentCatalog::from_artifact(artifact);
    let expected = [
        ("spit-acid", 9, TechniqueAttribute::Dexterity, 9, 30),
        ("br-fire", 20, TechniqueAttribute::Constitution, 0, 40),
        ("hypn-gaze", 12, TechniqueAttribute::Charisma, 12, 40),
        ("telekinesis", 9, TechniqueAttribute::Wisdom, 9, 40),
        ("teleport", 7, TechniqueAttribute::Wisdom, 7, 30),
        ("mind-blast", 5, TechniqueAttribute::Wisdom, 3, 30),
        ("radiation", 15, TechniqueAttribute::Constitution, 15, 30),
        ("vampirism", 2, TechniqueAttribute::Constitution, 1, 30),
        ("smell-metal", 3, TechniqueAttribute::Intelligence, 2, 30),
        ("smell-monsters", 5, TechniqueAttribute::Intelligence, 4, 30),
        ("blink", 3, TechniqueAttribute::Wisdom, 3, 30),
        ("swap-pos", 15, TechniqueAttribute::Dexterity, 12, 40),
        ("shriek", 20, TechniqueAttribute::Constitution, 14, 40),
        ("illumine", 3, TechniqueAttribute::Intelligence, 2, 30),
        ("det-curse", 7, TechniqueAttribute::Wisdom, 14, 30),
        ("berserk", 8, TechniqueAttribute::Strength, 8, 50),
        ("resist", 25, TechniqueAttribute::Constitution, 10, 50),
        ("dazzle", 7, TechniqueAttribute::Charisma, 15, 60),
        ("laser-eye", 7, TechniqueAttribute::Wisdom, 10, 50),
        ("recall", 17, TechniqueAttribute::Intelligence, 50, 70),
        ("banish", 25, TechniqueAttribute::Wisdom, 25, 70),
        ("cold-touch", 2, TechniqueAttribute::Constitution, 2, 30),
        ("eat-rock", 8, TechniqueAttribute::Constitution, 12, 40),
        ("polymorph", 18, TechniqueAttribute::Constitution, 20, 50),
        ("midas-touch", 10, TechniqueAttribute::Intelligence, 5, 70),
        ("grow-mold", 1, TechniqueAttribute::Constitution, 6, 60),
        ("earthquake", 12, TechniqueAttribute::Strength, 12, 50),
        ("eat-magic", 17, TechniqueAttribute::Wisdom, 1, 80),
        ("weigh-magic", 6, TechniqueAttribute::Intelligence, 6, 50),
        ("sterility", 12, TechniqueAttribute::Charisma, 23, 70),
        ("panic-hit", 10, TechniqueAttribute::Dexterity, 12, 60),
    ];
    for (suffix, minimum_level, attribute, cost, failure) in expected {
        let mutation_id = format!("rfb.mutation.{suffix}");
        let ability_id = format!("rfb.ability.mutation.{suffix}");
        let activation = catalog
            .mutation(&mutation_id)
            .and_then(|mutation| mutation.activation.as_ref())
            .unwrap_or_else(|| panic!("{mutation_id}"));
        assert_eq!(activation.minimum_level, minimum_level, "{mutation_id}");
        assert_eq!(activation.governing_attribute, attribute, "{mutation_id}");
        assert_eq!(activation.cost, cost, "{mutation_id}");
        assert_eq!(activation.base_failure_percent, failure, "{mutation_id}");
        assert_eq!(activation.ability_id, ability_id, "{mutation_id}");
        assert!(catalog.ability(&ability_id).is_some(), "{ability_id}");
    }

    assert_eq!(
        catalog
            .mutation("rfb.mutation.spit-acid")
            .unwrap()
            .activation
            .as_ref()
            .unwrap()
            .cost_scaling,
        Some(InnatePowerCostScalingDefinition {
            curve: InnatePowerCostScalingCurveDefinition::Step,
            start_level: 5,
            level_interval: 5,
            amount: 1,
            divisor: 1,
            round_up: false,
            linear_weight: 1,
            quadratic_weight: 0,
            cubic_weight: 0,
        })
    );
    assert_eq!(
        catalog
            .mutation("rfb.mutation.eat-magic")
            .unwrap()
            .activation
            .as_ref()
            .unwrap()
            .minimum_failure_percent,
        Some(11)
    );

    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(pack.join("legacy-mutation-plan.json")).expect("ledger should read"),
    )
    .expect("ledger should parse");
    let entries = ledger["mutations"].as_array().expect("mutation entries");
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry["status"] == "active")
            .count(),
        144
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| {
                entry["status"] == "active"
                    && entry["randomWeight"]
                        .as_u64()
                        .is_some_and(|weight| weight > 0)
            })
            .count(),
        104
    );
    assert!(entries.iter().all(|entry| {
        entry["status"] == "active"
            || entry["randomWeight"]
                .as_u64()
                .is_none_or(|weight| weight == 0)
            || catalog
                .mutation(entry["id"].as_str().expect("mutation ID"))
                .is_some_and(|mutation| !mutation.random_selection_enabled)
    }));
}

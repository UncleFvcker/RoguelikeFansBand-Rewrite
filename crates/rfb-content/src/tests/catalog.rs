use super::*;

#[test]
fn compiled_catalog_indexes_current_rfb_content() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

    assert_eq!(catalog.pack_id(), "rfb.demo.original-v1");
    assert_eq!(catalog.pack_version(), "1.274.0");
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
    assert!(catalog.class("demo.class.warrior").is_some());
    assert!(catalog.class("demo.class.high-mage").is_some());
    assert!(catalog.build("demo.build.high-mage-death").is_some());
    assert!(catalog.class("demo.class.archer").is_some());
    assert!(catalog.build("demo.build.archer").is_some());
    assert!(catalog.class("demo.class.paladin").is_some());
    assert!(catalog.build("demo.build.paladin-death").is_some());
    let paladin = catalog.class("demo.class.paladin").expect("Paladin class");
    assert!(
        paladin
            .abilities
            .iter()
            .any(|ability| ability.ability_id == "demo.ability.paladin-hell-lance")
    );
    assert_eq!(paladin.level_resistances[0].minimum_level, 40);
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
    assert!(catalog.affix("rfb-legacy.affix.slaying").is_some());
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
        119
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
    let id = "rfb.mutation.draconian-resistance";
    let entry = entries
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap_or_else(|| panic!("{id}"));
    assert_eq!(entry["status"], "blocked", "{id}");
    assert_eq!(
        entry["blockers"],
        serde_json::json!(["draconian-subrace-identity"]),
        "{id}"
    );
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
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
    let shield = entries
        .iter()
        .find(|entry| entry["id"] == "rfb.mutation.draconian-shield")
        .expect("draconian shield ledger entry");
    assert_eq!(shield["status"], "blocked");
    assert_eq!(
        shield["blockers"],
        serde_json::json!(["draconian-subrace-identity-and-aura-selection"])
    );
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
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap_or_else(|| panic!("{id}"));
        assert_eq!(entry["status"], "active", "{id}");
        assert_eq!(entry["blockers"], serde_json::json!([]), "{id}");
    }
    for (id, blocker) in [
        ("rfb.mutation.weapon-skills", "weapon-proficiency-system"),
        ("rfb.mutation.fell-sorcery", "spell-power-scaling"),
        (
            "rfb.mutation.vortex-melee",
            "vortex-race-innate-attack-identity",
        ),
        (
            "rfb.mutation.human-str",
            "player-critical-hit-balance-penalty",
        ),
        (
            "rfb.mutation.human-dex",
            "combat-strain-and-temporary-dexterity-loss",
        ),
        (
            "rfb.mutation.human-chr",
            "spell-failure-and-ranged-device-accuracy-modifiers",
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
    let activation = MutationActivationDefinition {
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
    invalid_level.mutations[0].activation = Some(MutationActivationDefinition {
        minimum_level: 0,
        ..activation.clone()
    });
    assert!(matches!(
        encode_content(invalid_level),
        Err(ContentError::InvalidMutation(id)) if id == mutation_id
    ));

    let mut invalid_minimum = artifact.content.clone();
    invalid_minimum.mutations[0].activation = Some(MutationActivationDefinition {
        minimum_failure_percent: Some(31),
        ..activation.clone()
    });
    assert!(matches!(
        encode_content(invalid_minimum),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut dangling = artifact.content.clone();
    dangling.mutations[0].activation = Some(MutationActivationDefinition {
        ability_id: "rfb.ability.missing".to_owned(),
        ..activation.clone()
    });
    assert!(matches!(
        encode_content(dangling),
        Err(ContentError::InvalidMutation(_))
    ));

    let mut ordinary = artifact.content.clone();
    ordinary.mutations[0].activation = Some(MutationActivationDefinition {
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
        Some(MutationActivationCostScalingDefinition {
            start_level: 5,
            level_interval: 5,
            amount: 1,
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
        119
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

use std::collections::BTreeSet;

use super::*;

#[test]
fn create_item_effect_accepts_only_bounded_plain_item_references() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let make_content = |item_kind_id: &str, quantity| {
        let mut content = artifact.content.clone();
        content
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.arcane-satisfy-hunger")
            .expect("Satisfy Hunger should provide a self-target fixture")
            .effect = AbilityEffectDefinition::CreateItem {
            item_kind_id: item_kind_id.to_owned(),
            quantity,
        };
        content
    };

    validate_and_normalize(&mut make_content("demo.item.ration-of-food", 1))
        .expect("a bounded plain item should be creatable");
    assert!(matches!(
        validate_and_normalize(&mut make_content("demo.item.ration-of-food", 101)),
        Err(ContentError::InvalidItemModifiers(id)) if id == "demo.item.ration-of-food"
    ));
    assert!(matches!(
        validate_and_normalize(&mut make_content("demo.item.crisdurian", 1)),
        Err(ContentError::InvalidItemModifiers(id)) if id == "demo.item.crisdurian"
    ));
    assert!(matches!(
        validate_and_normalize(&mut make_content("demo.item.missing", 1)),
        Err(ContentError::DanglingReference { owner, target })
            if owner == "demo.ability.arcane-satisfy-hunger"
                && target == "demo.item.missing"
    ));
}

#[test]
fn abilities_validate_actor_detection_control_and_level_scaling() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut valid = artifact.content.clone();
    let malediction = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("fixture should contain level-scaled damage");
    assert!(matches!(
        malediction.effect,
        AbilityEffectDefinition::Malediction {
            damage_dice: 3,
            damage_sides: 4,
            damage_bonus: 0,
        }
    ));
    assert_eq!(malediction.level_scaling.len(), 1);
    let unlife = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("fixture should contain actor detection");
    assert!(matches!(
        unlife.effect,
        AbilityEffectDefinition::Detect {
            subject: AbilityDetectSubjectDefinition::Actor,
            persistent: false,
            ..
        }
    ));
    validate_and_normalize(&mut valid).expect("P54 abilities should compile");

    let mut duplicate = artifact.content.clone();
    let malediction = duplicate
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("fixture should contain level-scaled damage");
    malediction
        .level_scaling
        .push(malediction.level_scaling[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut duplicate),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut out_of_bounds = artifact.content.clone();
    out_of_bounds
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-horrify")
        .expect("fixture should contain a scaled sequence")
        .level_scaling[0]
        .effect_index = 2;
    assert!(matches!(
        validate_and_normalize(&mut out_of_bounds),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut persistent_actor_detection = artifact.content.clone();
    let AbilityEffectDefinition::Detect { persistent, .. } = &mut persistent_actor_detection
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("fixture should contain actor detection")
        .effect
    else {
        panic!("detect unlife should use actor detection");
    };
    *persistent = true;
    assert!(matches!(
        validate_and_normalize(&mut persistent_actor_detection),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut missing_control_category = artifact.content;
    let AbilityEffectDefinition::Control { category, .. } = &mut missing_control_category
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-enslave-undead")
        .expect("fixture should contain actor control")
        .effect
    else {
        panic!("enslave undead should use actor control");
    };
    *category = "missing-category".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut missing_control_category),
        Err(ContentError::InvalidAbility(_))
    ));
}

#[test]
fn zero_ability_bases_require_matching_level_scaling() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for ability_id in [
        "demo.ability.death-death-ray",
        "demo.ability.death-raise-dead",
        "demo.ability.death-esoteria",
        "demo.ability.death-mass-genocide",
    ] {
        let mut invalid = artifact.content.clone();
        invalid
            .abilities
            .iter_mut()
            .find(|ability| ability.id == ability_id)
            .unwrap_or_else(|| panic!("fixture should contain {ability_id}"))
            .level_scaling
            .clear();
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidAbility(id)) if id == ability_id
        ));
    }
}

#[test]
fn arcane_first_book_keeps_the_original_spell_table_and_narrow_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.cantrips-for-beginners")
        .expect("Arcane first book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("arcane"));
    assert_eq!(book.rank, Some(1));
    assert_eq!(book.ability_ids.len(), 8);

    let expected = [
        ("demo.ability.arcane-zap", 1, 1, 15, 4),
        ("demo.ability.arcane-wizard-lock", 1, 1, 23, 5),
        ("demo.ability.arcane-detect-invisibility", 1, 1, 23, 4),
        ("demo.ability.arcane-detect-monsters", 1, 1, 23, 5),
        ("demo.ability.arcane-blink", 2, 1, 23, 10),
        ("demo.ability.arcane-light-area", 3, 2, 33, 18),
        ("demo.ability.arcane-trap-door-destruction", 4, 4, 23, 28),
        ("demo.ability.arcane-cure-light-wounds", 5, 4, 33, 25),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let lock = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-wizard-lock")
        .expect("Wizard Lock should compile");
    assert!(matches!(
        lock.effect,
        AbilityEffectDefinition::TerrainBeam {
            operation: AbilityTerrainBeamOperationDefinition::JamDoors
        }
    ));
    let cure = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-cure-light-wounds")
        .expect("Cure Light Wounds should compile");
    assert!(matches!(
        &cure.effect,
        AbilityEffectDefinition::Sequence { effects }
            if matches!(effects.as_slice(), [
                AbilityEffectDefinition::HealDice { dice: 2, sides: 8 },
                AbilityEffectDefinition::ReduceStatus { status_kind_id, amount: 10, .. }
            ] if status_kind_id == "rfb.status.bleeding")
    ));
}

#[test]
fn life_first_book_keeps_the_original_spell_table_allocation_and_final_scaling() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.book-of-common-prayer")
        .expect("Book of Common Prayer should compile");
    assert_eq!(book.realm_id.as_deref(), Some("life"));
    assert_eq!(book.rank, Some(1));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.book-of-common-prayer")
        .expect("Book of Common Prayer item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (10, 30, 100, Some("demo.ability-book.book-of-common-prayer"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Book of Common Prayer should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (10, 30, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.life-cure-light-wounds", 1, 1, 20, 4),
        ("demo.ability.life-bless", 2, 2, 25, 4),
        ("demo.ability.life-regeneration", 3, 3, 25, 4),
        ("demo.ability.life-call-light", 4, 4, 25, 4),
        ("demo.ability.life-detect-doors-and-traps", 5, 5, 25, 4),
        ("demo.ability.life-cure-medium-wounds", 6, 5, 30, 4),
        ("demo.ability.life-cure-poison", 9, 9, 30, 3),
        ("demo.ability.life-satisfy-hunger", 12, 10, 35, 3),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    for id in [
        "demo.ability.life-cure-light-wounds",
        "demo.ability.life-cure-medium-wounds",
    ] {
        let cure = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        assert_eq!(
            cure.spell_power_fields
                .iter()
                .map(|field| (field.effect_index, field.field))
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([(0, AbilitySpellPowerField::FinalHealing)])
        );
    }
    let call_light = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.life-call-light")
        .expect("Call Light should compile");
    assert_eq!(
        call_light
            .spell_power_fields
            .iter()
            .map(|field| (field.effect_index, field.field))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            (0, AbilitySpellPowerField::FinalDamage),
            (0, AbilitySpellPowerField::Radius),
        ])
    );
}

#[test]
fn daemon_first_book_keeps_the_original_identity_spell_table_and_effect_boundaries() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.dark-incantations")
        .expect("Dark Incantations should compile");
    assert_eq!(book.realm_id.as_deref(), Some("daemon"));
    assert_eq!(book.rank, Some(1));
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/dark-incantations.json"))
            .expect("Dark Incantations source should be readable"),
    )
    .expect("Dark Incantations source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.daemon-magic-missile",
            "demo.ability.daemon-detect-unlife",
            "demo.ability.daemon-evil-bless",
            "demo.ability.daemon-resist-fire",
            "demo.ability.daemon-horrify",
            "demo.ability.daemon-nether-bolt",
            "demo.ability.daemon-summon-manes",
            "demo.ability.daemon-hellish-flame",
        ])
    );
    assert!(book.ability_ids.iter().all(|id| {
        content
            .abilities
            .iter()
            .find(|ability| ability.id == *id)
            .expect("book ability should compile")
            .effect
            .ordered_effects()
            .iter()
            .all(|effect| !matches!(effect, AbilityEffectDefinition::NoOp { .. }))
    }));

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.dark-incantations")
        .expect("Dark Incantations item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (10, 30, 100, Some("demo.ability-book.dark-incantations"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Dark Incantations should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (10, 30, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.daemon-magic-missile", 1, 1, 15, 4),
        ("demo.ability.daemon-detect-unlife", 1, 1, 15, 4),
        ("demo.ability.daemon-evil-bless", 2, 2, 15, 4),
        ("demo.ability.daemon-resist-fire", 3, 4, 20, 1),
        ("demo.ability.daemon-horrify", 5, 4, 30, 1),
        ("demo.ability.daemon-nether-bolt", 7, 5, 40, 6),
        ("demo.ability.daemon-summon-manes", 9, 8, 25, 5),
        ("demo.ability.daemon-hellish-flame", 11, 11, 40, 5),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let summon = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.daemon-summon-manes")
        .expect("Summon Manes should compile");
    assert!(matches!(
        summon.effect,
        AbilityEffectDefinition::SummonCategory {
            ref category,
            maximum_level: 0,
            count_dice: 1,
            count_sides: 1,
            friendly_group_chance_percent: 100,
            group_count_dice: 1,
            group_count_sides: 10,
            duration_turns: 0,
            ..
        } if category == "manes"
    ));
    assert_eq!(
        summon
            .spell_power_fields
            .iter()
            .map(|field| (field.effect_index, field.field))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(0, AbilitySpellPowerField::SummonMaximumLevel)])
    );

    let nether_bolt = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.daemon-nether-bolt")
        .expect("Nether Bolt should compile");
    assert!(!nether_bolt.affects_ground_items);

    let flame = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.daemon-hellish-flame")
        .expect("Hellish Flame should compile");
    assert!(flame.affects_ground_items);
    assert!(matches!(
        flame.effect,
        AbilityEffectDefinition::AreaDamage {
            damage_dice: 3,
            damage_sides: 6,
            damage_type: ActorDamageType::HellFire,
            radius: 2,
            ..
        }
    ));
}

#[test]
fn crusade_first_book_keeps_original_identity_spell_table_and_effect_boundaries() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.rites-of-initiation")
        .expect("Rites of Initiation should compile");
    assert_eq!(book.realm_id.as_deref(), Some("crusade"));
    assert_eq!(book.rank, Some(1));
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/rites-of-initiation.json"))
            .expect("Rites of Initiation source should be readable"),
    )
    .expect("Rites of Initiation source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.crusade-punishment",
            "demo.ability.crusade-detect-evil",
            "demo.ability.crusade-remove-fear",
            "demo.ability.crusade-scare-monster",
            "demo.ability.crusade-sanctuary",
            "demo.ability.crusade-portal",
            "demo.ability.crusade-star-dust",
            "demo.ability.crusade-purification",
        ])
    );

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.rites-of-initiation")
        .expect("Rites of Initiation item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (10, 30, 100, Some("demo.ability-book.rites-of-initiation"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Rites of Initiation should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (10, 30, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    for (id, level, mana, failure, experience) in [
        ("demo.ability.crusade-punishment", 1, 1, 15, 4),
        ("demo.ability.crusade-detect-evil", 1, 1, 10, 4),
        ("demo.ability.crusade-remove-fear", 3, 2, 25, 4),
        ("demo.ability.crusade-scare-monster", 4, 4, 30, 2),
        ("demo.ability.crusade-sanctuary", 5, 4, 34, 4),
        ("demo.ability.crusade-portal", 7, 6, 30, 2),
        ("demo.ability.crusade-star-dust", 8, 8, 45, 6),
        ("demo.ability.crusade-purification", 10, 8, 45, 4),
    ] {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let punishment = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crusade-punishment")
        .expect("Punishment should compile");
    assert!(matches!(
        punishment.effect,
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice: 3,
            damage_sides: 4,
            damage_type: ActorDamageType::Electricity,
            beam_chance_modifier: -10,
            ..
        }
    ));
    let sanctuary = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crusade-sanctuary")
        .expect("Sanctuary should compile");
    assert!(matches!(
        sanctuary.effect,
        AbilityEffectDefinition::Sanctuary {
            power: 1,
            radius: 1
        }
    ));
    let stardust = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crusade-star-dust")
        .expect("Star Dust should compile");
    assert!(matches!(
        stardust.effect,
        AbilityEffectDefinition::Stardust {
            damage_dice: 3,
            damage_sides: 2,
            count: 10,
            deviation: 3
        }
    ));
    assert!(!stardust.affects_ground_items);
    assert_eq!(
        stardust
            .spell_power_fields
            .iter()
            .map(|field| (field.effect_index, field.field))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(0, AbilitySpellPowerField::DamageDice)])
    );
}

#[test]
fn crusade_second_book_keeps_original_identity_allocation_and_spell_table() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.ways-of-war")
        .expect("Ways of War should compile");
    assert_eq!(book.realm_id.as_deref(), Some("crusade"));
    assert_eq!(book.rank, Some(2));
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/ways-of-war.json"))
            .expect("Ways of War source should be readable"),
    )
    .expect("Ways of War source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.crusade-scatter-evil",
            "demo.ability.crusade-holy-orb",
            "demo.ability.crusade-exorcism",
            "demo.ability.crusade-remove-curse",
            "demo.ability.crusade-sense-unseen",
            "demo.ability.crusade-protection-from-evil",
            "demo.ability.crusade-judgment-thunder",
            "demo.ability.crusade-holy-word",
        ])
    );
    assert!(
        book.ability_ids
            .iter()
            .all(|id| { content.abilities.iter().any(|ability| ability.id == *id) })
    );

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.ways-of-war")
        .expect("Ways of War item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (20, 30, 1_000, Some("demo.ability-book.ways-of-war"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Ways of War should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (20, 50, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    for (id, level, mana, failure, experience) in [
        ("demo.ability.crusade-scatter-evil", 11, 11, 45, 10),
        ("demo.ability.crusade-holy-orb", 12, 11, 40, 4),
        ("demo.ability.crusade-exorcism", 15, 15, 35, 10),
        ("demo.ability.crusade-remove-curse", 16, 16, 55, 50),
        ("demo.ability.crusade-sense-unseen", 17, 16, 40, 7),
        ("demo.ability.crusade-protection-from-evil", 20, 18, 60, 10),
        ("demo.ability.crusade-judgment-thunder", 28, 30, 65, 15),
        ("demo.ability.crusade-holy-word", 36, 32, 75, 20),
    ] {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let scatter = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crusade-scatter-evil")
        .expect("Scatter Evil should compile");
    assert!(matches!(
        scatter.effect,
        AbilityEffectDefinition::TeleportAway {
            minimum_distance: 1,
            power: 100,
            stop_at_actor: true,
            target_category: Some(ref category),
        } if category == "evil"
    ));
    let exorcism = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crusade-exorcism")
        .expect("Exorcism should compile");
    assert!(matches!(
        exorcism.effect,
        AbilityEffectDefinition::Sequence { ref effects }
            if matches!(effects.as_slice(), [
                AbilityEffectDefinition::VisibleDamage { target_category: Some(undead), .. },
                AbilityEffectDefinition::VisibleDamage { target_category: Some(demon), .. },
                AbilityEffectDefinition::VisibleApplyStatus {
                    status_kind_id,
                    duration_dice: 3,
                    target_category: Some(evil),
                    ..
                }
            ] if undead == "undead" && demon == "demon" && evil == "evil" && status_kind_id == "rfb.status.fear")
    ));
}

#[test]
fn crusade_third_book_keeps_original_identity_allocation_and_spell_table() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.exorcism-and-dispelling")
        .expect("Exorcism and Dispelling should compile");
    assert_eq!(book.realm_id.as_deref(), Some("crusade"));
    assert_eq!(book.rank, Some(3));
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/exorcism-and-dispelling.json"))
            .expect("Exorcism and Dispelling source should be readable"),
    )
    .expect("Exorcism and Dispelling source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.crusade-unbarring-ways",
            "demo.ability.crusade-arrest",
            "demo.ability.crusade-angelic-cloak",
            "demo.ability.crusade-dispel-undead-and-demons",
            "demo.ability.crusade-dispel-evil",
            "demo.ability.crusade-holy-blade",
            "demo.ability.crusade-star-burst",
            "demo.ability.crusade-summon-angel",
        ])
    );

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.exorcism-and-dispelling")
        .expect("Exorcism and Dispelling item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (
            45,
            30,
            15_000,
            Some("demo.ability-book.exorcism-and-dispelling")
        )
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Exorcism and Dispelling should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (45, 90, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    for (id, level, mana, failure, experience) in [
        ("demo.ability.crusade-unbarring-ways", 4, 4, 23, 40),
        ("demo.ability.crusade-arrest", 13, 12, 35, 50),
        ("demo.ability.crusade-angelic-cloak", 15, 13, 55, 70),
        (
            "demo.ability.crusade-dispel-undead-and-demons",
            17,
            14,
            55,
            70,
        ),
        ("demo.ability.crusade-dispel-evil", 25, 20, 70, 120),
        ("demo.ability.crusade-holy-blade", 28, 65, 80, 100),
        ("demo.ability.crusade-star-burst", 32, 38, 70, 100),
        ("demo.ability.crusade-summon-angel", 38, 90, 75, 250),
    ] {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let arrest = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crusade-arrest")
        .expect("Arrest should compile");
    assert!(matches!(
        arrest.effect,
        AbilityEffectDefinition::ApplyStatus {
            ref status_kind_id,
            duration_ticks: 3,
            power: Some(2),
            ..
        } if status_kind_id == "rfb.status.paralysis"
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.crusade-summon-angel")
            .expect("Summon Angel should compile")
            .effect,
        AbilityEffectDefinition::AngelSummoning
    ));
    assert_eq!(
        content
            .affixes
            .iter()
            .find(|affix| affix.id == "demo.affix.holy-blade")
            .and_then(|affix| affix.slays.get(&SlayTarget::Evil)),
        Some(&SlayLevel::Slay)
    );
}

#[test]
fn crusade_fourth_book_keeps_original_identity_allocation_and_spell_table() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.wrath-of-god")
        .expect("Wrath of God should compile");
    assert_eq!(book.realm_id.as_deref(), Some("crusade"));
    assert_eq!(book.rank, Some(4));
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/wrath-of-god.json"))
            .expect("Wrath of God source should be readable"),
    )
    .expect("Wrath of God source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.crusade-heroism",
            "demo.ability.crusade-dispel-curse",
            "demo.ability.crusade-banish-evil",
            "demo.ability.crusade-armageddon",
            "demo.ability.crusade-an-eye-for-an-eye",
            "demo.ability.crusade-wrath-of-the-god",
            "demo.ability.crusade-divine-intervention",
            "demo.ability.crusade-crusade",
        ])
    );

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.wrath-of-god")
        .expect("Wrath of God item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (85, 30, 100_000, Some("demo.ability-book.wrath-of-god"))
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Wrath of God should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (85, u16::MAX, 33)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    for (id, level, mana, failure, experience) in [
        ("demo.ability.crusade-heroism", 8, 8, 50, 40),
        ("demo.ability.crusade-dispel-curse", 25, 25, 75, 250),
        ("demo.ability.crusade-banish-evil", 27, 24, 60, 100),
        ("demo.ability.crusade-armageddon", 28, 19, 70, 150),
        ("demo.ability.crusade-an-eye-for-an-eye", 34, 30, 80, 150),
        ("demo.ability.crusade-wrath-of-the-god", 39, 55, 75, 200),
        ("demo.ability.crusade-divine-intervention", 42, 85, 85, 200),
        ("demo.ability.crusade-crusade", 45, 90, 75, 250),
    ] {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.crusade-banish-evil")
            .expect("Banish Evil should compile")
            .effect,
        AbilityEffectDefinition::BanishEvil
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.crusade-wrath-of-the-god")
            .expect("Wrath of the God should compile")
            .effect,
        AbilityEffectDefinition::WrathOfGod
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.crusade-divine-intervention")
            .expect("Divine Intervention should compile")
            .effect,
        AbilityEffectDefinition::DivineIntervention
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.crusade-crusade")
            .expect("Crusade should compile")
            .effect,
        AbilityEffectDefinition::Crusade
    ));
}

#[test]
fn daemon_second_book_keeps_the_original_identity_allocation_and_spell_table() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.immortal-rituals")
        .expect("Immortal Rituals should compile");
    assert_eq!(book.realm_id.as_deref(), Some("daemon"));
    assert_eq!(book.rank, Some(2));
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/immortal-rituals.json"))
            .expect("Immortal Rituals source should be readable"),
    )
    .expect("Immortal Rituals source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.daemon-dominate-demon",
            "demo.ability.daemon-vision",
            "demo.ability.daemon-resist-nether",
            "demo.ability.daemon-plasma-bolt",
            "demo.ability.daemon-fire-ball",
            "demo.ability.daemon-fire-branding",
            "demo.ability.daemon-nether-ball",
            "demo.ability.daemon-summon-demon",
        ])
    );
    assert!(book.ability_ids.iter().all(|id| {
        content
            .abilities
            .iter()
            .find(|ability| ability.id == *id)
            .expect("book ability should compile")
            .effect
            .ordered_effects()
            .iter()
            .all(|effect| !matches!(effect, AbilityEffectDefinition::NoOp { .. }))
    }));

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.immortal-rituals")
        .expect("Immortal Rituals item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (20, 30, 1_000, Some("demo.ability-book.immortal-rituals"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Immortal Rituals should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (20, 50, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.daemon-dominate-demon", 13, 11, 35, 9),
        ("demo.ability.daemon-vision", 15, 13, 35, 10),
        ("demo.ability.daemon-resist-nether", 17, 15, 40, 11),
        ("demo.ability.daemon-plasma-bolt", 21, 12, 40, 12),
        ("demo.ability.daemon-fire-ball", 22, 13, 40, 12),
        ("demo.ability.daemon-fire-branding", 26, 65, 70, 8),
        ("demo.ability.daemon-nether-ball", 28, 25, 55, 15),
        ("demo.ability.daemon-summon-demon", 33, 65, 75, 40),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let effect = |id: &str| {
        &content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"))
            .effect
    };
    assert!(matches!(
        effect("demo.ability.daemon-dominate-demon"),
        AbilityEffectDefinition::Control { category, power: 2 } if category == "demon"
    ));
    assert!(matches!(
        effect("demo.ability.daemon-resist-nether"),
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            duration_ticks: 20,
            duration_dice: 1,
            duration_sides: 20,
            granted_resistances,
            ..
        } if status_kind_id == "rfb.status.resist-nether"
            && granted_resistances.get(&ActorDamageType::Nether)
                == Some(&ActorResistanceLevel::Resistant)
    ));
    assert!(matches!(
        effect("demo.ability.daemon-plasma-bolt"),
        AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice: 11,
            damage_sides: 8,
            damage_type: ActorDamageType::Plasma,
            beam_chance_percent: 0,
            beam_chance_modifier: 0,
            ..
        }
    ));
    assert!(matches!(
        effect("demo.ability.daemon-fire-ball"),
        AbilityEffectDefinition::AreaDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 55,
            damage_type: ActorDamageType::Fire,
            radius: 2,
            ..
        }
    ));
    assert!(matches!(
        effect("demo.ability.daemon-fire-branding"),
        AbilityEffectDefinition::BrandWeapon {
            brand: Some(WeaponBrand::Fire),
            resistance: Some(ActorDamageType::Fire),
            ..
        }
    ));
    assert!(matches!(
        effect("demo.ability.daemon-nether-ball"),
        AbilityEffectDefinition::AreaDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 100,
            damage_type: ActorDamageType::Nether,
            radius: 2,
            ..
        }
    ));
    assert!(matches!(
        effect("demo.ability.daemon-summon-demon"),
        AbilityEffectDefinition::DemonSummoning
    ));
    assert!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.daemon-fire-ball")
            .expect("Fire Ball should compile")
            .affects_ground_items
    );
    assert!(
        !content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.daemon-nether-ball")
            .expect("Nether Ball should compile")
            .affects_ground_items
    );
}

#[test]
fn daemon_third_book_keeps_the_original_identity_allocation_spell_table_and_demon_form() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.demonthoughts")
        .expect("Demonthoughts should compile");
    assert_eq!(book.realm_id.as_deref(), Some("daemon"));
    assert_eq!(book.rank, Some(3));
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/demonthoughts.json"))
            .expect("Demonthoughts source should be readable"),
    )
    .expect("Demonthoughts source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.daemon-devilish-eye",
            "demo.ability.daemon-devilish-cloak",
            "demo.ability.daemon-flow-of-lava",
            "demo.ability.daemon-plasma-ball",
            "demo.ability.daemon-polymorph-demon",
            "demo.ability.daemon-nether-wave",
            "demo.ability.daemon-kiss-of-succubus",
            "demo.ability.daemon-doom-hand",
        ])
    );

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.demonthoughts")
        .expect("Demonthoughts item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (45, 30, 15_000, Some("demo.ability-book.demonthoughts"))
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Demonthoughts should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (45, 90, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.daemon-devilish-eye", 9, 10, 35, 9),
        ("demo.ability.daemon-devilish-cloak", 12, 15, 70, 35),
        ("demo.ability.daemon-flow-of-lava", 22, 19, 70, 35),
        ("demo.ability.daemon-plasma-ball", 31, 26, 75, 150),
        ("demo.ability.daemon-polymorph-demon", 32, 35, 75, 200),
        ("demo.ability.daemon-nether-wave", 33, 32, 75, 100),
        ("demo.ability.daemon-kiss-of-succubus", 34, 35, 75, 40),
        ("demo.ability.daemon-doom-hand", 40, 70, 80, 250),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
        assert!(
            ability
                .effect
                .ordered_effects()
                .iter()
                .all(|effect| !matches!(effect, AbilityEffectDefinition::NoOp { .. }))
        );
    }

    let effect = |id: &str| {
        &content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"))
            .effect
    };
    assert!(matches!(
        effect("demo.ability.daemon-devilish-cloak"),
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            duration_ticks: 20,
            duration_dice: 1,
            duration_sides: 20,
            granted_resistances,
            ..
        } if status_kind_id == "rfb.status.fire-aura"
            && granted_resistances.get(&ActorDamageType::Acid)
                == Some(&ActorResistanceLevel::Resistant)
            && granted_resistances.get(&ActorDamageType::Fire)
                == Some(&ActorResistanceLevel::Resistant)
            && granted_resistances.get(&ActorDamageType::Poison)
                == Some(&ActorResistanceLevel::Resistant)
    ));
    assert!(matches!(
        effect("demo.ability.daemon-flow-of-lava"),
        AbilityEffectDefinition::LavaFlow {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 110,
            radius: 3,
            target_terrain_id,
        } if target_terrain_id == "demo.terrain.surface-lava-deep"
    ));
    assert!(matches!(
        effect("demo.ability.daemon-polymorph-demon"),
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            granted_race_id: Some(granted_race_id),
            ..
        } if status_kind_id == "rfb.status.demon-transformation"
            && granted_race_id == "demo.race.demon"
    ));
    assert!(matches!(
        effect("demo.ability.daemon-nether-wave"),
        AbilityEffectDefinition::Sequence { effects }
            if effects.len() == 2
                && matches!(effects[0], AbilityEffectDefinition::VisibleDamage { target_category: None, .. })
                && matches!(effects[1], AbilityEffectDefinition::VisibleDamage { target_category: Some(ref category), .. } if category == "good")
    ));
    assert!(matches!(
        effect("demo.ability.daemon-doom-hand"),
        AbilityEffectDefinition::DoomHand
    ));

    let race = content
        .races
        .iter()
        .find(|race| race.id == "demo.race.demon")
        .expect("the temporary demon race should compile");
    assert_eq!(race.legacy_index, Some(1000));
    assert_eq!(
        (
            race.modifiers.strength,
            race.modifiers.intelligence,
            race.modifiers.wisdom,
            race.modifiers.dexterity,
            race.modifiers.constitution,
            race.modifiers.charisma,
            race.modifiers.defense,
            race.modifiers.speed,
        ),
        (5, 3, 2, 3, 4, 3, 10, 3)
    );
    assert_eq!(
        (
            race.life_percent,
            race.experience_percent,
            race.base_hp,
            race.infravision,
            race.see_invisible,
        ),
        (106, 500, 24, 5, true)
    );
    assert_eq!(
        race.resistances.get(&ActorDamageType::Fire),
        Some(&ActorResistanceLevel::Strong)
    );
    let breath = race
        .abilities
        .first()
        .expect("demon should have its breath");
    assert_eq!(
        (
            breath.minimum_level,
            breath.cost,
            breath.base_failure_percent,
            breath.ability_id.as_str(),
        ),
        (15, 10, 70, "demo.ability.daemon-breath")
    );
    assert!(matches!(
        effect("demo.ability.daemon-breath"),
        AbilityEffectDefinition::RandomChoice { roll_sides: 2, branches, .. }
            if branches.len() == 2
                && branches.iter().all(|branch| matches!(branch.effect.as_ref(), AbilityEffectDefinition::ConeDamage { damage_dice: 0, damage_sides: 0, damage_bonus: 3, radius: 1, .. }))
    ));

    let mut invalid = content.clone();
    let lava = invalid
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.daemon-flow-of-lava")
        .expect("The Flow of Lava should compile");
    let AbilityEffectDefinition::LavaFlow {
        target_terrain_id, ..
    } = &mut lava.effect
    else {
        panic!("The Flow of Lava should retain its effect")
    };
    *target_terrain_id = "demo.terrain.missing-lava".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::DanglingReference { owner, target })
            if owner == "demo.ability.daemon-flow-of-lava"
                && target == "demo.terrain.missing-lava"
    ));

    let mut invalid = content.clone();
    invalid
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.daemon-breath")
        .expect("Demon breath should compile")
        .target
        .modes = vec![AbilityTargetModeDefinition::Entity];
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidAbility(id)) if id == "demo.ability.daemon-breath"
    ));
}

#[test]
fn daemon_fourth_book_completes_the_original_realm_and_keeps_composite_effects_explicit() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.hellfire-tome")
        .expect("Hellfire Tome should compile");
    assert_eq!(book.realm_id.as_deref(), Some("daemon"));
    assert_eq!(book.rank, Some(4));
    assert_eq!(book.ability_ids.len(), 8);
    let source_book: serde_json::Value = serde_json::from_slice(
        &std::fs::read(original_pack_path().join("abilityBooks/hellfire-tome.json"))
            .expect("Hellfire Tome source should be readable"),
    )
    .expect("Hellfire Tome source should be JSON");
    assert_eq!(
        source_book["abilityIds"],
        serde_json::json!([
            "demo.ability.daemon-raise-the-morale",
            "demo.ability.daemon-immortal-body",
            "demo.ability.daemon-insanity-circle",
            "demo.ability.daemon-explode-pets",
            "demo.ability.daemon-summon-greater-demon",
            "demo.ability.daemon-hellfire",
            "demo.ability.daemon-send-to-hell",
            "demo.ability.daemon-polymorph-demonlord",
        ])
    );

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.hellfire-tome")
        .expect("Hellfire Tome item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (80, 30, 100_000, Some("demo.ability-book.hellfire-tome"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Hellfire Tome should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (80, u16::MAX, 33)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.daemon-raise-the-morale", 8, 8, 55, 8),
        ("demo.ability.daemon-immortal-body", 23, 20, 75, 35),
        ("demo.ability.daemon-insanity-circle", 33, 30, 70, 200),
        ("demo.ability.daemon-explode-pets", 36, 44, 75, 100),
        ("demo.ability.daemon-summon-greater-demon", 38, 90, 75, 250),
        ("demo.ability.daemon-hellfire", 42, 85, 85, 250),
        ("demo.ability.daemon-send-to-hell", 43, 75, 70, 200),
        ("demo.ability.daemon-polymorph-demonlord", 46, 70, 75, 250),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
        assert!(!matches!(
            ability.effect,
            AbilityEffectDefinition::NoOp { .. }
        ));
    }

    let effect = |id: &str| {
        &content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"))
            .effect
    };
    assert!(matches!(
        effect("demo.ability.daemon-insanity-circle"),
        AbilityEffectDefinition::InsanityCircle {
            damage_bonus: 50,
            control_power: 20,
            radius: 3
        }
    ));
    assert!(matches!(
        effect("demo.ability.daemon-explode-pets"),
        AbilityEffectDefinition::ExplodePets
    ));
    assert!(matches!(
        effect("demo.ability.daemon-summon-greater-demon"),
        AbilityEffectDefinition::SummonGreaterDemon { corpse_item_kind_id, radius: 2 }
            if corpse_item_kind_id == "demo.item.corpse-remains"
    ));
    assert!(matches!(
        effect("demo.ability.daemon-hellfire"),
        AbilityEffectDefinition::Hellfire {
            damage_bonus: 666,
            radius: 3,
            backlash_dice: 1,
            backlash_sides: 30,
            backlash_bonus: 20,
        }
    ));
    assert!(matches!(
        effect("demo.ability.daemon-send-to-hell"),
        AbilityEffectDefinition::Genocide {
            scope: AbilityGenocideScopeDefinition::Single,
            power: 666,
            fatigue: false,
            ..
        }
    ));
    assert!(matches!(
        effect("demo.ability.daemon-polymorph-demonlord"),
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            duration_ticks: 15,
            duration_dice: 1,
            duration_sides: 15,
            granted_race_id: Some(race_id),
            grants_wall_passage: true,
            ..
        } if status_kind_id == "rfb.status.demon-lord-transformation"
            && race_id == "demo.race.demon-lord"
    ));

    let race = content
        .races
        .iter()
        .find(|race| race.id == "demo.race.demon-lord")
        .expect("the temporary demon lord race should compile");
    assert_eq!(race.legacy_index, Some(1001));
    assert_eq!(
        (
            race.modifiers.strength,
            race.modifiers.intelligence,
            race.modifiers.wisdom,
            race.modifiers.dexterity,
            race.modifiers.constitution,
            race.modifiers.charisma,
            race.modifiers.defense,
            race.modifiers.speed,
        ),
        (10, 10, 10, 10, 10, 10, 20, 5)
    );
    assert_eq!(
        (
            race.life_percent,
            race.experience_percent,
            race.base_hp,
            race.infravision,
            race.see_invisible,
        ),
        (110, 1500, 28, 20, true)
    );
    assert_eq!(
        race.resistances.get(&ActorDamageType::Fire),
        Some(&ActorResistanceLevel::Immune)
    );
    assert_eq!(
        race.resistances.get(&ActorDamageType::Chaos),
        Some(&ActorResistanceLevel::Resistant)
    );
}

#[test]
fn life_second_book_keeps_the_original_identity_allocation_and_spell_table() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.high-mass")
        .expect("High Mass should compile");
    assert_eq!(book.realm_id.as_deref(), Some("life"));
    assert_eq!(book.rank, Some(2));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.high-mass")
        .expect("High Mass item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (20, 30, 1_000, Some("demo.ability-book.high-mass"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("High Mass should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (20, 50, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.life-remove-curse", 14, 12, 35, 4),
        ("demo.ability.life-fasting", 15, 14, 40, 4),
        ("demo.ability.life-cure-critical-wounds", 15, 15, 40, 4),
        ("demo.ability.life-resist-heat-and-cold", 17, 15, 40, 4),
        ("demo.ability.life-sense-surroundings", 19, 17, 40, 4),
        ("demo.ability.life-turn-undead", 21, 19, 40, 4),
        ("demo.ability.life-healing", 25, 25, 45, 5),
        ("demo.ability.life-glyph-of-warding", 30, 50, 55, 5),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let cure = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.life-cure-critical-wounds")
        .expect("Cure Critical Wounds should compile");
    assert_eq!(
        cure.spell_power_fields
            .iter()
            .map(|field| (field.effect_index, field.field))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(0, AbilitySpellPowerField::FinalHealing)])
    );
    let healing = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.life-healing")
        .expect("Healing should compile");
    assert_eq!(
        healing
            .spell_power_fields
            .iter()
            .map(|field| (field.effect_index, field.field))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(0, AbilitySpellPowerField::HealingAmount)])
    );
    let turn = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.life-turn-undead")
        .expect("Turn Undead should compile");
    assert!(matches!(
        turn.effect,
        AbilityEffectDefinition::TurnUndead { power: 0 }
    ));
    assert_eq!(turn.level_scaling.len(), 1);
}

#[test]
fn life_third_book_keeps_the_original_identity_allocation_and_spell_table() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.book-of-the-unicorn")
        .expect("Book of the Unicorn should compile");
    assert_eq!(book.realm_id.as_deref(), Some("life"));
    assert_eq!(book.rank, Some(3));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.book-of-the-unicorn")
        .expect("Book of the Unicorn item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (
            45,
            30,
            15_000,
            Some("demo.ability-book.book-of-the-unicorn")
        )
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Book of the Unicorn should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (45, 90, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .map(|shop| shop.category)
            .collect::<Vec<_>>(),
        vec![
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
        ]
    );

    let expected = [
        ("demo.ability.life-dispel-curse", 20, 20, 40, 75),
        ("demo.ability.life-perception", 24, 24, 60, 150),
        ("demo.ability.life-dispel-undead", 30, 30, 50, 75),
        ("demo.ability.life-sustain-attributes", 31, 30, 50, 75),
        ("demo.ability.life-cure-mutation", 32, 30, 60, 75),
        ("demo.ability.life-word-of-recall", 33, 40, 60, 115),
        ("demo.ability.life-transcendence", 35, 35, 60, 125),
        ("demo.ability.life-warding-true", 40, 70, 70, 150),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let dispel = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.life-dispel-undead")
        .expect("Dispel Undead should compile");
    assert!(matches!(
        dispel.effect,
        AbilityEffectDefinition::VisibleDamage {
            damage_dice: 0,
            damage_sides: 0,
            unlife_change_on_hit: -2,
            ..
        }
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-dispel-curse")
            .expect("Dispel Curse should compile")
            .effect,
        AbilityEffectDefinition::RemoveEquippedCurses {
            include_heavy: true
        }
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-perception")
            .expect("Perception should compile")
            .effect,
        AbilityEffectDefinition::IdentifyItem {
            full_identify_power: 0,
            full_identify_roll_sides: 0
        }
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-word-of-recall")
            .expect("Word of Recall should compile")
            .effect,
        AbilityEffectDefinition::Recall {
            delay_dice: 1,
            delay_sides: 20,
            delay_bonus: 15
        }
    ));
    assert!(matches!(
        &content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-warding-true")
            .expect("Warding True should compile")
            .effect,
        AbilityEffectDefinition::Sequence { effects }
            if matches!(effects.as_slice(), [
                AbilityEffectDefinition::CreateCurrentTerrain { .. },
                AbilityEffectDefinition::CreateAdjacentTerrain { .. }
            ])
    ));
    for id in [
        "demo.ability.life-dispel-undead",
        "demo.ability.life-sustain-attributes",
        "demo.ability.life-transcendence",
    ] {
        assert_eq!(
            content
                .abilities
                .iter()
                .find(|ability| ability.id == id)
                .expect("scaled Life ability should compile")
                .spell_power_fields
                .len(),
            1
        );
    }
}

#[test]
fn life_fourth_book_completes_the_original_realm_and_acquisition() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.blessings-of-the-grail")
        .expect("Blessings of the Grail should compile");
    assert_eq!(book.realm_id.as_deref(), Some("life"));
    assert_eq!(book.rank, Some(4));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.blessings-of-the-grail")
        .expect("Blessings of the Grail item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (
            80,
            30,
            100_000,
            Some("demo.ability-book.blessings-of-the-grail")
        )
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Blessings of the Grail should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (80, u16::MAX, 33)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .map(|shop| shop.category)
            .collect::<Vec<_>>(),
        vec![
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
        ]
    );

    let expected = [
        ("demo.ability.life-sterilization", 5, 9, 40, 40),
        ("demo.ability.life-detection", 20, 20, 40, 50),
        ("demo.ability.life-annihilate-undead", 30, 50, 60, 115),
        ("demo.ability.life-clairvoyance", 40, 80, 60, 225),
        ("demo.ability.life-restoration", 42, 75, 60, 115),
        ("demo.ability.life-healing-true", 45, 40, 60, 100),
        ("demo.ability.life-holy-vision", 47, 90, 70, 250),
        ("demo.ability.life-ultimate-resistance", 49, 90, 70, 250),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-sterilization")
            .expect("Sterilization should compile")
            .effect,
        AbilityEffectDefinition::SuppressMonsterReproduction {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 0,
        }
    ));
    assert!(matches!(
        &content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-annihilate-undead")
            .expect("Annihilate Undead should compile")
            .effect,
        AbilityEffectDefinition::Genocide {
            scope: AbilityGenocideScopeDefinition::Nearby,
            power: 50,
            radius: 20,
            target_category: Some(category),
            fatigue: true,
            unlife_change_on_success: -2,
            chance_change_on_success: -1,
        } if category == "undead"
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-clairvoyance")
            .expect("Clairvoyance should compile")
            .effect,
        AbilityEffectDefinition::Clairvoyance {
            telepathy_duration_ticks: 0,
            telepathy_duration_dice: 0,
            telepathy_duration_sides: 0,
            grants_virtues: false,
            grants_telepathy: false,
        }
    ));
    assert!(matches!(
        content
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.life-restoration")
            .expect("Restoration should compile")
            .effect,
        AbilityEffectDefinition::RestoreVitality {
            life_force: 1_000,
            restore_attributes: true,
        }
    ));
    let ultimate = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.life-ultimate-resistance")
        .expect("Ultimate Resistance should compile");
    assert!(matches!(
        &ultimate.effect,
        AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            duration_ticks: 0,
            duration_dice: 1,
            duration_sides: 0,
            granted_resistances,
            granted_modifiers,
            granted_equipment_bonuses,
            granted_status_immunities,
            ..
        } if status_kind_id == "rfb.status.ultimate-resistance"
            && granted_resistances.len() == 17
            && granted_modifiers.defense == 100
            && granted_modifiers.speed == 10
            && granted_equipment_bonuses.light_radius == 1
            && granted_status_immunities.contains("rfb.status.paralysis")
    ));
    assert_eq!(ultimate.level_scaling.len(), 2);
    assert_eq!(ultimate.spell_power_fields.len(), 2);

    let life_books = content
        .ability_books
        .iter()
        .filter(|book| book.realm_id.as_deref() == Some("life"))
        .collect::<Vec<_>>();
    assert_eq!(life_books.len(), 4);
    let ability_ids = life_books
        .iter()
        .flat_map(|book| &book.ability_ids)
        .collect::<BTreeSet<_>>();
    assert_eq!(ability_ids.len(), 32);
    assert!(ability_ids.iter().all(|id| {
        content
            .abilities
            .iter()
            .find(|ability| &ability.id == *id)
            .is_some_and(|ability| {
                ability
                    .effect
                    .ordered_effects()
                    .iter()
                    .all(|effect| !matches!(effect, AbilityEffectDefinition::NoOp { .. }))
            })
    }));
}

#[test]
fn nature_first_book_keeps_the_original_spell_table_and_allocation() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.call-of-the-wild")
        .expect("Call of the Wild should compile");
    assert_eq!(book.realm_id.as_deref(), Some("nature"));
    assert_eq!(book.rank, Some(1));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.call-of-the-wild")
        .expect("Call of the Wild item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (10, 30, 100, Some("demo.ability-book.call-of-the-wild"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Call of the Wild should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (10, 30, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.nature-detect-creatures", 1, 1, 15, 4),
        ("demo.ability.nature-lightning", 2, 1, 15, 3),
        ("demo.ability.nature-detect-doors-and-traps", 2, 2, 15, 1),
        ("demo.ability.nature-produce-food", 3, 2, 25, 4),
        ("demo.ability.nature-daylight", 3, 3, 40, 5),
        ("demo.ability.nature-wind-walker", 4, 3, 40, 5),
        ("demo.ability.nature-resist-environment", 4, 4, 40, 5),
        ("demo.ability.nature-cure-wounds-and-poison", 5, 4, 25, 4),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let lightning = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.nature-lightning")
        .expect("Lightning should compile");
    assert!(matches!(
        lightning.effect,
        AbilityEffectDefinition::BeamDamage {
            damage_dice: 3,
            damage_sides: 4,
            maximum_range: Some(2),
            ..
        }
    ));
    assert_eq!(lightning.level_scaling.len(), 2);
    assert_eq!(lightning.spell_power_fields.len(), 2);

    let produce_food = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.nature-produce-food")
        .expect("Produce Food should compile");
    assert!(matches!(
        &produce_food.effect,
        AbilityEffectDefinition::CreateItem {
            item_kind_id,
            quantity: 1,
        } if item_kind_id == "demo.item.ration-of-food"
    ));
}

#[test]
fn nature_second_book_keeps_the_original_spell_table_and_allocation() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.nature-mastery")
        .expect("Nature Mastery should compile");
    assert_eq!(book.realm_id.as_deref(), Some("nature"));
    assert_eq!(book.rank, Some(2));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.nature-mastery")
        .expect("Nature Mastery item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (20, 30, 1_000, Some("demo.ability-book.nature-mastery"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Nature Mastery should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (20, 50, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.nature-stone-to-mud", 5, 4, 30, 6),
        ("demo.ability.nature-frost-bolt", 5, 4, 20, 6),
        ("demo.ability.nature-awareness", 5, 5, 35, 6),
        ("demo.ability.nature-fire-bolt", 5, 5, 30, 6),
        ("demo.ability.nature-ray-of-sunlight", 7, 5, 30, 5),
        ("demo.ability.nature-entangle", 14, 10, 35, 8),
        ("demo.ability.nature-natures-gate", 20, 20, 80, 50),
        ("demo.ability.nature-herbal-healing", 35, 50, 80, 50),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    for id in [
        "demo.ability.nature-frost-bolt",
        "demo.ability.nature-fire-bolt",
    ] {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .expect("Nature elemental bolt should compile");
        assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::BoltOrBeamDamage {
                damage_sides: 8,
                beam_chance_modifier: 0,
                ..
            }
        ));
        assert!(ability.affects_ground_items);
    }

    assert!(
        content
            .actors
            .iter()
            .any(|actor| actor.tags.iter().any(|tag| tag == "animal-ranger"))
    );
}

#[test]
fn commit32_nature_third_book_keeps_the_original_spell_table_and_allocation() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.natures-gifts")
        .expect("Nature's Gifts should compile");
    assert_eq!(book.realm_id.as_deref(), Some("nature"));
    assert_eq!(book.rank, Some(3));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.natures-gifts")
        .expect("Nature's Gifts item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (45, 30, 15_000, Some("demo.ability-book.natures-gifts"))
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Nature's Gifts should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (45, 90, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .map(|shop| shop.category)
            .collect::<Vec<_>>(),
        vec![
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
        ]
    );

    let expected = [
        ("demo.ability.nature-stair-building", 7, 7, 20, 44),
        ("demo.ability.nature-stone-skin", 8, 8, 65, 120),
        ("demo.ability.nature-resistance-true", 12, 15, 75, 60),
        ("demo.ability.nature-forest-creation", 17, 20, 60, 40),
        ("demo.ability.nature-stone-tell", 33, 35, 80, 200),
        ("demo.ability.nature-wall-of-stone", 35, 40, 65, 200),
        (
            "demo.ability.nature-protect-from-corrosion",
            37,
            65,
            80,
            250,
        ),
        ("demo.ability.nature-call-sunlight", 38, 30, 80, 300),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }
}

#[test]
fn commit33_nature_fourth_book_keeps_the_original_spell_table_and_allocation() {
    let content = compile_pack_dir(&original_pack_path())
        .expect("original pack should compile")
        .content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.natures-wrath")
        .expect("Nature's Wrath should compile");
    assert_eq!(book.realm_id.as_deref(), Some("nature"));
    assert_eq!(book.rank, Some(4));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.natures-wrath")
        .expect("Nature's Wrath item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (70, 30, 100_000, Some("demo.ability-book.natures-wrath"))
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Nature's Wrath should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (70, u16::MAX, 50)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .map(|shop| shop.category)
            .collect::<Vec<_>>(),
        vec![
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
        ]
    );

    let expected = [
        ("demo.ability.nature-earthquake", 15, 15, 50, 25),
        ("demo.ability.nature-fire-storm", 20, 20, 65, 50),
        ("demo.ability.nature-blizzard", 22, 22, 65, 29),
        ("demo.ability.nature-lightning-storm", 28, 25, 65, 35),
        ("demo.ability.nature-whirlpool", 32, 28, 75, 65),
        ("demo.ability.nature-ice-bolt", 36, 32, 65, 250),
        ("demo.ability.nature-gravity-storm", 38, 35, 70, 250),
        ("demo.ability.nature-natures-wrath", 39, 65, 55, 300),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }
}

#[test]
fn armageddon_first_book_keeps_the_original_spell_table_and_elemental_scaling() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.book-of-elements")
        .expect("Armageddon first book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("armageddon"));
    assert_eq!(book.rank, Some(1));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.book-of-elements")
        .expect("Book of Elements item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (10, 30, 100, Some("demo.ability-book.book-of-elements"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Book of Elements should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (10, 30, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.armageddon-lightning-bolt", 1, 2, 25, 4),
        ("demo.ability.armageddon-frost-bolt", 2, 2, 25, 4),
        ("demo.ability.armageddon-fire-bolt", 3, 3, 30, 1),
        ("demo.ability.armageddon-acid-bolt", 5, 4, 35, 2),
        ("demo.ability.armageddon-lightning-ball", 6, 5, 35, 4),
        ("demo.ability.armageddon-frost-ball", 9, 6, 40, 2),
        ("demo.ability.armageddon-fire-ball", 11, 8, 40, 6),
        ("demo.ability.armageddon-acid-ball", 12, 9, 40, 4),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
        assert!(ability.affects_ground_items);
        assert_eq!(ability.spell_power_fields.len(), 1);
        assert_eq!(ability.level_scaling.len(), 1);
    }

    for id in [
        "demo.ability.armageddon-lightning-bolt",
        "demo.ability.armageddon-frost-bolt",
        "demo.ability.armageddon-fire-bolt",
        "demo.ability.armageddon-acid-bolt",
    ] {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .expect("elemental bolt should compile");
        let AbilityEffectDefinition::BoltOrBeamDamage {
            damage_sides,
            beam_chance_modifier,
            ..
        } = &ability.effect
        else {
            panic!("{id} should use bolt-or-beam damage");
        };
        assert_eq!((*damage_sides, *beam_chance_modifier), (8, 10), "{id}");
        assert_eq!(
            ability.level_scaling[0].field,
            AbilityLevelScalingField::DamageDice
        );
    }
    for (id, base_bonus) in [
        "demo.ability.armageddon-lightning-ball",
        "demo.ability.armageddon-frost-ball",
        "demo.ability.armageddon-fire-ball",
        "demo.ability.armageddon-acid-ball",
    ]
    .into_iter()
    .zip([19_u16, 24, 29, 34])
    {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .expect("elemental ball should compile");
        assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::AreaDamage {
                damage_dice: 1,
                damage_sides: 1,
                damage_bonus,
                radius: 2,
                ..
            } if damage_bonus == base_bonus
        ));
        assert_eq!(
            ability.level_scaling[0].field,
            AbilityLevelScalingField::DamageBonus
        );
    }
}

#[test]
fn armageddon_second_book_keeps_the_original_spell_table_and_allocation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.earth-wind-and-fire")
        .expect("Armageddon second book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("armageddon"));
    assert_eq!(book.rank, Some(2));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.earth-wind-and-fire")
        .expect("Earth, Wind and Fire item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (20, 30, 1_000, Some("demo.ability-book.earth-wind-and-fire"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Earth, Wind and Fire should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (20, 50, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.armageddon-shard-bolt", 15, 10, 40, 10),
        ("demo.ability.armageddon-gravity-bolt", 17, 12, 50, 4),
        ("demo.ability.armageddon-plasma-bolt", 19, 15, 50, 10),
        ("demo.ability.armageddon-meteor", 21, 17, 50, 5),
        ("demo.ability.armageddon-thunderclap", 23, 19, 50, 7),
        ("demo.ability.armageddon-windblast", 26, 23, 50, 10),
        ("demo.ability.armageddon-hellstorm", 28, 25, 50, 15),
        ("demo.ability.armageddon-rocket", 31, 33, 50, 20),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
        assert!(ability.affects_ground_items);
        assert_eq!(ability.spell_power_fields.len(), 1);
    }
    assert!(
        content
            .actors
            .iter()
            .find(|actor| actor.id == "demo.actor.quartz-vein")
            .expect("quartz vein should compile")
            .status_immunities
            .iter()
            .any(|status| status == "rfb.status.stun")
    );
}

#[test]
fn armageddon_third_book_keeps_the_original_spell_table_and_allocation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.path-of-destruction")
        .expect("Armageddon third book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("armageddon"));
    assert_eq!(book.rank, Some(3));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.path-of-destruction")
        .expect("Path of Destruction item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (
            45,
            30,
            15_000,
            Some("demo.ability-book.path-of-destruction")
        )
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Path of Destruction should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (45, 90, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .map(|shop| shop.category)
            .collect::<Vec<_>>(),
        vec![
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
        ]
    );

    let expected = [
        ("demo.ability.armageddon-ice-bolt", 16, 14, 55, 40),
        ("demo.ability.armageddon-water-ball", 18, 17, 50, 50),
        ("demo.ability.armageddon-breathe-lightning", 21, 20, 55, 70),
        ("demo.ability.armageddon-breathe-frost", 22, 21, 55, 70),
        ("demo.ability.armageddon-breathe-fire", 24, 22, 60, 120),
        ("demo.ability.armageddon-breathe-acid", 26, 23, 60, 100),
        ("demo.ability.armageddon-breathe-plasma", 28, 28, 60, 200),
        ("demo.ability.armageddon-breathe-gravity", 36, 30, 60, 250),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
        assert!(ability.affects_ground_items);
        assert_eq!(ability.spell_power_fields.len(), 1);
    }
}

#[test]
fn armageddon_fourth_book_keeps_the_original_spell_table_and_allocation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.day-of-ragnarok")
        .expect("Day of Ragnarok should compile");
    assert_eq!(book.realm_id.as_deref(), Some("armageddon"));
    assert_eq!(book.rank, Some(4));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.day-of-ragnarok")
        .expect("Day of Ragnarok item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (85, 30, 100_000, Some("demo.ability-book.day-of-ragnarok"))
    );
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
            ItemDestructionElement::Cold,
        ])
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Day of Ragnarok should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (85, u16::MAX, 33)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .map(|shop| shop.category)
            .collect::<Vec<_>>(),
        vec![
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
            ShopCategory::BlackMarket,
        ]
    );

    let expected = [
        ("demo.ability.armageddon-mana-bolt", 19, 12, 45, 80),
        ("demo.ability.armageddon-plasma-ball", 22, 17, 55, 250),
        ("demo.ability.armageddon-mana-ball", 30, 22, 65, 100),
        ("demo.ability.armageddon-breathe-sound", 35, 26, 65, 150),
        ("demo.ability.armageddon-breathe-inertia", 38, 28, 65, 150),
        (
            "demo.ability.armageddon-breathe-disintegration",
            40,
            40,
            70,
            200,
        ),
        ("demo.ability.armageddon-breathe-mana", 42, 43, 75, 200),
        ("demo.ability.armageddon-breathe-shards", 44, 49, 75, 250),
    ];
    for (id, level, mana, failure, experience) in expected {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        let player = ability
            .player
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
        assert!(ability.affects_ground_items);
        assert_eq!(ability.spell_power_fields.len(), 1);
    }
}

#[test]
fn sorcery_first_two_books_keep_the_original_spell_table() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    for (book_id, rank) in [
        ("demo.ability-book.beginners-handbook", 1),
        ("demo.ability-book.master-sorcerers-handbook", 2),
    ] {
        let book = content
            .ability_books
            .iter()
            .find(|book| book.id == book_id)
            .unwrap_or_else(|| panic!("{book_id} should compile"));
        assert_eq!(book.realm_id.as_deref(), Some("sorcery"));
        assert_eq!(book.rank, Some(rank));
        assert_eq!(book.ability_ids.len(), 8);
    }

    let expected = [
        ("demo.ability.sorcery-detect-monsters", 1, 1, 15, 4),
        ("demo.ability.sorcery-phase-door", 1, 1, 15, 4),
        ("demo.ability.sorcery-detect-doors-traps", 2, 2, 15, 2),
        ("demo.ability.sorcery-light-area", 2, 2, 20, 2),
        ("demo.ability.sorcery-confuse-monster", 3, 3, 20, 3),
        ("demo.ability.sorcery-teleport", 4, 3, 25, 20),
        ("demo.ability.sorcery-sleep-monster", 5, 4, 20, 20),
        ("demo.ability.sorcery-recharging", 5, 5, 65, 45),
        ("demo.ability.sorcery-magic-mapping", 7, 5, 65, 56),
        ("demo.ability.sorcery-identify", 7, 5, 65, 56),
        ("demo.ability.sorcery-slow-monster", 9, 5, 65, 63),
        ("demo.ability.sorcery-mass-sleep", 9, 5, 40, 54),
        ("demo.ability.sorcery-teleport-away", 13, 8, 50, 104),
        ("demo.ability.sorcery-haste-self", 17, 10, 50, 136),
        ("demo.ability.sorcery-true-detection", 24, 15, 60, 360),
        ("demo.ability.sorcery-true-identify", 28, 20, 65, 560),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }
}

#[test]
fn sorcery_third_book_keeps_the_original_spell_table_and_acquisition() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.pattern-sorcery")
        .expect("Pattern Sorcery should compile");
    assert_eq!(book.realm_id.as_deref(), Some("sorcery"));
    assert_eq!(book.rank, Some(3));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.pattern-sorcery")
        .expect("Pattern Sorcery item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (45, 30, 15_000, Some("demo.ability-book.pattern-sorcery"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Pattern Sorcery should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (45, 90, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.sorcery-inventory-protection", 2, 2, 20, 30),
        ("demo.ability.sorcery-create-stair", 8, 8, 70, 320),
        ("demo.ability.sorcery-esp", 12, 9, 50, 300),
        ("demo.ability.sorcery-teleport-town", 15, 25, 60, 600),
        ("demo.ability.sorcery-self-knowledge", 15, 12, 65, 750),
        ("demo.ability.sorcery-teleport-level", 17, 12, 50, 425),
        ("demo.ability.sorcery-word-of-recall", 20, 20, 65, 380),
        ("demo.ability.sorcery-dimension-door", 36, 35, 70, 3_600),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }
    assert!(content.abilities.iter().all(|ability| {
        !book.ability_ids.contains(&ability.id)
            || !matches!(ability.effect, AbilityEffectDefinition::NoOp { .. })
    }));
}

#[test]
fn arcane_second_book_keeps_the_original_spell_table_and_narrow_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.minor-arcana")
        .expect("Arcane second book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("arcane"));
    assert_eq!(book.rank, Some(2));
    assert_eq!(book.ability_ids.len(), 8);
    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.minor-arcana")
        .expect("Minor Arcana item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (15, 30, 250, Some("demo.ability-book.minor-arcana"),)
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == "demo.item.minor-arcana")
        })
        .expect("Minor Arcana should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (15, 50, 100)
    );

    let expected = [
        ("demo.ability.arcane-detect-doors-traps", 6, 5, 30, 42),
        ("demo.ability.arcane-phlogiston", 7, 7, 50, 49),
        ("demo.ability.arcane-detect-treasure", 8, 7, 40, 48),
        ("demo.ability.arcane-detect-enchantment", 8, 8, 40, 48),
        ("demo.ability.arcane-detect-objects", 9, 8, 40, 54),
        ("demo.ability.arcane-cure-poison", 10, 9, 40, 60),
        ("demo.ability.arcane-resist-cold", 10, 10, 40, 50),
        ("demo.ability.arcane-resist-fire", 11, 10, 40, 55),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    for id in [
        "demo.ability.arcane-detect-doors-traps",
        "demo.ability.arcane-detect-treasure",
        "demo.ability.arcane-detect-enchantment",
        "demo.ability.arcane-detect-objects",
    ] {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        assert!(ability.effect.ordered_effects().iter().all(|effect| {
            matches!(
                effect,
                AbilityEffectDefinition::Detect {
                    radius: 30,
                    through_walls: true,
                    ..
                }
            )
        }));
    }

    let phlogiston = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-phlogiston")
        .expect("Phlogiston should compile");
    assert!(matches!(
        phlogiston.effect,
        AbilityEffectDefinition::RefuelEquippedLight {
            maximum_fraction_divisor: 2
        }
    ));

    let cure = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-cure-poison")
        .expect("Cure Poison should compile");
    assert!(matches!(
        &cure.effect,
        AbilityEffectDefinition::ReduceStatus {
            status_kind_id,
            amount: 100,
            current_divisor: Some(5),
            remaining_divisor: None,
        } if status_kind_id == "rfb.status.poison"
    ));

    for (id, status_kind_id, damage_type) in [
        (
            "demo.ability.arcane-resist-cold",
            "rfb.status.resist-cold",
            ActorDamageType::Cold,
        ),
        (
            "demo.ability.arcane-resist-fire",
            "rfb.status.resist-fire",
            ActorDamageType::Fire,
        ),
    ] {
        let ability = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap_or_else(|| panic!("{id} should compile"));
        assert!(matches!(
            &ability.effect,
            AbilityEffectDefinition::ApplyStatus {
                status_kind_id: actual_status,
                duration_ticks: 20,
                duration_dice: 1,
                duration_sides: 20,
                granted_resistances,
                ..
            } if actual_status == status_kind_id
                && granted_resistances.get(&damage_type)
                    == Some(&ActorResistanceLevel::Resistant)
        ));
        assert_eq!(
            ability
                .spell_power_fields
                .iter()
                .map(|field| field.field)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                AbilitySpellPowerField::StatusDurationTicks,
                AbilitySpellPowerField::StatusDurationSides,
            ])
        );
    }
}

#[test]
fn arcane_third_book_keeps_the_original_spell_table_and_narrow_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.major-arcana")
        .expect("Arcane third book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("arcane"));
    assert_eq!(book.rank, Some(3));
    assert_eq!(book.ability_ids.len(), 8);

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.major-arcana")
        .expect("Major Arcana item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (20, 30, 1_000, Some("demo.ability-book.major-arcana"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == "demo.item.major-arcana")
        })
        .expect("Major Arcana should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (20, 90, 100)
    );

    let expected = [
        ("demo.ability.arcane-resist-lightning", 12, 10, 40, 60),
        ("demo.ability.arcane-resist-acid", 13, 10, 40, 65),
        ("demo.ability.arcane-cure-medium-wounds", 14, 11, 22, 84),
        ("demo.ability.arcane-teleport", 15, 12, 40, 120),
        ("demo.ability.arcane-identify", 17, 17, 50, 425),
        ("demo.ability.arcane-stone-to-mud", 19, 15, 50, 171),
        ("demo.ability.arcane-ray-of-light", 20, 16, 50, 180),
        ("demo.ability.arcane-satisfy-hunger", 22, 18, 60, 264),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let cure = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-cure-medium-wounds")
        .expect("Cure Medium Wounds should compile");
    assert!(matches!(
        &cure.effect,
        AbilityEffectDefinition::Sequence { effects }
            if matches!(effects.as_slice(), [
                AbilityEffectDefinition::HealDice { dice: 4, sides: 8 },
                AbilityEffectDefinition::ReduceStatus {
                    status_kind_id,
                    amount: 50,
                    current_divisor: None,
                    remaining_divisor: Some(2),
                }
            ] if status_kind_id == "rfb.status.bleeding")
    ));
    assert_eq!(
        cure.spell_power_fields
            .iter()
            .map(|field| (field.effect_index, field.field))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(0, AbilitySpellPowerField::HealingSides)])
    );

    let teleport = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-teleport")
        .expect("Teleport should compile");
    assert!(matches!(
        teleport.effect,
        AbilityEffectDefinition::BlinkSelf { radius: 5 }
    ));
    assert_eq!(teleport.level_scaling.len(), 1);
    let identify = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-identify")
        .expect("Identify should compile");
    assert!(matches!(
        identify.effect,
        AbilityEffectDefinition::IdentifyItem {
            full_identify_power: 0,
            full_identify_roll_sides: 0,
        }
    ));
    let stone_to_mud = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-stone-to-mud")
        .expect("Stone to Mud should compile");
    assert!(matches!(
        stone_to_mud.effect,
        AbilityEffectDefinition::TerrainBeam {
            operation: AbilityTerrainBeamOperationDefinition::StoneToMud,
        }
    ));
    let ray = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-ray-of-light")
        .expect("Ray of Light should compile");
    assert!(matches!(
        ray.effect,
        AbilityEffectDefinition::LightLine {
            damage_dice: 6,
            damage_sides: 8,
        }
    ));
    let hunger = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-satisfy-hunger")
        .expect("Satisfy Hunger should compile");
    assert_eq!(hunger.effect, AbilityEffectDefinition::SatisfyHunger);
}

#[test]
fn arcane_fourth_book_completes_the_original_realm_and_acquisition() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.manual-of-mastery")
        .expect("Arcane fourth book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("arcane"));
    assert_eq!(book.rank, Some(4));
    assert_eq!(book.ability_ids.len(), 8);
    assert!(!book.tags.iter().any(|tag| tag == "incomplete"));

    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.manual-of-mastery")
        .expect("Manual of Mastery item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (25, 30, 2_500, Some("demo.ability-book.manual-of-mastery"))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Manual of Mastery should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (25, u16::MAX, 100)
    );
    assert_eq!(
        content
            .shops
            .iter()
            .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
            .count(),
        3
    );

    let expected = [
        ("demo.ability.arcane-see-invisible", 24, 22, 50, 312),
        ("demo.ability.arcane-resist-poison", 26, 26, 60, 780),
        ("demo.ability.arcane-teleport-level", 30, 30, 70, 750),
        ("demo.ability.arcane-teleport-away", 35, 28, 60, 875),
        ("demo.ability.arcane-recharging", 40, 28, 55, 1_200),
        ("demo.ability.arcane-detection", 41, 28, 70, 1_640),
        ("demo.ability.arcane-word-of-recall", 43, 40, 60, 2_150),
        ("demo.ability.arcane-clairvoyance", 46, 80, 70, 9_200),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let arcane_books = content
        .ability_books
        .iter()
        .filter(|book| book.realm_id.as_deref() == Some("arcane"))
        .collect::<Vec<_>>();
    assert_eq!(arcane_books.len(), 4);
    assert!(arcane_books.iter().all(|book| book.ability_ids.len() == 8));
    let arcane_ability_ids = arcane_books
        .iter()
        .flat_map(|book| book.ability_ids.iter())
        .collect::<BTreeSet<_>>();
    assert_eq!(arcane_ability_ids.len(), 32);
    assert!(
        content
            .abilities
            .iter()
            .filter(|ability| { arcane_ability_ids.contains(&ability.id) })
            .all(|ability| ability
                .effect
                .ordered_effects()
                .iter()
                .all(|effect| { !matches!(effect, AbilityEffectDefinition::NoOp { .. }) }))
    );

    let clairvoyance = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-clairvoyance")
        .expect("Clairvoyance should compile");
    assert!(matches!(
        clairvoyance.effect,
        AbilityEffectDefinition::Clairvoyance {
            telepathy_duration_ticks: 25,
            telepathy_duration_dice: 1,
            telepathy_duration_sides: 30,
            ..
        }
    ));
}

#[test]
fn sorcery_fourth_book_completes_the_original_realm_and_keeps_rare_books_out_of_bookstores() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let books = content
        .ability_books
        .iter()
        .filter(|book| book.realm_id.as_deref() == Some("sorcery"))
        .collect::<Vec<_>>();
    assert_eq!(books.len(), 4);
    assert!(books.iter().all(|book| book.ability_ids.len() == 8));
    let ability_ids = books
        .iter()
        .flat_map(|book| book.ability_ids.iter())
        .collect::<BTreeSet<_>>();
    assert_eq!(ability_ids.len(), 32);
    assert!(
        content
            .abilities
            .iter()
            .filter(|ability| ability_ids.contains(&ability.id))
            .all(|ability| ability
                .effect
                .ordered_effects()
                .iter()
                .all(|effect| !matches!(effect, AbilityEffectDefinition::NoOp { .. })))
    );

    let fourth = books
        .iter()
        .find(|book| book.rank == Some(4))
        .expect("Sorcery fourth book should compile");
    assert_eq!(fourth.id, "demo.ability-book.grimoire-of-power");
    let item = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.grimoire-of-power")
        .expect("Grimoire of Power item should compile");
    assert_eq!(
        (
            item.generation_level,
            item.weight_tenths_pound,
            item.base_value,
            item.ability_book_id.as_deref(),
        ),
        (80, 30, 100_000, Some(fourth.id.as_str()))
    );
    let allocation = content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Grimoire of Power should use its original allocation");
    assert_eq!(
        (
            allocation.min_depth,
            allocation.max_depth,
            allocation.weight
        ),
        (80, u16::MAX, 33)
    );
    let stocked = content
        .shops
        .iter()
        .filter(|shop| shop.stock.iter().any(|entry| entry.item_kind_id == item.id))
        .collect::<Vec<_>>();
    assert_eq!(stocked.len(), 3);
    assert!(
        stocked
            .iter()
            .all(|shop| shop.category == ShopCategory::BlackMarket)
    );

    let expected = [
        ("demo.ability.sorcery-probe", 8, 8, 30, 160),
        ("demo.ability.sorcery-create-door", 18, 20, 75, 1_800),
        ("demo.ability.sorcery-fetch", 20, 20, 65, 1_400),
        ("demo.ability.sorcery-clairvoyance", 25, 30, 70, 3_000),
        ("demo.ability.sorcery-device-mastery", 37, 55, 75, 3_700),
        ("demo.ability.sorcery-alchemy", 40, 40, 80, 7_000),
        ("demo.ability.sorcery-banish", 41, 43, 50, 8_200),
        ("demo.ability.sorcery-invulnerability", 42, 65, 75, 10_500),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }
}

use std::collections::BTreeSet;

use super::*;

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
fn arcane_fourth_book_prelude_keeps_original_spells_without_an_acquisition_path() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.manual-of-mastery")
        .expect("Arcane fourth book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("arcane"));
    assert_eq!(book.rank, Some(4));
    assert_eq!(book.ability_ids.len(), 7);
    assert!(book.tags.iter().any(|tag| tag == "incomplete"));

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
    assert!(content.loot_tables.iter().all(|table| {
        table
            .entries
            .iter()
            .all(|entry| entry.item_kind_id != item.id)
    }));
    assert!(
        content
            .shops
            .iter()
            .all(|shop| shop.stock.iter().all(|entry| entry.item_kind_id != item.id))
    );

    let expected = [
        ("demo.ability.arcane-see-invisible", 24, 22, 50, 312),
        ("demo.ability.arcane-resist-poison", 26, 26, 60, 780),
        ("demo.ability.arcane-teleport-level", 30, 30, 70, 750),
        ("demo.ability.arcane-teleport-away", 35, 28, 60, 875),
        ("demo.ability.arcane-recharging", 40, 28, 55, 1_200),
        ("demo.ability.arcane-detection", 41, 28, 70, 1_640),
        ("demo.ability.arcane-word-of-recall", 43, 40, 60, 2_150),
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

// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, dispatch_next, give_inventory_item};
use super::*;

const ARCHER_BUILD_ID: &str = "demo.build.archer";

fn archer_game(seed: u64) -> Game {
    Game::new_with_build(seed, ARCHER_BUILD_ID).expect("Archer build should create")
}

#[test]
fn archer_birth_uses_the_original_class_identity_skills_and_kit() {
    let game = archer_game(0x4152_4348_4552);
    let snapshot = game.snapshot();
    let build = snapshot
        .player
        .build
        .expect("Archer should project its build");

    assert_eq!(build.build_id, ARCHER_BUILD_ID);
    assert_eq!(build.class_id, "demo.class.archer");
    assert_eq!((build.life_percent, build.experience_percent), (110, 110));
    assert_eq!(snapshot.player.kind_id, "demo.actor.archer-player");
    assert_eq!(snapshot.player.progress.attributes.strength.effective, 15);
    assert_eq!(
        snapshot.player.progress.attributes.intelligence.effective,
        12
    );
    assert_eq!(snapshot.player.progress.attributes.wisdom.effective, 12);
    assert_eq!(snapshot.player.progress.attributes.dexterity.effective, 15);
    assert_eq!(
        snapshot.player.progress.attributes.constitution.effective,
        14
    );

    let skill = |id: &str| {
        snapshot
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == id)
            .expect("original Archer skill should be projected")
    };
    assert_eq!(
        (
            skill("demo.skill.ranged").base,
            skill("demo.skill.ranged").growth_per_ten_levels
        ),
        (82, 36)
    );
    assert_eq!(
        (
            skill("demo.skill.melee").base,
            skill("demo.skill.melee").growth_per_ten_levels
        ),
        (56, 18)
    );
    assert_eq!(
        (
            skill("demo.skill.disarming").base,
            skill("demo.skill.disarming").growth_per_ten_levels
        ),
        (38, 12)
    );

    for kind_id in [
        "demo.item.short-sword",
        "demo.item.leather-scale-mail",
        "demo.item.short-bow",
        "demo.item.quiver",
    ] {
        assert!(
            game.items.iter().any(|item| {
                item.kind_id == kind_id && matches!(item.location, ItemLocation::Equipped { .. })
            }),
            "birth kit should equip {kind_id}"
        );
    }
    let arrows = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("Archer should start with arrows");
    assert!((30..=50).contains(&arrows.quantity));

    let abilities = snapshot
        .player
        .abilities
        .iter()
        .filter(|ability| ability.source == AbilitySourceDto::Class)
        .collect::<Vec<_>>();
    assert_eq!(abilities.len(), 3);
    assert!(abilities.iter().all(|ability| {
        ability.ui_group_name_key.as_deref() == Some("ability-group-demo-archer-create-ammo-name")
    }));
    assert!(
        abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.archer-create-shots")
            .expect("level-one power")
            .can_cast
    );
    assert!(
        !abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.archer-create-arrows")
            .expect("level-ten power")
            .can_cast
    );
    assert!(
        !abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.archer-create-bolts")
            .expect("level-twenty power")
            .can_cast
    );
}

#[test]
fn equipped_quiver_carries_sixty_ammunition_outside_the_pack() {
    let mut game = archer_game(7);
    let arrow_index = game
        .items
        .iter()
        .position(|item| item.kind_id == "demo.item.arrow")
        .expect("birth arrows should exist");
    game.items[arrow_index].quantity = 60;
    let with_sixty = game.inventory_used_slots();
    game.items[arrow_index].quantity = 61;
    assert_eq!(game.inventory_used_slots(), with_sixty + 1);
    game.items[arrow_index].quantity = 60;

    let quiver_slot = game
        .items
        .iter()
        .find_map(|item| match &item.location {
            ItemLocation::Equipped { slot_id } if item.kind_id == "demo.item.quiver" => {
                Some(slot_id.clone())
            }
            _ => None,
        })
        .expect("quiver should be equipped");
    assert_eq!(
        game.unequip_slot(&quiver_slot),
        Some("demo.item.quiver".to_owned())
    );
    assert_eq!(game.inventory_used_slots(), with_sixty + 2);
}

#[test]
fn archer_makes_original_quantity_ammunition_from_terrain_and_skeletons() {
    let mut game = archer_game(11);
    let wall = game.position_in_direction(Direction::East);
    let wall_index = game
        .index(wall)
        .expect("adjacent position should be in bounds");
    game.terrain[wall_index] = "demo.terrain.wall".to_owned();
    game.debug_ability_casts_succeed = true;
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.archer-create-shots",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Create Shots should resolve");
    assert_eq!(game.terrain[wall_index], "demo.terrain.floor");
    let shots = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                resolution.effects.iter().find_map(|effect| match effect {
                    AbilityEffectResolutionDto::CreateAmmunition {
                        item_kind_id,
                        quantity,
                        source_position,
                        ..
                    } if item_kind_id == "demo.item.rounded-pebble"
                        || item_kind_id == "demo.item.iron-shot"
                        || item_kind_id == "demo.item.mithril-shot" =>
                    {
                        Some((*quantity, *source_position))
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("Create Shots should report its output");
    assert!((15..=30).contains(&shots.0));
    assert_eq!(shots.1, Some(wall));

    game.progress.level = 20;
    for (ability_id, item_prefix, minimum, maximum) in [
        ("demo.ability.archer-create-arrows", "arrow", 5, 10),
        ("demo.ability.archer-create-bolts", "bolt", 4, 8),
    ] {
        let source_id = format!("test.skeleton.{item_prefix}");
        give_inventory_item(&mut game, &source_id, "demo.item.skeleton-remains");
        events.clear();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::Item {
                item_id: source_id.clone(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("skeleton ammunition creation should resolve");
        assert!(!game.items.iter().any(|item| item.id == source_id));
        let quantity = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    resolution.effects.iter().find_map(|effect| match effect {
                        AbilityEffectResolutionDto::CreateAmmunition {
                            quantity,
                            source_item_id,
                            ..
                        } if source_item_id.as_deref() == Some(source_id.as_str()) => {
                            Some(*quantity)
                        }
                        _ => None,
                    })
                }
                _ => None,
            })
            .expect("created ammunition should be reported");
        assert!((minimum..=maximum).contains(&quantity));
    }
}

#[test]
fn archer_breakage_and_projectile_critical_hooks_are_active() {
    let mut game = archer_game(19);
    let profile = game
        .player_projectile_profile()
        .expect("birth bow should have arrows");
    assert!(
        profile.ammo_break_chance_percent < 20,
        "Archer arrows should break less often than their base 20% chance"
    );

    game.progress.level = 50;
    let ranged_skill = game.player_derived_stats().ranged_skill.value;
    let critical = (0..1_000).find_map(|seed| {
        game.rng = RfbRng::seeded(seed);
        let multiplier = game.roll_projectile_critical_multiplier(
            profile.ammunition_weight_tenths_pound,
            profile.to_hit,
            ranged_skill,
        );
        (multiplier > 100).then_some(multiplier)
    });
    assert!(
        critical.is_some(),
        "Archer level bonus should permit an ammunition critical"
    );

    let mut warrior = Game::new(19);
    assert_eq!(warrior.roll_projectile_critical_multiplier(2, 0, 500), 100);
}

#[test]
fn archer_shooting_energy_and_heavy_launcher_rules_match_original() {
    let breakage_factor = |ranged_skill: i32, class_modifier: i32| {
        (if ranged_skill > 80 {
            90_i32.saturating_sub((ranged_skill - 80) / 2)
        } else {
            100
        })
        .saturating_add(class_modifier)
        .max(0)
    };

    let mut novice = archer_game(29);
    clear_monsters(&mut novice);
    let novice_profile = novice
        .player_projectile_profile()
        .expect("birth bow should resolve");
    let novice_skill = novice.player_derived_stats().ranged_skill.value;
    assert_eq!(novice_profile.energy_cost, 88);
    assert_eq!(
        novice_profile.ammo_break_chance_percent,
        u8::try_from(20 * breakage_factor(novice_skill, -10) / 100).unwrap()
    );
    let novice_gain = energy_gain(derived_speed(&novice.player_derived_stats().speed));
    let novice_tick = novice.world_tick;
    dispatch_next(
        &mut novice,
        GameCommand::Fire {
            direction: Direction::East,
        },
    );
    let novice_ticks = novice.world_tick - novice_tick;
    assert_eq!(
        novice_ticks,
        u32::try_from((novice_profile.energy_cost + novice_gain - 1) / novice_gain).unwrap()
    );

    let mut expert = archer_game(29);
    clear_monsters(&mut expert);
    expert.progress.level = 50;
    expert.progress.max_level = 50;
    let expert_profile = expert
        .player_projectile_profile()
        .expect("expert bow should resolve");
    let expert_skill = expert.player_derived_stats().ranged_skill.value;
    assert_eq!(expert_profile.energy_cost, 8_888 / expert_skill);
    assert!(expert_profile.energy_cost < novice_profile.energy_cost);
    let expert_gain = energy_gain(derived_speed(&expert.player_derived_stats().speed));
    let expert_tick = expert.world_tick;
    dispatch_next(
        &mut expert,
        GameCommand::Fire {
            direction: Direction::East,
        },
    );
    let expert_ticks = expert.world_tick - expert_tick;
    assert_eq!(
        expert_ticks,
        u32::try_from((expert_profile.energy_cost + expert_gain - 1) / expert_gain).unwrap()
    );
    assert!(expert_ticks < novice_ticks);

    let mut heavy = archer_game(31);
    heavy
        .items
        .iter_mut()
        .find(|item| {
            matches!(item.location, ItemLocation::Equipped { ref slot_id } if slot_id == "shooting")
        })
        .expect("birth bow should be equipped")
        .location = ItemLocation::Inventory;
    give_inventory_item(
        &mut heavy,
        "test.heavy-crossbow",
        "demo.item.heavy-crossbow",
    );
    give_inventory_item(&mut heavy, "test.bolt", "demo.item.bolt");
    heavy
        .items
        .iter_mut()
        .find(|item| item.id == "test.heavy-crossbow")
        .expect("heavy crossbow should exist")
        .location = ItemLocation::Equipped {
        slot_id: "shooting".to_owned(),
    };
    let heavy_profile = heavy
        .player_projectile_profile()
        .expect("heavy crossbow should resolve");
    let heavy_skill = heavy.player_derived_stats().ranged_skill.value;
    assert_eq!(heavy_profile.to_hit, -8);
    assert_eq!(heavy_profile.energy_cost, 133);
    assert_eq!(
        heavy_profile.ammo_break_chance_percent,
        u8::try_from(10 * breakage_factor(heavy_skill, 0) / 100).unwrap()
    );
}

#[test]
fn blindness_blocks_create_ammunition_without_consuming_the_source() {
    let mut game = archer_game(23);
    game.progress.level = 10;
    give_inventory_item(
        &mut game,
        "test.blind-skeleton",
        "demo.item.skeleton-remains",
    );
    game.apply_player_melee_status(STATUS_BLINDNESS, 20, "test.blindness");
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.archer-create-arrows",
        TargetSelection::Item {
            item_id: "test.blind-skeleton".to_owned(),
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("blocked class power should resolve normally");
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.blind-skeleton")
    );
    assert!(events.iter().any(|event| matches!(event,
        DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "blind"
    )));
}

#[test]
fn create_ammunition_accepts_original_junk_bones_and_skeleton_corpses_on_floor_or_pack() {
    let mut game = archer_game(37);
    game.progress.level = 20;
    game.debug_ability_casts_succeed = true;
    let cases = [
        (
            "test.pottery",
            "demo.item.shard-of-pottery",
            ItemLocation::Inventory,
            None,
            2,
        ),
        (
            "test.stick",
            "demo.item.broken-stick",
            ItemLocation::Ground(game.player.position),
            None,
            2,
        ),
        (
            "test.bones",
            "demo.item.skeleton-remains",
            ItemLocation::Inventory,
            None,
            1,
        ),
        (
            "test.skeleton-corpse",
            "demo.item.corpse-remains",
            ItemLocation::Ground(game.player.position),
            Some("demo.actor.skeleton-orc"),
            1,
        ),
    ];
    for (id, kind_id, location, origin_actor_kind_id, quantity) in cases {
        give_inventory_item(&mut game, id, kind_id);
        let source = game
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("source material should exist");
        source.location = location;
        source.origin_actor_kind_id = origin_actor_kind_id.map(str::to_owned);
        source.quantity = quantity;
        let mut events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.archer-create-arrows",
            TargetSelection::Item {
                item_id: id.to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("original source material should create arrows");
        let remaining = game
            .items
            .iter()
            .find(|item| item.id == id)
            .map_or(0, |item| item.quantity);
        assert_eq!(remaining, quantity - 1, "only one {kind_id} should be used");
        let destination_ids = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    resolution.effects.iter().find_map(|effect| match effect {
                        AbilityEffectResolutionDto::CreateAmmunition {
                            source_item_id,
                            destination_item_ids,
                            ..
                        } if source_item_id.as_deref() == Some(id) => {
                            Some(destination_item_ids.clone())
                        }
                        _ => None,
                    })
                }
                _ => None,
            })
            .expect("created ammunition should report its destination");
        assert!(destination_ids.iter().all(|destination_id| {
            let item = game
                .items
                .iter()
                .find(|item| item.id == *destination_id)
                .expect("created ammunition should remain available");
            game.item_identification(item) == ItemIdentificationDto::Identified
        }));
    }
}

#[test]
fn original_neutral_apply_magic_covers_quality_curses_egos_and_damage_dice() {
    let mut game = archer_game(41);
    game.progress.level = 50;
    let template = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("birth arrows should exist")
        .clone();
    let mut saw_ordinary = false;
    let mut saw_fine = false;
    let mut saw_heavy_curse = false;
    let mut saw_slaying = false;
    let mut saw_elemental = false;
    let mut saw_supercharged = false;
    let mut persistent_sample = None;

    for seed in 0..20_000 {
        game.rng = RfbRng::seeded(seed);
        let mut item = template.clone();
        game.apply_rfb_ammunition_magic(&mut item);
        assert_eq!(item.origin_kind, Some(ItemOriginKindDto::PlayerMade));
        assert_eq!(item.discount_percent, 99);
        if item.quality == ItemQualityDto::Ordinary && item.curse.is_none() {
            saw_ordinary = true;
        }
        if item.quality == ItemQualityDto::Fine {
            saw_fine = true;
            assert!(item.enchantments.to_hit > 0 && item.enchantments.to_damage > 0);
        }
        if item.curse == Some(ItemCurseSeverityDto::Heavy) {
            saw_heavy_curse = true;
            assert!(item.enchantments.to_hit < 0 && item.enchantments.to_damage < 0);
        }
        if item.affix_ids == ["rfb-legacy.affix.slaying"] {
            saw_slaying = true;
            assert!(!item.rolled_affixes[0].properties.slays.is_empty());
        }
        if item.affix_ids == ["demo.affix.ammo-elemental"] {
            saw_elemental = true;
            assert!(!item.rolled_affixes[0].properties.brands.is_empty());
        }
        if let Some(dice) = item.damage_dice_override {
            saw_supercharged = true;
            assert!((4..=9).contains(&dice));
        }
        if persistent_sample.is_none()
            && item.quality == ItemQualityDto::Exceptional
            && item.damage_dice_override.is_some()
        {
            persistent_sample = Some(item);
        }
        if saw_ordinary
            && saw_fine
            && saw_heavy_curse
            && saw_slaying
            && saw_elemental
            && saw_supercharged
            && persistent_sample.is_some()
        {
            break;
        }
    }
    assert!(saw_ordinary && saw_fine && saw_heavy_curse);
    assert!(saw_slaying && saw_elemental && saw_supercharged);

    let sample = persistent_sample.expect("seed search should find a supercharged ego arrow");
    let saved = crate::save::inventory_to_save(std::slice::from_ref(&sample));
    let restored = crate::save::inventory_item_from_dto(saved[0].clone(), &game.content)
        .expect("player-made ammunition should round-trip");
    assert_eq!(restored.origin_kind, sample.origin_kind);
    assert_eq!(restored.discount_percent, sample.discount_percent);
    assert_eq!(restored.damage_dice_override, sample.damage_dice_override);
    assert_eq!(restored.enchantments, sample.enchantments);
    assert_eq!(restored.rolled_affixes, sample.rolled_affixes);

    let mut invalid = saved[0].clone();
    invalid.discount_percent = 50;
    assert!(crate::save::inventory_item_from_dto(invalid, &game.content).is_err());
}

#[test]
fn player_made_ammunition_dice_and_rolled_brands_feed_the_projectile_profile() {
    let mut game = archer_game(43);
    let arrows = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("birth arrows should exist");
    let mut properties = AffixPropertyBundleDefinition::default();
    properties.brands.insert(WeaponBrand::Fire);
    arrows.origin_kind = Some(ItemOriginKindDto::PlayerMade);
    arrows.discount_percent = 99;
    arrows.damage_dice_override = Some(9);
    arrows.quality = ItemQualityDto::Exceptional;
    arrows.affix_ids = vec!["demo.affix.ammo-elemental".to_owned()];
    arrows.rolled_affixes = vec![RolledAffixState {
        affix_id: "demo.affix.ammo-elemental".to_owned(),
        properties,
    }];

    let profile = game
        .player_projectile_profile()
        .expect("equipped bow and branded arrows should resolve");
    assert_eq!(profile.damage_dice, 9);
    let target = &game.player;
    let definition = game
        .actor_runtime_definition(target)
        .expect("player definition should remain available");
    assert_ne!(
        target.resistances.level(DamageType::Fire),
        ResistanceLevel::Immune
    );
    assert_eq!(
        game.player_projectile_damage_multiplier(&profile, target, definition),
        24
    );
}

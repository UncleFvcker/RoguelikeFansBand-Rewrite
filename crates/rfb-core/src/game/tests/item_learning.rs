// SPDX-License-Identifier: MPL-2.0
use super::support::{give_inventory_item, replace_terrain};
use super::*;
use crate::game::player_combat::ProjectileMode;
use rfb_content::{
    ActorDamageType, ActorResistanceLevel, RfbPvalDefinition, RfbPvalFlagDefinition,
};

fn empty_game() -> Game {
    let mut game = Game::new_with_build(610, "demo.build.warrior").unwrap();
    game.items.clear();
    game.entities.clear();
    game.item_property_knowledge.clear();
    game.item_lore = Default::default();
    game
}

fn armed_game() -> Game {
    let mut game = empty_game();
    give_inventory_item(&mut game, "weapon", "demo.item.dagger");
    game.equip_inventory_item("weapon", None).unwrap();
    game
}

fn target(game: &mut Game) {
    game.player.position = Position { x: 10, y: 10 };
    for x in 9..=20 {
        replace_terrain(game, Position { x, y: 10 }, "demo.terrain.floor");
    }
    game.glow.fill(true);
    let mut actor = game.generated_actor(
        "target".into(),
        "demo.actor.jackal",
        Position { x: 11, y: 10 },
    );
    actor.hp = 1_000_000;
    game.entities.push(actor);
    game.reveal_current_visibility();
}

#[test]
fn wearing_learns_obvious_and_zero_pval_flags_without_identifying_hidden_powers() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "weapon", "demo.item.dagger");
    let item = &mut game.items[0];
    item.intrinsic_properties.modifiers.strength = 2;
    item.intrinsic_properties.rfb_pval = Some(RfbPvalDefinition {
        value: 0,
        flags: BTreeSet::from([RfbPvalFlagDefinition::Speed]),
    });
    item.intrinsic_properties
        .passives
        .insert(EquipmentPassive::Regeneration);
    item.intrinsic_properties.brands.insert(WeaponBrand::Fire);
    item.intrinsic_properties
        .resistances
        .insert(ActorDamageType::Fire, ActorResistanceLevel::Resistant);
    assert!(
        game.equip_inventory_item("weapon", Some("missing-slot"))
            .is_none()
    );
    assert!(!game.item_property_knowledge.contains_key("weapon"));
    game.equip_inventory_item("weapon", None).unwrap();
    let knowledge = &game.item_property_knowledge["weapon"];
    assert!(knowledge.known_flags.is_superset(&BTreeSet::from([
        "STR".into(),
        "SPEED".into(),
        "REGEN".into()
    ])));
    assert!(!knowledge.appraised && !knowledge.identified && knowledge.known_affix_ids.is_empty());
    assert!(!knowledge.known_flags.contains("RES_FIRE"));
    assert!(game.visible_item_brands(&game.items[0]).is_empty());
    assert_eq!(game.visible_item_modifiers(&game.items[0]).strength, 2);
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    restored.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(
        restored.item_property_knowledge["weapon"].known_flags,
        knowledge.known_flags
    );
}

#[test]
fn priest_learns_blessing_on_wear_but_warrior_does_not() {
    for (build, learns) in [
        ("demo.build.warrior", false),
        ("demo.build.priest-life-sorcery", true),
    ] {
        let mut game = Game::new_with_build(610, build).unwrap();
        game.items.clear();
        give_inventory_item(&mut game, "blessed", "demo.item.dagger");
        game.items[0]
            .intrinsic_weapon_traits
            .insert(WeaponTraitDto::Blessed);
        game.equip_inventory_item("blessed", None).unwrap();
        assert_eq!(
            game.known_item_flags(&game.items[0]).contains("BLESSED"),
            learns
        );
    }
}

#[test]
fn resistance_learning_distinguishes_reduction_immunity_vulnerability_and_armor() {
    for (level, flag) in [
        (ActorResistanceLevel::Resistant, "RES_FIRE"),
        (ActorResistanceLevel::Immune, "IM_FIRE"),
        (ActorResistanceLevel::Vulnerable, "VULN_FIRE"),
    ] {
        let mut game = armed_game();
        game.items[0]
            .intrinsic_properties
            .resistances
            .insert(ActorDamageType::Fire, level);
        give_inventory_item(&mut game, "pack", "demo.item.dagger");
        game.items[1].intrinsic_properties = game.items[0].intrinsic_properties.clone();
        let rng = game.rng.clone();
        game.reduce_player_damage(resolve_damage(
            DamagePacket::after_armor(100, 50, DamageType::Physical),
            ResistanceLevel::Normal,
        ));
        game.reduce_player_damage(resolve_damage(
            DamagePacket::new(0, DamageType::Fire),
            level.into(),
        ));
        assert!(!game.known_item_flags(&game.items[0]).contains(flag));
        game.reduce_player_damage(resolve_damage(
            DamagePacket::new(100, DamageType::Fire),
            level.into(),
        ));
        assert!(game.known_item_flags(&game.items[0]).contains(flag));
        assert!(!game.known_item_flags(&game.items[1]).contains(flag));
        assert_eq!(game.rng, rng);
    }
}

#[test]
fn blocked_status_and_attribute_drain_learn_only_the_observed_protection() {
    let mut game = armed_game();
    game.items[0]
        .intrinsic_properties
        .status_immunities
        .push(STATUS_PARALYSIS.into());
    game.items[0]
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::SustainStrength);
    game.items[0]
        .intrinsic_properties
        .resistances
        .insert(ActorDamageType::Confusion, ActorResistanceLevel::Resistant);
    game.apply_player_melee_status(STATUS_PARALYSIS, 0, "test");
    assert!(!game.known_item_flags(&game.items[0]).contains("FREE_ACT"));
    game.apply_player_melee_status(STATUS_PARALYSIS, 5, "test");
    assert!(!game.player_has_status_kind(STATUS_PARALYSIS));
    assert!(game.known_item_flags(&game.items[0]).contains("FREE_ACT"));
    let strength = game.progress.attributes.strength;
    game.resolve_monster_attribute_drain(AttributeKind::Strength);
    assert_eq!(game.progress.attributes.strength, strength);
    assert!(game.known_item_flags(&game.items[0]).contains("SUST_STR"));
    assert!(!game.known_item_flags(&game.items[0]).contains("RES_CONF"));
}

#[test]
fn melee_hit_learns_matching_slay_and_nonimmune_brand_but_preview_and_miss_do_not() {
    let mut base = armed_game();
    target(&mut base);
    base.items[0]
        .affix_ids
        .push("demo.affix.frost-hunter".into());
    base.items[0]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    base.items[0]
        .intrinsic_properties
        .slays
        .insert(SlayTarget::Dragon, SlayLevel::Kill);
    base.entities[0]
        .resistances
        .set(DamageType::Cold, ResistanceLevel::Immune);
    give_inventory_item(&mut base, "unused", "demo.item.dagger");
    base.items[1]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    let before = base.item_property_knowledge.clone();
    let _ = base.snapshot();
    let definition = base.actor_runtime_definition(&base.entities[0]).unwrap();
    let _ = base.item_damage_multiplier(&base.items[0], &base.entities[0], definition);
    assert_eq!(base.item_property_knowledge, before);
    let mut miss = base.clone();
    miss.items[0]
        .intrinsic_properties
        .equipment_bonuses
        .melee_skill = -1_000_000;
    miss.resolve_player_melee(
        0,
        false,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(!miss.known_item_flags(&miss.items[0]).contains("BRAND_FIRE"));
    assert_eq!(miss.entities[0].hp, 1_000_000);
    let mut hit = false;
    for seed in 0..32 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut already_known = game.clone();
        already_known.learn_item_flag("weapon", "SLAY_ANIMAL");
        already_known.learn_item_flag("weapon", "BRAND_FIRE");
        already_known
            .resolve_player_melee(
                0,
                false,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        game.resolve_player_melee(
            0,
            false,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.rng, already_known.rng);
        assert_eq!(game.entities[0].hp, already_known.entities[0].hp);
        if game.entities[0].hp == 1_000_000 {
            continue;
        }
        let flags = game.known_item_flags(&game.items[0]);
        assert!(flags.contains("SLAY_ANIMAL") && flags.contains("BRAND_FIRE"));
        assert!(!flags.contains("KILL_DRAGON") && !flags.contains("BRAND_COLD"));
        assert!(!game.known_item_flags(&game.items[1]).contains("BRAND_FIRE"));
        hit = true;
        break;
    }
    assert!(hit, "a seeded real melee hit must exercise learning");
}

#[test]
fn fired_last_arrow_and_split_stack_keep_hit_knowledge_on_recovery_or_breakage() {
    let mut base = empty_game();
    target(&mut base);
    give_inventory_item(&mut base, "bow", "demo.item.short-bow");
    base.equip_inventory_item("bow", None).unwrap();
    give_inventory_item(&mut base, "arrows", "demo.item.arrow");
    base.items[1]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    for quantity in [1, 3] {
        let mut recovered = false;
        let mut broken = false;
        for seed in 0..64 {
            let mut game = base.clone();
            game.items[1].quantity = quantity;
            game.rng = RfbRng::seeded(seed);
            game.resolve_player_projectile(
                TargetSelection::Direction {
                    direction: Direction::East,
                },
                ProjectileMode::Normal,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            if game.entities[0].hp == 1_000_000 {
                continue;
            }
            let arrow = game.items.iter().find(|item| {
                item.kind_id == "demo.item.arrow"
                    && matches!(item.location, ItemLocation::Ground(_))
            });
            if let Some(arrow) = arrow {
                assert!(game.known_item_flags(arrow).contains("BRAND_FIRE"));
                recovered = true;
            } else {
                broken = true;
            }
            if quantity > 1 {
                let remaining = game.items.iter().find(|item| item.id == "arrows").unwrap();
                assert!(game.known_item_flags(remaining).contains("BRAND_FIRE"));
            } else if arrow.is_none() {
                assert!(!game.item_property_knowledge.contains_key("arrows"));
            }
            if recovered && broken {
                break;
            }
        }
        assert!(
            recovered && broken,
            "cover both actual projectile settlements"
        );
    }
}

#[test]
fn thrown_weapon_learns_its_brand_while_detached_from_inventory() {
    let mut base = empty_game();
    target(&mut base);
    give_inventory_item(&mut base, "thrown", "demo.item.dagger");
    base.items[0]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    for seed in 0..32 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        game.throw_inventory_item(
            "thrown",
            Direction::East,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.entities[0].hp == 1_000_000 {
            continue;
        }
        if let Some(item) = game.items.iter().find(|item| item.id == "thrown") {
            assert!(matches!(item.location, ItemLocation::Ground(_)));
            assert!(game.known_item_flags(item).contains("BRAND_FIRE"));
            return;
        }
    }
    panic!("a seeded real throw must hit and preserve the recovered weapon's lore");
}

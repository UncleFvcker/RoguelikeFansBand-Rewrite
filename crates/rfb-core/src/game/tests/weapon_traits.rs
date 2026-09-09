// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{MeleeDamageDiceDto, WeaponTraitDto};

use super::{
    support::{clear_monsters, give_inventory_item},
    *,
};
use crate::effect::advance_status_ticks;
use crate::game::player_stats::good_priest_weapon_penalty;

fn weapon_index(game: &Game) -> usize {
    game.items
        .iter()
        .position(|item| {
            matches!(
                &item.location,
                ItemLocation::Equipped { slot_id }
                    if game.body_slot_type(slot_id) == Some("weapon")
            )
        })
        .expect("test character should have an equipped weapon")
}

fn add_weapon_trait(game: &mut Game, trait_: WeaponTraitDto, dice: u16, sides: u16) {
    let index = weapon_index(game);
    game.items[index].rolled_affixes.push(RolledAffixState {
        affix_id: format!("test.weapon-trait.{trait_:?}"),
        melee_damage_dice: Some(MeleeDamageDiceDto { dice, sides }),
        weapon_traits: BTreeSet::from([trait_]),
        ..RolledAffixState::default()
    });
}

fn clear_weapon_traits(game: &mut Game) {
    let index = weapon_index(game);
    for affix in &mut game.items[index].rolled_affixes {
        affix.weapon_traits.clear();
    }
}

fn melee_game(seed: u64, build_id: &str) -> Game {
    let mut game = Game::new_with_build(seed, build_id).expect("test build should create");
    clear_monsters(&mut game);
    game.player.position = Position { x: 3, y: 3 };
    game.push_generated_actor(
        "test.weapon-trait-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );
    game.entities[0].hp = 100_000;
    game.entities[0].max_hp = 100_000;
    game
}

fn resolve_melee(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("test melee should resolve");
    events
}

fn hit_damage(events: &[DomainEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::PlayerMeleeHit { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .collect()
}

fn tomte_form(game: &mut Game) {
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tomte-combat").status;
    form.granted_race_id = Some("rfb-legacy.race.tomte".to_owned());
    game.player.statuses.push(form);
    game.player.hp = game.effective_player_max_hp();
}

fn tomte_without_melee_penalty() -> Arc<ContentCatalog> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut content = rfb_content::compile_pack_dir(&path).unwrap().content;
    content
        .races
        .iter_mut()
        .find(|race| race.id == "rfb-legacy.race.tomte")
        .unwrap()
        .melee_damage_percent = 100;
    Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(content).unwrap(),
    ))
}

#[test]
fn tomte_scales_weapon_and_innate_damage_after_criticals_without_changing_rng() {
    let mut base = melee_game(0, "demo.build.warrior");
    tomte_form(&mut base);
    assert!(base.gain_mutation("rfb.mutation.horns", &mut Vec::new()));
    let neutral_content = tomte_without_melee_penalty();
    for (damage, expected) in [
        (-1, 0),
        (0, 0),
        (1, 1),
        (2, 2),
        (3, 2),
        (8, 7),
        (25, 21),
        (100, 82),
    ] {
        assert_eq!(base.scale_player_melee_damage(damage), expected);
    }
    let melee_damage = |events: Vec<DomainEvent>| {
        events
            .into_iter()
            .filter_map(|event| match event {
                DomainEvent::PlayerMeleeHit { damage, .. } => Some((false, damage.raw)),
                DomainEvent::MutationMeleeHit { damage, .. } => Some((true, damage.raw)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let stats = base.player_derived_stats();
    let weapon = base.player_melee_profile(&stats);
    let innate = base.player_mutation_innate_attack_profiles(&stats);
    let maximum = |profile: &super::super::player_stats::ResolvedAttackProfile| {
        i32::from(profile.damage_dice) * i32::from(profile.damage_sides) + profile.to_damage
    };
    let mut hits = [0, 0];
    let mut criticals = [false, false];
    for seed in 0..128 {
        let mut tomte = base.clone();
        tomte.rng = RfbRng::seeded(seed);
        let mut control = tomte.clone();
        control.content = neutral_content.clone();
        let actual = melee_damage(resolve_melee(&mut tomte));
        let unscaled = melee_damage(resolve_melee(&mut control));
        assert_eq!(actual.len(), unscaled.len());
        for ((is_innate, damage), (control_innate, original)) in actual.into_iter().zip(unscaled) {
            assert_eq!(is_innate, control_innate);
            assert_eq!(damage, (original * 82 + 50) / 100, "seed {seed}");
            let index = usize::from(is_innate);
            hits[index] += 1;
            criticals[index] |= original > maximum(if is_innate { &innate[0] } else { &weapon });
        }
        assert_eq!(tomte.rng, control.rng, "seed {seed}");
    }
    assert!(hits.into_iter().all(|count| count > 0));
    assert_eq!(criticals, [true, true]);
    base.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(base.player_melee_damage_percent(), 100);
    assert_eq!(base.scale_player_melee_damage(25), 25);
}

#[test]
fn tomte_melee_preview_uses_the_damage_rule_and_hides_unidentified_equipment() {
    let mut game = melee_game(0, "demo.build.warrior");
    tomte_form(&mut game);
    assert!(game.gain_mutation("rfb.mutation.horns", &mut Vec::new()));
    add_weapon_trait(&mut game, WeaponTraitDto::Order, 2, 6);
    let equipped = game
        .items
        .iter()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    for id in &equipped {
        game.identify_item_instance(id, ItemIdentificationRequest::new(true));
    }
    let rng = game.rng.clone();
    let data = game.snapshot().player.trait_details;
    assert_eq!(game.rng, rng);
    assert_eq!(data.melee_damage.len(), 2);
    let weapon = game.player_melee_profile(&game.player_derived_stats());
    let expected = ((12 + weapon.to_damage).max(0) * 82 + 50) / 100;
    assert_eq!(data.melee_damage[0].base_damage, Some([expected, expected]));
    assert_eq!(data.melee_damage[0].damage_percent, 82);
    assert_eq!(data.melee_damage[1].attack_name.as_deref(), Some("长角"));
    let innate = game.player_mutation_innate_attack_profiles(&game.player_derived_stats());
    assert_eq!(
        data.melee_damage[1].base_damage,
        Some([
            ((2 + innate[0].to_damage).max(0) * 82 + 50) / 100,
            ((12 + innate[0].to_damage).max(0) * 82 + 50) / 100,
        ])
    );
    game.item_property_knowledge.remove(&equipped[0]);
    assert!(
        game.snapshot()
            .player
            .trait_details
            .melee_damage
            .iter()
            .all(|preview| preview.base_damage.is_none() && preview.damage_percent == 82)
    );
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert!(
        game.snapshot()
            .player
            .trait_details
            .melee_damage
            .iter()
            .all(|preview| preview.damage_percent == 100)
    );
}

#[test]
fn tomte_melee_penalty_does_not_scale_projectiles_or_spell_damage() {
    let mut base = melee_game(0, "demo.build.archer");
    tomte_form(&mut base);
    for position in [base.player.position, base.entities[0].position] {
        super::support::replace_terrain(&mut base, position, "demo.terrain.floor");
    }
    let neutral_content = tomte_without_melee_penalty();
    let mut hits = 0;
    for seed in 0..16 {
        let mut tomte = base.clone();
        tomte.rng = RfbRng::seeded(seed);
        let mut control = tomte.clone();
        control.content = neutral_content.clone();
        let shoot = |game: &mut Game| {
            let mut events = Vec::new();
            game.resolve_player_projectile(
                TargetSelection::Direction {
                    direction: Direction::East,
                },
                super::super::player_combat::ProjectileMode::Normal,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            events
        };
        let events = shoot(&mut tomte);
        assert_eq!(events, shoot(&mut control));
        hits += events
            .iter()
            .filter(|event| matches!(event, DomainEvent::ProjectileHit { .. }))
            .count();
        assert_eq!(tomte.rng, control.rng);
    }
    assert!(hits > 0);
    let trace = ProjectileTrace {
        origin: base.player.position,
        impact: base.entities[0].position,
        landing: base.entities[0].position,
        traversed: vec![base.entities[0].position],
    };
    let damage = base
        .resolve_ability_damage_to_entity(
            0,
            "test.tomte-spell",
            DamageType::Mana,
            100,
            trace,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(damage.raw, 100);
    assert_eq!(damage.applied, 100);
}

fn force_melee_misses(game: &mut Game) {
    game.player.statuses.push(StatusInstance {
        kind_id: "test.weapon-trait.no-melee-skill".to_owned(),
        intensity: 1,
        remaining_ticks: 10,
        source_id: Some("test.weapon-trait".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto {
            melee_skill: -10_000,
            ..EquipmentBonusesDto::default()
        },
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
}

fn tonberry_game(build: &str) -> Game {
    let mut game = melee_game(0, build);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tonberry-combat").status;
    form.granted_race_id = Some("rfb-legacy.race.tonberry".to_owned());
    game.player.statuses.push(form);
    game
}

#[test]
fn tonberry_dual_wielding_combines_equipment_blows_and_reports_each_source() {
    let mut game = tonberry_game("demo.build.warrior");
    game.progress.level = 10;
    let offhand_slot = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "shield")
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.offhand", "demo.item.dagger");
    assert!(
        game.equip_inventory_item("test.offhand", Some(&offhand_slot))
            .is_some()
    );
    let weapon = weapon_index(&game);
    let weapon_id = game.items[weapon].id.clone();
    game.items[weapon]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks_delta_percent = 75;
    let equipped = game
        .items
        .iter()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    for id in equipped {
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    }
    let stats = game.player_derived_stats();
    let mut human = game.clone();
    human.player.statuses.clear();
    let profiles = game.player_melee_profiles(&stats);
    assert_eq!(profiles.len(), 2);
    let human_profiles = human.player_melee_profiles(&stats);
    let rate = |profile: &crate::game::player_stats::ResolvedAttackProfile| {
        i32::from(profile.attacks) * 100 + i32::from(profile.extra_attack_chance_percent)
    };
    for (actual, control) in profiles.iter().zip(&human_profiles) {
        assert_eq!(actual.to_damage, control.to_damage + 10);
        assert_eq!(rate(actual), (rate(control) - 40).max(0));
    }
    let details = game.snapshot().player.trait_details;
    assert_eq!(details.melee_damage.len(), 2);
    let attacks = details
        .stats
        .iter()
        .find(|stat| stat.id == "melee-attacks-hundredths")
        .unwrap();
    assert_eq!(attacks.value, Some(profiles.iter().map(rate).sum()));
    assert_eq!(
        attacks.value,
        Some(attacks.sources.iter().map(|source| source.amount).sum())
    );
    assert_eq!(
        attacks
            .sources
            .iter()
            .find(|source| source.source_id == weapon_id)
            .unwrap()
            .amount,
        75
    );
    assert_eq!(
        attacks
            .sources
            .iter()
            .find(|source| source.source_id == "rfb-legacy.race.tonberry")
            .unwrap()
            .amount,
        -80
    );
}

#[test]
fn tonberry_weapon_profiles_preserve_fractional_blows_and_known_damage() {
    let mut game = tonberry_game("demo.build.warrior");
    let weapon = weapon_index(&game);
    let base_attacks = game.player_derived_stats().melee_attacks.value;
    game.items[weapon]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks += 2 - base_attacks;
    assert!(game.gain_mutation("rfb.mutation.horns", &mut Vec::new()));
    let equipped: Vec<_> = game
        .items
        .iter()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .map(|item| item.id.clone())
        .collect();
    for id in &equipped {
        game.identify_item_instance(id, ItemIdentificationRequest::new(true));
    }
    for (level, blows) in [(1, 196), (24, 104), (25, 100), (26, 96), (49, 4), (50, 0)] {
        game.progress.level = level;
        let stats = game.player_derived_stats();
        let profile = game.player_melee_profile(&stats);
        assert_eq!(
            i32::from(profile.attacks) * 100 + i32::from(profile.extra_attack_chance_percent),
            blows
        );
        assert_eq!(
            profile.to_damage,
            stats.melee_damage_bonus.value + 2 * i32::from(level)
        );
        let innate = game.player_mutation_innate_attack_profiles(&stats);
        let mut human = game.clone();
        human.player.statuses.clear();
        // Pass identical derived stats to isolate the weapon-only rule from racial attributes.
        let control_innate = human.player_mutation_innate_attack_profiles(&stats);
        assert_eq!(innate[0].to_damage, control_innate[0].to_damage);
        assert_eq!(innate[0].attacks, control_innate[0].attacks);
        let rng = game.rng.clone();
        let data = game.snapshot().player.trait_details;
        assert_eq!(game.rng, rng);
        let rate = data
            .stats
            .iter()
            .find(|stat| stat.id == "melee-attacks-hundredths")
            .unwrap();
        assert_eq!(rate.value, Some(blows));
        assert_eq!(
            rate.sources.iter().map(|source| source.amount).sum::<i32>(),
            blows
        );
        assert_eq!(
            data.melee_damage[0].base_damage,
            Some([
                (i32::from(profile.damage_dice) + profile.to_damage).max(0),
                (i32::from(profile.damage_dice) * i32::from(profile.damage_sides)
                    + profile.to_damage)
                    .max(0),
            ])
        );
    }
    game.items[weapon]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks -= 1;
    let below_zero = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        (below_zero.attacks, below_zero.extra_attack_chance_percent),
        (0, 0)
    );
    game.item_property_knowledge.remove(&equipped[0]);
    let data = game.snapshot().player.trait_details;
    let rate = data
        .stats
        .iter()
        .find(|stat| stat.id == "melee-attacks-hundredths")
        .unwrap();
    assert_eq!(rate.value, None);
    assert!(rate.sources.is_empty());
    assert!(
        data.melee_damage
            .iter()
            .all(|preview| preview.base_damage.is_none())
    );

    game.items[weapon].location = ItemLocation::Inventory;
    let stats = game.player_derived_stats();
    let unarmed = game.player_melee_profile(&stats);
    assert!(unarmed.source_item_id.is_none());
    assert_eq!(i32::from(unarmed.attacks), stats.melee_attacks.value);
    assert_eq!(unarmed.extra_attack_chance_percent, 0);
    assert_eq!(unarmed.to_damage, stats.melee_damage_bonus.value);
}

#[test]
fn tonberry_fractional_weapon_attacks_roll_once_and_allow_zero_attacks() {
    let mut base = tonberry_game("demo.build.warrior");
    force_melee_misses(&mut base);
    let weapon = weapon_index(&base);
    let base_attacks = base.player_derived_stats().melee_attacks.value;
    base.items[weapon]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks += 2 - base_attacks;
    for (level, guaranteed, chance) in [(1, 1, 96), (25, 1, 0), (26, 0, 96), (49, 0, 4), (50, 0, 0)]
    {
        base.progress.level = level;
        for seed in 0..32 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let mut expected_rng = game.rng.clone();
            let expected_attacks =
                guaranteed + usize::from(chance > 0 && expected_rng.bounded(100) < chance);
            let events = resolve_melee(&mut game);
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, DomainEvent::PlayerMeleeMissed { .. }))
                    .count(),
                expected_attacks
            );
            assert!(hit_damage(&events).is_empty());
            assert_eq!(game.rng, expected_rng, "level {level}, seed {seed}");
        }
    }
    let mut action = base.clone();
    // Native Tonberry precondition avoids the Human's unrelated unchosen level reward.
    action.build.as_mut().unwrap().race_id = "rfb-legacy.race.tonberry".to_owned();
    action
        .player
        .statuses
        .retain(|status| status.kind_id == STATUS_PLAYER_POLYMORPH);
    let position = action.player.position;
    let tick = action.world_tick;
    let hp = action.entities[0].hp;
    let update = super::support::dispatch_next(
        &mut action,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(
        update.world_tick > tick,
        "zero blows still consume the melee action"
    );
    assert_eq!(action.player.position, position);
    assert_eq!(action.entities[0].hp, hp);
    assert!(base.gain_mutation("rfb.mutation.horns", &mut Vec::new()));
    let events = resolve_melee(&mut base);
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MutationMeleeMissed { .. }))
    );
}

fn neutral_tonberry_content() -> Arc<ContentCatalog> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut content = rfb_content::compile_pack_dir(&path).unwrap().content;
    let race = content
        .races
        .iter_mut()
        .find(|race| race.id == "rfb-legacy.race.tonberry")
        .unwrap();
    race.id = "test.race.tonberry-control".to_owned();
    Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(content).unwrap(),
    ))
}

#[test]
fn tonberry_damage_is_added_after_weapon_criticals_and_does_not_change_shooting() {
    let content = neutral_tonberry_content();
    let mut base = tonberry_game("demo.build.warrior");
    base.progress.level = 25;
    let weapon = weapon_index(&base);
    let attacks = base.player_derived_stats().melee_attacks.value;
    base.items[weapon]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks += 3 - attacks;
    let mut control = base.clone();
    control.content = content.clone();
    control.player.statuses[0].granted_race_id = Some("test.race.tonberry-control".to_owned());
    control.items[weapon]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks -= 1;
    let profile = control.player_melee_profile(&control.player_derived_stats());
    let normal_maximum =
        i32::from(profile.damage_dice) * i32::from(profile.damage_sides) + profile.to_damage;
    let mut critical_seen = false;
    for seed in 0..128 {
        let mut actual = base.clone();
        let mut expected = control.clone();
        actual.rng = RfbRng::seeded(seed);
        expected.rng = actual.rng.clone();
        let raw_hits = |events: Vec<DomainEvent>| {
            events
                .into_iter()
                .filter_map(|event| match event {
                    DomainEvent::PlayerMeleeHit { damage, .. } => Some(damage.raw),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let actual_hits = raw_hits(resolve_melee(&mut actual));
        let expected_hits = raw_hits(resolve_melee(&mut expected));
        assert_eq!(actual_hits.len(), expected_hits.len());
        for (actual, expected) in actual_hits.into_iter().zip(expected_hits) {
            assert_eq!(actual, expected + 50);
            critical_seen |= expected > normal_maximum;
        }
        assert_eq!(actual.rng, expected.rng);
    }
    assert!(critical_seen);

    let mut archer = tonberry_game("demo.build.archer");
    for position in [archer.player.position, archer.entities[0].position] {
        super::support::replace_terrain(&mut archer, position, "demo.terrain.floor");
    }
    let mut control = archer.clone();
    control.content = content;
    control.player.statuses[0].granted_race_id = Some("test.race.tonberry-control".to_owned());
    let shoot = |game: &mut Game| {
        let mut events = Vec::new();
        game.resolve_player_projectile(
            TargetSelection::Direction {
                direction: Direction::East,
            },
            super::super::player_combat::ProjectileMode::Normal,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        events
    };
    let events = shoot(&mut archer);
    assert_eq!(events, shoot(&mut control));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::ProjectileHit { .. }))
    );
    assert_eq!(archer.rng, control.rng);
}

#[test]
fn mana_weapon_uses_current_dice_and_only_pays_for_successful_affordable_hits() {
    let base = melee_game(0, "demo.build.high-mage-arcane");
    assert_eq!(
        base.player_melee_profile(&base.player_derived_stats())
            .attacks,
        1
    );
    let resource_id = base
        .casting_profile()
        .expect("High Mage should have a casting resource")
        .resource_id
        .clone();
    let seed = (0..10_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            !hit_damage(&resolve_melee(&mut game)).is_empty()
        })
        .expect("a deterministic hit seed should exist");

    let mut control = base.clone();
    control.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut control, WeaponTraitDto::ManaBrand, 3, 7);
    clear_weapon_traits(&mut control);
    let control_damage = hit_damage(&resolve_melee(&mut control));

    let mut paid = base.clone();
    paid.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut paid, WeaponTraitDto::ManaBrand, 3, 7);
    paid.resources.get_mut(&resource_id).unwrap().current = 20;
    let paid_damage = hit_damage(&resolve_melee(&mut paid));
    assert!(paid_damage[0] > control_damage[0]);
    assert_eq!(paid.resources[&resource_id].current, 16);

    let mut insufficient = base.clone();
    insufficient.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut insufficient, WeaponTraitDto::ManaBrand, 3, 7);
    insufficient
        .resources
        .get_mut(&resource_id)
        .unwrap()
        .current = 3;
    let insufficient_damage = hit_damage(&resolve_melee(&mut insufficient));
    assert_eq!(insufficient.resources[&resource_id].current, 3);
    assert_eq!(insufficient_damage, control_damage);
    assert_eq!(insufficient.rng.draw_counter, control.rng.draw_counter);

    let mut missed = base.clone();
    missed.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut missed, WeaponTraitDto::ManaBrand, 3, 7);
    force_melee_misses(&mut missed);
    missed.resources.get_mut(&resource_id).unwrap().current = 20;
    let mut missed_control = missed.clone();
    clear_weapon_traits(&mut missed_control);
    assert!(hit_damage(&resolve_melee(&mut missed)).is_empty());
    resolve_melee(&mut missed_control);
    assert_eq!(missed.resources[&resource_id].current, 20);
    assert_eq!(missed.rng.draw_counter, missed_control.rng.draw_counter);
}

#[test]
fn vorpal_and_vorpal2_chain_after_the_shared_weapon_critical_path() {
    let base = melee_game(0, "demo.build.warrior");
    assert_eq!(
        base.player_melee_profile(&base.player_derived_stats())
            .attacks,
        1
    );
    let seed = (0..100_000)
        .find(|seed| {
            let mut normal = base.clone();
            normal.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut normal, WeaponTraitDto::Vorpal, 2, 6);
            let damage = hit_damage(&resolve_melee(&mut normal));
            let mut control = base.clone();
            control.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut control, WeaponTraitDto::Vorpal, 2, 6);
            clear_weapon_traits(&mut control);
            let control_damage = hit_damage(&resolve_melee(&mut control));
            !damage.is_empty() && !control_damage.is_empty() && damage[0] > control_damage[0]
        })
        .expect("a deterministic vorpal chain seed should exist");
    let mut vorpal = base.clone();
    vorpal.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut vorpal, WeaponTraitDto::Vorpal, 2, 6);
    let vorpal_damage = hit_damage(&resolve_melee(&mut vorpal));
    let mut control = base.clone();
    control.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut control, WeaponTraitDto::Vorpal, 2, 6);
    clear_weapon_traits(&mut control);
    let control_damage = hit_damage(&resolve_melee(&mut control));
    assert!(vorpal_damage[0] > control_damage[0]);
    assert!(vorpal.rng.draw_counter > control.rng.draw_counter);

    let vorpal2_seed = (0..100_000)
        .find(|seed| {
            let mut vorpal = base.clone();
            vorpal.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut vorpal, WeaponTraitDto::Vorpal, 2, 6);
            let vorpal_damage = hit_damage(&resolve_melee(&mut vorpal));
            let mut vorpal2 = base.clone();
            vorpal2.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut vorpal2, WeaponTraitDto::Vorpal2, 2, 6);
            let vorpal2_damage = hit_damage(&resolve_melee(&mut vorpal2));
            !vorpal_damage.is_empty()
                && !vorpal2_damage.is_empty()
                && vorpal2_damage[0] > vorpal_damage[0]
        })
        .expect("Vorpal2 should have a deterministic extra chain seed");
    assert!(vorpal2_seed < 100_000);

    let mut missed = base.clone();
    add_weapon_trait(&mut missed, WeaponTraitDto::Vorpal2, 2, 6);
    force_melee_misses(&mut missed);
    let mut missed_control = missed.clone();
    clear_weapon_traits(&mut missed_control);
    assert!(hit_damage(&resolve_melee(&mut missed)).is_empty());
    resolve_melee(&mut missed_control);
    assert_eq!(missed.rng.draw_counter, missed_control.rng.draw_counter);
}

#[test]
fn order_weapon_rolls_maximum_damage_and_skips_weapon_dice_and_critical_rng() {
    let base = melee_game(0, "demo.build.warrior");
    let successful_seeds = (0..10_000)
        .filter_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            add_weapon_trait(&mut game, WeaponTraitDto::Order, 2, 6);
            hit_damage(&resolve_melee(&mut game))
                .first()
                .copied()
                .map(|damage| (seed, damage, game.rng.draw_counter))
        })
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(successful_seeds.len(), 2);
    assert_eq!(successful_seeds[0].1, successful_seeds[1].1);

    let (seed, _, order_draws) = successful_seeds[0];
    let mut normal = base.clone();
    normal.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut normal, WeaponTraitDto::Vorpal, 2, 6);
    clear_weapon_traits(&mut normal);
    assert!(!hit_damage(&resolve_melee(&mut normal)).is_empty());
    assert!(order_draws < normal.rng.draw_counter);

    let item = &base.items[weapon_index(&base)];
    let mut visible = base.clone();
    add_weapon_trait(&mut visible, WeaponTraitDto::Order, 2, 6);
    let profile = visible
        .item_melee_profile(&visible.items[weapon_index(&visible)])
        .expect("equipped weapon should expose a melee profile");
    assert_eq!((profile.damage.dice, profile.damage.sides), (2, 6));
    assert!(base.item_melee_profile(item).is_some());
}

#[test]
fn impact_weapon_reuses_earthquake_and_strong_hit_stun_without_extra_trigger_rng() {
    let base = melee_game(0, "demo.build.warrior");
    let seed = (0..10_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut game, WeaponTraitDto::Impact, 60, 1);
            add_weapon_trait(&mut game, WeaponTraitDto::Order, 60, 1);
            resolve_melee(&mut game)
                .iter()
                .any(|event| matches!(event, DomainEvent::PlayerWeaponEarthquakeResolved { .. }))
        })
        .expect("a deterministic strong impact hit should exist");
    let mut strong = base.clone();
    strong.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut strong, WeaponTraitDto::Impact, 60, 1);
    add_weapon_trait(&mut strong, WeaponTraitDto::Order, 60, 1);
    let events = resolve_melee(&mut strong);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerWeaponEarthquakeResolved { source_item_id, .. }
            if source_item_id == &strong.items[weapon_index(&strong)].id
    )));
    assert!(
        strong.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    let weak_seed = (0..10_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut game, WeaponTraitDto::Impact, 1, 1);
            add_weapon_trait(&mut game, WeaponTraitDto::Order, 1, 1);
            let events = resolve_melee(&mut game);
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::PlayerMeleeHit { .. }))
                && !events.iter().any(|event| {
                    matches!(event, DomainEvent::PlayerWeaponEarthquakeResolved { .. })
                })
        })
        .expect("a deterministic non-triggering impact hit should exist");
    let mut weak = base.clone();
    weak.rng = RfbRng::seeded(weak_seed);
    add_weapon_trait(&mut weak, WeaponTraitDto::Impact, 1, 1);
    add_weapon_trait(&mut weak, WeaponTraitDto::Order, 1, 1);
    let events = resolve_melee(&mut weak);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerWeaponEarthquakeResolved { .. }))
    );
    assert!(
        !weak.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    let mut missed = base.clone();
    add_weapon_trait(&mut missed, WeaponTraitDto::Impact, 1, 1);
    force_melee_misses(&mut missed);
    let mut control = missed.clone();
    clear_weapon_traits(&mut control);
    resolve_melee(&mut missed);
    resolve_melee(&mut control);
    assert_eq!(missed.rng.draw_counter, control.rng.draw_counter);
}

#[test]
fn stun_weapon_checks_post_critical_damage_and_respects_immunity() {
    let base = melee_game(0, "demo.build.warrior");
    let seed = (0..10_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut game, WeaponTraitDto::Stun, 101, 1);
            add_weapon_trait(&mut game, WeaponTraitDto::Order, 101, 1);
            resolve_melee(&mut game);
            game.entities[0]
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_STUN)
        })
        .expect("a deterministic stunning hit should exist");
    let mut stunned = base.clone();
    stunned.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut stunned, WeaponTraitDto::Stun, 101, 1);
    add_weapon_trait(&mut stunned, WeaponTraitDto::Order, 101, 1);
    resolve_melee(&mut stunned);
    assert!(
        stunned.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    let mut immune = base.clone();
    immune.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut immune, WeaponTraitDto::Stun, 101, 1);
    add_weapon_trait(&mut immune, WeaponTraitDto::Order, 101, 1);
    immune.entities[0].statuses.push(StatusInstance {
        kind_id: "test.weapon-trait.stun-immunity".to_owned(),
        intensity: 1,
        remaining_ticks: 10,
        source_id: Some("test.weapon-trait".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::from([STATUS_STUN.to_owned()]),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    resolve_melee(&mut immune);
    assert!(
        !immune.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );
    assert_eq!(stunned.rng.draw_counter, immune.rng.draw_counter);

    let mut too_weak = base.clone();
    too_weak.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut too_weak, WeaponTraitDto::Stun, 1, 1);
    add_weapon_trait(&mut too_weak, WeaponTraitDto::Order, 1, 1);
    resolve_melee(&mut too_weak);
    assert!(
        !too_weak.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    let mut missed = base.clone();
    add_weapon_trait(&mut missed, WeaponTraitDto::Stun, 101, 1);
    force_melee_misses(&mut missed);
    let mut control = missed.clone();
    clear_weapon_traits(&mut control);
    resolve_melee(&mut missed);
    resolve_melee(&mut control);
    assert_eq!(missed.rng.draw_counter, control.rng.draw_counter);
}

#[test]
fn blessed_weapon_resists_curses_and_exempts_good_priest_weapon_penalties() {
    assert!(good_priest_weapon_penalty(true, true, Some(23), false));
    assert!(!good_priest_weapon_penalty(true, true, Some(23), true));
    assert!(!good_priest_weapon_penalty(true, false, Some(23), false));
    assert!(!good_priest_weapon_penalty(true, true, Some(21), false));

    let base = melee_game(0, "demo.build.warrior");
    let seed = (0..10_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut game, WeaponTraitDto::Blessed, 2, 6);
            game.curse_equipped_item(CurseEquippedItemRequest::new(
                EquippedItemCurseTarget::Weapon,
            ))
            .resisted
        })
        .expect("a deterministic blessed resistance seed should exist");
    let mut blessed = base.clone();
    blessed.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut blessed, WeaponTraitDto::Blessed, 2, 6);
    let outcome = blessed.curse_equipped_item(CurseEquippedItemRequest::new(
        EquippedItemCurseTarget::Weapon,
    ));
    assert!(outcome.resisted);
    assert_eq!(blessed.items[weapon_index(&blessed)].curse, None);

    let mut ordinary = base.clone();
    ordinary.rng = RfbRng::seeded(seed);
    let outcome = ordinary.curse_equipped_item(CurseEquippedItemRequest::new(
        EquippedItemCurseTarget::Weapon,
    ));
    assert!(!outcome.resisted);
    assert_eq!(
        ordinary.items[weapon_index(&ordinary)].curse,
        Some(ItemCurseSeverityDto::Normal)
    );

    let mut forced = base.clone();
    add_weapon_trait(&mut forced, WeaponTraitDto::Blessed, 2, 6);
    forced.debug_item_curses_land = true;
    let before = forced.rng.draw_counter;
    let outcome = forced.curse_equipped_item(CurseEquippedItemRequest::new(
        EquippedItemCurseTarget::Weapon,
    ));
    assert!(!outcome.resisted);
    assert_eq!(forced.rng.draw_counter, before);

    let mut no_candidate = base.clone();
    let index = weapon_index(&no_candidate);
    no_candidate.items[index].location = ItemLocation::Inventory;
    let before = no_candidate.rng.draw_counter;
    let outcome = no_candidate.curse_equipped_item(CurseEquippedItemRequest::new(
        EquippedItemCurseTarget::Weapon,
    ));
    assert_eq!(outcome.item_id, None);
    assert_eq!(no_candidate.rng.draw_counter, before);
}

fn wild_statuses(game: &Game) -> Vec<&StatusInstance> {
    game.player
        .statuses
        .iter()
        .filter(|status| {
            status
                .source_id
                .as_deref()
                .is_some_and(|source| source.starts_with("rfb.weapon.wild.slot."))
        })
        .collect()
}

#[test]
fn wild_weapon_activates_on_hit_for_two_ticks_without_spending_rng_on_miss() {
    let base = melee_game(0, "demo.build.warrior");
    let seed = (0..10_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            add_weapon_trait(&mut game, WeaponTraitDto::Wild, 1, 1);
            add_weapon_trait(&mut game, WeaponTraitDto::Order, 1, 1);
            resolve_melee(&mut game)
                .iter()
                .any(|event| matches!(event, DomainEvent::WildWeaponPowerActivated { .. }))
        })
        .expect("a deterministic wild weapon hit should exist");
    let mut hit = base.clone();
    hit.rng = RfbRng::seeded(seed);
    add_weapon_trait(&mut hit, WeaponTraitDto::Wild, 1, 1);
    add_weapon_trait(&mut hit, WeaponTraitDto::Order, 1, 1);
    let events = resolve_melee(&mut hit);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::WildWeaponPowerActivated { source_item_id, .. }
            if source_item_id == &hit.items[weapon_index(&hit)].id
    )));
    assert_eq!(wild_statuses(&hit).len(), 1);
    assert_eq!(wild_statuses(&hit)[0].remaining_ticks, 2);
    assert!(advance_status_ticks(&mut hit.player.statuses, 1).is_empty());
    assert_eq!(wild_statuses(&hit)[0].remaining_ticks, 1);
    assert_eq!(advance_status_ticks(&mut hit.player.statuses, 1).len(), 1);
    assert!(wild_statuses(&hit).is_empty());

    let mut missed = base.clone();
    add_weapon_trait(&mut missed, WeaponTraitDto::Wild, 1, 1);
    force_melee_misses(&mut missed);
    let mut control = missed.clone();
    clear_weapon_traits(&mut control);
    resolve_melee(&mut missed);
    resolve_melee(&mut control);
    assert!(wild_statuses(&missed).is_empty());
    assert_eq!(missed.rng.draw_counter, control.rng.draw_counter);
}

#[test]
fn wild_weapon_selects_inactive_powers_before_filling_or_replacing_five_slots() {
    let mut game = Game::new(0x5749_4c44_0005);
    let mut events = Vec::new();
    for _ in 0..5 {
        game.resolve_wild_weapon_strike("test.weapon.wild", &mut events);
    }
    let before = wild_statuses(&game)
        .into_iter()
        .map(|status| (status.source_id.clone().unwrap(), status.kind_id.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(before.len(), 5);
    assert_eq!(before.values().collect::<BTreeSet<_>>().len(), 5);
    assert!(
        wild_statuses(&game)
            .iter()
            .all(|status| status.remaining_ticks == 2)
    );

    game.resolve_wild_weapon_strike("test.weapon.wild", &mut events);
    let after = wild_statuses(&game)
        .into_iter()
        .map(|status| (status.source_id.clone().unwrap(), status.kind_id.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(after.len(), 5);
    assert_eq!(after.values().collect::<BTreeSet<_>>().len(), 5);
    assert_eq!(
        before
            .values()
            .filter(|kind| after.values().any(|after_kind| after_kind == *kind))
            .count(),
        4
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::WildWeaponPowerActivated { .. }))
            .count(),
        6
    );
}

#[test]
fn wild_weapon_weight_table_builds_all_fourteen_concrete_status_effects() {
    let mut statuses = BTreeMap::new();
    for seed in 0..10_000 {
        let mut game = Game::new(seed);
        game.progress.level = 50;
        game.resolve_wild_weapon_strike("test.weapon.wild", &mut Vec::new());
        let status = wild_statuses(&game)[0].clone();
        statuses.entry(status.kind_id.clone()).or_insert(status);
        if statuses.len() == 14 {
            break;
        }
    }
    assert_eq!(statuses.len(), 14);
    assert_eq!(
        statuses["rfb.status.infravision"]
            .granted_equipment_bonuses
            .infravision,
        3
    );
    assert_eq!(statuses["rfb.status.blessed"].granted_modifiers.defense, 5);
    assert_eq!(
        statuses[STATUS_BERSERK]
            .granted_equipment_bonuses
            .melee_damage,
        13
    );
    assert_eq!(
        statuses[STATUS_BASIC_RESISTANCE].granted_resistances.len(),
        5
    );
    assert_eq!(
        statuses["rfb.status.stone-skin"].granted_modifiers.defense,
        50
    );
    assert!(statuses["rfb.status.passwall"].grants_wall_passage);
    assert_eq!(
        statuses["rfb.status.wild-invulnerability"].incoming_damage_percent,
        0
    );
    assert_eq!(statuses[STATUS_WRAITHFORM].incoming_damage_percent, 50);
    assert_eq!(
        statuses[STATUS_WRAITHFORM]
            .granted_resistances
            .get(&DamageType::Dark),
        Some(&ResistanceLevel::Immune)
    );

    let mut magic = Game::new(0);
    magic.progress.level = 50;
    magic
        .player
        .statuses
        .push(statuses[STATUS_MAGIC_RESISTANCE].clone());
    assert!(magic.player_derived_stats().saving_throw_skill.value >= 145);
    let mut light_speed = Game::new(0);
    light_speed
        .player
        .statuses
        .push(statuses[STATUS_LIGHT_SPEED].clone());
    assert_eq!(light_speed.player_derived_stats().speed.value, 199);
    let mut telepathy = Game::new(0);
    telepathy
        .player
        .statuses
        .push(statuses[STATUS_TELEPATHY].clone());
    assert!(telepathy.player_has_telepathy());

    let speed_seed = (0..10_000)
        .find(|seed| {
            let mut game = Game::new(*seed);
            game.resolve_wild_weapon_strike("test.weapon.wild", &mut Vec::new());
            wild_statuses(&game)[0].kind_id == STATUS_HASTE
        })
        .expect("a deterministic wild speed seed should exist");
    let mut clobbered = Game::new(speed_seed);
    clobbered.player.statuses.push(StatusInstance {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 1,
        remaining_ticks: 20,
        source_id: Some("test.normal-haste".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    clobbered.resolve_wild_weapon_strike("test.weapon.wild", &mut Vec::new());
    assert_eq!(
        clobbered
            .player
            .statuses
            .iter()
            .filter(|status| status.kind_id == STATUS_HASTE)
            .count(),
        1
    );
    assert_eq!(wild_statuses(&clobbered)[0].remaining_ticks, 2);
}

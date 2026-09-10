// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, dispatch_next, give_inventory_item};
use super::*;

const BUILD: &str = "demo.build.berserker";

mod rewards;
mod spells;

#[test]
fn desktop_fixture_preserves_real_level_and_item_save_invariants() {
    let mut game = Game::new_with_build(923, BUILD).unwrap();
    game.debug_prepare_berserker_e2e(30, true, true).unwrap();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let snapshot = restored.snapshot();
    assert_eq!(snapshot.player.progress.level, 30);
    assert_eq!(snapshot.player.hp, 1);
    assert!(
        snapshot
            .player
            .abilities
            .iter()
            .all(|ability| !ability.can_cast
                && ability.unavailable_reason.as_deref() == Some("insufficient-hit-points"))
    );
}

fn berserker(level: u16) -> Game {
    let mut game = Game::new_with_build(923, BUILD).unwrap();
    clear_monsters(&mut game);
    game.progress.level = level;
    game.progress.max_level = level;
    game.refresh_character_skills();
    game.refresh_player_ability_state();
    game.player.hp = game.effective_player_max_hp();
    game
}

#[test]
fn birth_has_no_mana_negative_skills_equipment_and_source_proficiencies() {
    let game = berserker(1);
    let snapshot = game.snapshot();
    assert!(game.resources.is_empty());
    assert!(snapshot.player.ability_learning.is_none());
    assert!(game.learned_abilities.is_empty());
    assert_eq!(game.progress.dual_wielding_proficiency, 4_000);
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 0);
    for (kind, expected) in [
        ("demo.item.broad-axe", (4_000, 8_000)),
        ("demo.item.sling", (0, 0)),
        ("demo.item.wizardstaff", (0, 0)),
    ] {
        let proficiency = snapshot
            .player
            .progress
            .weapon_proficiencies
            .iter()
            .find(|entry| entry.item_kind_id == kind)
            .unwrap();
        assert_eq!((proficiency.current, proficiency.maximum), expected);
    }
    for kind in ["demo.item.broad-axe", "demo.item.augmented-chain-mail"] {
        let item = game.items.iter().find(|item| item.kind_id == kind).unwrap();
        assert!(matches!(item.location, ItemLocation::Equipped { .. }));
        assert_eq!(
            game.item_identification(item),
            ItemIdentificationDto::Identified
        );
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.kind_id == "demo.item.healing-potion")
            .unwrap()
            .quantity,
        1
    );
    assert_eq!(game.virtues[0].kind, VirtueKindDto::Valour);
    assert_eq!(game.virtues[1].kind, VirtueKindDto::Individualism);
    assert!(game.progress.skills["demo.skill.device"].current < -900);
    assert!(game.player_derived_stats().device_skill.value < -900);
    assert!(game.player_derived_stats().ranged_skill.value < -1900);
    assert!(game.player_derived_stats().saving_throw_skill.value < 0);
    assert_eq!(game.player_derived_stats().stealth_skill.value, 0);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn permanent_berserk_has_level_boundaries_and_potions_only_heal() {
    let mut game = berserker(1);
    assert!(game.player_has_status_kind(STATUS_BERSERK));
    assert!(
        !game
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_BERSERK)
    );
    assert_eq!(game.player_regeneration_rate_percent(), 200);
    for attribute in [
        AttributeKind::Strength,
        AttributeKind::Dexterity,
        AttributeKind::Constitution,
    ] {
        assert!(game.player_sustains_attribute(attribute));
    }
    assert!(!game.player_sustains_attribute(AttributeKind::Wisdom));
    let maximum = game.effective_player_max_hp();
    game.player.hp = maximum - 40;
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(25);
    game.resolve_item_berserk_strength("demo.item.fury-draught", 1, 25, 25, &mut Vec::new());
    assert_eq!(game.player.hp, maximum - 10);
    assert_eq!(game.effective_player_max_hp(), maximum);
    assert_eq!(game.rng, expected_rng);
    assert!(
        !game
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_BERSERK)
    );
    for (level, speed, stun, reflect) in [
        (29, 112, false, false),
        (30, 113, false, false),
        (34, 113, false, false),
        (35, 113, true, false),
        (39, 113, true, false),
        (40, 114, true, true),
        (45, 115, true, true),
        (50, 116, true, true),
    ] {
        let experience = game
            .experience_required_for_level(level)
            .saturating_sub(game.progress.experience);
        game.apply_player_experience(experience, &mut Vec::new());
        assert_eq!(game.progress.level, level);
        assert!(game.resources.is_empty());
        assert!(game.player_has_status_kind(STATUS_BERSERK));
        assert_eq!(game.player_derived_stats().speed.value, speed);
        assert_eq!(game.player_status_immunities().contains(STATUS_STUN), stun);
        assert_eq!(game.player_reflects_bolts(), reflect);
        assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    }
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn forbidden_item_attempts_preserve_items_and_follow_source_energy() {
    for (kind, spends_turn) in [
        ("demo.item.appraisal-scroll", true),
        ("demo.item.identify-staff", false),
        ("demo.item.magic-missile-wand", false),
        ("demo.item.detection-rod", false),
        ("demo.item.capture-ball", true),
        ("demo.item.dr-jones-whip", true),
    ] {
        let mut game = berserker(1);
        give_inventory_item(&mut game, "test.use", kind);
        let before = game.items.last().unwrap().clone();
        assert_eq!(
            game.berserker_item_use_rejection_cost(&before),
            Some(if spends_turn { 100 } else { 0 })
        );
        assert!(!game.inventory_item_dto(&before).usable);
        assert_eq!(
            game.inventory_item_dto(&before)
                .use_unavailable_reason
                .as_deref(),
            Some("berserker")
        );
        let mut events = Vec::new();
        let rng = game.rng.clone();
        game.use_inventory_item(
            "test.use",
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::ItemUseUnavailable]
        ));
        assert_eq!(*game.items.last().unwrap(), before);
        assert_eq!(game.rng, rng);
        let turn = game.world_tick;
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.use".to_owned(),
                target: None,
            },
        );
        assert_eq!(game.world_tick > turn, spends_turn, "{kind}");
        let after = game
            .items
            .iter()
            .find(|item| item.id == "test.use")
            .unwrap();
        assert_eq!(after.quantity, before.quantity);
        assert_eq!(after.charges, before.charges);
    }
    let mut game = berserker(1);
    give_inventory_item(&mut game, "test.recharge", "demo.item.recharging-scroll");
    give_inventory_item(&mut game, "test.device", "demo.item.magic-missile-wand");
    let charges = game.items.last().unwrap().charges;
    let tick = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::UseItemForRecharge {
            item_id: "test.recharge".to_owned(),
            source_item_id: "test.device".to_owned(),
            target_item_id: "test.device".to_owned(),
        },
    );
    assert!(game.world_tick > tick);
    assert_eq!(game.items.last().unwrap().charges, charges);
    assert!(game.items.iter().any(|item| item.id == "test.recharge"));
    let potion = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.healing-potion")
        .unwrap()
        .id
        .clone();
    game.player.hp = 1;
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: potion.clone(),
            target: None,
        },
    );
    assert!(game.player.hp > 1);
    assert!(!game.items.iter().any(|item| item.id == potion));
}

#[test]
fn strong_sensing_and_auto_identification_keep_their_separate_boundaries() {
    let mut game = berserker(1);
    give_inventory_item(&mut game, "test.axe", "demo.item.broad-axe");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
    game.items.last_mut().unwrap().curse = Some(ItemCurseSeverityDto::Heavy);
    let rng = game.rng.clone();
    game.apply_player_floor_item_knowledge();
    let axe = game
        .items
        .iter()
        .find(|item| item.id == "test.axe")
        .unwrap();
    assert_eq!(
        game.item_feeling(axe),
        Some(rfb_protocol::ItemFeelingDto::Bad)
    );
    assert_eq!(
        game.item_identification(axe),
        ItemIdentificationDto::Unexamined
    );
    assert_eq!(game.rng, rng);
    game.items.last_mut().unwrap().location = ItemLocation::Inventory;
    give_inventory_item(&mut game, "test.staff", "demo.item.identify-staff");
    give_inventory_item(&mut game, "test.scroll", "demo.item.appraisal-scroll");
    game.mark_item_aware("demo.item.identify-staff");
    game.mark_item_aware("demo.item.appraisal-scroll");
    assert!(
        game.configure_mogaminator(
            true,
            false,
            rfb_protocol::AutoGetModeDto::Off,
            LocaleDto::EnUs,
            "~?unidentified items".to_owned()
        )
        .is_empty()
    );
    let items = game.items.clone();
    assert!(
        game.apply_mogaminator_to_carried_items(vec!["test.axe".to_owned()])
            .unwrap()
            .is_empty()
    );
    assert_eq!(game.items, items);
    game.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(
        game.item_feeling(
            game.items
                .iter()
                .find(|item| item.id == "test.axe")
                .unwrap()
        ),
        Some(rfb_protocol::ItemFeelingDto::Bad)
    );
}

#[test]
fn forced_unequip_and_replacement_share_short_circuit_curse_rolls() {
    for severity in [
        ItemCurseSeverityDto::Normal,
        ItemCurseSeverityDto::Heavy,
        ItemCurseSeverityDto::Permanent,
    ] {
        for replacement in [false, true] {
            let mut template = berserker(1);
            let index = template
                .items
                .iter()
                .position(|item| item.kind_id == "demo.item.broad-axe")
                .unwrap();
            let ItemLocation::Equipped { slot_id } = template.items[index].location.clone() else {
                panic!();
            };
            template.items[index].curse = Some(severity);
            template.items[index]
                .intrinsic_curse_effects
                .insert(ItemCurseEffectDto::SlowRegeneration);
            give_inventory_item(&mut template, "test.replacement", "demo.item.broad-axe");
            for seed in 0..24 {
                let mut game = template.clone();
                game.rng = RfbRng::seeded(seed);
                let mut expected_rng = game.rng.clone();
                let succeeds = severity != ItemCurseSeverityDto::Permanent
                    && ((severity == ItemCurseSeverityDto::Heavy && expected_rng.bounded(7) == 0)
                        || expected_rng.bounded(4) == 0);
                let actual = if replacement {
                    game.equip_inventory_item("test.replacement", Some(&slot_id))
                        .is_some()
                } else {
                    game.unequip_slot(&slot_id).is_some()
                };
                assert_eq!(actual, succeeds, "{severity:?} {seed}");
                assert_eq!(game.rng, expected_rng);
                assert_eq!(
                    game.items[index].curse,
                    if succeeds { None } else { Some(severity) }
                );
                assert_eq!(
                    game.items[index].intrinsic_curse_effects.is_empty(),
                    succeeds
                );
                if succeeds {
                    assert_eq!(game.items[index].location, ItemLocation::Inventory);
                }
            }
        }
    }
}

#[test]
fn failed_curse_removal_spends_a_turn_but_full_inventory_preserves_the_roll() {
    let mut game = berserker(1);
    let index = game
        .items
        .iter()
        .position(|item| item.kind_id == "demo.item.broad-axe")
        .unwrap();
    let ItemLocation::Equipped { slot_id } = game.items[index].location.clone() else {
        panic!()
    };
    game.items[index].curse = Some(ItemCurseSeverityDto::Normal);
    game.rng = RfbRng::seeded(
        (0..100)
            .find(|seed| RfbRng::seeded(*seed).bounded(4) != 0)
            .unwrap(),
    );
    let tick = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: slot_id.clone(),
        },
    );
    assert!(game.world_tick > tick);
    assert_eq!(game.items[index].curse, Some(ItemCurseSeverityDto::Normal));
    while game.inventory_used_slots() < game.inventory_slot_capacity() {
        let id = format!("test.filler-{}", game.items.len());
        give_inventory_item(&mut game, &id, "demo.item.dagger");
    }
    let rng = game.rng.clone();
    let items = game.items.clone();
    assert!(game.unequip_slot(&slot_id).is_none());
    assert_eq!(game.rng, rng);
    assert_eq!(game.items, items);
}

#[test]
fn class_hp_cost_is_paid_after_healing_and_failure_can_leave_zero_hp() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    let mut ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.mindcrafter-clear-mind")
        .unwrap()
        .clone();
    ability.id = "test.ability.berserker-hp".to_owned();
    ability.effect = AbilityEffectDefinition::Heal { amount: 1000 };
    ability.tags.clear();
    artifact.content.abilities.push(ability.clone());
    let activation = rfb_content::ClassAbilityDefinition {
        ability_id: ability.id.clone(),
        blocked_by_dungeon_anti_magic: false,
        minimum_level: 1,
        ui_group_name_key: None,
        governing_attribute: Some(rfb_content::TechniqueAttribute::Strength),
        resource_id: None,
        resource_cost: 0,
        minimum_concentration: 0,
        hit_point_cost: 10,
        base_failure_percent: 95,
        minimum_failure_percent: 0,
    };
    artifact
        .content
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.berserker")
        .unwrap()
        .abilities
        .push(activation);
    let mut game = berserker(1);
    game.content = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    game.refresh_player_ability_state();
    let maximum = game.effective_player_max_hp();
    game.player.hp = maximum - 1;
    game.debug_ability_casts_succeed = true;
    let mut events = Vec::new();
    game.resolve_player_ability(
        &ability.id,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        game.player.hp,
        maximum - 10,
        "healing must run before the cost"
    );
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { resolution } if resolution.hp_paid == 10 && resolution.resource_paid == 0)));
    game.player.hp = 9;
    let rng = game.rng.clone();
    events.clear();
    game.resolve_player_ability(
        &ability.id,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.player.hp, 9);
    assert_eq!(game.rng, rng);
    game.debug_ability_casts_succeed = false;
    game.player.hp = 10;
    let failure =
        game.class_ability_failure_percent(game.class_ability_activation(&ability.id).unwrap());
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(failure))
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    events.clear();
    game.resolve_player_ability(
        &ability.id,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.player.hp, 0);
    assert!(!game.player_is_dead());
    assert!(
        matches!(events.as_slice(), [DomainEvent::AbilityCastFailed { resolution }] if resolution.hp_paid == 10)
    );
    let mut activation = game.class_ability_activation(&ability.id).unwrap().clone();
    activation.base_failure_percent = 30;
    let unstunned = game.class_ability_failure_percent(&activation);
    let mut stun = monster_combat::melee_status(STATUS_STUN, 20, "test.stun").status;
    stun.intensity = 20;
    game.player.statuses.push(stun);
    assert_eq!(
        game.class_ability_failure_percent(&activation),
        (unstunned + 10).min(95)
    );
    activation.base_failure_percent = 0;
    assert_eq!(game.class_ability_failure_percent(&activation), 0);
}

#[test]
fn mutations_guild_membership_and_draconian_scaling_use_class_identity() {
    let mut game = berserker(20);
    let (_, candidates) = game.pending_race_mutation_choice().unwrap();
    assert!(!candidates.iter().any(|id| matches!(
        id.as_str(),
        "rfb.mutation.astral-guide" | "rfb.mutation.fantastic-frenzy"
    )));
    assert!(candidates.contains(&"rfb.mutation.weapon-skills".to_owned()));
    for _ in 0..20 {
        let id = game.gain_random_mutation(&mut Vec::new()).unwrap();
        assert!(game.content.mutation(&id).unwrap().activation.is_none());
    }
    assert!(game.gain_mutation("rfb.mutation.hypn-gaze", &mut Vec::new()));
    let activation = game
        .content
        .mutation("rfb.mutation.hypn-gaze")
        .unwrap()
        .activation
        .as_ref()
        .unwrap();
    assert!(
        game.mutation_activation_for_ability(&activation.ability_id)
            .is_some()
    );
    for town in ["morivant", "angwil", "telmora", "anambar", "thalos"] {
        let facility = game
            .content
            .town_facility(&format!("demo.town-facility.{town}-warrior-guild"))
            .unwrap();
        assert_eq!(
            game.town_facility_membership(facility),
            rfb_protocol::FacilityMembershipDto::Owner
        );
    }
    let mut draconian =
        Game::new_with_build_race_and_name(923, BUILD, "rfb-legacy.race.draconian-red", "test")
            .unwrap();
    draconian.progress.level = 15;
    assert_eq!(draconian.draconian_metamorphosis_attack_level(), 52);
}

#[test]
fn permanent_fury_survives_dispel_and_clears_stun_at_the_level_threshold() {
    let mut game = berserker(34);
    game.apply_player_mental_status(STATUS_STUN, 30, "test");
    assert!(game.player_has_status_kind(STATUS_STUN));
    game.progress.level = 35;
    game.process_status_tick(
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        false,
    )
    .unwrap();
    assert!(!game.player_has_status_kind(STATUS_STUN));
    game.apply_player_mental_status(STATUS_STUN, 30, "test");
    assert!(!game.player_has_status_kind(STATUS_STUN));
    game.resolve_item_speed("demo.item.swiftstep-tonic", 0, 1, 10, &mut Vec::new());
    let dispel = game
        .content
        .ability("demo.ability.veil-dispel")
        .unwrap()
        .clone();
    game.resolve_monster_player_effects(
        "test.caster",
        "demo.actor.veil-warden",
        &dispel,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    assert!(!game.player_has_status_kind(STATUS_HASTE));
    assert!(game.player_has_status_kind(STATUS_BERSERK));
    let seed = (1..1000)
        .find(|seed| matches!(RfbRng::seeded(*seed).bounded(34) + 1, 13..=15 | 19 | 20))
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.resolve_equipped_ty_curse(
        "demo.item.broad-axe",
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(!game.player_has_status_kind(STATUS_PARALYSIS));
    assert!(game.player_has_status_kind(STATUS_BERSERK));
}

#[test]
fn weapon_bonuses_follow_hands_and_negative_blows_stop_at_zero() {
    let mut game = berserker(30);
    let two_hands = game.player_melee_profile(&game.player_derived_stats());
    let weapon_index = game
        .items
        .iter()
        .position(|item| item.kind_id == "demo.item.broad-axe")
        .unwrap();
    game.items[weapon_index].intrinsic_weight_tenths_pound = Some(10_000);
    let overweight = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(two_hands.to_damage - overweight.to_damage, 5);
    assert_eq!(
        (overweight.attacks, overweight.extra_attack_chance_percent),
        (2, 20)
    );
    game.items[weapon_index].intrinsic_weight_tenths_pound = None;
    give_inventory_item(&mut game, "test.shield", "demo.item.small-leather-shield");
    let shield_slot = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "shield")
        .unwrap()
        .id
        .clone();
    assert!(
        game.equip_inventory_item("test.shield", Some(&shield_slot))
            .is_some()
    );
    let one_hand = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(two_hands.to_damage - one_hand.to_damage, 5);
    assert_eq!(two_hands.to_hit - one_hand.to_hit, 6);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tonberry").status;
    form.granted_race_id = Some("rfb-legacy.race.tonberry".to_owned());
    game.player.statuses.push(form);
    let tonberry = game.player_melee_profile(&game.player_derived_stats());
    assert!(
        tonberry
            .attack_sources
            .iter()
            .any(|source| source.source_id == "rfb-legacy.race.tonberry" && source.amount == -120)
    );
    game.player.statuses.clear();
    let weapon = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.broad-axe")
        .unwrap();
    weapon
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks_delta_percent = -2000;
    let zero = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!((zero.attacks, zero.extra_attack_chance_percent), (0, 0));
}

#[test]
fn melee_can_pierce_invulnerability_and_refunds_unused_attacks() {
    let mut template = berserker(1);
    let position = Position {
        x: template.player.position.x + 1,
        y: template.player.position.y,
    };
    let mut target = template.generated_actor(
        "test.target".to_owned(),
        "demo.actor.small-kobold",
        position,
    );
    target.hp = 1000;
    target.max_hp = 1000;
    let mut shield = monster_combat::melee_status(STATUS_INVULNERABILITY, 50, "test.shield").status;
    shield.incoming_damage_percent = 0;
    target.statuses.push(shield);
    template.entities.push(target);
    let mut blocked = false;
    let mut pierced = false;
    for seed in 0..40 {
        let mut game = template.clone();
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        for event in events {
            if let DomainEvent::PlayerMeleeHit { damage, .. } = event {
                blocked |= damage.applied == 0;
                pierced |= damage.applied > 0;
            }
        }
        if blocked && pierced {
            break;
        }
    }
    assert!(blocked && pierced);
    template.entities[0].statuses.clear();
    template.entities[0].hp = 1;
    let (mut game, outcome) = (0..40)
        .find_map(|seed| {
            let mut game = template.clone();
            game.rng = RfbRng::seeded(seed);
            let outcome = game
                .resolve_player_melee(
                    0,
                    false,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            outcome.killed.then_some((game, outcome))
        })
        .unwrap();
    assert_eq!(
        outcome.energy_cost_on_kill,
        Some(i32::from(outcome.attacks_used) * 100 / i32::from(outcome.attacks_available))
    );
    let mut friend = game.generated_actor(
        "test.friend".to_owned(),
        "demo.actor.small-kobold",
        position,
    );
    friend.hp = 1;
    friend.controller_id = Some(game.player.id.clone());
    game.entities.push(friend);
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, template.player.position);
    assert!(
        game.entities
            .iter()
            .all(|entity| entity.id != "test.friend")
            || game.entities.last().unwrap().alerted
    );
}

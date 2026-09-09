// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::inventory::{ItemIdentificationRequest, RemoveEquippedCursesRequest};

fn game_with(effect: ItemCurseEffectDto) -> Game {
    let mut game = Game::new_with_build(82, "demo.build.warrior").unwrap();
    game.entities.clear();
    game.items.retain(|item| matches!(&item.location, ItemLocation::Equipped { slot_id } if game.body_slots.iter().any(|slot| slot.id == *slot_id && slot.slot_type == "weapon")));
    game.items[0].curse = Some(ItemCurseSeverityDto::Normal);
    game.items[0].affix_ids = vec!["rfb-legacy.affix.slaying".to_owned()];
    game.items[0].rolled_affixes = vec![RolledAffixState {
        affix_id: "rfb-legacy.affix.slaying".to_owned(),
        curse_effects: BTreeSet::from([effect]),
        ..Default::default()
    }];
    game.world_tick = 10;
    game
}

fn trigger_seed(odds: u64) -> u64 {
    (0..100_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(odds) == 0)
        .unwrap()
}

fn tick(game: &mut Game) {
    game.process_equipped_curse_effects(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
}

#[test]
fn every_random_curse_is_active_only_while_equipped_and_cursed_and_is_hidden_until_identified() {
    let mut game = game_with(ItemCurseEffectDto::LowDevice);
    game.items[0].rolled_affixes[0].curse_effects =
        ego::curses::CURSE_EFFECTS.into_iter().flatten().collect();
    let id = game.items[0].id.clone();
    game.item_property_knowledge.remove(&id);
    let details = game.character_trait_details(&game.player_derived_stats());
    assert!(details.negatives.iter().all(|entry| entry.source_id != id));
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    let details = game.character_trait_details(&game.player_derived_stats());
    assert_eq!(details.negatives[0].effects.len(), 26);
    for effect in ego::curses::CURSE_EFFECTS.into_iter().flatten() {
        assert!(game.player_has_equipped_curse_effect(effect), "{effect:?}");
    }
    let saved = game.to_save();
    let mut restored = Game::from_save(saved).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    restored.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
    for effect in ego::curses::CURSE_EFFECTS.into_iter().flatten() {
        assert!(
            !restored.player_has_equipped_curse_effect(effect),
            "{effect:?}"
        );
    }
    restored.rng = RfbRng::seeded(82);
    let before = restored.rng.clone();
    tick(&mut restored);
    assert_eq!(before, restored.rng);
}

#[test]
fn stat_penalties_stack_as_in_equip_and_permanent_does_not_imply_heavy() {
    let mut game = game_with(ItemCurseEffectDto::LowArmor);
    game.items[0].rolled_affixes[0].curse_effects.extend([
        ItemCurseEffectDto::LowMelee,
        ItemCurseEffectDto::LowMagic,
        ItemCurseEffectDto::LowDevice,
        ItemCurseEffectDto::SlowRegeneration,
        ItemCurseEffectDto::Catlike,
    ]);
    let mut clean = game.clone();
    clean.items[0].curse = None;
    let baseline = clean.player_derived_stats();
    for (severity, heavy) in [
        (ItemCurseSeverityDto::Normal, false),
        (ItemCurseSeverityDto::Heavy, true),
        (ItemCurseSeverityDto::Permanent, false),
    ] {
        game.items[0].curse = Some(severity);
        let stats = game.player_derived_stats();
        assert_eq!(
            stats.defense.value,
            (baseline.defense.value - if heavy { 30 } else { 10 }).max(0)
        );
        assert_eq!(
            stats.device_skill.value,
            (baseline.device_skill.value - if heavy { 10 } else { 5 }).max(0)
        );
        assert_eq!(
            stats.stealth_skill.value,
            (baseline.stealth_skill.value - 4).max(0)
        );
        assert_eq!(
            game.player_spell_failure_modifier_percent(),
            clean.player_spell_failure_modifier_percent() + if heavy { 10 } else { 3 }
        );
        assert_eq!(
            game.player_regeneration_rate_percent(),
            clean.player_regeneration_rate_percent() / 5
        );
        assert_eq!(
            game.player_melee_profile(&stats).to_hit,
            clean.player_melee_profile(&baseline).to_hit - if heavy { 15 } else { 5 }
        );
    }
    game.items[0].intrinsic_properties.rfb_heavy_curse = true;
    assert_eq!(
        game.equipped_curse_penalty(&game.items[0], ItemCurseEffectDto::LowDevice, 5, 10),
        10
    );
    let mut duplicate = game.items[0].clone();
    duplicate.id = "test.curse.second".to_owned();
    duplicate.location = ItemLocation::Equipped {
        slot_id: game
            .body_slots
            .iter()
            .find(|slot| slot.slot_type == "ring")
            .unwrap()
            .id
            .clone(),
    };
    duplicate.rolled_affixes[0].curse_effects =
        BTreeSet::from([ItemCurseEffectDto::LowDevice, ItemCurseEffectDto::LowMagic]);
    game.items.push(duplicate);
    assert_eq!(
        game.player_derived_stats().device_skill.value,
        (baseline.device_skill.value - 10).max(0)
    );
    assert_eq!(
        game.player_spell_failure_modifier_percent(),
        clean.player_spell_failure_modifier_percent() + 20
    );
}

#[test]
fn periodic_hp_mana_experience_and_device_drains_use_original_amounts_and_short_circuits() {
    for (effect, odds) in [
        (ItemCurseEffectDto::DrainHp, 666),
        (ItemCurseEffectDto::DrainMana, 666),
        (ItemCurseEffectDto::DrainExperience, 4),
    ] {
        let mut game = game_with(effect);
        game.player.hp = 100;
        game.progress.experience = 100;
        game.progress.maximum_experience = 150;
        game.rng = RfbRng::seeded(trigger_seed(odds));
        let level = game.progress.level;
        tick(&mut game);
        match effect {
            ItemCurseEffectDto::DrainHp => {
                assert_eq!(game.player.hp, 100 - i32::from(level) * 2);
                assert_eq!(game.rng_draw_counter(), 1);
            }
            ItemCurseEffectDto::DrainExperience => {
                assert_eq!(game.progress.experience, 100 - u64::from(level.div_ceil(2)));
                assert_eq!(
                    game.progress.maximum_experience,
                    150 - u64::from(level.div_ceil(2))
                );
                assert_eq!(game.rng_draw_counter(), 1);
            }
            _ => assert_eq!(
                game.rng_draw_counter(),
                0,
                "no mana pool skips the trigger draw"
            ),
        }
    }
    let mut mage = Game::new_with_build(82, "demo.build.high-mage-sorcery").unwrap();
    let mut cursed = game_with(ItemCurseEffectDto::DrainMana).items.remove(0);
    cursed.location = ItemLocation::Equipped {
        slot_id: mage.body_slots[0].id.clone(),
    };
    mage.items = vec![cursed];
    let id = mage.casting_profile().unwrap().resource_id.clone();
    mage.resources.get_mut(&id).unwrap().current = 100;
    mage.world_tick = 10;
    mage.rng = RfbRng::seeded(trigger_seed(666));
    tick(&mut mage);
    assert_eq!(
        mage.resources[&id].current,
        100 - u32::from(mage.progress.level)
    );
    assert_eq!(mage.rng_draw_counter(), 1);
}

#[test]
fn progressive_curses_and_normality_preserve_conditional_rng() {
    for (effect, power) in [
        (ItemCurseEffectDto::AddLightCurse, 0),
        (ItemCurseEffectDto::AddHeavyCurse, 1),
    ] {
        let mut game = game_with(effect);
        game.rng = RfbRng::seeded(trigger_seed(2000));
        let mut expected = game.rng.clone();
        expected.bounded(2000);
        let tval = game
            .content
            .item(&game.items[0].kind_id)
            .unwrap()
            .rfb_base_kind
            .unwrap()
            .tval;
        let added = ego::curses::get_curse(&mut expected, power, tval);
        tick(&mut game);
        assert!(
            game.items[0].rolled_affixes[0]
                .curse_effects
                .contains(&added)
        );
        // The newly added effect can run later in this same source-ordered cycle.
        assert!(game.rng_draw_counter() >= expected.draw_counter);
    }
    let mut game = game_with(ItemCurseEffectDto::Normality);
    game.player.statuses.clear();
    game.rng = RfbRng::seeded(trigger_seed(128));
    let mut expected = game.rng.clone();
    expected.bounded(128);
    let count = expected.bounded(20) + 1;
    for _ in 0..count * 200 {
        expected.bounded(33);
    }
    tick(&mut game);
    assert_eq!(
        game.rng, expected,
        "all 200 failed status attempts consume their draw"
    );
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_HASTE, 100, "test").status);
    game.rng = RfbRng::seeded(trigger_seed(128));
    tick(&mut game);
    assert!(!game.player_has_status_kind(STATUS_HASTE));
}

#[test]
fn summoning_bad_mutation_baby_curse_fear_and_allergy_have_actual_effects() {
    for (effect, odds, category) in [
        (ItemCurseEffectDto::CallAnimal, 2500, "animal"),
        (ItemCurseEffectDto::CallDemon, 1111, "demon"),
        (ItemCurseEffectDto::CallDragon, 800, "dragon"),
    ] {
        let mut game = game_with(effect);
        game.current_floor_id = game
            .content
            .world(&game.world_id)
            .unwrap()
            .procedural_floors
            .iter()
            .find(|floor| floor.depth == 50)
            .unwrap()
            .id
            .clone();
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.rng = RfbRng::seeded(trigger_seed(odds));
        tick(&mut game);
        assert!(!game.entities.is_empty(), "{effect:?}");
        assert!(
            game.entities
                .iter()
                .all(|actor| !actor.friendly && actor.controller_id.is_none())
        );
        assert!(
            game.content
                .actor(&game.entities[0].kind_id)
                .unwrap()
                .tags
                .iter()
                .any(|tag| tag == category)
        );
    }
    let mut game = game_with(ItemCurseEffectDto::CrappyMutation);
    game.progress.active_mutation_ids.clear();
    game.rng = RfbRng::seeded(trigger_seed(1500));
    tick(&mut game);
    assert_eq!(game.progress.active_mutation_ids.len(), 1);
    assert!(game.progress.active_mutation_ids.iter().all(|id| matches!(
        game.content.mutation(id).unwrap().rating,
        rfb_content::MutationRatingDefinition::Bad | rfb_content::MutationRatingDefinition::Awful
    )));
    let mut game = game_with(ItemCurseEffectDto::ByCurse);
    let before = game.progress.maximum_attributes;
    game.rng = RfbRng::seeded(trigger_seed(200));
    tick(&mut game);
    assert!(game.player_has_status_kind(STATUS_FEAR) && game.player_has_status_kind(STATUS_STUN));
    assert_ne!(game.progress.maximum_attributes, before);
    let mut game = game_with(ItemCurseEffectDto::Allergy);
    game.rng = RfbRng::seeded(trigger_seed(888));
    tick(&mut game);
    assert!(
        !game.player_has_status_kind(STATUS_UNWELL),
        "human has RACE_DEMI_TALENT and fails the literal source predicate"
    );
    assert_eq!(game.rng_draw_counter(), 1);
    let mut hobbit = Game::new_with_build_race_and_name(
        82,
        "demo.build.warrior",
        "rfb-legacy.race.hobbit",
        Game::DEFAULT_PLAYER_NAME,
    )
    .unwrap();
    hobbit.items = game.items.clone();
    hobbit.rng = RfbRng::seeded(trigger_seed(888));
    tick(&mut hobbit);
    assert_eq!(
        hobbit
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_UNWELL)
            .unwrap()
            .remaining_ticks,
        70
    );
    assert_eq!(hobbit.rng_draw_counter(), 1);
}

#[test]
fn digestion_wounds_device_drain_fear_and_danger_change_their_consumers() {
    let mut game = game_with(ItemCurseEffectDto::FastDigest);
    let mut clean = game.clone();
    clean.items[0].curse = None;
    game.world_tick = 50;
    clean.world_tick = 50;
    game.nutrition = 10_000;
    clean.nutrition = 10_000;
    game.process_hunger(&mut Vec::new());
    clean.process_hunger(&mut Vec::new());
    assert_eq!(game.nutrition, clean.nutrition - 30);

    let mut game = game_with(ItemCurseEffectDto::OpenWounds);
    game.player.hp = 100;
    game.player.statuses = vec![monster_combat::melee_status(STATUS_BLEEDING, 9, "test").status];
    let mut clean = game.clone();
    clean.items[0].curse = None;
    for tick in 0..3 {
        game.world_tick = tick;
        clean.world_tick = tick;
        game.process_status_tick(
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
            false,
        )
        .unwrap();
        clean
            .process_status_tick(
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
                false,
            )
            .unwrap();
    }
    assert_eq!(game.player.hp, clean.player.hp);
    assert_eq!(game.player.statuses[0].remaining_ticks, 8);
    assert_eq!(clean.player.statuses[0].remaining_ticks, 6);

    for (tag, drained) in [("staff", 100), ("wand", 100), ("rod", 33)] {
        let mut game = game_with(ItemCurseEffectDto::DrainPack);
        let kind = game
            .content
            .item_definitions()
            .find(|kind| kind.tags.iter().any(|entry| entry == tag))
            .unwrap()
            .id
            .clone();
        let (activation, charges) =
            initial_item_runtime_state(&game.content, &mut game.rng, &kind, &[], 50);
        let mut device = game.items[0].clone();
        device.id = "test.device".to_owned();
        device.kind_id = kind;
        device.affix_ids.clear();
        device.rolled_affixes.clear();
        device.curse = None;
        device.activation = activation;
        device.charges = charges;
        device.charges.as_mut().unwrap().current = 100;
        device.location = ItemLocation::Inventory;
        game.items.push(device);
        game.rng = RfbRng::seeded(trigger_seed(333));
        tick(&mut game);
        assert_eq!(game.items[1].charges.unwrap().current, 100 - drained);
        assert_eq!(
            game.rng_draw_counter(),
            1,
            "one inventory entry and one source do not consume selection RNG"
        );
        game.items[1].charges.as_mut().unwrap().current = 100;
        game.items[1]
            .intrinsic_properties
            .passives
            .insert(EquipmentPassive::HoldLife);
        game.rng = RfbRng::seeded(trigger_seed(333));
        tick(&mut game);
        assert_eq!(game.items[1].charges.unwrap().current, 100);
        assert_eq!(game.rng_draw_counter(), 1);
    }

    let mut game = game_with(ItemCurseEffectDto::Danger);
    let mut clean = game.clone();
    clean.items[0].curse = None;
    game.rng = RfbRng::seeded(82);
    clean.rng = game.rng.clone();
    let cursed = game.original_allocation_level(20);
    let ordinary = clean.original_allocation_level(20);
    assert_eq!(cursed, ordinary + 3, "surface bonus is reduced by a third");
    assert_eq!(game.rng, clean.rng);

    let mut game = game_with(ItemCurseEffectDto::Cowardice);
    game.current_floor_id = game
        .content
        .world(&game.world_id)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.depth == 50)
        .unwrap()
        .id
        .clone();
    let fear_seed = (0..100_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(1500) == 0 && rng.bounded(52) > 40
        })
        .unwrap();
    game.rng = RfbRng::seeded(fear_seed);
    tick(&mut game);
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_FEAR)
            .unwrap()
            .remaining_ticks,
        50
    );
}

// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters};
use super::*;
use crate::game::inventory::ItemIdentificationRequest;
use rfb_protocol::WeaponTraitDto;

const AMUN: &str = "demo.item.amun";

fn artifact_context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-60".into(),
        depth: 60,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    }
}

#[test]
fn pyramidal_mound_amun_uses_normal_artifact_rarity_and_unique_registration() {
    let mut game = Game::new_with_build(67, "demo.build.warrior").unwrap();
    let context = artifact_context();
    // Exercise the normal fixed-artifact selector after an ordinary dagger base
    // was selected. No Amun death reward or guaranteed-artifact table is used.
    for (roll, expected) in [(1, None), (119, None), (0, Some(AMUN))] {
        let seed = (0..10_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(120) == roll)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        assert_eq!(
            game.roll_fixed_artifact_kind_id(&context, Some("demo.item.dagger"), false)
                .as_deref(),
            expected
        );
        assert_eq!(game.rng.draw_counter, 1);
    }
    let draft = game.fixed_item_draft(&context, AMUN.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert!(item.curse.is_none());
    game.items.push(item);
    assert!(game.generated_artifact_ids.contains(AMUN));
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let rng = restored.rng.clone();
    assert!(
        restored
            .roll_fixed_artifact_kind_id(&context, Some("demo.item.dagger"), false)
            .is_none()
    );
    assert_eq!(restored.rng, rng);
}

#[test]
fn pyramidal_mound_amun_equips_senses_expires_and_recovers_across_save() {
    let mut game = Game::new_with_build(67, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    let draft = game.fixed_item_draft(&artifact_context(), AMUN.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    let before = game.player_derived_stats();
    let before_intelligence = game.equipment_modifiers().intelligence;
    let before_see_invisible = game.player_see_invisible_sources();
    game.equip_inventory_item(&id, None).unwrap();
    let after = game.player_derived_stats();
    assert_eq!(
        game.equipment_modifiers().intelligence,
        before_intelligence + 5
    );
    assert_eq!(after.speed.value, before.speed.value + 5);
    assert_eq!(after.stealth_skill.value, before.stealth_skill.value + 5);
    assert_eq!(after.search_skill.value, before.search_skill.value + 25);
    assert_eq!(
        after.perception_skill.value,
        before.perception_skill.value + 25
    );
    assert_eq!(
        game.player_see_invisible_sources(),
        before_see_invisible + 1
    );
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Immune
    );
    for damage in [
        DamageType::Poison,
        DamageType::Light,
        DamageType::Electricity,
        DamageType::Sound,
    ] {
        assert_eq!(
            game.effective_player_resistances().level(damage),
            ResistanceLevel::Resistant
        );
    }
    let weapon = &game.items[0];
    let melee = game.item_melee_profile(weapon).unwrap();
    assert_eq!((melee.damage.dice, melee.damage.sides), (3, 5));
    assert!(game.item_has_weapon_trait(weapon, WeaponTraitDto::Blessed));
    // Fixed definitions use the vorpal flag; item_has_weapon_trait reads
    // rolled/intrinsic traits instead (covered by the existing melee tests).
    assert!(game.content.item(&weapon.kind_id).unwrap().vorpal);
    assert_eq!(
        weapon.activation.as_ref().unwrap().device_check_difficulty,
        60
    );
    assert!(!game.player_has_telepathy());
    // A prepared ordinary monster exercises the actual sensing consumer.
    game.push_generated_actor(
        "test.esp-target".into(),
        "demo.actor.goblin",
        game.player.position,
    );
    assert!(!game.entity_is_visible_by_telepathy(&game.entities[0]));
    let mut boosted = game.clone();
    let mut power = monster_combat::melee_status(STATUS_HASTE, 2_000, "test.device-power").status;
    power.granted_modifiers.device_power_bonus = 5;
    boosted.player.statuses.push(power);
    let mut events = Vec::new();
    for current in [&mut game, &mut boosted] {
        for _ in 0..100 {
            current
                .use_inventory_item(
                    &id,
                    Some(&TargetSelection::SelfTarget),
                    None,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            if current.items[0].charges.unwrap().current == 0 {
                break;
            }
        }
        assert_eq!(current.items[0].charges.unwrap().current, 0);
    }
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    assert!(game.entity_is_visible_by_telepathy(&game.entities[0]));
    let duration = game
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_TELEPATHY)
        .unwrap()
        .remaining_ticks;
    assert!(
        (270..=560).contains(&duration) && duration % 10 == 0,
        "source 1d30+25 turns plus the item's immediate action window: {duration}"
    );
    let boosted_duration = boosted
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_TELEPATHY)
        .unwrap()
        .remaining_ticks;
    let source_turns = duration / 10 - 1;
    assert_eq!(
        boosted_duration,
        (source_turns + source_turns * 5 / 20 + 1) * 10
    );
    assert_eq!(
        boosted.rng, game.rng,
        "device power must not change the duration roll"
    );
    // Remove the prepared overlapping target before testing a valid save.
    clear_monsters(&mut game);
    let state = game.to_save();
    game.use_inventory_item(
        &id,
        Some(&TargetSelection::SelfTarget),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        game.to_save(),
        state,
        "cooldown cannot consume RNG or reapply ESP"
    );
    for tick in 1..=100 {
        game.world_tick = tick;
        game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut Vec::new(), false)
            .unwrap();
        game.process_inventory_device_recovery(&mut events);
    }
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(restored.player_has_telepathy());
    assert_eq!(restored.items[0].device_recovery_progress, 100);
    for tick in 101..=1000 {
        for current in [&mut game, &mut restored] {
            current.world_tick = tick;
            current
                .process_status_tick(
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                    false,
                )
                .unwrap();
            current.process_inventory_device_recovery(&mut Vec::new());
            if tick == duration {
                assert!(!current.player_has_telepathy());
            }
            if tick == 999 {
                assert_eq!(current.items[0].charges.unwrap().current, 0);
                assert_eq!(current.items[0].device_recovery_progress, 999);
            }
        }
    }
    assert_eq!(restored.items[0].charges.unwrap().current, 1);
    assert_eq!(restored.items[0].device_recovery_progress, 0);
    assert_eq!(restored.to_save(), game.to_save());
    assert_eq!(restored.rng, game.rng);
    for current in [&mut game, &mut restored] {
        current
            .use_inventory_item(
                &id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
    }
    assert_eq!(restored.to_save(), game.to_save());
    let slot = match &restored.items[0].location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => panic!("Amun must remain equipped"),
    };
    restored.unequip_slot(&slot).unwrap();
    assert_eq!(
        restored.equipment_modifiers().intelligence,
        before_intelligence
    );
}

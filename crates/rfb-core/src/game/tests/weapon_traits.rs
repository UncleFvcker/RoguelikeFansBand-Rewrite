// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{MeleeDamageDiceDto, WeaponTraitDto};

use super::{support::clear_monsters, *};
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

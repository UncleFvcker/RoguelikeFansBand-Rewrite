// SPDX-License-Identifier: MPL-2.0
use super::learning::{prepared, refill};
use super::*;
use crate::game::tests::support::{dispatch_next, replace_terrain};
use rfb_content::{SlayLevel, SlayTarget};
use rfb_protocol::{AbilityDto, ItemCurseSeverityDto, WeaponTraitDto};

pub(super) const BLESS: &str = "demo.ability.priest-bless-weapon";
pub(super) const EVOCATION: &str = "demo.ability.priest-evocation";
pub(super) const EVIL: &str = "demo.build.priest-death-sorcery";

pub(super) fn cast(game: &mut Game, ability: &str, target: TargetSelection) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        ability,
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn power(game: &Game, ability: &str) -> AbilityDto {
    game.snapshot()
        .player
        .abilities
        .into_iter()
        .find(|entry| entry.id == ability)
        .unwrap()
}

fn item_target(id: &str) -> TargetSelection {
    TargetSelection::Item {
        item_id: id.to_owned(),
    }
}

pub(super) fn weapon(game: &mut Game, kind: &str) -> String {
    let id = format!("test.priest.weapon.{}", game.items.len());
    give_inventory_item(game, &id, kind);
    id
}

fn index(game: &Game, id: &str) -> usize {
    game.items.iter().position(|item| item.id == id).unwrap()
}

#[test]
fn powers_follow_primary_realm_level_and_wisdom_without_learning_progress() {
    for (primary, ability, wrong, level) in [
        ("life", BLESS, EVOCATION, 35),
        ("crusade", BLESS, EVOCATION, 35),
        ("death", EVOCATION, BLESS, 42),
        ("daemon", EVOCATION, BLESS, 42),
    ] {
        let mut game = prepared(&format!("demo.build.priest-{primary}-sorcery"), level - 1);
        let id = weapon(&mut game, "demo.item.dagger");
        let target = if ability == BLESS {
            item_target(&id)
        } else {
            TargetSelection::SelfTarget
        };
        assert_eq!(
            power(&game, ability).unavailable_reason.as_deref(),
            Some("level-too-low")
        );
        assert!(
            !game
                .snapshot()
                .player
                .abilities
                .iter()
                .any(|entry| entry.id == wrong)
        );
        let before = game.to_save();
        assert!(matches!(&cast(&mut game, wrong, target.clone())[0],
            DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "realm-unavailable"));
        assert_eq!(game.to_save(), before);
        assert!(matches!(&cast(&mut game, ability, target.clone())[0],
            DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "level-too-low"));
        assert_eq!(game.to_save(), before);
        game.apply_player_experience(
            game.experience_required_for_level(level) - game.progress.experience,
            &mut Vec::new(),
        );
        refill(&mut game);
        assert!(power(&game, ability).can_cast);
        let before_failure = power(&game, ability).failure_percent;
        game.progress.attributes.intelligence = 3;
        assert_eq!(power(&game, ability).failure_percent, before_failure);
        game.progress.attributes.wisdom = 3;
        assert!(power(&game, ability).failure_percent >= before_failure);
        let learned = game.learned_abilities.clone();
        game.debug_set_ability_casts_succeed(true);
        cast(&mut game, ability, target);
        assert_eq!(game.learned_abilities, learned);
        assert!(!game.ability_progress.contains_key(ability));
    }
}

#[test]
fn both_powers_pay_mp_then_hp_on_success_and_failure_and_reject_shortfall_atomically() {
    for (build, ability, cost) in [(BUILD, BLESS, 70_u32), (EVIL, EVOCATION, 40)] {
        for (mana, succeeds) in [(100, true), (7, true), (0, true), (7, false)] {
            let mut game = prepared(build, 50);
            game.progress.attributes.wisdom = 18;
            game.progress.maximum_attributes.wisdom = 18;
            game.refresh_player_ability_state();
            let id = weapon(&mut game, "demo.item.dagger");
            let target = if ability == BLESS {
                item_target(&id)
            } else {
                TargetSelection::SelfTarget
            };
            game.resources.get_mut(MANA).unwrap().current = mana;
            let failure = power(&game, ability).failure_percent;
            assert!((1..=95).contains(&failure));
            game.rng = (0..1000)
                .map(RfbRng::seeded)
                .find(|rng| {
                    let mut rng = rng.clone();
                    (rng.bounded(100) >= u64::from(failure)) == succeeds
                })
                .unwrap();
            let hp = game.player.hp;
            let mut restored = Game::from_save(game.to_save()).unwrap();
            let events = cast(&mut game, ability, target.clone());
            assert_eq!(events, cast(&mut restored, ability, target));
            assert_eq!(game.to_save(), restored.to_save());
            let resolution = events
                .iter()
                .find_map(|event| match event {
                    DomainEvent::AbilityCastSucceeded { resolution }
                    | DomainEvent::AbilityCastFailed { resolution } => Some(resolution),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                (
                    resolution.succeeded,
                    resolution.resource_cost,
                    resolution.resource_paid,
                    resolution.hp_paid
                ),
                (succeeds, cost, mana.min(cost), cost.saturating_sub(mana))
            );
            assert_eq!(game.resources[MANA].current, mana.saturating_sub(cost));
            assert_eq!(game.player.hp, hp - cost.saturating_sub(mana) as i32);
            assert_eq!(
                game.item_has_weapon_trait(&game.items[index(&game, &id)], WeaponTraitDto::Blessed),
                ability == BLESS && succeeds
            );
        }
        let mut poor = prepared(build, 50);
        let id = weapon(&mut poor, "demo.item.dagger");
        poor.resources.get_mut(MANA).unwrap().current = 7;
        poor.player.hp = (cost - 8) as i32;
        assert!(!power(&poor, ability).can_cast);
        let before = poor.to_save();
        let target = if ability == BLESS {
            item_target(&id)
        } else {
            TargetSelection::SelfTarget
        };
        assert!(matches!(&cast(&mut poor, ability, target)[0],
            DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "insufficient-resource"));
        assert_eq!(poor.to_save(), before);
    }
}

#[test]
fn blessing_targets_cancel_freely_and_preserve_partial_knowledge_through_save_and_mundanity() {
    let mut game = prepared(BUILD, 50);
    game.debug_set_ability_casts_succeed(true);
    let id = weapon(&mut game, "demo.item.dagger");
    let bow = weapon(&mut game, "demo.item.short-bow");
    let ammo = weapon(&mut game, "demo.item.arrow");
    let ability = game.content.ability(BLESS).unwrap();
    let targets = game.craft_ability_item_targets(ability).unwrap();
    assert!(targets.iter().any(|target| target.item_id == bow));
    assert!(!targets.iter().any(|target| target.item_id == ammo));
    let before = game.to_save();
    cast(&mut game, BLESS, TargetSelection::SelfTarget);
    assert_eq!(game.to_save(), before);
    let i = index(&game, &id);
    game.items[i].location = ItemLocation::Ground(Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    });
    let before = game.to_save();
    cast(&mut game, BLESS, item_target(&id));
    assert_eq!(game.to_save(), before);
    game.items[i].location = ItemLocation::Ground(game.player.position);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    restored.debug_set_ability_casts_succeed(true);
    assert_eq!(
        cast(&mut game, BLESS, item_target(&id)),
        cast(&mut restored, BLESS, item_target(&id))
    );
    assert_eq!(game.to_save(), restored.to_save());
    let item = &game.items[index(&game, &id)];
    assert_eq!(item.discount_percent, 99);
    assert!(game.known_item_blessed(item));
    assert_ne!(
        game.item_identification(item),
        ItemIdentificationDto::Identified
    );
    let mut forged = game.to_save();
    forged
        .item_property_knowledge
        .iter_mut()
        .find(|entry| entry.item_id == ammo)
        .unwrap()
        .known_blessed = true;
    // A real unblessed inventory item always has a persisted knowledge entry.
    assert!(Game::from_save(forged).is_err());
    game.mundanify_item(&id);
    assert!(!game.known_item_blessed(&game.items[index(&game, &id)]));
    Game::from_save(game.to_save()).unwrap();
}

#[test]
fn blessing_obeys_curse_threshold_then_slay_resistance_and_ordered_disenchantment() {
    for (severity, roll, succeeds) in [
        (ItemCurseSeverityDto::Normal, 0, true),
        (ItemCurseSeverityDto::Heavy, 31, false),
        (ItemCurseSeverityDto::Heavy, 32, true),
        (ItemCurseSeverityDto::Permanent, 99, false),
    ] {
        let mut game = prepared(BUILD, 50);
        game.debug_set_ability_casts_succeed(true);
        let id = weapon(&mut game, "demo.item.dagger");
        let i = index(&game, &id);
        game.items[i].curse = Some(severity);
        game.rng = (0..10000)
            .map(RfbRng::seeded)
            .find(|rng| {
                let mut rng = rng.clone();
                rng.bounded(100);
                rng.bounded(100) == roll
            })
            .unwrap();
        cast(&mut game, BLESS, item_target(&id));
        assert_eq!(game.items[i].curse.is_none(), succeeds);
        assert_eq!(
            game.items[i]
                .intrinsic_weapon_traits
                .contains(&WeaponTraitDto::Blessed),
            succeeds
        );
        Game::from_save(game.to_save()).unwrap();
    }
    for (level, odds) in [(SlayLevel::Slay, 3_u64), (SlayLevel::Kill, 5)] {
        let mut game = prepared(BUILD, 50);
        game.debug_set_ability_casts_succeed(true);
        let id = weapon(&mut game, "demo.item.dagger");
        let i = index(&game, &id);
        game.items[i]
            .intrinsic_properties
            .slays
            .insert(SlayTarget::Good, level);
        game.items[i].enchantments = ItemEnchantmentsDto {
            to_hit: 9,
            to_damage: 9,
            to_armor: 9,
        };
        game.rng = (0..1000)
            .map(RfbRng::seeded)
            .find(|rng| {
                let mut rng = rng.clone();
                rng.bounded(100);
                rng.bounded(odds) != 0
            })
            .unwrap();
        let mut expected = game.rng.clone();
        expected.bounded(100);
        expected.bounded(odds);
        let values = [0; 3].map(|_| 8 - i16::from(expected.bounded(100) < 33));
        cast(&mut game, BLESS, item_target(&id));
        assert_eq!(
            game.items[i].enchantments,
            ItemEnchantmentsDto {
                to_hit: values[0],
                to_damage: values[1],
                to_armor: values[2]
            }
        );
        assert_eq!(game.rng, expected);
        assert!(!game.known_item_blessed(&game.items[i]));
        game.items[i]
            .intrinsic_properties
            .slays
            .insert(SlayTarget::Evil, SlayLevel::Slay);
        refill(&mut game);
        let mut expected = game.rng.clone();
        expected.bounded(100);
        cast(&mut game, BLESS, item_target(&id));
        assert!(game.known_item_blessed(&game.items[i]));
        assert_eq!(game.rng, expected);
        Game::from_save(game.to_save()).unwrap();
    }
}

#[test]
fn edged_weapon_penalties_are_per_hand_and_blessing_does_not_raise_proficiency_caps() {
    for (build, cap) in [(BUILD, 4000), (EVIL, 6000)] {
        let mut game = prepared(build, 35);
        game.progress.attributes.wisdom = 18;
        game.progress.maximum_attributes.wisdom = 18;
        for item in &mut game.items {
            if matches!(item.location, ItemLocation::Equipped { ref slot_id } if slot_id == "weapon" || slot_id == "shield")
            {
                item.location = ItemLocation::Inventory;
            }
        }
        let a = weapon(&mut game, "demo.item.dagger");
        let b = weapon(&mut game, "demo.item.sabre");
        for (id, slot) in [(&a, "weapon"), (&b, "shield")] {
            let i = index(&game, id);
            game.items[i].location = ItemLocation::Equipped {
                slot_id: slot.to_owned(),
            };
        }
        let mut blessed = game.clone();
        for id in [&a, &b] {
            let i = index(&blessed, id);
            blessed.items[i]
                .intrinsic_weapon_traits
                .insert(WeaponTraitDto::Blessed);
        }
        let raw = game.player_melee_profile(&game.player_derived_stats());
        let clean = blessed.player_melee_profile(&blessed.player_derived_stats());
        let penalty = if build == BUILD { 2 } else { 0 };
        assert_eq!(raw.to_hit, clean.to_hit - penalty);
        assert_eq!(raw.to_damage, clean.to_damage - penalty);
        assert_eq!(raw.melee_skill.value, clean.melee_skill.value - penalty);
        let prayer = |g: &Game| {
            let mut g = g.clone();
            g.progress.level = 1;
            let profile = g.casting_profile().unwrap();
            let ability = g.effective_casting_ability(
                profile,
                g.content.ability(super::learning::SORCERY).unwrap(),
            );
            g.ability_failure_percent(profile, &ability)
        };
        assert!(prayer(&game) >= prayer(&blessed));
        if build == BUILD {
            assert_eq!(prayer(&game) - prayer(&blessed), 50);
        }
        for g in [&game, &blessed] {
            assert_eq!(
                g.player_weapon_proficiencies()
                    .into_iter()
                    .find(|p| p.item_kind_id == "demo.item.sabre")
                    .unwrap()
                    .maximum,
                cap
            );
        }
        assert_eq!(game.player_melee_damage_percent(), 94);
        assert_eq!(game.scale_player_melee_damage(25), 24);
        let mut tonberry =
            Game::new_with_build_race_and_name(925, build, "rfb-legacy.race.tonberry", "Priest")
                .unwrap();
        assert_eq!(
            tonberry
                .player_weapon_proficiencies()
                .into_iter()
                .find(|p| p.item_kind_id == "demo.item.sabre")
                .unwrap()
                .maximum,
            cap
        );
        assert!(tonberry.gain_mutation("rfb.mutation.weapon-skills", &mut Vec::new()));
        assert_eq!(
            tonberry
                .player_weapon_proficiencies()
                .into_iter()
                .find(|p| p.item_kind_id == "demo.item.sabre")
                .unwrap()
                .maximum,
            8000
        );
    }
}

#[test]
fn evocation_damages_all_alignments_then_fears_and_teleports_only_survivors() {
    let mut game = prepared(EVIL, 50);
    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=12 {
        for x in 8..=25 {
            let position = Position { x, y };
            replace_terrain(&mut game, position, "demo.terrain.floor");
            let i = game.index(position).unwrap();
            game.glow[i] = true;
        }
    }
    for (id, kind, position, hp) in [
        (
            "test.dead",
            "demo.actor.sheep",
            Position { x: 11, y: 10 },
            1,
        ),
        (
            "test.alive",
            "demo.actor.orc-captain",
            Position { x: 12, y: 10 },
            10000,
        ),
    ] {
        game.push_generated_actor(id.to_owned(), kind, position);
        let entity = game
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .unwrap();
        entity.hp = hp;
        entity.max_hp = hp;
    }
    game.reveal_current_visibility();
    assert!(
        game.entities
            .iter()
            .all(|entity| game.entity_is_visible_to_player(entity))
    );
    let before = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.alive")
        .unwrap()
        .position;
    let expected_damage = 200 + i32::from(game.casting_spell_damage_bonus());
    let mut restored = Game::from_save(game.to_save()).unwrap();
    restored.debug_set_ability_casts_succeed(true);
    let events = cast(&mut game, EVOCATION, TargetSelection::SelfTarget);
    assert_eq!(
        events,
        cast(&mut restored, EVOCATION, TargetSelection::SelfTarget)
    );
    assert_eq!(game.to_save(), restored.to_save());
    assert!(!game.entities.iter().any(|entity| entity.id == "test.dead"));
    let alive = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.alive")
        .unwrap();
    assert_eq!(alive.hp, 10000 - expected_damage);
    assert_ne!(alive.position, before);
    let effects: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::AbilityEffectsResolved { resolution, .. } => Some(resolution),
            _ => None,
        })
        .collect();
    assert!(effects.iter().any(|resolution| resolution.target_entity_id.as_deref() == Some("test.alive")
        && resolution.effects.iter().any(|effect| matches!(effect, AbilityEffectResolutionDto::ApplyStatus { status_kind_id, power: Some(200), .. } if status_kind_id == STATUS_FEAR))));
    assert!(
        !effects
            .iter()
            .any(|resolution| resolution.target_entity_id.as_deref() == Some("test.dead"))
    );
    assert!(effects.iter().any(|resolution| resolution.effects.iter().any(|effect|
        matches!(effect, AbilityEffectResolutionDto::TeleportAway { target_entity_id, power: 200, to: Some(_), .. } if target_entity_id == "test.alive"))));
    Game::from_save(game.to_save()).unwrap();
}

#[test]
fn evocation_hits_unseen_in_sight_respects_unique_teleport_resistance_and_walls() {
    let mut game = prepared(EVIL, 50);
    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 10, y: 10 };
    for x in 10..=14 {
        replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
    }
    replace_terrain(&mut game, Position { x: 13, y: 10 }, "demo.terrain.wall");
    game.push_generated_actor(
        "test.invisible".to_owned(),
        "demo.actor.a-plain-gold-ring",
        Position { x: 11, y: 10 },
    );
    game.push_generated_actor(
        "test.blocked".to_owned(),
        "demo.actor.sheep",
        Position { x: 14, y: 10 },
    );
    game.reveal_current_visibility();
    assert!(!game.entity_is_visible_to_player(&game.entities[0]));
    let before = game.entities.clone();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    restored.debug_set_ability_casts_succeed(true);
    let events = cast(&mut game, EVOCATION, TargetSelection::SelfTarget);
    assert_eq!(
        events,
        cast(&mut restored, EVOCATION, TargetSelection::SelfTarget)
    );
    assert_eq!(game.to_save(), restored.to_save());
    assert!(game.entities[0].hp < before[0].hp);
    assert_eq!(game.entities[0].position, before[0].position);
    assert_eq!(game.entities[1], before[1]);
    assert!(events.iter().any(|event| matches!(event,
        DomainEvent::AbilityEffectsResolved { resolution, .. } if resolution.effects.iter().any(|effect|
            matches!(effect, AbilityEffectResolutionDto::TeleportAway { target_entity_id, resisted: true, to: None, .. } if target_entity_id == "test.invisible")))));
    game.entities.clear();
    refill(&mut game);
    let mana = game.resources[MANA].current;
    cast(&mut game, EVOCATION, TargetSelection::SelfTarget);
    assert_eq!(game.resources[MANA].current, mana - 40);
}

#[test]
fn successful_blessing_dispatch_spends_a_turn_and_continues_after_loading() {
    let mut game = prepared(BUILD, 50);
    game.debug_set_ability_casts_succeed(true);
    let id = weapon(&mut game, "demo.item.dagger");
    let before_tick = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: BLESS.to_owned(),
            target: item_target(&id),
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.cast-success")
    );
    assert!(game.world_tick > before_tick);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        dispatch_next(&mut game, GameCommand::Wait).events,
        dispatch_next(&mut restored, GameCommand::Wait).events
    );
    assert_eq!(game.to_save(), restored.to_save());
}

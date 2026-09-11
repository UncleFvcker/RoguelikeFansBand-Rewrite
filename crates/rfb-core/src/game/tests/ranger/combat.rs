// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::{dispatch_next, replace_terrain};
use rfb_protocol::{AbilityDto, AttributeKindDto};

const PROBE: &str = "demo.ability.ranger-probe-monsters";

fn equip(game: &mut Game, kind: &str, slot: &str) {
    for item in &mut game.items {
        if matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == slot) {
            item.location = ItemLocation::Inventory;
        }
    }
    let id = format!("test.ranger.{slot}");
    game.items.retain(|item| item.id != id);
    give_inventory_item(game, &id, kind);
    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .location = ItemLocation::Equipped {
        slot_id: slot.to_owned(),
    };
}

fn room(game: &mut Game) {
    game.player.position = Position { x: 10, y: 10 };
    for y in 9..=11 {
        for x in 9..=25 {
            let position = Position { x, y };
            replace_terrain(game, position, "demo.terrain.floor");
            let index = game.index(position).unwrap();
            game.glow[index] = true;
        }
    }
}

fn cast(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        PROBE,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn power(game: &Game) -> AbilityDto {
    game.snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == PROBE)
        .unwrap()
}

#[test]
fn every_launcher_adds_level_skill_but_only_arrows_keep_skill_shot_rate() {
    for level in [1, 25, 50] {
        for (kind, ammo, arrow) in [
            ("demo.item.short-bow", "demo.item.arrow", true),
            ("demo.item.sling", "demo.item.iron-shot", false),
            ("demo.item.light-crossbow", "demo.item.bolt", false),
            ("demo.item.heavy-crossbow", "demo.item.bolt", false),
        ] {
            let mut game = at_level(BUILD, level);
            game.progress.attributes.strength = 118;
            room(&mut game);
            equip(&mut game, kind, "shooting");
            give_inventory_item(&mut game, "test.ammo", ammo);
            let skill = game.player_derived_stats().ranged_skill.value;
            let mut unequipped = game.clone();
            unequipped
                .items
                .iter_mut()
                .find(|item| item.id == "test.ranger.shooting")
                .unwrap()
                .location = ItemLocation::Inventory;
            assert_eq!(
                skill - unequipped.player_derived_stats().ranged_skill.value,
                20 + i32::from(level)
            );
            assert!(unequipped.player_projectile_profile().is_none());
            let profile = game.player_projectile_profile().unwrap();
            assert_eq!(profile.base_shot, if arrow { skill.max(100) } else { 100 });
            let ammo_kind = game.content.item(ammo).unwrap();
            let factor = if skill > 80 {
                (90 - (skill - 80) / 2).max(0)
            } else {
                100
            };
            assert_eq!(
                i32::from(profile.ammo_break_chance_percent),
                i32::from(ammo_kind.break_chance_percent) * factor / 100
            );
            let gain = energy_gain(derived_speed(&game.player_derived_stats().speed));
            let ticks = game.world_tick;
            let quantity = game
                .items
                .iter()
                .find(|item| Some(&item.id) == profile.ammo_item_id.as_ref())
                .unwrap()
                .quantity;
            dispatch_next(
                &mut game,
                GameCommand::Fire {
                    direction: Direction::East,
                },
            );
            assert_eq!(
                game.world_tick - ticks,
                u32::try_from((profile.energy_cost + gain - 1) / gain).unwrap()
            );
            let after = game
                .items
                .iter()
                .find(|item| {
                    Some(&item.id) == profile.ammo_item_id.as_ref()
                        && item.location == ItemLocation::Inventory
                })
                .map_or(0, |item| item.quantity);
            assert_eq!(after, quantity - 1);
        }
    }
    let mut harp = at_level(BUILD, 25);
    equip(&mut harp, "demo.item.harp", "shooting");
    let skill = harp.player_derived_stats().ranged_skill.value;
    assert!(harp.player_projectile_profile().is_none());
    harp.items
        .iter_mut()
        .find(|item| item.id == "test.ranger.shooting")
        .unwrap()
        .location = ItemLocation::Inventory;
    assert_eq!(skill - harp.player_derived_stats().ranged_skill.value, 45);
}

#[test]
fn extra_shots_follow_non_arrow_reset_heavy_launcher_and_tomte_headgear() {
    let mut game = at_level(BUILD, 50);
    game.progress.attributes.strength = 118;
    equip(&mut game, "demo.item.sling", "shooting");
    let launcher = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.ranger.shooting")
        .unwrap();
    launcher
        .intrinsic_properties
        .equipment_bonuses
        .base_shot_delta_percent = 30;
    let armor = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.soft-leather-armour")
        .unwrap();
    armor
        .intrinsic_properties
        .equipment_bonuses
        .base_shot_delta_percent = 15;
    assert_eq!(game.player_projectile_profile().unwrap().base_shot, 145);
    // Mounted non-arrow restrictions precede equipment extra shots.
    game.push_generated_actor(
        "test.mount".to_owned(),
        "demo.actor.horse",
        game.player.position,
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.mount".to_owned());
    let mounted = game.player_projectile_profile().unwrap();
    assert_eq!(mounted.base_shot, 145);
    game.riding_actor_id = None;
    assert_ne!(
        mounted.to_hit,
        game.player_projectile_profile().unwrap().to_hit
    );
    game.entities.clear();
    game.progress.attributes.strength = 3;
    equip(&mut game, "demo.item.heavy-crossbow", "shooting");
    assert_eq!(game.player_projectile_profile().unwrap().base_shot, 100);
    let heavy_hit = game.player_projectile_profile().unwrap().to_hit;
    game.progress.attributes.strength = 118;
    assert!(heavy_hit < game.player_projectile_profile().unwrap().to_hit);
    equip(&mut game, "demo.item.short-bow", "shooting");
    let normal = game.player_projectile_profile().unwrap().base_shot;
    equip(&mut game, "demo.item.metal-scale-mail", "body");
    assert_eq!(
        game.player_projectile_profile().unwrap().base_shot,
        normal - 15
    );

    let mut tomte = Game::new_with_build_race_and_name(
        925,
        BUILD,
        "rfb-legacy.race.tomte",
        Game::DEFAULT_PLAYER_NAME,
    )
    .unwrap();
    clear_monsters(&mut tomte);
    tomte.apply_player_experience(tomte.experience_required_for_level(50), &mut Vec::new());
    tomte.progress.attributes.strength = 118;
    equip(&mut tomte, "demo.item.hard-leather-cap", "head");
    let head = tomte
        .items
        .iter_mut()
        .find(|item| item.id == "test.ranger.head")
        .unwrap();
    head.intrinsic_properties
        .equipment_bonuses
        .base_shot_delta_percent = 15;
    assert!(tomte.player_tomte_headgear_excess_weight() > 0);
    assert_eq!(tomte.player_projectile_profile().unwrap().base_shot, 115);
    tomte
        .items
        .iter_mut()
        .find(|item| item.id == "test.ranger.head")
        .unwrap()
        .location = ItemLocation::Inventory;
    assert!(tomte.player_projectile_profile().unwrap().base_shot > 115);
}

#[test]
fn probing_unlocks_at_fifteen_uses_wisdom_and_pays_mp_then_hp_even_on_failure() {
    let mut low = at_level(BUILD, 14);
    assert_eq!(
        power(&low).unavailable_reason.as_deref(),
        Some("level-too-low")
    );
    let before = low.rng.clone();
    assert!(
        matches!(&cast(&mut low)[0], DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "level-too-low")
    );
    assert_eq!(low.rng, before);
    for (mana, hp, succeed) in [
        (30, 40, true),
        (7, 40, true),
        (0, 40, true),
        (7, 40, false),
        (7, 13, true),
    ] {
        let mut game = at_level(BUILD, 15);
        room(&mut game);
        game.resources.get_mut(MANA).unwrap().current = mana;
        game.player.hp = hp;
        let projected = power(&game);
        assert_eq!(projected.source, AbilitySourceDto::Class);
        assert_eq!(
            projected.governing_attribute,
            Some(AttributeKindDto::Wisdom)
        );
        assert_eq!(
            (
                projected.base_resource_cost,
                projected.resource_cost,
                projected.hit_point_cost
            ),
            (20, mana.min(20), 20 - mana.min(20))
        );
        assert!(projected.can_cast);
        game.rng = (1..1000)
            .map(RfbRng::seeded)
            .find(|rng| {
                let mut rng = rng.clone();
                (rng.bounded(100) >= u64::from(projected.failure_percent)) == succeed
            })
            .unwrap();
        let events = cast(&mut game);
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
            (succeed, 20, mana.min(20), 20 - mana.min(20))
        );
        assert_eq!(game.resources[MANA].current, mana.saturating_sub(20));
        assert_eq!(
            game.player.hp,
            hp - i32::try_from(20 - mana.min(20)).unwrap()
        );
        assert_eq!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityMonstersProbed { .. })),
            succeed
        );
        assert!(game.probed_actor_kind_ids.is_empty());
    }
    let mut poor = at_level(BUILD, 15);
    poor.resources.get_mut(MANA).unwrap().current = 7;
    poor.player.hp = 12;
    assert!(!power(&poor).can_cast);
    let before = poor.state_hash();
    assert!(
        matches!(&cast(&mut poor)[0], DomainEvent::AbilityCastUnavailable { reason, .. } if reason == "insufficient-resource")
    );
    assert_eq!(poor.state_hash(), before);
    let failure = power(&poor).failure_percent;
    poor.progress.attributes.intelligence = 118;
    assert_eq!(power(&poor).failure_percent, failure);
    poor.progress.attributes.wisdom = 118;
    assert!(power(&poor).failure_percent < failure);
}

#[test]
fn probing_action_updates_lore_and_save_continuation_with_normal_energy() {
    let mut game = at_level(BUILD, 50);
    room(&mut game);
    game.progress.attributes.wisdom = game.progress.attribute_potentials.wisdom;
    game.progress.maximum_attributes.wisdom = game.progress.attributes.wisdom;
    game.refresh_player_ability_state();
    game.resources.get_mut(MANA).unwrap().current = 7;
    game.push_generated_actor(
        "test.probe".to_owned(),
        "demo.actor.sheep",
        Position { x: 12, y: 10 },
    );
    let failure = power(&game).failure_percent;
    game.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(100) >= u64::from(failure)
        })
        .unwrap();
    let gain = energy_gain(derived_speed(&game.player_derived_stats().speed));
    let ticks = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: PROBE.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(
        game.world_tick - ticks,
        u32::try_from((100 + gain - 1) / gain).unwrap()
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for game in [&mut game, &mut restored] {
        dispatch_next(
            game,
            GameCommand::CastAbility {
                ability_id: PROBE.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
    }
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn probing_filters_walls_fuzzy_sight_and_hallucination_and_reveals_disguise() {
    for mode in ["visible", "wall", "fuzzy", "hallucination"] {
        let mut game = at_level(BUILD, 15);
        room(&mut game);
        game.debug_ability_casts_succeed = true;
        game.resources.get_mut(MANA).unwrap().current = 0;
        game.push_generated_actor(
            "test.probe".to_owned(),
            "demo.actor.sheep",
            Position { x: 14, y: 10 },
        );
        game.entities[0].appearance_kind_id = Some("demo.actor.horse".to_owned());
        match mode {
            "wall" => replace_terrain(&mut game, Position { x: 12, y: 10 }, "demo.terrain.wall"),
            "fuzzy" => {
                assert!(game.gain_mutation("rfb.mutation.esp", &mut Vec::new()));
                game.entities[0].visible_weird_mind = true;
                game.player
                    .statuses
                    .push(monster_combat::melee_status(STATUS_BLINDNESS, 20, "test.blind").status);
                assert!(game.entity_is_fuzzy_to_player(&game.entities[0]));
            }
            "hallucination" => game
                .player
                .statuses
                .push(monster_combat::melee_status(STATUS_HALLUCINATION, 20, "test.image").status),
            _ => {}
        }
        let hp = game.player.hp;
        let events = cast(&mut game);
        let monsters = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityMonstersProbed { resolution, .. } => Some(&resolution.monsters),
                _ => None,
            })
            .unwrap();
        assert_eq!(monsters.len(), usize::from(mode == "visible"), "{mode}");
        assert_eq!(
            game.entities[0].appearance_kind_id.is_none(),
            mode == "visible",
            "{mode}"
        );
        assert_eq!(
            game.probed_actor_kind_ids.contains("demo.actor.sheep"),
            mode == "visible",
            "{mode}"
        );
        assert_eq!(game.player.hp, hp - 20, "empty probe still costs HP");
    }
}

#[test]
fn real_shots_use_ranger_skill_for_hit_and_ammunition_recovery() {
    use crate::game::player_combat::ProjectileMode;
    fn shoot(game: &mut Game) -> (bool, bool) {
        let mut events = Vec::new();
        game.resolve_player_projectile(
            TargetSelection::Direction {
                direction: Direction::East,
            },
            ProjectileMode::Normal,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        (
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::ProjectileHit { .. })),
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::ProjectileAmmoBroken { .. })),
        )
    }
    let mut base = at_level(BUILD, 50);
    room(&mut base);
    base.push_generated_actor(
        "test.target".to_owned(),
        "demo.actor.sheep",
        Position { x: 13, y: 10 },
    );
    let mut saw_hit_bonus = false;
    let mut saw_breakage_bonus = false;
    for seed in 0..1000 {
        let mut ranger = base.clone();
        ranger.entities[0].hp = 1000;
        ranger.entities[0].max_hp = 1000;
        ranger.rng = RfbRng::seeded(seed);
        let mut without_bonus = ranger.clone();
        without_bonus
            .items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.soft-leather-armour")
            .unwrap()
            .intrinsic_properties
            .equipment_bonuses
            .ranged_skill = -70;
        let boosted = shoot(&mut ranger);
        let ordinary = shoot(&mut without_bonus);
        saw_hit_bonus |= boosted.0 && !ordinary.0;
        saw_breakage_bonus |= boosted.0 && ordinary.0 && !boosted.1 && ordinary.1;
        if saw_hit_bonus && saw_breakage_bonus {
            break;
        }
    }
    assert!(saw_hit_bonus && saw_breakage_bonus);
    let mut restored = Game::from_save(base.to_save()).unwrap();
    assert_eq!(shoot(&mut restored), shoot(&mut base));
    assert_eq!(restored.state_hash(), base.state_hash());
    assert_eq!(restored.rng, base.rng);
}

#[test]
fn tree_travel_keeps_normal_cost_and_mount_rules_without_wall_or_snow_privileges() {
    for mount in [
        None,
        Some("demo.actor.horse"),
        Some("demo.actor.hippocampus"),
    ] {
        let mut game = at_level(BUILD, 1);
        let start = Position { x: 99, y: 33 };
        let target = Position { x: 100, y: 33 };
        game.player.position = start;
        replace_terrain(
            &mut game,
            start,
            if mount == Some("demo.actor.hippocampus") {
                "demo.terrain.surface-water-deep"
            } else {
                "demo.terrain.floor"
            },
        );
        replace_terrain(&mut game, target, "demo.terrain.surface-tree");
        let index = game.index(target).unwrap();
        game.explored[index] = true;
        if let Some(kind) = mount {
            game.push_generated_actor("test.mount".to_owned(), kind, start);
            game.entities[0].controller_id = Some(game.player.id.clone());
            game.riding_actor_id = Some("test.mount".to_owned());
        }
        let allowed = mount != Some("demo.actor.hippocampus");
        let tree = game.content.terrain("demo.terrain.surface-tree").unwrap();
        assert_eq!(game.player_can_cross_surface_terrain(tree), allowed);
        if !allowed {
            assert!(!game.actor_can_enter_position(0, target));
            let mut invalid = game.clone();
            invalid.player.position = target;
            invalid.entities[0].position = target;
            assert!(matches!(
                Game::from_save(invalid.to_save()),
                Err(CoreError::InvalidSave("entity position is invalid"))
            ));
        }
        let mut floor = game.clone();
        replace_terrain(&mut floor, target, "demo.terrain.floor");
        dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        assert_eq!(game.player.position, if allowed { target } else { start });
        if allowed {
            dispatch_next(
                &mut floor,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            assert_eq!(game.world_tick, floor.world_tick);
            if mount.is_some() {
                assert_eq!(game.entities[0].position, target);
                let mut invalid = game.clone();
                invalid.riding_actor_id = None;
                invalid.player.position = start;
                assert!(matches!(
                    Game::from_save(invalid.to_save()),
                    Err(CoreError::InvalidSave("entity position is invalid"))
                ));
            }
            let restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(restored.state_hash(), game.state_hash());
        }
    }
    let mut game = at_level(BUILD, 1);
    assert!(!game.player_can_cross_terrain(game.content.terrain("demo.terrain.wall").unwrap()));
    let position = game.player.position;
    replace_terrain(&mut game, position, "demo.terrain.surface-snow");
    assert_eq!(game.player_snow_movement_action_cost(100), 133);
}

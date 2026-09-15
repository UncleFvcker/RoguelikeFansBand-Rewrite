// SPDX-License-Identifier: MPL-2.0
use super::support::{
    choose_human_talent_if_pending, clear_monsters, dispatch_next, give_inventory_item,
};
use super::*;

fn samurai() -> Game {
    let mut g = Game::new_with_build(926, "demo.build.samurai").unwrap();
    clear_monsters(&mut g);
    g.apply_player_experience(g.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut g);
    g.player.position = Position { x: 10, y: 10 };
    for y in 4..=25 {
        for x in 4..=30 {
            let i = g.index(Position { x, y }).unwrap();
            g.terrain[i] = "demo.terrain.floor".into();
        }
    }
    g.glow.fill(true);
    g.reveal_current_visibility();
    for _ in 0..32 {
        g.samurai_concentrate();
    }
    assert!(!g.equipped_melee_weapons().is_empty());
    g
}

fn learn(g: &mut Game, slot: usize) -> String {
    let book = [
        "bugei-shofu",
        "yagyuu-bugeichou",
        "gorinnosho",
        "hokusin-ittouryuu-kaiden",
    ][slot / 8];
    let ids = g
        .content
        .ability_book(&format!("demo.ability-book.{book}"))
        .unwrap()
        .ability_ids
        .clone();
    let item = format!("test.{book}");
    give_inventory_item(g, &item, &format!("demo.item.{book}"));
    g.study_player_ability(&item, &ids[slot % 8]).unwrap();
    for id in &ids {
        assert!(g.learned_abilities.contains(id));
    }
    ids[slot % 8].clone()
}

fn cast(g: &mut Game, id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    g.resolve_player_ability(
        id,
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. })),
        "{id}: {events:?}"
    );
    events
}

fn target(g: &mut Game, hp: i32) {
    let mut m = g.generated_actor(
        "test.sword-target".into(),
        "demo.actor.war-bear",
        Position { x: 11, y: 10 },
    );
    m.hp = hp;
    m.max_hp = hp;
    m.nice = true;
    g.entities.push(m);
    g.reveal_current_visibility();
}

#[test]
fn hissatsu_formal_books_batch_study_and_all_32_execute_without_books() {
    for slot in 0..32 {
        let mut g = samurai();
        let id = learn(&mut g, slot);
        g.items.retain(|i| {
            g.content
                .item(&i.kind_id)
                .is_none_or(|d| d.ability_book_id.is_none())
        });
        target(&mut g, 5000);
        let t = if slot == 11 {
            TargetSelection::Item {
                item_id: g.equipped_melee_weapons()[0].id.clone(),
            }
        } else if matches!(slot, 4 | 6 | 19 | 22 | 25 | 31) {
            TargetSelection::SelfTarget
        } else if slot == 27 {
            TargetSelection::Position {
                position: Position { x: 15, y: 10 },
            }
        } else {
            TargetSelection::Direction {
                direction: Direction::East,
            }
        };
        let profile = g.casting_profile().unwrap();
        assert_eq!(
            g.ability_failure_percent(profile, g.content.ability(&id).unwrap()),
            0
        );
        cast(&mut g, &id, t);
        if slot != 31 {
            clear_monsters(&mut g);
            let restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
            assert_eq!(g.state_hash(), restored.state_hash(), "slot {slot}");
        } else {
            assert!(g.player_is_dead());
        }
    }
}

#[test]
fn hissatsu_supercharge_survives_recovery_and_save_but_rejects_excess() {
    let mut g = samurai();
    let pool = g.resources["demo.resource.mana"];
    assert!(pool.current > pool.maximum);
    g.recover_player_resources(false, &mut Vec::new());
    assert_eq!(g.resources["demo.resource.mana"].current, pool.current);
    let restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    assert_eq!(g.state_hash(), restored.state_hash());
    let mut save = g.to_save();
    save.player
        .resources
        .iter_mut()
        .find(|p| p.id == "demo.resource.mana")
        .unwrap()
        .current = Game::samurai_mana_limit(pool.maximum, 50) + 1;
    assert!(Game::from_save(save, Game::default_behavior_preferences()).is_err());
}

#[test]
fn hissatsu_postures_apply_and_musou_spends_mana_on_wait() {
    let mut g = samurai();
    let ac = g.player_derived_stats().armor_class.value;
    let blows = |g: &Game| {
        let p = g.player_melee_profiles(&g.player_derived_stats());
        u32::from(p[0].attacks) * 100 + u32::from(p[0].extra_attack_chance_percent)
    };
    let before = blows(&g);
    cast(
        &mut g,
        "demo.ability.samurai-fuujin",
        TargetSelection::SelfTarget,
    );
    assert_eq!(blows(&g), before.saturating_sub(100));
    let attrs = g.effective_player_attributes();
    cast(
        &mut g,
        "demo.ability.samurai-koukijin",
        TargetSelection::SelfTarget,
    );
    assert!(g.effective_player_attributes().wisdom > attrs.wisdom);
    assert!(g.effective_player_attributes().strength > attrs.strength);
    cast(
        &mut g,
        "demo.ability.samurai-no-posture",
        TargetSelection::SelfTarget,
    );
    cast(
        &mut g,
        "demo.ability.samurai-musou",
        TargetSelection::SelfTarget,
    );
    assert_eq!(g.player_derived_stats().armor_class.value, ac + 100);
    assert!(
        g.player_equipment_passives()
            .contains(&EquipmentPassive::ReflectsBolts)
    );
    let before = g.resources["demo.resource.mana"].current;
    g.advance_samurai();
    assert_eq!(g.resources["demo.resource.mana"].current, before - 2);
    g.resources.get_mut("demo.resource.mana").unwrap().current = 2;
    g.advance_samurai();
    assert_eq!(g.samurai.posture, 0);
    assert_eq!(g.player_derived_stats().armor_class.value, ac);
}

#[test]
fn hissatsu_stunning_hit_does_not_damage_and_sutemi_doubles_incoming_damage() {
    let mut g = samurai();
    let stun = learn(&mut g, 5);
    target(&mut g, 5000);
    for _ in 0..20 {
        g.samurai_concentrate();
        cast(
            &mut g,
            &stun,
            TargetSelection::Direction {
                direction: Direction::East,
            },
        );
    }
    assert_eq!(g.entities[0].hp, 5000);
    assert!(
        g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == STATUS_STUN)
    );
    let sutemi = learn(&mut g, 16);
    // Keep the actual hit independent of birth stock generation.
    g.rng = RfbRng::seeded(1);
    for _ in 0..32 {
        g.samurai_concentrate();
    }
    cast(
        &mut g,
        &sutemi,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert!(g.entities[0].hp < 5000);
    assert_eq!(g.player_incoming_damage_percent(), 200);
    g.advance_samurai();
    assert_eq!(g.player_incoming_damage_percent(), 100);
}

#[test]
fn hissatsu_counter_and_iai_use_actual_melee() {
    let mut g = samurai();
    target(&mut g, 5000);
    g.samurai.counter = true;
    g.rng = RfbRng::seeded(1);
    let before = g.resources["demo.resource.mana"].current;
    g.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert!(g.entities[0].hp < 5000);
    assert_eq!(g.resources["demo.resource.mana"].current, before - 7);
    g.samurai.counter = false;
    g.samurai.posture = 1;
    let before = g.entities[0].hp;
    for _ in 0..5 {
        g.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
    }
    assert!(g.entities[0].hp < before);
}

#[test]
fn hissatsu_hundred_slaughter_pause_save_and_cancel_finishes_the_action() {
    let mut g = samurai();
    let id = learn(&mut g, 26);
    target(&mut g, 1);
    // Search deterministic combat seeds until the first charge hits.
    for _ in 0..20 {
        cast(
            &mut g,
            &id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
        );
        if g.pending_ability_direction.is_some() {
            break;
        }
        g.samurai_concentrate();
    }
    assert!(g.pending_ability_direction.is_some());
    let mut restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    let before = g.resources["demo.resource.mana"].current;
    dispatch_next(&mut g, GameCommand::CancelAbilityDirection);
    dispatch_next(&mut restored, GameCommand::CancelAbilityDirection);
    assert!(g.pending_ability_direction.is_none());
    assert_eq!(g.state_hash(), restored.state_hash());
    assert!(g.resources["demo.resource.mana"].current <= before);
}

#[test]
fn hissatsu_four_books_generate_in_the_full_pool_and_teach_after_pickup() {
    for (rank, book) in [
        "bugei-shofu",
        "yagyuu-bugeichou",
        "gorinnosho",
        "hokusin-ittouryuu-kaiden",
    ]
    .iter()
    .enumerate()
    {
        let mut g = samurai();
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: g.current_floor_id.clone(),
            depth: [20, 30, 55, 85][rank],
            source: LootSource::ItemUse {
                item_id: "test.hissatsu-pool".into(),
            },
        };
        let kind = format!("demo.item.{book}");
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == kind)
            })
            .expect("formal sword book generation");
        let item = g
            .commit_generated_item_draft(draft, ItemLocation::Ground(g.player.position))
            .unwrap();
        let id = item.id.clone();
        g.items.push(item);
        g.pick_up_item_at_player(Some(&id)).unwrap();
        let spells = g
            .content
            .ability_book(&format!("demo.ability-book.{book}"))
            .unwrap()
            .ability_ids
            .clone();
        g.study_player_ability(&id, &spells[0]).unwrap();
        assert!(spells.iter().all(|id| g.learned_abilities.contains(id)));
        assert_eq!(
            g.state_hash(),
            Game::from_save(g.to_save(), g.behavior_preferences())
                .unwrap()
                .state_hash()
        );
    }
}

#[test]
fn hissatsu_dragon_flash_requires_clear_path_and_moves_after_actual_attacks() {
    let mut g = samurai();
    let id = learn(&mut g, 27);
    target(&mut g, 5000);
    let target = TargetSelection::Position {
        position: Position { x: 15, y: 10 },
    };
    let wall = g.index(Position { x: 13, y: 10 }).unwrap();
    g.terrain[wall] = "demo.terrain.wall".into();
    assert!(
        g.ability_target_plan(g.content.ability(&id).unwrap(), &target)
            .is_none()
    );
    g.terrain[wall] = "demo.terrain.floor".into();
    cast(&mut g, &id, target);
    assert_eq!(g.player.position, Position { x: 15, y: 10 });
    assert!(g.entities[0].hp < 5000);
}

#[test]
fn hissatsu_overcharge_decay_and_posture_rules_resume_identically() {
    let mut g = samurai();
    let before = g.resources["demo.resource.mana"].current;
    let mut restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    for _ in 0..3 {
        dispatch_next(&mut g, GameCommand::Wait);
        dispatch_next(&mut restored, GameCommand::Wait);
        assert_eq!(g.state_hash(), restored.state_hash());
    }
    assert!(g.resources["demo.resource.mana"].current < before);
    cast(
        &mut g,
        "demo.ability.samurai-koukijin",
        TargetSelection::SelfTarget,
    );
    assert!(Game::from_save(g.to_save(), g.behavior_preferences()).is_ok());
    let id = learn(&mut g, 4);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    assert_eq!(g.samurai.posture, 0);
    assert!(Game::from_save(g.to_save(), g.behavior_preferences()).is_ok());
}

#[test]
fn hissatsu_moon_respects_sleep_immunity_for_paralysis() {
    let mut g = samurai();
    let id = learn(&mut g, 25);
    target(&mut g, 5000);
    let seed = (0..100)
        .find(|&seed| {
            let mut trial = g.clone();
            trial.rng = RfbRng::seeded(seed);
            cast(&mut trial, &id, TargetSelection::SelfTarget);
            trial.entities[0]
                .statuses
                .iter()
                .any(|s| s.kind_id == "rfb.status.paralysis")
        })
        .expect("moon paralysis branch against a susceptible monster");
    let mut protection =
        crate::game::monster_combat::melee_status("test.sleep-immunity", 100, "test.moon").status;
    protection
        .granted_status_immunities
        .insert("rfb.status.sleep".into());
    g.entities[0].statuses.push(protection);
    g.rng = RfbRng::seeded(seed);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    assert!(
        g.entities[0]
            .statuses
            .iter()
            .all(|s| s.kind_id != "rfb.status.paralysis")
    );
}

#[test]
fn hissatsu_mana_brand_charges_the_samurai_weapon_dice_rate() {
    let mut g = samurai();
    let id = g.equipped_melee_weapons()[0].id.clone();
    g.items
        .iter_mut()
        .find(|i| i.id == id)
        .unwrap()
        .intrinsic_melee_damage_dice = Some(rfb_protocol::MeleeDamageDiceDto { dice: 2, sides: 7 });
    g.player.statuses.push(
        crate::game::monster_combat::melee_status(STATUS_MANA_BRAND, 100, "test.force").status,
    );
    target(&mut g, 5000);
    let before = g.resources["demo.resource.mana"].current;
    let hit = (0..100)
        .find_map(|seed| {
            let mut trial = g.clone();
            trial.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            trial
                .resolve_hissatsu_melee(0, 0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            events
                .iter()
                .any(|e| matches!(e, DomainEvent::PlayerMeleeHit { .. }))
                .then_some(trial)
        })
        .expect("actual branded hit");
    assert_eq!(before - hit.resources["demo.resource.mana"].current, 5);
}

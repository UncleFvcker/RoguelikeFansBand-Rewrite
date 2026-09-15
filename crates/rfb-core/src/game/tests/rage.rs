// SPDX-License-Identifier: MPL-2.0
use super::support::{
    choose_human_talent_if_pending, clear_monsters, dispatch_next, give_inventory_item,
};
use super::*;

fn mage() -> Game {
    let mut g = Game::new_with_build(927, "demo.build.rage-mage").unwrap();
    clear_monsters(&mut g);
    g.apply_player_experience(g.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut g);
    g.player.position = Position { x: 10, y: 10 };
    for y in 5..=25 {
        for x in 5..=30 {
            let i = g.index(Position { x, y }).unwrap();
            g.terrain[i] = "demo.terrain.floor".into();
        }
    }
    g.glow.fill(true);
    g.reveal_current_visibility();
    g.debug_ability_casts_succeed = true;
    g.player.hp = g.effective_player_max_hp();
    g
}
fn learn(g: &mut Game, slot: usize) -> String {
    let book = [
        "anger-management",
        "northern-frights",
        "the-sound-and-the-fury",
        "dire-ire",
    ][slot / 8];
    let id = g
        .content
        .ability_book(&format!("demo.ability-book.{book}"))
        .unwrap()
        .ability_ids[slot % 8]
        .clone();
    give_inventory_item(g, "test.book", &format!("demo.item.{book}"));
    g.study_player_ability("test.book", &id).unwrap();
    assert!(!g.items.iter().any(|i| i.id == "test.book"));
    id
}
fn cast(g: &mut Game, id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    for p in g.resources.values_mut() {
        p.current = p.maximum;
    }
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
fn target(g: &mut Game, kind: &str, p: Position) {
    let mut actor = g.generated_actor("test.target".into(), kind, p);
    actor.nice = true;
    g.entities.push(actor);
    g.reveal_current_visibility();
}

#[test]
fn rage_all_32_formal_techniques_cast_and_resume_without_books() {
    for slot in 0..32 {
        let mut g = mage();
        let id = learn(&mut g, slot);
        target(
            &mut g,
            "demo.actor.ancient-red-dragon",
            Position { x: 11, y: 10 },
        );
        let selection = match slot {
            0 | 12 => TargetSelection::Direction {
                direction: Direction::East,
            },
            2 | 10 | 22 | 23 | 30 | 31 => TargetSelection::Entity {
                entity_id: "test.target".into(),
            },
            3 => TargetSelection::SelfTarget,
            24 => TargetSelection::Item {
                item_id: g.equipped_melee_weapons()[0].id.clone(),
            },
            28 => {
                give_inventory_item(&mut g, "test.device", "demo.item.magic-missile-wand");
                TargetSelection::Item {
                    item_id: "test.device".into(),
                }
            }
            _ => TargetSelection::SelfTarget,
        };
        cast(&mut g, &id, selection);
        let mut restored = Game::from_save(g.to_save(), g.behavior_preferences())
            .unwrap_or_else(|e| panic!("slot {slot}: {e:?}"));
        g.rage_after_action(100);
        restored.rage_after_action(100);
        assert_eq!(g.state_hash(), restored.state_hash(), "slot {slot}");
    }
}

#[test]
fn rage_learning_consumes_one_copy_and_rejects_repeat_without_consumption() {
    let mut g = mage();
    give_inventory_item(&mut g, "test.book", "demo.item.anger-management");
    let i = g.items.iter().position(|i| i.id == "test.book").unwrap();
    g.items[i].quantity = 2;
    let id = "demo.ability.rage-shout";
    g.study_player_ability("test.book", id).unwrap();
    assert_eq!(
        g.items
            .iter()
            .find(|i| i.id == "test.book")
            .unwrap()
            .quantity,
        1
    );
    let hash = g.state_hash();
    assert!(g.study_player_ability("test.book", id).is_err());
    assert_eq!(g.state_hash(), hash);
    g.items
        .retain(|i| i.id != "test.book" && i.kind_id != "demo.item.anger-management");
    g.item_property_knowledge
        .retain(|id, _| g.items.iter().any(|i| &i.id == id));
    assert_eq!(
        g.forget_player_ability(id),
        Err("manual-forgetting-unavailable")
    );
    cast(
        &mut g,
        id,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
}

#[test]
fn rage_damage_fuels_mana_actual_spell_damage_sustains_it_and_idle_drains_it() {
    let mut g = mage();
    let hp = g.effective_player_max_hp();
    g.player.hp = hp;
    g.resources.get_mut("demo.resource.mana").unwrap().current = 0;
    g.apply_final_player_damage(
        crate::effect::resolve_damage(
            crate::effect::DamagePacket::new(20, DamageType::Physical),
            ResistanceLevel::Normal,
        ),
        crate::game::damage::FatalityPolicy::BelowZero,
    );
    assert_eq!(
        g.resources["demo.resource.mana"].current,
        if hp % 2 == 0 { 10 } else { 9 }
    );
    let id = learn(&mut g, 0);
    target(
        &mut g,
        "demo.actor.ancient-red-dragon",
        Position { x: 12, y: 10 },
    );
    cast(
        &mut g,
        &id,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert!(g.rage_mana_sustained);
    let before = g.resources["demo.resource.mana"].current;
    g.rage_after_action(100);
    assert_eq!(g.resources["demo.resource.mana"].current, before);
    g.rage_after_action(100);
    assert_eq!(
        g.resources["demo.resource.mana"].current,
        before.saturating_sub(before / 8 + 6)
    );
    assert_eq!(
        g.player_resource_recovery_change("demo.resource.mana", true),
        0
    );
    g.player.hp = hp;
    g.resources.get_mut("demo.resource.mana").unwrap().current = 0;
    let mut poison = monster_combat::melee_status(STATUS_POISON, 10, "test.poison").status;
    poison.intensity = 20;
    g.player.statuses.push(poison);
    g.process_status_tick(
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        false,
    )
    .unwrap();
    assert_eq!(g.player.hp, hp - 20);
    assert_eq!(
        g.resources["demo.resource.mana"].current,
        if hp % 2 == 0 { 10 } else { 9 }
    );
}

#[test]
fn rage_focus_failure_and_strike_pay_life_and_clear_mana_at_correct_boundaries() {
    let mut g = mage();
    let id = learn(&mut g, 5);
    g.resources.get_mut("demo.resource.mana").unwrap().current = 0;
    let hp = g.player.hp;
    g.resolve_player_ability(
        &id,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(g.player.hp, hp - 35);
    assert_eq!(g.resources["demo.resource.mana"].current, 35);
    g.rage_after_action(100);
    assert_eq!(g.resources["demo.resource.mana"].current, 35);
    g.rage_failure(5);
    assert_eq!(g.player.hp, hp - 70);
    assert_eq!(g.resources["demo.resource.mana"].current, 35);
    let id = learn(&mut g, 31);
    cast(
        &mut g,
        &id,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert_eq!(g.resources["demo.resource.mana"].current, 0);
    assert!(g.player_has_status_kind(STATUS_STUN));
    let mut bad = Game::new_with_build(7, "demo.build.warrior")
        .unwrap()
        .to_save();
    bad.player.rage_mana_sustained = true;
    assert!(Game::from_save(bad, Game::default_behavior_preferences()).is_err());
}

#[test]
fn rage_berserk_allows_casting_and_changes_actual_resistance_duration() {
    let mut g = mage();
    let disenchant = learn(&mut g, 9);
    for _ in 0..8 {
        cast(&mut g, &disenchant, TargetSelection::SelfTarget);
    }
    assert!(
        g.player.statuses.iter().any(
            |s| s.kind_id == "rfb.status.rage-resist-disenchantment" && s.remaining_ticks <= 20
        )
    );
    let berserk = learn(&mut g, 16);
    let resist = learn(&mut g, 18);
    cast(&mut g, &berserk, TargetSelection::SelfTarget);
    assert!(g.player_has_status_kind(STATUS_BERSERK));
    let effects = cast(&mut g, &resist, TargetSelection::SelfTarget);
    assert!(!effects.is_empty());
    assert!(
        g.player
            .statuses
            .iter()
            .any(|s| !s.granted_resistances.is_empty() && s.remaining_ticks >= 21)
    );
    assert!(
        g.player
            .statuses
            .iter()
            .any(|s| s.kind_id == STATUS_BERSERK && s.granted_modifiers.max_hp == 30)
    );
}

#[test]
fn rage_shatter_destroys_device_and_uses_actual_activation_damage() {
    let mut g = mage();
    let id = learn(&mut g, 28);
    target(
        &mut g,
        "demo.actor.ancient-red-dragon",
        Position { x: 12, y: 10 },
    );
    give_inventory_item(&mut g, "test.device", "demo.item.frost-ball-wand");
    let hp = g.entities[0].hp;
    cast(
        &mut g,
        &id,
        TargetSelection::Item {
            item_id: "test.device".into(),
        },
    );
    assert!(!g.items.iter().any(|i| i.id == "test.device"));
    assert!(g.entities.is_empty() || g.entities[0].hp < hp);
    give_inventory_item(&mut g, "test.empty", "demo.item.staff-of-nothing");
    cast(
        &mut g,
        &id,
        TargetSelection::Item {
            item_id: "test.empty".into(),
        },
    );
    assert!(!g.items.iter().any(|i| i.id == "test.empty"));
}

#[test]
fn rage_restore_mana_is_ineffective_and_boldness_converts_mana_to_life() {
    let mut g = mage();
    g.resources.get_mut("demo.resource.mana").unwrap().current = 100;
    let hp = g.player.hp;
    g.player.hp -= 60;
    g.resolve_item_restorative_resource_effect(
        "demo.item.perfect-focus-potion",
        &rfb_content::ItemUseEffectDefinition::RestoreResourceFull {
            resource_id: "demo.resource.mana".into(),
        },
        &mut Vec::new(),
    );
    assert_eq!(g.resources["demo.resource.mana"].current, 100);
    g.resolve_item_status_removal("demo.item.boldness-potion", STATUS_FEAR, &mut Vec::new());
    assert_eq!(g.player.hp, hp);
    assert_eq!(g.resources["demo.resource.mana"].current, 0);
    dispatch_next(&mut g, GameCommand::Wait);
    assert_eq!(
        g.state_hash(),
        Game::from_save(g.to_save(), g.behavior_preferences())
            .unwrap()
            .state_hash()
    );
}

#[test]
fn rage_anti_magic_turning_and_fury_affect_actual_monster_casts() {
    let mut g = mage();
    target(
        &mut g,
        "demo.actor.giant-white-mouse",
        Position { x: 12, y: 10 },
    );
    let ray = learn(&mut g, 30);
    for _ in 0..40 {
        cast(
            &mut g,
            &ray,
            TargetSelection::Entity {
                entity_id: "test.target".into(),
            },
        );
        if g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == "rfb.status.rage-anti-magic")
        {
            break;
        }
    }
    assert!(
        g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == "rfb.status.rage-anti-magic")
    );
    let a = g
        .content
        .ability("rfb-legacy.ability.no-air-40")
        .unwrap()
        .clone();
    let plan = g.monster_ability_target_plan(0, a, 1).unwrap();
    g.resolve_monster_ability_plan(
        0,
        "demo.actor.giant-white-mouse",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(!g.player_has_status_kind("rfb.status.no-air"));
    g.entities[0].statuses.clear();
    let armor = learn(&mut g, 20);
    cast(&mut g, &armor, TargetSelection::SelfTarget);
    let a = g
        .content
        .ability("rfb-legacy.ability.bolt-fire-9d8-4")
        .unwrap()
        .clone();
    let plan = g.monster_ability_target_plan(0, a, 1).unwrap();
    for _ in 0..40 {
        g.player.hp = g.effective_player_max_hp();
        g.resolve_monster_ability_plan(
            0,
            "demo.actor.giant-white-mouse",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );
        if g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == STATUS_SLOW)
        {
            break;
        }
    }
    assert!(
        g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == STATUS_SLOW)
    );
    let turning = learn(&mut g, 27);
    cast(&mut g, &turning, TargetSelection::SelfTarget);
    let mut reflected = false;
    for _ in 0..80 {
        g.player.hp = g.effective_player_max_hp();
        let before = g.player.hp;
        g.resolve_monster_ability_plan(
            0,
            "demo.actor.giant-white-mouse",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );
        if g.entities.is_empty() {
            assert_eq!(g.player.hp, before);
            reflected = true;
            break;
        }
    }
    assert!(
        reflected,
        "turned fire bolt kills the caster and spares the player"
    );
}

#[test]
fn rage_mana_clash_spares_noncasters_and_awesome_blow_reaches_collision_damage() {
    let mut g = mage();
    let clash = learn(&mut g, 23);
    target(
        &mut g,
        "demo.actor.giant-white-mouse",
        Position { x: 11, y: 10 },
    );
    let hp = g.entities[0].hp;
    cast(
        &mut g,
        &clash,
        TargetSelection::Entity {
            entity_id: "test.target".into(),
        },
    );
    assert_eq!(g.entities[0].hp, hp);
    g.entities.clear();
    target(
        &mut g,
        "demo.actor.ancient-red-dragon",
        Position { x: 11, y: 10 },
    );
    let hp = g.entities[0].hp;
    cast(
        &mut g,
        &clash,
        TargetSelection::Entity {
            entity_id: "test.target".into(),
        },
    );
    assert!(g.entities.is_empty() || g.entities[0].hp < hp);
    if g.entities.is_empty() {
        target(
            &mut g,
            "demo.actor.ancient-red-dragon",
            Position { x: 11, y: 10 },
        );
    }
    let blow = learn(&mut g, 10);
    let i = g.index(Position { x: 13, y: 10 }).unwrap();
    g.terrain[i] = "demo.terrain.wall".into();
    let mut hit = false;
    for _ in 0..50 {
        let events = cast(
            &mut g,
            &blow,
            TargetSelection::Entity {
                entity_id: "test.target".into(),
            },
        );
        if events
            .iter()
            .any(|e| matches!(e, DomainEvent::PlayerMeleeHit { .. }))
        {
            hit = true;
            assert!(g.entities.is_empty() || g.entities[0].position == Position { x: 12, y: 10 });
            break;
        }
    }
    assert!(hit);
}

#[test]
fn rage_four_books_generate_in_the_full_pool_and_teach_after_pickup() {
    for (rank, book) in [
        "anger-management",
        "northern-frights",
        "the-sound-and-the-fury",
        "dire-ire",
    ]
    .iter()
    .enumerate()
    {
        let mut g = mage();
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: g.current_floor_id.clone(),
            depth: [20, 30, 55, 85][rank],
            source: LootSource::ItemUse {
                item_id: "test.rage-pool".into(),
            },
        };
        let kind = format!("demo.item.{book}");
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == kind)
            })
            .expect("formal Rage book generation");
        let item = g
            .commit_generated_item_draft(draft, ItemLocation::Ground(g.player.position))
            .unwrap();
        let id = item.id.clone();
        g.items.push(item);
        g.pick_up_item_at_player(Some(&id)).unwrap();
        let spell = g
            .content
            .ability_book(&format!("demo.ability-book.{book}"))
            .unwrap()
            .ability_ids[0]
            .clone();
        g.study_player_ability(&id, &spell).unwrap();
        assert!(g.learned_abilities.contains(&spell));
        assert_eq!(
            g.state_hash(),
            Game::from_save(g.to_save(), g.behavior_preferences())
                .unwrap()
                .state_hash()
        );
    }
}

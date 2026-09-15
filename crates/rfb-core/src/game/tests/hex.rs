// SPDX-License-Identifier: MPL-2.0
use super::support::{
    choose_human_talent_if_pending, clear_monsters, dispatch_next, give_inventory_item,
};
use super::*;

fn mage() -> Game {
    let mut g = Game::new_with_build(927, "demo.build.high-mage-hex").unwrap();
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
    g
}
fn learn(g: &mut Game, slot: usize) -> String {
    let book = [
        "handbook-of-hex",
        "high-curse",
        "curse-and-spelling",
        "forbidden-cursebook",
    ][slot / 8];
    let id = g
        .content
        .ability_book(&format!("demo.ability-book.{book}"))
        .unwrap()
        .ability_ids[slot % 8]
        .clone();
    let item = format!("test.{book}");
    if !g.items.iter().any(|i| i.id == item) {
        give_inventory_item(g, &item, &format!("demo.item.{book}"));
    }
    g.study_player_ability(&item, &id).unwrap();
    id
}
fn cast(g: &mut Game, id: &str, target: TargetSelection) {
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
}
fn pulse(g: &mut Game) {
    g.advance_hex(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
}
fn target(g: &mut Game) {
    let mut e = g.generated_actor(
        "test.hex-target".into(),
        "demo.actor.ancient-red-dragon",
        Position { x: 12, y: 10 },
    );
    e.nice = true;
    g.entities.push(e);
    g.reveal_current_visibility();
}

#[test]
fn hex_all_32_formal_spells_cast_and_resume() {
    for slot in 0..32 {
        let mut g = mage();
        let id = learn(&mut g, slot);
        target(&mut g);
        let weapon = g.equipped_melee_weapons()[0].id.clone();
        let armor = g
            .items
            .iter()
            .find(|i| {
                i.kind_id == "demo.item.robe" && matches!(i.location, ItemLocation::Equipped { .. })
            })
            .unwrap()
            .id
            .clone();
        let selection = match slot {
            5 | 26 => {
                if slot == 26 {
                    g.items.iter_mut().find(|i| i.id == weapon).unwrap().curse =
                        Some(rfb_protocol::ItemCurseSeverityDto::Normal);
                }
                TargetSelection::Item { item_id: weapon }
            }
            20 => TargetSelection::Item { item_id: armor },
            10 => {
                give_inventory_item(&mut g, "test.potion", "demo.item.light-healing-potion");
                TargetSelection::Item {
                    item_id: "test.potion".into(),
                }
            }
            18 => {
                let id = g
                    .items
                    .iter()
                    .find(|i| i.charges.is_some())
                    .unwrap()
                    .id
                    .clone();
                g.items
                    .iter_mut()
                    .find(|i| i.id == id)
                    .unwrap()
                    .charges
                    .as_mut()
                    .unwrap()
                    .current = 0;
                TargetSelection::Item { item_id: id }
            }
            21 => {
                give_inventory_item(&mut g, "test.cloak", "demo.item.cloak");
                g.equip_inventory_item("test.cloak", None).unwrap();
                let item = g.items.iter_mut().find(|i| i.id == "test.cloak").unwrap();
                item.curse = Some(rfb_protocol::ItemCurseSeverityDto::Normal);
                TargetSelection::SelfTarget
            }
            29 => TargetSelection::Position {
                position: Position { x: 11, y: 10 },
            },
            _ => TargetSelection::SelfTarget,
        };
        cast(&mut g, &id, selection);
        let mut loaded = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
        pulse(&mut g);
        pulse(&mut loaded);
        assert_eq!(g.state_hash(), loaded.state_hash(), "slot {slot}");
    }
}

#[test]
fn hex_concurrent_upkeep_stop_dispel_and_mana_fraction_resume() {
    let mut g = mage();
    for slot in [0, 2, 8, 16] {
        let id = learn(&mut g, slot);
        cast(&mut g, &id, TargetSelection::SelfTarget);
    }
    let extra = learn(&mut g, 4);
    assert_eq!(
        g.hex_ability_unavailable_reason(&extra),
        Some("hex-capacity")
    );
    assert_eq!(g.hex.active.count_ones(), 4);
    let before = g.resources["demo.resource.mana"].current;
    pulse(&mut g);
    assert!(g.resources["demo.resource.mana"].current < before);
    assert!(g.resources["demo.resource.mana"].fraction > 0);
    g.interrupt_hex();
    assert!(!g.hexing(8));
    let mut loaded = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    pulse(&mut g);
    pulse(&mut loaded);
    assert!(g.hexing(8));
    assert_eq!(g.state_hash(), loaded.state_hash());
    g.player.energy_need = -100;
    dispatch_next(
        &mut g,
        GameCommand::CastAbility {
            ability_id: "demo.ability.hex-stop-ice-armor".into(),
            target: TargetSelection::SelfTarget,
        },
    );
    // The 10-energy stop is followed by one 13-energy pulse at speed 113.
    assert_eq!(g.player.energy_need, -103);
    assert!(!g.hexing(8));
    assert!(g.hexing(2));
    g.resources.get_mut("demo.resource.mana").unwrap().current = 0;
    g.resources.get_mut("demo.resource.mana").unwrap().fraction = 0;
    pulse(&mut g);
    assert_eq!(g.hex.active, 0);
}

#[test]
fn hex_cursed_weapon_bonuses_and_sustained_confusion_have_real_consumers() {
    let mut g = mage();
    let curse = learn(&mut g, 5);
    let rune = learn(&mut g, 12);
    let confuse = learn(&mut g, 13);
    let item = g.equipped_melee_weapons()[0].id.clone();
    assert_eq!(
        g.craft_ability_item_targets(g.content.ability(&curse).unwrap())
            .unwrap()[0]
            .confirmation_key
            .as_deref(),
        Some("item-hex-curse-confirm")
    );
    let base = g.player_melee_profiles(&g.player_derived_stats())[0].clone();
    cast(
        &mut g,
        &curse,
        TargetSelection::Item {
            item_id: item.clone(),
        },
    );
    assert!(
        g.items
            .iter()
            .find(|i| i.id == item)
            .unwrap()
            .curse
            .is_some()
    );
    cast(&mut g, &rune, TargetSelection::SelfTarget);
    cast(&mut g, &confuse, TargetSelection::SelfTarget);
    let boosted = g.player_melee_profiles(&g.player_derived_stats())[0].clone();
    assert!(boosted.to_damage >= base.to_damage + 5);
    assert!(boosted.melee_skill.value > base.melee_skill.value);
    clear_monsters(&mut g);
    let mut e = g.generated_actor(
        "test.confusion".into(),
        "demo.actor.war-bear",
        Position { x: 11, y: 10 },
    );
    e.hp = 5000;
    e.max_hp = 5000;
    g.entities.push(e);
    for _ in 0..20 {
        g.resolve_player_melee(
            0,
            true,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    }
    assert!(g.entities[0].hp < 5000);
    assert!(
        g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == "rfb.status.confusion")
    );
    assert!(g.hexing(13));
}

#[test]
fn hex_patience_and_revenge_store_actual_damage_and_prompt_at_expiry() {
    for slot in [7, 31] {
        let mut g = mage();
        let id = learn(&mut g, slot);
        target(&mut g);
        cast(&mut g, &id, TargetSelection::SelfTarget);
        let damage = resolve_damage(
            DamagePacket::new(30, DamageType::Physical),
            ResistanceLevel::Normal,
        );
        g.apply_final_player_damage(damage, super::super::damage::FatalityPolicy::BelowZero);
        assert_eq!(g.hex.revenge_damage, 30);
        while g.hex.revenge_ticks > 0 {
            pulse(&mut g);
        }
        if slot == 31 {
            assert!(g.pending_ability_direction.is_some());
            let mut invalid = g.to_save();
            invalid.player.hex = Default::default();
            assert!(Game::from_save(invalid, Game::default_behavior_preferences()).is_err());
            let turn = g.turn;
            dispatch_next(&mut g, GameCommand::CancelAbilityDirection);
            assert_eq!(g.turn, turn);
            assert!(g.pending_ability_direction.is_some());
            let mut loaded = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
            for x in [&mut g, &mut loaded] {
                dispatch_next(
                    x,
                    GameCommand::ResolveAbilityDirection {
                        direction: Direction::East,
                    },
                );
            }
            assert_eq!(g.state_hash(), loaded.state_hash());
        }
        assert!(g.entities[0].hp < g.entities[0].max_hp);
        assert_eq!(g.hex.revenge_kind, 0);
    }
}

#[test]
fn hex_inhale_preserves_chants_and_invalid_state_is_rejected() {
    let mut g = mage();
    let bless = learn(&mut g, 0);
    let inhale = learn(&mut g, 10);
    cast(&mut g, &bless, TargetSelection::SelfTarget);
    give_inventory_item(&mut g, "test.potion", "demo.item.light-healing-potion");
    cast(
        &mut g,
        &inhale,
        TargetSelection::Item {
            item_id: "test.potion".into(),
        },
    );
    assert!(g.hexing(0));
    assert!(!g.items.iter().any(|i| i.id == "test.potion"));
    assert_eq!(
        g.forget_player_ability(&bless),
        Err("manual-forgetting-unavailable")
    );
    let mut save = g.to_save();
    save.player.hex.active |= 1 << 5;
    assert!(Game::from_save(save, Game::default_behavior_preferences()).is_err());
    dispatch_next(&mut g, GameCommand::Wait);
    assert_eq!(
        g.state_hash(),
        Game::from_save(g.to_save(), g.behavior_preferences())
            .unwrap()
            .state_hash()
    );
}

#[test]
fn hex_barriers_stop_actual_teleport_reproduction_and_spell_effects() {
    let mut g = mage();
    let e = g.generated_actor(
        "test.mouse".into(),
        "demo.actor.giant-white-mouse",
        Position { x: 12, y: 10 },
    );
    g.entities.push(e);
    g.reveal_current_visibility();
    let mut normal = g.clone();
    assert!((0..200).any(|_| normal.try_original_reproduction(0, &mut BTreeSet::new())));
    let id = learn(&mut g, 24);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    assert!((0..200).all(|_| !g.try_original_reproduction(0, &mut BTreeSet::new())));
    let id = learn(&mut g, 15);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    let origin = g.entities[0].position;
    {
        let mut a = g
            .content
            .ability("demo.ability.chaos-teleport-other")
            .unwrap()
            .clone();
        if let AbilityEffectDefinition::TeleportAway { power, .. } = &mut a.effect {
            *power = 100;
        }
        let plan = g
            .ability_target_plan(
                &a,
                &TargetSelection::Direction {
                    direction: Direction::East,
                },
            )
            .unwrap();
        g.resolve_player_ability_effect(
            a,
            plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    }
    assert_eq!(g.entities[0].position, origin);
    g.stop_hex(Some(15));
    {
        let mut a = g
            .content
            .ability("demo.ability.chaos-teleport-other")
            .unwrap()
            .clone();
        if let AbilityEffectDefinition::TeleportAway { power, .. } = &mut a.effect {
            *power = 100;
        }
        let plan = g
            .ability_target_plan(
                &a,
                &TargetSelection::Direction {
                    direction: Direction::East,
                },
            )
            .unwrap();
        g.resolve_player_ability_effect(
            a,
            plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    }
    assert_ne!(g.entities[0].position, origin);
    g.entities[0].position = origin;
    g.reveal_current_visibility();
    let id = learn(&mut g, 30);
    cast(&mut g, &id, TargetSelection::SelfTarget);
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
    g.stop_hex(Some(30));
    g.resolve_monster_ability_plan(
        0,
        "demo.actor.giant-white-mouse",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(g.player_has_status_kind("rfb.status.no-air"));
}

#[test]
fn hex_four_books_generate_in_the_full_pool_and_teach_after_pickup() {
    for (rank, book) in [
        "handbook-of-hex",
        "high-curse",
        "curse-and-spelling",
        "forbidden-cursebook",
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
                item_id: "test.hex-pool".into(),
            },
        };
        let kind = format!("demo.item.{book}");
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == kind)
            })
            .expect("formal Hex book generation");
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

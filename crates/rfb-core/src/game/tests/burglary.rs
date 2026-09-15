// SPDX-License-Identifier: MPL-2.0
use super::support::{
    choose_human_talent_if_pending, clear_monsters, descend_one_floor, dispatch_next,
    give_inventory_item,
};
use super::*;

fn rogue() -> Game {
    let mut g = Game::new_with_build(928, "demo.build.rogue").unwrap();
    clear_monsters(&mut g);
    g.apply_player_experience(g.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut g);
    g.progress.attributes.dexterity = g.progress.attribute_potentials.dexterity;
    g.progress.maximum_attributes.dexterity = g.progress.attributes.dexterity;
    g.refresh_player_ability_state();
    g.debug_ability_casts_succeed = true;
    g.player.hp = g.effective_player_max_hp();
    g.gold = 500_000;
    g
}
fn prepare_map(g: &mut Game) {
    clear_monsters(g);
    g.player.position = Position { x: 10, y: 10 };
    for y in 5..=20 {
        for x in 5..=25 {
            let i = g.index(Position { x, y }).unwrap();
            g.terrain[i] = "demo.terrain.floor".into();
        }
    }
    g.glow.fill(true);
    g.reveal_current_visibility();
}
fn learn(g: &mut Game, slot: usize) -> String {
    let book = [
        "burglars-handbook",
        "thieving-ways",
        "great-escapes",
        "book-of-shadows",
    ][slot / 8];
    let id = g
        .content
        .ability_book(&format!("demo.ability-book.{book}"))
        .unwrap()
        .ability_ids[slot % 8]
        .clone();
    let item_id = format!("test.{book}");
    if !g.items.iter().any(|i| i.id == item_id) {
        give_inventory_item(g, &item_id, &format!("demo.item.{book}"));
    }
    g.study_player_ability(&item_id, &id).unwrap();
    for p in g.resources.values_mut() {
        p.current = p.maximum;
    }
    id
}
fn target(g: &mut Game, kind: &str) {
    let mut actor = g.generated_actor("test.target".into(), kind, Position { x: 11, y: 10 });
    actor.nice = true;
    g.entities.push(actor);
    g.reveal_current_visibility();
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

#[test]
fn burglary_all_32_formal_techniques_cast_and_resume() {
    for slot in 0..32 {
        let mut g = rogue();
        if matches!(slot, 16 | 21 | 22) {
            descend_one_floor(&mut g);
        }
        prepare_map(&mut g);
        let id = learn(&mut g, slot);
        target(&mut g, "demo.actor.novice-rogue");
        if slot == 31 {
            g.entities[0]
                .statuses
                .push(monster_combat::melee_status(STATUS_SLEEP, 100, "test.sleep").status);
        }
        if slot == 18 {
            give_inventory_item(&mut g, "test.sling", "demo.item.sling");
            g.equip_inventory_item("test.sling", Some("shooting"))
                .unwrap();
            give_inventory_item(&mut g, "test.shot", "demo.item.iron-shot");
        }
        let target = match slot {
            1 | 9 | 10 | 17 | 18 | 25 | 26 | 27 | 31 => TargetSelection::Entity {
                entity_id: "test.target".into(),
            },
            11 => {
                give_inventory_item(&mut g, "test.fetch", "demo.item.dagger");
                g.items
                    .iter_mut()
                    .find(|i| i.id == "test.fetch")
                    .unwrap()
                    .location = ItemLocation::Ground(Position { x: 13, y: 10 });
                TargetSelection::Direction {
                    direction: Direction::East,
                }
            }
            13 => TargetSelection::Item {
                item_id: g.equipped_melee_weapons()[0].id.clone(),
            },
            _ => TargetSelection::SelfTarget,
        };
        let update = dispatch_next(
            &mut g,
            GameCommand::CastAbility {
                ability_id: id.clone(),
                target,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|e| e.kind == "ability.cast-success"),
            "slot {slot}: {:?}",
            update.events
        );
        let mut restored =
            Game::from_save(g.to_save()).unwrap_or_else(|e| panic!("slot {slot}: {e}"));
        assert_eq!(g.state_hash(), restored.state_hash(), "slot {slot}");
        if g.pending_duelist.is_some() {
            let command = GameCommand::ResolveDuelistChoice {
                choice: rfb_protocol::DuelistChoiceDto::Confirm { accepted: false },
            };
            dispatch_next(&mut g, command.clone());
            dispatch_next(&mut restored, command);
            assert_eq!(g.state_hash(), restored.state_hash(), "slot {slot}");
        }
    }
}

#[test]
fn burglary_theft_and_death_share_one_saved_drop_budget() {
    let mut g = rogue();
    prepare_map(&mut g);
    let id = learn(&mut g, 26);
    target(&mut g, "demo.actor.novice-rogue");
    for n in 0..30 {
        g.player.position = Position { x: 10, y: 10 };
        g.entities[0].position = Position { x: 11, y: 10 };
        g.reveal_current_visibility();
        for p in g.resources.values_mut() {
            p.current = p.maximum;
        }
        cast(
            &mut g,
            &id,
            TargetSelection::Entity {
                entity_id: "test.target".into(),
            },
        );
        g.pending_duelist = None;
        if g.entities[0].burglary_drops_remaining == Some(0) {
            break;
        }
        assert!(n < 29);
    }
    g.reveal_current_visibility();
    let mut restored = Game::from_save(g.to_save()).unwrap();
    let victim = g.entities[0].clone();
    let a = g.generate_death_loot(&victim).unwrap();
    let b = restored.generate_death_loot(&victim).unwrap();
    assert_eq!(a, b);
    assert!(a.0.is_empty() && a.1.is_empty());
    assert_eq!(g.rng, restored.rng);
    g.entities[0].controller_id = Some(g.player.id.clone());
    give_inventory_item(&mut g, "test.ball", "demo.item.capture-ball");
    g.equip_inventory_item("test.ball", None).unwrap();
    let ball = g.items.iter().position(|i| i.id == "test.ball").unwrap();
    for _ in 0..32 {
        g.use_capture_ball(
            ball,
            Some(&TargetSelection::Entity {
                entity_id: victim.id.clone(),
            }),
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );
        if g.items[ball].captured_actor.is_some() {
            break;
        }
    }
    assert_eq!(
        g.items[ball]
            .captured_actor
            .as_ref()
            .unwrap()
            .burglary_drops_remaining,
        Some(0)
    );
    let mut captured_save = g.to_save();
    captured_save
        .equipment
        .iter_mut()
        .find(|i| i.id == "test.ball")
        .unwrap()
        .captured_actor
        .as_mut()
        .unwrap()
        .burglary_drops_remaining = Some(u32::MAX);
    assert!(Game::from_save(captured_save).is_err());
    g = Game::from_save(g.to_save()).unwrap();
    let ball = g.items.iter().position(|i| i.id == "test.ball").unwrap();
    g.use_capture_ball(
        ball,
        Some(&TargetSelection::Direction {
            direction: Direction::East,
        }),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(g.entities[0].burglary_drops_remaining, Some(0));
    let released = g.entities[0].clone();
    let drops = g.generate_death_loot(&released).unwrap();
    assert!(drops.0.is_empty() && drops.1.is_empty());
    let mut bad = g.to_save();
    bad.entities[0].burglary_drops_remaining = Some(u32::MAX);
    assert!(Game::from_save(bad).is_err());
}

#[test]
fn burglary_teleport_preserves_sleep_and_shadows_suppress_real_light() {
    let mut g = rogue();
    prepare_map(&mut g);
    let id = learn(&mut g, 25);
    target(&mut g, "demo.actor.novice-rogue");
    g.entities[0].position = Position { x: 16, y: 10 };
    g.entities[0]
        .statuses
        .push(monster_combat::melee_status(STATUS_SLEEP, 100, "test.sleep").status);
    g.reveal_current_visibility();
    cast(
        &mut g,
        &id,
        TargetSelection::Entity {
            entity_id: "test.target".into(),
        },
    );
    assert_eq!(
        chebyshev_distance(g.entities[0].position, g.player.position),
        1
    );
    assert!(
        g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == STATUS_SLEEP)
    );
    let id = learn(&mut g, 28);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    assert_eq!(g.player_light_radius(), Some(0));
    assert!(g.player_has_night_vision());
    let a = g.content.ability(&id).unwrap();
    assert!(
        g.ability_target_plan(a, &TargetSelection::SelfTarget)
            .is_none()
    );
}

#[test]
fn burglary_traps_consume_on_real_monster_entry_and_resume() {
    for slot in [7, 14, 30] {
        let mut g = rogue();
        prepare_map(&mut g);
        let id = learn(&mut g, slot);
        cast(&mut g, &id, TargetSelection::SelfTarget);
        target(&mut g, "demo.actor.novice-rogue");
        let p = g.player.position;
        g.player.position = Position { x: 9, y: 10 };
        g.entities[0].position = p;
        let mut restored = Game::from_save(g.to_save()).unwrap();
        g.trigger_actor_trap(0, p, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        restored
            .trigger_actor_trap(0, p, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        assert!(!g.terrain_at(p).contains("trap"));
        assert_eq!(g.state_hash(), restored.state_hash());
    }
}

#[test]
fn burglary_four_books_generate_in_the_full_pool_and_teach_after_pickup() {
    for (rank, book) in [
        "burglars-handbook",
        "thieving-ways",
        "great-escapes",
        "book-of-shadows",
    ]
    .iter()
    .enumerate()
    {
        let mut g = rogue();
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: g.current_floor_id.clone(),
            depth: [20, 30, 55, 85][rank],
            source: LootSource::ItemUse {
                item_id: "test.burglary-pool".into(),
            },
        };
        let kind = format!("demo.item.{book}");
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == kind)
            })
            .expect("formal Burglary book generation");
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
            Game::from_save(g.to_save()).unwrap().state_hash()
        );
    }
}

#[test]
fn burglary_disarm_ray_unlocks_chests_and_reveals_closed_secret_door() {
    let mut g = rogue();
    prepare_map(&mut g);
    let id = learn(&mut g, 1);
    let trap = Position { x: 11, y: 10 };
    let door = Position { x: 13, y: 10 };
    let i = g.index(trap).unwrap();
    g.terrain[i] = "demo.terrain.burglary-minor-trap".into();
    let i = g.index(door).unwrap();
    g.terrain[i] = "demo.terrain.door-secret".into();
    give_inventory_item(&mut g, "test.chest", "demo.item.large-wooden-chest");
    let chest = g.items.iter_mut().find(|i| i.id == "test.chest").unwrap();
    chest.location = ItemLocation::Ground(Position { x: 12, y: 10 });
    chest.chest = Some(rfb_protocol::ChestSaveDto {
        difficulty: 10,
        opening_depth: 10,
    });
    cast(
        &mut g,
        &id,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert_eq!(g.terrain_at(trap), "demo.terrain.floor");
    assert_eq!(g.terrain_at(door), "demo.terrain.door-closed");
    assert_eq!(
        g.items
            .iter()
            .find(|i| i.id == "test.chest")
            .unwrap()
            .chest
            .unwrap()
            .difficulty,
        -10
    );
    assert_eq!(
        g.state_hash(),
        Game::from_save(g.to_save()).unwrap().state_hash()
    );
}

#[test]
fn burglary_negotiation_saves_confirmation_and_charges_once() {
    let mut g = rogue();
    prepare_map(&mut g);
    let id = learn(&mut g, 10);
    target(&mut g, "demo.actor.novice-rogue");
    for _ in 0..100 {
        dispatch_next(
            &mut g,
            GameCommand::CastAbility {
                ability_id: id.clone(),
                target: TargetSelection::Entity {
                    entity_id: "test.target".into(),
                },
            },
        );
        if g.pending_duelist.is_some() {
            break;
        }
        for p in g.resources.values_mut() {
            p.current = p.maximum;
        }
    }
    let rfb_protocol::DuelistPromptDto::BurglaryNegotiate { cost, .. } =
        g.pending_duelist.as_ref().unwrap().prompt.clone().unwrap()
    else {
        panic!("negotiation prompt")
    };
    let balance = g.gold;
    let mut restored = Game::from_save(g.to_save()).unwrap();
    let command = GameCommand::ResolveDuelistChoice {
        choice: rfb_protocol::DuelistChoiceDto::Confirm { accepted: true },
    };
    dispatch_next(&mut g, command.clone());
    dispatch_next(&mut restored, command.clone());
    assert_eq!(g.gold, balance - cost);
    assert_eq!(g.state_hash(), restored.state_hash());
    let snapshot = g.snapshot();
    assert!(
        g.dispatch(super::support::command(
            snapshot.last_command_seq + 1,
            snapshot.revision,
            command
        ))
        .is_err()
    );
    assert_eq!(g.gold, balance - cost);
}

#[test]
fn burglary_panic_shot_spends_ammunition_and_assassination_requires_sleep() {
    let mut g = rogue();
    prepare_map(&mut g);
    let id = learn(&mut g, 18);
    target(&mut g, "demo.actor.novice-rogue");
    give_inventory_item(&mut g, "test.sling", "demo.item.sling");
    g.items
        .iter_mut()
        .find(|i| i.id == "test.sling")
        .unwrap()
        .location = ItemLocation::Equipped {
        slot_id: "shooting".into(),
    };
    give_inventory_item(&mut g, "test.ammo", "demo.item.iron-shot");
    cast(
        &mut g,
        &id,
        TargetSelection::Entity {
            entity_id: "test.target".into(),
        },
    );
    assert!(
        !g.items
            .iter()
            .any(|i| i.id == "test.ammo" && matches!(i.location, ItemLocation::Inventory))
    );
    prepare_map(&mut g);
    let id = learn(&mut g, 31);
    target(&mut g, "demo.actor.novice-rogue");
    let a = g.content.ability(&id).unwrap();
    let t = TargetSelection::Entity {
        entity_id: "test.target".into(),
    };
    assert!(g.ability_target_plan(a, &t).is_none());
    g.entities[0]
        .statuses
        .push(monster_combat::melee_status(STATUS_SLEEP, 100, "test.sleep").status);
    let before = g.entities[0].hp;
    cast(&mut g, &id, t);
    assert!(
        g.entities
            .iter()
            .find(|a| a.id == "test.target")
            .is_none_or(|a| a.hp < before)
    );
}

#[test]
fn burglary_birth_dexterity_capacity_and_thief_gloves_use_real_consumers() {
    let birth = Game::new_with_build(928, "demo.build.rogue").unwrap();
    assert_eq!(birth.progress.level, 1);
    assert!(
        birth
            .items
            .iter()
            .any(|i| i.kind_id == "demo.item.burglars-handbook")
    );
    let scrolls: u32 = birth
        .items
        .iter()
        .filter(|i| i.kind_id == "demo.item.farstep-scroll")
        .map(|i| i.quantity)
        .sum();
    assert!((1..=3).contains(&scrolls));
    assert!(birth.gold >= 402);
    let mut g = rogue();
    let high = g.resources.clone();
    g.progress.attributes.dexterity = 13;
    g.refresh_player_ability_state();
    assert!(
        g.resources
            .iter()
            .any(|(id, pool)| pool.maximum < high[id].maximum)
    );
    give_inventory_item(&mut g, "test.gloves", "demo.item.leather-gloves");
    let item = g.items.iter_mut().find(|i| i.id == "test.gloves").unwrap();
    item.affix_ids.push("rfb-legacy.affix.the-thief".into());
    let item = item.clone();
    assert!(g.item_has_glove_encumbrance(&item));
    g.equip_inventory_item("test.gloves", None).unwrap();
    g.refresh_player_ability_state();
    assert_eq!(
        g.state_hash(),
        Game::from_save(g.to_save()).unwrap().state_hash()
    );
}

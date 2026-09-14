// SPDX-License-Identifier: MPL-2.0
use super::existing_realms::prepared;
use super::support::{clear_monsters, give_inventory_item};
use super::*;

fn caster() -> Game {
    let mut g = prepared("demo.build.necromancer");
    clear_monsters(&mut g);
    g.player.position = Position { x: 10, y: 10 };
    for y in 5..=23 {
        for x in 5..=23 {
            let i = g.index(Position { x, y }).unwrap();
            g.terrain[i] = "demo.terrain.floor".into();
        }
    }
    g.glow.fill(true);
    g.reveal_current_visibility();
    g
}
fn learn(g: &mut Game, slot: usize) -> String {
    let book = [
        "stench-of-death",
        "sepulchral-ways",
        "return-of-the-dead",
        "necromatic-tome",
    ][slot / 8];
    let id = g
        .content
        .ability_book(&format!("demo.ability-book.{book}"))
        .unwrap()
        .ability_ids[slot % 8]
        .clone();
    let kind = format!("demo.item.{book}");
    let item = g
        .items
        .iter()
        .find(|i| i.kind_id == kind)
        .map(|i| i.id.clone())
        .unwrap_or_else(|| {
            let item = format!("test.{book}");
            give_inventory_item(g, &item, &kind);
            item
        });
    g.study_player_ability(&item, &id).unwrap();
    id
}
fn target(g: &mut Game, kind: &str) -> String {
    let mut a = g.generated_actor(
        "test.necromancy-target".into(),
        kind,
        Position { x: 11, y: 10 },
    );
    a.nice = true;
    let id = a.id.clone();
    g.entities.push(a);
    g.reveal_current_visibility();
    id
}
fn cast(g: &mut Game, id: &str, target: TargetSelection, success: bool) -> Vec<DomainEvent> {
    g.reveal_current_visibility();
    for r in g.resources.values_mut() {
        r.current = r.maximum;
    }
    for seed in 0..512 {
        let mut trial = g.clone();
        trial.rng = RfbRng::seeded(0x9e3779b97f4a7c15_u64.wrapping_mul(seed + 1));
        let saved = trial.to_save();
        let mut events = Vec::new();
        trial
            .resolve_player_ability(
                id,
                target.clone(),
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, DomainEvent::AbilityTargetUnavailable { .. })),
            "{id}: {events:?}"
        );
        let got = events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }));
        if got != success {
            continue;
        }
        if !got {
            assert!(
                events
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityCastFailed { .. })),
                "{id}: {events:?}"
            );
        }
        let mut restored = Game::from_save(saved).unwrap();
        restored
            .resolve_player_ability(
                id,
                target.clone(),
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(trial.state_hash(), restored.state_hash(), "{id}");
        assert_eq!(trial.rng, restored.rng);
        trial.reveal_current_visibility();
        Game::from_save(trial.to_save()).unwrap();
        *g = trial;
        return events;
    }
    panic!("no requested cast outcome {id}");
}

#[test]
fn necromancy_all_32_formal_spells_study_cast_and_resume() {
    for slot in 0..32 {
        let mut g = caster();
        let id = learn(&mut g, slot);
        let aim = if matches!(slot, 0 | 4 | 7 | 12 | 13 | 15 | 24 | 27 | 30) {
            TargetSelection::Entity {
                entity_id: target(&mut g, "demo.actor.war-bear"),
            }
        } else if slot == 11 {
            give_inventory_item(&mut g, "test.lore", "demo.item.dagger");
            TargetSelection::Item {
                item_id: "test.lore".into(),
            }
        } else {
            TargetSelection::SelfTarget
        };
        if slot == 15 {
            let pile = g
                .generate_gold_pile(Position { x: 12, y: 10 }, 10, false)
                .unwrap();
            g.gold_piles.push(pile);
        }
        let before = g.player_derived_stats();
        cast(&mut g, &id, aim, true);
        if matches!(slot, 0 | 4 | 7 | 13 | 27) {
            assert!(
                g.entities.is_empty() || g.entities[0].hp < g.entities[0].max_hp,
                "{id}"
            );
        }
        if matches!(slot, 1 | 5 | 8 | 14 | 16..=21 | 31) {
            assert!(!g.entities.is_empty(), "{id}");
            assert!(
                g.entities
                    .iter()
                    .all(|a| a.controller_id.as_deref() == Some(&g.player.id)),
                "{id}"
            );
        }
        if slot == 9 {
            assert!(g.player_has_status_kind("rfb.status.necromancy-cloak"));
            assert_eq!(
                g.player_derived_stats().stealth_skill.value - before.stealth_skill.value,
                13
            );
        }
        if slot == 15 {
            assert_eq!(
                g.terrain_at(Position { x: 12, y: 10 }),
                "demo.terrain.floor"
            );
            assert_eq!(g.terrain_at(Position { x: 12, y: 11 }), "demo.terrain.wall");
        }
        if slot == 26 {
            assert!(g.player_has_status_kind("rfb.status.necromancy-shield"));
            assert_eq!(
                g.player_derived_stats().armor_class.value - before.armor_class.value,
                50
            );
            assert_eq!(
                g.effective_player_resistances().level(DamageType::Nether),
                ResistanceLevel::Resistant
            );
        }
        if slot == 28 {
            assert!(g.player_has_status_kind("rfb.status.paralysis"));
        }
    }
}

#[test]
fn necromancy_failed_summons_are_reduced_hostile_and_not_tameable() {
    for slot in [1, 5, 8, 14, 16, 17, 18, 19, 20, 21] {
        let mut g = caster();
        let id = learn(&mut g, slot);
        cast(&mut g, &id, TargetSelection::SelfTarget, false);
        assert_eq!(g.entities.len(), 1, "{id}");
        assert!(g.entities[0].no_pet);
        assert!(g.entities[0].controller_id.is_none());
    }
}

#[test]
fn necromancy_touch_rejects_empty_distant_gloved_two_handed_and_frightened_targets_without_rng() {
    use crate::game::monster_combat::melee_status;
    let mut g = caster();
    let id = learn(&mut g, 0);
    let ability = g.content.ability(&id).unwrap().clone();
    assert!(
        g.ability_target_plan(
            &ability,
            &TargetSelection::Direction {
                direction: Direction::East
            }
        )
        .is_none()
    );
    let entity = target(&mut g, "demo.actor.war-bear");
    let aim = TargetSelection::Entity { entity_id: entity };
    assert!(g.ability_target_plan(&ability, &aim).is_some());
    g.entities[0].position.x = 12;
    assert!(g.ability_target_plan(&ability, &aim).is_none());
    g.entities[0].position.x = 11;
    let mut fear = g.clone();
    fear.player
        .statuses
        .push(melee_status("rfb.status.fear", 10, "test.fear").status);
    assert!(fear.ability_target_plan(&ability, &aim).is_none());
    for kind in ["demo.item.leather-gloves", "demo.item.quarterstaff"] {
        let mut equipped = g.clone();
        let definition = equipped.content.item(kind).unwrap();
        let slot = definition.equipment_slot.clone().unwrap();
        give_inventory_item(&mut equipped, "test.equipment", kind);
        let slot = equipped
            .body_slots
            .iter()
            .find(|s| s.slot_type == slot)
            .unwrap()
            .id
            .clone();
        equipped
            .items
            .iter_mut()
            .find(|i| i.id == "test.equipment")
            .unwrap()
            .location = ItemLocation::Equipped { slot_id: slot };
        assert!(
            equipped.ability_target_plan(&ability, &aim).is_none(),
            "{kind}"
        );
    }
    let rng = g.rng.clone();
    g.ability_target_plan(&ability, &aim);
    assert_eq!(g.rng, rng);
}

#[test]
fn necromancy_repose_resumes_sleep_and_restores_attributes_without_filling_hp_or_mana() {
    let mut g = caster();
    let id = learn(&mut g, 28);
    g.player.hp = 20;
    g.progress.attributes.strength = g.progress.maximum_attributes.strength - 2;
    cast(&mut g, &id, TargetSelection::SelfTarget, true);
    let hp = g.player.hp;
    let mana = g.resources["demo.resource.mana"].current;
    let mut restored = Game::from_save(g.to_save()).unwrap();
    for _ in 0..9 {
        for game in [&mut g, &mut restored] {
            super::support::dispatch_next(game, GameCommand::Wait);
        }
        assert_eq!(g.state_hash(), restored.state_hash());
        if !g.player_has_status_kind("rfb.status.paralysis") {
            break;
        }
    }
    assert!(!g.player_has_status_kind("rfb.status.paralysis"));
    assert_eq!(
        g.progress.attributes.strength,
        g.progress.maximum_attributes.strength
    );
    assert!(g.player.hp < g.effective_player_max_hp());
    assert!(g.player.hp >= hp);
    assert!(g.resources["demo.resource.mana"].current < g.resources["demo.resource.mana"].maximum);
    assert!(g.resources["demo.resource.mana"].current >= mana);
}

#[test]
fn necromancy_birth_intrinsics_destroy_holy_books_and_class_powers_use_real_class() {
    let mut g = caster();
    assert!(g.player_is_necromancer());
    assert!(g.player_hold_life_sources() >= 2);
    assert!(g.player_see_invisible_sources() >= 1);
    assert!(
        g.items
            .iter()
            .any(|i| i.kind_id == "demo.item.stench-of-death")
    );
    for (kind, amount) in [
        ("demo.item.book-of-common-prayer", 10),
        ("demo.item.high-mass", 25),
    ] {
        give_inventory_item(&mut g, "test.holy", kind);
        g.resources.get_mut("demo.resource.mana").unwrap().current = 0;
        g.destroy_item("test.holy", 1).unwrap();
        assert_eq!(g.resources["demo.resource.mana"].current, amount);
    }
    for id in [
        "demo.ability.necromancer-animate-dead",
        "demo.ability.necromancer-enslave-undead",
    ] {
        let aim = if id.ends_with("enslave-undead") {
            TargetSelection::Entity {
                entity_id: target(&mut g, "demo.actor.risen-thrall"),
            }
        } else {
            TargetSelection::SelfTarget
        };
        cast(&mut g, id, aim, true);
    }
}

#[test]
fn necromancy_books_generate_from_ordinary_pool_and_are_studied_after_pickup() {
    for (rank, book) in [
        "stench-of-death",
        "sepulchral-ways",
        "return-of-the-dead",
        "necromatic-tome",
    ]
    .iter()
    .enumerate()
    {
        let mut g = caster();
        let kind = format!("demo.item.{book}");
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: g.current_floor_id.clone(),
            depth: [20, 30, 55, 85][rank],
            source: LootSource::ItemUse {
                item_id: "test.necromancy-pool".into(),
            },
        };
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == kind)
            })
            .expect("book in actual ordinary pool");
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
        g.reveal_current_visibility();
        Game::from_save(g.to_save()).unwrap();
    }
}

#[test]
fn necromancy_unholy_word_repelling_and_mana_touch_affect_actual_targets() {
    let mut g = caster();
    let id = learn(&mut g, 22);
    target(&mut g, "demo.actor.skeleton-kobold");
    g.entities[0].controller_id = Some(g.player.id.clone());
    g.entities[0].hp = 1;
    g.entities[0]
        .statuses
        .push(crate::game::monster_combat::melee_status("rfb.status.confusion", 10, "test").status);
    cast(&mut g, &id, TargetSelection::SelfTarget, true);
    assert!(g.entities[0].hp > 1);
    assert!(
        g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == "rfb.status.haste")
    );
    assert!(
        !g.entities[0]
            .statuses
            .iter()
            .any(|s| s.kind_id == "rfb.status.confusion")
    );
    let mut g = caster();
    let id = learn(&mut g, 12);
    let aim = TargetSelection::Entity {
        entity_id: target(&mut g, "demo.actor.war-bear"),
    };
    let hp = g.entities[0].hp;
    cast(&mut g, &id, aim, true);
    assert_eq!(g.entities[0].position, Position { x: 21, y: 10 });
    assert_eq!(g.entities[0].hp, hp);
    for magical in [false, true] {
        let mut g = caster();
        let id = learn(&mut g, 24);
        let kind = if magical {
            g.content
                .actor_definitions()
                .find(|a| {
                    a.role == rfb_content::ActorRole::Monster
                        && a.monster_casting
                            .as_ref()
                            .is_some_and(|c| c.abilities.iter().any(|a| !a.innate))
                        && a.max_hp > 100
                        && !a.tags.iter().any(|t| t == "unique")
                })
                .unwrap()
                .id
                .clone()
        } else {
            "demo.actor.war-bear".into()
        };
        let aim = TargetSelection::Entity {
            entity_id: target(&mut g, &kind),
        };
        let hp = g.entities[0].hp;
        cast(&mut g, &id, aim, true);
        if magical {
            assert!(g.entities.is_empty() || g.entities[0].hp < hp);
            assert_eq!(
                g.resources["demo.resource.mana"].current,
                g.resources["demo.resource.mana"].maximum
            );
        } else {
            assert_eq!(g.entities[0].hp, hp);
            assert!(
                g.resources["demo.resource.mana"].current
                    < g.resources["demo.resource.mana"].maximum
            );
        }
    }
}

#[test]
#[ignore = "controlled preparation for ordinary standalone Necromancy acceptance"]
fn export_necromancy_desktop_saves() {
    use super::support::dispatch_next;
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, _) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    let mut scenarios = Vec::new();
    for (name, slot) in [
        ("first-book", 0),
        ("pets", 1),
        ("failed-summon", 1),
        ("lore", 11),
        ("shield", 26),
    ] {
        let mut base = caster();
        let ability = if slot == 0 {
            "demo.ability.necromancy-cold-touch".into()
        } else {
            learn(&mut base, slot)
        };
        base.apply_player_melee_status(STATUS_INVULNERABILITY, 1000, "test.necromancy-desktop");
        let aim = if slot == 0 {
            target(&mut base, "demo.actor.war-bear");
            TargetSelection::Direction {
                direction: Direction::East,
            }
        } else if slot == 1 {
            TargetSelection::Position {
                position: base.player.position,
            }
        } else if slot == 11 {
            give_inventory_item(&mut base, "test.necromancy-lore", "demo.item.dagger");
            TargetSelection::Item {
                item_id: "test.necromancy-lore".into(),
            }
        } else {
            TargetSelection::SelfTarget
        };
        for r in base.resources.values_mut() {
            r.current = r.maximum;
        }
        base.reveal_current_visibility();
        let (start, steps) = (0..4096)
            .find_map(|seed| {
                let mut game = base.clone();
                game.rng = RfbRng::seeded(0x9e3779b97f4a7c15_u64.wrapping_mul(seed + 1));
                let start = Game::from_save(game.to_save()).unwrap();
                let mut steps = Vec::new();
                if slot == 0 {
                    let book = game
                        .items
                        .iter()
                        .find(|i| i.kind_id == "demo.item.stench-of-death")
                        .unwrap()
                        .id
                        .clone();
                    let command = GameCommand::StudyAbility {
                        book_item_id: book,
                        ability_id: ability.clone(),
                    };
                    dispatch_next(&mut game, command.clone());
                    steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                }
                let command = GameCommand::CastAbility {
                    ability_id: ability.clone(),
                    target: aim.clone(),
                };
                let update = game
                    .dispatch(GameCommandEnvelope {
                        expected_revision: game.revision,
                        command_seq: game.last_command_seq + 1,
                        command: command.clone(),
                    })
                    .ok()?;
                let expected = if name == "failed-summon" {
                    "ability-cast-failure"
                } else {
                    "ability-cast-success"
                };
                if !update.events.iter().any(|e| e.message_key == expected)
                    || (slot == 1 && game.entities.is_empty())
                {
                    return None;
                }
                steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                dispatch_next(&mut game, GameCommand::Wait);
                steps.push(
                    serde_json::json!({"command":GameCommand::Wait,"hash":game.state_hash()}),
                );
                Game::from_save(game.to_save()).unwrap();
                Some((start, steps))
            })
            .expect("production commands");
        std::fs::write(
            directory.join(format!("{name}.rfbsave")),
            rfb_save::encode(&header, &start.to_save()).unwrap(),
        )
        .unwrap();
        scenarios
            .push(serde_json::json!({"name":name,"initialHash":start.state_hash(),"steps":steps}));
    }
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&scenarios).unwrap(),
    )
    .unwrap();
}

#[test]
fn necromancy_deadly_touch_uses_two_intelligence_saves_and_nonliving_immunity() {
    for kind in ["demo.actor.war-bear", "demo.actor.skeleton-kobold"] {
        let mut base = caster();
        let id = learn(&mut base, 30);
        let entity = target(&mut base, kind);
        let ability = base.content.ability(&id).unwrap().clone();
        let aim = TargetSelection::Entity { entity_id: entity };
        let mut killed = false;
        let mut resisted = false;
        for seed in 0..64 {
            let mut g = base.clone();
            g.rng = RfbRng::seeded(0x9e3779b97f4a7c15_u64.wrapping_mul(seed + 1));
            let mut oracle = g.clone();
            let immune = kind == "demo.actor.skeleton-kobold";
            let saves = immune
                || oracle.monster_saves_against_attribute(0, AttributeKind::Intelligence)
                || oracle.monster_saves_against_attribute(0, AttributeKind::Intelligence);
            let plan = g.ability_target_plan(&ability, &aim).unwrap();
            g.resolve_player_ability_effect(
                ability.clone(),
                plan,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(g.entities.is_empty(), !saves, "{kind}, seed {seed}");
            killed |= !saves;
            resisted |= saves;
        }
        assert!(resisted);
        assert_eq!(killed, kind == "demo.actor.war-bear");
    }
}

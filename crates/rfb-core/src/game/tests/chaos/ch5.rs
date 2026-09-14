// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::existing_realms::prepared;
use crate::game::tests::support::{dispatch_next, reward_ready};
use rfb_protocol::FacilityMembershipDto;

fn arena(game: &mut Game) {
    clear_monsters(game);
    game.player.position = Position { x: 10, y: 10 };
    for y in 6..=17 {
        for x in 6..=17 {
            let i = game.index(Position { x, y }).unwrap();
            game.terrain[i] = "demo.terrain.floor".into();
        }
    }
    game.glow.fill(true);
}

#[test]
fn ch5_all_formal_builds_generate_study_cast_and_resume() {
    let content = load_built_in_content().unwrap();
    let builds: Vec<_> = content
        .builds()
        .filter(|b| {
            b.first_realm_id.as_deref() == Some("chaos")
                || b.second_realm_id.as_deref() == Some("chaos")
        })
        .map(|b| b.id.clone())
        .collect();
    assert_eq!(builds.len(), 27);
    for build in builds {
        let birth = Game::new_with_build(925, &build).unwrap();
        assert!(birth.items.iter().any(|i| i.kind_id == BOOK));
        assert!(birth.learned_abilities.is_empty());
        Game::from_save(birth.to_save()).unwrap();
        let mut game = prepared(&build);
        arena(&mut game);
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: game.current_floor_id.clone(),
            depth: 20,
            source: LootSource::ItemUse {
                item_id: "test.chaos-book-generation".into(),
            },
        };
        let draft = (0..32768)
            .find_map(|_| {
                game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == BOOK)
            })
            .expect("current Chaos book must actually generate");
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        let item_id = item.id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&item_id)).unwrap();
        assert!(game.item_knowledge[BOOK].found_count > 0);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .map(|d| d.kind_id),
            restored
                .generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .map(|d| d.kind_id)
        );
        assert_eq!(game.rng, restored.rng);
        let book = game
            .items
            .iter()
            .find(|i| i.kind_id == BOOK && i.location == ItemLocation::Inventory)
            .unwrap()
            .id
            .clone();
        let spell = "demo.ability.chaos-magic-missile";
        if build.contains("priest-") || build.contains("ranger-") {
            // Real random prayers, with shared learning capacity; do not substitute chosen study.
            for _ in 0..8 {
                if game.learned_abilities.contains(spell) {
                    break;
                }
                dispatch_next(
                    &mut game,
                    GameCommand::StudyPrayer {
                        book_item_id: book.clone(),
                    },
                );
            }
        } else {
            game.study_player_ability(&book, spell).unwrap();
        }
        assert!(game.learned_abilities.contains(spell));
        let actor = game.generated_actor(
            "test.ch5-target".into(),
            "demo.actor.war-bear",
            Position { x: 11, y: 10 },
        );
        let hp = actor.hp;
        game.entities.push(actor);
        cast_saved(
            &mut game,
            spell,
            TargetSelection::Direction {
                direction: Direction::East,
            },
        );
        assert!(
            game.entities
                .iter()
                .find(|a| a.id == "test.ch5-target")
                .is_none_or(|a| a.hp < hp)
        );
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn ch5_realm_change_updates_chaos_access_and_saved_learning() {
    for (realm, book, tower) in [
        ("chaos", BOOK, "demo.town-facility.zul-chaos-tower"),
        (
            "trump",
            "demo.item.conjurings-and-tricks",
            "demo.town-facility.angwil-trump-tower",
        ),
    ] {
        for build in [
            "mage-life-arcane",
            "priest-life-arcane",
            "priest-death-arcane",
            "warrior-mage-arcane-life",
            "ranger-nature-sorcery",
        ] {
            let mut game = prepared(&format!("demo.build.{build}"));
            let original = game.current_second_realm_id().unwrap().to_owned();
            let old_book = game
                .items
                .iter()
                .find(|i| {
                    game.content
                        .item(&i.kind_id)
                        .and_then(|d| d.ability_book_id.as_deref())
                        .and_then(|id| game.content.ability_book(id))
                        .is_some_and(|b| b.realm_id.as_deref() == Some(&original))
                })
                .unwrap()
                .id
                .clone();
            give_inventory_item(&mut game, "test.chaos", book);
            dispatch_next(
                &mut game,
                GameCommand::BeginRealmChange {
                    book_item_id: "test.chaos".into(),
                },
            );
            let pending = game.to_save();
            let mut cancelled = Game::from_save(pending.clone()).unwrap();
            dispatch_next(
                &mut cancelled,
                GameCommand::ResolveRealmChange { confirm: false },
            );
            assert_eq!(cancelled.current_second_realm_id(), Some(original.as_str()));
            let mut restored = Game::from_save(pending).unwrap();
            for g in [&mut game, &mut restored] {
                dispatch_next(g, GameCommand::ResolveRealmChange { confirm: true });
            }
            assert_eq!(game.state_hash(), restored.state_hash());
            assert_eq!(game.current_second_realm_id(), Some(realm));
            crate::game::tests::town::enter_town_facility(&mut game, tower);
            let snapshot = game.snapshot();
            assert_eq!(
                snapshot
                    .task_services
                    .iter()
                    .find(|s| s.id == tower)
                    .unwrap()
                    .membership,
                FacilityMembershipDto::Owner
            );
            dispatch_next(
                &mut game,
                GameCommand::BeginRealmChange {
                    book_item_id: old_book,
                },
            );
            dispatch_next(&mut game, GameCommand::ResolveRealmChange { confirm: true });
            assert_eq!(game.current_second_realm_id(), Some(original.as_str()));
            assert!(game.active_casting_book_ids().iter().all(|id| {
                *id != game
                    .content
                    .item(book)
                    .unwrap()
                    .ability_book_id
                    .as_deref()
                    .unwrap()
            }));
            Game::from_save(game.to_save()).unwrap();
        }
    }
}

#[test]
fn ch5_existing_node_reward_is_claimed_once_and_can_be_studied_and_cast() {
    // The node lifecycle has its own real-map tests. Here only reward delivery and its new book link matter.
    let (mut game, task, facility, item) =
        reward_ready(925, "demo.build.high-mage-chaos", "zul-chaos-node");
    game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    assert!(game.claim_task_reward(&facility, &task).is_ok());
    assert!(game.claim_task_reward(&facility, &task).is_err());
    assert_eq!(
        game.items.iter().find(|i| i.id == item).unwrap().kind_id,
        "demo.item.armageddon-tome"
    );
    game = Game::from_save(game.to_save()).unwrap();
    game.study_player_ability(&item, "demo.ability.chaos-gravity-beam")
        .unwrap();
    arena(&mut game);
    let actor = game.generated_actor(
        "test.ch5-reward-target".into(),
        "demo.actor.war-bear",
        Position { x: 11, y: 10 },
    );
    let hp = actor.hp;
    game.entities.push(actor);
    cast_saved(
        &mut game,
        "demo.ability.chaos-gravity-beam",
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert!(
        game.entities
            .iter()
            .find(|a| a.id == "test.ch5-reward-target")
            .is_none_or(|a| a.hp < hp)
    );
    Game::from_save(game.to_save()).unwrap();
}

#[test]
#[ignore = "controlled preparation for ordinary standalone Chaos acceptance"]
fn export_chaos_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, _) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    let mut scenarios = Vec::new();
    for (name, slug) in [
        ("first-book", "magic-missile"),
        ("wonder", "wonder"),
        ("recharge", "arcane-binding"),
        ("branding", "chaos-branding"),
        ("mimic", "polymorph-self"),
        ("meteor", "meteor-swarm"),
        ("pending-chaos", "call-chaos"),
        ("void", "call-the-void"),
    ] {
        let mut base = prepared("demo.build.high-mage-chaos");
        arena(&mut base);
        let ability = if name == "first-book" {
            "demo.ability.chaos-magic-missile".to_owned()
        } else {
            learn(&mut base, slug)
        };
        base.apply_player_melee_status(STATUS_INVULNERABILITY, 1000, "test.chaos-desktop");
        let target = if matches!(name, "recharge" | "branding") {
            let kind = if name == "recharge" {
                "demo.item.detect-objects-staff"
            } else {
                "demo.item.dagger"
            };
            give_inventory_item(&mut base, "test.chaos-item", kind);
            if name == "recharge" {
                base.items
                    .iter_mut()
                    .find(|i| i.id == "test.chaos-item")
                    .unwrap()
                    .charges
                    .as_mut()
                    .unwrap()
                    .current = 0;
            }
            TargetSelection::Item {
                item_id: "test.chaos-item".into(),
            }
        } else if matches!(name, "first-book" | "wonder") {
            let actor = base.generated_actor(
                "test.chaos-desktop-target".into(),
                "demo.actor.war-bear",
                Position { x: 11, y: 10 },
            );
            base.entities.push(actor);
            TargetSelection::Direction {
                direction: Direction::East,
            }
        } else {
            TargetSelection::SelfTarget
        };
        for resource in base.resources.values_mut() {
            resource.current = resource.maximum;
        }
        let (start, steps) = (0..4096)
            .find_map(|seed| {
                let mut game = base.clone();
                game.rng = RfbRng::seeded(seed);
                let start = game.clone();
                let mut steps = Vec::new();
                if name == "first-book" {
                    let book = game
                        .items
                        .iter()
                        .find(|i| i.kind_id == BOOK)
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
                    target: target.clone(),
                };
                let update = game
                    .dispatch(GameCommandEnvelope {
                        expected_revision: game.revision,
                        command_seq: game.last_command_seq + 1,
                        command: command.clone(),
                    })
                    .ok()?;
                if !update
                    .events
                    .iter()
                    .any(|e| e.message_key == "ability-cast-success")
                {
                    return None;
                }
                if game.pending_ability_glyph.is_some() {
                    return None;
                }
                if name == "pending-chaos" && game.pending_ability_direction.is_none() {
                    return None;
                }
                if name == "recharge"
                    && game
                        .items
                        .iter()
                        .find(|i| i.id == "test.chaos-item")
                        .is_none_or(|i| i.charges.unwrap().current == 0)
                {
                    return None;
                }
                if name == "branding"
                    && game
                        .items
                        .iter()
                        .find(|i| i.id == "test.chaos-item")
                        .is_none_or(|i| i.affix_ids.is_empty())
                {
                    return None;
                }
                if name == "mimic"
                    && game.character_definitions().unwrap().1.legacy_index != Some(51)
                {
                    return None;
                }
                Game::from_save(game.to_save()).unwrap();
                steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                if game.pending_ability_direction.is_some() {
                    let command = GameCommand::ResolveAbilityDirection {
                        direction: Direction::East,
                    };
                    dispatch_next(&mut game, command.clone());
                    steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                }
                let waits = if name == "mimic" { 100 } else { 1 };
                for _ in 0..waits {
                    dispatch_next(&mut game, GameCommand::Wait);
                    steps.push(
                        serde_json::json!({"command":GameCommand::Wait,"hash":game.state_hash()}),
                    );
                    if name == "mimic"
                        && game
                            .player
                            .statuses
                            .iter()
                            .all(|s| s.granted_race_id.is_none())
                    {
                        break;
                    }
                }
                if name == "mimic" {
                    assert!(
                        game.player
                            .statuses
                            .iter()
                            .all(|s| s.granted_race_id.is_none())
                    );
                }
                Game::from_save(game.to_save()).unwrap();
                Some((start, steps))
            })
            .expect("successful real Chaos command sequence");
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

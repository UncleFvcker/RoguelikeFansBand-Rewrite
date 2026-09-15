// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
#[ignore = "explicit Q2–Q5 preparation for ordinary standalone UI acceptance"]
fn export_quest_item_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut base = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    choose_human_talent_if_pending(&mut base);
    base.apply_player_experience(base.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut base);
    base.apply_player_melee_status(STATUS_INVULNERABILITY, 200_000, "test.quest.desktop");
    let mut scenarios = Vec::new();
    for (name, actor, slug) in [
        ("q2-eyes", "omarax-the-eye-tyrant", "eyes"),
        ("q3-hydra", "the-lernean-hydra", "eye-of-the-hydra"),
        ("q4-sting", "", "sting"),
        ("q5-rama", "rama-the-exiled-prince", "rama"),
    ] {
        let mut game = base.clone();
        let kind = format!("demo.item.{slug}");
        if actor.is_empty() {
            clear_monsters(&mut game);
            dispatch_next(
                &mut game,
                GameCommand::EnterWorldMap {
                    cancel_recall: false,
                },
            );
            game.wilderness_position = Some(Position { x: 87, y: 49 });
            dispatch_next(&mut game, GameCommand::LeaveWorldMap);
            clear_monsters(&mut game);
            game.player.position = game
                .town_local_to_wilderness_view_position(
                    "demo.town.telmora",
                    Position { x: 41, y: 21 },
                )
                .unwrap();
            dispatch_next(
                &mut game,
                GameCommand::AcceptTask {
                    facility_id: "demo.town-facility.telmora-castle".into(),
                    task_id: "demo.task.telmora-vault".into(),
                },
            );
            game.player.position = game
                .town_local_to_wilderness_view_position(
                    "demo.town.telmora",
                    Position { x: 133, y: 14 },
                )
                .unwrap();
            dispatch_next(&mut game, GameCommand::TraverseStairs);
            assert_eq!(game.current_floor_id, "demo.floor.telmora-vault");
        } else {
            prepare_combat(&mut game, &format!("demo.actor.{actor}"));
            game = successful_kill_start(&game, &kind);
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
        }
        let item = game
            .items
            .iter()
            .find(|item| item.kind_id == kind)
            .unwrap()
            .clone();
        let ItemLocation::Ground(position) = item.location else {
            panic!("real source reward")
        };
        // Explicit post-acquisition scene preparation. Reward identity and its
        // actual source death/map placement are retained, never synthesized.
        clear_monsters(&mut game);
        game.items.retain(|candidate| candidate.id == item.id);
        game.player.position = position;
        if slug == "rama" {
            give_inventory_item(&mut game, "test.quest.desktop.arrow", "demo.item.arrow");
            game.items
                .iter_mut()
                .find(|item| item.id == "test.quest.desktop.arrow")
                .unwrap()
                .quantity = 10;
        }
        game.reveal_current_visibility();
        let mut commands = vec![
            GameCommand::PickUp,
            GameCommand::Equip {
                item_id: item.id.clone(),
                slot_id: None,
            },
        ];
        if slug != "sting" {
            commands.push(GameCommand::UseItem {
                item_id: item.id.clone(),
                target: Some(if slug == "rama" {
                    TargetSelection::Position {
                        position: Position {
                            x: position.x + 1,
                            y: position.y,
                        },
                    }
                } else {
                    TargetSelection::SelfTarget
                }),
            });
        }
        commands.push(GameCommand::Wait);
        if slug != "sting" {
            let seed = (0..1000)
                .find(|seed| {
                    let mut candidate = game.clone();
                    candidate.rng = RfbRng::seeded(*seed);
                    for command in &commands {
                        dispatch_next(&mut candidate, command.clone());
                    }
                    candidate
                        .items
                        .iter()
                        .find(|candidate| candidate.id == item.id)
                        .unwrap()
                        .charges
                        .unwrap()
                        .current
                        == 0
                })
                .unwrap();
            game.rng = RfbRng::seeded(seed);
        }
        let start = game.clone();
        let mut steps = Vec::new();
        for command in commands {
            let activation_slot = game
                .items
                .iter()
                .find(|candidate| candidate.id == item.id)
                .and_then(|item| match &item.location {
                    ItemLocation::Equipped { slot_id } => Some(slot_id.clone()),
                    _ => None,
                });
            dispatch_next(&mut game, command.clone());
            assert_eq!(
                game.state_hash(),
                Game::from_save(game.to_save(), game.behavior_preferences())
                    .unwrap()
                    .state_hash()
            );
            steps.push(serde_json::json!({"command":command,"hash":game.state_hash(),"activationSlot":activation_slot}));
        }
        if slug == "sting" {
            assert_eq!(game.task_states["demo.task.telmora-vault"].current, 1);
        }
        assert_eq!(
            start.state_hash(),
            Game::from_save(start.to_save(), start.behavior_preferences())
                .unwrap()
                .state_hash()
        );
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

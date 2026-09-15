// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
#[ignore = "explicit item selection preparation for ordinary standalone acceptance"]
fn export_item_selection_desktop_save() {
    let input = std::path::PathBuf::from(std::env::var("ITEM_SELECTION_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut game = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 99, y: 33 };
    for y in 30..=36 {
        for x in 95..=103 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    for (id, kind) in [
        ("is.bag", "dwarven-backpack"),
        ("is.quiver", "quiver"),
        ("is.weapon", "dagger"),
    ] {
        give_inventory_item(&mut game, id, &format!("demo.item.{kind}"));
        assert!(game.equip_inventory_item(id, None).is_some());
    }
    for index in 0..28 {
        give_inventory_item(
            &mut game,
            &format!("is.pack.{index:02}"),
            "demo.item.dagger",
        );
        game.items.last_mut().unwrap().inscription = Some(match index {
            0 => "@I3".into(),
            1 => "@I3 @I4".into(),
            2 => "@IA".into(),
            _ => format!("验收物品 {index}"),
        });
    }
    for (id, kind, quantity, inscription) in [
        ("is.arrows", "arrow", 50, "箭袋"),
        ("is.overflow", "arrow", 80, "背包溢出"),
        ("is.potion", "water-potion", 5, "@q1 !q"),
        ("is.staff", "identify-staff", 1, "@u1"),
        ("is.floor", "dagger", 1, "地面目标"),
    ] {
        give_inventory_item(&mut game, id, &format!("demo.item.{kind}"));
        let item = game.items.last_mut().unwrap();
        item.quantity = quantity;
        item.inscription = Some(inscription.into());
        if id == "is.floor" {
            item.location = ItemLocation::Ground(game.player.position);
        }
    }
    for id in ["is.staff", "is.potion", "is.bag", "is.quiver"] {
        game.identify_item_instance(id, ItemIdentificationRequest::new(true));
    }
    game.glow.fill(true);
    game.reveal_current_visibility();
    assert_eq!(game.snapshot().player.quiver_item_ids, ["is.arrows"]);
    assert_eq!(
        game.state_hash(),
        Game::from_save(game.to_save(), game.behavior_preferences())
            .unwrap()
            .state_hash()
    );
    let initial = game.clone();
    let mut steps = Vec::new();
    for command in [
        GameCommand::UseItem {
            item_id: "is.staff".into(),
            target: None,
        },
        GameCommand::UseItem {
            item_id: "is.potion".into(),
            target: None,
        },
        GameCommand::UseItem {
            item_id: "is.staff".into(),
            target: Some(rfb_protocol::TargetSelection::Item {
                item_id: "is.floor".into(),
            }),
        },
        GameCommand::DropQuantity {
            item_id: "is.potion".into(),
            quantity: 2,
        },
        GameCommand::DropQuantity {
            item_id: "is.potion".into(),
            quantity: 2,
        },
        GameCommand::DropQuantity {
            item_id: "is.potion".into(),
            quantity: 2,
        },
    ] {
        let update = dispatch_next(&mut game, command.clone());
        assert_eq!(
            game.state_hash(),
            Game::from_save(game.to_save(), game.behavior_preferences())
                .unwrap()
                .state_hash()
        );
        steps.push(serde_json::json!({ "command": command, "hash": game.state_hash(), "events": update.events }));
    }
    std::fs::write(
        directory.join("prepared.rfbsave"),
        rfb_save::encode(&header, &initial.to_save()).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("scenario.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "initialHash": initial.state_hash(), "steps": steps,
        "preparation": "Fresh human Warrior save; cleared/lit local patch; equipped ordinary dwarven backpack, quiver and dagger; 28 separate daggers with inscriptions, 50 quivered and 80 overflow arrows, 5 water potions, identified identification staff and a floor dagger. No natural acquisition claim."
    })).unwrap()).unwrap();
}

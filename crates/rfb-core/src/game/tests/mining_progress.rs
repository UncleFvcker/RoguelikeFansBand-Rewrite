// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn only_player_digging_that_removes_a_vein_trains_mining() {
    let mut game = Game::new(0x4d49_4e45);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(&mut game, "test.pick", "demo.item.orcish-pick");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.pick".to_owned(),
            slot_id: Some("tool".to_owned()),
        },
    );
    game.progress.mining_proficiency = 3_999;
    let position = game.position_in_direction(Direction::North);

    replace_terrain(&mut game, position, "demo.terrain.rubble");
    for _ in 0..100 {
        dispatch_next(
            &mut game,
            GameCommand::DigTerrain {
                direction: Direction::North,
            },
        );
        if game.terrain[game.index(position).expect("terrain index")] == "demo.terrain.floor" {
            break;
        }
    }
    assert_eq!(game.progress.mining_proficiency, 3_999);

    replace_terrain(&mut game, position, "demo.terrain.magma-vein");
    let update = loop {
        let update = dispatch_next(
            &mut game,
            GameCommand::DigTerrain {
                direction: Direction::North,
            },
        );
        if game.terrain[game.index(position).expect("terrain index")] == "demo.terrain.floor" {
            break update;
        }
    };
    assert_eq!(game.progress.mining_proficiency, 4_012);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "mining-proficiency-improved")
    );
}

#[test]
fn mining_and_sparse_materials_project_and_round_trip_strictly() {
    let mut game = Game::new(0x4d41_5453);
    game.progress.mining_proficiency = 6_000;
    game.progress
        .materials
        .insert("rfb.material.iron-ore".to_owned(), 3);
    game.progress
        .materials
        .insert("rfb.material.rare-catalyst".to_owned(), 1);

    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.progress.mining_proficiency.current, 6_000);
    assert_eq!(
        snapshot.player.progress.mining_proficiency.rank,
        rfb_protocol::ProficiencyRankDto::Skilled
    );
    assert_eq!(snapshot.player.progress.materials.len(), 10);
    assert_eq!(
        snapshot
            .player
            .progress
            .materials
            .iter()
            .find(|material| material.material_id == "rfb.material.iron-ore")
            .expect("iron ore should be projected")
            .quantity,
        3
    );

    let saved = game.to_save();
    let progress = saved
        .player
        .progress
        .as_ref()
        .expect("new save should include progress");
    assert_eq!(progress.mining_proficiency, 6_000);
    assert_eq!(progress.materials.len(), 2);
    let restored = Game::from_save(saved.clone()).expect("mining state should round-trip");
    assert_eq!(restored.progress.mining_proficiency, 6_000);
    assert_eq!(restored.progress.materials, game.progress.materials);

    let mut excessive = saved.clone();
    excessive
        .player
        .progress
        .as_mut()
        .expect("progress")
        .mining_proficiency = 8_001;
    assert!(matches!(
        Game::from_save(excessive),
        Err(CoreError::InvalidSave(
            "player mining or material state is invalid"
        ))
    ));

    for material_id in ["rfb.material.unknown", "rfb.material.iron-ore"] {
        let mut invalid = saved.clone();
        invalid
            .player
            .progress
            .as_mut()
            .expect("progress")
            .materials = vec![rfb_protocol::MaterialSaveDto {
            material_id: material_id.to_owned(),
            quantity: if material_id == "rfb.material.iron-ore" {
                0
            } else {
                1
            },
        }];
        assert!(matches!(
            Game::from_save(invalid),
            Err(CoreError::InvalidSave(
                "player mining or material state is invalid"
            ))
        ));
    }

    let mut duplicate = saved;
    duplicate
        .player
        .progress
        .as_mut()
        .expect("progress")
        .materials = vec![
        rfb_protocol::MaterialSaveDto {
            material_id: "rfb.material.iron-ore".to_owned(),
            quantity: 1,
        },
        rfb_protocol::MaterialSaveDto {
            material_id: "rfb.material.iron-ore".to_owned(),
            quantity: 2,
        },
    ];
    assert!(matches!(
        Game::from_save(duplicate),
        Err(CoreError::InvalidSave("player material state is invalid"))
    ));
}

#[test]
fn mining_and_material_save_fields_are_required() {
    for field in ["miningProficiency", "materials"] {
        let mut value =
            serde_json::to_value(Game::new(0x5354_5249_4354).to_save()).expect("serialize save");
        value["player"]["progress"]
            .as_object_mut()
            .expect("progress should be an object")
            .remove(field);
        assert!(serde_json::from_value::<rfb_protocol::SavePayloadV1>(value).is_err());
    }
}

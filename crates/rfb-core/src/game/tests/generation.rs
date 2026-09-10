// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use super::*;

#[test]
fn ordinary_room_and_anywhere_allocations_reach_pickup_and_save() {
    let mut game = Game::new_with_build(617, "demo.build.warrior").unwrap();
    let mut definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    definition.vault_id = None;
    definition.guaranteed_items.clear();
    let area = u32::from(definition.width) * u32::from(definition.height);
    // One placement from each caller, retaining the formal shared pool.
    definition.loot_allocation = Some(rfb_content::ProceduralLootAllocationDefinition {
        reference_area_tiles: area,
        room_objects: rfb_content::ProceduralNormalAllocationDefinition {
            mean: 1,
            standard_deviation: 0,
        },
        anywhere_objects: rfb_content::ProceduralNormalAllocationDefinition {
            mean: 1,
            standard_deviation: 0,
        },
    });
    game.dungeon_states
        .get_mut("demo.dungeon.warrens")
        .unwrap()
        .next_instance_ordinal = 1;
    let floor = game
        .generate_procedural_floor(&definition, Some("demo.dungeon.warrens.instance.1".into()))
        .unwrap();
    assert_eq!(floor.items.len(), 2);
    let items = floor.items.clone();
    game.activate_floor(floor, Vec::new());
    for item in items {
        assert!(
            game.content
                .item(&item.kind_id)
                .unwrap()
                .rfb_base_kind
                .is_some()
        );
        let ItemLocation::Ground(position) = item.location else {
            panic!("floor loot must be on the ground")
        };
        game.player.position = position;
        game.pick_up_item_at_player(Some(&item.id)).unwrap();
        assert_eq!(
            game.items
                .iter()
                .find(|value| value.id == item.id)
                .unwrap()
                .location,
            ItemLocation::Inventory
        );
    }
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn free_room_placement_uses_the_full_floor_without_overlap() {
    let template = Game::new(1);
    let definition = template
        .content
        .world(DEFAULT_WORLD_ID)
        .expect("Middle-earth world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .expect("Warrens depth one should exist")
        .clone();
    let geometry = definition
        .layout
        .as_ref()
        .and_then(|layout| layout.rooms.as_ref())
        .expect("Warrens should retain room geometry");
    assert_eq!(geometry.placement, ProceduralRoomPlacement::Free);

    let mut center_signatures = BTreeSet::new();
    for seed in 0..32 {
        let mut game = Game::new(seed);
        let rooms = game.generate_budgeted_rooms(&definition, geometry);
        assert_eq!(rooms.len(), 5);
        assert_eq!(rooms[0].id, "entry");
        assert_eq!(rooms[1].id, "remote");
        assert!(rooms.iter().map(GeneratedRoom::area).sum::<u32>() <= 450);

        for (index, room) in rooms.iter().enumerate() {
            assert!(room.x >= 1 && room.y >= 1);
            assert!(room.x + room.width < i32::from(definition.width));
            assert!(room.y + room.height < i32::from(definition.height));
            for other in rooms.iter().skip(index + 1) {
                assert!(
                    room.x + room.width < other.x
                        || other.x + other.width < room.x
                        || room.y + room.height < other.y
                        || other.y + other.height < room.y,
                    "free rooms must retain at least one wall tile between bounds"
                );
            }
        }
        center_signatures.insert(rooms.iter().map(GeneratedRoom::center).collect::<Vec<_>>());
    }
    assert!(center_signatures.len() >= 30);
}

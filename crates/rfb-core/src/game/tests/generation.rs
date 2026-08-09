// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use super::*;

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

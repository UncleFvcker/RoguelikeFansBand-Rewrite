// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn built_in_game_is_created_from_the_compiled_content_pack() {
    let snapshot = Game::new(42).snapshot();
    let shard = snapshot
        .items
        .iter()
        .find(|item| item.id == "demo.item.luminous-shard.1")
        .expect("compiled world should spawn its item");

    assert_eq!(snapshot.content_id, "rfb.demo.original-v1");
    assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
    assert_eq!(snapshot.world_id, BUILT_IN_WORLD_ID);
    assert_eq!(
        snapshot.player.melee_damage.damage_type,
        DamageTypeDto::Physical
    );
    assert_eq!(
        snapshot.entities[0].melee_damage.damage_type,
        DamageTypeDto::Fire
    );
    assert_eq!(snapshot.player.id, "demo.actor.player.1");
    assert_eq!(snapshot.player.kind_id, "demo.actor.explorer");
    assert_eq!(snapshot.player.base_attack, 2);
    assert_eq!(snapshot.player.attack, 2);
    assert_eq!(snapshot.player.base_defense, 1);
    assert_eq!(snapshot.player.defense, 1);
    assert!(snapshot.inventory.is_empty());
    assert!(snapshot.equipment.is_empty());
    assert_eq!(snapshot.items.len(), 5);
    assert_eq!(snapshot.entities[0].position, Position { x: 8, y: 5 });
    assert_eq!(snapshot.entities[0].attack, 1);
    assert_eq!(snapshot.entities[0].defense, 1);
    assert_eq!(shard.position, Position { x: 4, y: 3 });
    assert_eq!(
        snapshot
            .cells
            .iter()
            .find(|cell| cell.position == shard.position)
            .and_then(|cell| cell.item_id.as_deref()),
        Some("demo.item.luminous-shard.1")
    );
    assert!(
        snapshot
            .content_visuals
            .iter()
            .any(|visual| visual.id == "demo.item.luminous-shard" && visual.glyph == "!")
    );
    assert_eq!(snapshot.visual_cells.len(), snapshot.cells.len());
    assert_eq!(
        visual_at(&snapshot, snapshot.player.position).visibility,
        VisibilityState::Visible
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 19, y: 19 }).visibility,
        VisibilityState::Hidden
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 8, y: 5 }).light.color,
        ACTOR_LIGHT_COLOR
    );
}

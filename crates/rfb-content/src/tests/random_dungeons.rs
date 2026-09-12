use super::*;

#[test]
fn random_generation_requires_consumed_source_parameters_and_valid_materials() {
    let content = compile_pack_dir(&original_pack_path()).unwrap().content;
    let index = content.worlds[0]
        .dungeons
        .iter()
        .position(|dungeon| dungeon.random)
        .unwrap();
    for update in [0, 1, 2] {
        let mut invalid = content.clone();
        let dungeon = &mut invalid.worlds[0].dungeons[index];
        match update {
            0 => dungeon.loot_quality_policy = None,
            1 => dungeon.tunnel_percent = Some(101),
            _ => dungeon.outer_wall_terrain_id = Some("test.missing-terrain".into()),
        }
        assert!(validate_and_normalize(&mut invalid).is_err());
    }
    let mut invalid = content;
    invalid.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.random-forest-depth-25")
        .unwrap()
        .layout = None;
    assert!(validate_and_normalize(&mut invalid).is_err());
}

#[test]
fn random_depths_are_independent_surface_returns_with_strict_member_ranges() {
    let mut content = compile_pack_dir(&original_pack_path()).unwrap().content;
    for (kind, min, max) in [
        ("random-forest", 25, 50),
        ("random-volcano", 50, 90),
        ("random-mountain", 40, 70),
        ("random-sea", 55, 75),
    ] {
        let world = &content.worlds[0];
        let id = format!("demo.dungeon.{kind}");
        let dungeon = world
            .dungeons
            .iter()
            .find(|dungeon| dungeon.id == id)
            .unwrap();
        assert!(dungeon.random);
        let mut depths = world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some(id.as_str()))
            .map(|floor| {
                assert_eq!(floor.return_floor_id, world.initial_floor_id);
                assert!(floor.next_floor_id.is_none() && floor.down_stair_terrain_id.is_none());
                assert!(!floor.final_floor && floor.guardian.is_none());
                floor.depth
            })
            .collect::<Vec<_>>();
        depths.sort_unstable();
        assert_eq!(depths, (min..=max).collect::<Vec<_>>());
    }

    // A hole or duplicate depth would bias/ambiguate random entry selection.
    let floors = &mut content.worlds[0].procedural_floors;
    let index = floors
        .iter()
        .position(|floor| floor.id == "demo.floor.random-forest-depth-30")
        .unwrap();
    let removed = floors.remove(index);
    assert!(validate_and_normalize(&mut content).is_err());
    content.worlds[0].procedural_floors.push(removed);
    let floor = content.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.random-forest-depth-30")
        .unwrap();
    floor.depth = 31;
    assert!(validate_and_normalize(&mut content).is_err());
}

#[test]
fn wilderness_pool_preserves_non_entrances_and_validates_real_map_bindings() {
    let content = compile_pack_dir(&original_pack_path()).unwrap().content;
    let wilderness = content.worlds[0].wilderness.as_ref().unwrap();
    assert_eq!(wilderness.encounters.len(), 30);
    assert_eq!(
        wilderness
            .encounters
            .iter()
            .filter(|entry| entry.entrance_map.is_some())
            .count(),
        6
    );
    let swimming_hole = wilderness
        .encounters
        .iter()
        .find(|entry| entry.id == "demo.wilderness-encounter.snow-swimming-hole")
        .unwrap();
    assert_eq!(swimming_hole.rarity, 3);
    assert!(swimming_hole.entrance_map.is_none());
    assert!(!wilderness.locations.iter().any(|location| matches!(location,
        WildernessLocationDefinition::Dungeon { dungeon_id, .. } if dungeon_id.contains(".random-"))));

    let mut invalid = content.clone();
    let map = invalid.worlds[0]
        .wilderness
        .as_mut()
        .unwrap()
        .encounters
        .iter_mut()
        .find_map(|entry| entry.entrance_map.as_mut())
        .unwrap();
    map.dungeon_id = "demo.dungeon.warrens".into();
    assert!(validate_and_normalize(&mut invalid).is_err());

    let mut invalid = content;
    let map = invalid.worlds[0]
        .wilderness
        .as_mut()
        .unwrap()
        .encounters
        .iter_mut()
        .find(|entry| entry.id == "demo.wilderness-encounter.trees-random-forest-level")
        .unwrap()
        .entrance_map
        .as_mut()
        .unwrap();
    map.rows[0] = " ".into();
    assert!(validate_and_normalize(&mut invalid).is_err());
}

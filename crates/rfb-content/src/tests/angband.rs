use super::*;

#[test]
fn angband_campaign_requires_valid_distinct_task_ids() {
    let content = compile_pack_dir(&original_pack_path()).unwrap().content;
    assert_eq!(
        content.worlds[0]
            .campaign
            .as_ref()
            .unwrap()
            .victory_task_ids,
        ["demo.task.angband-serpent-of-chaos"]
    );
    for ids in [
        vec![],
        vec!["demo.dungeon.warrens"],
        vec![
            "demo.task.angband-serpent-of-chaos",
            "demo.task.angband-serpent-of-chaos",
        ],
    ] {
        let mut invalid = content.clone();
        invalid.worlds[0]
            .campaign
            .as_mut()
            .unwrap()
            .victory_task_ids = ids.into_iter().map(str::to_owned).collect();
        assert!(validate_and_normalize(&mut invalid).is_err());
    }
}

#[test]
fn angband_chaos_crown_accepts_source_pval_but_rejects_values_above_it() {
    let content = compile_pack_dir(&original_pack_path()).unwrap().content;
    let index = content
        .items
        .iter()
        .position(|item| item.id == "demo.item.crown-of-chaos")
        .unwrap();
    assert_eq!(content.items[index].modifiers.strength, 125);
    assert_eq!(content.items[index].equipment_bonuses.infravision, 125);
    let mut invalid = content.clone();
    invalid.items[index].modifiers.strength = 126;
    assert!(validate_and_normalize(&mut invalid).is_err());
    let mut invalid = content;
    invalid.items[index].equipment_bonuses.infravision = 126;
    assert!(validate_and_normalize(&mut invalid).is_err());
}

#[test]
fn angband_birth_tasks_keep_source_candidates_separate_from_floor_allocation() {
    let content = compile_pack_dir(&original_pack_path()).unwrap().content;
    let world = &content.worlds[0];
    let mut bases = world
        .tasks
        .iter()
        .filter_map(|task| match task.location {
            TaskLocationDefinition::RandomDungeonDepth { base_depth, .. } => Some(base_depth),
            _ => None,
        })
        .collect::<Vec<_>>();
    bases.sort();
    assert_eq!(bases, [10, 18, 26, 34, 42, 50, 58, 66, 74, 82]);
    for kind_id in [
        "demo.actor.bull-gates",
        "demo.actor.eric-the-usurper",
        "demo.actor.sauron-the-sorcerer",
    ] {
        assert!(
            content
                .actors
                .iter()
                .find(|actor| actor.id == kind_id)
                .unwrap()
                .allocation
                .is_none()
        );
        assert!(
            world
                .random_task_candidates
                .iter()
                .any(|candidate| candidate.actor_kind_id == kind_id)
        );
    }
    assert!(
        !world
            .random_task_candidates
            .iter()
            .find(|candidate| candidate.actor_kind_id == "demo.actor.robin-hood-the-outlaw")
            .unwrap()
            .can_be_target
    );
    for kind_id in [
        "demo.actor.oberon-king-of-amber",
        "demo.actor.the-serpent-of-chaos",
        "demo.actor.the-icky-queen",
    ] {
        assert!(
            !world
                .random_task_candidates
                .iter()
                .any(|candidate| candidate.actor_kind_id == kind_id)
        );
    }
    let random_index = world
        .tasks
        .iter()
        .position(|task| {
            matches!(
                task.location,
                TaskLocationDefinition::RandomDungeonDepth { .. }
            )
        })
        .unwrap();
    let mut invalid = content.clone();
    invalid.worlds[0].tasks[random_index].objectives[0].actor_kind_id =
        Some("demo.actor.bull-gates".into());
    assert!(validate_and_normalize(&mut invalid).is_err());
    let mut invalid = content.clone();
    invalid.worlds[0].random_task_candidates[0].actor_kind_id = "demo.actor.missing".into();
    assert!(validate_and_normalize(&mut invalid).is_err());
    let fixed_index = world
        .tasks
        .iter()
        .position(|task| task.id == "demo.task.angband-oberon")
        .unwrap();
    let mut invalid = content.clone();
    invalid.worlds[0].tasks[fixed_index].objectives[0].kind = TaskObjectiveKind::ClearFloor;
    assert!(validate_and_normalize(&mut invalid).is_err());
    let mut invalid = content;
    let duplicate = invalid.worlds[0].random_task_candidates[0].clone();
    invalid.worlds[0].random_task_candidates.push(duplicate);
    assert!(validate_and_normalize(&mut invalid).is_err());
}

#[test]
fn angband_has_one_source_entrance_and_a_complete_bounded_depth_graph() {
    let mut content = compile_pack_dir(&original_pack_path()).unwrap().content;
    let world = &content.worlds[0];
    let dungeon = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.angband")
        .unwrap();
    assert_eq!(dungeon.legacy_index, Some(1));
    assert!(!dungeon.random);
    assert!(dungeon.guardian_actor_kind_id.is_none());
    assert!(dungeon.entry_requirements.is_empty());
    assert_eq!(dungeon.tunnel_percent, Some(50));
    assert_eq!(
        dungeon.loot_quality_policy,
        Some(LootQualityPolicyDefinition::RfbDepth {
            good_cap_percent: 75,
            great_cap_percent: 20,
        })
    );
    let entrances = world
        .wilderness
        .as_ref()
        .unwrap()
        .locations
        .iter()
        .filter_map(|location| match location {
            WildernessLocationDefinition::Dungeon {
                dungeon_id,
                position,
            } if dungeon_id == &dungeon.id => Some(*position),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(entrances, vec![ContentPosition { x: 57, y: 40 }]);
    let floors = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_ref() == Some(&dungeon.id))
        .collect::<Vec<_>>();
    assert_eq!(floors.len(), 127);
    for depth in 1..=127 {
        let floor = floors.iter().find(|floor| floor.depth == depth).unwrap();
        assert_eq!(floor.final_floor, depth == 127);
        assert!(floor.guardian.is_none());
        assert_eq!(
            floor.next_floor_id,
            (depth < 127).then(|| format!("demo.floor.angband-depth-{}", depth + 1))
        );
        for connection in &floor.connections {
            if connection.target_floor_id == world.initial_floor_id {
                assert_eq!(depth, 1);
                continue;
            }
            let target = floors
                .iter()
                .find(|target| target.id == connection.target_floor_id)
                .unwrap();
            let span = if connection.kind == FloorConnectionKind::Shaft {
                2
            } else {
                1
            };
            assert_eq!(depth.abs_diff(target.depth), span);
            assert!(target.connections.iter().any(|reverse| Some(&reverse.id)
                == connection.target_connection_id.as_ref()
                && reverse.target_floor_id == floor.id));
        }
    }
    let floor = content.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.angband-depth-127")
        .unwrap();
    floor.connections[0].target_floor_id = "demo.floor.angband-depth-128".into();
    assert!(validate_and_normalize(&mut content).is_err());
}

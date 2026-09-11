// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use std::sync::OnceLock;

const DUNGEON: &str = "demo.dungeon.mount-olympus";
const FLOOR: &str = "demo.floor.mount-olympus-depth-80";
const OTHER_FLOOR: &str = "demo.floor.castle-depth-40";
const ZEUS: &str = "demo.actor.zeus-king-of-the-olympians";

fn catalog() -> Arc<ContentCatalog> {
    static CONTENT: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packs/rfb-demo-original");
            let artifact = rfb_content::compile_pack_dir(&root).unwrap();
            Arc::new(ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content).unwrap(),
            ))
        })
        .clone()
}

fn game_with_olympians(active: bool) -> Game {
    (0..32)
        .map(|seed| {
            Game::from_content_with_build(seed, catalog(), DEFAULT_WORLD_ID, "demo.build.warrior")
                .unwrap()
        })
        .find(|game| (game.active_pantheons & 2 != 0) == active)
        .unwrap()
}

#[test]
fn pantheons_birth_selection_persists_without_rng_and_rejects_invalid_saves() {
    let mut masks = BTreeSet::new();
    for seed in 0..16 {
        let game =
            Game::from_content_with_build(seed, catalog(), DEFAULT_WORLD_ID, "demo.build.warrior")
                .unwrap();
        assert_eq!(game.active_pantheons.count_ones(), 2);
        assert_eq!(game.active_pantheons & !0x1e, 0);
        masks.insert(game.active_pantheons);
        assert_eq!(
            game.dungeon_is_active(DUNGEON),
            game.active_pantheons & 2 != 0
        );
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng_draw_counter(), restored.rng_draw_counter());
        assert_eq!(game.clone().rng.bounded(1000), restored.rng.bounded(1000));
    }
    assert_eq!(masks.len(), 6);
    let game = game_with_olympians(false);
    for mask in [0, 2, 0x1e, 0x81] {
        let mut invalid = game.to_save();
        invalid.active_pantheons = mask;
        assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());
    }
    let mut invalid = game.to_save();
    invalid
        .dungeon_states
        .iter_mut()
        .find(|d| d.dungeon_id == DUNGEON)
        .unwrap()
        .suppressed = false;
    assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());
    let mut missing = serde_json::to_value(game.to_save()).unwrap();
    missing.as_object_mut().unwrap().remove("activePantheons");
    assert!(serde_json::from_value::<rfb_protocol::SavePayloadV1>(missing).is_err());
    let mut changed = game.clone();
    changed.active_pantheons = *masks
        .iter()
        .find(|mask| **mask != game.active_pantheons)
        .unwrap();
    assert_ne!(changed.state_hash(), game.state_hash());
}

#[test]
fn pantheons_control_world_entry_and_entrance_guardian() {
    for active in [false, true] {
        let mut game = game_with_olympians(active);
        choose_human_talent_if_pending(&mut game);
        let position = Position { x: 5, y: 9 };
        assert_eq!(
            game.wilderness_cell_dto(position)
                .locations
                .iter()
                .any(|l| l.id == DUNGEON),
            active
        );
        dispatch_next(
            &mut game,
            GameCommand::EnterWorldMap {
                leave_pets: false,
                cancel_recall: false,
            },
        );
        game.wilderness_position = Some(position);
        dispatch_next(&mut game, GameCommand::LeaveWorldMap);
        assert_eq!(
            game.terrain
                .iter()
                .any(|id| id == "demo.terrain.mount-olympus-entrance"),
            active
        );
        assert_eq!(
            game.entities
                .iter()
                .any(|a| a.id == "demo.guardian.mount-olympus-entrance.1"),
            active
        );
        if !active {
            assert!(
                game.transition_floor(FLOOR.into(), None, None, false)
                    .unwrap()
                    .is_none()
            );
        }
        let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(restored.dungeon_is_active(DUNGEON), active);
    }
}

#[test]
fn pantheons_primary_secondary_and_dungeon_lock_have_distinct_meanings() {
    let mut game = game_with_olympians(true);
    game.active_pantheons = 2 | 8;
    let zeus = game.content.actor(ZEUS).unwrap();
    assert!(game.pantheon_allows_allocation(FLOOR, zeus));
    assert!(!game.pantheon_allows_allocation(OTHER_FLOOR, zeus));
    assert!(!game.pantheon_allows_allocation(&game.current_floor_id, zeus));
    let aegir = game
        .content
        .actor("demo.actor.aegir-god-king-of-the-sea-giants")
        .unwrap();
    assert!(aegir.tags.iter().any(|tag| tag == "norse2"));
    assert!(game.pantheon_allows_allocation(OTHER_FLOOR, aegir));
    assert!(!game.pantheon_allows_allocation(FLOOR, aegir));
    assert!(!game.pantheon_allows_allocation(&game.current_floor_id, aegir));
    let hypnos = game
        .content
        .actor("demo.actor.hypnos-lord-of-sleep")
        .unwrap()
        .clone();
    assert!(!hypnos.tags.iter().any(|tag| tag == "olympian"));
    game.active_pantheons = 4 | 16;
    for floor in [FLOOR, OTHER_FLOOR, game.current_floor_id.as_str()] {
        assert!(game.pantheon_allows_allocation(floor, &hypnos));
        assert!(!game.pantheon_allows_allocation(floor, game.content.actor(ZEUS).unwrap()));
    }
    let mut locked = hypnos;
    locked.allocation.as_mut().unwrap().legacy_dungeon_indices = vec![22];
    assert!(game.actor_is_pantheon_suppressed(&locked));
    // An active primary membership takes precedence over the dungeon lock
    // when computing suppression (dungeon.c: flag_mask_keep).
    locked.tags.push("egyptian".into());
    assert!(!game.actor_is_pantheon_suppressed(&locked));
}

#[test]
fn pantheons_random_allocation_and_category_summons_share_qualification() {
    let mut game = game_with_olympians(true);
    game.active_pantheons = 2 | 8;
    let mut policy = game
        .content
        .encounter_table("demo.encounter-table.mount-olympus")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    policy.preferred_tags = vec!["olympian".into()];
    policy.preferred_glyphs.clear();
    policy.preferred_movement_modes.clear();
    policy.preferred_habitats.clear();
    policy.preferred_damage_immunities.clear();
    policy.preferred_damage_resistances.clear();
    policy.special_div = 0;
    for _ in 0..8 {
        let id = game
            .select_original_allocated_monster(FLOOR, &policy, 100, 100, None, &[], None, None)
            .unwrap();
        assert!(
            game.content
                .actor(&id)
                .unwrap()
                .tags
                .iter()
                .any(|tag| tag == "olympian")
        );
    }
    game.active_pantheons = 4 | 16;
    assert!(
        game.select_original_allocated_monster(FLOOR, &policy, 100, 100, None, &[], None, None)
            .is_none()
    );
    game.active_pantheons = 2 | 8;
    game.current_floor_id = FLOOR.into();
    let active = game.summon_category_candidate_kind_ids("olympian", None, 100, true, true);
    assert!(active.iter().any(|id| id == ZEUS));
    game.defeated_limited_actor_counts.insert(ZEUS.into(), 1);
    assert!(
        !game
            .summon_category_candidate_kind_ids("olympian", None, 100, true, true)
            .iter()
            .any(|id| id == ZEUS)
    );
    // Requesting an inactive pantheon uses the source's unique-category rule.
    game.current_floor_id = OTHER_FLOOR.into();
    let fallback = game.summon_category_candidate_kind_ids("egyptian", None, 100, true, false);
    assert!(!fallback.is_empty());
    assert!(!fallback.iter().any(|id| id == "demo.actor.nazgul"));
    for id in &fallback {
        let actor = game.content.actor(id).unwrap();
        assert!(actor.tags.iter().any(|tag| tag == "unique"));
        assert!(!actor.tags.iter().any(|tag| tag == "egyptian"));
    }
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".into());
    game.player.position = Position { x: 80, y: 20 };
    game.push_generated_actor(
        "test.caster".into(),
        "demo.actor.osiris-the-reborn",
        Position { x: 20, y: 20 },
    );
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-egyptian-l97-1d2")
        .unwrap()
        .clone();
    let plan = game.monster_ability_target_plan(0, ability, 1).unwrap();
    let MonsterAbilityTargetPlan::SummonCategory {
        candidate_kind_ids, ..
    } = &plan.target
    else {
        panic!("category plan");
    };
    assert!(!candidate_kind_ids.is_empty());
    assert!(candidate_kind_ids.iter().all(|id| fallback.contains(id)));
}

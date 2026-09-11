// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters};
use super::*;
use crate::game::inventory::ItemIdentificationRequest;
use crate::game::movement::actor_can_cross_terrain;
use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};
use rfb_protocol::WeaponTraitDto;
use serde_json::json;
use std::sync::OnceLock;

const AMUN: &str = "demo.item.amun";
const DUNGEON: &str = "test.dungeon.pyramidal-mound";
const TABLE: &str = "test.encounter-table.pyramidal-mound";
const GUARDIAN: &str = "demo.actor.amun-the-mysterious";

fn floor_id(depth: u16) -> String {
    format!("test.floor.pyramidal-mound-depth-{depth}")
}

fn connection(
    depth: u16,
    target: Option<u16>,
    kind: &str,
    direction: &str,
) -> rfb_content::ProceduralFloorConnectionDefinition {
    let reverse = if direction == "up" { "down" } else { "up" };
    serde_json::from_value(json!({
        "id": format!("test.connection.pm-{depth}-{kind}-{direction}"),
        "kind": kind,
        "terrainId": format!("demo.terrain.{kind}-{direction}"),
        "targetFloorId": target.map(floor_id).unwrap_or("demo.floor.surface".into()),
        "targetConnectionId": target.map(|d| format!("test.connection.pm-{d}-{kind}-{reverse}"))
    }))
    .unwrap()
}

fn generation_catalog() -> Arc<ContentCatalog> {
    static CONTENT: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packs/rfb-demo-original");
            let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
            // PM2 uses a separate test dungeon. PM4 will replace this builder with
            // the formal catalog; no existing dungeon is repurposed as a stand-in.
            let mut entry = artifact
                .content
                .terrain
                .iter()
                .find(|t| t.id == "demo.terrain.mount-olympus-entrance")
                .unwrap()
                .clone();
            entry.id = "test.terrain.pyramidal-mound-entrance".into();
            artifact.content.terrain.push(entry);
            let mut table = artifact
                .content
                .encounter_tables
                .iter()
                .find(|t| t.id == "demo.encounter-table.mount-olympus")
                .unwrap()
                .clone();
            table.id = TABLE.into();
            let policy = table.global_allocation.as_mut().unwrap();
            policy.preferred_tags = vec!["egyptian".into(), "egyptian2".into()];
            policy.special_div = 1;
            artifact.content.encounter_tables.push(table);
            let world = artifact
                .content
                .worlds
                .iter_mut()
                .find(|w| w.id == DEFAULT_WORLD_ID)
                .unwrap();
            let mut template = world
                .procedural_floors
                .iter()
                .find(|f| f.id == "demo.floor.mount-olympus-depth-80")
                .unwrap()
                .clone();
            let streamers = world
                .procedural_floors
                .iter()
                .find(|f| f.id == "demo.floor.warrens-depth-1")
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .streamers
                .clone();
            template.dungeon_id = Some(DUNGEON.into());
            template.name_key = "floor-test-pyramidal-mound-name".into();
            template.encounter_table_id = Some(TABLE.into());
            template.generation_budget = Some(
                serde_json::from_value(json!({
                    "actorSlots": 20, "lootPlacements": 8, "roomPlacements": 6,
                    "roomAreaTiles": 1100, "streamerPlacements": 2, "streamerAreaTiles": 32
                }))
                .unwrap(),
            );
            template.layout = Some(
                serde_json::from_value(json!({
                    "rooms": {
                        "placement": "free", "minWidth": 8, "maxWidth": 20,
                        "minHeight": 7, "maxHeight": 13,
                        "shapes": [{"shape": "cavern", "weight": 1000},
                                   {"shape": "rectangle", "weight": 450}]
                    },
                    "wallMix": [{"terrainId": "demo.terrain.quartz-vein", "percent": 5}],
                    "streamers": streamers, "placeDoors": true
                }))
                .unwrap(),
            );
            world.dungeons.push(
                serde_json::from_value(json!({
                    "id": DUNGEON, "legacyIndex": 34, "pantheon": 2,
                    "rootFloorId": floor_id(64), "guardianActorKindId": GUARDIAN
                }))
                .unwrap(),
            );
            // The complete logical chain is required by content validation, even
            // though PM2 only exercises representative maps and connection edges.
            for depth in 64..=92 {
                let mut floor = template.clone();
                floor.id = floor_id(depth);
                floor.depth = depth;
                floor.return_floor_id = if depth == 64 {
                    world.initial_floor_id.clone()
                } else {
                    floor_id(depth - 1)
                };
                floor.next_floor_id = (depth < 92).then(|| floor_id(depth + 1));
                floor.down_stair_terrain_id =
                    (depth < 92).then(|| "demo.terrain.stairs-down".into());
                floor.entry_terrain_id =
                    (depth == 64).then(|| "test.terrain.pyramidal-mound-entrance".into());
                floor.entry_connection_id =
                    (depth == 64).then(|| "test.connection.pm-64-stairs-up".into());
                floor.final_floor = depth == 92;
                floor.guardian = (depth == 92).then(|| {
                    serde_json::from_value(json!({
                        "instanceId": "test.guardian.pyramidal-mound.1", "actorKindId": GUARDIAN
                    }))
                    .unwrap()
                });
                floor.connections = vec![connection(
                    depth,
                    (depth >= 66).then(|| depth - 2),
                    if depth == 64 { "stairs" } else { "shaft" },
                    "up",
                )];
                if depth < 92 {
                    floor.connections.push(connection(
                        depth,
                        Some(if depth == 91 { 92 } else { depth + 2 }),
                        if depth == 91 { "stairs" } else { "shaft" },
                        "down",
                    ));
                } else {
                    floor
                        .connections
                        .push(connection(92, Some(91), "stairs", "up"));
                }
                world.procedural_floors.push(floor);
            }
            Arc::new(ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content).unwrap(),
            ))
        })
        .clone()
}

fn generation_game(active: bool) -> Game {
    (0..32)
        .map(|seed| {
            Game::from_content_with_build(
                seed,
                generation_catalog(),
                DEFAULT_WORLD_ID,
                "demo.build.warrior",
            )
            .unwrap()
        })
        .find(|g| (g.active_pantheons & 4 != 0) == active)
        .unwrap()
}

fn floor_definition(game: &Game, depth: u16) -> rfb_content::ProceduralFloorDefinition {
    game.content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|f| f.id == floor_id(depth))
        .unwrap()
        .clone()
}

#[test]
fn pyramidal_mound_representative_maps_keep_materials_routes_and_legal_spawns() {
    let base = generation_game(true);
    let mut materials = BTreeSet::new();
    let mut items = 0;
    let mut gold = 0;
    let mut primary = false;
    let mut secondary = false;
    let mut ordinary = false;
    for depth in [64, 78, 92] {
        let definition = floor_definition(&base, depth);
        for seed in [2, 7, 42] {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let floor = game.generate_procedural_floor(&definition, None).unwrap();
            assert_eq!((floor.width, floor.height), (96, 33));
            let at = |p| {
                game.content
                    .terrain(&floor.terrain[generated_terrain_index(floor.width, p)])
                    .unwrap()
            };
            let walkable = floor
                .terrain
                .iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    let terrain = game.content.terrain(id).unwrap();
                    (terrain.walkable || terrain.open_to_terrain_id.is_some()).then_some(Position {
                        x: (i % usize::from(floor.width)) as i32,
                        y: (i / usize::from(floor.width)) as i32,
                    })
                })
                .collect::<BTreeSet<_>>();
            let reached = maze_floor_distances(&walkable, floor.player_position);
            assert_eq!(
                reached.len(),
                walkable.len(),
                "depth {depth}, seed {seed}: all walkable cells connect through openable doors"
            );
            let mut occupied = BTreeSet::from([floor.player_position]);
            for c in &floor.connections {
                assert!(
                    reached.contains_key(&c.position),
                    "depth {depth}, seed {seed}, {c:?}"
                );
                occupied.insert(c.position);
            }
            assert!(
                floor
                    .terrain
                    .iter()
                    .any(|id| id == &definition.closed_door_terrain_id)
            );
            assert!(
                floor
                    .terrain
                    .iter()
                    .any(|id| id == &definition.trap_terrain_id)
            );
            materials.extend(floor.terrain.iter().cloned());
            assert!(!floor.entities.is_empty());
            for actor in &floor.entities {
                let kind = game.content.actor(&actor.kind_id).unwrap();
                assert!(occupied.insert(actor.position));
                assert!(actor_can_cross_terrain(kind, at(actor.position)));
                if actor.kind_id == GUARDIAN {
                    assert_eq!(depth, 92);
                    assert_eq!(actor.id, "test.guardian.pyramidal-mound.1");
                    assert_eq!(kind.level, 97);
                    assert!(reached.contains_key(&actor.position));
                } else {
                    assert!(game.pantheon_allows_allocation(&definition.id, kind));
                    primary |= kind.tags.iter().any(|t| t == "egyptian");
                    secondary |= kind.tags.iter().any(|t| t == "egyptian2");
                    ordinary |= !kind
                        .tags
                        .iter()
                        .any(|t| t == "egyptian" || t == "egyptian2");
                }
            }
            assert_eq!(
                floor
                    .entities
                    .iter()
                    .filter(|a| a.kind_id == GUARDIAN)
                    .count(),
                usize::from(depth == 92)
            );
            assert!(floor.items.iter().all(|item| match item.location {
                ItemLocation::Ground(p) => at(p).allows_items(),
                _ => false,
            }));
            assert!(
                floor
                    .gold_piles
                    .iter()
                    .all(|pile| at(pile.position).allows_items())
            );
            items += floor.items.len();
            gold += floor.gold_piles.len();
        }
    }
    for id in [
        "demo.terrain.wall",
        "demo.terrain.quartz-vein",
        "demo.terrain.magma-vein",
    ] {
        assert!(materials.contains(id), "missing {id}");
    }
    assert!(
        !materials
            .iter()
            .any(|id| id.contains("water") || id.contains("waste"))
    );
    assert!(items > 0 && gold > 0, "items={items}, gold={gold}");
    assert!(
        primary && secondary && ordinary,
        "primary={primary}, secondary={secondary}, ordinary={ordinary}"
    );
}

#[test]
fn pyramidal_mound_shafts_stop_at_surface_and_final_floor() {
    let base = generation_game(true);
    for (depth, targets) in [
        (64, vec![0, 66]),
        (65, vec![0, 67]),
        (78, vec![76, 80]),
        (91, vec![89, 92]),
        (92, vec![90, 91]),
    ] {
        let mut game = base.clone();
        let definition = floor_definition(&game, depth);
        let floor = game.generate_procedural_floor(&definition, None).unwrap();
        game.activate_floor(floor, Vec::new());
        let mut actual = Vec::new();
        for c in &game.floor_connections {
            let target_id = c.target_floor_id.as_ref().unwrap();
            let target = if target_id == "demo.floor.surface" {
                0
            } else {
                target_id
                    .rsplit('-')
                    .next()
                    .unwrap()
                    .parse::<u16>()
                    .unwrap()
            };
            actual.push(target);
            let terrain = game.content.terrain(game.terrain_at(c.position)).unwrap();
            assert_eq!(
                terrain.tags.iter().any(|t| t == "shaft"),
                !(depth == 64 && target == 0 || depth.abs_diff(target) == 1)
            );
            if target != 0 {
                let mut traversed = game.clone();
                let update = traversed
                    .transition_floor(
                        target_id.clone(),
                        c.target_connection_id.clone(),
                        None,
                        false,
                    )
                    .unwrap();
                assert!(update.is_some());
                assert_eq!(traversed.current_floor_id, floor_id(target));
                let return_link = traversed
                    .floor_connections
                    .iter()
                    .find(|r| Some(&r.id) == c.target_connection_id.as_ref())
                    .unwrap();
                assert_eq!(traversed.player.position, return_link.position);
                assert_eq!(return_link.target_floor_id.as_ref(), Some(&definition.id));
            }
        }
        actual.sort_unstable();
        assert_eq!(actual, targets);
    }
}

#[test]
fn pyramidal_mound_ecology_keeps_rarity_divisor_and_egyptian_qualification() {
    let mut game = generation_game(true);
    let policy = game
        .content
        .encounter_table(TABLE)
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    for id in [
        "demo.actor.osiris-the-reborn",
        "demo.actor.mummy-king",
        "demo.actor.the-phoenix",
    ] {
        let actor = game.content.actor(id).unwrap().clone();
        let before = game.rng.clone();
        assert_eq!(
            game.original_dungeon_weight(&actor, &policy),
            100 / actor.allocation.as_ref().unwrap().rarity
        );
        assert_eq!(game.rng, before);
    }
    let newt = game.content.actor("demo.actor.newt").unwrap().clone();
    let base_weight = 100 / newt.allocation.as_ref().unwrap().rarity;
    for round_up in [false, true] {
        let seed = (0..1000)
            .find(|seed| {
                (RfbRng::seeded(*seed).bounded(64) < u64::from(base_weight % 64)) == round_up
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        game.monster_division_remainders.clear();
        assert_eq!(
            game.original_dungeon_weight(&newt, &policy),
            base_weight / 64 + u32::from(round_up)
        );
        let after = game.rng.clone();
        game.original_dungeon_weight(&newt, &policy);
        assert_eq!(
            game.rng, after,
            "fractional division is stable within this allocation table"
        );
    }
    for active in [true, false] {
        let mut game = generation_game(active);
        let osiris = game.content.actor("demo.actor.osiris-the-reborn").unwrap();
        let bast = game
            .content
            .actor("demo.actor.bast-goddess-of-cats")
            .unwrap();
        let mummy = game.content.actor("demo.actor.mummy-king").unwrap();
        assert_eq!(
            game.pantheon_allows_allocation(&floor_id(78), osiris),
            active
        );
        assert!(!game.pantheon_allows_allocation("demo.floor.castle-depth-40", osiris));
        assert_eq!(
            game.pantheon_allows_allocation("demo.floor.castle-depth-40", bast),
            active
        );
        assert!(!game.pantheon_allows_allocation("demo.floor.mount-olympus-depth-80", bast));
        assert!(game.pantheon_allows_allocation("demo.floor.castle-depth-40", mummy));
        // Ordinary secondary-tagged monsters stay eligible when Egypt is inactive.
        let mut seen = BTreeSet::new();
        for _ in 0..64 {
            let id = game
                .select_original_allocated_monster(
                    &floor_id(78),
                    &policy,
                    92,
                    78,
                    None,
                    &[],
                    None,
                    None,
                )
                .unwrap();
            assert_ne!(id, GUARDIAN);
            assert!(
                game.pantheon_allows_allocation(&floor_id(78), game.content.actor(&id).unwrap())
            );
            seen.insert(id);
        }
        assert!(seen.iter().any(|id| {
            game.content
                .actor(id)
                .unwrap()
                .tags
                .iter()
                .any(|t| t == "egyptian2")
        }));
        if !active {
            assert!(seen.iter().all(|id| {
                !game
                    .content
                    .actor(id)
                    .unwrap()
                    .tags
                    .iter()
                    .any(|t| t == "egyptian")
            }));
            assert!(
                game.transition_floor(floor_id(78), None, None, false)
                    .unwrap()
                    .is_none()
            );
        }
        // Use a non-pantheon dungeon for the inactive request's unique category.
        game.current_floor_id = if active {
            floor_id(78)
        } else {
            "demo.floor.castle-depth-40".into()
        };
        let candidates =
            game.summon_category_candidate_kind_ids("egyptian", None, 100, true, false);
        assert!(!candidates.is_empty());
        for id in &candidates {
            let actor = game.content.actor(id).unwrap();
            assert!(actor.tags.iter().any(|t| t == "unique"));
            assert_eq!(actor.tags.iter().any(|t| t == "egyptian"), active);
        }
        // Prepared open space isolates the actual random-summon consumer.
        let mut casting = game.clone();
        clear_monsters(&mut casting);
        casting.terrain.fill("demo.terrain.floor".into());
        casting.player.position = Position { x: 80, y: 20 };
        casting.push_generated_actor(
            "test.amun-caster".into(),
            GUARDIAN,
            Position { x: 20, y: 20 },
        );
        let ability = casting
            .content
            .ability("rfb-legacy.ability.summon-egyptian-l97-1d2")
            .unwrap()
            .clone();
        let plan = casting.monster_ability_target_plan(0, ability, 1).unwrap();
        let summoned = casting
            .resolve_monster_ability_plan(
                0,
                GUARDIAN,
                &plan,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .summon
            .unwrap();
        assert!((1..=2).contains(&summoned.summoned_kind_ids.len()));
        for id in &summoned.summoned_kind_ids {
            assert!(candidates.contains(id));
            assert_ne!(
                id, GUARDIAN,
                "the existing caster consumes its unique allowance"
            );
            assert!(
                casting
                    .entities
                    .iter()
                    .any(|a| &a.kind_id == id && a.summon.is_some())
            );
        }
        if active {
            assert!(candidates.iter().any(|id| id == GUARDIAN));
            game.defeated_limited_actor_counts
                .insert(GUARDIAN.into(), 1);
            assert!(
                !game
                    .summon_category_candidate_kind_ids("egyptian", None, 100, true, false)
                    .iter()
                    .any(|id| id == GUARDIAN)
            );
        }
    }
    let definition = floor_definition(&game, 78);
    let floor = game.generate_procedural_floor(&definition, None).unwrap();
    game.activate_floor(floor, Vec::new());
    clear_monsters(&mut game);
    let chance = u64::from(policy.ambient_chance_one_in) * 178 / 100;
    let seed = (0..10000)
        .find(|seed| RfbRng::seeded(*seed).bounded(chance) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.process_ambient_monster_allocation(&mut BTreeSet::new())
        .unwrap();
    assert!(!game.entities.is_empty());
    for actor in &game.entities {
        assert_ne!(actor.kind_id, GUARDIAN);
        assert!(game.pantheon_allows_allocation(
            &definition.id,
            game.content.actor(&actor.kind_id).unwrap()
        ));
    }
}

fn artifact_context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-60".into(),
        depth: 60,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    }
}

#[test]
fn pyramidal_mound_amun_uses_normal_artifact_rarity_and_unique_registration() {
    let mut game = Game::new_with_build(67, "demo.build.warrior").unwrap();
    let context = artifact_context();
    // Exercise the normal fixed-artifact selector after an ordinary dagger base
    // was selected. No Amun death reward or guaranteed-artifact table is used.
    for (roll, expected) in [(1, None), (119, None), (0, Some(AMUN))] {
        let seed = (0..10_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(120) == roll)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        assert_eq!(
            game.roll_fixed_artifact_kind_id(&context, Some("demo.item.dagger"), false)
                .as_deref(),
            expected
        );
        assert_eq!(game.rng.draw_counter, 1);
    }
    let draft = game.fixed_item_draft(&context, AMUN.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert!(item.curse.is_none());
    game.items.push(item);
    assert!(game.generated_artifact_ids.contains(AMUN));
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let rng = restored.rng.clone();
    assert!(
        restored
            .roll_fixed_artifact_kind_id(&context, Some("demo.item.dagger"), false)
            .is_none()
    );
    assert_eq!(restored.rng, rng);
}

#[test]
fn pyramidal_mound_amun_equips_senses_expires_and_recovers_across_save() {
    let mut game = Game::new_with_build(67, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    let draft = game.fixed_item_draft(&artifact_context(), AMUN.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    let before = game.player_derived_stats();
    let before_intelligence = game.equipment_modifiers().intelligence;
    let before_see_invisible = game.player_see_invisible_sources();
    game.equip_inventory_item(&id, None).unwrap();
    let after = game.player_derived_stats();
    assert_eq!(
        game.equipment_modifiers().intelligence,
        before_intelligence + 5
    );
    assert_eq!(after.speed.value, before.speed.value + 5);
    assert_eq!(after.stealth_skill.value, before.stealth_skill.value + 5);
    assert_eq!(after.search_skill.value, before.search_skill.value + 25);
    assert_eq!(
        after.perception_skill.value,
        before.perception_skill.value + 25
    );
    assert_eq!(
        game.player_see_invisible_sources(),
        before_see_invisible + 1
    );
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Immune
    );
    for damage in [
        DamageType::Poison,
        DamageType::Light,
        DamageType::Electricity,
        DamageType::Sound,
    ] {
        assert_eq!(
            game.effective_player_resistances().level(damage),
            ResistanceLevel::Resistant
        );
    }
    let weapon = &game.items[0];
    let melee = game.item_melee_profile(weapon).unwrap();
    assert_eq!((melee.damage.dice, melee.damage.sides), (3, 5));
    assert!(game.item_has_weapon_trait(weapon, WeaponTraitDto::Blessed));
    // Fixed definitions use the vorpal flag; item_has_weapon_trait reads
    // rolled/intrinsic traits instead (covered by the existing melee tests).
    assert!(game.content.item(&weapon.kind_id).unwrap().vorpal);
    assert_eq!(
        weapon.activation.as_ref().unwrap().device_check_difficulty,
        60
    );
    assert!(!game.player_has_telepathy());
    // A prepared ordinary monster exercises the actual sensing consumer.
    game.push_generated_actor(
        "test.esp-target".into(),
        "demo.actor.goblin",
        game.player.position,
    );
    assert!(!game.entity_is_visible_by_telepathy(&game.entities[0]));
    let mut boosted = game.clone();
    let mut power = monster_combat::melee_status(STATUS_HASTE, 2_000, "test.device-power").status;
    power.granted_modifiers.device_power_bonus = 5;
    boosted.player.statuses.push(power);
    let mut events = Vec::new();
    for current in [&mut game, &mut boosted] {
        for _ in 0..100 {
            current
                .use_inventory_item(
                    &id,
                    Some(&TargetSelection::SelfTarget),
                    None,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            if current.items[0].charges.unwrap().current == 0 {
                break;
            }
        }
        assert_eq!(current.items[0].charges.unwrap().current, 0);
    }
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    assert!(game.entity_is_visible_by_telepathy(&game.entities[0]));
    let duration = game
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_TELEPATHY)
        .unwrap()
        .remaining_ticks;
    assert!(
        (270..=560).contains(&duration) && duration % 10 == 0,
        "source 1d30+25 turns plus the item's immediate action window: {duration}"
    );
    let boosted_duration = boosted
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_TELEPATHY)
        .unwrap()
        .remaining_ticks;
    let source_turns = duration / 10 - 1;
    assert_eq!(
        boosted_duration,
        (source_turns + source_turns * 5 / 20 + 1) * 10
    );
    assert_eq!(
        boosted.rng, game.rng,
        "device power must not change the duration roll"
    );
    // Remove the prepared overlapping target before testing a valid save.
    clear_monsters(&mut game);
    let state = game.to_save();
    game.use_inventory_item(
        &id,
        Some(&TargetSelection::SelfTarget),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        game.to_save(),
        state,
        "cooldown cannot consume RNG or reapply ESP"
    );
    for tick in 1..=100 {
        game.world_tick = tick;
        game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut Vec::new(), false)
            .unwrap();
        game.process_inventory_device_recovery(&mut events);
    }
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(restored.player_has_telepathy());
    assert_eq!(restored.items[0].device_recovery_progress, 100);
    for tick in 101..=1000 {
        for current in [&mut game, &mut restored] {
            current.world_tick = tick;
            current
                .process_status_tick(
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                    false,
                )
                .unwrap();
            current.process_inventory_device_recovery(&mut Vec::new());
            if tick == duration {
                assert!(!current.player_has_telepathy());
            }
            if tick == 999 {
                assert_eq!(current.items[0].charges.unwrap().current, 0);
                assert_eq!(current.items[0].device_recovery_progress, 999);
            }
        }
    }
    assert_eq!(restored.items[0].charges.unwrap().current, 1);
    assert_eq!(restored.items[0].device_recovery_progress, 0);
    assert_eq!(restored.to_save(), game.to_save());
    assert_eq!(restored.rng, game.rng);
    for current in [&mut game, &mut restored] {
        current
            .use_inventory_item(
                &id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
    }
    assert_eq!(restored.to_save(), game.to_save());
    let slot = match &restored.items[0].location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => panic!("Amun must remain equipped"),
    };
    restored.unequip_slot(&slot).unwrap();
    assert_eq!(
        restored.equipment_modifiers().intelligence,
        before_intelligence
    );
}

use super::*;

#[test]
fn compiled_catalog_exposes_stable_runtime_indexes() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

    assert_eq!(catalog.pack_id(), "rfb.demo.original-v1");
    assert_eq!(catalog.pack_version(), "1.220.0");
    assert!(
        catalog
            .ability("demo.ability.warrens-scare")
            .is_some_and(|ability| ability.player.is_none())
    );
    assert!(
        catalog
            .ability("demo.ability.mending-echo")
            .is_some_and(|ability| ability.player.is_some())
    );
    assert_eq!(
        catalog
            .town("demo.town.outpost")
            .map(|town| (town.floor_id.as_str(), town.shop_ids.as_slice())),
        Some((
            "demo.floor.surface",
            [
                "demo.shop.outpost-alchemist".to_owned(),
                "demo.shop.outpost-armoury".to_owned(),
                "demo.shop.outpost-black-market".to_owned(),
                "demo.shop.outpost-bookstore".to_owned(),
                "demo.shop.outpost-general-store".to_owned(),
                "demo.shop.outpost-magic-shop".to_owned(),
                "demo.shop.outpost-temple".to_owned(),
                "demo.shop.outpost-weaponsmith".to_owned(),
            ]
            .as_slice()
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-black-market").map(|shop| (
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            ShopCategory::BlackMarket,
            ContentPosition { x: 55, y: 19 },
            "demo.terrain.black-market-entrance",
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-bookstore").map(|shop| (
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            ShopCategory::Bookstore,
            ContentPosition { x: 55, y: 13 },
            "demo.terrain.bookstore-entrance",
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-armoury").map(|shop| (
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            ShopCategory::Armoury,
            ContentPosition { x: 30, y: 19 },
            "demo.terrain.armoury-entrance",
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-weaponsmith").map(|shop| (
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            ShopCategory::Weaponsmith,
            ContentPosition { x: 34, y: 19 },
            "demo.terrain.weaponsmith-entrance",
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-temple").map(|shop| (
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            ShopCategory::Temple,
            ContentPosition { x: 45, y: 19 },
            "demo.terrain.temple-entrance",
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-alchemist").map(|shop| (
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            ShopCategory::Alchemist,
            ContentPosition { x: 53, y: 13 },
            "demo.terrain.alchemist-entrance",
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-general-store").map(|shop| (
            shop.town_id.as_str(),
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            "demo.town.outpost",
            ShopCategory::GeneralStore,
            ContentPosition { x: 32, y: 13 },
            "demo.terrain.general-store-entrance",
        ))
    );
    assert_eq!(
        catalog.shop("demo.shop.outpost-magic-shop").map(|shop| (
            shop.category,
            shop.entrance_position,
            shop.entrance_terrain_id.as_str(),
        )),
        Some((
            ShopCategory::MagicShop,
            ContentPosition { x: 57, y: 13 },
            "demo.terrain.magic-shop-entrance",
        ))
    );
    assert_eq!(
        catalog.resource("demo.resource.mana").map(|resource| (
            resource.name_key.as_str(),
            resource.wait_recovery_amount,
            resource.rest_recovery_amount,
        )),
        Some(("resource-demo-mana-name", 1, 3))
    );
    assert_eq!(
        catalog
            .ability_book("demo.ability-book.echo-primer")
            .map(|book| book.ability_ids.as_slice()),
        Some(
            [
                "demo.ability.death-black-sleep".to_owned(),
                "demo.ability.death-detect-evil".to_owned(),
                "demo.ability.death-detect-unlife".to_owned(),
                "demo.ability.death-enslave-undead".to_owned(),
                "demo.ability.death-horrify".to_owned(),
                "demo.ability.death-malediction".to_owned(),
                "demo.ability.death-necromantic-resistance".to_owned(),
                "demo.ability.death-stinking-cloud".to_owned(),
                "demo.ability.echo-binding".to_owned(),
                "demo.ability.echo-burst".to_owned(),
                "demo.ability.echo-companion".to_owned(),
                "demo.ability.echo-delving".to_owned(),
                "demo.ability.echo-fan".to_owned(),
                "demo.ability.echo-lance".to_owned(),
                "demo.ability.echo-pulse".to_owned(),
                "demo.ability.echo-quickening".to_owned(),
                "demo.ability.echo-rampart".to_owned(),
                "demo.ability.echo-sight".to_owned(),
                "demo.ability.echo-step".to_owned(),
                "demo.ability.harmonic-spark".to_owned(),
                "demo.ability.resonant-bolt".to_owned(),
            ]
            .as_slice()
        )
    );
    assert_eq!(
        catalog
            .item("demo.item.echo-primer")
            .and_then(|item| item.ability_book_id.as_deref()),
        Some("demo.ability-book.echo-primer")
    );
    assert_eq!(
        catalog
            .class("demo.class.mage")
            .and_then(|class| class.casting_profile.as_ref())
            .map(|profile| (
                profile.resource_id.as_str(),
                profile.casting_attribute,
                profile.base_capacity,
                profile.capacity_per_level,
                profile.capacity_per_attribute_index,
                profile.base_learning_capacity,
                profile.learning_capacity_per_level,
                profile.learning_capacity_per_attribute_index,
                profile.learning_capacity_cap,
                profile.minimum_failure_percent,
                profile.ability_book_ids.as_slice(),
            )),
        Some((
            "demo.resource.mana",
            CastingAttribute::Intelligence,
            4,
            2,
            1,
            2,
            1,
            0,
            16,
            5,
            [
                "demo.ability-book.black-channels".to_owned(),
                "demo.ability-book.echo-primer".to_owned(),
                "demo.ability-book.necronomicon".to_owned(),
                "demo.ability-book.sepulchral-ways".to_owned(),
                "demo.ability-book.stench-of-death".to_owned(),
                "demo.ability-book.stillwater-notes".to_owned(),
            ]
            .as_slice(),
        ))
    );
    assert_eq!(
        catalog
            .class("demo.class.artificer")
            .and_then(|class| class.device_recharge_profile.as_ref())
            .map(|profile| (
                profile.resource_id.as_str(),
                profile.governing_attribute,
                profile.base_capacity,
                profile.capacity_per_level,
                profile.capacity_per_attribute_index,
                profile.power,
                profile.source_item_destruction_one_in,
            )),
        Some((
            "demo.resource.resonance",
            TechniqueAttribute::Intelligence,
            8,
            2,
            1,
            90,
            3,
        ))
    );
    assert_eq!(
        catalog.build("demo.build.vanguard").map(|build| (
            build.race_id.as_str(),
            build.class_id.as_str(),
            build.personality_id.as_str(),
        )),
        Some((
            "demo.race.human",
            "demo.class.warrior",
            "demo.personality.combat",
        ))
    );
    assert_eq!(
        catalog.build("demo.build.warrior").map(|build| (
            build.race_id.as_str(),
            build.class_id.as_str(),
            build.personality_id.as_str(),
        )),
        Some((
            "demo.race.rfb-human",
            "demo.class.warrior",
            "demo.personality.ordinary",
        ))
    );
    assert_eq!(
        catalog.class("demo.class.warrior").map(|class| class
            .starting_items
            .iter()
            .map(|item| (item.item_kind_id.as_str(), item.quantity, item.equipped))
            .collect::<Vec<_>>()),
        Some(vec![
            ("demo.item.arrow", 22, false),
            ("demo.item.broad-sword", 1, true),
            ("demo.item.chain-mail", 1, true),
            ("demo.item.short-bow", 1, true),
        ])
    );
    assert_eq!(
        catalog
            .actor("demo.actor.ember-mote")
            .and_then(|actor| actor.loot_table_id.as_deref()),
        Some("demo.loot-table.ember-mote")
    );
    assert_eq!(
        catalog
            .actor("demo.actor.ember-mote")
            .and_then(|actor| actor.carried_loot_table_id.as_deref()),
        Some("demo.loot-table.ember-mote-carried")
    );
    assert_eq!(
        catalog
            .loot_table("demo.loot-table.ember-mote")
            .map(|table| (table.rolls, table.entries.len())),
        Some((1, 2))
    );
    assert_eq!(
        catalog
            .encounter_table("demo.encounter-table.echo-depths")
            .map(|table| (table.rolls, table.entries.len())),
        Some((1, 5))
    );
    assert_eq!(
        catalog
            .encounter_table("demo.encounter-table.resonance-formations")
            .map(|table| {
                table
                    .entries
                    .iter()
                    .filter(|entry| entry.group.is_some())
                    .count()
            }),
        Some(2)
    );
    assert_eq!(
        catalog
            .encounter_table("demo.encounter-table.resonance-formations")
            .and_then(|table| table.entries.iter().find_map(|entry| entry.group.as_ref()))
            .map(|group| group.pack_ai),
        Some(EncounterPackAiDefinition {
            leader: MonsterPackBehavior::Seek,
            friends: MonsterPackBehavior::Surround,
            escorts: MonsterPackBehavior::GuardLeader,
        })
    );
    assert_eq!(
        catalog
            .theme_table("demo.theme-table.echo-depths")
            .map(|table| table.entries[0].vault_candidates.len()),
        Some(2)
    );
    assert_eq!(
        catalog
            .region_table("demo.region-table.resonance-biomes")
            .map(|table| {
                table
                    .entries
                    .iter()
                    .map(|entry| (entry.region_id.as_str(), entry.weight))
                    .collect::<Vec<_>>()
            }),
        Some(vec![
            ("demo.region.resonance-gallery", 1),
            ("demo.region.resonance-grotto", 3),
        ])
    );
    assert_eq!(
        catalog
            .terrain_feature_table("demo.terrain-feature-table.resonance-hazards")
            .map(|table| (table.rolls, table.entries.len())),
        Some((4, 4))
    );
    let world = catalog
        .world("demo.world.original-v1")
        .expect("demo world should remain available");
    assert_eq!(world.initial_floor_id, "demo.floor.surface");
    assert_eq!(world.dungeons.len(), 3);
    assert_eq!(world.procedural_floors.len(), 24);
    assert_eq!(world.procedural_floors[0].id, "demo.floor.echo-depth-1");
    assert_eq!(world.procedural_floors[0].depth, 1);
    let regional_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-2")
        .expect("demo world should retain its regional floor");
    assert_eq!(
        regional_floor.region_table_id.as_deref(),
        Some("demo.region-table.resonance-biomes")
    );
    assert_eq!(
        regional_floor.generation_budget.as_ref().map(|budget| (
            budget.actor_slots,
            budget.loot_placements,
            budget.region_placements,
        )),
        Some((4, 2, Some(2)))
    );
    assert_eq!(
        world.procedural_floors[0].closed_door_terrain_id,
        "demo.terrain.door-secret"
    );
    assert!(world.procedural_floors[0].actor_spawns.is_empty());
    assert!(world.procedural_floors[0].loot_spawns.is_empty());
    assert_eq!(
        world.procedural_floors[0].encounter_table_id.as_deref(),
        Some("demo.encounter-table.echo-depths")
    );
    assert_eq!(
        world.procedural_floors[0].loot_table_id.as_deref(),
        Some("demo.loot-table.echo-depth-1-room")
    );
    assert_eq!(
        world.procedural_floors[0].theme_table_id.as_deref(),
        Some("demo.theme-table.echo-depths")
    );
    let final_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("demo world should retain the budgeted cavern floor");
    assert_eq!(
        final_floor.generation_budget.as_ref().map(|budget| (
            budget.room_placements,
            budget.room_area_tiles,
            budget.cavern_area_tiles,
            budget.lake_area_tiles,
            budget.lake_deep_area_tiles,
            budget.river_area_tiles,
            budget.destruction_centers,
            budget.destroyed_area_tiles,
            budget.streamer_placements,
            budget.streamer_area_tiles,
        )),
        Some((
            Some(5),
            Some(112),
            Some(64),
            Some(76),
            Some(30),
            Some(52),
            Some(2),
            Some(48),
            Some(2),
            Some(24)
        ))
    );
    assert_eq!(
        final_floor.layout.as_ref().map(|layout| (
            layout.rooms.as_ref().map_or(0, |rooms| rooms.shapes.len()),
            layout
                .cavern
                .as_ref()
                .map(|cavern| cavern.terrain_id.as_str()),
            layout
                .lake
                .as_ref()
                .map(|lake| lake.deep_terrain_id.as_str()),
            layout
                .river
                .as_ref()
                .map(|river| river.shallow_terrain_id.as_str()),
            layout
                .destroyed
                .as_ref()
                .map(|destroyed| destroyed.terrain_id.as_str()),
            layout.streamers.len(),
        )),
        Some((
            2,
            Some("demo.terrain.resonance-cavern"),
            Some("demo.terrain.resonance-water-deep"),
            Some("demo.terrain.resonance-water-shallow"),
            Some("demo.terrain.resonance-ruin"),
            1
        ))
    );
    let maze_floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-9")
        .expect("demo world should retain the maze floor");
    assert_eq!(
        maze_floor.generation_budget.as_ref().map(|budget| (
            budget.maze_floor_tiles,
            budget.streamer_placements,
            budget.streamer_area_tiles
        )),
        Some((Some(127), Some(2), Some(24)))
    );
    assert_eq!(
        maze_floor.layout.as_ref().and_then(|layout| {
            layout
                .maze
                .as_ref()
                .map(|maze| (layout.mode, maze.width, maze.height, layout.streamers.len()))
        }),
        Some((ProceduralLayoutMode::MazeOnly, 15, 15, 1))
    );
    assert_eq!(
        final_floor.layout.as_ref().and_then(|layout| {
            layout.pit.as_ref().map(|pit| {
                (
                    pit.encounter_table_id.as_str(),
                    pit.inner_width,
                    pit.inner_height,
                    pit.roster_size,
                )
            })
        }),
        Some(("demo.encounter-table.resonance-pit", 5, 5, 5))
    );
    assert_eq!(
        final_floor.generation_budget.as_ref().map(|budget| (
            budget.actor_slots,
            budget.pit_placements,
            budget.pit_actor_slots,
        )),
        Some((30, Some(1), Some(25)))
    );
    assert_eq!(
        world.procedural_floors[0]
            .generation_budget
            .as_ref()
            .map(|budget| (budget.actor_slots, budget.loot_placements)),
        Some((4, 1))
    );
    assert_eq!(
        world.procedural_floors[0]
            .nest
            .as_ref()
            .map(|nest| (nest.room_id.as_str(), nest.spawn_count)),
        Some(("remote", 3))
    );
    let pressure_final = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("demo world should contain the pressure final floor");
    assert!(pressure_final.final_floor);
    assert_eq!(pressure_final.depth, 10);
    assert_eq!(
        pressure_final
            .generation_budget
            .as_ref()
            .map(|budget| (budget.actor_slots, budget.loot_placements)),
        Some((30, 3))
    );
    assert_eq!(
        catalog
            .vault("demo.vault.harmonic-sepulcher")
            .map(|vault| (vault.theme_id.as_str(), vault.encounter_groups.len())),
        Some(("demo.theme.echo-depths", 1))
    );
    assert_eq!(
        catalog
            .terrain("demo.terrain.door-closed")
            .and_then(|terrain| terrain.open_to_terrain_id.as_deref()),
        Some("demo.terrain.door-open")
    );
    assert_eq!(
        catalog.terrain("demo.terrain.door-locked").map(|terrain| (
            terrain.open_check_difficulty,
            terrain.bash_to_terrain_id.as_deref(),
            terrain.bash_check_difficulty,
        )),
        Some((Some(24), Some("demo.terrain.door-broken"), Some(18)))
    );
    assert_eq!(
        catalog.terrain("demo.terrain.door-secret").map(|terrain| (
            terrain.concealed_as_terrain_id.as_deref(),
            terrain.search_check_difficulty,
        )),
        Some((Some("demo.terrain.wall"), Some(8)))
    );
    assert_eq!(
        catalog
            .terrain("demo.terrain.door-open")
            .and_then(|terrain| terrain.close_to_terrain_id.as_deref()),
        Some("demo.terrain.door-closed")
    );
    assert_eq!(
        catalog.actor("demo.actor.explorer").map(|actor| (
            actor.door_skill,
            actor.bash_power,
            actor.search_skill
        )),
        Some((24, 30, 24))
    );
    assert_eq!(
        catalog
            .actor("demo.actor.echo-hound")
            .and_then(|actor| actor.melee_routine.as_ref())
            .map(|routine| routine
                .blows
                .iter()
                .map(|blow| blow.method_id.as_str())
                .collect::<Vec<_>>()),
        Some(vec!["rfb.blow.echo-bite", "rfb.blow.echo-rake"])
    );
    assert_eq!(
        catalog
            .item("demo.item.echo-blade")
            .and_then(|item| item.melee_profile.as_ref())
            .map(|profile| (profile.attacks, profile.to_hit, profile.to_damage)),
        Some((2, 10, 1))
    );
    assert_eq!(
        catalog
            .item("demo.item.resonance-sling")
            .and_then(|item| item.projectile_profile.as_ref())
            .map(|profile| (
                profile.range,
                profile.damage_multiplier_percent,
                profile.to_hit,
                profile.to_damage,
                profile.ammunition_type,
            )),
        Some((6, 100, 30, 1, AmmunitionTypeDefinition::Shot))
    );
    assert_eq!(catalog.content_hash(), artifact.content_hash);
    assert_eq!(
        catalog
            .terrain("demo.terrain.wall")
            .map(|terrain| terrain.walkable),
        Some(false)
    );
    assert_eq!(
        catalog
            .actor("demo.actor.ember-mote")
            .map(|actor| actor.max_hp),
        Some(3)
    );
    assert_eq!(
        catalog
            .actor("demo.actor.ember-mote")
            .map(|actor| actor.damage_type),
        Some(ActorDamageType::Fire)
    );
    assert_eq!(
        catalog.actor("demo.actor.explorer").map(|actor| (
            actor.attack,
            actor.defense,
            actor.damage_dice,
            actor.damage_sides,
            actor.speed,
        )),
        Some((2, 1, 1, 2, 110))
    );
    assert_eq!(
        catalog
            .item("demo.item.luminous-shard")
            .map(|item| item.max_stack),
        Some(20)
    );
    assert!(matches!(
        catalog
            .item("demo.item.luminous-shard")
            .and_then(|item| item.use_action.as_ref())
            .map(|action| &action.effect),
        Some(ItemUseEffectDefinition::Heal { amount: 4 })
    ));
    assert_eq!(
        catalog
            .item("demo.item.resonance-stabilizer")
            .and_then(|item| item.use_action.as_ref())
            .and_then(|action| action.device_check_difficulty),
        Some(60)
    );
    assert!(matches!(
        catalog
            .item("demo.item.resonance-stabilizer")
            .and_then(|item| item.use_action.as_ref())
            .map(|action| &action.effect),
        Some(ItemUseEffectDefinition::Heal { amount: 6 })
    ));
    assert!(matches!(
        catalog
            .item("demo.item.resonance-staff")
            .and_then(|item| item.device_generation.as_ref())
            .and_then(|generation| generation.activations.first())
            .map(|activation| &activation.effect),
        Some(ItemUseEffectDefinition::Heal { amount: 50 })
    ));
    assert_eq!(
        catalog
            .actor("demo.actor.echo-listener")
            .and_then(|actor| actor.awareness.as_ref())
            .map(|awareness| (
                awareness.detection_difficulty,
                awareness.detection_range,
                awareness.starts_alerted,
            )),
        Some((7, 8, false))
    );
    assert_eq!(
        catalog
            .terrain("demo.terrain.echo-rune-hidden")
            .and_then(|terrain| terrain.perception_check_difficulty),
        Some(24)
    );
    assert_eq!(
        catalog
            .terrain("demo.terrain.trap-resonance-ward")
            .and_then(|terrain| terrain.trap.as_ref())
            .and_then(|trap| trap.saving_throw_difficulty),
        Some(40)
    );
    assert_eq!(
        catalog
            .item("demo.item.echo-charm")
            .and_then(|item| item.equipment_slot.as_deref()),
        Some("charm")
    );
    assert_eq!(
        catalog
            .item("demo.item.echo-charm")
            .map(|item| item.modifiers.max_hp),
        Some(4)
    );
    assert_eq!(
        catalog
            .item("demo.item.echo-charm")
            .map(|item| (item.modifiers.attack, item.modifiers.defense)),
        Some((1, 1))
    );
    assert_eq!(
        catalog
            .affix("demo.affix.harmonic-edge")
            .map(|affix| affix.modifiers.attack),
        Some(1)
    );
    assert_eq!(
        catalog
            .world("demo.world.original-v1")
            .and_then(|world| world
                .items
                .iter()
                .find(|item| item.kind_id == "demo.item.echo-charm")
                .map(|item| (item.quality, item.affix_ids.as_slice()))),
        Some((
            ItemQuality::Fine,
            ["demo.affix.harmonic-edge".to_owned()].as_slice()
        ))
    );
    assert!(catalog.world("demo.world.original-v1").is_some());
    let warrens = catalog
        .world("demo.world.warrens-journey")
        .expect("Warrens world should remain available");
    assert_eq!((warrens.width, warrens.height), (96, 32));
    assert_eq!(warrens.procedural_floors.len(), 10);
    let warrens_first = &warrens.procedural_floors[0];
    assert_eq!((warrens_first.width, warrens_first.height), (66, 22));
    assert_eq!(
        warrens_first.generation_budget.as_ref().map(|budget| (
            budget.actor_slots,
            budget.room_placements,
            budget.room_area_tiles,
            budget.streamer_placements,
            budget.streamer_area_tiles,
        )),
        Some((4, Some(5), Some(450), Some(2), Some(24)))
    );
    assert_eq!(
        warrens_first.layout.as_ref().map(|layout| (
            layout.rooms.as_ref().map(|rooms| (
                rooms.placement,
                rooms
                    .shapes
                    .iter()
                    .map(|candidate| (candidate.shape, candidate.weight))
                    .collect::<Vec<_>>()
            )),
            layout
                .streamers
                .iter()
                .map(|streamer| (streamer.terrain_id.as_str(), streamer.weight))
                .collect::<Vec<_>>(),
            layout.stairs,
        )),
        Some((
            Some((
                ProceduralRoomPlacement::Free,
                vec![
                    (ProceduralRoomShape::Rectangle, 1),
                    (ProceduralRoomShape::Cavern, 9),
                ]
            )),
            vec![
                ("demo.terrain.magma-vein", 1),
                ("demo.terrain.quartz-vein", 1)
            ],
            Some(ProceduralStairLayoutDefinition {
                up: ProceduralCountRangeDefinition {
                    minimum: 1,
                    maximum: 2,
                },
                down: Some(ProceduralCountRangeDefinition {
                    minimum: 4,
                    maximum: 5,
                }),
            }),
        ))
    );
    assert_eq!(
        warrens.procedural_floors[8]
            .layout
            .as_ref()
            .and_then(|layout| layout.stairs)
            .and_then(|stairs| stairs.down),
        Some(ProceduralCountRangeDefinition {
            minimum: 4,
            maximum: 5,
        })
    );
    assert_eq!(
        catalog.visual_glyphs().get("demo.item.luminous-shard"),
        Some(&"!".to_owned())
    );
}

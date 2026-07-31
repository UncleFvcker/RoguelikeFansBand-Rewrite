use super::*;

fn original_pack_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate should be inside the workspace")
        .join("packs/rfb-demo-original")
}

#[test]
fn source_roots_and_validation_precedence_are_stable() {
    assert_eq!(
        SUPPORTED_ROOTS,
        [
            "abilities",
            "abilityBooks",
            "abilityPrograms",
            "actors",
            "affixes",
            "builds",
            "classes",
            "effectPrograms",
            "encounterTables",
            "items",
            "lootTables",
            "personalities",
            "playerAbilityBindings",
            "races",
            "regionTables",
            "resources",
            "skills",
            "skillSets",
            "terrain",
            "terrainFeatureTables",
            "themeTables",
            "vaults",
            "worlds",
        ]
    );

    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid_metadata = artifact.content.clone();
    invalid_metadata.format = "invalid".to_owned();
    invalid_metadata.pack_id = "INVALID".to_owned();
    invalid_metadata.pack_version = "01.0.0".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_metadata),
        Err(ContentError::InvalidCompiledMetadata)
    ));

    let mut invalid_pack_id = artifact.content.clone();
    invalid_pack_id.pack_id = "INVALID".to_owned();
    invalid_pack_id.pack_version = "01.0.0".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_pack_id),
        Err(ContentError::InvalidStableId(id)) if id == "INVALID"
    ));

    let mut invalid_pack_version = artifact.content.clone();
    invalid_pack_version.pack_version = "01.0.0".to_owned();
    invalid_pack_version.title_key = "INVALID TITLE".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_pack_version),
        Err(ContentError::InvalidPackVersion(version)) if version == "01.0.0"
    ));

    let mut invalid_title = artifact.content.clone();
    invalid_title.title_key = "INVALID TITLE".to_owned();
    invalid_title.dependencies.push(PackDependency {
        id: invalid_title.pack_id.clone(),
        version_requirement: "*".to_owned(),
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid_title),
        Err(ContentError::InvalidMessageKey(key)) if key == "INVALID TITLE"
    ));

    let mut invalid_terrain_schema = artifact.content.clone();
    invalid_terrain_schema.terrain[0].schema = "invalid".to_owned();
    invalid_terrain_schema.terrain[0].format_version = CONTENT_FORMAT_VERSION + 1;
    invalid_terrain_schema.terrain[0].id = "INVALID".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_terrain_schema),
        Err(ContentError::SchemaMismatch(_))
    ));

    let mut invalid_terrain_version = artifact.content.clone();
    invalid_terrain_version.terrain[0].format_version = CONTENT_FORMAT_VERSION + 1;
    invalid_terrain_version.terrain[0].id = "INVALID".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_terrain_version),
        Err(ContentError::UnsupportedSourceVersion { .. })
    ));
}

#[cfg(feature = "schemas")]
#[test]
fn generated_schema_document_order_is_stable() {
    let documents = generated_schema_documents().expect("schemas should generate");
    assert_eq!(
        documents
            .iter()
            .map(|(file_name, _)| *file_name)
            .collect::<Vec<_>>(),
        vec![
            "pack.schema.json",
            "terrain.schema.json",
            "actor.schema.json",
            "item.schema.json",
            "effect-program.schema.json",
            "resource.schema.json",
            "ability.schema.json",
            "ability-program.schema.json",
            "player-ability-binding.schema.json",
            "ability-book.schema.json",
            "skill.schema.json",
            "skill-set.schema.json",
            "race.schema.json",
            "class.schema.json",
            "personality.schema.json",
            "build.schema.json",
            "affix.schema.json",
            "encounter-table.schema.json",
            "loot-table.schema.json",
            "theme-table.schema.json",
            "region-table.schema.json",
            "terrain-feature-table.schema.json",
            "vault.schema.json",
            "world.schema.json",
        ]
    );
}

#[test]
fn original_pack_compiles_deterministically_and_round_trips() {
    let first = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let second = compile_pack_dir(&original_pack_path()).expect("recompile should succeed");
    let decoded = decode_content(&first.bytes).expect("compiled pack should decode");

    assert_eq!(first.content_hash, second.content_hash);
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(decoded, first);
    assert_eq!(first.content.pack_id, "rfb.demo.original-v1");
    assert_eq!(first.content.terrain.len(), 48);
    assert_eq!(first.content.actors.len(), 28);
    assert_eq!(first.content.affixes.len(), 4);
    assert_eq!(first.content.items.len(), 90);
    assert_eq!(first.content.resources.len(), 3);
    assert_eq!(first.content.abilities.len(), 68);
    assert_eq!(first.content.ability_books.len(), 5);
    assert_eq!(first.content.skills.len(), 10);
    assert_eq!(first.content.skill_sets.len(), 13);
    assert_eq!(first.content.races.len(), 4);
    assert_eq!(first.content.classes.len(), 6);
    assert_eq!(first.content.personalities.len(), 3);
    assert_eq!(first.content.builds.len(), 6);
    assert_eq!(first.content.encounter_tables.len(), 6);
    assert_eq!(first.content.loot_tables.len(), 8);
    assert_eq!(first.content.theme_tables.len(), 3);
    assert_eq!(first.content.region_tables.len(), 1);
    assert_eq!(first.content.terrain_feature_tables.len(), 1);
    assert_eq!(first.content.vaults.len(), 6);
    assert_eq!(first.content.worlds.len(), 1);
}

#[test]
fn compiled_catalog_exposes_stable_runtime_indexes() {
    let artifact = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

    assert_eq!(catalog.pack_id(), "rfb.demo.original-v1");
    assert_eq!(catalog.pack_version(), "1.140.0");
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
                profile.to_hit,
                profile.to_damage,
                profile.ammo_kind_id.as_str(),
            )),
        Some((6, 30, 1, "demo.item.resonance-pellet"))
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
            actor.carry_capacity_tenths_pound,
        )),
        Some((2, 1, 1, 2, 110, 100))
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
    assert_eq!(
        catalog.visual_glyphs().get("demo.item.luminous-shard"),
        Some(&"!".to_owned())
    );
}

#[test]
fn observable_rule_entries_require_their_skill_kinds() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (kind, expected) in [
        (SkillKind::Device, "device"),
        (SkillKind::SavingThrow, "saving-throw"),
        (SkillKind::Stealth, "stealth"),
        (SkillKind::Perception, "perception"),
    ] {
        let mut invalid = artifact.content.clone();
        invalid.skills.retain(|skill| skill.kind != kind);
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::MissingRequiredSkillKind(actual)) if actual == expected
        ));
    }
}

#[test]
fn dangling_references_and_checksum_corruption_are_rejected() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    invalid.worlds[0].fill_terrain_id = "demo.terrain.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut blocked_spawn = artifact.content.clone();
    blocked_spawn.worlds[0].player.position = ContentPosition { x: 11, y: 3 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_spawn),
        Err(ContentError::SpawnOnBlockedTerrain(_))
    ));

    let mut corrupted = artifact.bytes;
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0x01;
    assert!(matches!(
        decode_content(&corrupted),
        Err(ContentError::ChecksumMismatch)
    ));
}

#[test]
fn loot_tables_require_valid_weights_references_and_instance_shapes() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_weight = artifact.content.clone();
    zero_weight
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .entries[0]
        .weight = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_weight),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut dangling_affix = artifact.content.clone();
    dangling_affix
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .affix_weights[1]
        .affix_id = Some("demo.affix.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_affix),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut stackable_quality = artifact.content.clone();
    stackable_quality
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.ember-mote")
        .expect("fixture should contain the death loot table")
        .entries[0]
        .item_kind_id = "demo.item.luminous-shard".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut stackable_quality),
        Err(ContentError::InvalidLootTable(_))
    ));

    let mut player_drop = artifact.content.clone();
    let player = player_drop
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain the player");
    player.loot_table_id = Some("demo.loot-table.ember-mote".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut player_drop),
        Err(ContentError::InvalidActorLootTable(_))
    ));

    let mut player_carry = artifact.content.clone();
    let player = player_carry
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain the player");
    player.carried_loot_table_id = Some("demo.loot-table.ember-mote-carried".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut player_carry),
        Err(ContentError::InvalidActorLootTable(_))
    ));
}

#[test]
fn procedural_floor_tables_require_valid_depth_roles_and_references() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_depth = artifact.content.clone();
    zero_depth.worlds[0].procedural_floors[0].depth = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_depth),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut player_candidate = artifact.content.clone();
    player_candidate.encounter_tables[0].entries[0].actor_kind_id =
        "demo.actor.explorer".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut player_candidate),
        Err(ContentError::WrongActorRole(_))
    ));

    let mut dangling_loot = artifact.content.clone();
    dangling_loot.worlds[0].procedural_floors[0].loot_table_id =
        Some("demo.loot-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_loot),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut duplicate_actor = artifact.content.clone();
    duplicate_actor.worlds[0].procedural_floors[0].encounter_table_id = None;
    duplicate_actor.worlds[0].procedural_floors[0].generation_budget = None;
    duplicate_actor.worlds[0].procedural_floors[0].nest = None;
    duplicate_actor.worlds[0].procedural_floors[0]
        .actor_spawns
        .push(ProceduralActorSpawnDefinition {
            instance_id: "demo.monster.ember-mote.1".to_owned(),
            room_id: "remote".to_owned(),
            actor_kind_ids: vec!["demo.actor.echo-hound".to_owned()],
        });
    assert!(matches!(
        validate_and_normalize(&mut duplicate_actor),
        Err(ContentError::DuplicateInstanceId(_))
    ));

    let mut zero_weight = artifact.content.clone();
    zero_weight.encounter_tables[0].entries[0].weight = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_weight),
        Err(ContentError::InvalidEncounterTable(_))
    ));

    let mut missing_theme = artifact.content.clone();
    missing_theme.worlds[0].procedural_floors[0].theme_table_id =
        Some("demo.theme-table.missing".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut missing_theme),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut exhausted_actor_budget = artifact.content.clone();
    exhausted_actor_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain the nest floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .actor_slots = 3;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_actor_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut exhausted_loot_budget = artifact.content.clone();
    exhausted_loot_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-2")
        .expect("fixture should contain the vault floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .loot_placements = 1;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_loot_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_spatial_budget = artifact.content.clone();
    incomplete_spatial_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-8")
        .expect("fixture should contain the spatial Vault floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .vault_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_spatial_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_group_budget = artifact.content.clone();
    incomplete_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .expect("fixture should contain the dynamic group floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .group_actor_slots = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut undersized_group_budget = artifact.content.clone();
    undersized_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .expect("fixture should contain the dynamic group floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .group_actor_slots = Some(1);
    assert!(matches!(
        validate_and_normalize(&mut undersized_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut player_escort = artifact.content.clone();
    player_escort
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.resonance-formations")
        .expect("fixture should contain the formation encounter table")
        .entries
        .iter_mut()
        .find_map(|entry| entry.group.as_mut())
        .and_then(|group| group.escort.as_mut())
        .expect("fixture should contain an escort table")
        .entries[0]
        .actor_kind_id = "demo.actor.explorer".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut player_escort),
        Err(ContentError::WrongActorRole(_))
    ));

    let mut self_guarding_leader = artifact.content.clone();
    self_guarding_leader
        .encounter_tables
        .iter_mut()
        .find(|table| table.id == "demo.encounter-table.resonance-formations")
        .expect("fixture should contain the formation encounter table")
        .entries
        .iter_mut()
        .find_map(|entry| entry.group.as_mut())
        .expect("fixture should contain a dynamic group")
        .pack_ai
        .leader = MonsterPackBehavior::GuardLeader;
    assert!(matches!(
        validate_and_normalize(&mut self_guarding_leader),
        Err(ContentError::InvalidEncounterTable(_))
    ));

    let mut invalid_feature_terrain = artifact.content.clone();
    invalid_feature_terrain.terrain_feature_tables[0].entries[0].terrain_id =
        "demo.terrain.floor".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_feature_terrain),
        Err(ContentError::InvalidTerrainFeatureTable(_))
    ));

    let mut incomplete_feature_budget = artifact.content.clone();
    incomplete_feature_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-3")
        .expect("fixture should contain the feature-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .feature_placements = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_feature_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut oversized_feature_budget = artifact.content.clone();
    oversized_feature_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-3")
        .expect("fixture should contain the feature-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .feature_placements = Some(5);
    assert!(matches!(
        validate_and_normalize(&mut oversized_feature_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_room_budget = artifact.content.clone();
    incomplete_room_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .room_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_room_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut undersized_room_budget = artifact.content.clone();
    undersized_room_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-budget floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .room_area_tiles = Some(35);
    assert!(matches!(
        validate_and_normalize(&mut undersized_room_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut blocked_cavern = artifact.content.clone();
    blocked_cavern.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the cavern floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .cavern
        .as_mut()
        .expect("fixture should contain a cavern")
        .terrain_id = "demo.terrain.wall".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut blocked_cavern),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_cavern_budget = artifact.content.clone();
    incomplete_cavern_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the cavern floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .cavern_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_cavern_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incomplete_lake_budget = artifact.content.clone();
    incomplete_lake_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the lake floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .lake_deep_area_tiles = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_lake_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut walkable_deep_water = artifact.content.clone();
    walkable_deep_water
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.resonance-water-deep")
        .expect("fixture should contain deep water")
        .walkable = true;
    assert!(matches!(
        validate_and_normalize(&mut walkable_deep_water),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut incompatible_river = artifact.content.clone();
    incompatible_river.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the river floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .river
        .as_mut()
        .expect("fixture should contain a river")
        .shallow_terrain_id = "demo.terrain.floor".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut incompatible_river),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_maze_budget = artifact.content.clone();
    mismatched_maze_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-9")
        .expect("fixture should contain the maze floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .maze_floor_tiles = Some(126);
    assert!(matches!(
        validate_and_normalize(&mut mismatched_maze_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut maze_with_rooms = artifact.content.clone();
    let room_geometry = maze_with_rooms.worlds[0]
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.layout.as_ref())
        .and_then(|layout| layout.rooms.clone())
        .expect("fixture should contain room geometry");
    maze_with_rooms.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-9")
        .and_then(|floor| floor.layout.as_mut())
        .expect("fixture should contain the maze-only layout")
        .rooms = Some(room_geometry);
    assert!(matches!(
        validate_and_normalize(&mut maze_with_rooms),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut room_overlay_maze = artifact.content.clone();
    let final_floor = room_overlay_maze.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the rooms floor");
    final_floor
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .maze_floor_tiles = Some(127);
    final_floor
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .maze = Some(ProceduralMazeDefinition {
        width: 15,
        height: 15,
    });
    assert!(matches!(
        validate_and_normalize(&mut room_overlay_maze),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_pit_budget = artifact.content.clone();
    mismatched_pit_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the pit floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .pit_actor_slots = Some(24);
    assert!(matches!(
        validate_and_normalize(&mut mismatched_pit_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut dangling_pit_table = artifact.content.clone();
    dangling_pit_table.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the pit floor")
        .layout
        .as_mut()
        .and_then(|layout| layout.pit.as_mut())
        .expect("fixture should contain a pit")
        .encounter_table_id = "demo.encounter-table.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut dangling_pit_table),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut incomplete_destroyed_budget = artifact.content.clone();
    incomplete_destroyed_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the destroyed floor")
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .destruction_centers = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_destroyed_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut walkable_streamer = artifact.content.clone();
    walkable_streamer
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.resonance-vein")
        .expect("fixture should contain the streamer terrain")
        .walkable = true;
    assert!(validate_and_normalize(&mut walkable_streamer).is_err());

    let mut duplicate_room_shape = artifact.content.clone();
    let shapes = &mut duplicate_room_shape.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the room-layout floor")
        .layout
        .as_mut()
        .expect("fixture should contain a layout")
        .rooms
        .as_mut()
        .expect("fixture should contain room geometry")
        .shapes;
    shapes[1].shape = shapes[0].shape;
    assert!(matches!(
        validate_and_normalize(&mut duplicate_room_shape),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn region_tables_require_depth_eligible_candidates_and_composable_budgets() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    fn regional_floor(content: &mut CompiledContentV1) -> &mut ProceduralFloorDefinition {
        content.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-2")
            .expect("fixture should contain the regional floor")
    }

    let mut exhausted_depth = artifact.content.clone();
    regional_floor(&mut exhausted_depth).depth = 11;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_depth),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_budget = artifact.content.clone();
    regional_floor(&mut missing_budget)
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .region_placements = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut oversized_budget = artifact.content.clone();
    regional_floor(&mut oversized_budget)
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .region_placements = Some(3);
    assert!(matches!(
        validate_and_normalize(&mut oversized_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mixed_floor_tables = artifact.content.clone();
    regional_floor(&mut mixed_floor_tables).encounter_table_id =
        Some("demo.encounter-table.resonance-descent".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut mixed_floor_tables),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut composable_features = artifact.content.clone();
    composable_features.terrain_feature_tables[0].entries[0].min_depth = 2;
    let floor = regional_floor(&mut composable_features);
    floor.terrain_feature_table_id =
        Some("demo.terrain-feature-table.resonance-hazards".to_owned());
    floor
        .generation_budget
        .as_mut()
        .expect("regional floor should retain a generation budget")
        .feature_placements = Some(1);
    validate_and_normalize(&mut composable_features)
        .expect("regional feature, theme, vault, and connections should compose");

    let mut missing_theme = artifact.content.clone();
    missing_theme.region_tables[0].entries[0].theme_id = "demo.theme.resonance-missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut missing_theme),
        Err(ContentError::InvalidRegionTable(_))
    ));

    let mut incomplete_group_budget = artifact.content.clone();
    let budget = incomplete_group_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional group budget");
    budget.group_actor_slots = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_group_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut exhausted_special_actor_budget = artifact.content.clone();
    exhausted_special_actor_budget.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional pit budget")
        .actor_slots = 27;
    assert!(matches!(
        validate_and_normalize(&mut exhausted_special_actor_budget),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut pit_consumes_too_many_rooms = artifact.content.clone();
    pit_consumes_too_many_rooms.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .and_then(|floor| floor.generation_budget.as_mut())
        .expect("fixture should contain the regional pit budget")
        .room_placements = Some(2);
    assert!(matches!(
        validate_and_normalize(&mut pit_consumes_too_many_rooms),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn vaults_require_walkable_unique_positions_and_depth_eligible_encounters() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut blocked_member = artifact.content.clone();
    blocked_member.vaults[0].encounter_groups[0].member_positions[0] =
        ContentPosition { x: 0, y: 0 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_member),
        Err(ContentError::InvalidVault(_))
    ));

    let mut duplicate_transform = artifact.content.clone();
    let transform = duplicate_transform.vaults[0]
        .transforms
        .first()
        .copied()
        .unwrap_or(VaultTransform::Identity);
    duplicate_transform.vaults[0].transforms = vec![transform, transform];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_transform),
        Err(ContentError::InvalidVault(_))
    ));

    let mut interior_entrance = artifact.content.clone();
    let vault = interior_entrance
        .vaults
        .iter_mut()
        .find(|vault| vault.width >= 4 && vault.height >= 4)
        .expect("fixture should contain a large Vault");
    vault.entrance_positions = vec![ContentPosition { x: 1, y: 1 }];
    assert!(matches!(
        validate_and_normalize(&mut interior_entrance),
        Err(ContentError::InvalidVault(_))
    ));

    let mut duplicate_entrance = artifact.content.clone();
    let entrance = duplicate_entrance.vaults[0].entrance_positions[0];
    duplicate_entrance.vaults[0].entrance_positions = vec![entrance, entrance];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_entrance),
        Err(ContentError::InvalidVault(_))
    ));

    let mut disconnected_interior = artifact.content.clone();
    let vault = disconnected_interior
        .vaults
        .iter_mut()
        .find(|vault| vault.id == "demo.vault.harmonic-sepulcher")
        .expect("fixture should contain the sepulcher Vault");
    vault
        .terrain_overrides
        .iter_mut()
        .find(|terrain| terrain.terrain_id == "demo.terrain.wall")
        .expect("fixture should contain Vault walls")
        .positions
        .extend((1..5).map(|x| ContentPosition { x, y: 2 }));
    assert!(matches!(
        validate_and_normalize(&mut disconnected_interior),
        Err(ContentError::InvalidVault(_))
    ));

    let mut legacy_entrance = artifact.content.clone();
    let entrance = legacy_entrance.vaults[0].entrance_positions[0];
    legacy_entrance.vaults[0].entrance_positions.clear();
    legacy_entrance.vaults[0].entrance_position = Some(entrance);
    validate_and_normalize(&mut legacy_entrance)
        .expect("legacy single Vault entrance should normalize");
    assert_eq!(legacy_entrance.vaults[0].entrance_position, None);
    assert_eq!(legacy_entrance.vaults[0].entrance_positions, [entrance]);

    let mut theme_mismatch = artifact.content.clone();
    theme_mismatch.vaults[0].theme_id = "demo.theme.other".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut theme_mismatch),
        Err(ContentError::InvalidThemeTable(_))
    ));

    let mut no_depth_candidate = artifact.content.clone();
    for entry in &mut no_depth_candidate.vaults[0].encounter_groups[0].entries {
        entry.min_depth = 1;
        entry.max_depth = 1;
    }
    assert!(matches!(
        validate_and_normalize(&mut no_depth_candidate),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn staged_tasks_require_ordered_member_floor_objectives() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut outside_member = artifact.content.clone();
    outside_member.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-chain-rift")
        .expect("fixture should contain the staged task")
        .task_stages[1]
        .floor_id = Some("demo.floor.echo-bounty-rift".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut outside_member),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut duplicate_action_floor = artifact.content.clone();
    duplicate_action_floor.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-chain-rift")
        .expect("fixture should contain the staged task")
        .task_stages[2]
        .floor_id = Some("demo.floor.echo-chain-rift".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut duplicate_action_floor),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut non_retakeable = artifact.content.clone();
    for floor in non_retakeable.worlds[0]
        .procedural_floors
        .iter_mut()
        .filter(|floor| floor.task_id.as_deref() == Some("demo.task.echo-chain"))
    {
        floor.retakeable = false;
    }
    assert!(matches!(
        validate_and_normalize(&mut non_retakeable),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut zero_limit = artifact.content.clone();
    zero_limit.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-bounty-rift")
        .expect("fixture should contain the retakeable bounty")
        .max_retakes = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut zero_limit),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_policy = artifact.content.clone();
    mismatched_policy.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-bounty-annex-rift")
        .expect("fixture should contain the shared bounty member")
        .retake_floor_policy = RetakeFloorPolicy::PreserveFloor;
    assert!(matches!(
        validate_and_normalize(&mut mismatched_policy),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn dungeon_trees_require_shared_guardian_mirrors() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut missing_guardian = artifact.content.clone();
    missing_guardian.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor")
        .guardian = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut broken_chain = artifact.content.clone();
    broken_chain.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor")
        .dungeon_id = Some("demo.dungeon.other".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut broken_chain),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut final_with_descent = artifact.content.clone();
    let final_floor = final_with_descent.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3")
        .expect("fixture should contain the final floor");
    final_floor.next_floor_id = Some("demo.floor.echo-depth-1".to_owned());
    final_floor.down_stair_terrain_id = Some("demo.terrain.stairs-down".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut final_with_descent),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut mismatched_guardian = artifact.content.clone();
    mismatched_guardian.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
        .expect("fixture should contain a guardian mirror")
        .guardian
        .as_mut()
        .expect("mirror should retain a guardian")
        .actor_kind_id = "demo.actor.echo-hound".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut mismatched_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut converging_tree = artifact.content.clone();
    let child_parent = converging_tree.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-2-mirror")
        .expect("fixture should contain the mirror branch");
    child_parent
        .connections
        .push(ProceduralFloorConnectionDefinition {
            id: "demo.connection.test.second-parent-down".to_owned(),
            kind: FloorConnectionKind::Stairs,
            terrain_id: "demo.terrain.stairs-down".to_owned(),
            target_floor_id: "demo.floor.echo-depth-3-mirror".to_owned(),
            target_connection_id: Some("demo.connection.test.second-parent-up".to_owned()),
            target_candidates: Vec::new(),
        });
    let child = converging_tree.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
        .expect("fixture should contain the existing mirror final");
    child.connections.push(ProceduralFloorConnectionDefinition {
        id: "demo.connection.test.second-parent-up".to_owned(),
        kind: FloorConnectionKind::Stairs,
        terrain_id: "demo.terrain.stairs-up".to_owned(),
        target_floor_id: "demo.floor.echo-depth-2-mirror".to_owned(),
        target_connection_id: Some("demo.connection.test.second-parent-down".to_owned()),
        target_candidates: Vec::new(),
    });
    assert!(matches!(
        validate_and_normalize(&mut converging_tree),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn dungeon_entrance_guardians_and_entry_requirements_are_validated() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let world = &artifact.content.worlds[0];
    let resonance = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.resonance-descent")
        .expect("demo should contain the resonance dungeon");
    let entrance = resonance
        .entrance_guardian
        .as_ref()
        .expect("resonance should declare an entrance guardian");
    assert_eq!(entrance.position, ContentPosition { x: 2, y: 1 });
    assert!(resonance.entry_requirements.is_empty());

    let mut zero_ttl = artifact.content.clone();
    zero_ttl.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.archive-depths")
        .expect("archive dungeon should remain available")
        .instance_lifecycle = DungeonInstanceLifecycle::TurnTtl { ttl_turns: 0 };
    assert!(matches!(
        validate_and_normalize(&mut zero_ttl),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut blocked_guardian = artifact.content.clone();
    blocked_guardian.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == resonance.id)
        .expect("resonance should remain available")
        .entrance_guardian
        .as_mut()
        .expect("entrance guardian should remain available")
        .position = ContentPosition { x: 3, y: 2 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_guardian),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut duplicate_requirement = artifact.content.clone();
    let dungeon = duplicate_requirement.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
        .expect("echo dungeon should remain available");
    let requirement = DungeonEntryRequirementDefinition::CarriedItem {
        item_kind_id: "demo.item.luminous-shard".to_owned(),
        quantity: 1,
    };
    dungeon.entry_requirements = vec![requirement.clone(), requirement];
    assert!(matches!(
        validate_and_normalize(&mut duplicate_requirement),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut dangling_requirement = artifact.content.clone();
    dangling_requirement.worlds[0]
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
        .expect("echo dungeon should remain available")
        .entry_requirements = vec![DungeonEntryRequirementDefinition::TaskStatus {
        task_id: "demo.task.missing".to_owned(),
        status: DungeonEntryTaskStatus::Completed,
    }];
    assert!(matches!(
        validate_and_normalize(&mut dangling_requirement),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn floor_connections_require_reciprocal_targets_and_matching_terrain_roles() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut broken_pair = artifact.content.clone();
    broken_pair.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .connections
        .iter_mut()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.down-a")
        .expect("fixture should contain the first downward connection")
        .target_connection_id = Some("demo.connection.echo-depth-2.up-b".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut broken_pair),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut wrong_shaft_kind = artifact.content.clone();
    wrong_shaft_kind.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .connections
        .iter_mut()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.shaft-down")
        .expect("fixture should contain the downward shaft")
        .kind = FloorConnectionKind::Stairs;
    assert!(matches!(
        validate_and_normalize(&mut wrong_shaft_kind),
        Err(ContentError::InvalidProceduralFloor(_))
    ));

    let mut missing_entry = artifact.content.clone();
    missing_entry.worlds[0]
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.echo-depth-1")
        .expect("fixture should contain echo depth one")
        .entry_connection_id = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_entry),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn door_terrain_transitions_are_reciprocal_and_match_collision() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut missing_reciprocal = artifact.content.clone();
    missing_reciprocal
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-closed")
        .expect("fixture should contain the closed door")
        .open_to_terrain_id = None;
    assert!(matches!(
        validate_and_normalize(&mut missing_reciprocal),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut blocked_open_door = artifact.content.clone();
    blocked_open_door
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-open")
        .expect("fixture should contain the open door")
        .walkable = false;
    assert!(matches!(
        validate_and_normalize(&mut blocked_open_door),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut incomplete_bash = artifact.content.clone();
    incomplete_bash
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-locked")
        .expect("fixture should contain the locked door")
        .bash_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_bash),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut invalid_lock = artifact.content.clone();
    invalid_lock
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-locked")
        .expect("fixture should contain the locked door")
        .open_check_difficulty = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut invalid_lock),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut incomplete_concealment = artifact.content.clone();
    incomplete_concealment
        .terrain
        .iter_mut()
        .find(|terrain| terrain.id == "demo.terrain.door-secret")
        .expect("fixture should contain the secret door")
        .search_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut incomplete_concealment),
        Err(ContentError::InvalidTerrainTransition(_))
    ));

    let mut non_door_generator = artifact.content.clone();
    non_door_generator.worlds[0].procedural_floors[0].closed_door_terrain_id =
        "demo.terrain.wall".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut non_door_generator),
        Err(ContentError::InvalidProceduralFloor(_))
    ));
}

#[test]
fn equippable_items_require_a_valid_slot_and_single_item_stack() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the shard");
    shard.equipment_slot = Some("charm".to_owned());

    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidEquipmentSlot(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the shard");
    shard.modifiers.max_hp = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemModifiers(_))
    ));

    let mut invalid = artifact.content.clone();
    let pellet = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-pellet")
        .expect("fixture should contain the ammunition");
    pellet.break_chance_percent = 101;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemBreakChance(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the throwable shard");
    shard.weight_tenths_pound = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemWeight(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the throwable shard");
    shard.appearance_name_key = Some(shard.name_key.clone());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemAppearance(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the usable shard");
    shard.use_action = Some(ItemUseActionDefinition {
        device_check_difficulty: None,
        charges: None,
        effect: ItemUseEffectDefinition::Heal { amount: 0 },
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid.affixes[0].modifiers = StatModifiers::default();
    invalid.affixes[0].equipment_bonuses = EquipmentBonuses::default();
    invalid.affixes[0].resistances.clear();
    invalid.affixes[0].status_immunities.clear();
    invalid.affixes[0].slays.clear();
    invalid.affixes[0].brands.clear();
    invalid.affixes[0].passives.clear();
    invalid.affixes[0].roll_groups.clear();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidAffixModifiers(_))
    ));

    let mut invalid = artifact.content.clone();
    let charm = invalid.worlds[0]
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("fixture should contain the charm");
    charm.affix_ids.push("demo.affix.harmonic-edge".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemAffixes(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid.worlds[0]
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("fixture should contain the shard");
    shard.quality = ItemQuality::Fine;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemAffixes(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the throwable shard");
    shard
        .throw_profile
        .as_mut()
        .expect("shard should have a throw profile")
        .damage_dice = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidThrowProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let blade = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.echo-blade")
        .expect("fixture should contain the blade");
    blade.equipment_slot = Some("charm".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidAttackProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let sling = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-sling")
        .expect("fixture should contain the sling");
    sling.equipment_slot = Some("weapon".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidProjectileProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-sling")
        .expect("fixture should contain the sling")
        .projectile_profile
        .as_mut()
        .expect("sling should have a projectile profile")
        .ammo_kind_id = "demo.item.missing-ammunition".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::DanglingReference { .. })
    ));
}

#[test]
fn charged_item_actions_require_bounded_single_instance_devices() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let action = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.resonance-mender")
        .and_then(|item| item.use_action.as_ref())
        .expect("fixture should contain the charged device action");
    assert_eq!(
        action.charges,
        Some(ItemChargeDefinition {
            initial: 3,
            maximum: 3,
            cost: 1,
        })
    );
    assert!(matches!(
        action.effect,
        ItemUseEffectDefinition::HealDice { dice: 2, sides: 4 }
    ));

    let mut invalid = artifact.content.clone();
    let mender = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-mender")
        .expect("fixture should contain the charged device");
    mender
        .use_action
        .as_mut()
        .and_then(|action| action.charges.as_mut())
        .expect("charged action should exist")
        .maximum = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let mender = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-mender")
        .expect("fixture should contain the charged device");
    mender.max_stack = 2;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let mender = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-mender")
        .expect("fixture should contain the charged device");
    mender
        .use_action
        .as_mut()
        .expect("charged action should exist")
        .device_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));
}

#[test]
fn restorative_item_sequences_require_bounded_effects_and_known_resources() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let clarity = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.clarity-draught")
        .and_then(|item| item.use_action.as_ref())
        .expect("fixture should contain the clarity action");
    let ItemUseEffectDefinition::Sequence { effects } = &clarity.effect else {
        panic!("clarity should use an ordered effect sequence");
    };
    assert!(matches!(
        &effects[0],
        ItemUseEffectDefinition::RestoreResourceDice {
            resource_id,
            dice: 3,
            sides: 6,
            bonus: 3,
        } if resource_id == "demo.resource.mana"
    ));
    assert!(matches!(
        &effects[1],
        ItemUseEffectDefinition::RemoveStatus { status_kind_id }
            if status_kind_id == "rfb.status.confusion"
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.clarity-draught")
        .and_then(|item| item.use_action.as_mut())
        .expect("clarity action should exist")
        .effect = ItemUseEffectDefinition::RestoreResourceFull {
        resource_id: "demo.resource.missing".to_owned(),
    };
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let action = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.clarity-draught")
        .and_then(|item| item.use_action.as_mut())
        .expect("clarity action should exist");
    action.effect = ItemUseEffectDefinition::Sequence {
        effects: vec![
            ItemUseEffectDefinition::Heal { amount: 1 },
            ItemUseEffectDefinition::Sequence {
                effects: vec![
                    ItemUseEffectDefinition::Heal { amount: 1 },
                    ItemUseEffectDefinition::Heal { amount: 1 },
                ],
            },
        ],
    };
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));
}

#[test]
fn dynamic_devices_require_stable_profiles_depth_coverage_and_capacity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let wand = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.resonance-wand")
        .and_then(|item| item.device_generation.as_ref())
        .expect("fixture should contain dynamic wand profiles");
    assert_eq!(
        wand.activations
            .iter()
            .map(|activation| activation.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "demo.device-activation.frost-bolt",
            "demo.device-activation.spark-bolt",
        ]
    );
    assert_eq!(
        wand.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 10,
            energy_per_mille: 10,
        })
    );

    let mut invalid = artifact.content.clone();
    let profiles = &mut invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-wand")
        .expect("fixture should contain the dynamic wand")
        .device_generation
        .as_mut()
        .expect("dynamic generation should exist")
        .activations;
    profiles
        .iter_mut()
        .find(|profile| profile.id == "demo.device-activation.spark-bolt")
        .expect("shallow profile should exist")
        .min_depth = 2;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-wand")
        .and_then(|item| item.device_generation.as_mut())
        .and_then(|generation| generation.recovery.as_mut())
        .expect("dynamic wand should recover")
        .energy_per_mille = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let wand = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-wand")
        .expect("fixture should contain the dynamic wand");
    wand.device_generation
        .as_mut()
        .expect("dynamic generation should exist")
        .activations[0]
        .charges
        .cost = 1_000_001;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let wand = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-wand")
        .expect("fixture should contain the dynamic wand");
    wand.use_action = Some(ItemUseActionDefinition {
        device_check_difficulty: None,
        charges: None,
        effect: ItemUseEffectDefinition::Heal { amount: 1 },
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));
}

#[test]
fn device_recharge_profiles_require_distinct_bounded_resources() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    invalid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.artificer")
        .and_then(|class| class.device_recharge_profile.as_mut())
        .expect("artificer should recharge devices")
        .source_item_destruction_one_in = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidDeviceRechargeProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let mage = invalid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .expect("mage class should exist");
    mage.device_recharge_profile = Some(DeviceRechargeProfileDefinition {
        resource_id: "demo.resource.mana".to_owned(),
        governing_attribute: TechniqueAttribute::Intelligence,
        base_capacity: 1,
        capacity_per_level: 0,
        capacity_per_attribute_index: 0,
        power: 90,
        source_item_destruction_one_in: 3,
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidDeviceRechargeProfile(_))
    ));
}

#[test]
fn ability_books_require_consistent_resources_items_and_casting_profiles() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid_recovery = artifact.content.clone();
    invalid_recovery.resources[0].rest_recovery_amount = 1_000_001;
    assert!(matches!(
        validate_and_normalize(&mut invalid_recovery),
        Err(ContentError::InvalidResource(_))
    ));

    let mut invalid_healing_target = artifact.content.clone();
    invalid_healing_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.mending-echo")
        .expect("fixture should contain the healing ability")
        .target
        .range = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_healing_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_area_radius = artifact.content.clone();
    let AbilityEffectDefinition::AreaDamage { radius, .. } = &mut invalid_area_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-burst")
        .expect("fixture should contain the area damage ability")
        .effect
    else {
        panic!("echo burst should use area damage");
    };
    *radius = 17;
    assert!(matches!(
        validate_and_normalize(&mut invalid_area_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_beam_target = artifact.content.clone();
    invalid_beam_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-lance")
        .expect("fixture should contain the beam damage ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::SelfTarget];
    assert!(matches!(
        validate_and_normalize(&mut invalid_beam_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_cone_radius = artifact.content.clone();
    let AbilityEffectDefinition::ConeDamage { radius, .. } = &mut invalid_cone_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-fan")
        .expect("fixture should contain the cone damage ability")
        .effect
    else {
        panic!("echo fan should use cone damage");
    };
    *radius = 17;
    assert!(matches!(
        validate_and_normalize(&mut invalid_cone_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_cone_target = artifact.content.clone();
    invalid_cone_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-fan")
        .expect("fixture should contain the cone damage ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::Position];
    assert!(matches!(
        validate_and_normalize(&mut invalid_cone_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_teleport_target = artifact.content.clone();
    invalid_teleport_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-step")
        .expect("fixture should contain the teleport ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::Entity];
    assert!(matches!(
        validate_and_normalize(&mut invalid_teleport_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_detect_category = artifact.content.clone();
    let AbilityEffectDefinition::Detect { category, .. } = &mut invalid_detect_category
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-sight")
        .expect("fixture should contain the persistent detection ability")
        .effect
    else {
        panic!("echo sight should use detection");
    };
    *category = "missing-category".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_detect_category),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_detect_radius = artifact.content.clone();
    let AbilityEffectDefinition::Detect { radius, .. } = &mut invalid_detect_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-sight")
        .expect("fixture should contain the persistent detection ability")
        .effect
    else {
        panic!("echo sight should use detection");
    };
    *radius = 9;
    assert!(matches!(
        validate_and_normalize(&mut invalid_detect_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_detect_target = artifact.content.clone();
    invalid_detect_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-pulse")
        .expect("fixture should contain the transient detection ability")
        .target
        .range = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_detect_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut duplicate_transform_source = artifact.content.clone();
    let AbilityEffectDefinition::TransformTerrain {
        source_terrain_ids, ..
    } = &mut duplicate_transform_source
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-delving")
        .expect("fixture should contain the digging terrain ability")
        .effect
    else {
        panic!("echo delving should transform terrain");
    };
    source_terrain_ids.push("demo.terrain.wall".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut duplicate_transform_source),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_transform_target = artifact.content.clone();
    let AbilityEffectDefinition::TransformTerrain {
        target_terrain_id, ..
    } = &mut invalid_transform_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-rampart")
        .expect("fixture should contain the terrain creation ability")
        .effect
    else {
        panic!("echo rampart should transform terrain");
    };
    *target_terrain_id = "demo.terrain.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_transform_target),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut invalid_transform_radius = artifact.content.clone();
    let AbilityEffectDefinition::TransformTerrain { radius, .. } = &mut invalid_transform_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-delving")
        .expect("fixture should contain the digging terrain ability")
        .effect
    else {
        panic!("echo delving should transform terrain");
    };
    *radius = 9;
    assert!(matches!(
        validate_and_normalize(&mut invalid_transform_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_transform_target_mode = artifact.content.clone();
    invalid_transform_target_mode
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-rampart")
        .expect("fixture should contain the terrain creation ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::Direction];
    assert!(matches!(
        validate_and_normalize(&mut invalid_transform_target_mode),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_empty_sequence = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_empty_sequence
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-quickening")
        .expect("fixture should contain the self status sequence")
        .effect
    else {
        panic!("echo quickening should use an effect sequence");
    };
    effects.clear();
    assert!(matches!(
        validate_and_normalize(&mut invalid_empty_sequence),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_nested_sequence = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_nested_sequence
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-binding")
        .expect("fixture should contain the target status sequence")
        .effect
    else {
        panic!("echo binding should use an effect sequence");
    };
    effects[0] = AbilityEffectDefinition::Sequence {
        effects: effects.clone(),
    };
    assert!(matches!(
        validate_and_normalize(&mut invalid_nested_sequence),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_status_duration = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_status_duration
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-binding")
        .expect("fixture should contain the target status sequence")
        .effect
    else {
        panic!("echo binding should use an effect sequence");
    };
    let AbilityEffectDefinition::ApplyStatus { duration_ticks, .. } = &mut effects[1] else {
        panic!("echo binding should apply slow second");
    };
    *duration_ticks = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_status_duration),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_self_sequence_member = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_self_sequence_member
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-quickening")
        .expect("fixture should contain the self status sequence")
        .effect
    else {
        panic!("echo quickening should use an effect sequence");
    };
    effects.push(AbilityEffectDefinition::Damage {
        damage_dice: 1,
        damage_sides: 1,
        damage_bonus: 0,
        damage_type: ActorDamageType::Physical,
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid_self_sequence_member),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_proficiency = artifact.content.clone();
    invalid_proficiency.abilities[0].proficiency.cap = 1_601;
    assert!(matches!(
        validate_and_normalize(&mut invalid_proficiency),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_cooldown = artifact.content.clone();
    invalid_cooldown
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.mending-echo")
        .expect("fixture should contain the healing ability")
        .cooldown
        .as_mut()
        .expect("healing ability should declare a cooldown")
        .turns = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_cooldown),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut dangling_resource = artifact.content.clone();
    dangling_resource.abilities[0].resource_id = "demo.resource.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut dangling_resource),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut invalid_book_item = artifact.content.clone();
    let primer = invalid_book_item
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.echo-primer")
        .expect("fixture should contain the ability book item");
    primer.max_stack = 2;
    assert!(matches!(
        validate_and_normalize(&mut invalid_book_item),
        Err(ContentError::InvalidAbilityBookItem(_))
    ));

    let mut mismatched_profile = artifact.content;
    let mut focus = mismatched_profile.resources[0].clone();
    focus.id = "demo.resource.focus".to_owned();
    mismatched_profile.resources.push(focus);
    mismatched_profile
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .expect("fixture should contain the mage class")
        .casting_profile
        .as_mut()
        .expect("mage should have a casting profile")
        .resource_id = "demo.resource.focus".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut mismatched_profile),
        Err(ContentError::InvalidCastingProfile(_))
    ));
}

#[test]
fn casting_profiles_validate_per_ability_parameter_overrides() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut overridden = artifact.content;
    let profile = overridden
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("fixture should contain the mage casting profile");
    profile
        .ability_overrides
        .push(AbilityCastingOverrideDefinition {
            ability_id: "demo.ability.mending-echo".to_owned(),
            minimum_level: 7,
            resource_cost: 11,
            base_failure_percent: 42,
            level_scaling: Vec::new(),
        });
    validate_and_normalize(&mut overridden).expect("valid override should compile");

    let mut duplicate = overridden.clone();
    duplicate
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("fixture should contain the mage casting profile")
        .ability_overrides
        .push(AbilityCastingOverrideDefinition {
            ability_id: "demo.ability.mending-echo".to_owned(),
            minimum_level: 8,
            resource_cost: 12,
            base_failure_percent: 43,
            level_scaling: Vec::new(),
        });
    assert!(matches!(
        validate_and_normalize(&mut duplicate),
        Err(ContentError::InvalidCastingProfile(_))
    ));

    let mut unsupported = overridden;
    unsupported
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("fixture should contain the mage casting profile")
        .ability_overrides[0]
        .ability_id = "demo.ability.not-in-a-mage-book".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut unsupported),
        Err(ContentError::InvalidCastingProfile(_))
    ));
}

#[test]
fn abilities_validate_actor_detection_control_and_level_scaling() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut valid = artifact.content.clone();
    let malediction = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("fixture should contain level-scaled damage");
    assert_eq!(malediction.level_scaling.len(), 1);
    let unlife = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("fixture should contain actor detection");
    assert!(matches!(
        unlife.effect,
        AbilityEffectDefinition::Detect {
            subject: AbilityDetectSubjectDefinition::Actor,
            persistent: false,
            ..
        }
    ));
    validate_and_normalize(&mut valid).expect("P54 abilities should compile");

    let mut duplicate = artifact.content.clone();
    let malediction = duplicate
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("fixture should contain level-scaled damage");
    malediction
        .level_scaling
        .push(malediction.level_scaling[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut duplicate),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut out_of_bounds = artifact.content.clone();
    out_of_bounds
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-horrify")
        .expect("fixture should contain a scaled sequence")
        .level_scaling[0]
        .effect_index = 2;
    assert!(matches!(
        validate_and_normalize(&mut out_of_bounds),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut persistent_actor_detection = artifact.content.clone();
    let AbilityEffectDefinition::Detect { persistent, .. } = &mut persistent_actor_detection
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("fixture should contain actor detection")
        .effect
    else {
        panic!("detect unlife should use actor detection");
    };
    *persistent = true;
    assert!(matches!(
        validate_and_normalize(&mut persistent_actor_detection),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut missing_control_category = artifact.content;
    let AbilityEffectDefinition::Control { category, .. } = &mut missing_control_category
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-enslave-undead")
        .expect("fixture should contain actor control")
        .effect
    else {
        panic!("enslave undead should use actor control");
    };
    *category = "missing-category".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut missing_control_category),
        Err(ContentError::InvalidAbility(_))
    ));
}

#[test]
fn zero_ability_bases_require_matching_level_scaling() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for ability_id in [
        "demo.ability.death-death-ray",
        "demo.ability.death-raise-dead",
        "demo.ability.death-esoteria",
        "demo.ability.death-mass-genocide",
    ] {
        let mut invalid = artifact.content.clone();
        invalid
            .abilities
            .iter_mut()
            .find(|ability| ability.id == ability_id)
            .unwrap_or_else(|| panic!("fixture should contain {ability_id}"))
            .level_scaling
            .clear();
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidAbility(id)) if id == ability_id
        ));
    }
}

#[test]
fn player_carry_capacity_is_positive_and_monsters_cannot_declare_one() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    let player = invalid
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain a player actor");
    player.carry_capacity_tenths_pound = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidActorCarryCapacity(_))
    ));

    let mut invalid = artifact.content.clone();
    let monster = invalid
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Monster)
        .expect("fixture should contain a monster actor");
    monster.carry_capacity_tenths_pound = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidActorCarryCapacity(_))
    ));
}

#[test]
fn melee_routines_require_monsters_and_valid_blow_profiles() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    let hound = invalid
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-hound")
        .expect("fixture should contain the echo hound");
    hound.role = ActorRole::Player;
    hound.experience_value = 0;
    hound.carry_capacity_tenths_pound = 100;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidMeleeRoutine(_))
    ));

    let mut invalid = artifact.content;
    let hound = invalid
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-hound")
        .expect("fixture should contain the echo hound");
    hound
        .melee_routine
        .as_mut()
        .expect("hound should have a melee routine")
        .blows[0]
        .damage_dice = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidMeleeRoutine(_))
    ));
}

#[test]
fn monster_casting_requires_weighted_supported_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid_frequency = artifact.content.clone();
    invalid_frequency
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast")
        .frequency_percent = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_frequency),
        Err(ContentError::InvalidMonsterCasting(_))
    ));

    let mut invalid_tactics = artifact.content.clone();
    let casting = invalid_tactics
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast");
    casting.preferred_distance = Some(1);
    casting.flee_hp_percent = 100;
    assert!(matches!(
        validate_and_normalize(&mut invalid_tactics),
        Err(ContentError::InvalidMonsterCasting(_))
    ));

    let mut duplicate_ability = artifact.content.clone();
    let casting = duplicate_ability
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast");
    casting.abilities.push(casting.abilities[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut duplicate_ability),
        Err(ContentError::InvalidMonsterCasting(_))
    ));

    let mut dangling_ability = artifact.content.clone();
    dangling_ability
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast")
        .abilities[0]
        .ability_id = "demo.ability.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut dangling_ability),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut unsupported_ability = artifact.content;
    unsupported_ability
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast")
        .abilities[0]
        .ability_id = "demo.ability.echo-step".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut unsupported_ability),
        Err(ContentError::InvalidMonsterCasting(_))
    ));
}

#[test]
fn semantic_versions_are_checked_strictly() {
    assert!(validate_semver("1.2.3-alpha.1+build.5").is_ok());
    for invalid in ["01.2.3", "1.2", "1.2.3-", "1.2.3+", "1.2.3-alpha..1"] {
        assert!(matches!(
            validate_semver(invalid),
            Err(ContentError::InvalidPackVersion(_))
        ));
    }
}

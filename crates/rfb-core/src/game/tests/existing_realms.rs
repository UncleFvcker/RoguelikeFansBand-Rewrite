// SPDX-License-Identifier: MPL-2.0
use super::support::{
    choose_human_talent_if_pending, clear_monsters, dispatch_next, give_inventory_item,
};
use super::*;

fn prepared(build: &str) -> Game {
    let mut game = Game::new_with_build(925, build).unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    game.progress.attributes.intelligence = game.progress.attribute_potentials.intelligence;
    game.progress.maximum_attributes.intelligence = game.progress.attributes.intelligence;
    game.progress.attributes.wisdom = game.progress.attribute_potentials.wisdom;
    game.progress.maximum_attributes.wisdom = game.progress.attributes.wisdom;
    game.refresh_player_ability_state();
    game.player.hp = game.effective_player_max_hp();
    game.glow.fill(true);
    game
}

fn cast_after_save(game: &mut Game, ability: &str, target: TargetSelection) {
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    // Repeat actual failure rolls too, until the same successful cast in both saves.
    for _ in 0..64 {
        let mut succeeded = false;
        for current in [&mut *game, &mut restored] {
            for resource in current.resources.values_mut() {
                resource.current = resource.maximum;
            }
            let mut events = Vec::new();
            current
                .resolve_player_ability(
                    ability,
                    target.clone(),
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            succeeded = events
                .iter()
                .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }));
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
        if succeeded {
            return;
        }
    }
    panic!("no successful cast: {ability}");
}

#[test]
fn existing_realms_new_builds_generate_pick_up_study_cast_and_resume() {
    let content = load_built_in_content().unwrap();
    let builds: Vec<_> = content
        .builds()
        .filter(|b| match b.class_id.as_str() {
            "demo.class.high-mage" => {
                !matches!(b.first_realm_id.as_deref(), Some("death" | "craft"))
            }
            "demo.class.mage" => {
                b.first_realm_id.as_deref() == Some("craft")
                    || b.second_realm_id.as_deref() == Some("craft")
            }
            "demo.class.paladin" => b.first_realm_id.as_deref() != Some("death"),
            _ => false,
        })
        .map(|b| b.id.clone())
        .collect();
    assert_eq!(builds.len(), 26);
    for build in builds {
        let mut game = prepared(&build);
        let book_ids: BTreeSet<_> = game
            .active_casting_book_ids()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: game.current_floor_id.clone(),
            depth: 20,
            source: LootSource::ItemUse {
                item_id: "test.realm-generation".into(),
            },
        };
        let draft = (0..8192)
            .find_map(|_| {
                game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|draft| {
                        game.content
                            .item(&draft.kind_id)
                            .unwrap()
                            .ability_book_id
                            .as_ref()
                            .is_some_and(|id| book_ids.contains(id))
                    })
            })
            .unwrap_or_else(|| panic!("no real current-realm book for {build}"));
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
            .unwrap();
        let id = item.id.clone();
        let kind = item.kind_id.clone();
        game.items.push(item);
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert!(game.item_knowledge[&kind].found_count > 0);
        let before = game.to_save();
        let mut restored = Game::from_save(before).unwrap();
        let a = game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary);
        let b = restored.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary);
        assert_eq!(
            a.as_ref().map(|x| &x.kind_id),
            b.as_ref().map(|x| &x.kind_id)
        );
        assert_eq!(game.rng, restored.rng);
        // The newborn first book guarantees a legal first spell; divine study still rolls.
        let first = if build.starts_with("demo.build.mage-") {
            "craft"
        } else {
            game.character_definitions()
                .unwrap()
                .0
                .first_realm_id
                .as_deref()
                .unwrap()
        };
        let book = game
            .content
            .item_definitions()
            .filter_map(|item| {
                item.ability_book_id
                    .as_deref()
                    .and_then(|id| game.content.ability_book(id))
            })
            .find(|b| b.realm_id.as_deref() == Some(first) && b.rank == Some(1))
            .unwrap();
        let requested = book.ability_ids[usize::from(first == "craft")].clone();
        let book_kind = game
            .content
            .item_definitions()
            .find(|i| i.ability_book_id.as_deref() == Some(book.id.as_str()))
            .unwrap()
            .id
            .clone();
        let book_item = game
            .items
            .iter()
            .find(|i| i.kind_id == book_kind)
            .unwrap()
            .id
            .clone();
        if build.starts_with("demo.build.paladin-") {
            dispatch_next(
                &mut game,
                GameCommand::StudyPrayer {
                    book_item_id: book_item,
                },
            );
        } else {
            game.study_player_ability(&book_item, &requested).unwrap();
        }
        let learned = game.learned_abilities.iter().next().unwrap().clone();
        let ability = game.content.ability(&learned).unwrap();
        let target = if ability
            .target
            .modes
            .contains(&AbilityTargetModeDefinition::SelfTarget)
        {
            TargetSelection::SelfTarget
        } else {
            let entity_target = ability
                .target
                .modes
                .contains(&AbilityTargetModeDefinition::Entity);
            let position = Position {
                x: game.player.position.x + 1,
                y: game.player.position.y,
            };
            game.replace_terrain_from_source(
                position,
                "demo.terrain.floor",
                terrain::TerrainChangeSource::Magic,
                &mut Vec::new(),
                &mut BTreeSet::new(),
            );
            let actor =
                game.generated_actor("test.spell-target".into(), "demo.actor.war-bear", position);
            game.entities.push(actor);
            if entity_target {
                TargetSelection::Entity {
                    entity_id: "test.spell-target".into(),
                }
            } else {
                TargetSelection::Direction {
                    direction: Direction::East,
                }
            }
        };
        cast_after_save(&mut game, &learned, target);
    }
}

#[test]
fn existing_realms_mage_changes_into_and_out_of_craft_with_pending_save() {
    let mut game = prepared("demo.build.mage-life-death");
    give_inventory_item(&mut game, "test.craft", "demo.item.handbook-for-pupils");
    let spent = game.spent_spell_learning;
    dispatch_next(
        &mut game,
        GameCommand::BeginRealmChange {
            book_item_id: "test.craft".into(),
        },
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for current in [&mut game, &mut restored] {
        dispatch_next(current, GameCommand::ResolveRealmChange { confirm: true });
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.current_second_realm_id(), Some("craft"));
    assert_eq!(game.spent_spell_learning, spent);
    game.study_player_ability("test.craft", "demo.ability.craft-regeneration")
        .unwrap();
    cast_after_save(
        &mut game,
        "demo.ability.craft-regeneration",
        TargetSelection::SelfTarget,
    );
    let death = game
        .items
        .iter()
        .find(|i| i.kind_id == "demo.item.black-prayers")
        .unwrap()
        .id
        .clone();
    dispatch_next(
        &mut game,
        GameCommand::BeginRealmChange {
            book_item_id: death,
        },
    );
    dispatch_next(&mut game, GameCommand::ResolveRealmChange { confirm: true });
    assert_eq!(game.current_second_realm_id(), Some("death"));
    assert!(
        !game
            .learned_abilities
            .contains("demo.ability.craft-regeneration")
    );
    Game::from_save(game.to_save()).unwrap();
}

#[test]
fn existing_realms_paladin_lances_reject_the_wrong_alignment_and_hit_real_targets() {
    for (realm, slug, damage_type) in [
        ("life", "holy", DamageType::HolyFire),
        ("crusade", "holy", DamageType::HolyFire),
        ("daemon", "hell", DamageType::HellFire),
        ("death", "hell", DamageType::HellFire),
    ] {
        let mut game = prepared(&format!("demo.build.paladin-{realm}"));
        for kind in [
            VirtueKindDto::Justice,
            VirtueKindDto::Valour,
            VirtueKindDto::Honour,
            VirtueKindDto::Faith,
        ] {
            assert!(game.virtues.iter().any(|v| v.kind == kind));
        }
        let id = format!("demo.ability.paladin-{slug}-lance");
        let wrong = format!(
            "demo.ability.paladin-{}-lance",
            if slug == "holy" { "hell" } else { "holy" }
        );
        assert!(game.class_ability_activation(&wrong).is_none());
        assert!(
            game.snapshot()
                .player
                .abilities
                .iter()
                .all(|a| a.id != wrong)
        );
        let before = game.state_hash();
        let mut events = Vec::new();
        game.resolve_player_ability(
            &wrong,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(events.iter().any(|e|matches!(e,DomainEvent::AbilityCastUnavailable { reason,.. } if reason=="realm-unavailable")));
        assert_eq!(game.state_hash(), before);
        let position = Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        };
        game.replace_terrain_from_source(
            position,
            "demo.terrain.floor",
            terrain::TerrainChangeSource::Magic,
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );
        let actor =
            game.generated_actor("test.lance-target".into(), "demo.actor.war-bear", position);
        let hp = actor.hp;
        game.entities.push(actor);
        cast_after_save(
            &mut game,
            &id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
        );
        assert!(
            game.entities
                .iter()
                .find(|a| a.id == "test.lance-target")
                .is_none_or(|a| a.hp < hp),
            "{realm} {damage_type:?}"
        );
    }
}

#[test]
#[ignore = "explicit preparation for ordinary standalone realm acceptance"]
fn export_existing_realms_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, _) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    let mut scenarios = Vec::new();
    for (build, spell) in [
        ("high-mage-sorcery", "sorcery-detect-monsters"),
        ("mage-craft-arcane", "craft-regeneration"),
        ("paladin-life", "paladin-holy-lance"),
        ("paladin-daemon", "paladin-hell-lance"),
    ] {
        let mut base = prepared(&format!("demo.build.{build}"));
        for r in base.resources.values_mut() {
            r.current = r.maximum;
        }
        let ability = format!("demo.ability.{spell}");
        let paladin = build.starts_with("paladin");
        let book = base
            .items
            .iter()
            .find(|i| {
                base.content
                    .item(&i.kind_id)
                    .unwrap()
                    .ability_book_id
                    .as_deref()
                    .and_then(|id| base.content.ability_book(id))
                    .is_some_and(|b| paladin || b.ability_ids.contains(&ability))
            })
            .unwrap()
            .id
            .clone();
        let target = if paladin {
            TargetSelection::Direction {
                direction: Direction::East,
            }
        } else {
            TargetSelection::SelfTarget
        };
        if paladin {
            let pos = Position {
                x: base.player.position.x + 1,
                y: base.player.position.y,
            };
            base.replace_terrain_from_source(
                pos,
                "demo.terrain.floor",
                terrain::TerrainChangeSource::Magic,
                &mut Vec::new(),
                &mut BTreeSet::new(),
            );
            let actor =
                base.generated_actor("test.realm-target".into(), "demo.actor.war-bear", pos);
            base.entities.push(actor);
        }
        let commands = vec![
            if paladin {
                GameCommand::StudyPrayer { book_item_id: book }
            } else {
                GameCommand::StudyAbility {
                    book_item_id: book,
                    ability_id: ability.clone(),
                }
            },
            GameCommand::CastAbility {
                ability_id: ability.clone(),
                target,
            },
            GameCommand::Wait,
        ];
        let (start, steps) = (0..128)
            .find_map(|seed| {
                let mut game = base.clone();
                game.rng = RfbRng::seeded(seed);
                let start = game.clone();
                let mut steps = Vec::new();
                for command in &commands {
                    let update = game
                        .dispatch(GameCommandEnvelope {
                            expected_revision: game.revision,
                            command_seq: game.last_command_seq + 1,
                            command: command.clone(),
                        })
                        .ok()?;
                    if matches!(command, GameCommand::CastAbility { .. })
                        && !update
                            .events
                            .iter()
                            .any(|e| e.message_key == "ability-cast-success")
                    {
                        return None;
                    }
                    Game::from_save(game.to_save()).unwrap();
                    steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                }
                Some((start, steps))
            })
            .expect("successful real study/cast sequence");
        std::fs::write(
            directory.join(format!("{build}.rfbsave")),
            rfb_save::encode(&header, &start.to_save()).unwrap(),
        )
        .unwrap();
        scenarios
            .push(serde_json::json!({"name":build,"initialHash":start.state_hash(),"steps":steps}));
    }
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&scenarios).unwrap(),
    )
    .unwrap();
}

// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters, give_inventory_item};
use super::*;

const BOOK: &str = "demo.item.sign-of-chaos";

// CH1 is an internal realm milestone. Exercise formal effects with real class
// profiles without publishing an incomplete realm in the creation catalog.
fn caster(class: &str, level: u16) -> Game {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    let source: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("legacy-chaos-source.json")).unwrap(),
    )
    .unwrap();
    let class_id = format!("demo.class.{class}");
    let profile = source["classProfiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["classId"] == class_id)
        .unwrap();
    let overrides = profile["abilityOverrides"].as_array().unwrap()[..8].to_vec();
    artifact
        .content
        .classes
        .iter_mut()
        .find(|c| c.id == class_id)
        .unwrap()
        .casting_profile
        .as_mut()
        .unwrap()
        .realm_profiles
        .push(rfb_content::CastingRealmProfileDefinition {
            realm_id: "chaos".into(),
            ability_book_ids: vec!["demo.ability-book.sign-of-chaos".into()],
            learning_capacity_bonus: 0,
            ability_overrides: serde_json::from_value(serde_json::Value::Array(overrides)).unwrap(),
        });
    let original = match class {
        "high-mage" => "demo.build.high-mage-death",
        "mage" => "demo.build.mage-life-arcane",
        "warrior-mage" => "demo.build.warrior-mage-arcane-life",
        _ => panic!("unprepared CH1 caster"),
    };
    let mut build = artifact
        .content
        .builds
        .iter()
        .find(|b| b.id == original)
        .unwrap()
        .clone();
    build.id = format!("test.build.chaos-{class}");
    if class == "high-mage" {
        build.first_realm_id = Some("chaos".into());
    } else {
        build.second_realm_id = Some("chaos".into());
    }
    build
        .starting_items
        .retain(|i| i.item_kind_id != "demo.item.black-prayers");
    let build_id = build.id.clone();
    artifact.content.builds.push(build);
    let content = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    let mut game =
        Game::from_content_with_build(715, content, DEFAULT_WORLD_ID, &build_id).unwrap();
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    game.progress.attributes.intelligence = game.progress.attribute_potentials.intelligence;
    game.progress.maximum_attributes.intelligence = game.progress.attributes.intelligence;
    game.refresh_player_ability_state();
    game.player.hp = game.effective_player_max_hp();
    clear_monsters(&mut game);
    game.player.position = Position { x: 10, y: 10 };
    for y in 6..=17 {
        for x in 6..=17 {
            let index = game.index(Position { x, y }).unwrap();
            game.terrain[index] = "demo.terrain.floor".into();
        }
    }
    game.glow.fill(true);
    game
}

fn learn(game: &mut Game, slug: &str) -> String {
    let book = game
        .items
        .iter()
        .find(|i| i.kind_id == BOOK)
        .map(|i| i.id.clone());
    let book = book.unwrap_or_else(|| {
        give_inventory_item(game, "test.chaos-book", BOOK);
        "test.chaos-book".into()
    });
    let id = format!("demo.ability.chaos-{slug}");
    game.study_player_ability(&book, &id).unwrap();
    id
}

fn cast_saved(game: &mut Game, id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    for resource in game.resources.values_mut() {
        resource.current = resource.maximum;
    }
    // Select an actual successful failure roll, then run the same command from
    // the saved state; no debug success override or fabricated effect event.
    for seed in 0..256 {
        let mut trial = game.clone();
        trial.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        trial
            .resolve_player_ability(
                id,
                target.clone(),
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        if !events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
        {
            continue;
        }
        game.rng = RfbRng::seeded(seed);
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        let mut resumed_events = Vec::new();
        restored
            .resolve_player_ability(
                id,
                target.clone(),
                &mut resumed_events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(trial.state_hash(), restored.state_hash());
        assert_eq!(trial.rng, restored.rng);
        *game = restored;
        return resumed_events;
    }
    panic!("no successful real cast for {id}");
}

fn target(game: &mut Game) {
    let mut actor = game.generated_actor(
        "test.chaos-target".into(),
        "demo.actor.war-bear",
        Position { x: 12, y: 10 },
    );
    actor.hp = 10_000;
    actor.max_hp = 10_000;
    game.entities.push(actor);
}

#[test]
fn ch1_ordinary_book_is_picked_up_learned_and_cast_after_save() {
    let mut game = caster("high-mage", 40);
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 20,
        source: LootSource::ItemUse {
            item_id: "test.ordinary-pool".into(),
        },
    };
    let draft = (0..16_384)
        .find_map(|_| {
            game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .filter(|d| d.kind_id == BOOK)
        })
        .expect("first Chaos book is in the actual ordinary pool");
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    assert!(game.item_knowledge[BOOK].found_count > 0);
    game = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    let spell = learn(&mut game, "magic-missile");
    target(&mut game);
    cast_saved(
        &mut game,
        &spell,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert!(game.entities[0].hp < 10_000);
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert!(restored.learned_abilities.contains(&spell));
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn ch1_damage_spells_hit_and_mana_burst_uses_class_and_level_boundaries() {
    for class in ["mage", "high-mage", "warrior-mage"] {
        let mut game = caster(class, 40);
        let id = learn(&mut game, "mana-burst");
        for level in [29, 30, 40] {
            game.progress.level = level;
            let ability = game.effective_casting_ability(
                game.casting_profile().unwrap(),
                game.content.ability(&id).unwrap(),
            );
            let AbilityEffectDefinition::AreaDamage {
                damage_bonus,
                radius,
                damage_type,
                ..
            } = ability.effect
            else {
                panic!()
            };
            assert_eq!(
                damage_bonus,
                level + level / if class == "warrior-mage" { 4 } else { 2 }
            );
            assert_eq!(radius, if level < 30 { 2 } else { 3 });
            assert_eq!(damage_type, ActorDamageType::Physical);
        }
        target(&mut game);
        cast_saved(
            &mut game,
            &id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
        );
        assert!(game.entities[0].hp < 10_000);
    }
    for slug in ["fire-bolt", "fist-of-force"] {
        let mut game = caster("high-mage", 40);
        let id = learn(&mut game, slug);
        target(&mut game);
        cast_saved(
            &mut game,
            &id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
        );
        assert!(game.entities[0].hp < 10_000);
        if slug == "fist-of-force" {
            let AbilityEffectDefinition::AreaDamage {
                radius,
                damage_type,
                ..
            } = game.content.ability(&id).unwrap().effect
            else {
                panic!()
            };
            assert_eq!(radius, 0);
            assert_eq!(damage_type, ActorDamageType::Disintegrate);
        }
    }
}

#[test]
fn ch1_touch_destruction_light_and_teleport_use_actual_world_state() {
    let mut game = caster("high-mage", 40);
    let door = Position { x: 11, y: 10 };
    let trap = Position { x: 9, y: 9 };
    let outside = Position { x: 12, y: 10 };
    for (position, kind) in [
        (Position { x: 10, y: 10 }, "demo.terrain.created-trap"),
        (door, "demo.terrain.door-closed"),
        (trap, "demo.terrain.created-trap"),
        (outside, "demo.terrain.door-closed"),
    ] {
        let index = game.index(position).unwrap();
        game.terrain[index] = kind.into();
    }
    let id = learn(&mut game, "trap-door-destruction");
    cast_saved(&mut game, &id, TargetSelection::SelfTarget);
    assert_eq!(
        game.terrain[game.index(game.player.position).unwrap()],
        "demo.terrain.floor"
    );
    assert_ne!(
        game.terrain[game.index(door).unwrap()],
        "demo.terrain.door-closed"
    );
    assert_ne!(
        game.terrain[game.index(trap).unwrap()],
        "demo.terrain.created-trap"
    );
    assert_eq!(
        game.terrain[game.index(outside).unwrap()],
        "demo.terrain.door-closed"
    );
    let id = learn(&mut game, "flash-of-light");
    game.glow.fill(false);
    // Studied spells may be cast in the dark; book study already happened in light.
    cast_saved(&mut game, &id, TargetSelection::SelfTarget);
    assert!(game.glow[game.index(game.player.position).unwrap()]);
    let id = learn(&mut game, "teleport-self");
    let before = game.player.position;
    cast_saved(&mut game, &id, TargetSelection::SelfTarget);
    assert_ne!(game.player.position, before);
}

#[test]
fn ch1_confusing_touch_survives_misses_and_is_consumed_by_a_real_melee_hit() {
    let mut base = caster("high-mage", 40);
    let id = learn(&mut base, "confusing-touch");
    cast_saved(&mut base, &id, TargetSelection::SelfTarget);
    assert!(base.confusing_strike_ready);
    cast_saved(&mut base, &id, TargetSelection::SelfTarget);
    target(&mut base);
    base.entities[0].position = Position { x: 11, y: 10 };
    let mut missed = false;
    let mut applied = false;
    let mut resisted = false;
    for seed in 0..2048 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        if game.entities[0].hp == 10_000 && game.confusing_strike_ready {
            missed = true;
        }
        let applications = events
            .iter()
            .filter(|e| matches!(e, DomainEvent::ConfusingStrikeApplied { .. }))
            .count();
        assert!(applications <= 1);
        if applications == 1 {
            assert!(!game.confusing_strike_ready);
            assert!(
                game.entities[0]
                    .statuses
                    .iter()
                    .any(|s| s.kind_id == STATUS_CONFUSION)
            );
            applied = true;
        }
        if events
            .iter()
            .any(|e| matches!(e, DomainEvent::ConfusingStrikeResisted { .. }))
        {
            assert!(!game.confusing_strike_ready);
            resisted = true;
        }
        if missed && applied && resisted {
            break;
        }
    }
    assert!(missed && applied && resisted);
    let mut immune_target = base.generated_actor(
        "test.chaos-target".into(),
        "demo.actor.adobe-golem",
        Position { x: 11, y: 10 },
    );
    immune_target.hp = 10_000;
    immune_target.max_hp = 10_000;
    base.entities[0] = immune_target;
    let immune = (0..2048)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .unwrap();
            events
                .iter()
                .any(|e| matches!(e, DomainEvent::ConfusingStrikeImmune { .. }))
                .then_some(game)
        })
        .expect("a real hit on a confusion-immune monster");
    assert!(!immune.confusing_strike_ready);
    let restored = Game::from_save_with_content(immune.to_save(), immune.content.clone()).unwrap();
    assert!(!restored.confusing_strike_ready);
    assert_eq!(immune.state_hash(), restored.state_hash());
}

#[test]
fn ch1_invalid_target_does_not_arm_touch_or_spend_resources_or_rng() {
    let mut game = caster("high-mage", 40);
    for slug in ["confusing-touch", "trap-door-destruction"] {
        let id = learn(&mut game, slug);
        let before = game.state_hash();
        let mut events = Vec::new();
        game.resolve_player_ability(
            &id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
        );
        assert_eq!(before, game.state_hash());
        assert!(!game.confusing_strike_ready);
    }
}

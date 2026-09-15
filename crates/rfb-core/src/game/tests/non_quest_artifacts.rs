// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::inventory::RemoveEquippedCursesRequest;
use rfb_content::AmmunitionTypeDefinition;
mod n2;
mod n3;

const REMAINING_N1: &[u32] = &[
    35, 58, 71, 72, 77, 81, 87, 90, 91, 134, 137, 140, 142, 154, 155, 156, 158, 160, 161, 165, 167,
    176, 189, 191, 193, 200, 210, 216, 221, 222, 223, 224, 228, 229, 232, 250, 253, 254, 274, 281,
    292, 295, 296, 323, 325, 330, 331, 337, 338, 339, 349, 351, 352,
];

#[test]
#[ignore = "explicit N1 preparation for ordinary standalone UI acceptance"]
fn export_n1_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut base = Game::from_save(payload).unwrap();
    choose_human_talent_if_pending(&mut base);
    base.apply_player_experience(base.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut base);
    descend_one_floor(&mut base);
    base.apply_player_melee_status(STATUS_INVULNERABILITY, 200_000, "test.n1.desktop");
    let mut scenarios = Vec::new();
    for slug in [
        "necklace-of-the-dwarves",
        "aglarang",
        "hellfire",
        "dunce-cap",
    ] {
        let mut game = base.clone();
        clear_monsters(&mut game);
        game.items.clear();
        game.player.position = Position { x: 10, y: 10 };
        for x in 10..=15 {
            replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
        }
        let id = generate(&mut game, &format!("demo.item.{slug}"));
        game.items.iter_mut().find(|i| i.id == id).unwrap().location =
            ItemLocation::Ground(game.player.position);
        if slug == "hellfire" {
            give_inventory_item(&mut game, "test.n1.bolts", "demo.item.bolt");
            give_inventory_item(&mut game, "test.n1.desktop-light", "demo.item.wooden-torch");
            game.equip_inventory_item("test.n1.desktop-light", None)
                .unwrap();
        }
        if matches!(slug, "aglarang" | "hellfire") {
            game.push_generated_actor(
                "test.n1.desktop-target".into(),
                "demo.actor.blubbering-idiot",
                Position { x: 11, y: 10 },
            );
        }
        game.refresh_player_resource_maxima();
        let mut commands = vec![
            GameCommand::PickUp,
            GameCommand::Equip {
                item_id: id.clone(),
                slot_id: None,
            },
        ];
        if slug == "aglarang" {
            commands.push(GameCommand::Move {
                direction: Direction::East,
            });
        }
        if slug == "hellfire" {
            commands.push(GameCommand::Fire {
                direction: Direction::East,
            });
        }
        commands.push(GameCommand::Wait);
        if matches!(slug, "aglarang" | "hellfire") {
            let seed = (0..1000)
                .find(|seed| {
                    let mut trial = game.clone();
                    trial.rng = RfbRng::seeded(*seed);
                    for cmd in &commands {
                        if matches!(cmd, GameCommand::Fire { .. } | GameCommand::Move { .. })
                            && !trial.entities.iter().any(|actor| {
                                actor.id == "test.n1.desktop-target"
                                    && actor.position == Position { x: 11, y: 10 }
                            })
                        {
                            return false;
                        }
                        dispatch_next(&mut trial, cmd.clone());
                    }
                    trial
                        .entities
                        .iter()
                        .all(|actor| actor.id != "test.n1.desktop-target")
                })
                .unwrap();
            game.rng = RfbRng::seeded(seed);
        }
        game.reveal_current_visibility();
        let start = game.clone();
        let mut steps = Vec::new();
        for command in commands {
            dispatch_next(&mut game, command.clone());
            assert_eq!(
                game.state_hash(),
                Game::from_save(game.to_save()).unwrap().state_hash()
            );
            steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
        }
        if matches!(slug, "aglarang" | "hellfire") {
            assert!(
                game.entities
                    .iter()
                    .all(|actor| actor.id != "test.n1.desktop-target")
            );
        }
        std::fs::write(
            directory.join(format!("{slug}.rfbsave")),
            rfb_save::encode(&header, &start.to_save()).unwrap(),
        )
        .unwrap();
        scenarios
            .push(serde_json::json!({"name":slug,"initialHash":start.state_hash(),"steps":steps}));
    }
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&scenarios).unwrap(),
    )
    .unwrap();
}

fn context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.n1-depth-100".into(),
        depth: 100,
        source: LootSource::ItemUse {
            item_id: "test.n1-generation".into(),
        },
    }
}

fn prepared_mage() -> Game {
    let mut game = Game::new_with_build(493, "demo.build.mage-life-arcane").unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for x in 10..=15 {
        replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
    }
    // Controlled real XP and base/depth, not natural leveling or acquisition.
    game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    game.refresh_player_resource_maxima();
    game
}

fn generate(game: &mut Game, kind: &str) -> String {
    let instant = game
        .content
        .item(kind)
        .unwrap()
        .artifact_generation
        .as_ref()
        .unwrap()
        .instant;
    let base = game
        .content
        .item(kind)
        .unwrap()
        .artifact_generation
        .as_ref()
        .unwrap()
        .base_item_kind_id
        .clone();
    let selected = (0..30_000)
        .find_map(|_| {
            game.roll_fixed_artifact_kind_id(&context(), Some(&base), instant)
                .filter(|id| id == kind)
        })
        .unwrap_or_else(|| panic!("ordinary source base/rarity must admit {kind}"));
    let draft = game.fixed_item_draft(&context(), selected);
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    id
}

fn shoot(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_projectile(
        TargetSelection::Direction {
            direction: Direction::East,
        },
        player_combat::ProjectileMode::Normal,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn strike(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    events
}

#[test]
fn n1_remaining_ordinary_artifacts_generate_act_and_resume_after_save() {
    let prepared = prepared_mage();
    let definitions: Vec<_> = prepared
        .content
        .item_definitions()
        .filter(|item| {
            item.artifact_generation
                .as_ref()
                .is_some_and(|g| REMAINING_N1.contains(&g.source_index))
        })
        .cloned()
        .collect();
    assert_eq!(definitions.len(), REMAINING_N1.len());
    for definition in definitions {
        let kind = &definition.id;
        let mut game = prepared.clone();
        let id = generate(&mut game, kind);
        assert!(
            !game
                .item_property_knowledge
                .get(&id)
                .is_some_and(|k| k.appraised)
        );
        game.reveal_current_visibility();
        let loaded = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            loaded.state_hash(),
            game.state_hash(),
            "{kind}: unknown save"
        );
        assert_eq!(loaded.rng, game.rng);
        game = loaded;
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        if definition.equipment_slot.is_some() {
            game.equip_inventory_item(&id, None).unwrap();
        }
        if let Some(ammo) = &definition.ammunition_profile {
            let launcher = if ammo.ammunition_type == AmmunitionTypeDefinition::Bolt {
                "demo.item.light-crossbow"
            } else {
                "demo.item.sling"
            };
            give_inventory_item(&mut game, "test.n1-launcher", launcher);
            game.equip_inventory_item("test.n1-launcher", None).unwrap();
        } else if let Some(profile) = &definition.projectile_profile {
            let ammo = match profile.ammunition_type {
                AmmunitionTypeDefinition::Bolt => "demo.item.bolt",
                AmmunitionTypeDefinition::Shot => "demo.item.iron-shot",
                _ => "demo.item.arrow",
            };
            give_inventory_item(&mut game, "test.n1-ammunition", ammo);
        }
        game.refresh_player_resource_maxima();
        let attack: Option<fn(&mut Game) -> Vec<DomainEvent>> =
            if definition.melee_profile.is_some() {
                Some(strike)
            } else if definition.projectile_profile.is_some()
                || definition.ammunition_profile.is_some()
            {
                Some(shoot)
            } else {
                None
            };
        if definition.equipment_slot.as_deref() == Some("tool") {
            // Tool slot supplies digging only; wielding activates its other powers.
            let wall = Position { x: 10, y: 9 };
            replace_terrain(&mut game, wall, "demo.terrain.magma-vein");
            let seed = (0..1000)
                .find(|seed| {
                    let mut trial = game.clone();
                    trial.rng = RfbRng::seeded(*seed);
                    trial
                        .dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new())
                        .unwrap();
                    trial.terrain_at(wall) != game.terrain_at(wall)
                })
                .expect("artifact digger must remove real terrain");
            game.rng = RfbRng::seeded(seed);
            game.reveal_current_visibility();
            let mut restored = Game::from_save(game.to_save()).unwrap();
            for digger in [&mut game, &mut restored] {
                digger
                    .dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new())
                    .unwrap();
            }
            assert_eq!(game.state_hash(), restored.state_hash());
            game.unequip_slot("tool").unwrap();
            game.equip_inventory_item(&id, Some("right-hand")).unwrap();
            game.refresh_player_resource_maxima();
        }
        if let Some(attack) = attack {
            game.push_generated_actor(
                "test.n1-target".into(),
                "demo.actor.blubbering-idiot",
                Position { x: 11, y: 10 },
            );
            let seed = (0..1000)
                .find(|seed| {
                    let mut trial = game.clone();
                    trial.rng = RfbRng::seeded(*seed);
                    attack(&mut trial).iter().any(|event| {
                        matches!(
                            event.clone().into_dto().outcome,
                            Some(GameEventOutcomeDto::Damage { .. })
                        )
                    })
                })
                .unwrap_or_else(|| panic!("{kind} must hit through its real attack consumer"));
            game.rng = RfbRng::seeded(seed);
            game.reveal_current_visibility();
            let mut restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(
                attack(&mut restored),
                attack(&mut game),
                "{kind}: saved combat"
            );
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(restored.rng, game.rng);
            if definition.ammunition_profile.is_some() {
                let ammo = game
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .expect("fixed ammunition must not break");
                assert!(
                    matches!(ammo.location, ItemLocation::Ground(_)),
                    "ordinary artifacts do not return like Brahmastra"
                );
            }
        }
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            restored.state_hash(),
            game.state_hash(),
            "{kind}: equipped save"
        );
        assert_eq!(restored.equipment_modifiers(), game.equipment_modifiers());
        assert_eq!(
            restored.player_equipment_bonuses(),
            game.player_equipment_bonuses()
        );
        assert_eq!(
            restored
                .generate_loot_instances(&context(), ItemLocation::Inventory)
                .unwrap(),
            game.generate_loot_instances(&context(), ItemLocation::Inventory)
                .unwrap()
        );
        assert_eq!(restored.rng, game.rng);
        assert!(game.generated_artifact_ids.contains(kind));
        for _ in 0..8 {
            assert_ne!(
                game.roll_fixed_artifact_kind_id(
                    &context(),
                    Some(
                        &definition
                            .artifact_generation
                            .as_ref()
                            .unwrap()
                            .base_item_kind_id
                    ),
                    false
                )
                .as_deref(),
                Some(kind.as_str())
            );
        }
        if definition.equipment_slot.is_some() {
            let index = game.items.iter().position(|item| item.id == id).unwrap();
            let slot = match &game.items[index].location {
                ItemLocation::Equipped { slot_id } => slot_id.clone(),
                _ => unreachable!(),
            };
            if game.items[index].curse.is_some() {
                assert!(game.unequip_slot(&slot).is_none());
            }
            game.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
            if game.items[index].curse == Some(ItemCurseSeverityDto::Permanent) {
                assert!(
                    game.unequip_slot(&slot).is_none(),
                    "{kind}: permanent curse survives greater removal"
                );
            } else {
                game.unequip_slot(&slot).unwrap();
                assert_eq!(
                    game.equipment_modifiers(),
                    prepared.equipment_modifiers(),
                    "{kind}: unequip"
                );
            }
        }
    }
}

#[test]
fn n1_random_curses_roll_once_with_source_power_and_persist_until_removed() {
    let prepared = prepared_mage();
    for (kind, power, tval) in [("demo.item.thanos", 1, 31), ("demo.item.liweris", 0, 22)] {
        for seed in 0..28 {
            let mut game = prepared.clone();
            game.rng = RfbRng::seeded(seed);
            let mut expected_rng = game.rng.clone();
            let expected = super::super::ego::curses::get_curse(&mut expected_rng, power, tval);
            let draft = game.fixed_item_draft(&context(), kind.into());
            assert_eq!(draft.intrinsic_curse_effects, BTreeSet::from([expected]));
            assert_eq!(
                game.rng, expected_rng,
                "{kind}: source get_curse draw order"
            );
            let item = game
                .commit_generated_item_draft(draft, ItemLocation::Inventory)
                .unwrap();
            let id = item.id.clone();
            game.items.push(item);
            game.equip_inventory_item(&id, None).unwrap();
            assert!(game.player_has_equipped_curse_effect(expected));
            game.reveal_current_visibility();
            let mut restored = Game::from_save(game.to_save()).unwrap();
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(restored.rng, expected_rng);
            restored.remove_equipped_curses(RemoveEquippedCursesRequest::new(false));
            assert!(restored.player_has_equipped_curse_effect(expected));
            restored.remove_equipped_curses(RemoveEquippedCursesRequest::new(true));
            let item = restored.items.iter().find(|item| item.id == id).unwrap();
            assert!(item.curse.is_none());
            assert_eq!(
                restored.player_has_equipped_curse_effect(expected),
                restored.item_has_intrinsic_curse_effect(item, expected)
            );
        }
    }
}

#[test]
fn n1_permanently_cursed_mage_equipment_changes_actual_learned_spell_damage() {
    let prepared = prepared_mage();
    for (kind, power) in [("demo.item.beruthiel", 20), ("demo.item.kaschei", 4)] {
        let mut game = prepared.clone();
        let id = generate(&mut game, kind);
        game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
        game.equip_inventory_item(&id, None).unwrap();
        assert_eq!(game.effective_player_spell_power_bonus(), power);
        give_inventory_item(&mut game, "test.n1-light", "demo.item.wooden-torch");
        game.equip_inventory_item("test.n1-light", None).unwrap();
        give_inventory_item(
            &mut game,
            "test.n1-book",
            "demo.item.cantrips-for-beginners",
        );
        game.study_player_ability("test.n1-book", "demo.ability.arcane-zap")
            .unwrap();
        game.refresh_player_resource_maxima();
        for pool in game.resources.values_mut() {
            pool.current = pool.maximum;
        }
        game.push_generated_actor(
            "test.n1-spell".into(),
            "demo.actor.small-kobold",
            Position { x: 12, y: 10 },
        );
        game.reveal_current_visibility();
        let mut powered = Game::from_save(game.to_save()).unwrap();
        assert_eq!(powered.state_hash(), game.state_hash());
        let mut control = powered.clone();
        // Same penalties, equipment and learned spell; remove only spell power.
        control
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .unwrap()
            .intrinsic_properties
            .modifiers
            .spell_power_bonus = -power;
        let mut damage = Vec::new();
        for caster in [&mut control, &mut powered] {
            caster.debug_ability_casts_succeed = true;
            caster.rng = RfbRng::seeded(476);
            let mut events = Vec::new();
            caster
                .resolve_player_ability(
                    "demo.ability.arcane-zap",
                    TargetSelection::Direction {
                        direction: Direction::East,
                    },
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            damage.push(
                events
                    .iter()
                    .find_map(|e| match e {
                        DomainEvent::AbilityHit { damage, .. } => Some(damage.raw),
                        _ => None,
                    })
                    .unwrap(),
            );
        }
        assert!(damage[1] > damage[0]);
        assert_eq!(damage[1], damage[0] + damage[0] * power / 13);
        assert_eq!(powered.rng, control.rng);
    }
}

#[test]
fn n1_launcher_and_ammunition_offense_reaches_shots_without_leaking_into_melee() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let artifact = rfb_content::compile_pack_dir(&root).unwrap();
    let prepared = prepared_mage();
    for (launcher, ammunition, target) in [
        ("hellfire", "bolt", "blubbering-idiot"),
        ("heracles", "arrow", "blubbering-idiot"),
        ("wilhelm-tell-crossbow", "bolt", "blubbering-idiot"),
        (
            "wilhelm-tell-crossbow",
            "wilhelm-tell-bolt",
            "blubbering-idiot",
        ),
        ("sling", "david", "hill-giant"),
    ] {
        let mut game = prepared.clone();
        for (slug, id, equip) in [
            (launcher, "test.n1-bow", true),
            (ammunition, "test.n1-ammo", false),
        ] {
            let kind = format!("demo.item.{slug}");
            if game
                .content
                .item(&kind)
                .unwrap()
                .artifact_generation
                .is_some()
            {
                let draft = game.fixed_item_draft(&context(), kind);
                let mut item = game
                    .commit_generated_item_draft(draft, ItemLocation::Inventory)
                    .unwrap();
                item.id = id.into();
                game.items.push(item);
            } else {
                give_inventory_item(&mut game, id, &kind);
            }
            if equip {
                game.equip_inventory_item(id, None).unwrap();
            }
        }
        give_inventory_item(&mut game, "test.n1-sword", "demo.item.long-sword");
        game.equip_inventory_item("test.n1-sword", None).unwrap();
        game.push_generated_actor(
            "test.n1-ranged-target".into(),
            &format!("demo.actor.{target}"),
            Position { x: 11, y: 10 },
        );
        let mut plain_content = artifact.content.clone();
        for slug in [launcher, ammunition] {
            let item = plain_content
                .items
                .iter_mut()
                .find(|item| item.id == format!("demo.item.{slug}"))
                .unwrap();
            item.brands.clear();
            item.slays.clear();
        }
        let mut plain = game.clone();
        plain.content = std::sync::Arc::new(rfb_content::ContentCatalog::from_artifact(
            rfb_content::encode_content(plain_content).unwrap(),
        ));
        let damage = |events: Vec<DomainEvent>| {
            events
                .into_iter()
                .find_map(|event| match event.into_dto().outcome {
                    Some(GameEventOutcomeDto::Damage { resolution }) => Some(resolution.raw_damage),
                    _ => None,
                })
        };
        let seed = (0..1000)
            .find(|seed| {
                let mut a = game.clone();
                let mut b = plain.clone();
                a.rng = RfbRng::seeded(*seed);
                b.rng = a.rng.clone();
                match (damage(shoot(&mut a)), damage(shoot(&mut b))) {
                    (Some(a), Some(b)) => a > b,
                    _ => false,
                }
            })
            .unwrap_or_else(|| panic!("{launcher}/{ammunition}: actual shot must receive offense"));
        let mut a = game.clone();
        let mut b = plain.clone();
        a.rng = RfbRng::seeded(seed);
        b.rng = a.rng.clone();
        assert!(damage(shoot(&mut a)).unwrap() > damage(shoot(&mut b)).unwrap());
        // The same bow and ammunition cannot brand the equipped sword.
        game.rng = RfbRng::seeded(seed);
        plain.rng = game.rng.clone();
        assert_eq!(damage(strike(&mut game)), damage(strike(&mut plain)));
        if ammunition == "wilhelm-tell-bolt" {
            assert_eq!(
                game.player_projectile_profile().unwrap().ammunition_slays[&SlayTarget::Human],
                SlayLevel::Kill
            );
        }
    }
}

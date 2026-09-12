// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_protocol::MeleeDamageDiceDto;

const KIND: &str = "demo.item.death-scythe";

#[test]
#[ignore = "explicit equipment preparation for ordinary Tauri standalone acceptance"]
fn export_ordinary_equipment_desktop_saves() {
    let input = std::path::PathBuf::from(std::env::var("ORDINARY_EQUIPMENT_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let fresh = Game::from_save(payload).unwrap();
    let mut scenarios = Vec::new();
    for (name, slugs, action) in [
        (
            "weapon",
            vec!["guisarme"],
            Some(GameCommand::Move {
                direction: Direction::East,
            }),
        ),
        (
            "digger",
            vec!["dwarven-pick"],
            Some(GameCommand::DigTerrain {
                direction: Direction::North,
            }),
        ),
        (
            "armor",
            vec!["mithril-plate-mail", "ethereal-cloak", "mithril-gauntlets"],
            None,
        ),
        ("swimsuit", vec!["sexy-swimsuit"], Some(GameCommand::Wait)),
        (
            "scythe",
            vec!["death-scythe"],
            Some(GameCommand::Move {
                direction: Direction::East,
            }),
        ),
    ] {
        let mut base = fresh.clone();
        choose_human_talent_if_pending(&mut base);
        clear_monsters(&mut base);
        base.items.clear();
        base.player.position = Position { x: 10, y: 10 };
        for y in 8..=12 {
            for x in 8..=14 {
                replace_terrain(&mut base, Position { x, y }, "demo.terrain.floor");
            }
        }
        base.player.hp = base.effective_player_max_hp();
        if name == "digger" {
            replace_terrain(
                &mut base,
                Position { x: 10, y: 9 },
                "demo.terrain.magma-vein",
            );
        }
        if matches!(name, "weapon" | "scythe" | "swimsuit") {
            base.push_generated_actor(
                "test.desktop.target".into(),
                "demo.actor.blubbering-idiot",
                Position { x: 11, y: 10 },
            );
        }
        if matches!(name, "swimsuit" | "weapon") {
            base.apply_actor_melee_status(0, STATUS_SLEEP, 500, "test.desktop.sleep");
        }
        if name == "scythe" {
            let mut buffer =
                monster_combat::melee_status("rfb.status.hero", 200000, "test.desktop.hp-buffer")
                    .status;
            buffer.granted_modifiers.max_hp = 2000;
            base.player.statuses.push(buffer);
            base.player.hp = base.effective_player_max_hp();
        }
        let mut commands = Vec::new();
        for slug in &slugs {
            let id = format!("test.desktop.{slug}");
            give_inventory_item(&mut base, &id, &format!("demo.item.{slug}"));
            if name == "scythe" {
                let item = base.items.last_mut().unwrap();
                item.enchantments.to_hit = -255;
            }
            base.identify_item_instance(&id, ItemIdentificationRequest::new(true));
            commands.push(GameCommand::Equip {
                item_id: id,
                slot_id: (name == "digger").then(|| "tool".into()),
            });
        }
        if let Some(action) = action {
            commands.push(action);
        }
        base.reveal_current_visibility();
        let (game, steps) = (0..256)
            .find_map(|seed| {
                let mut game = base.clone();
                game.rng = RfbRng::seeded(seed);
                let initial = game.clone();
                let mut steps = Vec::new();
                let mut backlash = false;
                let mut hit = false;
                for command in &commands {
                    let update = dispatch_next(&mut game, command.clone());
                    hit |= update.events.iter().any(|event| event.kind == "combat.hit");
                    backlash |= update
                        .events
                        .iter()
                        .any(|event| event.kind == "item.use-life-loss");
                    steps.push(serde_json::json!({"command":command,"hash":game.state_hash()}));
                }
                ((name != "scythe" || (backlash && !game.player_is_dead()))
                    && (name != "weapon" || hit))
                    .then_some((initial, steps))
            })
            .unwrap();
        assert_eq!(
            game.state_hash(),
            Game::from_save(game.to_save()).unwrap().state_hash()
        );
        std::fs::write(
            directory.join(format!("{name}.rfbsave")),
            rfb_save::encode(&header, &game.to_save()).unwrap(),
        )
        .unwrap();
        scenarios.push(serde_json::json!({"name":name,"slugs":slugs,"initialHash":game.state_hash(),"steps":steps}));
    }
    std::fs::write(
        directory.join("scenarios.json"),
        serde_json::to_vec_pretty(&scenarios).unwrap(),
    )
    .unwrap();
}

fn scythe_game(build: &str, race: &str) -> Game {
    let mut game = Game::new_with_build_race_and_name(501, build, race, "Scythe").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for x in 10..=18 {
        replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
    }
    give_inventory_item(&mut game, "test.scythe", KIND);
    game.items[0].intrinsic_properties.modifiers.max_hp = 100_000;
    game.identify_item_instance("test.scythe", ItemIdentificationRequest::new(true));
    game.equip_inventory_item("test.scythe", None).unwrap();
    game.player.hp = game.effective_player_max_hp();
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    game
}

fn backlash_amount(events: &[DomainEvent]) -> i32 {
    events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::ItemLifeLost {
                source_kind_id,
                amount,
                ..
            } if source_kind_id == KIND => Some(*amount),
            _ => None,
        })
        .sum()
}

fn attack(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    events
}

#[test]
fn source_allocation_preserves_special_generation_and_distinct_crafting_rules() {
    let mut game = scythe_game("demo.build.warrior", "demo.race.rfb-human");
    game.items.clear();
    game.refresh_player_resource_maxima();
    game.player.hp = game.effective_player_max_hp();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-85".into(),
        depth: 85,
        source: LootSource::MonsterDeath {
            actor_id: "test.scythe-drop".into(),
        },
    };
    let item = (0..100_000)
        .find_map(|_| {
            game.generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == KIND)
        })
        .unwrap();
    assert_eq!(item.curse, Some(ItemCurseSeverityDto::Heavy));
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert_eq!(item.enchantments, Default::default());
    assert!(!item.is_artifact(&game.content));
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.reveal_current_visibility();
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
    assert!(!game.item_is_valid_crafting_target(None, &id));
    assert!(
        !game.item_resists_enchantment(&game.items[0]),
        "ordinary enchantment remains legal"
    );
    assert!(
        game.artifact_creation_plan(
            "test.scroll",
            &TargetSelection::ArtifactCreationItem {
                item_id: id.clone(),
                quantity: 1,
                name: None
            }
        )
        .is_some()
    );
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&id, None).unwrap();
    assert!(game.unequip_slot("right-hand").is_none());
    assert!(game.item_has_heavy_curse(&game.items[0]));
}

#[test]
fn backlash_race_resistance_and_mimic_multipliers_share_all_five_save_draws() {
    for seed in 0..64 {
        let mut human = scythe_game("demo.build.warrior", "demo.race.rfb-human");
        human.items[0].intrinsic_melee_damage_dice =
            Some(MeleeDamageDiceDto { dice: 10, sides: 1 });
        human.items[0].enchantments.to_hit = -255; // No critical; its check still consumes RNG.
        let item = human.items[0].clone();
        let mut baseline = None;
        for (race, immune, multiplier) in [
            ("demo.race.rfb-human", false, 25),
            ("rfb-legacy.race.tonberry", false, 30),
            ("rfb-legacy.race.high-elf", true, 10),
            ("rfb-legacy.race.high-elf", false, 25),
            ("demo.race.demon", true, 30),
        ] {
            let mut game = human.clone();
            let mut form =
                monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 500, "test.scythe-form")
                    .status;
            form.granted_race_id = Some(race.into());
            game.player.statuses.push(form);
            if immune {
                for element in [
                    DamageType::Acid,
                    DamageType::Electricity,
                    DamageType::Fire,
                    DamageType::Cold,
                    DamageType::Poison,
                ] {
                    game.player
                        .resistances
                        .set(element, ResistanceLevel::Immune);
                }
            }
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            game.resolve_death_scythe_backlash(&item, None, &mut events);
            let scaled = (backlash_amount(&events) - 30) / multiplier;
            assert!(scaled >= 1);
            if let Some((expected, rng)) = &baseline {
                assert_eq!(scaled, *expected);
                assert_eq!(&game.rng, rng, "all five resistance checks consume RNG");
            } else {
                baseline = Some((scaled, game.rng));
            }
        }
    }
}

#[test]
fn missed_and_duelist_hit_backlash_resume_and_stop_at_real_death() {
    for (build, force_miss) in [("demo.build.warrior", true), ("demo.build.duelist", false)] {
        let mut base = scythe_game(build, "demo.race.rfb-human");
        base.items[0].enchantments.to_hit = if force_miss { -255 } else { 255 };
        base.push_generated_actor(
            "test.scythe-target".into(),
            "demo.actor.steam-powered-mechanical-dragon",
            Position { x: 11, y: 10 },
        );
        // Keep the Duelist's voluntary challenge from replacing the first attack.
        if !force_miss {
            base.duelist_target_id = Some(base.entities[0].id.clone());
        }
        base.reveal_current_visibility();
        let mut observed = BTreeSet::new();
        for seed in 0..128 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let mut restored = Game::from_save(game.to_save()).unwrap();
            let events = attack(&mut game);
            assert_eq!(attack(&mut restored), events);
            assert_eq!(restored.state_hash(), game.state_hash());
            assert_eq!(restored.rng, game.rng);
            observed.insert(backlash_amount(&events) > 0);
            if backlash_amount(&events) > 0 {
                let mut lethal = base.clone();
                lethal.player.hp = 1;
                lethal.rng = RfbRng::seeded(seed);
                let events = attack(&mut lethal);
                assert!(lethal.player_is_dead());
                assert_eq!(events.iter().filter(|event| matches!(event, DomainEvent::ItemLifeLost { source_kind_id, .. } if source_kind_id == KIND)).count(), 1);
            }
            if observed.len() == 2 {
                break;
            }
        }
        assert_eq!(observed, BTreeSet::from([false, true]));
    }
}

#[test]
fn return_backlash_keeps_catch_drop_short_circuit_and_force_damage() {
    let mut base = scythe_game("demo.build.warrior", "demo.race.rfb-human");
    base.items[0].curse = None;
    let mut shield =
        monster_combat::melee_status(STATUS_INVULNERABILITY, 500, "test.scythe-shield").status;
    shield.incoming_damage_percent = 0;
    base.player.statuses.push(shield);
    for returning in [
        None,
        Some((false, false)),
        Some((true, false)),
        Some((true, true)),
    ] {
        let mut outcomes = BTreeSet::new();
        for seed in 0..64 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let item = game.items[0].clone();
            if returning.is_none() {
                game.items.clear();
            }
            let mut restored = game.clone();
            let mut events = Vec::new();
            let mut replay = Vec::new();
            game.finish_item_throw(
                item.clone(),
                Position { x: 15, y: 10 },
                returning,
                &mut events,
                &mut BTreeSet::new(),
            );
            restored.finish_item_throw(
                item,
                Position { x: 15, y: 10 },
                returning,
                &mut replay,
                &mut BTreeSet::new(),
            );
            assert_eq!(events, replay);
            assert_eq!(game.state_hash(), restored.state_hash());
            assert_eq!(game.rng, restored.rng);
            let damaged = backlash_amount(&events) > 0;
            outcomes.insert(damaged);
            if returning == Some((true, false)) {
                assert!(damaged);
            }
            if matches!(returning, None | Some((false, false))) {
                assert!(!damaged);
            }
            assert_eq!(game.items.len(), 1);
            assert_eq!(
                game.items[0].location,
                match returning {
                    Some((true, true)) => base.items[0].location.clone(),
                    Some((true, false)) => ItemLocation::Ground(base.player.position),
                    _ => ItemLocation::Ground(Position { x: 15, y: 10 }),
                }
            );
        }
        if returning == Some((true, true)) {
            assert_eq!(outcomes, BTreeSet::from([false, true]));
        }
    }
}

#[test]
fn melee_pierces_invulnerability_but_preserves_other_damage_reduction() {
    let mut base = scythe_game("demo.build.warrior", "demo.race.rfb-human");
    base.items[0].enchantments.to_hit = 1000;
    base.push_generated_actor(
        "test.target".into(),
        "demo.actor.small-kobold",
        Position { x: 11, y: 10 },
    );
    base.entities[0].hp = 1_000_000;
    base.entities[0].max_hp = 1_000_000;
    let mut hits = 0;
    for seed in 0..16 {
        let mut plain = base.clone();
        plain.rng = RfbRng::seeded(seed);
        let mut shielded = plain.clone();
        let mut shield =
            monster_combat::melee_status(STATUS_INVULNERABILITY, 500, "test.shield").status;
        shield.incoming_damage_percent = 0;
        shielded.entities[0].statuses.push(shield);
        let plain_events = attack(&mut plain);
        assert_eq!(attack(&mut shielded), plain_events);
        hits += plain_events
            .iter()
            .filter(
                |e| matches!(e, DomainEvent::PlayerMeleeHit { damage, .. } if damage.applied > 0),
            )
            .count();
        let mut reduced = base.clone();
        reduced.rng = RfbRng::seeded(seed);
        reduced.entities[0]
            .resistances
            .set(DamageType::Physical, ResistanceLevel::Resistant);
        let reduced_events = attack(&mut reduced);
        let damages = |events: &[DomainEvent]| {
            events
                .iter()
                .filter_map(|e| match e {
                    DomainEvent::PlayerMeleeHit { damage, .. } => Some(damage.applied),
                    _ => None,
                })
                .sum::<i32>()
        };
        assert!(damages(&reduced_events) <= damages(&plain_events));
        if damages(&plain_events) > 0 {
            assert!(damages(&reduced_events) < damages(&plain_events));
        }
    }
    assert!(hits > 0);
}

#[test]
fn force_brand_pays_source_mana_threshold_and_alignment_scales_before_flat_damage() {
    let mut base = scythe_game("demo.build.mage-life-sorcery", "rfb-legacy.race.high-elf");
    base.items[0].intrinsic_melee_damage_dice = Some(MeleeDamageDiceDto { dice: 10, sides: 1 });
    base.items[0].enchantments.to_hit = -255;
    for element in [
        DamageType::Acid,
        DamageType::Electricity,
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Poison,
    ] {
        base.player
            .resistances
            .set(element, ResistanceLevel::Immune);
    }
    let item = base.items[0].clone();
    let mut amounts = Vec::new();
    for (evil, mana, expected_mana) in [
        (false, None, None),
        (true, None, None),
        (false, Some(2), Some(2)),
        (false, Some(3), Some(0)),
    ] {
        let mut game = base.clone();
        if evil {
            game.virtues[0].kind = VirtueKindDto::Justice;
            game.virtues[0].value = -50;
        }
        if let Some(mana) = mana {
            let pool = game.resources.get_mut("demo.resource.mana").unwrap();
            pool.maximum = 60;
            pool.current = mana;
            game.player
                .statuses
                .push(monster_combat::melee_status(STATUS_MANA_BRAND, 500, "test.force").status);
        }
        game.rng = RfbRng::seeded(73);
        let mut events = Vec::new();
        game.resolve_death_scythe_backlash(&item, None, &mut events);
        amounts.push(backlash_amount(&events) - 30);
        if let Some(expected) = expected_mana {
            assert_eq!(game.resources["demo.resource.mana"].current, expected);
        }
    }
    assert_eq!(
        amounts,
        vec![amounts[0], amounts[0] * 2, amounts[0], amounts[0] * 3]
    );
}

#[test]
fn ordinary_throw_drops_scythe_without_return_backlash() {
    let mut game = scythe_game("demo.build.warrior", "demo.race.rfb-human");
    game.items[0].curse = None;
    let slot = match &game.items[0].location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => unreachable!(),
    };
    game.unequip_slot(&slot).unwrap();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let command = GameCommand::Throw {
        item_id: "test.scythe".into(),
        direction: Direction::East,
    };
    let result = dispatch_next(&mut game, command.clone());
    assert_eq!(dispatch_next(&mut restored, command), result);
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
    assert!(matches!(game.items[0].location, ItemLocation::Ground(_)));
}

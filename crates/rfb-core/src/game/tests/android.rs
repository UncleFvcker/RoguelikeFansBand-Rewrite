// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: races_a.c, xtra2.c, cmd6.c.
use super::support::*;
use super::*;

const ANDROID: &str = "rfb-legacy.race.android";
const OIL: &str = "demo.item.flask-of-oil";
const START: Position = Position { x: 99, y: 33 };

fn prepared() -> Game {
    let mut game =
        Game::new_with_build_race_and_name(83, "demo.build.warrior", ANDROID, "test").unwrap();
    clear_monsters(&mut game);
    game.player.position = START;
    for y in 30..=36 {
        for x in 97..=118 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.world_tick = 1;
    game.reveal_current_visibility();
    game
}

#[test]
fn android_birth_omits_body_armor_and_has_oil_light_and_valid_derived_progress() {
    for build in ["warrior", "mage-life-sorcery", "berserker"] {
        let game =
            Game::new_with_build_race_and_name(83, &format!("demo.build.{build}"), ANDROID, "test")
                .unwrap();
        let oil = game.items.iter().find(|item| item.kind_id == OIL).unwrap();
        assert!((7..=12).contains(&oil.quantity));
        assert!(game.inventory_item_dto(oil).usable);
        assert!(
            game.items
                .iter()
                .any(|item| item.kind_id == "demo.item.wooden-torch")
        );
        assert!(
            !game
                .items
                .iter()
                .any(|item| item.kind_id == "demo.item.ration-of-food")
        );
        assert!(!game.items.iter().any(|item| {
            game.content
                .item(&item.kind_id)
                .unwrap()
                .equipment_slot
                .as_deref()
                == Some("body")
        }));
        assert_eq!(game.character_experience_percent(), 200);
        assert!((40..=160).contains(&game.gold));
        assert_eq!(game.experience_required_for_level(10), 2800);
        assert_eq!(game.experience_required_for_level(25), 150_000);
        assert_eq!(game.experience_required_for_level(50), 9_000_000);
        assert_eq!(
            game.progress.experience,
            game.android_equipment_experience()
        );
        assert_eq!(game.progress.maximum_experience, game.progress.experience);
        let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.snapshot(), game.snapshot());
    }
}

#[test]
fn android_chain_mail_uses_real_value_and_equip_commands_raise_and_lower_level_once() {
    let mut game = prepared();
    game.items.clear();
    game.refresh_android_experience(&mut Vec::new());
    give_inventory_item(&mut game, "test.mail", "demo.item.chain-mail");
    // COST_REAL = 1590; kind level 25 gives 1590 * 17 * 3 / 32 = 2534, CL9.
    assert_eq!(game.android_item_experience(&game.items[0]), 2534);
    game.items[0].discount_percent = 90;
    game.items[0].curse = Some(ItemCurseSeverityDto::Permanent);
    assert_eq!(game.android_item_experience(&game.items[0]), 2534);
    game.items[0].curse = None;
    game.items[0].discount_percent = 0;
    game.identify_item_instance(
        "test.mail",
        crate::game::inventory::ItemIdentificationRequest::new(true),
    );
    assert_eq!(game.android_item_experience(&game.items[0]), 2534);
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.mail".to_owned(),
            slot_id: Some("body".to_owned()),
        },
    );
    assert_eq!((game.progress.experience, game.progress.level), (2534, 9));
    let rewards = game.progress.pending_attribute_increases;
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(
        dispatch_next(
            &mut game,
            GameCommand::Unequip {
                slot_id: "body".to_owned()
            }
        ),
        dispatch_next(
            &mut restored,
            GameCommand::Unequip {
                slot_id: "body".to_owned()
            }
        )
    );
    assert_eq!(
        (
            game.progress.experience,
            game.progress.maximum_experience,
            game.progress.level
        ),
        (0, 0, 1)
    );
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.mail".to_owned(),
            slot_id: Some("body".to_owned()),
        },
    );
    assert_eq!(
        (
            game.progress.level,
            game.progress.max_level,
            game.progress.pending_attribute_increases
        ),
        (9, 9, rewards)
    );
    let before = game.progress.experience;
    game.apply_player_experience(1_000_000, &mut Vec::new());
    assert_eq!(game.progress.experience, before);
    assert_eq!(
        game.apply_player_experience_drain(1_000_000, "test.drain", &mut Vec::new()),
        0
    );
    assert_eq!(game.progress.experience, before);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.mail")
        .unwrap()
        .enchantments
        .to_armor = 9;
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.progress.experience, 4626);
    assert_eq!(game.progress.level, 12);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.mail")
        .unwrap()
        .enchantments
        .to_armor = -9;
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.progress.experience < before);
    assert!(game.progress.level < 9);
    let mut save = game.to_save();
    save.player.progress.as_mut().unwrap().experience += 1;
    assert!(Game::from_save_with_content(save, game.content.clone()).is_err());
}

#[test]
fn android_experience_excludes_jewelry_lights_and_inventory() {
    let mut game = prepared();
    game.items.clear();
    for (id, kind) in [
        ("test.mail", "demo.item.chain-mail"),
        ("test.light", "demo.item.wooden-torch"),
    ] {
        give_inventory_item(&mut game, id, kind);
    }
    let jewelry = game
        .content
        .item_definitions()
        .find(|kind| kind.rfb_base_kind.is_some_and(|kind| kind.tval == 45))
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.ring", &jewelry);
    for id in ["test.light", "test.ring"] {
        assert_eq!(
            game.android_item_experience(game.items.iter().find(|item| item.id == id).unwrap()),
            0
        );
    }
    game.refresh_android_experience(&mut Vec::new());
    assert_eq!(game.progress.experience, 0);
}

#[test]
fn android_fixed_ego_and_random_artifact_experience_survives_save_and_mundanity() {
    let mut game = prepared();
    game.items.clear();
    game.debug_add_generated_inventory_item("test.fixed", "demo.item.aeglos", 80)
        .unwrap();
    // Source value 43613; base level 1, artifact level 27, rarity 45/3:
    // effective level 29 compresses to 22, then weapon divisor 48.
    assert_eq!(
        game.android_item_experience(game.items.last().unwrap()),
        219_882
    );
    game.items.clear();
    give_inventory_item(&mut game, "test.ego", "demo.item.chain-mail");
    let ego = crate::game::ego::materialize_ego_with_rng(
        false,
        &game.content,
        &mut game.rng,
        "demo.item.chain-mail",
        vec!["rfb-legacy.affix.protection".to_owned()],
        |_| 25,
        25,
        2,
    );
    ego.apply_to(game.items.last_mut().unwrap());
    game.items.last_mut().unwrap().quality = ItemQualityDto::Exceptional;
    let ego_experience = game.android_item_experience(game.items.last().unwrap());
    assert!(ego_experience > 2534);
    give_inventory_item(&mut game, "test.random", "demo.item.chain-mail");
    let random = game.items.last_mut().unwrap();
    random.artifact_name = Some("(永恒蘑菇)".to_owned());
    random.quality = ItemQualityDto::Exceptional;
    random.enchantments.to_armor = 20;
    random.location = ItemLocation::Equipped {
        slot_id: "body".to_owned(),
    };
    let experience = game.android_item_experience(game.items.last().unwrap());
    assert!(experience > 2534);
    game.identify_item_instance(
        "test.random",
        crate::game::inventory::ItemIdentificationRequest::new(true),
    );
    let rng = game.rng.clone();
    game.refresh_android_experience(&mut Vec::new());
    assert_eq!(game.progress.experience, experience);
    assert_eq!(game.rng, rng);
    game.reveal_current_visibility();
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored.android_item_experience(
            restored
                .items
                .iter()
                .find(|item| item.id == "test.ego")
                .unwrap()
        ),
        ego_experience
    );
    for state in [&mut game, &mut restored] {
        assert!(state.mundanify_item("test.random"));
    }
    assert_eq!(
        dispatch_next(&mut game, GameCommand::Wait),
        dispatch_next(&mut restored, GameCommand::Wait)
    );
    assert_eq!(game.progress.experience, 2534);
}

#[test]
fn android_caster_downgrade_clamps_mana_and_forgets_then_remembers_without_new_learning() {
    let mut game =
        Game::new_with_build_race_and_name(83, "demo.build.mage-life-sorcery", ANDROID, "test")
            .unwrap();
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.mail", "demo.item.chain-mail");
    game.items.last_mut().unwrap().enchantments.to_armor = 9;
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.mail".to_owned(),
            slot_id: Some("body".to_owned()),
        },
    );
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.minimum_level >= 5 && ability.can_study)
        .unwrap();
    let book = game
        .items
        .iter()
        .find(|item| {
            game.content
                .item(&item.kind_id)
                .and_then(|kind| kind.ability_book_id.as_deref())
                .and_then(|id| game.content.ability_book(id))
                .is_some_and(|book| book.ability_ids.contains(&ability.id))
        })
        .unwrap()
        .id
        .clone();
    game.study_player_ability(&book, &ability.id).unwrap();
    let spent = game.spent_spell_learning;
    let maximum = game.resources["demo.resource.mana"].maximum;
    game.resources
        .get_mut("demo.resource.mana")
        .unwrap()
        .current = maximum;
    dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: "body".to_owned(),
        },
    );
    let pool = &game.resources["demo.resource.mana"];
    assert!(pool.maximum < maximum);
    assert!(pool.current <= pool.maximum);
    assert!(!game.learned_abilities.contains(&ability.id));
    assert!(game.ability_learning_order.contains(&ability.id));
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    let equip = GameCommand::Equip {
        item_id: "test.mail".to_owned(),
        slot_id: Some("body".to_owned()),
    };
    assert_eq!(
        dispatch_next(&mut game, equip.clone()),
        dispatch_next(&mut restored, equip)
    );
    assert!(game.learned_abilities.contains(&ability.id));
    assert_eq!(game.spent_spell_learning, spent);
    assert_eq!(game.resources["demo.resource.mana"].maximum, maximum);
}

#[test]
fn android_oil_works_in_pack_and_at_feet_but_food_and_other_races_cannot_replace_it() {
    for on_ground in [false, true] {
        let mut game = prepared();
        let oil = game
            .items
            .iter_mut()
            .find(|item| item.kind_id == OIL)
            .unwrap();
        oil.quantity = 2;
        if on_ground {
            oil.location = ItemLocation::Ground(START);
        }
        let id = oil.id.clone();
        game.nutrition = 1000;
        game.player.hp = 1;
        assert_eq!(
            game.item_is_edible_at_feet(game.items.iter().find(|item| item.id == id).unwrap()),
            on_ground
        );
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: id.clone(),
                target: None,
            },
        );
        assert_eq!(game.nutrition, 6000);
        assert_eq!(game.player.hp, 1);
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .quantity,
            1
        );
        give_inventory_item(&mut game, "test.food", "demo.item.ration-of-food");
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.food".to_owned(),
                target: None,
            },
        );
        assert_eq!(game.nutrition, 6000);
        give_inventory_item(&mut game, "test.water", "demo.item.water-potion");
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.water".to_owned(),
                target: None,
            },
        );
        assert_eq!(game.nutrition, 6010);
        let experience = game.progress.experience;
        give_inventory_item(&mut game, "test.experience", "demo.item.experience-potion");
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.experience".to_owned(),
                target: None,
            },
        );
        assert_eq!(game.progress.experience, experience);
    }
    let mut human = Game::new_with_build(83, "demo.build.warrior").unwrap();
    clear_monsters(&mut human);
    choose_human_talent_if_pending(&mut human);
    give_inventory_item(&mut human, "test.oil", OIL);
    let oil = human
        .items
        .iter()
        .find(|item| item.id == "test.oil")
        .unwrap();
    assert!(!human.inventory_item_dto(oil).usable);
    let nutrition = human.nutrition;
    let tick = human.world_tick;
    dispatch_next(
        &mut human,
        GameCommand::UseItem {
            item_id: "test.oil".to_owned(),
            target: None,
        },
    );
    assert_eq!(human.nutrition, nutrition);
    assert!(human.world_tick > tick);
    assert!(human.items.iter().any(|item| item.id == "test.oil"));
}

#[test]
fn android_current_stage_has_exact_damage_cost_and_free_invalid_target() {
    let mut game = prepared();
    for (level, stage, damage, cost) in [
        (1, "ray-gun", 6, 7),
        (9, "ray-gun", 10, 7),
        (10, "blaster", 15, 13),
        (24, "blaster", 29, 13),
        (25, "bazooka", 75, 26),
        (34, "bazooka", 93, 26),
        (35, "beam-cannon", 130, 40),
        (44, "beam-cannon", 157, 40),
        (45, "rocket-launcher", 315, 60),
        (50, "rocket-launcher", 350, 60),
    ] {
        // Isolate casting boundaries; equipment-driven level changes are tested above.
        game.progress.level = level;
        game.progress.max_level = level;
        game.refresh_character_skills();
        game.refresh_player_resource_maxima();
        let id = format!("rfb.ability.race.android-{stage}");
        let powers = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .filter(|ability| ability.id.starts_with("rfb.ability.race.android-"))
            .collect::<Vec<_>>();
        assert_eq!(powers.len(), 1);
        assert_eq!(powers[0].id, id);
        let activation = game.race_ability_activation(&id).unwrap();
        assert_eq!(game.innate_power_resource_cost(activation), cost);
        let mut ability = game.content.ability(&id).unwrap().clone();
        Game::apply_player_level_scaling(&mut ability, level);
        match ability.effect {
            AbilityEffectDefinition::Damage { damage_bonus, .. }
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. } => {
                assert_eq!(damage_bonus, damage)
            }
            _ => panic!("android weapon"),
        }
        let before = game.to_save();
        game.resolve_player_ability(
            &id,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.to_save(), before);
        for succeeds in [false, true] {
            game.entities.clear();
            game.entities.push(actor_from_runtime_spawn(
                "test.target",
                "demo.actor.sheep",
                Position { x: 100, y: 33 },
                10_000,
                1,
                100_000,
                true,
            ));
            game.player.hp = game.effective_player_max_hp();
            game.entities[0]
                .resistances
                .set(DamageType::Physical, ResistanceLevel::Immune);
            assert!(game.resources.is_empty());
            let seed = (0..10_000)
                .find(|seed| {
                    let roll = RfbRng::seeded(*seed).bounded(100);
                    roll == if succeeds { 99 } else { 0 }
                })
                .unwrap();
            game.rng = RfbRng::seeded(seed);
            let hp = game.player.hp;
            game.resolve_player_ability(
                &id,
                TargetSelection::Direction {
                    direction: Direction::East,
                },
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(game.player.hp, hp - cost as i32);
            assert_eq!(
                game.entities[0].hp,
                10_000 - if succeeds { i32::from(damage) } else { 0 }
            );
        }
    }
    game.progress.level = 9;
    assert!(
        game.race_ability_activation("rfb.ability.race.android-ray-gun")
            .is_some()
    );
    assert!(
        game.race_ability_activation("rfb.ability.race.android-rocket-launcher")
            .is_none()
    );
}

#[test]
fn android_passives_feed_existing_consumers_without_flat_electrical_vulnerability() {
    let mut game = prepared();
    assert!(game.player_is_nonliving());
    assert!(game.player_hold_life_sources() > 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );
    for status in [STATUS_BLEEDING, STATUS_UNWELL, STATUS_PARALYSIS] {
        game.apply_player_melee_status(status, 100, "test.passive");
        assert!(!game.player_has_status_kind(status));
    }
    assert_eq!(
        game.adjust_player_resistance_percent(DamageType::Electricity, ResistanceLevel::Resistant),
        ResistanceLevel::Resistant.reduction_percent() * 7 / 10
    );
    assert_eq!(
        game.adjust_player_resistance_percent(DamageType::Electricity, ResistanceLevel::Normal),
        0
    );
    assert_eq!(
        game.adjust_player_resistance_percent(DamageType::Electricity, ResistanceLevel::Immune),
        100
    );
    let mut form = monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.form").status;
    form.granted_race_id = Some("demo.race.rfb-human".to_owned());
    game.player.statuses.push(form);
    assert!(!game.player_is_android());
    assert!(game.player_is_native_android());
    assert_eq!(
        game.adjust_player_resistance_percent(DamageType::Electricity, ResistanceLevel::Resistant),
        ResistanceLevel::Resistant.reduction_percent()
    );
    assert_eq!(
        game.apply_player_experience_drain(100, "test.drain", &mut Vec::new()),
        0
    );
}

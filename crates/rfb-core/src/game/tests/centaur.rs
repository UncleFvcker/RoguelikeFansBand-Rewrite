// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: races_a.c, skills.c, py_birth.c.
use super::support::*;
use super::*;

const CENTAUR: &str = "rfb-legacy.race.centaur";
const JUMP: &str = "rfb.ability.race.jump";
const START: Position = Position { x: 99, y: 33 };
const EAST: Position = Position { x: 100, y: 33 };
const LANDING: Position = Position { x: 101, y: 33 };

fn prepared(level: u16) -> Game {
    let mut game =
        Game::new_with_build_race_and_name(83, "demo.build.warrior", CENTAUR, "test").unwrap();
    clear_monsters(&mut game);
    game.player.position = START;
    for y in 30..=36 {
        for x in 96..=104 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    game.player.hp = game.effective_player_max_hp();
    game
}

fn human_form(game: &mut Game) {
    let mut status =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.form").status;
    status.granted_race_id = Some("demo.race.rfb-human".to_owned());
    game.player.statuses.push(status);
    game.reconcile_player_body_slots_for_current_form();
    game.refresh_player_resource_maxima();
    game.player.hp = game.player.hp.min(game.effective_player_max_hp());
    game.reveal_current_visibility();
}

#[test]
fn centaur_native_birth_preserves_class_kit_body_supplies_and_saved_training() {
    for build in [
        "warrior",
        "berserker",
        "duelist",
        "archer",
        "sniper",
        "ranger-nature-sorcery",
        "mage-life-sorcery",
        "high-mage-death",
        "magic-eater",
        "priest-life-sorcery",
        "paladin-death",
        "warrior-mage-arcane-life",
        "mindcrafter",
    ] {
        let game =
            Game::new_with_build_race_and_name(83, &format!("demo.build.{build}"), CENTAUR, "test")
                .unwrap();
        assert_eq!(game.progress.centaur_hoof_proficiency, 4_000);
        assert!(!game.body_slots.iter().any(|slot| slot.slot_type == "boots"));
        assert!(
            game.virtues
                .iter()
                .any(|virtue| virtue.kind == VirtueKindDto::Nature)
        );
        assert!(
            game.items
                .iter()
                .any(|item| item.kind_id == "demo.item.ration-of-food"
                    && item.location == ItemLocation::Inventory)
        );
        assert!(
            game.items
                .iter()
                .any(|item| item.kind_id == "demo.item.wooden-torch"
                    && item.location == ItemLocation::Inventory)
        );
        let (build, _, class, personality) = game.character_definitions().unwrap();
        for expected in class
            .starting_items
            .iter()
            .chain(&personality.starting_items)
            .chain(&build.starting_items)
        {
            assert!(game.items.iter().any(|item| {
                item.kind_id == expected.item_kind_id
                    && (expected.quantity..=expected.maximum_quantity.unwrap_or(expected.quantity))
                        .contains(&item.quantity)
                    && matches!(item.location, ItemLocation::Equipped { .. }) == expected.equipped
            }));
        }
        let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.snapshot(), game.snapshot());
    }
    assert!(
        matches!(Game::new_with_build_race_and_name(83, "demo.build.cavalry", CENTAUR, "test"), Err(CoreError::CharacterRaceUnavailable(race)) if race == CENTAUR)
    );
}

#[test]
fn centaur_speed_forest_and_body_armor_follow_current_form_and_signed_enchantment() {
    let mut game = prepared(1);
    game.items.clear();
    give_inventory_item(&mut game, "test.armor", "demo.item.chain-mail");
    game.identify_item_instance(
        "test.armor",
        crate::game::inventory::ItemIdentificationRequest::new(true),
    );
    for (enchantment, armor) in [(0, 100), (9, 160), (-9, 10)] {
        game.items[0].enchantments.to_armor = enchantment;
        game.items[0].location = ItemLocation::Inventory;
        let bare = game.player_derived_stats().armor_class.value;
        game.items[0].location = ItemLocation::Equipped {
            slot_id: "body".to_owned(),
        };
        assert_eq!(game.player_derived_stats().armor_class.value - bare, armor);
        assert_eq!(
            (game.visible_item_modifiers(&game.items[0]).defense
                + i32::from(game.visible_item_enchantments(&game.items[0]).to_armor))
                * 10,
            armor
        );
        human_form(&mut game);
        game.items[0].location = ItemLocation::Inventory;
        let human_bare = game.player_derived_stats().armor_class.value;
        game.items[0].location = ItemLocation::Equipped {
            slot_id: "body".to_owned(),
        };
        assert_eq!(
            game.player_derived_stats().armor_class.value - human_bare,
            (14 + i32::from(enchantment)) * 10
        );
        game.player.statuses.clear();
    }
    for level in [9, 10, 49, 50] {
        game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
        assert!(game.player_is_forest_adapted());
        let speed = game.player_derived_stats().speed.value;
        human_form(&mut game);
        assert!(!game.player_is_forest_adapted());
        assert_eq!(
            speed - game.player_derived_stats().speed.value,
            i32::from(level / 10)
        );
        game.player.statuses.clear();
    }
}

#[test]
fn centaur_hooves_scale_with_level_and_do_not_inherit_weapon_enchantment() {
    let mut game = prepared(1);
    for (level, dice, sides) in [
        (15, 1, 4),
        (16, 2, 4),
        (21, 2, 5),
        (32, 3, 5),
        (42, 3, 6),
        (48, 4, 6),
        (50, 4, 6),
    ] {
        game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
        let hoof = game
            .player_mutation_innate_attack_profiles(&game.player_derived_stats())
            .pop()
            .unwrap();
        assert_eq!(hoof.attack_name.as_deref(), Some("马蹄"));
        assert_eq!((hoof.damage_dice, hoof.damage_sides), (dice, sides));
        assert!(
            (100..=200)
                .contains(&(hoof.attacks * 100 + u16::from(hoof.extra_attack_chance_percent)))
        );
    }
    let weapon = game.items.iter_mut().find(|item| matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "right-hand")).unwrap();
    weapon.enchantments.to_hit = 30;
    weapon.enchantments.to_damage = 30;
    let armed = game
        .player_mutation_innate_attack_profiles(&game.player_derived_stats())
        .pop()
        .unwrap();
    game.items.retain(|item| !matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "right-hand"));
    let unarmed = game
        .player_mutation_innate_attack_profiles(&game.player_derived_stats())
        .pop()
        .unwrap();
    assert_eq!(armed.to_damage, unarmed.to_damage);
    assert_eq!(armed.to_hit, unarmed.to_hit);
    human_form(&mut game);
    assert!(
        game.player_mutation_innate_attack_profiles(&game.player_derived_stats())
            .is_empty()
    );
}

#[test]
fn centaur_melee_trains_hooves_once_even_when_all_attacks_miss() {
    for armed in [false, true] {
        let mut game = prepared(20);
        if !armed {
            game.items.clear();
        }
        let mut penalty = monster_combat::melee_status("test.no-melee-skill", 10, "test").status;
        penalty.granted_equipment_bonuses.melee_skill = -10_000;
        game.player.statuses.push(penalty);
        game.entities.push(actor_from_runtime_spawn(
            "test.target",
            "demo.actor.ogre-mage",
            EAST,
            10_000,
            1,
            100_000,
            true,
        ));
        let index = game.entities.len() - 1;
        let mut events = Vec::new();
        game.resolve_player_melee(
            index,
            false,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.progress.centaur_hoof_proficiency, 4_008);
        assert_eq!(game.entities[index].hp, 10_000);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::PlayerMeleeMissed { .. })),
            "armed={armed}"
        );
    }
}

#[test]
fn centaur_hoof_training_caps_hashes_and_survives_other_forms_and_save_continuation() {
    let mut game = prepared(20);
    let hash = game.state_hash();
    game.train_centaur_hooves(27, &mut Vec::new());
    assert_eq!(game.progress.centaur_hoof_proficiency, 4_008);
    assert_ne!(game.state_hash(), hash);
    human_form(&mut game);
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for current in [&mut game, &mut restored] {
        current.player.statuses.clear();
        current.reconcile_player_body_slots_for_current_form();
        current.refresh_player_resource_maxima();
        current.train_centaur_hooves(27, &mut Vec::new());
    }
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    game.progress.centaur_hoof_proficiency = 7_999;
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(10) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    game.train_centaur_hooves(80, &mut events);
    assert_eq!(game.progress.centaur_hoof_proficiency, 8_000);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::CentaurHoofProficiencyImproved))
    );
    let rng = game.rng.clone();
    game.train_centaur_hooves(80, &mut Vec::new());
    assert_eq!(game.rng, rng);
    let mut invalid = game.to_save();
    invalid
        .player
        .progress
        .as_mut()
        .unwrap()
        .centaur_hoof_proficiency = 8_001;
    assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());

    let mut visitor = Game::new_with_build(83, "demo.build.warrior").unwrap();
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.hooves").status;
    form.granted_race_id = Some(CENTAUR.to_owned());
    visitor.player.statuses.push(form);
    visitor.reconcile_player_body_slots_for_current_form();
    assert_eq!(visitor.progress.centaur_hoof_proficiency, 0);
    visitor.train_centaur_hooves(27, &mut Vec::new());
    assert_eq!(visitor.progress.centaur_hoof_proficiency, 128);
    visitor.player.statuses.clear();
    visitor.reconcile_player_body_slots_for_current_form();
    assert_eq!(visitor.progress.centaur_hoof_proficiency, 128);
}

#[test]
fn centaur_jump_unlock_range_and_landing_use_source_distance_los_and_teleport_rules() {
    let mut game = prepared(14);
    let jump = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == JUMP)
        .unwrap();
    assert_eq!(jump.minimum_level, 15);
    assert!(!jump.can_cast);
    let mut ability = game.content.ability(JUMP).unwrap().clone();
    Game::apply_player_level_scaling(&mut ability, 34);
    assert_eq!(ability.target.range, 2);
    let far = TargetSelection::Position {
        position: Position { x: 102, y: 33 },
    };
    assert!(game.ability_target_plan(&ability, &far).is_none());
    ability = game.content.ability(JUMP).unwrap().clone();
    Game::apply_player_level_scaling(&mut ability, 35);
    assert_eq!(ability.target.range, 3);
    assert!(game.ability_target_plan(&ability, &far).is_some());
    let target = TargetSelection::Position { position: LANDING };
    game.push_generated_actor("test.over".to_owned(), "demo.actor.sheep", EAST);
    assert!(game.ability_target_plan(&ability, &target).is_some());
    replace_terrain(&mut game, EAST, "demo.terrain.wall");
    assert!(game.ability_target_plan(&ability, &target).is_none());
    replace_terrain(&mut game, EAST, "demo.terrain.floor");
    game.entities.last_mut().unwrap().position = LANDING;
    assert!(game.ability_target_plan(&ability, &target).is_none());
    clear_monsters(&mut game);
    let index = game.index(LANDING).unwrap();
    game.vault_cells[index] = true;
    assert!(game.ability_target_plan(&ability, &target).is_none());
    game.vault_cells[index] = false;
    for terrain in [
        "demo.terrain.created-trap",
        "demo.terrain.surface-water-deep",
        "demo.terrain.surface-lava-deep",
        "demo.terrain.surface-lava-shallow",
    ] {
        replace_terrain(&mut game, LANDING, terrain);
        assert!(
            game.ability_target_plan(&ability, &target).is_none(),
            "{terrain}"
        );
    }
}

#[test]
fn centaur_jump_spends_hp_on_failure_and_antiteleport_and_continues_after_save() {
    let mut game = prepared(35);
    assert!(game.resources.is_empty());
    let command = GameCommand::CastAbility {
        ability_id: JUMP.to_owned(),
        target: TargetSelection::Position { position: LANDING },
    };
    let before = game.to_save();
    game.resolve_player_ability(
        JUMP,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.to_save(), before);
    for succeeds in [false, true] {
        game.player.position = START;
        game.player.hp = game.effective_player_max_hp();
        let seed = (0..1000)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) == if succeeds { 99 } else { 0 })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let hp = game.player.hp;
        let mut events = Vec::new();
        game.resolve_player_ability(
            JUMP,
            TargetSelection::Position { position: LANDING },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.player.hp, hp - 10);
        assert_eq!(game.player.position, if succeeds { LANDING } else { START });
    }
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        dispatch_next(&mut restored, GameCommand::Wait),
        dispatch_next(&mut game, GameCommand::Wait)
    );
    game.player.position = START;
    for item in &mut game.items {
        if matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "body") {
            item.location = ItemLocation::Inventory;
        }
    }
    give_inventory_item(&mut game, "test.no-teleport", "demo.item.chain-mail");
    let armor = game.items.last_mut().unwrap();
    armor.location = ItemLocation::Equipped {
        slot_id: "body".to_owned(),
    };
    armor
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::AntiTeleport);
    assert!(game.player_has_anti_teleport());
    game.rng = RfbRng::seeded(
        (0..1000)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) == 99)
            .unwrap(),
    );
    let hp = game.player.hp;
    let result = dispatch_next(&mut game, command);
    assert_eq!(game.player.position, START);
    assert!(game.player.hp <= hp - 10);
    assert!(
        result
            .events
            .iter()
            .any(|event| event.message_key == "jump-teleport-blocked")
    );
}

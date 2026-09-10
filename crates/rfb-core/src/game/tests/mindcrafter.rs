// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, dispatch_next, give_inventory_item};
use super::*;

const BUILD: &str = "demo.build.mindcrafter";
const CLEAR_MIND: &str = "demo.ability.mindcrafter-clear-mind";
const MANA: &str = "demo.resource.mana";

mod spells;

#[test]
fn normal_creation_projects_cast_reasons_and_preserves_upgraded_abilities_after_loading() {
    let mut game =
        Game::new_with_build_race_and_name(924, BUILD, "demo.race.rfb-human", " 心灵旅人 ")
            .unwrap();
    assert_eq!(game.snapshot().player.name, "心灵旅人");
    let ability = |game: &Game, slug: &str| {
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == format!("demo.ability.mindcrafter-{slug}"))
            .unwrap()
    };
    assert!(ability(&game, "neural-blast").can_cast);
    game.debug_prepare_mindcrafter_e2e(2);
    assert!(ability(&game, "precognition").can_cast);
    assert_eq!(
        ability(&game, "psycho-storm").unavailable_reason.as_deref(),
        Some("level-too-low")
    );
    for (status, reason) in [
        (STATUS_CONFUSION, "confused"),
        (STATUS_FEAR, "afraid"),
        (STATUS_BERSERK, "berserk"),
        (crate::effect::STATUS_ANTI_MAGIC, "anti-magic"),
    ] {
        game.apply_player_mental_status(status, 10, "test");
        let projected = ability(&game, "precognition");
        assert!(!projected.can_cast);
        assert_eq!(projected.unavailable_reason.as_deref(), Some(reason));
        let mut events = Vec::new();
        game.resolve_player_ability(
            &projected.id,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityCastUnavailable { reason: actual, .. } if actual == reason)));
        game.player.statuses.clear();
    }
    game.resources.get_mut(MANA).unwrap().current = 0;
    assert_eq!(
        ability(&game, "precognition").unavailable_reason.as_deref(),
        Some("insufficient-resource")
    );
    game.debug_prepare_mindcrafter_e2e(45);
    super::support::choose_human_talent_if_pending(&mut game);
    let door = ability(&game, "minor-displacement");
    assert_eq!(door.resource_cost, 42);
    assert_eq!(
        door.target_spec.modes,
        [rfb_protocol::TargetModeDto::Position]
    );
    assert!(door.can_cast);
    assert!(game.snapshot().player.ability_learning.is_none());
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        restored.snapshot().player.abilities,
        game.snapshot().player.abilities
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

fn mindcrafter(level: u16) -> Game {
    let mut game = Game::new_with_build(924, BUILD).expect("formal Mindcrafter build");
    clear_monsters(&mut game);
    game.progress.level = level;
    game.progress.max_level = level;
    game.refresh_character_skills();
    game.refresh_player_ability_state();
    game.player.hp = game.effective_player_max_hp();
    game
}

fn cast_clear_mind(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        CLEAR_MIND,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn equip(game: &mut Game, id: &str, kind: &str, slot_type: &str) {
    give_inventory_item(game, id, kind);
    let slot_id = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == slot_type)
        .unwrap()
        .id
        .clone();
    game.items.last_mut().unwrap().location = ItemLocation::Equipped { slot_id };
}

#[test]
fn birth_has_wisdom_mana_proficiencies_and_no_spell_study() {
    let game = mindcrafter(1);
    let snapshot = game.snapshot();
    let attributes = game.effective_player_attributes();
    assert_eq!(
        [
            attributes.strength,
            attributes.intelligence,
            attributes.wisdom,
            attributes.dexterity,
            attributes.constitution,
            attributes.charisma
        ],
        [12, 13, 16, 12, 12, 15]
    );
    assert_eq!(game.resources[MANA].maximum, 9);
    assert!(snapshot.player.ability_learning.is_none());
    assert!(game.learned_abilities.is_empty());
    assert!(game.ability_progress.is_empty());
    assert!(!game.uses_spell_scrolls());
    let clear = snapshot
        .player
        .abilities
        .iter()
        .find(|entry| entry.id == CLEAR_MIND)
        .unwrap();
    assert_eq!(clear.source, AbilitySourceDto::Class);
    assert_eq!(clear.minimum_level, 15);
    assert!(!clear.can_cast);
    assert!(!clear.can_study);
    let sword = snapshot
        .player
        .progress
        .weapon_proficiencies
        .iter()
        .find(|entry| entry.item_kind_id == "demo.item.small-sword")
        .unwrap();
    assert_eq!((sword.current, sword.maximum), (4_000, 6_000));
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 2_000);
    assert_eq!(game.progress.dual_wielding_proficiency, 0);
    assert_eq!(
        &game.virtues[..3]
            .iter()
            .map(|entry| entry.kind)
            .collect::<Vec<_>>(),
        &[
            VirtueKindDto::Harmony,
            VirtueKindDto::Enlightenment,
            VirtueKindDto::Patience
        ]
    );
    for kind in ["demo.item.small-sword", "demo.item.soft-leather-armour"] {
        let item = game.items.iter().find(|item| item.kind_id == kind).unwrap();
        assert!(matches!(item.location, ItemLocation::Equipped { .. }));
        assert_eq!(
            game.item_identification(item),
            ItemIdentificationDto::Identified
        );
    }
    let potions = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.swiftstep-tonic")
        .unwrap();
    assert!((2..=5).contains(&potions.quantity));
    assert_eq!(potions.location, ItemLocation::Inventory);
    assert!(!game.items.iter().any(|item| {
        game.content
            .item(&item.kind_id)
            .unwrap()
            .ability_book_id
            .is_some()
    }));
}

#[test]
fn clear_mind_unlocks_matches_projection_and_blocks_even_free_pets() {
    let mut game = mindcrafter(14);
    let rng_before = game.rng.clone();
    assert!(matches!(cast_clear_mind(&mut game).as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "level-too-low"));
    assert_eq!(game.rng, rng_before);
    game.progress.level = 15;
    game.refresh_player_ability_state();
    game.resources.get_mut(MANA).unwrap().current = 0;
    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|entry| entry.id == CLEAR_MIND)
        .unwrap();
    assert!(projected.can_cast);
    assert_eq!(projected.resource_cost, 0);
    assert_eq!(
        projected.effects,
        [AbilityEffectSpecDto::ClearMind { amount: 2 }]
    );
    // Select an ordinary success roll; the reported failure rate must be the UI's rate.
    game.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(100) >= u64::from(projected.failure_percent))
        .unwrap();
    let events = cast_clear_mind(&mut game);
    assert!(events.iter().any(
        |event| matches!(event, DomainEvent::AbilityCastSucceeded { resolution }
        if resolution.failure_percent == projected.failure_percent && resolution.resource_paid == 0)
    ));
    assert_eq!(game.resources[MANA].current, 2);

    let position = game.position_in_direction(Direction::East);
    game.push_generated_actor(
        "test.mindcrafter-pet".to_owned(),
        "demo.actor.horse",
        position,
    );
    let pet = game.entities.last_mut().unwrap();
    pet.controller_id = Some(game.player.id.clone());
    assert_eq!(game.pet_upkeep().percent, 0);
    let before = game.rng.clone();
    assert!(matches!(cast_clear_mind(&mut game).as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "pets-require-attention"));
    assert_eq!(game.rng, before);
    assert_eq!(game.resources[MANA].current, 2);
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|entry| entry.id == CLEAR_MIND)
            .unwrap()
            .can_cast
    );
}

#[test]
fn level_passives_stack_with_race_and_rest_recovers_without_a_power_roll() {
    let mut game = mindcrafter(9);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fear),
        ResistanceLevel::Normal
    );
    game.progress.level = 10;
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fear),
        ResistanceLevel::Resistant
    );
    game.progress.level = 19;
    assert!(!game.player_sustains_attribute(AttributeKind::Wisdom));
    game.progress.level = 20;
    assert!(game.player_sustains_attribute(AttributeKind::Wisdom));
    game.progress.level = 29;
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Confusion),
        ResistanceLevel::Normal
    );
    let recovery29 = game.player_resource_recovery_amount(MANA, true);
    game.progress.level = 30;
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Confusion),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.player_resource_recovery_amount(MANA, true),
        recovery29 + 1
    );
    game.progress.level = 39;
    assert!(!game.player_has_permanent_telepathy());
    game.progress.level = 40;
    assert!(game.player_has_permanent_telepathy());
    let details = game.character_trait_details(&game.player_derived_stats());
    let class = details
        .sources
        .iter()
        .find(|source| source.source_id == "demo.class.mindcrafter")
        .unwrap();
    assert!(class.passives.contains(&EquipmentPassiveDto::SustainWisdom));
    assert!(class.passives.contains(&EquipmentPassiveDto::Telepathy));

    let mut rested = mindcrafter(15);
    rested.resources.get_mut(MANA).unwrap().current = 0;
    let expected = rested.player_resource_recovery_amount(MANA, true);
    let update = dispatch_next(&mut rested, GameCommand::Rest { turns: 1 });
    assert_eq!(rested.resources[MANA].current, expected);
    assert!(
        !update
            .events
            .iter()
            .any(|event| matches!(event.outcome, Some(GameEventOutcomeDto::AbilityCast { .. })))
    );

    let mut flayer =
        Game::new_with_build_race_and_name(925, BUILD, "rfb-legacy.race.mindflayer", "Mentalist")
            .unwrap();
    flayer.progress.level = 30;
    assert!(flayer.player_has_permanent_telepathy());
    assert!(flayer.player_sustains_attribute(AttributeKind::Intelligence));
    assert!(flayer.player_sustains_attribute(AttributeKind::Wisdom));
    let powers = flayer.snapshot().player.abilities;
    assert!(powers.iter().any(
        |ability| ability.source == rfb_protocol::AbilitySourceDto::Race
            && ability.id == "rfb.ability.race.mind-blast"
    ));
    assert!(powers.iter().any(
        |ability| ability.source == rfb_protocol::AbilitySourceDto::Class
            && ability.id == "demo.ability.mindcrafter-neural-blast"
    ));
}

#[test]
fn weapon_blows_use_attributes_weight_and_two_handed_grip() {
    let mut game = mindcrafter(1);
    let initial = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        (initial.attacks, initial.extra_attack_chance_percent),
        (1, 0)
    );
    game.progress.attributes.strength = 118;
    game.progress.attributes.dexterity = 118;
    let fast = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!((fast.attacks, fast.extra_attack_chance_percent), (3, 50));
    let weapon = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.small-sword")
        .unwrap();
    weapon.intrinsic_weight_tenths_pound = Some(200);
    let two_hands = game.player_melee_profile(&game.player_derived_stats());
    assert!(
        u32::from(two_hands.attacks) * 100 + u32::from(two_hands.extra_attack_chance_percent) < 350
    );
    equip(
        &mut game,
        "test.mindcrafter-shield",
        "demo.item.small-leather-shield",
        "shield",
    );
    let shielded = game.player_melee_profile(&game.player_derived_stats());
    assert!(
        u32::from(shielded.attacks) * 100 + u32::from(shielded.extra_attack_chance_percent)
            < u32::from(two_hands.attacks) * 100 + u32::from(two_hands.extra_attack_chance_percent)
    );
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.small-sword")
        .unwrap()
        .intrinsic_weight_tenths_pound = Some(1_000);
    let heavy = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!((heavy.attacks, heavy.extra_attack_chance_percent), (1, 0));
}

#[test]
fn mana_encumbrance_counts_half_weapon_weight_without_glove_penalty() {
    let mut game = mindcrafter(20);
    game.items.clear();
    game.refresh_player_ability_state();
    let unburdened = game.resources[MANA].maximum;
    equip(
        &mut game,
        "test.gloves",
        "demo.item.set-of-gauntlets",
        "gloves",
    );
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(100);
    equip(&mut game, "test.weapon", "demo.item.small-sword", "weapon");
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(600);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, unburdened);
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(1400);
    game.refresh_player_ability_state();
    assert_eq!(
        game.resources[MANA].maximum,
        unburdened - unburdened * 400 / 800
    );
}

#[test]
fn sensing_preserves_weak_knowledge_and_uses_the_world_processing_interval() {
    let mut game = mindcrafter(35);
    game.items.clear();
    game.item_property_knowledge.clear();
    equip(
        &mut game,
        "test.sensed-sword",
        "demo.item.small-sword",
        "weapon",
    );
    game.items.last_mut().unwrap().enchantments.to_hit = 1;
    give_inventory_item(
        &mut game,
        "test.sensed-wand",
        "demo.item.magic-missile-wand",
    );
    game.items.last_mut().unwrap().curse = Some(ItemCurseSeverityDto::Heavy);
    game.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(3) == 0)
        .unwrap();
    game.world_tick = 9;
    let rng = game.rng.clone();
    game.process_class_item_sensing();
    assert_eq!(game.rng, rng);
    assert!(game.item_property_knowledge.is_empty());
    game.world_tick = 10;
    game.process_class_item_sensing();
    assert_eq!(
        game.item_feeling(&game.items[0]),
        Some(rfb_protocol::ItemFeelingDto::Enchanted)
    );
    assert_eq!(
        game.item_feeling(&game.items[1]),
        Some(rfb_protocol::ItemFeelingDto::Bad)
    );
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Unexamined
    );
    assert_eq!(
        game.visible_item_enchantments(&game.items[0]),
        ItemEnchantmentsDto::default()
    );
    let known = game.item_property_knowledge.clone();
    let rng = game.rng.clone();
    game.process_class_item_sensing();
    assert_eq!(game.item_property_knowledge, known);
    assert_eq!(game.rng, rng);

    game.item_property_knowledge.clear();
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 100, "test.confused").status);
    game.process_class_item_sensing();
    assert!(game.item_property_knowledge.is_empty());
    assert_eq!(game.rng, rng);
    game.player.statuses.clear();
    let knowledge = game
        .virtues
        .iter()
        .position(|entry| entry.kind == VirtueKindDto::Knowledge)
        .unwrap_or(0);
    game.virtues[knowledge].kind = VirtueKindDto::Knowledge;
    game.virtues[knowledge].value = 100;
    game.process_class_item_sensing();
    assert_eq!(
        game.item_feeling(&game.items[0]),
        Some(rfb_protocol::ItemFeelingDto::Good)
    );
}

#[test]
fn auto_identify_uses_devices_then_scrolls_then_twelve_mana() {
    let mut game = mindcrafter(24);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.interface_locale = LocaleDto::EnUs;
    assert!(
        game.configure_mogaminator(
            true,
            false,
            rfb_protocol::AutoGetModeDto::Off,
            LocaleDto::EnUs,
            "~?unidentified items".to_owned()
        )
        .is_empty()
    );
    give_inventory_item(&mut game, "test.to-identify", "demo.item.dagger");
    game.resources.get_mut(MANA).unwrap().current = 12;
    let rng = game.rng.clone();
    assert!(
        game.apply_mogaminator_to_carried_items(vec!["test.to-identify".to_owned()])
            .unwrap()
            .is_empty()
    );
    assert_eq!(game.resources[MANA].current, 12);
    game.progress.level = 25;
    game.resources.get_mut(MANA).unwrap().current = 11;
    assert!(
        game.apply_mogaminator_to_carried_items(vec!["test.to-identify".to_owned()])
            .unwrap()
            .is_empty()
    );
    assert_eq!(game.resources[MANA].current, 11);
    game.resources.get_mut(MANA).unwrap().current = 12;
    let outcomes = game
        .apply_mogaminator_to_carried_items(vec!["test.to-identify".to_owned()])
        .unwrap();
    assert_eq!(outcomes.len(), 1);
    assert_eq!(game.resources[MANA].current, 0);
    assert_eq!(game.rng, rng);
    let mut events = Vec::new();
    game.record_mogaminator_resolutions(outcomes, &mut events, &mut BTreeSet::new());
    assert!(
        matches!(events.as_slice(), [DomainEvent::ItemIdentified { display_name_key, .. }]
        if display_name_key == "class-demo-mindcrafter-name")
    );

    give_inventory_item(&mut game, "test.identify-staff", "demo.item.identify-staff");
    give_inventory_item(
        &mut game,
        "test.identify-scroll",
        "demo.item.appraisal-scroll",
    );
    game.mark_item_aware("demo.item.identify-staff");
    game.mark_item_aware("demo.item.appraisal-scroll");
    let staff = game
        .items
        .iter()
        .find(|item| item.id == "test.identify-staff")
        .unwrap();
    let charge_before = staff.charges.unwrap().current;
    let charge_cost = staff.activation.as_ref().unwrap().cost;
    game.resources.get_mut(MANA).unwrap().current = 12;
    for id in ["test.device-target", "test.scroll-target"] {
        give_inventory_item(&mut game, id, "demo.item.short-sword");
        let outcomes = game
            .apply_mogaminator_to_carried_items(vec![id.to_owned()])
            .unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(game.resources[MANA].current, 12);
        let staff = game
            .items
            .iter_mut()
            .find(|item| item.id == "test.identify-staff")
            .unwrap();
        if id == "test.device-target" {
            assert_eq!(staff.charges.unwrap().current, charge_before - charge_cost);
            staff.charges.as_mut().unwrap().current = 0;
            assert!(
                game.items
                    .iter()
                    .any(|item| item.id == "test.identify-scroll")
            );
        } else {
            assert!(
                !game
                    .items
                    .iter()
                    .any(|item| item.id == "test.identify-scroll")
            );
        }
    }
}

#[test]
fn tomte_sensing_and_free_identification_precede_paid_mindcraft_without_replacing_it() {
    for (level, heavy_headgear, expected_cost) in [(39, false, 12), (40, false, 0), (40, true, 12)]
    {
        let mut game =
            Game::new_with_build_race_and_name(928, BUILD, "rfb-legacy.race.tomte", "心灵感知")
                .unwrap();
        game.debug_prepare_mindcrafter_e2e(level);
        if heavy_headgear {
            give_inventory_item(&mut game, "test.helmet", "demo.item.iron-helm");
            assert!(
                game.equip_inventory_item("test.helmet", Some("head"))
                    .is_some()
            );
        }
        assert!(
            game.configure_mogaminator(
                true,
                false,
                rfb_protocol::AutoGetModeDto::Off,
                LocaleDto::EnUs,
                "~?unidentified items".to_owned()
            )
            .is_empty()
        );
        game.interface_locale = LocaleDto::EnUs;
        give_inventory_item(&mut game, "test.sensed", "demo.item.dagger");
        let item = game.items.last_mut().unwrap();
        item.location = ItemLocation::Ground(game.player.position);
        item.enchantments.to_hit = 2;
        game.resources.get_mut(MANA).unwrap().current = 12;
        game.apply_player_floor_item_knowledge();
        assert_eq!(
            game.item_feeling(game.items.last().unwrap()),
            (!heavy_headgear).then_some(rfb_protocol::ItemFeelingDto::Good)
        );
        assert_eq!(
            game.item_property_knowledge
                .get("test.sensed")
                .is_some_and(|knowledge| knowledge.appraised),
            expected_cost == 0
        );
        if level == 39 {
            game.items.last_mut().unwrap().location = ItemLocation::Inventory;
            game.lose_mindcraft_information(&mut BTreeSet::new());
            assert_eq!(
                game.item_feeling(game.items.last().unwrap()),
                Some(rfb_protocol::ItemFeelingDto::Good)
            );
        }
        let outcomes = game
            .apply_mogaminator_to_items(vec!["test.sensed".to_owned()], false)
            .unwrap();
        assert_eq!(outcomes.len(), usize::from(expected_cost > 0));
        assert_eq!(game.resources[MANA].current, 12 - expected_cost);
        assert!(game.item_property_knowledge["test.sensed"].appraised);
    }
}

#[test]
fn low_wisdom_and_stun_failure_are_bounded_and_fail_without_backlash() {
    let mut game = mindcrafter(15);
    game.progress.attributes.wisdom = 3;
    let mut stun = monster_combat::melee_status(STATUS_STUN, 100, "test.stun").status;
    stun.intensity = 80;
    game.player.statuses.push(stun);
    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|entry| entry.id == CLEAR_MIND)
        .unwrap();
    assert_eq!(projected.failure_percent, 95);
    game.resources.get_mut(MANA).unwrap().current = 0;
    game.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(100) < 95)
        .unwrap();
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(100);
    assert!(
        matches!(cast_clear_mind(&mut game).as_slice(), [DomainEvent::AbilityCastFailed { resolution }]
        if resolution.failure_percent == 95 && resolution.resource_cost == 0)
    );
    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.resources[MANA].current, 0);
    assert_eq!(game.player.statuses.len(), 1);
}

#[test]
fn bookless_resources_and_sensed_knowledge_round_trip_and_continue() {
    let mut game =
        Game::new_with_build_race_and_name(926, BUILD, "rfb-legacy.race.mindflayer", "Mentalist")
            .unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(35), &mut Vec::new());
    game.resources.get_mut(MANA).unwrap().current = 0;
    game.debug_set_ability_casts_succeed(true);
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: CLEAR_MIND.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.cast-success")
    );
    assert_eq!(game.resources[MANA].current, 3);
    game.debug_set_ability_casts_succeed(false);
    let weapon_id = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.small-sword")
        .unwrap()
        .id
        .clone();
    game.item_property_knowledge.remove(&weapon_id);
    game.items
        .iter_mut()
        .find(|item| item.id == weapon_id)
        .unwrap()
        .enchantments
        .to_hit = 1;
    // Equipping identifies gear in this runtime; sense carried gear instead.
    game.items
        .iter_mut()
        .find(|item| item.id == weapon_id)
        .unwrap()
        .location = ItemLocation::Inventory;
    game.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(3) == 0)
        .unwrap();
    game.world_tick = game.world_tick.next_multiple_of(10);
    game.process_class_item_sensing();
    assert_eq!(
        game.item_property_knowledge[&weapon_id].feeling,
        Some(rfb_protocol::ItemFeelingDto::Enchanted)
    );
    let mut restored = Game::from_save(game.to_save()).expect("bookless character should load");
    let before = serde_json::to_value(game.snapshot()).unwrap();
    let after = serde_json::to_value(restored.snapshot()).unwrap();
    let differences = before
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| after.get(*key) != Some(*value))
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    assert!(before == after, "snapshot fields differ: {differences:?}");
    assert_eq!(game.state_hash(), restored.state_hash());
    let original = dispatch_next(&mut game, GameCommand::Rest { turns: 1 });
    let loaded = dispatch_next(&mut restored, GameCommand::Rest { turns: 1 });
    assert_eq!(original, loaded);
    assert_eq!(game.state_hash(), restored.state_hash());
}

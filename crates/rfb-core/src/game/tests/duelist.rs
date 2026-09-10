// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, command, dispatch_next, give_inventory_item};
use super::*;
use crate::effect::STATUS_ANTI_MAGIC;

const BUILD: &str = "demo.build.duelist";
const MARK: &str = "demo.ability.duelist-mark-target";

mod abilities;
mod choices;
mod combat_rules;
mod rewards;

#[test]
fn ui_fixture_uses_real_levels_and_round_trips_visible_targets_and_hp_costs() {
    let mut game = Game::new_with_build(923, BUILD).unwrap();
    super::support::descend_one_floor(&mut game);
    let revealed_trap = game.player.position;
    let trap_index = game.index(revealed_trap).unwrap();
    game.terrain[trap_index] = "demo.terrain.warren-snare".to_owned();
    game.revealed_terrain.insert(revealed_trap);
    give_inventory_item(&mut game, "test.newly-visible", "demo.item.healing-potion");
    let item_position = Position { x: 40, y: 20 };
    super::support::replace_terrain(&mut game, item_position, "demo.terrain.floor");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(item_position);
    Game::from_save(game.to_save()).unwrap();
    game.debug_prepare_duelist_e2e(35, 7, false).unwrap();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let snapshot = restored.snapshot();
    assert_eq!(snapshot.player.progress.level, 35);
    assert_eq!(snapshot.entities.len(), 2);
    assert!(
        snapshot
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == "demo.ability.duelist-charge"
                && ability.hit_point_cost == 10)
    );
    game.debug_prepare_duelist_e2e(35, 7, true).unwrap();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.snapshot().player.hp, 1);
    assert!(
        restored
            .snapshot()
            .player
            .abilities
            .iter()
            .filter(|ability| ability.hit_point_cost > 1)
            .all(|ability| !ability.can_cast)
    );
}

fn at_level(level: u16) -> Game {
    let mut game = duelist();
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    super::support::choose_human_talent_if_pending(&mut game);
    game.player.hp = game.effective_player_max_hp();
    game
}

fn duelist() -> Game {
    let mut game = Game::new_with_build(923, BUILD).unwrap();
    clear_monsters(&mut game);
    game.player.position = Position { x: 77, y: 33 };
    game.terrain.fill("demo.terrain.floor".to_owned());
    game
}

fn target(game: &mut Game, id: &str, offset: i32) {
    let mut actor = game.generated_actor(
        id.to_owned(),
        "demo.actor.sheep",
        Position {
            x: game.player.position.x + offset,
            y: game.player.position.y,
        },
    );
    actor.energy_need = STANDARD_ACTION_COST;
    game.entities.push(actor);
}

fn mark(id: &str) -> GameCommand {
    GameCommand::CastAbility {
        ability_id: MARK.to_owned(),
        target: TargetSelection::Entity {
            entity_id: id.to_owned(),
        },
    }
}

#[test]
fn birth_merges_equipment_has_no_mana_and_uses_source_proficiencies() {
    let mut game = duelist();
    assert!(game.resources.is_empty());
    assert!(game.learned_abilities.is_empty());
    let snapshot = game.snapshot();
    assert!(snapshot.player.ability_learning.is_none());
    assert_eq!(snapshot.player.duelist_target_id, None);
    assert_eq!(game.progress.dual_wielding_proficiency, 0);
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 0);
    for (kind, expected) in [
        ("demo.item.rapier", (4000, 8000)),
        ("demo.item.dagger", (4000, 7000)),
        ("demo.item.broad-axe", (0, 4000)),
        ("demo.item.sling", (0, 0)),
        ("demo.item.wizardstaff", (0, 0)),
    ] {
        let proficiency = snapshot
            .player
            .progress
            .weapon_proficiencies
            .iter()
            .find(|entry| entry.item_kind_id == kind)
            .unwrap_or_else(|| panic!("missing proficiency: {kind}"));
        assert_eq!(
            (proficiency.current, proficiency.maximum),
            expected,
            "{kind}"
        );
    }
    for kind in ["demo.item.rapier", "demo.item.soft-leather-armour"] {
        let item = game
            .items
            .iter()
            .find(|item| item.kind_id == kind)
            .unwrap_or_else(|| {
                panic!(
                    "missing birth item {kind}: {:?}",
                    game.items
                        .iter()
                        .map(|item| &item.kind_id)
                        .collect::<Vec<_>>()
                )
            });
        assert!(matches!(item.location, ItemLocation::Equipped { .. }));
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.kind_id == "demo.item.swiftstep-tonic")
            .unwrap()
            .quantity,
        1
    );
    assert!(game.duelist_equipment_error().is_none());
    let x = i32::from(
        game.effective_player_attributes()
            .index(AttributeKind::Intelligence),
    ) + 3;
    assert!(
        game.player_derived_stats()
            .armor_class
            .contributions
            .iter()
            .any(|entry| entry.source_id == "demo.class.duelist"
                && entry.amount == -50 + x / 2 + x / 50)
    );
    game.unequip_slot("body").unwrap();
    assert!(game.player_derived_stats().armor_class.value < 0);
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
    let tomte =
        Game::new_with_build_race_and_name(923, BUILD, "rfb-legacy.race.tomte", "test").unwrap();
    assert!(
        tomte
            .items
            .iter()
            .any(|item| item.kind_id == "demo.item.rapier")
    );
    assert!(Game::from_save(tomte.to_save()).is_ok());
    assert!(matches!(
        Game::new_with_build_race_and_name(923, BUILD, "rfb-legacy.race.tonberry", "test"),
        Err(CoreError::CharacterRaceUnavailable(_))
    ));
}

#[test]
fn strong_sensing_and_actual_experience_growth_keep_the_class_parameters() {
    let mut game = duelist();
    give_inventory_item(&mut game, "test.sense", "demo.item.dagger");
    game.items.last_mut().unwrap().enchantments.to_damage = 2;
    game.apply_player_item_knowledge(vec!["test.sense".to_owned()]);
    assert_eq!(
        game.item_feeling(game.items.last().unwrap()),
        Some(rfb_protocol::ItemFeelingDto::Good)
    );
    assert_ne!(
        game.item_identification(game.items.last().unwrap()),
        ItemIdentificationDto::Identified
    );
    game.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(
        game.item_feeling(game.items.last().unwrap()),
        Some(rfb_protocol::ItemFeelingDto::Good)
    );
    let old_melee = game.progress.skills["demo.skill.melee"].current;
    game.apply_player_experience(game.experience_required_for_level(10), &mut Vec::new());
    assert_eq!(game.progress.level, 10);
    assert!(game.progress.skills["demo.skill.melee"].current > old_melee);
    assert!(game.resources.is_empty());
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn armor_threshold_and_equipment_restrictions_clear_without_restoring_challenges() {
    let mut game = duelist();
    target(&mut game, "test.a", 2);
    game.duelist_target_id = Some("test.a".to_owned());
    let armor = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.soft-leather-armour")
        .unwrap();
    armor.intrinsic_weight_tenths_pound = Some(123);
    assert_eq!(game.duelist_equipment_error(), None);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.soft-leather-armour")
        .unwrap()
        .intrinsic_weight_tenths_pound = Some(124);
    assert_eq!(game.duelist_equipment_error(), Some("duelist-heavy-armor"));
    game.refresh_duelist_challenge();
    assert_eq!(game.duelist_target_id, None);
    game.progress.level = 2;
    assert_eq!(game.duelist_equipment_error(), None);
    game.refresh_duelist_challenge();
    assert_eq!(game.duelist_target_id, None);

    for (kind, reason) in [
        ("demo.item.small-leather-shield", "duelist-shield"),
        ("demo.item.capture-ball", "duelist-shield"),
        ("demo.item.dagger", "duelist-multiple-weapons"),
    ] {
        let mut game = duelist();
        target(&mut game, "test.a", 2);
        game.duelist_target_id = Some("test.a".to_owned());
        give_inventory_item(&mut game, "test.equip", kind);
        game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(0);
        let slot = "shield";
        // Use the real equipment transaction and the declared slot identities.
        let slot_id = game
            .body_slots
            .iter()
            .find(|entry| entry.slot_type == slot)
            .map(|entry| entry.id.clone());
        assert!(
            game.equip_inventory_item("test.equip", slot_id.as_deref())
                .is_some(),
            "{kind}"
        );
        assert_eq!(game.duelist_equipment_error(), Some(reason), "{kind}");
        assert_eq!(game.duelist_target_id, None);
        let ItemLocation::Equipped { slot_id } = game
            .items
            .iter()
            .find(|item| item.id == "test.equip")
            .unwrap()
            .location
            .clone()
        else {
            panic!()
        };
        game.unequip_slot(&slot_id).unwrap();
        assert_eq!(game.duelist_target_id, None);
    }

    // Poison Needle is not a formal item yet; exercise its source-kind boundary
    // without adding its unrelated weapon rules or allocation to this batch.
    let mut content = rfb_content::decode_content(BUILT_IN_CONTENT_BYTES)
        .unwrap()
        .content;
    let kind = content
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.rapier")
        .unwrap()
        .rfb_base_kind
        .as_mut()
        .unwrap();
    kind.source_index = 75;
    kind.sval = 32;
    game.content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(content).unwrap(),
    ));
    assert_eq!(
        game.duelist_equipment_error(),
        Some("duelist-poison-needle")
    );
}

#[test]
fn leaving_the_floor_clears_and_off_floor_references_cannot_be_loaded() {
    let mut game = duelist();
    target(&mut game, "test.a", 2);
    game.duelist_target_id = Some("test.a".to_owned());
    game.transition_floor("demo.floor.warrens-depth-1".to_owned(), None, None, false)
        .unwrap()
        .unwrap();
    assert_eq!(game.duelist_target_id, None);
    assert!(Game::from_save(game.to_save()).is_ok());
    game.duelist_target_id = Some("test.a".to_owned());
    assert!(Game::from_save(game.to_save()).is_err());
}

#[test]
fn weapon_damage_and_positive_cap_preserve_zero_attacks() {
    let mut game = duelist();
    let weapon_index = game
        .items
        .iter()
        .position(|item| item.kind_id == "demo.item.rapier")
        .unwrap();
    let normal = game.player_melee_profile(&game.player_derived_stats());
    game.items[weapon_index]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks_delta_percent = 500;
    let boosted = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        (boosted.attacks, boosted.extra_attack_chance_percent),
        (1, 0)
    );
    assert_eq!(boosted.to_damage, normal.to_damage);
    game.items[weapon_index]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks_delta_percent = -200;
    let zero = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!((zero.attacks, zero.extra_attack_chance_percent), (0, 0));
    target(&mut game, "test.a", 1);
    let hp = game.entities[0].hp;
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.entities[0].hp, hp);
    assert_eq!(game.duelist_target_id, None);
    game.items[weapon_index]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks_delta_percent = 0;
    let weight = game.item_instance_weight(&game.items[weapon_index]);
    let bonus = i32::from(
        game.effective_player_attributes()
            .index(AttributeKind::Dexterity),
    ) + 3
        - 10
        + i32::from(game.progress.level / 2)
        - i32::from(weight / 10);
    game.items[weapon_index]
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::AntiMagic);
    let invalid = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(normal.to_damage - invalid.to_damage, bonus);
    game.items[weapon_index]
        .intrinsic_properties
        .passives
        .clear();
    game.apply_player_experience(game.experience_required_for_level(10), &mut Vec::new());
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tonberry").status;
    form.granted_race_id = Some("rfb-legacy.race.tonberry".to_owned());
    game.player.statuses.push(form);
    let profile = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        (profile.attacks, profile.extra_attack_chance_percent),
        (1, 0)
    );
    assert!(
        !profile
            .attack_sources
            .iter()
            .any(|entry| entry.source_id == "rfb-legacy.race.tonberry")
    );
    let expected_damage = game.player_derived_stats().melee_damage_bonus.value
        + i32::from(
            game.effective_player_attributes()
                .index(AttributeKind::Dexterity),
        )
        + 3
        - 10
        + i32::from(game.progress.level / 2)
        - i32::from(weight / 10);
    assert_eq!(profile.to_damage, expected_damage);
}

#[test]
fn manual_challenge_replaces_wakes_turns_hostile_and_round_trips() {
    let mut game = duelist();
    target(&mut game, "test.a", 2);
    target(&mut game, "test.b", 3);
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.entities[0]
        .statuses
        .push(monster_combat::melee_status(STATUS_SLEEP, 100, "test.sleep").status);
    assert!(Game::from_save(game.to_save()).is_ok());
    let tick = game.world_tick;
    let update = dispatch_next(&mut game, mark("test.a"));
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.a"));
    assert!(game.world_tick > tick);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "duelist.challenge-issued")
    );
    assert!(!game.entities[0].friendly);
    assert_eq!(game.entities[0].controller_id, None);
    assert!(
        !game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLEEP)
    );
    let before = game.to_save();
    assert!(matches!(
        game.dispatch(command(
            game.last_command_seq + 1,
            game.revision,
            mark("test.a")
        )),
        Err(CoreError::DuelistChallengeUnavailable)
    ));
    assert_eq!(game.to_save(), before);
    let mut restored = Game::from_save(before).unwrap();
    assert_eq!(
        dispatch_next(&mut restored, mark("test.b")),
        dispatch_next(&mut game, mark("test.b"))
    );
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.b"));
    assert_eq!(game.state_hash(), restored.state_hash());
    let tick = game.world_tick;
    let rng = game.rng.clone();
    dispatch_next(&mut game, GameCommand::ClearDuelistChallenge);
    assert_eq!(game.duelist_target_id, None);
    assert_eq!(game.world_tick, tick);
    assert_eq!(game.rng, rng);
}

#[test]
fn temporary_blocks_and_invalid_target_do_not_spend_time_or_clear_the_mark() {
    for status in [
        STATUS_CONFUSION,
        STATUS_FEAR,
        STATUS_ANTI_MAGIC,
        STATUS_BERSERK,
    ] {
        let mut game = duelist();
        target(&mut game, "test.a", 2);
        target(&mut game, "test.b", 3);
        game.duelist_target_id = Some("test.a".to_owned());
        game.player
            .statuses
            .push(monster_combat::melee_status(status, 100, "test.status").status);
        let rng = game.rng.clone();
        let tick = game.world_tick;
        dispatch_next(&mut game, mark("test.b"));
        assert_eq!(game.world_tick, tick, "{status}");
        assert_eq!(game.rng, rng, "{status}");
        assert_eq!(
            game.duelist_target_id.as_deref(),
            Some("test.a"),
            "{status}"
        );
    }
    let mut game = duelist();
    target(&mut game, "test.a", 2);
    game.duelist_target_id = Some("test.a".to_owned());
    let rng = game.rng.clone();
    let tick = game.world_tick;
    dispatch_next(&mut game, mark("test.missing"));
    assert_eq!(game.rng, rng);
    assert_eq!(game.world_tick, tick);
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.a"));
}

#[test]
fn current_mount_requires_dismount_before_becoming_a_hostile_challenge() {
    let mut game = duelist();
    let mut mount = game.generated_actor(
        "test.mount".to_owned(),
        "demo.actor.horse",
        game.player.position,
    );
    mount.controller_id = Some(game.player.id.clone());
    game.entities.push(mount);
    game.riding_actor_id = Some("test.mount".to_owned());
    assert!(Game::from_save(game.to_save()).is_ok());
    let rng = game.rng.clone();
    let tick = game.world_tick;
    dispatch_next(&mut game, mark("test.mount"));
    assert_eq!(game.rng, rng);
    assert_eq!(game.world_tick, tick);
    assert_eq!(game.duelist_target_id, None);
    assert_eq!(game.riding_actor_id.as_deref(), Some("test.mount"));
    dispatch_next(
        &mut game,
        GameCommand::Ride {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, mark("test.mount"));
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.mount"));
    assert_eq!(game.riding_actor_id, None);
    assert_eq!(game.entities[0].controller_id, None);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn teleport_keeps_identity_hidden_projection_omits_position_and_death_clears_it() {
    let mut game = duelist();
    target(&mut game, "test.a", 2);
    game.duelist_target_id = Some("test.a".to_owned());
    let from = game.entities[0].position;
    let mut ability = game
        .content
        .ability("demo.ability.arcane-teleport-away")
        .unwrap()
        .clone();
    if let AbilityEffectDefinition::TeleportAway { power, .. } = &mut ability.effect {
        *power = 40;
    }
    let plan = game
        .ability_target_plan(
            &ability,
            &TargetSelection::Direction {
                direction: Direction::East,
            },
        )
        .unwrap();
    game.resolve_player_ability_effect(
        ability,
        plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_ne!(game.entities[0].position, from);
    assert_eq!(game.duelist_target_id.as_deref(), Some("test.a"));
    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.duelist_target_id.as_deref(), Some("test.a"));
    assert!(!snapshot.entities.iter().any(|entity| entity.id == "test.a"));
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
    game.resolve_actor_death_without_credit(
        0,
        DomainEvent::Waited,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.duelist_target_id, None);
}

#[test]
fn loading_rejects_missing_dead_other_class_and_invalid_equipment_targets() {
    let mut game = duelist();
    target(&mut game, "test.a", 2);
    game.duelist_target_id = Some("test.a".to_owned());
    let mut missing = game.clone();
    missing.duelist_target_id = Some("test.missing".to_owned());
    assert!(Game::from_save(missing.to_save()).is_err());
    let mut dead = game.clone();
    dead.entities[0].hp = 0;
    assert!(Game::from_save(dead.to_save()).is_err());
    let mut other_class = Game::new_with_build(923, "demo.build.warrior").unwrap();
    target(&mut other_class, "test.a", 2);
    other_class.duelist_target_id = Some("test.a".to_owned());
    assert!(Game::from_save(other_class.to_save()).is_err());
    let weapon = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.rapier")
        .unwrap();
    weapon
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::AntiMagic);
    assert!(Game::from_save(game.to_save()).is_err());
}

// SPDX-License-Identifier: MPL-2.0
use super::{support::give_inventory_item, *};
use crate::game::inventory::ItemIdentificationRequest;
use rfb_content::{ActorDamageType, ActorResistanceLevel};
use rfb_protocol::{CharacterTraitDetailsDto, EquipmentPassiveDto, TraitAttackScopeDto};

fn game() -> Game {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    game.items.clear();
    game
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
    game.identify_item_instance(id, ItemIdentificationRequest::new(true));
}

fn status(id: &str) -> StatusInstance {
    StatusInstance {
        kind_id: id.to_owned(),
        intensity: 1,
        remaining_ticks: 50,
        source_id: None,
        granted_modifiers: Default::default(),
        granted_equipment_bonuses: Default::default(),
        granted_resistances: Default::default(),
        granted_status_immunities: Default::default(),
        granted_brands: Default::default(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }
}

fn details(game: &Game) -> CharacterTraitDetailsDto {
    game.character_trait_details(&game.player_derived_stats())
}

#[test]
fn trait_details_auras_share_combat_sources_without_rolling_damage() {
    let mut game = game();
    for id in [
        STATUS_FIRE_AURA,
        STATUS_DEMON_LORD_TRANSFORMATION,
        STATUS_ULTIMATE_RESISTANCE,
        STATUS_HOLY_AURA,
        STATUS_REGENERATION,
        STATUS_HOLD_LIFE,
    ] {
        game.player.statuses.push(status(id));
    }
    game.player
        .statuses
        .sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
    assert_eq!(
        game.player_elemental_contact_aura_sources(DamageType::Fire)
            .len(),
        2,
        "fire aura and demon lord share one timed contribution"
    );
    let saved = game.to_save();
    let data = details(&game);
    assert_eq!(data.auras.len(), 4);
    let fire = data
        .auras
        .iter()
        .find(|aura| aura.damage_type == DamageTypeDto::Fire)
        .unwrap();
    assert_eq!(fire.source_ids.len(), 3);
    assert!(!fire.evil_only);
    assert!(
        data.auras
            .iter()
            .find(|aura| aura.damage_type == DamageTypeDto::Mana)
            .unwrap()
            .evil_only
    );
    assert_eq!(game.to_save(), saved, "projection must not consume RNG");
    let restored = Game::from_save(saved).expect("timed aura and passive sources should restore");
    assert_eq!(restored.snapshot(), game.snapshot());
    game.player.statuses.clear();
    assert!(details(&game).auras.is_empty());
}

#[test]
fn trait_details_curses_do_not_infer_effects_from_severity_or_reveal_unknown_affixes() {
    let mut game = game();
    equip(&mut game, "test.weapon", "demo.item.short-sword", "weapon");
    game.items[0].curse = Some(ItemCurseSeverityDto::Heavy);
    assert!(details(&game).negatives[0].effects.is_empty());
    game.items[0].rolled_affixes.push(RolledAffixState {
        affix_id: "test.negative".to_owned(),
        curse_effects: BTreeSet::from([
            ItemCurseEffectDto::Aggravate,
            ItemCurseEffectDto::Teleport,
            ItemCurseEffectDto::DrainExperience,
        ]),
        ..Default::default()
    });
    game.item_property_knowledge.remove("test.weapon");
    assert!(details(&game).negatives.is_empty());
    game.identify_item_instance("test.weapon", ItemIdentificationRequest::new(true));
    let data = details(&game);
    assert_eq!(data.negatives[0].curse, Some(ItemCurseSeverityDto::Heavy));
    assert_eq!(
        data.negatives[0].effects.len(),
        3,
        "every identified curse with a runtime consumer is projected"
    );
    assert!(
        data.negatives[0]
            .effects
            .iter()
            .all(|effect| effect.active == Some(true))
    );
    let mut form = status(STATUS_PLAYER_POLYMORPH);
    form.granted_race_id = Some("rfb-legacy.race.shadow-fairy".to_owned());
    game.player.statuses.push(form);
    assert!(
        details(&game).negatives[0]
            .effects
            .iter()
            .find(|effect| effect.effect == ItemCurseEffectDto::Aggravate)
            .unwrap()
            .as_stealth_penalty
    );
    let tool_slot = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "tool")
        .unwrap()
        .id
        .clone();
    game.items[0].location = ItemLocation::Equipped { slot_id: tool_slot };
    game.item_property_knowledge.remove("test.weapon");
    let unknown_tool = details(&game);
    assert!(unknown_tool.negatives.is_empty());
    assert!(
        unknown_tool
            .stats
            .iter()
            .find(|row| row.id == "stealth")
            .unwrap()
            .value
            .is_none()
    );
    game.identify_item_instance("test.weapon", ItemIdentificationRequest::new(true));
    assert_eq!(
        details(&game)
            .stats
            .iter()
            .find(|row| row.id == "stealth")
            .unwrap()
            .value,
        Some(game.player_derived_stats().stealth_skill.value)
    );
    game.items[0].curse = None;
    assert!(
        details(&game).negatives[0]
            .effects
            .iter()
            .all(|effect| effect.active == Some(false))
    );
    game.items[0].location = ItemLocation::Inventory;
    assert!(details(&game).negatives.is_empty());
}

#[test]
fn trait_details_attack_counts_include_each_equipped_weapon_and_the_launcher() {
    let mut game = game();
    equip(&mut game, "test.first", "demo.item.short-sword", "weapon");
    let mut slot = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "weapon")
        .unwrap()
        .clone();
    slot.id = "test.second-hand".to_owned();
    game.body_slots.push(slot);
    equip(&mut game, "test.second", "demo.item.short-sword", "weapon");
    game.items[1].location = ItemLocation::Equipped {
        slot_id: "test.second-hand".to_owned(),
    };
    game.items[1]
        .intrinsic_properties
        .equipment_bonuses
        .melee_attacks = 1;
    equip(
        &mut game,
        "test.launcher",
        "demo.item.short-bow",
        "launcher",
    );
    game.items[2]
        .intrinsic_properties
        .equipment_bonuses
        .base_shot_delta_percent = 50;
    give_inventory_item(&mut game, "test.ammo", "demo.item.arrow");
    let data = details(&game);
    let stats = game.player_derived_stats();
    assert_eq!(
        data.active_weapon_id,
        game.player_melee_profile(&stats).source_item_id
    );
    assert_eq!(data.active_weapon_id.as_deref(), Some("test.first"));
    assert_eq!(data.active_weapon_ids, ["test.first", "test.second"]);
    assert_eq!(data.active_launcher_id.as_deref(), Some("test.launcher"));
    assert_eq!(
        data.stats
            .iter()
            .find(|row| row.id == "melee-attacks")
            .unwrap()
            .value,
        Some(
            game.player_melee_profiles(&stats)
                .iter()
                .map(|profile| i32::from(profile.attacks))
                .sum()
        )
    );
    let projectile = game.player_projectile_profile().unwrap();
    assert_eq!(
        data.stats
            .iter()
            .find(|row| row.id == "ranged-base-shot")
            .unwrap()
            .value,
        Some(projectile.base_shot)
    );
    assert_eq!(
        data.stats
            .iter()
            .find(|row| row.id == "ranged-energy")
            .unwrap()
            .value,
        Some(projectile.energy_cost)
    );
    game.item_property_knowledge.remove("test.second");
    assert!(
        details(&game)
            .stats
            .iter()
            .filter(|row| ["melee-attacks", "ranged-base-shot", "ranged-energy"]
                .contains(&row.id.as_str()))
            .all(|row| row.value.is_none())
    );
}

#[test]
fn trait_details_follow_resistance_merge_and_do_not_mutate_state() {
    let mut game = game();
    equip(&mut game, "test.weapon", "demo.item.short-sword", "weapon");
    game.player
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Resistant);
    game.items[0]
        .intrinsic_properties
        .resistances
        .insert(ActorDamageType::Fire, ActorResistanceLevel::Resistant);
    let fire = |data: CharacterTraitDetailsDto| {
        data.resistances
            .into_iter()
            .find(|row| row.damage_type == DamageTypeDto::Fire)
            .unwrap()
    };
    assert_eq!(
        fire(details(&game)).level,
        Some(ResistanceLevelDto::Resistant)
    );
    game.items[0].rolled_affixes.push(RolledAffixState {
        affix_id: "test.conflicting-resistance".to_owned(),
        properties: rfb_content::AffixPropertyBundleDefinition {
            resistances: BTreeMap::from([(
                ActorDamageType::Fire,
                ActorResistanceLevel::Vulnerable,
            )]),
            ..Default::default()
        },
        ..Default::default()
    });
    game.identify_item_instance("test.weapon", ItemIdentificationRequest::new(true));
    let data = details(&game);
    let equipment = data
        .sources
        .iter()
        .find(|source| source.source_id == "test.weapon")
        .unwrap();
    assert_eq!(
        equipment
            .resistances
            .iter()
            .filter(|entry| entry.damage_type == DamageTypeDto::Fire)
            .count(),
        2
    );
    assert_eq!(fire(data).level, Some(ResistanceLevelDto::Normal));
    let mut protection = status("test.protection");
    protection
        .granted_resistances
        .insert(DamageType::Fire, ResistanceLevel::Strong);
    game.player.statuses.push(protection);
    assert_eq!(fire(details(&game)).reduction_percent, Some(65));
    game.player.statuses[0]
        .granted_resistances
        .insert(DamageType::Fire, ResistanceLevel::Immune);
    let saved = game.to_save();
    let hash = game.state_hash();
    let data = game.snapshot().player.trait_details;
    for row in &data.resistances {
        let level = game
            .effective_player_resistances()
            .level(row.damage_type.into());
        assert_eq!(row.level, Some(level.into()));
        assert_eq!(row.reduction_percent, Some(level.reduction_percent()));
    }
    assert_eq!(fire(data).reduction_percent, Some(100));
    assert_eq!(game.to_save(), saved);
    assert_eq!(game.state_hash(), hash);
}

#[test]
fn trait_details_withhold_unknown_totals_and_reveal_only_known_properties() {
    let mut game = game();
    equip(&mut game, "test.weapon", "demo.item.short-sword", "weapon");
    game.items[0].rolled_affixes.push(RolledAffixState {
        affix_id: "test.hidden-traits".to_owned(),
        properties: rfb_content::AffixPropertyBundleDefinition {
            resistances: BTreeMap::from([(ActorDamageType::Time, ActorResistanceLevel::Immune)]),
            passives: BTreeSet::from([EquipmentPassive::HoldLife, EquipmentPassive::Telepathy]),
            brands: BTreeSet::from([WeaponBrand::Cold]),
            modifiers: StatModifiers {
                speed: 7,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });
    game.item_property_knowledge.remove("test.weapon");
    game.player.statuses.push(status(STATUS_HOLD_LIFE));
    let data = details(&game);
    assert!(!data.equipment_complete);
    assert_eq!(data.resistances.len(), 36);
    assert!(
        data.resistances
            .iter()
            .all(|row| row.level.is_none() && row.reduction_percent.is_none())
    );
    assert!(
        data.sources
            .iter()
            .all(|source| !source.passives.contains(&EquipmentPassiveDto::Telepathy))
    );
    assert!(
        data.sources
            .iter()
            .flat_map(|source| &source.resistances)
            .all(|entry| entry.damage_type != DamageTypeDto::Time)
    );
    assert!(data.attacks.is_empty());
    assert!(data.status_immunities.is_none());
    let hold = data
        .passives
        .iter()
        .find(|row| row.passive == EquipmentPassiveDto::HoldLife)
        .unwrap();
    assert_eq!((hold.active, hold.source_count), (Some(true), None));
    let speed = data.stats.iter().find(|row| row.id == "speed").unwrap();
    assert!(speed.value.is_none() && speed.sources.is_empty());
    game.identify_item_instance("test.weapon", ItemIdentificationRequest::new(false));
    let partial = details(&game);
    assert!(!partial.equipment_complete);
    assert!(partial.resistances.iter().all(|row| row.level.is_none()));
    assert!(
        partial.attacks.is_empty(),
        "appraisal must not disclose the hidden brand"
    );
    assert!(
        partial
            .sources
            .iter()
            .all(|source| !source.passives.contains(&EquipmentPassiveDto::Telepathy))
    );
    game.identify_item_instance("test.weapon", ItemIdentificationRequest::new(true));
    let data = details(&game);
    assert!(data.equipment_complete);
    assert_eq!(data.resistances.len(), 36);
    assert_eq!(
        data.passives
            .iter()
            .find(|row| row.passive == EquipmentPassiveDto::HoldLife)
            .unwrap()
            .source_count,
        Some(2)
    );
    assert_eq!(
        data.stats
            .iter()
            .find(|row| row.id == "speed")
            .unwrap()
            .value,
        Some(i32::from(derived_speed(&game.player_derived_stats().speed)))
    );
    assert_eq!(data.attacks.len(), 1);
    let slot_id = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "tool")
        .unwrap()
        .id
        .clone();
    game.items[0].location = ItemLocation::Equipped { slot_id };
    game.item_property_knowledge.remove("test.weapon");
    let data = details(&game);
    assert!(
        data.equipment_complete,
        "tool magic does not participate in these global rules"
    );
    assert!(
        !data
            .sources
            .iter()
            .any(|source| source.source_id == "test.weapon")
    );
    assert!(data.attacks.is_empty());
}

#[test]
fn trait_details_count_only_rule_counted_abilities_and_preserve_units() {
    let mut game = game();
    for (id, slot) in [("test.first", "weapon"), ("test.second", "body")] {
        equip(&mut game, id, "demo.item.short-sword", slot);
        game.items
            .last_mut()
            .unwrap()
            .intrinsic_properties
            .passives
            .extend([
                EquipmentPassive::HoldLife,
                EquipmentPassive::SeeInvisible,
                EquipmentPassive::Levitation,
            ]);
    }
    for id in [
        STATUS_HOLD_LIFE,
        STATUS_SEE_INVISIBLE,
        STATUS_SIGHT,
        STATUS_ULTIMATE_RESISTANCE,
        STATUS_REGENERATION,
    ] {
        game.player.statuses.push(status(id));
    }
    game.player.statuses[0].grants_wall_passage = true;
    let data = details(&game);
    for passive in [
        EquipmentPassiveDto::HoldLife,
        EquipmentPassiveDto::SeeInvisible,
    ] {
        let row = data
            .passives
            .iter()
            .find(|row| row.passive == passive)
            .unwrap();
        assert_eq!((row.active, row.source_count), (Some(true), Some(3)));
    }
    assert!(
        data.passives
            .iter()
            .filter(|row| !matches!(
                row.passive,
                EquipmentPassiveDto::HoldLife | EquipmentPassiveDto::SeeInvisible
            ))
            .all(|row| row.source_count.is_none())
    );
    assert!(data.passes_walls && data.sources.iter().any(|row| row.passes_walls));
    for passive in [
        EquipmentPassiveDto::SustainStrength,
        EquipmentPassiveDto::SustainIntelligence,
        EquipmentPassiveDto::SustainWisdom,
        EquipmentPassiveDto::SustainDexterity,
        EquipmentPassiveDto::SustainConstitution,
        EquipmentPassiveDto::SustainCharisma,
        EquipmentPassiveDto::Telepathy,
        EquipmentPassiveDto::Levitation,
    ] {
        let row = data
            .passives
            .iter()
            .find(|row| row.passive == passive)
            .unwrap();
        assert_eq!((row.active, row.source_count), (Some(true), None));
        assert!(
            data.sources
                .iter()
                .any(|source| source.passives.contains(&passive))
        );
    }
    assert_eq!(data.reflects_bolts, Some(true));
    let regen = data
        .stats
        .iter()
        .find(|row| row.id == "natural-regeneration")
        .unwrap();
    assert_eq!(
        regen.value,
        Some(game.player_regeneration_rate_percent() as i32)
    );
    assert_eq!(
        regen
            .sources
            .iter()
            .filter(|row| row.source_id == "timed-regeneration")
            .count(),
        1
    );
    assert_eq!(
        data.stats
            .iter()
            .find(|row| row.id == "equipment-life")
            .unwrap()
            .value,
        Some(100)
    );
}

#[test]
fn trait_details_action_protection_and_targeted_senses_follow_known_current_sources() {
    let mut game = game();
    equip(&mut game, "test.weapon", "demo.item.short-sword", "weapon");
    let senses = BTreeSet::from([
        EquipmentPassive::EspAnimal,
        EquipmentPassive::EspUndead,
        EquipmentPassive::EspDemon,
        EquipmentPassive::EspOrc,
        EquipmentPassive::EspTroll,
        EquipmentPassive::EspGiant,
        EquipmentPassive::EspDragon,
        EquipmentPassive::EspHuman,
        EquipmentPassive::EspGood,
        EquipmentPassive::EspEvil,
        EquipmentPassive::EspLiving,
        EquipmentPassive::EspNonliving,
    ]);
    game.items[0].rolled_affixes.push(RolledAffixState {
        affix_id: "test.action-and-senses".to_owned(),
        properties: rfb_content::AffixPropertyBundleDefinition {
            status_immunities: vec![STATUS_PARALYSIS.to_owned()],
            passives: senses.clone(),
            ..Default::default()
        },
        ..Default::default()
    });
    game.item_property_knowledge.remove("test.weapon");
    let unknown = details(&game);
    assert!(unknown.status_immunities.is_none());
    assert!(!unknown.sources.iter().any(|source| {
        source
            .status_immunities
            .iter()
            .any(|id| id == STATUS_PARALYSIS)
    }));
    for passive in &senses {
        assert_eq!(
            unknown
                .passives
                .iter()
                .find(|row| row.passive == equipment_passive_dto(*passive))
                .unwrap()
                .active,
            None
        );
    }
    game.identify_item_instance("test.weapon", ItemIdentificationRequest::new(true));
    let known = details(&game);
    assert!(
        known
            .status_immunities
            .as_ref()
            .unwrap()
            .iter()
            .any(|id| id == STATUS_PARALYSIS)
    );
    for passive in &senses {
        let row = known
            .passives
            .iter()
            .find(|row| row.passive == equipment_passive_dto(*passive))
            .unwrap();
        assert_eq!((row.active, row.source_count), (Some(true), None));
        assert!(
            known
                .sources
                .iter()
                .any(|source| source.source_id == "test.weapon"
                    && source.passives.contains(&row.passive))
        );
    }
    game.items[0].location = ItemLocation::Inventory;
    assert_eq!(details(&game).status_immunities, Some(Vec::new()));
    let mut protection = status("test.temporary-protection");
    protection
        .granted_status_immunities
        .insert(STATUS_PARALYSIS.to_owned());
    game.player.statuses.push(protection);
    assert_eq!(
        details(&game).status_immunities,
        Some(vec![STATUS_PARALYSIS.to_owned()])
    );
    game.player.statuses.clear();
    assert_eq!(details(&game).status_immunities, Some(Vec::new()));
}

#[test]
fn trait_details_separate_armed_melee_ammunition_and_own_weapon() {
    let mut game = game();
    equip(&mut game, "test.weapon", "demo.item.short-sword", "weapon");
    game.items[0]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    game.items[0]
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::Vampiric);
    equip(
        &mut game,
        "test.launcher",
        "demo.item.short-bow",
        "launcher",
    );
    game.items[1]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Cold);
    give_inventory_item(&mut game, "test.ammo", "demo.item.arrow");
    game.items[2]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Poison);
    game.identify_item_instance("test.ammo", ItemIdentificationRequest::new(true));
    let data = details(&game);
    let ammo = data
        .attacks
        .iter()
        .find(|row| row.scope == TraitAttackScopeDto::CurrentAmmunition)
        .unwrap();
    assert_eq!(ammo.source_id, "test.ammo");
    assert_eq!(ammo.brands, vec![rfb_protocol::WeaponBrandDto::Poison]);
    assert!(data.attacks.iter().any(
        |row| row.source_id == "test.launcher" && row.scope == TraitAttackScopeDto::ArmedMelee
    ));
    assert!(data.attacks.iter().any(|row| row.source_id == "test.weapon"
        && row.scope == TraitAttackScopeDto::OwnWeapon
        && row.vampiric));
    assert!(
        !data
            .passives
            .iter()
            .any(|row| row.passive == EquipmentPassiveDto::Vampiric)
    );
}

// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(in crate::game) fn device_pval(item: &ItemInstance) -> u16 {
    item.rolled_affixes
        .iter()
        .find_map(|rolled| rolled.device_pval)
        .unwrap_or(0)
}

pub(crate) fn device_difficulty(difficulty: i32, pval: u16) -> i32 {
    (difficulty - i32::from(pval).max(difficulty * i32::from(pval) / 30)).max(1)
}

pub(crate) fn device_capacity(capacity: u32, pval: u16) -> u32 {
    capacity + capacity * u32::from(pval) / 10
}

pub(in crate::game) fn materialize_device(
    content: &ContentCatalog,
    rng: &mut RfbRng,
    item: &ItemDefinition,
    level: u16,
    exceptional: bool,
    forced_affix: Option<&AffixDefinition>,
) -> Option<EgoMaterialization> {
    if !item.tags.iter().any(|tag| tag == "device") {
        return None;
    }
    // obj_create_device initializes the effect and energy before choosing its ego.
    let (activation, charges) = initial_item_runtime_state(content, rng, &item.id, &[], level);
    let (Some(mut activation), Some(mut charges)) = (activation, charges) else {
        return None;
    };
    let affix = if let Some(affix) = forced_affix {
        affix
    } else if exceptional {
        loop {
            let id = roll_rfb_ego_from_affixes(
                content.affix_definitions(),
                rng,
                level,
                &[RfbEgoTypeDefinition::Device],
            )?;
            let affix = content.affix(id)?;
            if device_can_apply(item, affix.rfb_ego.as_ref()?.source_index) {
                break affix;
            }
        }
    } else {
        return Some(EgoMaterialization::new(
            Vec::new(),
            Vec::new(),
            None,
            None,
            Some(activation),
            Some(charges),
        ));
    };
    let index = affix.rfb_ego.as_ref()?.source_index;
    if !device_can_apply(item, index) {
        return None;
    }
    let pval = matches!(index, 251..=254 | 256).then(|| 1 + rfb_m_bonus(rng, 4, level));
    if index == 251 {
        charges.maximum = device_capacity(charges.maximum, pval.unwrap());
    }
    if index == 253 {
        activation.device_check_difficulty =
            device_difficulty(activation.device_check_difficulty, pval.unwrap());
    }
    let state = RolledAffixState {
        affix_id: affix.id.clone(),
        device_pval: pval,
        ..Default::default()
    };
    Some(EgoMaterialization::new(
        vec![affix.id.clone()],
        state
            .has_instance_state()
            .then_some(state)
            .into_iter()
            .collect(),
        None,
        None,
        Some(activation),
        Some(charges),
    ))
}

fn device_can_apply(item: &ItemDefinition, index: u32) -> bool {
    (250..=256).contains(&index)
        && (index != 250
            || (!item.tags.iter().any(|tag| tag == "rod")
                && !item
                    .elemental_destruction_immunities
                    .contains(&rfb_content::ItemDestructionElement::Acid)))
}

pub(in crate::game) fn roll_quiver_capacity(rng: &mut RfbRng) -> u16 {
    let mut capacity = 60_u16;
    while one_in(rng, 2) {
        capacity = capacity.saturating_add(10);
    }
    capacity
}

pub(super) fn materialize_quiver(
    item: &ItemDefinition,
    affix: &AffixDefinition,
    capacity: u16,
) -> Option<EgoMaterialization> {
    let base = item.rfb_base_kind?;
    let index = affix.rfb_ego.as_ref()?.source_index;
    if base.tval != 46 || base.sval != 0 || !(265..=268).contains(&index) {
        return None;
    }
    let mut state = RolledAffixState {
        affix_id: affix.id.clone(),
        ..Default::default()
    };
    if index == 268 {
        state.weight_tenths_pound = Some(0);
    }
    let capacity = (if index == 265 {
        capacity.saturating_mul(2)
    } else {
        capacity
    })
    .saturating_add(50);
    let (activation, charges) = affix
        .device_generation
        .as_ref()
        .and_then(|generation| generation.activations.first())
        .map(materialize_rfb_activation)
        .unzip();
    Some(EgoMaterialization::new(
        vec![affix.id.clone()],
        state
            .has_instance_state()
            .then_some(state)
            .into_iter()
            .collect(),
        Some(AffixPropertyBundleDefinition {
            ammunition_capacity: Some(capacity),
            ..Default::default()
        }),
        None,
        activation,
        charges,
    ))
}

pub(super) fn light_can_apply(index: u32, tval: u16, sval: u16) -> bool {
    tval == 39
        && match index {
            237 => sval <= 1,
            242 => sval == 2,
            243 => sval > 1,
            235..=243 => true,
            _ => false,
        }
}

pub(super) fn materialize_light(
    rng: &mut RfbRng,
    item: &ItemDefinition,
    affix: &AffixDefinition,
    level: u16,
) -> Option<EgoMaterialization> {
    let base = item.rfb_base_kind?;
    let index = affix.rfb_ego.as_ref()?.source_index;
    if !light_can_apply(index, base.tval, base.sval) {
        return None;
    }
    let mut state = RolledAffixState {
        affix_id: affix.id.clone(),
        ..Default::default()
    };
    let properties = &mut state.properties;
    let mut profile = None;
    match index {
        236 => profile = affix.device_generation.as_ref()?.activations.first(),
        238 => properties.equipment_bonuses.infravision += i32::from(randint1(rng, 3)),
        242 => {
            let stealth = one_in(rng, 7);
            if one_in(rng, 5) {
                let profiles = &affix.device_generation.as_ref()?.activations;
                profile = armor::random_activation(rng, affix, level, true)
                    .and_then(|index| profiles.get(index));
            }
            add_one_high_resistance(rng, properties);
            if randint1(rng, level) > 60 {
                add_one_high_resistance(rng, properties);
            }
            let pval = i32::from(randint1(rng, 2));
            properties.modifiers.speed += pval;
            if stealth {
                properties.equipment_bonuses.stealth_skill += pval;
            }
        }
        243 => {
            if one_in(rng, 2) {
                add_esp_strong(rng, properties);
            } else {
                add_esp_weak(rng, properties, false);
            }
        }
        _ => {}
    }
    let (activation, charges) = profile.map(materialize_rfb_activation).unzip();
    let mut result = EgoMaterialization::new(
        vec![affix.id.clone()],
        state
            .has_instance_state()
            .then_some(state)
            .into_iter()
            .collect(),
        None,
        None,
        activation,
        charges,
    );
    result.extinguish_fuel = index == 240;
    Some(result)
}

pub(in crate::game) fn item_has_ego(
    content: &ContentCatalog,
    item: &ItemInstance,
    index: u32,
) -> bool {
    item.affix_ids.iter().any(|id| {
        content
            .affix(id)
            .and_then(|affix| affix.rfb_ego.as_ref())
            .is_some_and(|ego| ego.source_index == index)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::Game, state::ItemLocation};

    #[test]
    fn blasted_weapon_loses_old_properties_and_dice_and_round_trips() {
        use crate::game::inventory::{CurseEquippedItemRequest, EquippedItemCurseTarget};
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let weapon_id = game.equipped_melee_weapons()[0].id.clone();
        let index = game
            .items
            .iter()
            .position(|item| item.id == weapon_id)
            .unwrap();
        game.items[index].enchantments = ItemEnchantmentsDto {
            to_hit: 10,
            to_damage: 10,
            to_armor: 10,
        };
        game.items[index].intrinsic_properties.modifiers.speed = 5;
        game.items[index]
            .permanent_destruction_immunities
            .insert(rfb_content::ItemDestructionElement::Acid);
        game.debug_item_curses_land = true;
        game.curse_equipped_item(
            CurseEquippedItemRequest::new(EquippedItemCurseTarget::Weapon).blasting(),
        );
        let item = &game.items[index];
        assert!(item_has_ego(&game.content, item, 260));
        assert_eq!(item.intrinsic_properties, Default::default());
        assert!(item.permanent_destruction_immunities.is_empty());
        assert!((-10..=-2).contains(&item.enchantments.to_hit));
        assert!((-10..=-2).contains(&item.enchantments.to_damage));
        assert_eq!(item.enchantments.to_armor, 0);
        assert_eq!(game.item_melee_profile(item).unwrap().damage.dice, 0);
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == weapon_id)
                .unwrap(),
            &game.items[index]
        );
        let mut item = item.clone();
        item.location = ItemLocation::Inventory;
        let dto = crate::save::inventory_to_save(std::slice::from_ref(&item)).remove(0);
        assert_eq!(
            crate::save::inventory_item_from_dto(dto, &game.content).unwrap(),
            item
        );
    }

    #[test]
    fn blasted_artifact_armor_loses_identity_and_ac_but_retains_its_activation() {
        use crate::game::inventory::{CurseEquippedItemRequest, EquippedItemCurseTarget};
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        for item in &mut game.items {
            item.location = ItemLocation::Inventory;
        }
        game.debug_add_generated_inventory_item(
            "test.artifact",
            "demo.item.set-of-gauntlets-paurnimmen",
            50,
        )
        .unwrap();
        let item = game.items.last_mut().unwrap();
        item.location = ItemLocation::Equipped {
            slot_id: game
                .body_slots
                .iter()
                .find(|slot| slot.slot_type == "gloves")
                .unwrap()
                .id
                .clone(),
        };
        let activation = item.activation.clone();
        assert!(activation.is_some());
        game.debug_item_curses_land = true;
        game.curse_equipped_item(
            CurseEquippedItemRequest::new(EquippedItemCurseTarget::Armor).blasting(),
        );
        let item = game.items.last().unwrap();
        assert_eq!(item.kind_id, "demo.item.set-of-gauntlets");
        assert!(item_has_ego(&game.content, item, 260));
        let base = game.content.item(&item.kind_id).unwrap();
        assert_eq!(
            base.modifiers.defense + item.rolled_affixes[0].properties.modifiers.defense,
            0
        );
        assert!((-10..=-2).contains(&item.enchantments.to_armor));
        assert_eq!(item.activation, activation);
        assert!(
            crate::game::item_device_generation(
                &game.content,
                &item.kind_id,
                &item.affix_ids,
                item.activation
                    .as_ref()
                    .map(|activation| activation.profile_id.as_str())
            )
            .is_some()
        );
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.items.last().unwrap(), game.items.last().unwrap());
    }

    #[test]
    fn device_egos_keep_the_initialized_effect_and_round_trip_pval_capacity_and_difficulty() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut seen = BTreeSet::new();
        for kind in [
            "demo.item.identify-staff",
            "demo.item.magic-missile-wand",
            "demo.item.recall-rod",
        ] {
            let definition = game.content.item(kind).unwrap();
            for seed in 1..=500 {
                let result = materialize_device(
                    &game.content,
                    &mut RfbRng::seeded(seed),
                    definition,
                    100,
                    true,
                    None,
                )
                .unwrap();
                let index = game
                    .content
                    .affix(&result.affix_ids[0])
                    .unwrap()
                    .rfb_ego
                    .as_ref()
                    .unwrap()
                    .source_index;
                seen.insert(index);
                if kind.ends_with("rod") {
                    assert_ne!(index, 250);
                }
                let mut item = game.items[0].clone();
                item.kind_id = kind.to_owned();
                item.quantity = 1;
                item.quality = rfb_protocol::ItemQualityDto::Exceptional;
                item.location = ItemLocation::Inventory;
                item.fuel = None;
                result.apply_to(&mut item);
                let (base_activation, base_charges) = initial_item_runtime_state(
                    &game.content,
                    &mut RfbRng::seeded(seed),
                    kind,
                    &[],
                    100,
                );
                let base_activation = base_activation.unwrap();
                assert_eq!(
                    item.activation.as_ref().unwrap().profile_id,
                    base_activation.profile_id
                );
                assert_eq!(item.charges.unwrap().current, base_charges.unwrap().current);
                if index == 251 {
                    assert!(item.charges.unwrap().maximum > base_charges.unwrap().maximum);
                }
                if index == 253 {
                    assert!(
                        item.activation.as_ref().unwrap().device_check_difficulty
                            <= base_activation.device_check_difficulty
                    );
                }
                let dto = crate::save::inventory_to_save(std::slice::from_ref(&item)).remove(0);
                assert_eq!(
                    crate::save::inventory_item_from_dto(dto, &game.content).unwrap(),
                    item
                );
            }
        }
        assert_eq!(seen, (250..=256).collect());
    }

    #[test]
    fn regenerating_device_recovers_its_own_energy_faster() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let item = game.content.item("demo.item.magic-missile-wand").unwrap();
        let affix = game
            .content
            .affix("rfb-legacy.affix.regeneration-device")
            .unwrap();
        let result =
            materialize_device(&game.content, &mut game.rng, item, 100, true, Some(affix)).unwrap();
        game.debug_add_generated_inventory_item("test.device", "demo.item.magic-missile-wand", 100)
            .unwrap();
        let index = game.items.len() - 1;
        result.apply_to(&mut game.items[index]);
        game.items[index].charges.as_mut().unwrap().current = 0;
        let expected = game.items[index].charges.unwrap().maximum
            * 10
            * u32::from(1 + device_pval(&game.items[index]));
        game.world_tick = 10;
        game.process_inventory_device_recovery(&mut Vec::new());
        assert_eq!(game.items[index].charges.unwrap().current, expected / 1000);
        assert_eq!(
            game.items[index].device_recovery_progress,
            (expected % 1000) as u16
        );
    }

    #[test]
    fn quiver_egos_preserve_rolled_capacity_and_phase_removes_only_quivered_weight() {
        let mut game = Game::new_with_build(7, "demo.build.archer").unwrap();
        let quiver_index = game
            .items
            .iter()
            .position(|item| item.kind_id == "demo.item.quiver")
            .unwrap();
        let arrow_index = game
            .items
            .iter()
            .position(|item| item.kind_id == "demo.item.arrow")
            .unwrap();
        game.items[arrow_index].quantity = 60;
        let before_weight = game.carried_weight_tenths_pound();
        let arrow_weight = game.item_instance_weight(&game.items[arrow_index]) as u32 * 60;
        let definition = game.content.item("demo.item.quiver").unwrap().clone();
        let mut seen = BTreeSet::new();
        for seed in 1..=200 {
            let mut rng = RfbRng::seeded(seed);
            let capacity = roll_quiver_capacity(&mut rng);
            let intrinsic = AffixPropertyBundleDefinition {
                ammunition_capacity: Some(capacity),
                ..Default::default()
            };
            let result = roll_and_materialize_rfb_ego_from_affixes_with_rng(
                rfb_protocol::ItemEnchantmentsDto::default(),
                &mut rng,
                &definition,
                game.content.affix_definitions(),
                100,
                Some(&intrinsic),
            )
            .unwrap();
            let index = game
                .content
                .affix(&result.affix_ids[0])
                .unwrap()
                .rfb_ego
                .as_ref()
                .unwrap()
                .source_index;
            seen.insert(index);
            assert_eq!(
                result
                    .intrinsic_properties
                    .as_ref()
                    .unwrap()
                    .ammunition_capacity,
                Some(if index == 265 {
                    capacity * 2 + 50
                } else {
                    capacity + 50
                })
            );
            result.apply_to(&mut game.items[quiver_index]);
            let mut item = game.items[quiver_index].clone();
            item.location = ItemLocation::Inventory;
            let dto = crate::save::inventory_to_save(std::slice::from_ref(&item)).remove(0);
            assert_eq!(
                crate::save::inventory_item_from_dto(dto, &game.content).unwrap(),
                item
            );
            if index == 268 {
                assert_eq!(
                    game.carried_weight_tenths_pound(),
                    before_weight - arrow_weight - definition.weight_tenths_pound as u32
                );
                game.items[arrow_index].quantity = 1000;
                assert!(game.carried_weight_tenths_pound() > before_weight);
                game.items[arrow_index].quantity = 60;
            }
        }
        assert_eq!(seen, (265..=268).collect());
    }

    #[test]
    fn endless_quiver_activation_refills_at_most_fifty_and_spends_its_charge() {
        let mut game = Game::new_with_build(7, "demo.build.archer").unwrap();
        let index = game
            .items
            .iter()
            .position(|item| item.kind_id == "demo.item.quiver")
            .unwrap();
        let quiver_id = game.items[index].id.clone();
        let result = materialize_ego_with_rng(
            &game.content,
            &mut game.rng,
            "demo.item.quiver",
            vec!["rfb-legacy.affix.endless-quiver".to_owned()],
            |_| 100,
            100,
        );
        result.apply_to(&mut game.items[index]);
        game.items.retain(|item| {
            game.content
                .item(&item.kind_id)
                .unwrap()
                .ammunition_profile
                .is_none()
        });
        let mut events = Vec::new();
        for _ in 0..100 {
            game.use_inventory_item(
                &quiver_id,
                None,
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            if game
                .items
                .iter()
                .find(|item| item.id == quiver_id)
                .unwrap()
                .charges
                .as_ref()
                .unwrap()
                .current
                == 0
            {
                break;
            }
        }
        let quiver = game.items.iter().find(|item| item.id == quiver_id).unwrap();
        assert_eq!(quiver.charges.as_ref().unwrap().current, 0);
        let ammo = game
            .items
            .iter()
            .find(|item| item.origin_kind == Some(rfb_protocol::ItemOriginKindDto::EndlessQuiver))
            .unwrap();
        assert_eq!(ammo.kind_id, "demo.item.arrow");
        assert_eq!(ammo.quantity, 50);
        assert!(ammo.affix_ids.is_empty());
        assert_eq!(ammo.enchantments, ItemEnchantmentsDto::default());
        game.refill_quiver("demo.item.quiver", None, &mut events)
            .unwrap();
        game.refill_quiver("demo.item.quiver", None, &mut events)
            .unwrap();
        let count: u32 = game
            .items
            .iter()
            .filter(|item| item.kind_id == "demo.item.arrow")
            .map(|item| item.quantity)
            .sum();
        assert_eq!(count, 110);
        game.refill_quiver("demo.item.quiver", None, &mut events)
            .unwrap();
        assert_eq!(
            game.items
                .iter()
                .filter(|item| item.kind_id == "demo.item.arrow")
                .map(|item| item.quantity)
                .sum::<u32>(),
            110
        );
    }

    #[test]
    fn light_egos_generate_only_on_allowed_bases_with_real_senses_and_activations() {
        let game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        let mut seen = BTreeSet::new();
        let mut valinor_activations = BTreeSet::new();
        for kind in ["wooden-torch", "brass-lantern", "feanorian-lamp"] {
            let item = game.content.item(&format!("demo.item.{kind}")).unwrap();
            for seed in 1..=2500 {
                // Lower levels also exercise W: maximum generation depths.
                let level = (seed % 100 + 1) as u16;
                let result = roll_and_materialize_rfb_ego_from_affixes_with_rng(
                    rfb_protocol::ItemEnchantmentsDto::default(),
                    &mut RfbRng::seeded(seed),
                    item,
                    game.content.affix_definitions(),
                    level,
                    None,
                )
                .unwrap();
                let index = game
                    .content
                    .affix(&result.affix_ids[0])
                    .unwrap()
                    .rfb_ego
                    .as_ref()
                    .unwrap()
                    .source_index;
                seen.insert(index);
                if index == 237 {
                    assert!(item.fuel.is_some());
                }
                if index == 242 {
                    assert_eq!(kind, "feanorian-lamp");
                    assert!((1..=2).contains(&result.rolled_affixes[0].properties.modifiers.speed));
                    if let Some(activation) = &result.activation {
                        valinor_activations.insert(activation.profile_id.clone());
                    }
                }
                if index == 243 {
                    assert!(item.fuel.is_none());
                    assert!(!result.rolled_affixes[0].properties.passives.is_empty());
                }
                if index == 236 {
                    assert!(result.activation.is_some());
                }
                if index == 240 {
                    assert!(result.extinguish_fuel);
                }
            }
        }
        assert_eq!(seen, (235..=243).collect());
        assert_eq!(valinor_activations.len(), 4);
    }

    #[test]
    fn light_duration_extra_light_and_darkness_change_equipped_behavior() {
        let mut game = Game::new_with_build(7, "demo.build.warrior").unwrap();
        game.items.retain(|item| !matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "light"));
        let mut light = game.items[0].clone();
        light.kind_id = "demo.item.brass-lantern".to_owned();
        light.quantity = 1;
        light.affix_ids = vec!["rfb-legacy.affix.duration-light".to_owned()];
        light.rolled_affixes.clear();
        light.intrinsic_properties = Default::default();
        light.fuel = crate::save::initial_item_fuel(&game.content, &light.kind_id);
        light.fuel.as_mut().unwrap().current = 10;
        light.location = ItemLocation::Equipped {
            slot_id: "light".to_owned(),
        };
        game.items.push(light);
        let index = game.items.len() - 1;
        game.world_tick = 10;
        game.process_equipped_light_fuel(&mut Vec::new());
        game.world_tick = 20;
        game.process_equipped_light_fuel(&mut Vec::new());
        assert_eq!(game.items[index].fuel.unwrap().current, 9);
        let radius = game.player_light_radius().unwrap();
        game.items[index].affix_ids = vec!["rfb-legacy.affix.extra-light-light".to_owned()];
        assert_eq!(game.player_light_radius(), Some(radius + 1));
        let result = materialize_ego_with_rng(
            &game.content,
            &mut game.rng,
            &game.items[index].kind_id,
            vec!["rfb-legacy.affix.darkness-light".to_owned()],
            |_| 50,
            50,
        );
        result.apply_to(&mut game.items[index]);
        assert_eq!(game.items[index].fuel.unwrap().current, 0);
        assert_eq!(game.player_light_radius(), None);
    }
}

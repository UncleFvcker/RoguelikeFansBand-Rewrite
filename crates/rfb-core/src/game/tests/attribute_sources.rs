// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::inventory::ItemIdentificationRequest;
use rfb_protocol::{AttributeKindDto, AttributeSourceKindDto as Source};

fn unwell_status() -> StatusInstance {
    StatusInstance {
        kind_id: STATUS_UNWELL.to_owned(),
        intensity: 1,
        remaining_ticks: 50,
        source_id: None,
        granted_modifiers: StatModifiersDto {
            strength: -1,
            ..Default::default()
        },
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_equipment_bonuses: Default::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }
}

#[test]
fn attribute_sources_follow_calculation_order_without_mutating_state() {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    game.items.clear();
    let saved = game.to_save();
    let hash = game.state_hash();
    let expected = game.effective_player_attributes();
    let progress = game.snapshot().player.progress;
    assert_eq!(progress.attribute_sources.len(), 6);
    for row in &progress.attribute_sources {
        let value = match row.attribute {
            AttributeKindDto::Strength => progress.attributes.strength,
            AttributeKindDto::Intelligence => progress.attributes.intelligence,
            AttributeKindDto::Wisdom => progress.attributes.wisdom,
            AttributeKindDto::Dexterity => progress.attributes.dexterity,
            AttributeKindDto::Constitution => progress.attributes.constitution,
            AttributeKindDto::Charisma => progress.attributes.charisma,
        };
        assert_eq!(
            (row.natural, row.effective),
            (value.natural, value.effective)
        );
        assert_eq!((row.minimum, row.maximum), (3, progress.attribute_cap));
        assert_eq!(
            row.sources
                .iter()
                .map(|source| source.kind)
                .collect::<Vec<_>>(),
            [
                Source::Race,
                Source::Class,
                Source::Personality,
                Source::Equipment
            ]
        );
        let mut effective = row.natural;
        for source in &row.sources {
            effective =
                crate::stats::modify_attribute_value(effective, source.modifier, row.maximum);
            assert_eq!(source.effective_after, Some(effective));
            assert!(source.complete);
        }
        assert_eq!(effective, row.effective);
    }
    assert_eq!(game.effective_player_attributes(), expected);
    assert_eq!(game.to_save(), saved);
    assert_eq!(game.state_hash(), hash);
}

#[test]
fn attribute_sources_hide_unknown_equipment_and_reveal_only_after_identification() {
    let mut game = Game::new_with_build(0, "demo.build.warrior").unwrap();
    let item_id = game
        .items
        .iter()
        .find(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .unwrap()
        .id
        .clone();
    game.items.retain(|item| item.id == item_id);
    let item = &mut game.items[0];
    item.intrinsic_properties.modifiers.strength = 1;
    item.rolled_affixes.push(crate::state::RolledAffixState {
        affix_id: "test.affix.attribute-source".to_owned(),
        properties: rfb_content::AffixPropertyBundleDefinition {
            modifiers: StatModifiers {
                strength: 4,
                ..StatModifiers::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });
    game.item_property_knowledge.remove(&item_id);
    let kind_id = game.items[0].kind_id.clone();
    game.mark_item_aware(&kind_id);
    let known = game.visible_item_modifiers(&game.items[0]).strength;
    assert_eq!(game.equipment_modifiers().strength, known + 5);
    game.player.statuses.push(unwell_status());
    let hidden = game.snapshot().player.progress.attribute_sources;
    for row in &hidden {
        let equipment = row
            .sources
            .iter()
            .find(|source| source.kind == Source::Equipment)
            .unwrap();
        assert!(!equipment.complete);
        assert_eq!(equipment.effective_after, None);
        assert_eq!(equipment.upper_limit_applied, None);
        if row.attribute == AttributeKindDto::Strength {
            assert_eq!(equipment.modifier, known);
        }
        let temporary = row
            .sources
            .iter()
            .find(|source| source.kind == Source::TemporaryEffect)
            .unwrap();
        assert!(temporary.complete, "public status modifiers remain visible");
        assert_eq!(
            temporary.effective_after, None,
            "later intermediate values must not reveal hidden equipment"
        );
        assert_eq!(temporary.upper_limit_applied, None);
    }
    let effective = game.effective_player_attributes();
    game.identify_item_instance(&item_id, ItemIdentificationRequest::new(true));
    let revealed = game.snapshot().player.progress.attribute_sources;
    assert_eq!(game.effective_player_attributes(), effective);
    for row in &revealed {
        let equipment = row
            .sources
            .iter()
            .find(|source| source.kind == Source::Equipment)
            .unwrap();
        assert!(equipment.complete);
        assert!(equipment.effective_after.is_some());
        assert_eq!(
            row.sources.last().unwrap().effective_after,
            Some(row.effective)
        );
        if row.attribute == AttributeKindDto::Strength {
            assert_eq!(equipment.modifier, known + 5);
        }
    }
    // Tool slots are excluded by the real attribute path, even with hidden bonuses.
    let tool_slot = game
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "tool")
        .unwrap()
        .id
        .clone();
    game.items[0].location = ItemLocation::Equipped { slot_id: tool_slot };
    game.item_property_knowledge.remove(&item_id);
    let tool_rows = game.snapshot().player.progress.attribute_sources;
    for row in tool_rows {
        let equipment = row
            .sources
            .iter()
            .find(|source| source.kind == Source::Equipment)
            .unwrap();
        assert_eq!(equipment.modifier, 0);
        assert!(equipment.complete);
    }
}

#[test]
fn attribute_sources_project_caps_normal_appearance_and_unwell_phases() {
    let mut game = Game::new(0);
    game.items.clear();
    let normal = game
        .content
        .mutations()
        .find(|mutation| mutation.normal_appearance)
        .unwrap()
        .id
        .clone();
    game.progress.active_mutation_ids.insert(normal);
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.silly-voice".to_owned());
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.hyper-str".to_owned());
    game.progress.attributes.strength = CharacterProgress::attribute_cap(false);
    game.progress.attributes.charisma = 3;
    game.progress.level = 25;
    game.player.statuses.push(unwell_status());
    for (ticks, penalty) in [(60, 0), (50, -4), (30, -3), (1, -1)] {
        game.player.statuses[0].remaining_ticks = ticks;
        let rows = game.snapshot().player.progress.attribute_sources;
        for row in &rows {
            let temporary = row
                .sources
                .iter()
                .find(|source| source.kind == Source::TemporaryEffect)
                .unwrap();
            if matches!(
                row.attribute,
                AttributeKindDto::Dexterity | AttributeKindDto::Constitution
            ) {
                assert_eq!(temporary.modifier, penalty);
            }
            assert_eq!(
                row.sources.last().unwrap().effective_after,
                Some(row.effective)
            );
        }
        let strength = &rows[0];
        assert!(
            strength
                .sources
                .iter()
                .any(|source| source.upper_limit_applied == Some(true))
        );
        assert_eq!(
            strength.effective,
            strength.maximum - 10,
            "cap before the temporary penalty, not after summing modifiers"
        );
        let charisma = &rows[5];
        assert_eq!(charisma.normal_appearance_minimum, Some(58));
        assert_eq!(charisma.effective, 58);
        assert!(
            charisma
                .sources
                .iter()
                .any(|source| source.kind == Source::Mutation
                    && source.modifier < 0
                    && source.suppressed)
        );
        assert_eq!(
            charisma.sources.last().unwrap().kind,
            Source::NormalAppearance
        );
    }
}

#[test]
fn attribute_sources_track_damaged_attributes_and_real_status_expiry() {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    game.items.clear();
    assert!(game.resolve_item_drain_attribute(
        "demo.item.frailty-tonic",
        AttributeKind::Strength,
        &mut Vec::new(),
    ));
    let mut unwell = unwell_status();
    unwell.remaining_ticks = 1;
    game.player.statuses.push(unwell);
    let progress = game.snapshot().player.progress;
    assert!(progress.attributes.strength.natural < progress.attributes.strength.maximum_natural);
    let strength = &progress.attribute_sources[0];
    assert_eq!(strength.natural, progress.attributes.strength.natural);
    assert_eq!(strength.effective, progress.attributes.strength.effective);
    assert!(
        strength
            .sources
            .iter()
            .any(|source| source.kind == Source::TemporaryEffect)
    );
    game.process_status_tick(
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        false,
    )
    .unwrap();
    let saved = game.to_save();
    let hash = game.state_hash();
    let progress = game.snapshot().player.progress;
    for row in progress.attribute_sources {
        assert!(
            !row.sources
                .iter()
                .any(|source| source.kind == Source::TemporaryEffect)
        );
        assert_eq!(
            row.sources.last().unwrap().effective_after,
            Some(row.effective)
        );
    }
    assert_eq!(game.to_save(), saved);
    assert_eq!(game.state_hash(), hash);
}

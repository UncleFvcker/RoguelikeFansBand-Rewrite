// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn monster_level_teleport_uses_resistance_save_and_floor_transition() {
    let mut base = Game::new(17);
    clear_monsters(&mut base);
    descend_one_floor(&mut base);
    clear_monsters(&mut base);
    let caster_position = Position {
        x: base.player.position.x.saturating_add(3),
        y: base.player.position.y,
    };
    replace_terrain(&mut base, caster_position, "demo.terrain.floor");
    base.push_generated_actor(
        "test.monster.quasit.1".to_owned(),
        "demo.actor.quasit",
        caster_position,
    );
    let ability = base
        .content
        .ability("rfb-legacy.ability.teleport-level")
        .expect("Quasit level teleport ability")
        .clone();

    let mut immune = base.clone();
    immune
        .player
        .resistances
        .set(DamageType::Nexus, ResistanceLevel::Immune);
    let immune_floor_id = immune.current_floor_id.clone();
    let immune_plan = immune
        .monster_ability_plan(0, ability.clone(), 1)
        .expect("unobserved nexus immunity should not hide the spell");
    let mut immune_events = Vec::new();
    let immune_resolution = immune.resolve_monster_ability_plan(
        0,
        "demo.actor.quasit",
        &immune_plan,
        &mut immune_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(immune.current_floor_id, immune_floor_id);
    assert!(matches!(
        immune_resolution.effects.as_slice(),
        [AbilityEffectResolutionDto::Skipped {
            effect_index: 0,
            reason: AbilityEffectSkipReasonDto::Saved,
        }]
    ));
    assert!(immune_events.is_empty());
    assert_eq!(
        immune.entities[0]
            .observed_player_resistances
            .get(&DamageType::Nexus),
        Some(&ResistanceLevel::Immune)
    );

    let (game, resolution, events) = (0..1_000_u64)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let plan = game
                .monster_ability_plan(0, ability.clone(), 1)
                .expect("level teleport should target the player");
            let from_floor_id = game.current_floor_id.clone();
            let mut events = Vec::new();
            let resolution = game.resolve_monster_ability_plan(
                0,
                "demo.actor.quasit",
                &plan,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            );
            (game.current_floor_id != from_floor_id).then_some((game, resolution, events))
        })
        .expect("a bounded seed should fail both defenses");

    assert!(matches!(
        resolution.effects.as_slice(),
        [AbilityEffectResolutionDto::TeleportLevel {
            effect_index: 0,
            to_floor_id,
            ..
        }] if to_floor_id == &game.current_floor_id
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::SavingThrowChecked {
            source_kind_id,
            succeeded: false,
            ..
        } if source_kind_id == "demo.actor.quasit"
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::FloorTransitioned { .. }))
    );
}

#[test]
fn monster_shriek_excludes_the_caster_and_aggravates_other_monsters() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.player.position = Position { x: 3, y: 3 };
    game.push_generated_actor(
        "test.monster.shrieker.1".to_owned(),
        "demo.actor.shrieker-mushroom-patch",
        Position { x: 5, y: 3 },
    );
    game.push_generated_actor(
        "test.monster.green-mold.1".to_owned(),
        "demo.actor.green-mold",
        Position { x: 6, y: 3 },
    );
    for actor in &mut game.entities {
        actor.statuses.push(StatusInstance {
            kind_id: STATUS_SLEEP.to_owned(),
            intensity: 1,
            remaining_ticks: 25,
            source_id: None,
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
    }
    let ability = game
        .content
        .ability("rfb-legacy.ability.shriek")
        .expect("formal P8 content should contain Shriek")
        .clone();
    let plan = game
        .monster_ability_plan(0, ability, 1)
        .expect("Shriek should always be a viable utility action");
    let mut changed = BTreeSet::new();
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.shrieker-mushroom-patch",
        &plan,
        &mut Vec::new(),
        &mut changed,
        &mut Vec::new(),
    );

    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLEEP)
    );
    assert!(
        !game.entities[1]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLEEP)
    );
    assert!(
        game.entities[1]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_HASTE && status.remaining_ticks == 100)
    );
    assert_eq!(
        resolution.effects,
        vec![AbilityEffectResolutionDto::AggravateMonsters {
            effect_index: 0,
            awakened: 1,
            hastened: 1,
        }]
    );
    assert_eq!(resolution.affected_positions, vec![Position { x: 6, y: 3 }]);
    assert!(!changed.contains(&Position { x: 5, y: 3 }));
    assert!(changed.contains(&Position { x: 6, y: 3 }));
}

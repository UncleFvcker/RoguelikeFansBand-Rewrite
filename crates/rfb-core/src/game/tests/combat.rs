// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn haste_and_slow_modify_scheduler_speed_without_changing_base_speed() {
    let mut haste_payload = Game::new(42).to_save();
    haste_payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 1,
        remaining_ticks: 20,
        source_id: None,
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut haste = Game::from_save(haste_payload).expect("haste setup should load");
    assert_eq!(haste.snapshot().player.speed, 120);
    let haste_update = haste
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("hasted wait should execute");
    assert_eq!(haste_update.world_tick, 5);
    assert_eq!(haste_update.player.speed, 120);
    assert_eq!(haste.to_save().player.base_speed, 110);
    assert_eq!(haste_update.player.statuses[0].remaining_ticks, 15);

    let mut slow_payload = Game::new(42).to_save();
    slow_payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_SLOW.to_owned(),
        intensity: 1,
        remaining_ticks: 40,
        source_id: None,
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut slow = Game::from_save(slow_payload).expect("slow setup should load");
    let slow_update = slow
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("slowed wait should execute");
    assert_eq!(slow_update.world_tick, 20);
    assert_eq!(slow_update.player.speed, 100);
    assert_eq!(slow_update.player.statuses[0].remaining_ticks, 20);
}

#[test]
fn poison_uses_resistance_then_expires_and_round_trips() {
    let mut payload = Game::new(42).to_save();
    payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 2,
        remaining_ticks: 3,
        source_id: Some("demo.actor.ember-mote.1".to_owned()),
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    payload.player.resistances = vec![ResistanceSaveDto {
        damage_type: DamageTypeDto::Poison,
        level: ResistanceLevelDto::Resistant,
    }];
    let mut game = Game::from_save(payload).expect("poison setup should load");
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("poisoned wait should execute");

    assert_eq!(update.player.hp, 7);
    assert!(update.player.statuses.is_empty());
    assert_eq!(update.player.resistances.len(), 1);
    assert_eq!(
        update
            .events
            .iter()
            .filter(|event| event.message_key == "status-player-damage")
            .count(),
        3
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "status-player-expired")
    );
    let restored = Game::from_save(game.to_save()).expect("status save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn bleeding_ticks_as_physical_damage_in_stable_status_order() {
    let mut payload = Game::new(42).to_save();
    payload.player.statuses = vec![
        StatusSaveDto {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 1,
            remaining_ticks: 1,
            source_id: None,
            granted_resistances: Vec::new(),
            granted_brands: Vec::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: Vec::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        },
        StatusSaveDto {
            kind_id: STATUS_BLEEDING.to_owned(),
            intensity: 2,
            remaining_ticks: 2,
            source_id: None,
            granted_resistances: Vec::new(),
            granted_brands: Vec::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: Vec::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        },
    ];
    let mut game = Game::from_save(payload).expect("bleeding setup should load");
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("bleeding wait should execute");

    assert_eq!(update.player.hp, 5);
    assert!(update.player.statuses.is_empty());
    let damage_statuses = update
        .events
        .iter()
        .filter(|event| event.message_key == "status-player-damage")
        .map(|event| event.args["status"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        damage_statuses,
        [STATUS_BLEEDING, STATUS_POISON, STATUS_BLEEDING]
    );
}

#[test]
fn content_driven_fire_melee_uses_the_player_resistance_profile() {
    let (seed, normal_damage) = (0_u64..1_000)
        .find_map(|seed| {
            let mut game = Game::new(42);
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            game.resolve_monster_melee(0, &mut events);
            events.into_iter().find_map(|event| match event {
                DomainEvent::MonsterMeleeHit { damage, .. } if damage.applied >= 2 => {
                    Some((seed, damage.applied))
                }
                _ => None,
            })
        })
        .expect("a deterministic seed should produce a fire hit of at least two damage");

    let mut resistant = Game::new(42);
    resistant.player.resistances.set(
        DamageType::Fire,
        crate::resistance::ResistanceLevel::Resistant,
    );
    resistant.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    resistant.resolve_monster_melee(0, &mut events);
    let resisted_damage = events
        .into_iter()
        .find_map(|event| match event {
            DomainEvent::MonsterMeleeHit { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .expect("the same seed should preserve the hit result");

    assert_eq!(resisted_damage, normal_damage - normal_damage / 2);
    assert_eq!(resistant.player.hp, 10 - resisted_damage);
}

#[test]
fn content_driven_monster_routine_resolves_blows_in_declared_order() {
    let mut game = Game::new(0);
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    let routine = game.snapshot().entities[0].melee_routine.clone();

    assert_eq!(routine.blows.len(), 2);
    assert_eq!(routine.blows[0].method_id, "rfb.blow.echo-bite");
    assert_eq!(routine.blows[1].method_id, "rfb.blow.echo-rake");

    let mut events = Vec::new();
    game.resolve_monster_melee(0, &mut events);
    let projected = project_events(events);

    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].args["method"], "rfb.blow.echo-bite");
    assert_eq!(projected[1].args["method"], "rfb.blow.echo-rake");
}

#[test]
fn lethal_monster_status_removes_the_entity_before_energy_actions() {
    let mut payload = Game::new(42).to_save();
    payload.entities[0].statuses = vec![StatusSaveDto {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some("demo.player.1".to_owned()),
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut game = Game::from_save(payload).expect("monster poison setup should load");
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("wait should process monster poison");

    assert_eq!(update.entities.len(), 1);
    assert_eq!(
        update.entities[0].id,
        "demo.z-entrance-guardian.resonance-descent.1"
    );
    assert_eq!(update.removed_entities, ["demo.monster.ember-mote.1"]);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "status-entity-death")
    );
}

#[test]
fn leader_death_dissolves_pack_before_remaining_members_act() {
    let mut payload = Game::new(42).to_save();
    let leader_id = payload.entities[0].id.clone();
    let pack_id = "test.pack.leader-death".to_owned();
    payload.entities[0].statuses = vec![StatusSaveDto {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some("demo.player.1".to_owned()),
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    payload.entities[0].pack = Some(rfb_protocol::MonsterPackSaveDto {
        id: pack_id.clone(),
        leader_id: leader_id.clone(),
        role: MonsterPackRoleDto::Leader,
        behavior: MonsterPackBehaviorDto::Seek,
    });
    let mut member = payload.entities[0].clone();
    member.id = "test.pack.member".to_owned();
    member.position = Position { x: 8, y: 6 };
    member.statuses.clear();
    member.pack = Some(rfb_protocol::MonsterPackSaveDto {
        id: pack_id,
        leader_id,
        role: MonsterPackRoleDto::Member,
        behavior: MonsterPackBehaviorDto::GuardLeader,
    });
    payload.entities.push(member);

    let mut game = Game::from_save(payload).expect("pack death setup should load");
    game.dispatch(command(1, 0, GameCommand::Wait))
        .expect("leader death should resolve");

    assert_eq!(game.entities.len(), 2);
    let member = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.pack.member")
        .expect("pack member should remain");
    assert!(member.pack.is_none());
    Game::from_save(game.to_save()).expect("dissolved pack should remain saveable");
}

#[test]
fn rfb_style_armor_reduction_uses_the_legacy_linear_cap() {
    assert_eq!(apply_melee_armor_reduction(100, 0), 100);
    assert_eq!(apply_melee_armor_reduction(100, 90), 70);
    assert_eq!(apply_melee_armor_reduction(100, 180), 40);
    assert_eq!(apply_melee_armor_reduction(100, 999), 40);
}

#[test]
fn fixed_seed_exercises_player_miss_and_death_rejection() {
    let mut miss_game = Game::new(0);
    miss_game.rng = RfbRng::seeded(0);
    miss_game.entities[0].position = Position { x: 4, y: 4 };
    miss_game.entities[0].energy_need = STANDARD_ACTION_COST;
    let miss_update = miss_game
        .dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::SouthEast,
            },
        ))
        .expect("fixed-seed player attack should execute");
    assert!(
        miss_update
            .events
            .iter()
            .any(|event| event.message_key == "combat-player-miss")
    );

    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.entities[0].position = Position { x: 4, y: 4 };
    game.entities[0].energy_need = STANDARD_ACTION_COST;
    game.player.hp = 0;
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("adjacent monster turn should execute");
    assert!(update.player.is_dead);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "combat-player-death")
    );
    assert!(matches!(
        game.dispatch(command(2, 1, GameCommand::Wait)),
        Err(CoreError::PlayerDead)
    ));

    let mut full_health_game = Game::new(0);
    full_health_game.entities[0].position = Position { x: 4, y: 4 };
    full_health_game.entities[0].energy_need = STANDARD_ACTION_COST;
    let death_command = (1..100_u32).find(|seq| {
        full_health_game
            .dispatch(command(*seq, *seq - 1, GameCommand::Wait))
            .is_ok_and(|update| update.player.is_dead)
    });
    assert!(death_command.is_some());
}

// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn monster_effect_game(seed: u64, effect: MeleeBlowEffectDefinition) -> Game {
    monster_effect_game_with_method(seed, "rfb.blow.touch", effect)
}

fn monster_effect_game_with_method(
    seed: u64,
    method_id: &str,
    effect: MeleeBlowEffectDefinition,
) -> Game {
    let mut game = game_with_actor_definition(seed, "demo.actor.echo-hound", |actor| {
        actor.attack = 1_000_000;
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
            blows: vec![rfb_content::MeleeBlowDefinition {
                method_id: method_id.to_owned(),
                to_hit: 0,
                self_destructs: false,
                effects: vec![effect],
            }],
        });
    });
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    game
}

#[test]
fn mutation_contact_auras_retaliate_only_against_unresisted_contact_attacks() {
    let harmless = MeleeBlowEffectDefinition::Damage {
        chance_percent: None,
        damage_dice: 0,
        damage_sides: 0,
        damage_type: rfb_content::ActorDamageType::Physical,
        armor_mitigated: true,
    };
    let mut game = monster_effect_game(0, harmless.clone());
    assert!(game.gain_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    assert!(game.gain_mutation("rfb.mutation.elec-aura", &mut Vec::new()));
    game.entities[0].hp = 100;
    game.entities[0].max_hp = 100;
    let hp_before = game.entities[0].hp;
    let mut events = Vec::new();
    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("contact attack should resolve mutation auras");
    let aura_damage = events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::MutationAuraHit { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(aura_damage.len(), 2);
    assert!(aura_damage.iter().all(|damage| (3..=4).contains(damage)));
    assert_eq!(
        game.entities[0].hp,
        hp_before - aura_damage.iter().sum::<i32>()
    );

    let mut resisted = monster_effect_game(0, harmless.clone());
    assert!(resisted.gain_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    resisted.entities[0]
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Resistant);
    let hp_before = resisted.entities[0].hp;
    let mut events = Vec::new();
    resisted
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("fire-resistant contact attack should resolve");
    assert_eq!(resisted.entities[0].hp, hp_before);
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationAuraHit { .. } | DomainEvent::MutationAuraSlew { .. }
    )));

    let mut vulnerable = monster_effect_game(0, harmless.clone());
    assert!(vulnerable.gain_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    vulnerable.entities[0].hp = 100;
    vulnerable.entities[0].max_hp = 100;
    vulnerable.entities[0]
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Vulnerable);
    let mut events = Vec::new();
    vulnerable
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("fire-vulnerable contact attack should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationAuraHit { damage, .. }
            if (3..=4).contains(&damage.applied)
                && damage.resistance == ResistanceLevel::Normal
    )));

    let mut gaze = monster_effect_game_with_method(0, "rfb.blow.gaze", harmless);
    assert!(gaze.gain_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    let hp_before = gaze.entities[0].hp;
    gaze.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("non-contact gaze should resolve");
    assert_eq!(gaze.entities[0].hp, hp_before);
}

#[test]
fn fatal_mutation_aura_uses_the_shared_actor_death_transaction() {
    let mut game = monster_effect_game(
        0,
        MeleeBlowEffectDefinition::Damage {
            chance_percent: None,
            damage_dice: 0,
            damage_sides: 0,
            damage_type: rfb_content::ActorDamageType::Physical,
            armor_mitigated: true,
        },
    );
    assert!(game.gain_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    let removed_id = game.entities[0].id.clone();
    game.entities[0].hp = 1;
    game.entities[0].max_hp = 1;
    let mut events = Vec::new();
    let mut removed = Vec::new();

    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut removed)
        .expect("fatal aura should resolve");

    assert!(!game.entities.iter().any(|entity| entity.id == removed_id));
    assert!(removed.contains(&removed_id));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationAuraSlew { damage, .. } if damage.applied >= 3
    )));
}

#[test]
fn mutation_innate_attacks_follow_source_order_and_use_their_damage_types() {
    let mut base = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.defense = 0;
    });
    base.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    base.entities[0].hp = 10_000;
    base.entities[0].max_hp = 10_000;
    for id in [
        "rfb.mutation.scorpion-tail",
        "rfb.mutation.horns",
        "rfb.mutation.beak",
        "rfb.mutation.trunk",
        "rfb.mutation.tentacles",
    ] {
        base.progress.active_mutation_ids.insert(id.to_owned());
    }
    let attacker = base.player_derived_stats();
    let profiles = base.player_mutation_innate_attack_profiles(&attacker, None);
    assert_eq!(
        profiles
            .iter()
            .map(|profile| profile.source_mutation_id.as_deref().unwrap())
            .collect::<Vec<_>>(),
        [
            "rfb.mutation.scorpion-tail",
            "rfb.mutation.horns",
            "rfb.mutation.beak",
            "rfb.mutation.trunk",
            "rfb.mutation.tentacles",
        ]
    );

    let events = (0..128)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            game.resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .expect("mutation melee should resolve");
            (events
                .iter()
                .filter(|event| matches!(event, DomainEvent::MutationMeleeHit { .. }))
                .count()
                == 5)
                .then_some(events)
        })
        .expect("a deterministic seed should land all five innate attacks");
    let mutation_hits = events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::MutationMeleeHit {
                mutation_id,
                attack_name,
                damage,
                ..
            } => Some((
                mutation_id.as_str(),
                attack_name.as_str(),
                damage.damage_type,
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        mutation_hits,
        [
            ("rfb.mutation.scorpion-tail", "尾巴", DamageType::Poison),
            ("rfb.mutation.horns", "长角", DamageType::Physical),
            ("rfb.mutation.beak", "鸟喙", DamageType::Physical),
            ("rfb.mutation.trunk", "象鼻", DamageType::Physical),
            ("rfb.mutation.tentacles", "触手", DamageType::Physical),
        ]
    );
}

#[test]
fn fatal_mutation_innate_attack_uses_the_shared_actor_death_transaction() {
    let mut base = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.defense = 0;
    });
    base.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    base.entities[0].hp = 1;
    base.entities[0].max_hp = 1;
    base.entities[0]
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);
    assert!(base.gain_mutation("rfb.mutation.scorpion-tail", &mut Vec::new()));
    let removed_id = base.entities[0].id.clone();

    let (game, events, removed) = (0..128)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            let mut removed = Vec::new();
            game.resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut removed)
                .expect("fatal mutation melee should resolve");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::MutationMeleeSlew { .. }))
                .then_some((game, events, removed))
        })
        .expect("a deterministic seed should land the poisonous tail");

    assert!(!game.entities.iter().any(|entity| entity.id == removed_id));
    assert!(removed.contains(&removed_id));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationMeleeSlew {
            mutation_id,
            attack_name,
            damage,
            ..
        } if mutation_id == "rfb.mutation.scorpion-tail"
            && attack_name == "尾巴"
            && damage.damage_type == DamageType::Poison
            && damage.applied >= 3
    )));
}

#[test]
fn innate_critical_roll_uses_original_weight_level_and_quality_bands() {
    let (seed, multiplier) = (0..10_000)
        .find_map(|seed| {
            let mut game = Game::new(seed);
            let multiplier = game.roll_innate_critical_multiplier(200, 50);
            (multiplier > 100).then_some((seed, multiplier))
        })
        .expect("a deterministic seed should produce an innate critical");
    let mut repeated = Game::new(seed);
    assert_eq!(
        repeated.roll_innate_critical_multiplier(200, 50),
        multiplier
    );
    assert!([200, 250, 300, 350, 400].contains(&multiplier));
}

#[test]
fn zero_dice_hurt_hits_without_dealing_damage() {
    let mut game = monster_effect_game(
        0,
        MeleeBlowEffectDefinition::Damage {
            chance_percent: None,
            damage_dice: 0,
            damage_sides: 0,
            damage_type: rfb_content::ActorDamageType::Physical,
            armor_mitigated: true,
        },
    );
    let hp_before = game.player.hp;
    let mut events = Vec::new();

    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("zero-damage HURT should resolve");

    assert_eq!(game.player.hp, hp_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterMeleeHit { damage, .. }
            if damage.requested == 0 && damage.applied == 0
    )));
}

#[test]
fn resource_drain_melee_heals_six_times_the_amount_actually_drained() {
    let mut game = monster_effect_game(
        0,
        MeleeBlowEffectDefinition::DrainResource {
            chance_percent: None,
            amount_dice: 1,
            amount_sides: 1,
        },
    );
    game.resources.insert(
        "test.resource.mana".to_owned(),
        ResourcePool {
            current: 1,
            maximum: 1,
        },
    );
    game.entities[0].hp = 1;
    game.entities[0].max_hp = 20;

    game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("resource-draining melee should resolve");

    assert_eq!(game.resources["test.resource.mana"].current, 0);
    assert_eq!(game.entities[0].hp, 7);
}

#[test]
fn experience_drain_lowers_current_level_but_preserves_character_history() {
    let mut game = monster_effect_game(
        0,
        MeleeBlowEffectDefinition::DrainExperience {
            chance_percent: None,
            amount_dice: 100,
            amount_sides: 1,
        },
    );
    game.apply_player_experience(100, &mut Vec::new());
    let maximum_experience = game.progress.maximum_experience;
    let max_level = game.progress.max_level;
    let mut events = Vec::new();

    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("experience-draining melee should resolve");

    assert_eq!(game.progress.experience, 0);
    assert_eq!(game.progress.maximum_experience, maximum_experience);
    assert_eq!(game.progress.level, 1);
    assert_eq!(game.progress.max_level, max_level);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::ExperienceDrained {
            source_kind_id,
            amount,
            total: 0,
        } if source_kind_id == "demo.actor.echo-hound" && *amount == maximum_experience
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerLevelLost { level: 1, .. }))
    );
}

#[test]
fn poison_contact_aura_triggers_after_a_fatal_player_hit() {
    let base = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.defense = 0;
        actor.contact_auras = vec![rfb_content::ActorContactAuraDefinition {
            damage_type: rfb_content::ActorDamageType::Poison,
            damage_dice: 1,
            damage_sides: 1,
            chance_percent: None,
        }];
    });
    let (game, events) = (0..128)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
            game.entities[0].hp = 1;
            game.entities[0].max_hp = 1;
            let mut events = Vec::new();
            game.resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .expect("contact aura melee should resolve");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::PlayerSlew { .. }))
                .then_some((game, events))
        })
        .expect("a deterministic seed should produce a fatal player hit");

    assert!(
        game.player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_POISON && status.remaining_ticks == 1)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterContactAuraApplied {
            source_kind_id,
            status_kind_id,
            duration: 1,
        } if source_kind_id == "demo.actor.echo-hound" && status_kind_id == STATUS_POISON
    )));
}

#[test]
fn elemental_contact_auras_deal_immediate_resisted_damage() {
    for (actor_damage_type, damage_type) in [
        (rfb_content::ActorDamageType::Acid, DamageType::Acid),
        (rfb_content::ActorDamageType::Fire, DamageType::Fire),
        (rfb_content::ActorDamageType::Cold, DamageType::Cold),
        (
            rfb_content::ActorDamageType::Electricity,
            DamageType::Electricity,
        ),
    ] {
        let mut game = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
            actor.contact_auras = vec![rfb_content::ActorContactAuraDefinition {
                damage_type: actor_damage_type,
                damage_dice: 2,
                damage_sides: 1,
                chance_percent: None,
            }];
        });
        game.player.hp = 10;
        game.player
            .resistances
            .set(damage_type, crate::resistance::ResistanceLevel::Resistant);
        let definition = game
            .content
            .actor("demo.actor.echo-hound")
            .expect("contact aura actor definition")
            .clone();
        let mut events = Vec::new();

        assert!(!game.resolve_monster_contact_auras(&definition, &mut events));

        assert_eq!(game.player.hp, 9);
        assert!(
            !game
                .player
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_POISON)
        );
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::MonsterMeleeHit {
                source_kind_id,
                method_id: None,
                damage,
            } if source_kind_id == "demo.actor.echo-hound"
                && damage.damage_type == damage_type
                && damage.requested == 2
                && damage.applied == 1
        )));
    }
}

#[test]
fn multiple_contact_auras_resolve_in_declaration_order_with_resistance() {
    let mut game = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.contact_auras = vec![
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Poison,
                damage_dice: 2,
                damage_sides: 1,
                chance_percent: None,
            },
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Acid,
                damage_dice: 2,
                damage_sides: 1,
                chance_percent: None,
            },
        ];
    });
    game.player.hp = 10;
    game.player.resistances.set(
        DamageType::Acid,
        crate::resistance::ResistanceLevel::Resistant,
    );
    let definition = game
        .content
        .actor("demo.actor.echo-hound")
        .expect("contact aura actor definition")
        .clone();
    let mut events = Vec::new();

    assert!(!game.resolve_monster_contact_auras(&definition, &mut events));

    assert_eq!(game.player.hp, 9);
    assert!(
        game.player
            .statuses
            .iter()
            .any(|status| { status.kind_id == STATUS_POISON && status.remaining_ticks == 3 })
    );
    assert!(matches!(
        events.as_slice(),
        [
            DomainEvent::MonsterContactAuraApplied { status_kind_id, .. },
            DomainEvent::MonsterMeleeHit { damage, .. }
        ] if status_kind_id == STATUS_POISON
            && damage.damage_type == DamageType::Acid
            && damage.requested == 2
            && damage.applied == 1
    ));
}

#[test]
fn fatal_elemental_contact_aura_stops_player_melee() {
    let mut base = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.defense = 0;
        actor.contact_auras = vec![
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Fire,
                damage_dice: 2,
                damage_sides: 1,
                chance_percent: None,
            },
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Electricity,
                damage_dice: 2,
                damage_sides: 1,
                chance_percent: None,
            },
        ];
    });
    let mut extra_attacks = monster_combat::melee_status(STATUS_HASTE, 20, "test.setup").status;
    extra_attacks.granted_equipment_bonuses.melee_attacks = 2;
    base.player.statuses.push(extra_attacks);
    let (game, events) = (0..128)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            game.player.hp = 1;
            game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
            game.entities[0].hp = 100;
            game.entities[0].max_hp = 100;
            let mut events = Vec::new();
            game.resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .expect("elemental contact aura melee should resolve");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::PlayerDied { .. }))
                .then_some((game, events))
        })
        .expect("a deterministic seed should trigger the elemental contact aura");

    assert_eq!(game.player.hp, -1);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::MonsterMeleeHit { .. }))
            .count(),
        1
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterMeleeHit { damage, .. }
            if damage.damage_type == DamageType::Electricity
    )));
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
                )
            })
            .count(),
        1
    );
}

#[test]
fn disenchant_melee_removes_positive_status_or_reduces_equipment_enchantments() {
    let effect = MeleeBlowEffectDefinition::Disenchant {
        chance_percent: None,
    };
    let mut status_base = monster_effect_game(0, effect.clone());
    status_base
        .player
        .statuses
        .push(monster_combat::melee_status(STATUS_HASTE, 20, "test.setup").status);
    status_base
        .player
        .statuses
        .push(monster_combat::melee_status(STATUS_POISON, 20, "test.setup").status);
    let (status_seed, status_game) = (0..100)
        .find_map(|seed| {
            let mut game = status_base.clone();
            game.rng = RfbRng::seeded(seed);
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .expect("disenchanting melee should resolve");
            (!game
                .player
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_HASTE))
            .then_some((seed, game))
        })
        .expect("a deterministic seed should select the positive-status branch");
    assert!(
        status_game
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_POISON)
    );
    assert_eq!(status_game.player.hp, status_base.player.hp);

    let mut immune = status_base.clone();
    immune.player.resistances.set(
        DamageType::Disenchant,
        crate::resistance::ResistanceLevel::Immune,
    );
    immune.rng = RfbRng::seeded(status_seed);
    immune
        .resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("resisted disenchanting melee should resolve");
    assert!(
        immune
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_HASTE)
    );

    let mut equipment_base = monster_effect_game(0, effect);
    equipment_base.items.clear();
    give_inventory_item(
        &mut equipment_base,
        "test.enchanted-dagger",
        "demo.item.dagger",
    );
    equipment_base.items[0].location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    equipment_base.items[0].enchantments = ItemEnchantmentsDto {
        to_hit: 2,
        to_damage: 3,
        to_armor: 0,
    };
    let equipment_game = (0..100)
        .find_map(|seed| {
            let mut game = equipment_base.clone();
            game.rng = RfbRng::seeded(seed);
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .expect("disenchanting melee should resolve");
            (game.items[0].enchantments != equipment_base.items[0].enchantments).then_some(game)
        })
        .expect("a deterministic seed should select the equipment branch");
    assert_eq!(
        equipment_game.items[0].enchantments,
        ItemEnchantmentsDto {
            to_hit: 1,
            to_damage: 2,
            to_armor: 0,
        }
    );
    assert_eq!(equipment_game.player.hp, equipment_base.player.hp);
}

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
            game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .expect("monster melee should resolve");
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
    resistant
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("monster melee should resolve");
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
    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("monster melee should resolve");
    let projected = project_events(events);

    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].args["method"], "rfb.blow.echo-bite");
    assert_eq!(projected[1].args["method"], "rfb.blow.echo-rake");
}

#[test]
fn explicit_empty_melee_routine_performs_no_attack() {
    let mut game = Game::new(0);
    game.entities[0].kind_id = "demo.actor.culverin".to_owned();
    let hp_before = game.player.hp;
    let draws_before = game.rng.draw_counter;
    let mut events = Vec::new();

    assert!(
        !game
            .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .expect("empty monster melee should resolve")
    );
    assert_eq!(game.player.hp, hp_before);
    assert_eq!(game.rng.draw_counter, draws_before);
    assert!(events.is_empty());
}

#[test]
fn item_theft_splits_a_stack_into_monster_carried_loot_and_blinks() {
    let mut game = monster_effect_game(
        7,
        MeleeBlowEffectDefinition::EatItem {
            chance_percent: None,
        },
    );
    game.items.clear();
    give_inventory_item(&mut game, "test.rations", "demo.item.ration-of-food");
    game.items[0].quantity = 2;
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_PARALYSIS, 10, "test.setup").status);
    let origin = game.entities[0].position;
    let mut events = Vec::new();

    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("item theft should resolve");

    assert_eq!(game.items[0].quantity, 1);
    assert!(matches!(game.items[0].location, ItemLocation::Inventory));
    let stolen = game
        .items
        .iter()
        .find(|item| matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id == &game.entities[0].id))
        .expect("the stolen item should be carried by the thief");
    assert_eq!(stolen.kind_id, "demo.item.ration-of-food");
    assert_eq!(stolen.quantity, 1);
    assert_ne!(game.entities[0].position, origin);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterItemStolen { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterBlinked { .. }))
    );
}

#[test]
fn gold_theft_uses_the_original_amount_and_dexterity_protection() {
    let effect = MeleeBlowEffectDefinition::EatGold {
        chance_percent: None,
    };
    let mut stolen = monster_effect_game(11, effect.clone());
    stolen.gold = 1_000;
    stolen
        .player
        .statuses
        .push(monster_combat::melee_status(STATUS_PARALYSIS, 10, "test.setup").status);
    let mut events = Vec::new();
    stolen
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("gold theft should resolve");
    assert!((875..=899).contains(&stolen.gold));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterGoldStolen {
            amount: 101..=125,
            ..
        }
    )));

    let mut protected = monster_effect_game(11, effect);
    protected.gold = 1_000;
    protected.progress.attributes.dexterity = 238;
    protected.progress.maximum_attributes.dexterity = 238;
    let mut events = Vec::new();
    protected
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("protected gold theft should resolve");
    assert_eq!(protected.gold, 1_000);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterGoldTheftPrevented { .. }))
    );
}

#[test]
fn food_and_light_eating_consume_one_food_and_leave_one_light_fuel() {
    let mut food = monster_effect_game(
        13,
        MeleeBlowEffectDefinition::EatFood {
            chance_percent: None,
        },
    );
    food.items.clear();
    give_inventory_item(&mut food, "test.rations", "demo.item.ration-of-food");
    food.items[0].quantity = 2;
    let mut events = Vec::new();
    food.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("food eating should resolve");
    assert_eq!(food.items[0].quantity, 1);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterFoodEaten { .. }))
    );

    let mut light = monster_effect_game(
        17,
        MeleeBlowEffectDefinition::EatLight {
            chance_percent: None,
        },
    );
    light.items.clear();
    give_inventory_item(&mut light, "test.torch", "demo.item.wooden-torch");
    light.items[0].location = ItemLocation::Equipped {
        slot_id: "light".to_owned(),
    };
    light.items[0]
        .fuel
        .as_mut()
        .expect("torch should carry fuel")
        .current = 300;
    let mut events = Vec::new();
    light
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("light eating should resolve");
    assert_eq!(
        light.items[0]
            .fuel
            .expect("torch should retain fuel")
            .current,
        1
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterLightEaten { amount: 299, .. }))
    );
}

#[test]
fn non_damage_melee_riders_apply_the_existing_player_statuses() {
    let base = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.attack = 1_000_000;
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
            blows: vec![rfb_content::MeleeBlowDefinition {
                method_id: "rfb.blow.status-check".to_owned(),
                to_hit: 0,
                self_destructs: false,
                effects: vec![
                    rfb_content::MeleeBlowEffectDefinition::Blind {
                        chance_percent: None,
                    },
                    rfb_content::MeleeBlowEffectDefinition::Confusion {
                        chance_percent: None,
                        damage_dice: 1,
                        damage_sides: 1,
                    },
                    rfb_content::MeleeBlowEffectDefinition::Paralysis {
                        chance_percent: None,
                    },
                    rfb_content::MeleeBlowEffectDefinition::Slow {
                        chance_percent: None,
                    },
                    rfb_content::MeleeBlowEffectDefinition::Stun {
                        chance_percent: None,
                        duration_dice: 1,
                        duration_sides: 1,
                    },
                    rfb_content::MeleeBlowEffectDefinition::Terrify {
                        chance_percent: None,
                    },
                ],
            }],
        });
    });
    let game = (0..100)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
            game.player.hp = 100;
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .expect("monster melee should resolve");
            (game.player.statuses.len() == 6).then_some(game)
        })
        .expect("a deterministic seed should land the status blow");

    let duration = |kind_id| {
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == kind_id)
            .expect("melee rider should apply its status")
            .remaining_ticks
    };
    assert!((12..=15).contains(&duration(STATUS_BLINDNESS)));
    assert!((11..=30).contains(&duration(STATUS_CONFUSION)));
    assert!((1..=3).contains(&duration(STATUS_PARALYSIS)));
    assert_eq!(duration(STATUS_SLOW), 25);
    assert_eq!(duration(STATUS_STUN), 1);
    assert_eq!(
        duration(STATUS_FEAR),
        game.content
            .actor("demo.actor.echo-hound")
            .expect("test actor definition")
            .level
    );
    assert_eq!(game.player.hp, 99);
}

#[test]
fn self_destructing_blow_skips_single_target_effect_and_explodes_on_death() {
    let base = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.attack = 1_000_000;
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
            blows: vec![rfb_content::MeleeBlowDefinition {
                method_id: "rfb.blow.explode".to_owned(),
                to_hit: 0,
                self_destructs: true,
                effects: vec![rfb_content::MeleeBlowEffectDefinition::Damage {
                    chance_percent: None,
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_type: rfb_content::ActorDamageType::Physical,
                    armor_mitigated: false,
                }],
            }],
        });
    });
    let (game, events, removed) = (0..100)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
            game.entities[0].position = Position {
                x: game.player.position.x + 1,
                y: game.player.position.y,
            };
            let target = MonsterHostileTarget::Player {
                entity_id: game.player.id.clone(),
                kind_id: "demo.actor.player".to_owned(),
                position: game.player.position,
            };
            let mut events = Vec::new();
            let mut removed = Vec::new();
            game.resolve_monster_melee_target(
                0,
                &target,
                &mut events,
                &mut BTreeSet::new(),
                &mut removed,
            )
            .expect("self-destructing melee should resolve");
            (!removed.is_empty()).then_some((game, events, removed))
        })
        .expect("a deterministic seed should land the self-destructing blow");

    assert_eq!(game.player.hp, 9);
    assert_eq!(removed, ["demo.monster.ember-mote.1"]);
    assert!(game.entities.iter().all(|actor| actor.id != removed[0]));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterMeleeHit { .. }))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterDeathExplosionHit { target_kind_id, damage, .. }
            if target_kind_id == &game.player.kind_id && damage.applied == 1
    )));
}

#[test]
fn ordinary_death_uses_the_first_self_destructing_blow_as_a_radius_three_explosion() {
    let mut game = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
            blows: vec![rfb_content::MeleeBlowDefinition {
                method_id: "rfb.blow.explode".to_owned(),
                to_hit: 0,
                self_destructs: true,
                effects: vec![rfb_content::MeleeBlowEffectDefinition::Damage {
                    chance_percent: None,
                    damage_dice: 1,
                    damage_sides: 4,
                    damage_type: rfb_content::ActorDamageType::Fire,
                    armor_mitigated: false,
                }],
            }],
        });
    });
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    game.entities[0].position = Position {
        x: game.player.position.x + 3,
        y: game.player.position.y,
    };
    game.rng = RfbRng::seeded(1);
    let hp_before = game.player.hp;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    game.resolve_actor_death(
        0,
        DomainEvent::Waited,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .expect("ordinary death explosion should resolve");

    assert!(game.player.hp < hp_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterDeathExplosionHit { target_kind_id, .. }
            if target_kind_id == &game.player.kind_id
    )));
    assert!(
        changed
            .iter()
            .any(|position| rfb_distance(game.player.position, *position) == 3)
    );
}

#[test]
fn death_explosion_removes_player_summons_without_death_drops() {
    let mut game = game_with_actor_definition(0, "demo.actor.echo-hound", |actor| {
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
            blows: vec![rfb_content::MeleeBlowDefinition {
                method_id: "rfb.blow.explode".to_owned(),
                to_hit: 0,
                self_destructs: true,
                effects: vec![rfb_content::MeleeBlowEffectDefinition::Damage {
                    chance_percent: None,
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_type: rfb_content::ActorDamageType::Physical,
                    armor_mitigated: false,
                }],
            }],
        });
    });
    clear_monsters(&mut game);
    let source_position = Position { x: 8, y: 4 };
    let summon_position = Position { x: 9, y: 4 };
    game.push_generated_actor(
        "test.exploder".to_owned(),
        "demo.actor.echo-hound",
        source_position,
    );
    add_player_summon(&mut game, "test.summon", summon_position, 10);
    game.entities[1].kind_id = "demo.actor.warrens-keeper".to_owned();
    game.entities[1].hp = 1;
    game.rng = RfbRng::seeded(1);
    let mut events = Vec::new();
    let mut removed = Vec::new();

    game.resolve_actor_death(
        0,
        DomainEvent::Waited,
        &mut events,
        &mut BTreeSet::new(),
        &mut removed,
    )
    .expect("death explosion should remove its summoned target");

    assert!(removed.iter().any(|entity_id| entity_id == "test.summon"));
    assert!(game.entities.iter().all(|actor| actor.id != "test.summon"));
    assert!(game.items.iter().all(|item| {
        !matches!(item.location, ItemLocation::Ground(position) if position == summon_position)
    }));
    assert!(
        game.gold_piles
            .iter()
            .all(|gold| gold.position != summon_position)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterDeathExplosionSlew { target_kind_id, .. }
            if target_kind_id == "demo.actor.warrens-keeper"
    )));
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

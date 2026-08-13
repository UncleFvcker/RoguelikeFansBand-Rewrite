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
    let mut game = game_with_actor_definition(seed, "demo.actor.small-kobold", |actor| {
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
    clear_monsters(&mut game);
    let monster = game.generated_actor(
        "test.monster.melee-effect".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    game.entities.push(monster);
    game
}

fn resolve_p62_polymorph(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_polymorph("demo.actor.lord-of-change", 61, &mut events);
    events
}

fn p62_polymorph_legacy_index(game: &Game) -> Option<u16> {
    game.player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_PLAYER_POLYMORPH)
        .and_then(|status| status.granted_race_id.as_deref())
        .and_then(|race_id| game.content.race(race_id))
        .and_then(|race| race.legacy_index)
}

fn p62_seed_for_legacy_index(legacy_index: u16) -> u64 {
    let base = Game::new_with_build(0, "demo.build.warrior").expect("P62 test build should create");
    (0..100_000)
        .find(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(*seed);
            resolve_p62_polymorph(&mut game);
            p62_polymorph_legacy_index(&game) == Some(legacy_index)
        })
        .unwrap_or_else(|| panic!("a bounded seed should select legacy race {legacy_index}"))
}

#[test]
fn p62_polymorph_immunity_and_successful_save_do_not_draw_a_form_or_duration() {
    let mut immune =
        Game::new_with_build(0, "demo.build.warrior").expect("P62 test build should create");
    let mut android = monster_combat::melee_status("test.status.android", 10, "test.setup").status;
    android.granted_race_id = Some("rfb-legacy.race.android".to_owned());
    immune.player.statuses.push(android);
    immune.rng = RfbRng::seeded(7);
    let rng_before = immune.rng.clone();
    let events = resolve_p62_polymorph(&mut immune);
    assert_eq!(immune.rng, rng_before);
    assert!(events.is_empty());
    assert_eq!(
        immune
            .character_definitions()
            .expect("test build should retain character definitions")
            .1
            .legacy_index,
        Some(36)
    );

    let (seed, expected_rng) = (0..10_000)
        .find_map(|seed| {
            let mut probe = Game::new_with_build(0, "demo.build.warrior").ok()?;
            probe.rng = RfbRng::seeded(seed);
            let saved =
                probe.monster_saving_throw("demo.actor.lord-of-change", 61, &mut Vec::new());
            saved.then_some((seed, probe.rng))
        })
        .expect("a bounded seed should pass the polymorph saving throw");
    let mut saved =
        Game::new_with_build(0, "demo.build.warrior").expect("P62 test build should create");
    saved.rng = RfbRng::seeded(seed);
    let events = resolve_p62_polymorph(&mut saved);
    assert_eq!(saved.rng, expected_rng);
    assert_eq!(p62_polymorph_legacy_index(&saved), None);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::SavingThrowChecked {
            succeeded: true,
            ..
        }
    )));
}

#[test]
fn p62_polymorph_preserves_legacy_branches_rejection_rng_and_temporary_state() {
    for legacy_index in [6, 15, 1_007, 1_008] {
        let seed = p62_seed_for_legacy_index(legacy_index);
        let mut game =
            Game::new_with_build(0, "demo.build.warrior").expect("P62 test build should create");
        game.rng = RfbRng::seeded(seed);
        resolve_p62_polymorph(&mut game);
        let status = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_PLAYER_POLYMORPH)
            .expect("selected fixed branch should apply a temporary form");
        assert!((51..=100).contains(&status.remaining_ticks));
        assert_eq!(p62_polymorph_legacy_index(&game), Some(legacy_index));
    }

    let mut snotling_base =
        Game::new_with_build(0, "demo.build.warrior").expect("P62 test build should create");
    snotling_base
        .build
        .as_mut()
        .expect("test build identity should exist")
        .race_id = "rfb-legacy.race.snotling".to_owned();
    let fallback_seed = (0..100_000)
        .find(|seed| {
            let mut game = snotling_base.clone();
            game.rng = RfbRng::seeded(*seed);
            resolve_p62_polymorph(&mut game);
            p62_polymorph_legacy_index(&game) == Some(15)
        })
        .expect("a Snotling branch-one seed should fall through to Yeek");
    let mut fallback = snotling_base;
    fallback.rng = RfbRng::seeded(fallback_seed);
    resolve_p62_polymorph(&mut fallback);
    assert_eq!(p62_polymorph_legacy_index(&fallback), Some(15));

    let rejection_base =
        Game::new_with_build(0, "demo.build.warrior").expect("P62 test build should create");
    let (rejection_seed, expected_save_draws) = (0..100_000)
        .find_map(|seed| {
            let mut probe = rejection_base.clone();
            probe.rng = RfbRng::seeded(seed);
            if probe.monster_saving_throw("demo.actor.lord-of-change", 61, &mut Vec::new()) {
                return None;
            }
            let save_draws = probe.rng.draw_counter;
            let mut game = rejection_base.clone();
            game.rng = RfbRng::seeded(seed);
            resolve_p62_polymorph(&mut game);
            let selected = p62_polymorph_legacy_index(&game)?;
            let fixed = [6, 15, 1_007, 1_008].contains(&selected);
            (!fixed && game.rng.draw_counter.saturating_sub(save_draws) > 3)
                .then_some((seed, save_draws))
        })
        .expect("a bounded seed should reject at least one random legacy race index");
    let mut rejected =
        Game::new_with_build(0, "demo.build.warrior").expect("P62 test build should create");
    rejected.rng = RfbRng::seeded(rejection_seed);
    resolve_p62_polymorph(&mut rejected);
    assert!(
        rejected
            .rng
            .draw_counter
            .saturating_sub(expected_save_draws)
            > 3
    );

    let permanent_build = rejected.build.clone();
    let permanent_skills = rejected.progress.skills.clone();
    let first_race = rejected
        .character_definitions()
        .expect("temporary form should retain character definitions")
        .1
        .id
        .clone();
    let mut stale = monster_combat::melee_status("test.status.stale-form", 10, "test.setup").status;
    stale.granted_race_id = Some("rfb-legacy.race.small-kobold".to_owned());
    rejected.player.statuses.push(stale);
    rejected
        .player
        .statuses
        .sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
    rejected.rng = RfbRng::seeded(p62_seed_for_legacy_index(15));
    resolve_p62_polymorph(&mut rejected);
    assert_eq!(
        rejected
            .player
            .statuses
            .iter()
            .filter(|status| status.granted_race_id.is_some())
            .count(),
        1
    );
    assert_ne!(
        rejected
            .character_definitions()
            .expect("replacement form should retain definitions")
            .1
            .id,
        first_race
    );
    assert_eq!(rejected.build, permanent_build);
    assert_eq!(rejected.progress.skills, permanent_skills);

    let restored = Game::from_save_with_content(rejected.to_save(), rejected.content.clone())
        .expect("temporary polymorph state should restore");
    assert_eq!(restored.build, permanent_build);
    assert_eq!(restored.state_hash(), rejected.state_hash());
    assert_eq!(p62_polymorph_legacy_index(&restored), Some(15));
}

#[test]
fn p62_polymorph_melee_changes_the_player_without_polymorphing_the_attacker() {
    let effect = MeleeBlowEffectDefinition::PolymorphPlayer {
        chance_percent: None,
    };
    let base = monster_effect_game(0, effect);
    let (seed, game) = (0..100_000)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .ok()?;
            p62_polymorph_legacy_index(&game).map(|_| (seed, game))
        })
        .expect("a bounded melee seed should polymorph the player");
    assert_ne!(
        game.character_definitions()
            .expect("polymorphed player should retain definitions")
            .1
            .legacy_index,
        Some(0)
    );
    assert_eq!(game.entities[0].kind_id, "demo.actor.small-kobold");
    assert_eq!(game.entities[0].appearance_kind_id, None);

    let mut replay = base;
    replay.rng = RfbRng::seeded(seed);
    replay
        .resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("polymorph melee replay should resolve");
    assert_eq!(replay.state_hash(), game.state_hash());
}

#[test]
fn p62_polymorph_reconciles_body_slots_and_expiry_does_not_reequip_items() {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    artifact
        .content
        .races
        .iter_mut()
        .find(|race| race.legacy_index == Some(1_007))
        .expect("small-kobold form should exist")
        .body_slots = vec![rfb_content::BodySlotDefinition {
        id: "weapon".to_owned(),
        slot_type: "weapon".to_owned(),
    }];
    let content = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("custom P62 race content should encode"),
    ));
    let mut game =
        Game::from_content_with_build(0, content, DEFAULT_WORLD_ID, "demo.build.warrior")
            .expect("custom P62 game should create");
    let permanent_build = game.build.clone();
    let permanent_attributes = game.effective_player_attributes();
    let permanent_resistances = game.effective_player_resistances();
    let permanent_skills = game.effective_player_skill_progress();
    let unequipped_id = game
        .items
        .iter()
        .find(|item| {
            matches!(item.location, ItemLocation::Equipped { .. })
                && game
                    .content
                    .item(&item.kind_id)
                    .and_then(|definition| definition.equipment_slot.as_deref())
                    != Some("weapon")
        })
        .expect("warrior should start with non-weapon equipment")
        .id
        .clone();
    game.rng = RfbRng::seeded(p62_seed_for_legacy_index(1_007));
    resolve_p62_polymorph(&mut game);
    assert_eq!(p62_polymorph_legacy_index(&game), Some(1_007));
    assert_eq!(game.body_slots.len(), 1);
    assert_ne!(game.effective_player_attributes(), permanent_attributes);
    assert_ne!(game.effective_player_resistances(), permanent_resistances);
    assert_ne!(game.effective_player_skill_progress(), permanent_skills);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );
    assert!(!matches!(
        game.items
            .iter()
            .find(|item| item.id == unequipped_id)
            .expect("unequipped item should remain present")
            .location,
        ItemLocation::Equipped { .. }
    ));

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("custom body form should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    game.player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == STATUS_PLAYER_POLYMORPH)
        .expect("temporary form should remain active")
        .remaining_ticks = 1;
    game.process_status_tick(
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        false,
    )
    .expect("polymorph expiry should process");
    assert_eq!(p62_polymorph_legacy_index(&game), None);
    assert_eq!(game.build, permanent_build);
    assert_eq!(game.effective_player_attributes(), permanent_attributes);
    assert_eq!(game.effective_player_resistances(), permanent_resistances);
    assert_eq!(game.effective_player_skill_progress(), permanent_skills);
    assert_eq!(
        game.body_slots,
        resolve_body_slots(&game.content, game.build.as_ref())
            .expect("permanent body slots should resolve")
    );
    assert!(!matches!(
        game.items
            .iter()
            .find(|item| item.id == unequipped_id)
            .expect("expired-form item should remain present")
            .location,
        ItemLocation::Equipped { .. }
    ));
}

#[test]
fn p60_melee_curse_damage_uses_the_existing_monster_curse_save() {
    fn resolve(seed: u64) -> Option<(bool, i32)> {
        let mut game = monster_effect_game(
            0,
            MeleeBlowEffectDefinition::Damage {
                chance_percent: None,
                damage_dice: 6,
                damage_sides: 6,
                damage_type: ActorDamageType::Curse,
                armor_mitigated: false,
                vampiric: false,
            },
        );
        game.rng = RfbRng::seeded(seed);
        let hp_before = game.player.hp;
        let mut events = Vec::new();
        game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .expect("curse melee should resolve");
        let saved = events.iter().find_map(|event| match event {
            DomainEvent::SavingThrowChecked { succeeded, .. } => Some(*succeeded),
            _ => None,
        });
        saved.map(|saved| (saved, hp_before - game.player.hp))
    }

    assert_eq!(resolve(0), Some((true, 0)));
    assert!(matches!(resolve(7), Some((false, 6..=36))));
}

#[test]
fn mutation_contact_auras_retaliate_only_against_unresisted_contact_attacks() {
    let harmless = MeleeBlowEffectDefinition::Damage {
        chance_percent: None,
        damage_dice: 0,
        damage_sides: 0,
        damage_type: rfb_content::ActorDamageType::Physical,
        armor_mitigated: true,
        vampiric: false,
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
fn effectless_beg_always_succeeds_without_damage_contact_or_rng() {
    let mut game = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.attack = 1;
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
            blows: vec![rfb_content::MeleeBlowDefinition {
                method_id: "rfb.blow.beg".to_owned(),
                to_hit: -1_000_000,
                self_destructs: false,
                effects: Vec::new(),
            }],
        });
    });
    clear_monsters(&mut game);
    let beggar = game.generated_actor(
        "test.monster.beggar".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    game.entities.push(beggar);
    assert!(game.gain_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    game.entities[0].hp = 100;
    game.entities[0].max_hp = 100;
    let player_hp = game.player.hp;
    let monster_hp = game.entities[0].hp;
    let draws = game.rng.draw_counter;
    let mut events = Vec::new();

    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("BEG should resolve");

    assert_eq!(game.player.hp, player_hp);
    assert_eq!(game.entities[0].hp, monster_hp);
    assert_eq!(game.rng.draw_counter, draws);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterBegged { source_kind_id }]
            if source_kind_id == "demo.actor.small-kobold"
    ));
}

#[test]
fn monster_contact_auras_apply_elemental_damage_and_curse_saves() {
    let template = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.level = 50;
        actor.contact_auras = vec![
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Fire,
                damage_dice: 1,
                damage_sides: 1,
                chance_percent: None,
                ravages_time: false,
            },
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Electricity,
                damage_dice: 1,
                damage_sides: 1,
                chance_percent: None,
                ravages_time: false,
            },
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Ice,
                damage_dice: 1,
                damage_sides: 1,
                chance_percent: None,
                ravages_time: false,
            },
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Light,
                damage_dice: 1,
                damage_sides: 1,
                chance_percent: None,
                ravages_time: false,
            },
            rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Curse,
                damage_dice: 1,
                damage_sides: 1,
                chance_percent: None,
                ravages_time: false,
            },
        ];
    });
    let (game, events) = (0..1_000)
        .find_map(|seed| {
            let mut game = template.clone();
            game.rng = RfbRng::seeded(seed);
            game.player.hp = 100;
            game.player.max_hp = 100;
            let definition = game
                .content
                .actor("demo.actor.small-kobold")
                .expect("test monster should exist")
                .clone();
            let mut events = Vec::new();
            game.resolve_monster_contact_auras(0, &definition, &mut events, &mut BTreeSet::new());
            events
                .iter()
                .any(|event| {
                    matches!(
                        event,
                        DomainEvent::SavingThrowChecked {
                            succeeded: false,
                            ..
                        }
                    )
                })
                .then_some((game, events))
        })
        .expect("a bounded seed should fail the curse save");

    assert_eq!(game.player.hp, 95);
    let damage_types = events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::MonsterMeleeHit { damage, .. } => Some(damage.damage_type),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        damage_types,
        vec![
            DamageType::Fire,
            DamageType::Electricity,
            DamageType::Ice,
            DamageType::Light,
            DamageType::Curse,
        ]
    );
}

#[test]
fn monster_revenge_aura_uses_one_blow_and_cannot_recurse() {
    let mut game = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.level = 150;
        actor.attack = 1_000_000;
        actor.tags.push("aura-revenge".to_owned());
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
            blows: vec![rfb_content::MeleeBlowDefinition {
                method_id: "rfb.blow.hit".to_owned(),
                to_hit: 0,
                self_destructs: false,
                effects: vec![MeleeBlowEffectDefinition::Damage {
                    chance_percent: None,
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_type: rfb_content::ActorDamageType::Physical,
                    armor_mitigated: false,
                    vampiric: false,
                }],
            }],
        });
    });
    clear_monsters(&mut game);
    game.player.position = Position { x: 3, y: 3 };
    game.player.hp = 100;
    game.player.max_hp = 100;
    game.push_generated_actor(
        "test.revenge".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    let mut events = Vec::new();

    assert_eq!(
        game.resolve_monster_revenge_aura(
            0,
            0,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("revenge aura should resolve"),
        Some(false)
    );
    assert_eq!(game.player.hp, 99);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::MonsterMeleeHit { .. }))
            .count(),
        1
    );

    game.entities[0]
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 10, "test").status);
    let draws_before = game.rng.draw_counter;
    assert_eq!(
        game.resolve_monster_revenge_aura(
            0,
            0,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("incapacitated revenge aura should be skipped"),
        None
    );
    assert_eq!(game.rng.draw_counter, draws_before);
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
            vampiric: false,
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
            vampiric: false,
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
fn percent_gated_resource_drain_uses_level_power_and_heals_the_caster() {
    let game = (0..100_u64)
        .find_map(|seed| {
            let mut game = monster_effect_game(
                seed,
                MeleeBlowEffectDefinition::DrainResource {
                    chance_percent: Some(25),
                    amount_dice: 1,
                    amount_sides: 25,
                },
            );
            game.resources.insert(
                "test.resource.mana".to_owned(),
                ResourcePool {
                    current: 25,
                    maximum: 25,
                },
            );
            game.entities[0].hp = 1;
            game.entities[0].max_hp = 200;
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .expect("percent-gated resource drain should resolve");
            (game.resources["test.resource.mana"].current < 25).then_some(game)
        })
        .expect("a deterministic seed should pass the 25% gate");

    let drained = 25 - game.resources["test.resource.mana"].current;
    assert!((1..=25).contains(&drained));
    assert_eq!(game.entities[0].hp, 1 + i32::try_from(drained * 6).unwrap());
}

#[test]
fn inertia_melee_uses_minor_slow_and_free_action_reduces_it() {
    let inertia = MeleeBlowEffectDefinition::Inertia {
        chance_percent: None,
    };
    let mut ordinary = monster_effect_game(0, inertia.clone());
    ordinary
        .resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("inertia melee should resolve");
    assert_eq!(ordinary.minor_slow, 5);

    let mut free_action = monster_effect_game(0, inertia);
    give_inventory_item(
        &mut free_action,
        "test.item.free-action",
        "demo.item.calm-pendant",
    );
    free_action
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.free-action")
        .expect("free-action item should exist")
        .location = ItemLocation::Equipped {
        slot_id: "amulet".to_owned(),
    };
    free_action
        .resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("free action inertia melee should resolve");
    assert_eq!(free_action.minor_slow, 1);
}

#[test]
fn amberite_death_can_curse_equipment_and_apply_multiple_nonlethal_ty_curses() {
    let game = (0..64)
        .find_map(|seed| {
            let mut game = game_with_actor_definition(seed, "demo.actor.small-kobold", |actor| {
                actor.level = 41;
                actor.tags.push("amberite".to_owned());
            });
            clear_monsters(&mut game);
            for item in game
                .items
                .iter_mut()
                .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
            {
                item.location = ItemLocation::Inventory;
            }
            give_inventory_item(&mut game, "test.item.dagger", "demo.item.dagger");
            game.items
                .iter_mut()
                .find(|item| item.id == "test.item.dagger")
                .expect("dagger should exist")
                .location = ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            };
            game.debug_set_item_curses_land(true);
            game.player.hp = 500;
            game.player.max_hp = 500;
            let monster = game.generated_actor(
                "test.actor.amberite".to_owned(),
                "demo.actor.small-kobold",
                Position { x: 4, y: 3 },
            );
            game.entities.push(monster);
            game.resolve_actor_death_without_rewards(
                0,
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("Amberite death should resolve");
            (game.player.hp < 500).then_some(game)
        })
        .expect("a deterministic seed should trigger the blood curse");

    assert!((1..500).contains(&game.player.hp));
    assert!(matches!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.dagger")
            .expect("dagger should remain equipped")
            .curse,
        Some(ItemCurseSeverityDto::Normal | ItemCurseSeverityDto::Heavy)
    ));
    for status_kind_id in [STATUS_CONFUSION, STATUS_STUN] {
        let status = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == status_kind_id)
            .expect("blood curse should apply its status");
        assert!((82..=164).contains(&status.remaining_ticks));
        assert_eq!(status.source_id.as_deref(), Some("demo.actor.small-kobold"));
    }
}

#[test]
fn bomb_death_explosion_splits_sound_and_shards_with_status_riders() {
    fn bomb_game(resistant: bool) -> Game {
        let mut game = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
            actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
                blows: vec![rfb_content::MeleeBlowDefinition {
                    method_id: "rfb.blow.explode".to_owned(),
                    to_hit: 20,
                    self_destructs: true,
                    effects: vec![MeleeBlowEffectDefinition::Bomb {
                        chance_percent: None,
                        damage_dice: 90,
                        damage_sides: 1,
                    }],
                }],
            });
        });
        clear_monsters(&mut game);
        game.player.hp = 500;
        game.player.max_hp = 500;
        if resistant {
            game.player
                .resistances
                .set(DamageType::Shards, ResistanceLevel::Resistant);
            game.player
                .resistances
                .set(DamageType::Sound, ResistanceLevel::Resistant);
        }
        let position = Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        };
        let monster = game.generated_actor(
            "test.actor.bomb".to_owned(),
            "demo.actor.small-kobold",
            position,
        );
        game.entities.push(monster);
        game.resolve_actor_death_without_rewards(
            0,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("bomb death should resolve");
        game
    }

    let normal = bomb_game(false);
    assert_eq!(normal.player.hp, 446);
    let bleeding = normal
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_BLEEDING)
        .expect("unresisted shards should cause bleeding");
    assert_eq!(bleeding.remaining_ticks, 24);
    let stun = normal
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_STUN)
        .expect("unresisted sound should cause stun");
    assert!((1..=15).contains(&stun.remaining_ticks));

    let resistant = bomb_game(true);
    assert_eq!(resistant.player.hp, 473);
    assert!(
        resistant
            .player
            .statuses
            .iter()
            .all(|status| { !matches!(status.kind_id.as_str(), STATUS_BLEEDING | STATUS_STUN) })
    );
}

#[test]
fn slow_death_explosion_uses_radius_free_action_and_monster_saves() {
    fn slow_game(target_kind_id: Option<&str>, free_action: bool) -> Game {
        let mut game = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
            actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition {
                blows: vec![rfb_content::MeleeBlowDefinition {
                    method_id: "rfb.blow.explode".to_owned(),
                    to_hit: 20,
                    self_destructs: true,
                    effects: vec![MeleeBlowEffectDefinition::Slow {
                        chance_percent: None,
                    }],
                }],
            });
        });
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 6, y: 3 };
        if free_action {
            give_inventory_item(&mut game, "test.item.free-action", "demo.item.calm-pendant");
            game.items
                .iter_mut()
                .find(|item| item.id == "test.item.free-action")
                .expect("free-action item should exist")
                .location = ItemLocation::Equipped {
                slot_id: "amulet".to_owned(),
            };
        }
        let source = game.generated_actor(
            "test.actor.exploder".to_owned(),
            "demo.actor.small-kobold",
            Position { x: 3, y: 3 },
        );
        game.entities.push(source);
        if let Some(target_kind_id) = target_kind_id {
            let mut target = game.generated_actor(
                "test.actor.target".to_owned(),
                target_kind_id,
                Position { x: 3, y: 4 },
            );
            target.hp = 1_000;
            target.max_hp = 1_000;
            game.entities.push(target);
        }
        game.resolve_actor_death_without_rewards(
            0,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("slow death explosion should resolve");
        game
    }

    let ordinary = slow_game(Some("demo.actor.small-kobold"), false);
    assert_eq!(
        ordinary.minor_slow, 1,
        "radius-three player should be slowed"
    );
    assert!(
        ordinary.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLOW)
    );

    let free_action = slow_game(None, true);
    assert_eq!(free_action.minor_slow, 0);

    for protected_kind_id in ["demo.actor.aether-vortex", "demo.actor.smeagol"] {
        let protected = slow_game(Some(protected_kind_id), false);
        assert!(
            protected.entities[0]
                .statuses
                .iter()
                .all(|status| status.kind_id != STATUS_SLOW)
        );
    }
}

#[test]
fn charge_drain_melee_consumes_a_carried_device_or_player_nutrition() {
    let effect = MeleeBlowEffectDefinition::DrainCharges {
        chance_percent: None,
    };
    let mut charged = monster_effect_game(0, effect.clone());
    give_inventory_item(
        &mut charged,
        "test.item.identify-staff",
        "demo.item.identify-staff",
    );
    let charges = charged
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.identify-staff")
        .and_then(|item| item.charges.as_mut())
        .expect("identify staff should carry charges");
    charges.current = 5;
    charged.entities[0].hp = 1;
    charged.entities[0].max_hp = 20;
    let mut events = Vec::new();
    charged
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("charge drain should resolve");
    assert_eq!(
        charged
            .items
            .iter()
            .find(|item| item.id == "test.item.identify-staff")
            .and_then(|item| item.charges)
            .expect("identify staff should retain its charge pool")
            .current,
        4
    );
    assert_eq!(charged.entities[0].hp, 2);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterChargesDrained { amount: 1, .. }))
    );

    let mut hungry = monster_effect_game(0, effect);
    hungry.nutrition = 9_000;
    let mut events = Vec::new();
    hungry
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("fallback nutrition drain should resolve");
    assert_eq!(hungry.nutrition, 6_000);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterNutritionDrained { amount: 3_000, .. }
    )));
}

#[test]
fn vampiric_melee_heals_from_applied_damage_but_not_from_nonliving_players() {
    let effect = MeleeBlowEffectDefinition::Damage {
        chance_percent: None,
        damage_dice: 1,
        damage_sides: 4,
        damage_type: rfb_content::ActorDamageType::Physical,
        armor_mitigated: false,
        vampiric: true,
    };
    let mut living = monster_effect_game(0, effect.clone());
    living.entities[0].hp = 1;
    living.entities[0].max_hp = 20;
    let mut events = Vec::new();
    living
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("vampiric melee should resolve");
    let applied = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::MonsterMeleeHit { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .expect("vampiric melee should damage the player");
    assert!(applied > 0);
    assert_eq!(living.entities[0].hp, 1 + applied);

    let mut nonliving = monster_effect_game(0, effect);
    nonliving.entities[0].hp = 1;
    nonliving.entities[0].max_hp = 20;
    let mut race_status =
        monster_combat::melee_status("test.status.nonliving", 10, "test.setup").status;
    race_status.granted_race_id = Some("demo.race.vampire-lord".to_owned());
    nonliving.player.statuses.push(race_status);
    let hp_before = nonliving.player.hp;
    nonliving
        .resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("vampiric melee against a nonliving player should resolve");
    assert!(nonliving.player.hp < hp_before);
    assert_eq!(nonliving.entities[0].hp, 1);
}

#[test]
fn shatter_melee_uses_the_shared_earthquake_only_above_the_damage_threshold() {
    let mut strong = monster_effect_game(
        0,
        MeleeBlowEffectDefinition::Shatter {
            chance_percent: None,
            damage_dice: 10,
            damage_sides: 10,
        },
    );
    strong.player.hp = 10_000;
    strong.player.max_hp = 10_000;
    let mut events = Vec::new();
    strong
        .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("strong shatter should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterEarthquakeResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::Earthquake { radius: 8, .. }]
            )
    )));

    let mut weak = monster_effect_game(
        0,
        MeleeBlowEffectDefinition::Shatter {
            chance_percent: None,
            damage_dice: 1,
            damage_sides: 1,
        },
    );
    weak.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("weak shatter should resolve");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::MonsterEarthquakeResolved { .. }))
            .count(),
        1
    );
}

#[test]
fn gaze_projects_the_melee_routine_to_a_distant_target() {
    let mut game = game_with_actor_definition(0, "demo.actor.beholder", |actor| {
        actor.attack = 1_000_000;
        actor
            .monster_casting
            .as_mut()
            .expect("Beholder should cast")
            .frequency_percent = 100;
    });
    clear_monsters(&mut game);
    let origin = Position { x: 4, y: 3 };
    game.player.position = Position { x: 8, y: 3 };
    for x in 4..=8 {
        replace_terrain(&mut game, Position { x, y: 3 }, "demo.terrain.floor");
    }
    game.push_generated_actor("test.beholder".to_owned(), "demo.actor.beholder", origin);
    game.entities[0].nice = false;
    let mut events = Vec::new();

    assert!(game.resolve_monster_ability(0, &mut events));
    assert_eq!(game.entities[0].position, origin);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterAbilityCast { resolution, .. }
            if resolution.ability_id == "rfb-legacy.ability.gaze"
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterMeleeHit { .. }))
    );
}

#[test]
fn melee_amnesia_uses_the_existing_save_and_floor_memory_wipe() {
    let template = monster_effect_game(
        0,
        MeleeBlowEffectDefinition::Amnesia {
            chance_percent: None,
        },
    );
    let seed = (0..1_000)
        .find(|seed| {
            let mut trial = template.clone();
            trial.explored.fill(true);
            trial.rng = RfbRng::seeded(*seed);
            let mut events = Vec::new();
            trial
                .resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
                .expect("melee amnesia should resolve");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::MonsterMeleeAmnesia { .. }))
        })
        .expect("a deterministic seed should fail the amnesia save");
    let mut game = template;
    game.explored.fill(true);
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();

    game.resolve_monster_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("melee amnesia should resolve");
    assert!(game.explored.iter().all(|explored| !explored));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterMeleeAmnesia { cleared_cells, .. } if *cleared_cells > 0
    )));
}

#[test]
fn dice_less_time_uses_exp_or_fractional_attribute_ravaging_without_damage() {
    let template = Game::new(0);
    let exp_seed = (0..100)
        .find(|seed| {
            let mut trial = template.clone();
            trial.rng = RfbRng::seeded(*seed);
            trial.progress.experience = 1_000;
            trial.progress.maximum_experience = 1_000;
            let mut events = Vec::new();
            trial.resolve_time_melee("demo.actor.chronomage", &mut events);
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::ExperienceDrained { .. }))
        })
        .expect("a deterministic seed should select TIME experience drain");
    let mut exp = template.clone();
    exp.rng = RfbRng::seeded(exp_seed);
    exp.progress.experience = 1_000;
    exp.progress.maximum_experience = 1_000;
    let hp_before = exp.player.hp;
    exp.resolve_time_melee("demo.actor.chronomage", &mut Vec::new());
    assert_eq!(exp.progress.experience, 880);
    assert_eq!(exp.player.hp, hp_before);

    let all_seed = (0..100)
        .find(|seed| {
            let mut trial = template.clone();
            trial.rng = RfbRng::seeded(*seed);
            let mut events = Vec::new();
            trial.resolve_time_melee("demo.actor.chronomage", &mut events);
            events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::MonsterTimeRavaged {
                        attribute_count: 6,
                        ..
                    }
                )
            })
        })
        .expect("a deterministic seed should select all-attribute TIME ravaging");
    let mut all = template;
    all.rng = RfbRng::seeded(all_seed);
    all.progress.attributes = AttributeSet {
        strength: 16,
        intelligence: 16,
        wisdom: 16,
        dexterity: 16,
        constitution: 16,
        charisma: 16,
    };
    all.resolve_time_melee("demo.actor.chronomage", &mut Vec::new());
    assert_eq!(
        all.progress.attributes,
        AttributeSet {
            strength: 14,
            intelligence: 14,
            wisdom: 14,
            dexterity: 14,
            constitution: 14,
            charisma: 14,
        }
    );
}

#[test]
fn unlife_melee_drains_life_force_and_persistently_empowers_the_monster() {
    let effect = MeleeBlowEffectDefinition::Unlife {
        chance_percent: None,
        amount_dice: 100,
        amount_sides: 1,
    };
    let mut game = monster_effect_game(0, effect.clone());
    let full_max_hp = game.effective_player_max_hp();
    game.player.hp = full_max_hp;
    let base_attack = game
        .actor_derived_stats(
            &game.entities[0],
            game.content
                .actor(&game.entities[0].kind_id)
                .expect("test monster definition should exist"),
            false,
        )
        .attack
        .value;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    game.resolve_monster_melee(0, &mut events, &mut changed, &mut Vec::new())
        .expect("UNLIFE melee should resolve");

    assert_eq!(game.progress.life_force, 900);
    assert_eq!(game.entities[0].power_per_mille, 1_100);
    assert_eq!(
        game.effective_player_max_hp(),
        full_max_hp - full_max_hp * 100 / 2_000
    );
    assert_eq!(game.player.hp, game.effective_player_max_hp());
    assert!(changed.contains(&game.entities[0].position));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterUnlifeDrained {
            amount: 100,
            life_force_before: 1_000,
            life_force_after: 900,
            power_before: 1_000,
            power_after: 1_100,
            ..
        }
    )));
    assert_eq!(
        game.actor_derived_stats(
            &game.entities[0],
            game.content
                .actor(&game.entities[0].kind_id)
                .expect("test monster definition should exist"),
            false,
        )
        .attack
        .value,
        base_attack * 11 / 10
    );

    let spell = game.resolve_monster_damage_to_player(
        &game.entities[0].id.clone(),
        &game.entities[0].kind_id.clone(),
        "test.ability.unlife-power",
        0,
        10,
        10,
        DamageType::Mana,
        &mut events,
    );
    assert!(matches!(
        spell,
        AbilityEffectResolutionDto::Damage { resolution, .. }
            if resolution.final_damage == 11
    ));

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("empowered monster should round trip");
    assert_eq!(restored.entities[0].power_per_mille, 1_100);
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut nonliving = monster_effect_game(0, effect);
    let mut race_status =
        monster_combat::melee_status("test.status.nonliving", 10, "test.setup").status;
    race_status.granted_race_id = Some("demo.race.vampire-lord".to_owned());
    nonliving.player.statuses.push(race_status);
    nonliving
        .resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("UNLIFE should harmlessly reach a nonliving player");
    assert_eq!(nonliving.progress.life_force, 1_000);
    assert_eq!(nonliving.entities[0].power_per_mille, 1_000);
}

#[test]
fn hold_life_can_save_against_unlife_without_changing_life_force_or_monster_power() {
    let effect = MeleeBlowEffectDefinition::Unlife {
        chance_percent: None,
        amount_dice: 100,
        amount_sides: 1,
    };
    let mut protected = monster_effect_game(0, effect.clone());
    protected.progress.level = 50;
    let equipment_index = protected
        .items
        .iter()
        .position(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if protected.body_slot_type(slot_id) != Some("tool"))
        })
        .expect("test character should have non-tool equipment");
    protected.items[equipment_index]
        .rolled_affixes
        .push(RolledAffixState {
            affix_id: "test.affix.hold-life".to_owned(),
            properties: AffixPropertyBundleDefinition {
                passives: BTreeSet::from([EquipmentPassive::HoldLife]),
                ..AffixPropertyBundleDefinition::default()
            },
        });

    let save_seed = (0..100)
        .find(|seed| {
            let mut game = protected.clone();
            game.rng = RfbRng::seeded(*seed);
            game.resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
                .expect("protected UNLIFE melee should resolve");
            game.progress.life_force == 1_000 && game.entities[0].power_per_mille == 1_000
        })
        .expect("Hold Life should save for at least one deterministic seed");

    let mut unprotected = monster_effect_game(save_seed, effect);
    unprotected.progress.level = 50;
    unprotected
        .resolve_monster_melee(0, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("unprotected UNLIFE melee should resolve");
    assert_eq!(unprotected.progress.life_force, 900);
    assert_eq!(unprotected.entities[0].power_per_mille, 1_100);
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
fn bleeding_ticks_as_physical_damage_in_stable_status_order() {
    let mut payload = Game::new(42).to_save();
    let initial_hp = payload.player.hp;
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

    assert_eq!(update.player.hp, initial_hp - 5);
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
    let fire = MeleeBlowEffectDefinition::Damage {
        chance_percent: None,
        damage_dice: 1,
        damage_sides: 4,
        damage_type: rfb_content::ActorDamageType::Fire,
        armor_mitigated: false,
        vampiric: false,
    };
    let (seed, normal_damage) = (0_u64..1_000)
        .find_map(|seed| {
            let mut game = monster_effect_game(seed, fire.clone());
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

    let mut resistant = monster_effect_game(seed, fire);
    resistant.player.resistances.set(
        DamageType::Fire,
        crate::resistance::ResistanceLevel::Resistant,
    );
    let hp_before = resistant.player.hp;
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
    assert_eq!(resistant.player.hp, hp_before - resisted_damage);
}

#[test]
fn explicit_empty_melee_routine_performs_no_attack() {
    let mut game = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.melee_routine = Some(rfb_content::MeleeRoutineDefinition { blows: Vec::new() });
    });
    clear_monsters(&mut game);
    let monster = game.generated_actor(
        "test.monster.empty-melee".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    game.entities.push(monster);
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
    game.rng = RfbRng::seeded(7);
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
        18,
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
        .current = 250;
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
            .any(|event| matches!(event, DomainEvent::MonsterLightEaten { amount: 249, .. }))
    );
}

#[test]
fn leader_death_dissolves_pack_before_remaining_members_act() {
    let mut initial = Game::new(42);
    clear_monsters(&mut initial);
    let mut leader = initial.generated_actor(
        "test.pack.leader".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 7, y: 6 },
    );
    leader.hp = 1;
    initial.entities.push(leader);
    let mut payload = initial.to_save();
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

    assert_eq!(game.entities.len(), 1);
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

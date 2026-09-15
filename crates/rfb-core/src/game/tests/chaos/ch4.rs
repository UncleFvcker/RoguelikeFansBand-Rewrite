// SPDX-License-Identifier: MPL-2.0
use super::ch3::cast_where;
use super::*;
use crate::game::ability_scaling::spell_power_value;
use rfb_protocol::{AbilityEffectResolutionDto, DamageTypeDto};

fn east() -> TargetSelection {
    TargetSelection::Direction {
        direction: Direction::East,
    }
}

fn bonus(game: &Game) -> u16 {
    let profile = game.casting_profile().unwrap();
    profile.spell_damage_bonus_base
        + profile.spell_damage_bonus_per_level
            * (game.progress.level / u16::from(profile.spell_damage_bonus_level_divisor))
        + game.armor_spell_damage_bonus()
}

#[test]
fn ch4_existing_fourth_book_is_generated_picked_up_saved_and_studied() {
    let mut game = caster("high-mage", 50);
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 85,
        source: LootSource::ItemUse {
            item_id: "test.ordinary-pool".into(),
        },
    };
    let draft = (0..32_768)
        .find_map(|_| {
            game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .filter(|d| d.kind_id == "demo.item.armageddon-tome")
        })
        .expect("existing kind 515 must remain in the ordinary pool");
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let item_id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&item_id)).unwrap();
    game = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    let id = learn(&mut game, "gravity-beam");
    target(&mut game);
    cast_saved(&mut game, &id, east());
    assert!(game.entities[0].hp < 10_000);
    let book = game.content.item("demo.item.armageddon-tome").unwrap();
    assert_eq!(book.rfb_base_kind.as_ref().unwrap().source_index, 515);
    assert_eq!(
        book.ability_book_id.as_deref(),
        Some("demo.ability-book.armageddon-tome")
    );
}

#[test]
fn ch4_flame_mana_and_logrus_use_executed_damage_radius_and_current_hp() {
    for class in ["mage", "high-mage"] {
        for (slug, hp) in [
            ("flame-strike", 100),
            ("mana-storm", 100),
            ("breathe-logrus", 40),
            ("breathe-logrus", 120),
        ] {
            let mut game = caster(class, 50);
            let id = learn(&mut game, slug);
            target(&mut game);
            game.player.hp = hp;
            let power = game.effective_player_spell_power_bonus();
            let extra = i32::from(bonus(&game));
            let (base, radius, kind) = match slug {
                "flame-strike" => (450 + 2 * extra, 8, DamageTypeDto::Fire),
                "mana-storm" => (
                    500 + extra,
                    spell_power_value(4, power) as u8,
                    DamageTypeDto::Mana,
                ),
                _ => (
                    hp * 3 / 4 + extra,
                    spell_power_value(2, power) as u8,
                    DamageTypeDto::Chaos,
                ),
            };
            let expected = spell_power_value(base as u64, power) as i32;
            let selection = if slug == "flame-strike" {
                TargetSelection::SelfTarget
            } else {
                TargetSelection::Position {
                    position: game.entities[0].position,
                }
            };
            let events = cast_saved(&mut game, &id, selection);
            let blast = events
                .iter()
                .find_map(|e| match e {
                    DomainEvent::AbilityAreaDamage { resolution, .. } => Some(resolution),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                (blast.base_raw_damage, blast.radius, blast.damage_type),
                (expected, radius, kind)
            );
            assert_eq!(
                10_000 - game.entities[0].hp,
                if slug == "flame-strike" {
                    (expected + 2) / 3
                } else {
                    expected
                }
            );
        }
    }
}

#[test]
fn ch4_meteor_swarm_replays_independent_centers_and_does_not_project_terrain() {
    let mut base = caster("high-mage", 50);
    let id = learn(&mut base, "meteor-swarm");
    target(&mut base);
    let tree = base.index(Position { x: 11, y: 11 }).unwrap();
    base.terrain[tree] = "demo.terrain.surface-tree".into();
    let expected = spell_power_value(
        u64::from(100 + bonus(&base)),
        base.effective_player_spell_power_bonus(),
    ) as i32;
    let (game, events) = cast_where(&base, &id, TargetSelection::SelfTarget, |g, _| {
        g.entities[0].hp < 10_000
    });
    let blasts: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            DomainEvent::AbilityAreaDamage { resolution, .. } => Some(resolution),
            _ => None,
        })
        .collect();
    assert!((11..=20).contains(&blasts.len()));
    assert!(
        blasts
            .iter()
            .map(|b| b.center)
            .collect::<BTreeSet<_>>()
            .len()
            > 1
    );
    for blast in blasts {
        assert_eq!(
            (blast.base_raw_damage, blast.radius, blast.damage_type),
            (expected, 2, DamageTypeDto::Meteor)
        );
        assert!(
            crate::game::projectile_geometry::rfb_distance(base.player.position, blast.center) < 9
        );
    }
    assert_eq!(game.terrain[tree], "demo.terrain.surface-tree");
}

#[test]
fn ch4_call_chaos_all_31_types_and_both_shapes_resume_without_repaying() {
    let mut base = caster("high-mage", 50);
    let id = learn(&mut base, "call-chaos");
    // Keep source entries MISSILE and ARROW separate despite their shared physical representation.
    let kinds = [
        DamageTypeDto::Electricity,
        DamageTypeDto::Poison,
        DamageTypeDto::Acid,
        DamageTypeDto::Cold,
        DamageTypeDto::Fire,
        DamageTypeDto::Physical,
        DamageTypeDto::Physical,
        DamageTypeDto::Plasma,
        DamageTypeDto::HolyFire,
        DamageTypeDto::Water,
        DamageTypeDto::Light,
        DamageTypeDto::Dark,
        DamageTypeDto::Force,
        DamageTypeDto::Inertia,
        DamageTypeDto::Mana,
        DamageTypeDto::Meteor,
        DamageTypeDto::Ice,
        DamageTypeDto::Chaos,
        DamageTypeDto::Nether,
        DamageTypeDto::Disenchant,
        DamageTypeDto::Shards,
        DamageTypeDto::Sound,
        DamageTypeDto::Nexus,
        DamageTypeDto::Confusion,
        DamageTypeDto::Time,
        DamageTypeDto::Gravity,
        DamageTypeDto::Rocket,
        DamageTypeDto::Nuke,
        DamageTypeDto::HellFire,
        DamageTypeDto::Disintegrate,
        DamageTypeDto::PsySpear,
    ];
    for roll in 1..=62 {
        let (mut game, _) = cast_where(&base, &id, TargetSelection::SelfTarget, |g, _| {
            g.pending_ability_direction
                .as_ref()
                .is_some_and(|p| p.branch_roll == roll)
        });
        let paid = game.resources["demo.resource.mana"].current;
        let count = game.ability_progress[&id].cast_count;
        let mut restored = Game::from_save_with_content(
            game.to_save(),
            game.content.clone(),
            game.behavior_preferences(),
        )
        .unwrap();
        for g in [&mut game, &mut restored] {
            let mut events = Vec::new();
            g.resolve_pending_ability_direction(
                Direction::East,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            let (damage, kind, beam) = events
                .iter()
                .find_map(|e| match e {
                    DomainEvent::AbilityBeamDamage { resolution, .. } => {
                        Some((resolution.base_raw_damage, resolution.damage_type, true))
                    }
                    DomainEvent::AbilityAreaDamage { resolution, .. } => {
                        assert_eq!(resolution.radius, 4);
                        Some((resolution.base_raw_damage, resolution.damage_type, false))
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                (damage, kind, beam),
                (250, kinds[usize::from((roll - 1) / 2)], roll % 2 == 0)
            );
            assert_eq!(g.resources["demo.resource.mana"].current, paid);
            assert_eq!(g.ability_progress[&id].cast_count, count);
            assert!(g.pending_ability_direction.is_none());
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
    }
}

#[test]
fn ch4_call_chaos_full_compass_and_self_blast_keep_fixed_damage() {
    let mut base = caster("high-mage", 50);
    let id = learn(&mut base, "call-chaos");
    target(&mut base);
    for (count, damage) in [(8, 150), (1, 500)] {
        let (game, events) = cast_where(&base, &id, TargetSelection::SelfTarget, |g, events| {
            g.pending_ability_direction.is_none()
                && events
                    .iter()
                    .filter(|e| match e {
                        DomainEvent::AbilityAreaDamage { resolution, .. } => {
                            resolution.base_raw_damage == damage
                        }
                        DomainEvent::AbilityBeamDamage { resolution, .. } => {
                            resolution.base_raw_damage == damage
                        }
                        _ => false,
                    })
                    .count()
                    == count
        });
        assert!(game.entities[0].hp < 10_000);
        if count == 1 {
            assert!(events.iter().any(|e| matches!(e,
                DomainEvent::AbilityAreaDamage { resolution, .. }
                    if resolution.radius == 8 && resolution.center == base.player.position)));
        }
    }
}

#[test]
fn ch4_call_chaos_paid_cancel_waits_one_action_and_rejects_forged_saves() {
    use crate::game::tests::support::{command, dispatch_next};
    let mut base = caster("mage", 50);
    let id = learn(&mut base, "call-chaos");
    for p in base.resources.values_mut() {
        p.current = p.maximum;
    }
    let mut pending = (0..8192)
        .find_map(|seed| {
            let mut g = base.clone();
            g.rng = RfbRng::seeded(seed);
            dispatch_next(
                &mut g,
                GameCommand::CastAbility {
                    ability_id: id.clone(),
                    target: TargetSelection::SelfTarget,
                },
            );
            g.pending_ability_direction.is_some().then_some(g)
        })
        .unwrap();
    assert_eq!(
        (pending.turn, pending.world_tick),
        (base.turn, base.world_tick)
    );
    let paid = pending.resources["demo.resource.mana"].current;
    assert!(paid < base.resources["demo.resource.mana"].current);
    let snapshot = pending.snapshot();
    assert!(matches!(
        pending.dispatch(command(
            snapshot.last_command_seq + 1,
            snapshot.revision,
            GameCommand::Wait
        )),
        Err(CoreError::AbilityDirectionRequired)
    ));
    for invalid in 0..5 {
        let mut save = pending.to_save();
        let p = save.player.pending_ability_direction.as_mut().unwrap();
        match invalid {
            0 => p.branch_roll = 0,
            1 => p.branch_roll = 63,
            2 => p.cast_resolution.resource_after += 1,
            3 => p.cast_resolution.succeeded = false,
            _ => p.cast_resolution.ability_id = "demo.ability.chaos-gravity-beam".into(),
        }
        assert!(
            Game::from_save_with_content(
                save,
                pending.content.clone(),
                Game::default_behavior_preferences()
            )
            .is_err()
        );
    }
    let mut restored = Game::from_save_with_content(
        pending.to_save(),
        pending.content.clone(),
        pending.behavior_preferences(),
    )
    .unwrap();
    let mut direct = pending.clone();
    let mut rng = direct.rng.clone();
    // Finishing a paid book spell still performs its one Chance virtue roll.
    rng.bounded(100);
    direct
        .resolve_pending_call_chaos(None, &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert_eq!(direct.rng, rng);
    assert_eq!(direct.resources["demo.resource.mana"].current, paid);
    assert_eq!(direct.ability_progress[&id].cast_count, 1);
    for g in [&mut pending, &mut restored] {
        dispatch_next(g, GameCommand::CancelAbilityDirection);
        assert_eq!(g.turn, base.turn + 1);
        assert!(g.pending_ability_direction.is_none());
        assert_eq!(g.ability_progress[&id].cast_count, 1);
    }
    assert_eq!(pending.state_hash(), restored.state_hash());
    assert_eq!(pending.rng, restored.rng);
}

#[test]
fn ch4_polymorph_all_source_candidates_use_real_temporary_races_and_expire() {
    let mut base = caster("high-mage", 50);
    let id = learn(&mut base, "polymorph-self");
    let birth = base.build.as_ref().unwrap().race_id.clone();
    for &(index, race_id) in rfb_content::CHAOS_POLYMORPH_RACES {
        if race_id == birth {
            continue;
        }
        let (mut game, _) = cast_where(&base, &id, TargetSelection::SelfTarget, |g, _| {
            g.player
                .statuses
                .iter()
                .any(|s| s.granted_race_id.as_deref() == Some(race_id))
        });
        assert_eq!(
            game.character_definitions().unwrap().1.legacy_index,
            Some(index)
        );
        assert_eq!(game.build.as_ref().unwrap().race_id, birth);
        let duration = game
            .player
            .statuses
            .iter()
            .find(|s| s.granted_race_id.is_some())
            .unwrap()
            .remaining_ticks;
        assert!((51..=100).contains(&duration));
        assert_eq!(
            game.player
                .statuses
                .iter()
                .filter(|s| s.granted_race_id.is_some())
                .count(),
            1
        );
        if index == 1001 {
            assert!(game.player_levitates());
        }
        for _ in 0..duration {
            game.process_status_tick(
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
                false,
            )
            .unwrap();
        }
        assert!(
            game.player
                .statuses
                .iter()
                .all(|s| s.granted_race_id.is_none())
        );
        assert_eq!(game.character_definitions().unwrap().1.id, birth);
        let saved = Game::from_save_with_content(
            game.to_save(),
            game.content.clone(),
            game.behavior_preferences(),
        )
        .unwrap();
        assert_eq!(saved.state_hash(), game.state_hash());
    }
}

#[test]
fn ch4_polymorph_replaces_demon_form_and_reconciles_centaur_equipment() {
    let mut base = caster("high-mage", 50);
    let id = learn(&mut base, "polymorph-self");
    give_inventory_item(&mut base, "test.boots", "demo.item.soft-leather-boots");
    assert!(base.equip_inventory_item("test.boots", None).is_some());
    let (demon, _) = cast_where(&base, &id, TargetSelection::SelfTarget, |g, _| {
        g.player
            .statuses
            .iter()
            .any(|s| s.granted_race_id.as_deref() == Some("demo.race.demon-lord"))
    });
    let (mut centaur, _) = cast_where(&demon, &id, TargetSelection::SelfTarget, |g, _| {
        g.character_definitions().unwrap().1.legacy_index == Some(51)
    });
    assert!(!centaur.player_has_status_kind(crate::effect::STATUS_DEMON_LORD_TRANSFORMATION));
    assert!(
        !centaur
            .body_slots
            .iter()
            .any(|slot| slot.slot_type == "boots")
    );
    assert!(matches!(
        centaur
            .items
            .iter()
            .find(|i| i.id == "test.boots")
            .unwrap()
            .location,
        ItemLocation::Inventory
    ));
    let duration = centaur
        .player
        .statuses
        .iter()
        .find(|s| s.granted_race_id.is_some())
        .unwrap()
        .remaining_ticks;
    for _ in 0..duration {
        centaur
            .process_status_tick(
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
                false,
            )
            .unwrap();
    }
    assert!(
        centaur
            .body_slots
            .iter()
            .any(|slot| slot.slot_type == "boots")
    );
    assert!(matches!(
        centaur
            .items
            .iter()
            .find(|i| i.id == "test.boots")
            .unwrap()
            .location,
        ItemLocation::Equipped { .. }
    ));
    let saved = Game::from_save_with_content(
        centaur.to_save(),
        centaur.content.clone(),
        centaur.behavior_preferences(),
    )
    .unwrap();
    assert_eq!(saved.state_hash(), centaur.state_hash());
}

fn void_branch(events: &[DomainEvent], vanish: bool) -> bool {
    events.iter().any(|e| {
        matches!(e, DomainEvent::AbilityEffectsResolved { resolution, .. }
        if resolution.effects.iter().any(|effect| matches!(effect,
            AbilityEffectResolutionDto::RandomChoice { roll, maximum_roll: 666, .. }
                if (*roll == 1) == vanish)))
    })
}

fn self_damage(events: &[DomainEvent]) -> Option<(i32, bool)> {
    events.iter().find_map(|e| match e {
        DomainEvent::AbilityEffectsResolved { resolution, .. } => {
            resolution.effects.iter().find_map(|effect| match effect {
                AbilityEffectResolutionDto::SelfDamage { damage, fatal, .. } => {
                    Some((*damage, *fatal))
                }
                _ => None,
            })
        }
        _ => None,
    })
}

#[test]
fn ch4_void_open_ground_projects_three_complete_rounds() {
    let mut game = caster("high-mage", 50);
    let id = learn(&mut game, "call-the-void");
    target(&mut game);
    let events = cast_saved(&mut game, &id, TargetSelection::SelfTarget);
    let blasts: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            DomainEvent::AbilityAreaDamage { resolution, .. } => Some(resolution),
            _ => None,
        })
        .collect();
    assert_eq!(blasts.len(), 24);
    for (round, blasts) in blasts.chunks_exact(8).enumerate() {
        for b in blasts {
            assert_eq!(
                (b.base_raw_damage, b.radius, b.damage_type),
                (
                    175,
                    2 + round as u8,
                    [
                        DamageTypeDto::Rocket,
                        DamageTypeDto::Mana,
                        DamageTypeDto::Nuke
                    ][round]
                )
            );
        }
    }
    assert!(game.entities[0].hp < 10_000);
    assert!(self_damage(&events).is_none());
}

#[test]
fn ch4_void_near_wall_destroys_or_vanishes_then_hurts_but_town_is_protected() {
    let mut base = caster("high-mage", 50);
    let id = learn(&mut base, "call-the-void");
    let wall = base.index(Position { x: 11, y: 10 }).unwrap();
    base.terrain[wall] = "demo.terrain.wall".into();
    let mut town = base.clone();
    let hp = town.player.hp;
    let events = cast_saved(&mut town, &id, TargetSelection::SelfTarget);
    assert_eq!(town.player.hp, hp);
    assert_eq!(town.terrain[wall], "demo.terrain.wall");
    assert!(self_damage(&events).is_none());

    super::ch2::dungeon(&mut base);
    let wall = base.index(Position { x: 11, y: 10 }).unwrap();
    base.terrain[wall] = "demo.terrain.wall".into();
    base.vault_cells[wall] = true;
    base.terrain[0] = "demo.terrain.permanent-wall".into();
    let terrain = base.terrain.clone();
    for vanish in [false, true] {
        let (game, events) = cast_where(&base, &id, TargetSelection::SelfTarget, |g, e| {
            void_branch(e, vanish) && !g.player_is_dead()
        });
        let (damage, fatal) = self_damage(&events).unwrap();
        assert!((101..=250).contains(&damage));
        assert!(!fatal);
        assert_eq!(base.player.hp - game.player.hp, damage);
        assert_ne!(game.terrain, terrain);
        assert_eq!(game.terrain[0], terrain[0]);
        if vanish {
            assert!(game.vault_cells.iter().all(|v| !v));
            assert_ne!(game.terrain[wall], "demo.terrain.wall");
        }
    }
}

#[test]
fn ch4_void_fatal_backlash_does_not_award_first_success_experience() {
    let mut game = caster("high-mage", 50);
    let id = learn(&mut game, "call-the-void");
    super::ch2::dungeon(&mut game);
    let wall = game.index(Position { x: 11, y: 10 }).unwrap();
    game.terrain[wall] = "demo.terrain.wall".into();
    game.player.hp = 1;
    let experience = game.progress.experience;
    let events = cast_saved(&mut game, &id, TargetSelection::SelfTarget);
    assert!(game.player_is_dead());
    assert!(self_damage(&events).unwrap().1);
    assert_eq!(game.progress.experience, experience);
    assert!(matches!(
        events.last(),
        Some(DomainEvent::AbilityEffectsResolved { .. })
    ));
}

#[test]
fn ch4_void_noescape_still_uses_invulnerability_wraith_and_transcendence() {
    use crate::effect::{
        STATUS_INVULNERABILITY, STATUS_TRANSCENDENCE, STATUS_WRAITHFORM, StatusInstance,
    };
    let mut base = caster("high-mage", 50);
    let id = learn(&mut base, "call-the-void");
    super::ch2::dungeon(&mut base);
    let wall = base.index(Position { x: 11, y: 10 }).unwrap();
    base.terrain[wall] = "demo.terrain.wall".into();
    for (kind, percent) in [
        (STATUS_INVULNERABILITY, 0),
        (STATUS_WRAITHFORM, 50),
        (STATUS_TRANSCENDENCE, 100),
    ] {
        let mut protected = base.clone();
        protected.player.statuses.push(StatusInstance {
            kind_id: kind.into(),
            intensity: 1,
            remaining_ticks: 100,
            source_id: None,
            granted_resistances: Default::default(),
            granted_brands: Default::default(),
            granted_modifiers: Default::default(),
            granted_equipment_bonuses: Default::default(),
            granted_status_immunities: Default::default(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: percent,
        });
        protected
            .player
            .statuses
            .sort_by(|a, b| a.kind_id.cmp(&b.kind_id));
        let (game, events) = cast_where(&protected, &id, TargetSelection::SelfTarget, |g, e| {
            void_branch(e, false)
                && !g.player_is_dead()
                && self_damage(e).is_some_and(|(damage, _)| {
                    if kind == STATUS_WRAITHFORM {
                        (50..=125).contains(&damage)
                    } else {
                        damage == 0
                    }
                })
        });
        let damage = self_damage(&events).unwrap().0;
        assert_eq!(protected.player.hp - game.player.hp, damage);
        if kind == STATUS_TRANSCENDENCE {
            let cast = events
                .iter()
                .find_map(|e| match e {
                    DomainEvent::AbilityCastSucceeded { resolution } => Some(resolution),
                    _ => None,
                })
                .unwrap();
            assert!(
                (101..=250).contains(
                    &(cast.resource_after - game.resources["demo.resource.mana"].current)
                )
            );
        }
    }
}

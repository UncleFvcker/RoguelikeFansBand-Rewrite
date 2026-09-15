// SPDX-License-Identifier: MPL-2.0
use super::support::{
    choose_human_talent_if_pending, clear_monsters, dispatch_next, give_inventory_item,
};
use super::*;

fn bard() -> Game {
    let mut g = Game::new_with_build(925, "demo.build.bard").unwrap();
    clear_monsters(&mut g);
    g.apply_player_experience(g.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut g);
    g.player.position = Position { x: 10, y: 10 };
    for y in 5..=23 {
        for x in 5..=30 {
            let i = g.index(Position { x, y }).unwrap();
            g.terrain[i] = "demo.terrain.floor".into();
        }
    }
    g.glow.fill(true);
    g.reveal_current_visibility();
    for p in g.resources.values_mut() {
        p.current = p.maximum;
    }
    g.debug_ability_casts_succeed = true;
    g
}

fn learn(g: &mut Game, slot: usize) -> String {
    let book = [
        "apprentice-handbook",
        "minstrels-music",
        "harps-of-rivendell",
        "lays-of-beleriand",
    ][slot / 8];
    let id = g
        .content
        .ability_book(&format!("demo.ability-book.{book}"))
        .unwrap()
        .ability_ids[slot % 8]
        .clone();
    let item = format!("test.{book}");
    if !g.items.iter().any(|i| i.id == item) {
        give_inventory_item(g, &item, &format!("demo.item.{book}"));
    }
    g.study_player_ability(&item, &id).unwrap();
    id
}

fn cast(g: &mut Game, id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    g.resolve_player_ability(
        id,
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. })),
        "{id}: {events:?}"
    );
    events
}

#[test]
fn music_all_32_formal_songs_study_cast_and_resume() {
    for slot in 0..32 {
        let mut g = bard();
        let id = learn(&mut g, slot);
        let mut monster = g.generated_actor(
            "test.music-target".into(),
            "demo.actor.war-bear",
            Position { x: 12, y: 10 },
        );
        monster.nice = true;
        g.entities.push(monster);
        g.reveal_current_visibility();
        for p in g.resources.values_mut() {
            p.current = p.maximum;
        }
        cast(
            &mut g,
            &id,
            if matches!(slot, 2 | 22 | 30) {
                TargetSelection::Direction {
                    direction: Direction::East,
                }
            } else {
                TargetSelection::SelfTarget
            },
        );
        assert_eq!(
            g.music.spell.is_some(),
            crate::game::abilities::music::continuous(slot as u8)
        );
        g.reveal_current_visibility();
        let mut restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
        g.advance_music(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        restored
            .advance_music(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        // Direct rule calls bypass the command boundary's discovery update.
        g.reveal_current_visibility();
        restored.reveal_current_visibility();
        assert_eq!(g.state_hash(), restored.state_hash(), "slot {slot}");
        assert_eq!(g.rng, restored.rng);
        if matches!(slot, 2 | 9 | 13 | 20 | 22 | 27 | 30) {
            assert!(
                g.entities.is_empty() || g.entities[0].hp < g.entities[0].max_hp,
                "slot {slot}"
            );
        }
    }
}

#[test]
fn music_real_wait_charges_half_mana_and_exhaustion_removes_bonuses() {
    let mut g = bard();
    let id = learn(&mut g, 1);
    let before = g.player_derived_stats().armor_class.value;
    cast(&mut g, &id, TargetSelection::SelfTarget);
    assert_eq!(g.player_derived_stats().armor_class.value, before + 5);
    g.ability_progress.get_mut(&id).unwrap().proficiency = 1600;
    g.resources.get_mut("demo.resource.mana").unwrap().current = 2;
    let mut restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    for turn in 0..5 {
        dispatch_next(&mut g, GameCommand::Wait);
        dispatch_next(&mut restored, GameCommand::Wait);
        assert_eq!(g.state_hash(), restored.state_hash());
        if turn == 0 {
            assert_eq!(g.resources["demo.resource.mana"].current, 1);
            assert_eq!(g.resources["demo.resource.mana"].fraction, 1 << 31);
        }
    }
    assert!(g.music.spell.is_none());
    assert_eq!(g.player_derived_stats().armor_class.value, before);
}

#[test]
fn music_switch_failure_disruption_and_stop_preserve_independent_buffs() {
    let mut g = bard();
    let bless = learn(&mut g, 1);
    let hero = learn(&mut g, 7);
    cast(&mut g, &bless, TargetSelection::SelfTarget);
    let mut independent =
        crate::game::monster_combat::melee_status("rfb.status.blessed", 100, "test.independent")
            .status;
    independent.granted_modifiers.defense = 5;
    g.player.statuses.push(independent);
    let ac = g.player_derived_stats().armor_class.value;
    g.interrupt_music();
    assert_eq!(g.player_derived_stats().armor_class.value, ac);
    assert_eq!(g.player.energy_need, 100);
    // Prepare the ready boundary after interruption has charged its extra action.
    g.player.energy_need = 0;
    let mut restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    restored
        .advance_music(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .unwrap();
    assert!(restored.singing(1));
    cast(&mut restored, &hero, TargetSelection::SelfTarget);
    assert!(restored.singing(7));
    restored.debug_ability_casts_succeed = false;
    restored.player.statuses.push(
        crate::game::monster_combat::melee_status("rfb.status.stun", 100, "test.stun").status,
    );
    let mut failed = false;
    for seed in 1..200 {
        let mut trial = restored.clone();
        trial.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        trial
            .resolve_player_ability(
                &bless,
                TargetSelection::SelfTarget,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        if events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastFailed { .. }))
        {
            assert!(trial.music.spell.is_none());
            failed = true;
            break;
        }
    }
    assert!(failed);
    restored.stop_music();
    assert!(
        restored
            .player
            .statuses
            .iter()
            .any(|s| s.source_id.as_deref() == Some("test.independent"))
    );
}

#[test]
fn music_detection_progress_and_save_validation_follow_active_song() {
    let mut g = bard();
    let id = learn(&mut g, 8);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    for _ in 0..21 {
        g.advance_music(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
    }
    assert_eq!(g.music.beats, 19);
    let saved = g.to_save();
    assert!(Game::from_save(saved.clone(), Game::default_behavior_preferences()).is_ok());
    let mut bad = saved;
    bad.player.music.spell = Some(30);
    assert!(Game::from_save(bad, Game::default_behavior_preferences()).is_err());
}

#[test]
fn music_bard_birth_and_formal_books_are_available() {
    let g = Game::new_with_build(925, "demo.build.bard").unwrap();
    assert_eq!(
        g.casting_profile().unwrap().casting_attribute,
        rfb_content::CastingAttribute::Charisma
    );
    assert!(
        g.items
            .iter()
            .any(|i| i.kind_id == "demo.item.apprentice-handbook")
    );
    assert_eq!(
        g.effective_player_resistances().level(DamageType::Sound),
        ResistanceLevel::Resistant
    );
    for name in [
        "apprentice-handbook",
        "minstrels-music",
        "harps-of-rivendell",
        "lays-of-beleriand",
    ] {
        assert!(
            g.content
                .loot_table("demo.loot-table.base-items")
                .unwrap()
                .entries
                .iter()
                .any(|e| e.item_kind_id == format!("demo.item.{name}"))
        );
    }
}

#[test]
fn music_books_and_bard_harp_use_real_generation_pickup_and_save() {
    for (rank, name) in [
        "apprentice-handbook",
        "minstrels-music",
        "harps-of-rivendell",
        "lays-of-beleriand",
        "harp",
    ]
    .iter()
    .enumerate()
    {
        let mut g = bard();
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: g.current_floor_id.clone(),
            depth: [20, 30, 55, 85, 100][rank],
            source: LootSource::ItemUse {
                item_id: "test.music-pool".into(),
            },
        };
        let kind = format!("demo.item.{name}");
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| {
                        d.kind_id == kind
                            && (rank != 4 || d.intrinsic_properties.modifiers.charisma == 3)
                    })
            })
            .expect("formal book/poet harp generation");
        let item = g
            .commit_generated_item_draft(draft, ItemLocation::Ground(g.player.position))
            .unwrap();
        let id = item.id.clone();
        g.items.push(item);
        g.pick_up_item_at_player(Some(&id)).unwrap();
        if rank < 4 {
            let ability = g
                .content
                .ability_book(&format!("demo.ability-book.{name}"))
                .unwrap()
                .ability_ids[0]
                .clone();
            g.study_player_ability(&id, &ability).unwrap();
        } else {
            let before = g.effective_player_attributes().charisma;
            dispatch_next(
                &mut g,
                GameCommand::Equip {
                    item_id: id.clone(),
                    slot_id: None,
                },
            );
            assert!(g.effective_player_attributes().charisma > before);
            assert!(g.player_projectile_profile().is_none());
        }
        assert_eq!(
            g.state_hash(),
            Game::from_save(g.to_save(), g.behavior_preferences())
                .unwrap()
                .state_hash()
        );
    }
}

#[test]
fn music_invulnerability_exhaustion_pays_extra_action_before_ready_save() {
    let mut g = bard();
    let id = learn(&mut g, 31);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    assert!(g.player_has_status_kind(STATUS_INVULNERABILITY));
    g.resources.get_mut("demo.resource.mana").unwrap().current = 0;
    let mut ordinary = g.clone();
    ordinary.music.spell = None;
    let before = g.world_tick;
    dispatch_next(&mut ordinary, GameCommand::Wait);
    dispatch_next(&mut g, GameCommand::Wait);
    assert!(g.music.spell.is_none());
    assert!(g.player.energy_need <= 0);
    assert!(g.world_tick - before > ordinary.world_tick - before);
    assert!(Game::from_save(g.to_save(), g.behavior_preferences()).is_ok());
}

#[test]
fn music_wall_song_destroys_entered_rock_and_ground_items() {
    let mut g = bard();
    super::trump::dungeon(&mut g);
    let id = learn(&mut g, 16);
    cast(&mut g, &id, TargetSelection::SelfTarget);
    let target = Position { x: 11, y: 10 };
    let wall = g.index(target).unwrap();
    g.terrain[wall] = "demo.terrain.wall".into();
    dispatch_next(
        &mut g,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(g.player.position, target);
    assert_eq!(g.terrain[wall], "demo.terrain.floor");
    give_inventory_item(&mut g, "test.music-dagger", "demo.item.dagger");
    g.items.last_mut().unwrap().location = ItemLocation::Ground(target);
    dispatch_next(&mut g, GameCommand::Wait);
    assert!(!g.items.iter().any(|i| i.id == "test.music-dagger"));
    assert!(Game::from_save(g.to_save(), g.behavior_preferences()).is_ok());
}

#[test]
fn music_sound_pulse_scales_the_actual_damage_roll_once() {
    let mut base = bard();
    let id = learn(&mut base, 13);
    let monster = base.generated_actor(
        "test.sound-target".into(),
        "demo.actor.war-bear",
        Position { x: 12, y: 10 },
    );
    base.entities.push(monster);
    base.reveal_current_visibility();
    cast(&mut base, &id, TargetSelection::SelfTarget);
    let mut enhanced = base.clone();
    let mut power =
        crate::game::monster_combat::melee_status("test.spell-power", 100, "test.music").status;
    power.granted_modifiers.spell_power_bonus = 4;
    enhanced.player.statuses.push(power);
    let damage = |g: &mut Game| {
        let mut events = Vec::new();
        g.advance_music(&mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        events
            .into_iter()
            .find_map(|e| match e {
                DomainEvent::AbilityVisibleDamage { resolution, .. } => {
                    Some(resolution.base_raw_damage)
                }
                _ => None,
            })
            .unwrap()
    };
    let raw = damage(&mut base);
    assert_eq!(
        damage(&mut enhanced) as u64,
        crate::game::ability_scaling::spell_power_value(raw as u64, 4)
    );
}

#[test]
fn music_status_projection_and_stop_power_follow_the_player_only() {
    let mut g = bard();
    let song = learn(&mut g, 1);
    let monster = g.generated_actor(
        "test.song-observer".into(),
        "demo.actor.war-bear",
        Position { x: 12, y: 10 },
    );
    g.entities.push(monster);
    g.reveal_current_visibility();
    cast(&mut g, &song, TargetSelection::SelfTarget);
    let snapshot = g.snapshot();
    assert!(
        snapshot
            .player
            .statuses
            .iter()
            .any(|s| s.kind_id == "rfb.status.music")
    );
    assert!(
        snapshot
            .entities
            .iter()
            .all(|e| e.statuses.iter().all(|s| s.kind_id != "rfb.status.music"))
    );
    assert!(
        !snapshot
            .player
            .abilities
            .iter()
            .find(|a| a.id == song)
            .unwrap()
            .can_forget
    );
    let mana = g.resources["demo.resource.mana"].current;
    cast(
        &mut g,
        "demo.ability.bard-stop-singing",
        TargetSelection::SelfTarget,
    );
    assert!(g.music.spell.is_none());
    assert_eq!(g.resources["demo.resource.mana"].current, mana);
    assert_eq!(
        g.ability_state_unavailable_reason("demo.ability.bard-stop-singing"),
        Some("no-active-song")
    );
}

#[test]
fn music_world_contortion_uses_distance_reduced_power_and_moves_the_target() {
    let mut g = bard();
    let song = learn(&mut g, 19);
    let position = Position { x: 12, y: 10 };
    let monster = g.generated_actor("test.contortion".into(), "demo.actor.war-bear", position);
    g.entities.push(monster);
    g.reveal_current_visibility();
    let events = cast(&mut g, &song, TargetSelection::SelfTarget);
    assert!(events.iter().any(|event| matches!(event,
        DomainEvent::AbilityEffectsResolved { resolution, .. } if resolution.effects.iter().any(|e|
            matches!(e, AbilityEffectResolutionDto::TeleportAway { power: 51, resisted: false, from, to: Some(to), .. } if *from == position && *to != position)
        )
    )));
}

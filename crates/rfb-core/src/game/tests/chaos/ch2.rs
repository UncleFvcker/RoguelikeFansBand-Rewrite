// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::ability_scaling::spell_power_value;
use rfb_protocol::{AbilityEffectResolutionDto, VirtueDto, VirtueKindDto};

fn east() -> TargetSelection {
    TargetSelection::Direction {
        direction: Direction::East,
    }
}

pub(super) fn dungeon(game: &mut Game) {
    let definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|f| f.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    game.dungeon_states
        .get_mut("demo.dungeon.warrens")
        .unwrap()
        .next_instance_ordinal = 1;
    let floor = game
        .generate_procedural_floor(&definition, Some("demo.dungeon.warrens.instance.1".into()))
        .unwrap();
    let carried = game
        .items
        .iter()
        .filter(|item| !matches!(item.location, ItemLocation::Ground(_)))
        .cloned()
        .collect();
    game.activate_floor(floor, carried);
    clear_monsters(game);
    game.player.position = Position { x: 10, y: 10 };
    for y in 6..=17 {
        for x in 6..=17 {
            let index = game.index(Position { x, y }).unwrap();
            game.terrain[index] = "demo.terrain.floor".into();
        }
    }
    game.glow.fill(true);
    game.reveal_current_visibility();
}

fn chance(game: &mut Game, value: i16) {
    let index = game
        .virtues
        .iter()
        .position(|v| v.kind == VirtueKindDto::Chance)
        .unwrap_or(0);
    game.virtues[index] = VirtueDto {
        kind: VirtueKindDto::Chance,
        value,
    };
}

fn branch_index(events: &[DomainEvent]) -> Option<u16> {
    events.iter().find_map(|event| match event {
        DomainEvent::AbilityEffectsResolved { resolution, .. } => {
            resolution.effects.iter().find_map(|e| {
                if let AbilityEffectResolutionDto::RandomChoice { branch_index, .. } = e {
                    Some(*branch_index)
                } else {
                    None
                }
            })
        }
        _ => None,
    })
}

// Search only the real cast's seed. Then execute the same command from a save;
// the branch, resources and effect are never substituted by the fixture.
fn wonder_branch(base: &Game, wanted: u16) -> (Game, Vec<DomainEvent>) {
    for seed in 0..8192 {
        let mut trial = base.clone();
        trial.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        trial
            .resolve_player_ability(
                "demo.ability.chaos-wonder",
                east(),
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        if branch_index(&events) != Some(wanted) {
            continue;
        }
        let mut before = base.clone();
        before.rng = RfbRng::seeded(seed);
        let mut restored =
            Game::from_save_with_content(before.to_save(), before.content.clone()).unwrap();
        let mut resumed = Vec::new();
        restored
            .resolve_player_ability(
                "demo.ability.chaos-wonder",
                east(),
                &mut resumed,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(branch_index(&resumed), Some(wanted));
        assert_eq!(trial.state_hash(), restored.state_hash());
        assert_eq!(trial.rng, restored.rng);
        return (restored, resumed);
    }
    panic!("no real Wonder seed for branch {wanted}");
}

#[test]
fn second_book_is_obtainable_and_its_damage_spells_hit_after_restore() {
    let mut game = caster("high-mage", 40);
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 20,
        source: LootSource::ItemUse {
            item_id: "test.ordinary-pool".into(),
        },
    };
    let draft = (0..16_384)
        .find_map(|_| {
            game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .filter(|d| d.kind_id == "demo.item.chaos-mastery")
        })
        .expect("second book in ordinary pool");
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let item_id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&item_id)).unwrap();
    assert!(game.item_knowledge["demo.item.chaos-mastery"].found_count > 0);
    for slug in ["chaos-bolt", "doom-bolt", "fireball", "invoke-logrus"] {
        let mut trial = game.clone();
        let id = learn(&mut trial, slug);
        target(&mut trial);
        cast_saved(&mut trial, &id, east());
        assert!(trial.entities[0].hp < 10_000, "{slug} actually hits");
    }
}

#[test]
fn sonic_boom_doubles_after_spell_power_then_attenuates_and_teleport_is_a_beam() {
    for class in ["mage", "high-mage"] {
        let mut game = caster(class, 40);
        let id = learn(&mut game, "sonic-boom");
        target(&mut game);
        // Distance two: shared ball attenuation is (damage + distance)/(distance + 1).
        let profile = game.casting_profile().unwrap();
        let bonus = profile.spell_damage_bonus_base
            + profile.spell_damage_bonus_per_level
                * (40 / u16::from(profile.spell_damage_bonus_level_divisor));
        let raw = spell_power_value(
            u64::from(90 + bonus),
            game.effective_player_spell_power_bonus(),
        ) as i32
            * 2;
        let hp = game.entities[0].hp;
        cast_saved(&mut game, &id, TargetSelection::SelfTarget);
        assert_eq!(hp - game.entities[0].hp, (raw + 2) / 3);
    }
    let mut game = caster("high-mage", 40);
    let id = learn(&mut game, "teleport-other");
    target(&mut game);
    let mut second = game.entities[0].clone();
    second.id = "test.second-beam-target".into();
    second.position.x += 2;
    game.entities.push(second);
    let positions: Vec<_> = game.entities.iter().map(|a| a.position).collect();
    cast_saved(&mut game, &id, east());
    assert!(
        game.entities
            .iter()
            .zip(positions)
            .all(|(a, before)| a.position != before)
    );
}

#[test]
fn destruction_changes_real_terrain_and_retains_resistance_on_restore() {
    let mut game = caster("high-mage", 40);
    let id = learn(&mut game, "word-of-destruction");
    dungeon(&mut game);
    target(&mut game);
    game.entities[0].no_destruction = true;
    let protected_position = game.entities[0].position;
    let vein = game.index(Position { x: 9, y: 10 }).unwrap();
    game.terrain[vein] = "demo.terrain.magma-hidden-treasure".into();
    let gold_ids = game
        .gold_piles
        .iter()
        .map(|p| p.id.clone())
        .collect::<BTreeSet<_>>();
    let before = game.terrain.clone();
    cast_saved(&mut game, &id, TargetSelection::SelfTarget);
    assert_ne!(game.terrain, before);
    assert!(game.entities[0].no_destruction);
    assert_eq!(game.terrain_at(protected_position), "demo.terrain.floor");
    assert!(
        game.gold_piles
            .iter()
            .all(|pile| gold_ids.contains(&pile.id)),
        "destruction must not generate mining gold inside replacement rock"
    );
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert!(restored.entities[0].no_destruction);
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn wonder_executes_every_branch_with_saved_rng_and_monster_aid_is_real() {
    let mut base = caster("high-mage", 40);
    learn(&mut base, "wonder");
    dungeon(&mut base);
    target(&mut base);
    base.entities[0].hp = 5000;
    base.player.hp = (base.player.hp - 50).max(1);
    for pool in base.resources.values_mut() {
        pool.current = pool.maximum;
    }
    for index in 0..23 {
        base.glow.fill(index != 7);
        chance(&mut base, if index == 0 { -125 } else { 125 });
        let (mut game, events) = wonder_branch(&base, index);
        match index {
            0 => {
                assert_eq!(game.entities[0].hp, 10_000);
                assert_eq!(game.entities.len(), 2);
                assert!(game.entities[1].cloned);
                assert_ne!(game.entities[0].id, game.entities[1].id);
                let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
                assert!(restored.entities[1].cloned);
            }
            1 => assert!(game.entities[0].statuses.iter().any(|s| s.kind_id == crate::effect::STATUS_HASTE && s.remaining_ticks == 100)),
            2 => assert!(game.entities[0].hp > 5000 && game.entities[0].hp <= 5029),
            3 => assert!(events.iter().any(|e| matches!(e, DomainEvent::AbilityEffectsResolved { resolution, .. }
                if resolution.effects.iter().any(|e| matches!(e, AbilityEffectResolutionDto::PolymorphTarget { .. }))))),
            5 => assert!(events.iter().any(|e| matches!(e, DomainEvent::AbilityEffectsResolved { resolution, .. }
                if resolution.effects.iter().any(|e| matches!(e, AbilityEffectResolutionDto::ApplyStatus { .. }))))),
            // Light line only hurts light-vulnerable monsters, but still lights the actual path.
            7 => assert!(game.glow[game.index(base.entities[0].position).unwrap()]),
            12 | 17 => {
                assert!(game.entities[0].hp < 5000);
                assert_eq!(game.player.hp, base.player.hp, "Wonder drain does not heal");
            }
            18 => assert_ne!(game.terrain, base.terrain),
            19 => assert_ne!(game.terrain, base.terrain),
            20 => {
                assert!(game.pending_ability_glyph.is_some());
                game.resolve_pending_ability_glyph(Some("q".into()), &mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new()).unwrap();
                assert!(game.pending_ability_glyph.is_none());
            }
            22 => {
                assert!(game.player.hp > base.player.hp);
                assert!(game.entities[0].hp < 5000);
            }
            _ => assert!(game.entities[0].hp < 5000, "Wonder branch {index} deals actual damage"),
        }
    }
}

#[test]
fn wonder_glyph_choice_cannot_reroll_or_refund_and_survives_save() {
    let mut base = caster("mage", 40);
    learn(&mut base, "wonder");
    dungeon(&mut base);
    target(&mut base);
    chance(&mut base, 125);
    let (pending, _) = wonder_branch(&base, 20);
    let mp = pending.resources["demo.resource.mana"].current;
    let cast_count = pending.ability_progress["demo.ability.chaos-wonder"].cast_count;
    assert!(mp < base.resources["demo.resource.mana"].current);
    let mut invalid = pending.clone();
    let before = invalid.state_hash();
    assert!(
        invalid
            .resolve_pending_ability_glyph(
                Some("qq".into()),
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new()
            )
            .is_err()
    );
    assert_eq!(invalid.state_hash(), before);
    for glyph in [None, Some("q".to_owned())] {
        let mut direct = pending.clone();
        let mut restored =
            Game::from_save_with_content(pending.to_save(), pending.content.clone()).unwrap();
        for game in [&mut direct, &mut restored] {
            game.resolve_pending_ability_glyph(
                glyph.clone(),
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(game.resources["demo.resource.mana"].current, mp);
            assert_eq!(
                game.ability_progress["demo.ability.chaos-wonder"].cast_count,
                cast_count
            );
            assert!(game.pending_ability_glyph.is_none());
        }
        assert_eq!(direct.state_hash(), restored.state_hash());
        assert_eq!(direct.rng, restored.rng);
        if glyph.is_none() {
            assert_eq!(direct.entities, pending.entities);
        }
    }
    let mut forged = pending.to_save();
    forged
        .player
        .pending_ability_glyph
        .as_mut()
        .unwrap()
        .cast_resolution
        .ability_id = "demo.ability.chaos-fireball".into();
    assert!(Game::from_save_with_content(forged, pending.content.clone()).is_err());
}

#[test]
fn wonder_dispatch_waits_for_glyph_and_cancellation_spends_one_turn() {
    use crate::game::tests::support::{command, dispatch_next};
    let mut base = caster("mage", 40);
    let id = learn(&mut base, "wonder");
    chance(&mut base, 125);
    let before_turn = base.turn;
    let before_tick = base.world_tick;
    let mut pending = (0..8192)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            dispatch_next(
                &mut game,
                GameCommand::CastAbility {
                    ability_id: id.clone(),
                    target: east(),
                },
            );
            game.pending_ability_glyph.is_some().then_some(game)
        })
        .expect("a real dispatched Wonder reaches the glyph branch");
    assert_eq!(pending.turn, before_turn);
    assert_eq!(pending.world_tick, before_tick);
    let snapshot = pending.snapshot();
    assert!(matches!(
        pending.dispatch(command(
            snapshot.last_command_seq + 1,
            snapshot.revision,
            GameCommand::Wait
        )),
        Err(CoreError::AbilityGlyphRequired)
    ));
    let mut restored =
        Game::from_save_with_content(pending.to_save(), pending.content.clone()).unwrap();
    for game in [&mut pending, &mut restored] {
        dispatch_next(game, GameCommand::ResolveAbilityGlyph { glyph: None });
        assert_eq!(game.turn, before_turn + 1);
        assert!(game.pending_ability_glyph.is_none());
    }
    assert_eq!(pending.state_hash(), restored.state_hash());
    assert_eq!(pending.rng, restored.rng);
}

#[test]
fn wonder_uses_bonus_dice_and_class_beam_chance_without_death_roll_power() {
    let game = caster("high-mage", 40);
    let mut profile = game.casting_profile().unwrap().clone();
    profile.spell_damage_bonus_base = 7;
    profile.spell_damage_bonus_per_level = 0;
    let mut ability = game.effective_casting_ability(
        &profile,
        game.content.ability("demo.ability.chaos-wonder").unwrap(),
    );
    Game::apply_player_level_scaling(&mut ability, 40);
    Game::apply_casting_profile_effect_scaling(&profile, &mut ability, 40);
    game.apply_casting_profile_damage_bonus(&profile, &mut ability, 40);
    Game::apply_player_spell_power(&mut ability, 3);
    assert!(ability.spell_power_fields.is_empty());
    let AbilityEffectDefinition::RandomChoice { branches, .. } = &ability.effect else {
        panic!()
    };
    let beam = (40 * i32::from(profile.beam_chance_level_multiplier)
        / i32::from(profile.beam_chance_level_divisor)
        + i32::from(profile.beam_chance_bonus)) as u8;
    for (index, dice, modifier) in [
        (4, 17, 10),
        (8, 18, 10),
        (9, 20, 10),
        (10, 21, 0),
        (11, 23, 0),
    ] {
        let AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice,
            damage_bonus,
            beam_chance_percent,
            ..
        } = *branches[index].effect
        else {
            panic!()
        };
        assert_eq!(damage_dice, dice);
        assert_eq!(damage_bonus, 0);
        assert_eq!(beam_chance_percent, beam.saturating_sub(modifier));
    }
    let AbilityEffectDefinition::VisibleDamage { damage_bonus, .. } = *branches[21].effect else {
        panic!()
    };
    assert_eq!(
        damage_bonus, 120,
        "the 120-damage dispel branch has no to_d_spell bonus"
    );
}

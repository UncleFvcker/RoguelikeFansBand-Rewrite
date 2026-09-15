// SPDX-License-Identifier: MPL-2.0
use super::support::{clear_monsters, give_inventory_item};
use super::*;

fn caster(race: &str) -> Game {
    let mut g = Game::new_with_build_race_and_name(
        925,
        "demo.build.high-mage-law",
        race,
        "Law",
        Game::default_behavior_preferences(),
    )
    .unwrap();
    clear_monsters(&mut g);
    g.apply_player_experience(g.experience_required_for_level(50), &mut Vec::new());
    super::support::choose_human_talent_if_pending(&mut g);
    g.progress.attributes.intelligence = g.progress.attribute_potentials.intelligence;
    g.progress.maximum_attributes.intelligence = g.progress.attributes.intelligence;
    g.refresh_player_ability_state();
    g.player.hp = g.effective_player_max_hp();
    g.player.position = Position { x: 10, y: 10 };
    for y in 5..=23 {
        for x in 5..=23 {
            let i = g.index(Position { x, y }).unwrap();
            g.terrain[i] = "demo.terrain.floor".into();
        }
    }
    g.glow.fill(true);
    g.reveal_current_visibility();
    g
}

fn learn(g: &mut Game, slot: usize) -> String {
    let book = [
        "attractions-of-law",
        "obstacle-coursebook",
        "building-alternative-realities",
        "acquiris-quodcumque-rapis",
    ][slot / 8];
    let id = g
        .content
        .ability_book(&format!("demo.ability-book.{book}"))
        .unwrap()
        .ability_ids[slot % 8]
        .clone();
    let item = format!("test.{book}");
    give_inventory_item(g, &item, &format!("demo.item.{book}"));
    g.study_player_ability(&item, &id).unwrap();
    id
}

fn cast(g: &mut Game, id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    g.reveal_current_visibility();
    for r in g.resources.values_mut() {
        r.current = r.maximum;
    }
    for seed in 0..512 {
        let mut trial = g.clone();
        trial.rng = RfbRng::seeded(0x9e3779b97f4a7c15_u64.wrapping_mul(seed + 1));
        let mut restored = Game::from_save(trial.to_save(), trial.behavior_preferences()).unwrap();
        let mut events = Vec::new();
        trial
            .resolve_player_ability(
                id,
                target.clone(),
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, DomainEvent::AbilityTargetUnavailable { .. })),
            "{id}: {events:?}"
        );
        restored
            .resolve_player_ability(
                id,
                target.clone(),
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(trial.state_hash(), restored.state_hash(), "{id}");
        assert_eq!(trial.rng, restored.rng);
        if events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
        {
            trial.reveal_current_visibility();
            Game::from_save(trial.to_save(), trial.behavior_preferences()).unwrap();
            *g = trial;
            return events;
        }
    }
    panic!("no successful cast {id}")
}

#[test]
fn law_all_32_formal_spells_study_cast_and_resume() {
    for slot in 0..32 {
        let mut g = caster("rfb-legacy.race.vampire");
        let id = learn(&mut g, slot);
        let mut actor = g.generated_actor(
            "test.law-target".into(),
            if slot == 19 {
                "demo.actor.skeleton-orc"
            } else {
                "demo.actor.war-bear"
            },
            Position { x: 12, y: 10 },
        );
        actor.nice = true;
        g.entities.push(actor);
        g.reveal_current_visibility();
        let target = if matches!(slot, 5 | 7 | 9 | 10 | 11 | 16 | 22 | 27 | 29) {
            TargetSelection::Entity {
                entity_id: "test.law-target".into(),
            }
        } else if slot == 6 {
            give_inventory_item(&mut g, "test.lore", "demo.item.dagger");
            TargetSelection::Item {
                item_id: "test.lore".into(),
            }
        } else {
            TargetSelection::SelfTarget
        };
        let before = g.player_derived_stats();
        let events = cast(&mut g, &id, target);
        if matches!(slot, 4 | 12 | 15 | 17) {
            assert_ne!(
                g.terrain[g.index(g.player.position).unwrap()],
                "demo.terrain.floor"
            );
        }
        if slot == 21 {
            assert!(g.player_has_status_kind("rfb.status.law-spin"));
            assert_eq!(
                g.player
                    .statuses
                    .iter()
                    .find(|s| s.kind_id == "rfb.status.law-spin")
                    .unwrap()
                    .granted_resistances[&DamageType::Nether],
                ResistanceLevel::Resistant
            );
        }
        if slot == 25 {
            assert_eq!(
                g.player_derived_stats().stealth_skill.value - before.stealth_skill.value,
                13
            );
        }
        if matches!(slot, 19 | 22 | 27) {
            assert!(
                g.entities.is_empty() || g.entities[0].hp < g.entities[0].max_hp,
                "{id}: {events:?}"
            );
        }
        if slot == 29 {
            assert!(chebyshev_distance(g.player.position, g.entities[0].position) <= 1);
        }
        if slot == 23 {
            assert_eq!(g.reality_change_ticks, 1);
        }
    }
}

#[test]
fn law_aptitude_changes_real_learning_and_parameters() {
    for (race, level, cost) in [
        ("demo.race.rfb-human", 14, 7),
        ("rfb-legacy.race.ent", 13, 6),
        ("rfb-legacy.race.vampire", 13, 6),
    ] {
        let g = caster(race);
        let ability = g.effective_casting_ability(
            g.character_definitions()
                .unwrap()
                .2
                .casting_profile
                .as_ref()
                .unwrap(),
            g.content.ability("demo.ability.law-dig").unwrap(),
        );
        let p = ability.player.unwrap();
        assert_eq!(p.minimum_level, level);
        assert_eq!(p.resource_cost, cost);
        assert_eq!(p.base_failure_percent, 40);
        let ability = g.effective_casting_ability(
            g.character_definitions()
                .unwrap()
                .2
                .casting_profile
                .as_ref()
                .unwrap(),
            g.content.ability("demo.ability.law-alter-reality").unwrap(),
        );
        assert_eq!(
            ability.player.unwrap().minimum_level,
            if race.ends_with("human") { 99 } else { 50 }
        );
    }
}

#[test]
fn law_traps_are_single_use_and_resume_actual_monster_entry() {
    for terrain in ["law-basic-trap", "law-expert-trap", "law-semicolon"] {
        for seed in 0..32 {
            let mut g = caster("rfb-legacy.race.vampire");
            let p = Position { x: 12, y: 10 };
            let i = g.index(p).unwrap();
            g.terrain[i] = format!("demo.terrain.{terrain}");
            let actor = g.generated_actor("test.trap-target".into(), "demo.actor.war-bear", p);
            g.entities.push(actor);
            g.rng = RfbRng::seeded(seed);
            let mut restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
            for current in [&mut g, &mut restored] {
                current
                    .trigger_actor_trap(
                        0,
                        p,
                        &mut Vec::new(),
                        &mut BTreeSet::new(),
                        &mut Vec::new(),
                    )
                    .unwrap();
            }
            assert_eq!(g.state_hash(), restored.state_hash());
            assert_eq!(g.rng, restored.rng);
            assert_ne!(g.terrain[i], format!("demo.terrain.{terrain}"));
            g.reveal_current_visibility();
            Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
        }
    }
}

#[test]
fn law_all_29_builds_learn_cast_and_resume() {
    let content = load_built_in_content().unwrap();
    let builds = content
        .builds()
        .filter(|b| {
            b.first_realm_id.as_deref() == Some("law")
                || b.second_realm_id.as_deref() == Some("law")
        })
        .map(|b| b.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(builds.len(), 29);
    for build in builds {
        let mut g = super::existing_realms::prepared(&build);
        let book = g
            .items
            .iter()
            .find(|i| i.kind_id == "demo.item.attractions-of-law")
            .unwrap()
            .id
            .clone();
        let ability = "demo.ability.law-detect-money";
        for _ in 0..8 {
            if g.learned_abilities.contains(ability) {
                break;
            }
            if g.casting_profile().unwrap().study_mode
                == rfb_content::CastingStudyMode::DivineRandom
            {
                super::support::dispatch_next(
                    &mut g,
                    GameCommand::StudyPrayer {
                        book_item_id: book.clone(),
                    },
                );
            } else {
                g.study_player_ability(&book, ability).unwrap();
            }
        }
        assert!(g.learned_abilities.contains(ability), "{build}");
        cast(&mut g, ability, TargetSelection::SelfTarget);
    }
}

#[test]
fn law_four_books_use_the_ordinary_pool_pickup_study_and_save() {
    for (rank, book) in [
        "attractions-of-law",
        "obstacle-coursebook",
        "building-alternative-realities",
        "acquiris-quodcumque-rapis",
    ]
    .iter()
    .enumerate()
    {
        let mut g = caster("rfb-legacy.race.vampire");
        let kind = format!("demo.item.{book}");
        let context = LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: g.current_floor_id.clone(),
            depth: [20, 30, 55, 85][rank],
            source: LootSource::ItemUse {
                item_id: "test.law-pool".into(),
            },
        };
        let draft = (0..65536)
            .find_map(|_| {
                g.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                    .filter(|d| d.kind_id == kind)
            })
            .unwrap_or_else(|| panic!("book {book} not generated"));
        let item = g
            .commit_generated_item_draft(draft, ItemLocation::Ground(g.player.position))
            .unwrap();
        let id = item.id.clone();
        g.items.push(item);
        g.pick_up_item_at_player(Some(&id)).unwrap();
        let spell = g
            .content
            .ability_book(&format!("demo.ability-book.{book}"))
            .unwrap()
            .ability_ids[0]
            .clone();
        g.study_player_ability(&id, &spell).unwrap();
        assert!(g.learned_abilities.contains(&spell));
        assert_eq!(
            g.content
                .item(&kind)
                .unwrap()
                .rfb_base_kind
                .as_ref()
                .unwrap()
                .source_index,
            548 + rank as u32
        );
        assert_eq!(
            g.state_hash(),
            Game::from_save(g.to_save(), g.behavior_preferences())
                .unwrap()
                .state_hash()
        );
    }
}

#[test]
fn law_getaway_confirmation_is_paid_saved_and_resumed_once() {
    let mut base = caster("rfb-legacy.race.vampire");
    let id = learn(&mut base, 18);
    super::trump::dungeon(&mut base);
    for r in base.resources.values_mut() {
        r.current = r.maximum;
    }
    let pending = (0..512)
        .find_map(|seed| {
            let mut g = base.clone();
            g.rng = RfbRng::seeded(seed);
            super::support::dispatch_next(
                &mut g,
                GameCommand::CastAbility {
                    ability_id: id.clone(),
                    target: TargetSelection::SelfTarget,
                },
            );
            matches!(
                g.duelist_prompt(),
                Some(rfb_protocol::DuelistPromptDto::LawEscape)
            )
            .then_some(g)
        })
        .expect("level-teleport branch");
    assert!(
        pending.resources["demo.resource.mana"].current
            < base.resources["demo.resource.mana"].current
    );
    for accepted in [false, true] {
        let mut g = pending.clone();
        let mut restored = Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
        for current in [&mut g, &mut restored] {
            super::support::dispatch_next(
                current,
                GameCommand::ResolveDuelistChoice {
                    choice: rfb_protocol::DuelistChoiceDto::Confirm { accepted },
                },
            );
        }
        assert_eq!(g.state_hash(), restored.state_hash());
        assert_eq!(g.rng, restored.rng);
        assert!(g.duelist_prompt().is_none());
        assert_eq!(g.current_floor_id != pending.current_floor_id, accepted);
        Game::from_save(g.to_save(), g.behavior_preferences()).unwrap();
    }
}

#[test]
fn law_spin_changes_actual_charm_power_and_expiry_restores_it() {
    let mut g = caster("rfb-legacy.race.vampire");
    let id = learn(&mut g, 21);
    let actor = g.generated_actor(
        "test.charm-target".into(),
        "demo.actor.war-bear",
        Position { x: 12, y: 10 },
    );
    g.entities.push(actor);
    let mut normal = g.clone();
    let r = normal.resolve_psychic_charm(0, 0, 40);
    let rfb_protocol::AbilityEffectResolutionDto::Control { power: before, .. } = r else {
        panic!("charm result")
    };
    cast(&mut g, &id, TargetSelection::SelfTarget);
    let mut spinning = g.clone();
    let r = spinning.resolve_psychic_charm(0, 0, 40);
    let rfb_protocol::AbilityEffectResolutionDto::Control { power: after, .. } = r else {
        panic!("charm result")
    };
    assert_eq!(after, before + 25.max(before * 2 / 5));
    g.player
        .statuses
        .retain(|s| s.kind_id != "rfb.status.law-spin");
    let r = g.resolve_psychic_charm(0, 0, 40);
    assert!(
        matches!(r,rfb_protocol::AbilityEffectResolutionDto::Control{power,..} if power==before)
    );
}

#[test]
fn law_unholy_rage_heals_75_once_and_uses_its_own_duration() {
    let mut g = caster("rfb-legacy.race.vampire");
    let id = learn(&mut g, 28);
    g.player.hp = 1;
    cast(&mut g, &id, TargetSelection::SelfTarget);
    assert_eq!(g.player.hp, 76);
    let status = g
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == "rfb.status.berserk")
        .unwrap();
    assert!((21..=40).contains(&status.remaining_ticks));
    assert_eq!(status.granted_equipment_bonuses.melee_damage, 13);
}

#[test]
fn law_dig_deep_destroys_real_ground_items_and_rock() {
    let mut g = caster("rfb-legacy.race.vampire");
    let id = learn(&mut g, 27);
    let mut power =
        crate::game::monster_combat::melee_status("test.spell-power", 100, "test.power");
    power.status.granted_modifiers.spell_power_bonus = 8;
    g.player.statuses.push(power.status);
    let projected = g
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|a| a.id == id)
        .unwrap();
    let range = projected.target_spec.range;
    assert!(range > 18);
    assert!(projected.beam_damage);
    for x in 11..=11 + i32::from(range) {
        let i = g.index(Position { x, y: 10 }).unwrap();
        g.terrain[i] = "demo.terrain.floor".into();
    }
    give_inventory_item(&mut g, "test.disintegrate", "demo.item.dagger");
    let edge = Position {
        x: 10 + i32::from(range),
        y: 10,
    };
    g.items.last_mut().unwrap().location = ItemLocation::Ground(edge);
    let wall = g
        .index(Position {
            x: edge.x - 1,
            y: 10,
        })
        .unwrap();
    g.terrain[wall] = "demo.terrain.wall".into();
    let beyond = g
        .index(Position {
            x: edge.x + 1,
            y: 10,
        })
        .unwrap();
    g.terrain[beyond] = "demo.terrain.wall".into();
    cast(
        &mut g,
        &id,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert!(!g.items.iter().any(|i| i.id == "test.disintegrate"));
    assert_eq!(g.terrain[wall], "demo.terrain.floor");
    assert_eq!(g.terrain[beyond], "demo.terrain.wall");
}

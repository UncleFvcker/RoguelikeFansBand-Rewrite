// SPDX-License-Identifier: MPL-2.0
use super::*;

const ODIN: &str = "demo.actor.odin-the-all-father";
const VIDARR: &str = "demo.actor.vidarr-the-silent-avenger";
const FRIGG: &str = "demo.actor.frigg-queen-of-asgard";
const FREYJA: &str = "demo.actor.freyja-lady-of-the-slain";
const CHEST: &str = "demo.item.large-wooden-chest";

fn death(game: &mut Game, id: &str, rewards: bool) -> Vec<DomainEvent> {
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    let mut events = Vec::new();
    if rewards {
        let target_kind_id = game.entities[index].kind_id.clone();
        game.resolve_actor_death_without_credit(
            index,
            DomainEvent::EntityDiedFromStatus {
                target_kind_id,
                status_kind_id: STATUS_POISON.into(),
                damage: crate::effect::resolve_damage(
                    crate::effect::DamagePacket::new(1, crate::resistance::DamageType::Poison),
                    ResistanceLevel::Normal,
                ),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    } else {
        game.resolve_actor_death_without_rewards(
            index,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    }
    events
}

fn chest(game: &mut Game, difficulty: i16) -> String {
    let mut draft = game.fixed_item_draft(&context(), CHEST.into());
    assert!((1..=15).contains(&draft.chest.unwrap().difficulty));
    assert_eq!(draft.chest.unwrap().opening_depth, 105);
    draft.chest.as_mut().unwrap().difficulty = difficulty;
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(Position { x: 11, y: 10 }))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.mogaminator.enabled = false;
    game.mark_item_instances_discovered(&[id.clone()]);
    id
}

fn unlock_seed() -> u64 {
    (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 2)
        .unwrap()
}

#[test]
fn asgard_alternative_is_chosen_before_probability_luck_and_generated_exclusion() {
    let mut base = game_with_actor_definition(71, FREYJA, |actor| {
        actor.death_drop = None;
        actor.loot_table_id = None;
    });
    clear_monsters(&mut base);
    base.items.clear();
    base.player.position = Position { x: 10, y: 10 };
    replace_terrain(&mut base, Position { x: 11, y: 10 }, "demo.terrain.floor");
    let actor = base.generated_actor("test.freyja".into(), FREYJA, Position { x: 11, y: 10 });
    for alternate in [false, true] {
        let (kind, ordinary_chance, unlucky_chance) = if alternate {
            ("demo.item.brisingamen", 40, 30)
        } else {
            ("demo.item.freyja", 30, 23)
        };
        let seed = (0..10000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                let branch = rng.bounded(2) != 0;
                let roll = rng.bounded(100);
                branch == alternate && roll >= unlucky_chance && roll < ordinary_chance
            })
            .unwrap();
        let mut ordinary = base.clone();
        ordinary.rng = RfbRng::seeded(seed);
        assert_eq!(
            ordinary.generate_death_loot(&actor).unwrap().0[0].kind_id,
            kind
        );
        let mut unlucky = base.clone();
        unlucky
            .progress
            .active_mutation_ids
            .insert("rfb.mutation.bad-luck".into());
        unlucky.rng = RfbRng::seeded(seed);
        assert!(unlucky.generate_death_loot(&actor).unwrap().0.is_empty());
        let mut expected = RfbRng::seeded(seed);
        expected.bounded(2);
        expected.bounded(100);
        assert_eq!(unlucky.rng, expected);
        let mut already_generated = base.clone();
        already_generated.generated_artifact_ids.insert(kind.into());
        already_generated.rng = RfbRng::seeded(seed);
        assert!(
            already_generated
                .generate_death_loot(&actor)
                .unwrap()
                .0
                .is_empty(),
            "selected generated artifact must not fall back to the other branch"
        );
        assert_eq!(already_generated.rng, expected);
    }
    let mut pet = actor;
    pet.controller_id = Some(base.player.id.clone());
    let before = base.rng.clone();
    assert!(base.generate_death_loot(&pet).unwrap().0.is_empty());
    assert_eq!(base.rng, before);
}

#[test]
fn asgard_odin_actual_death_summons_once_and_unique_lifetime_survives_save() {
    let mut game = game();
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.bad-luck".into());
    game.push_generated_actor("test.odin".into(), ODIN, Position { x: 12, y: 10 });
    death(&mut game, "test.odin", true);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.runespear")
            .count(),
        1
    );
    assert!(
        game.items
            .iter()
            .any(|item| item.kind_id != "demo.item.runespear")
    );
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == VIDARR)
            .count(),
        1
    );
    let avenger = game
        .entities
        .iter()
        .find(|actor| actor.kind_id == VIDARR)
        .unwrap()
        .id
        .clone();
    death(&mut game, &avenger, false);
    assert!(!game.unique_actor_kind_is_available(VIDARR));
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.defeated_limited_actor_counts[VIDARR], 1);
    assert_eq!(restored.defeated_limited_actor_counts[ODIN], 1);
}

#[test]
fn asgard_odin_respects_existing_dead_and_full_map_avenger_limits_and_pet_control() {
    let base = game();
    for blocked in ["existing", "dead", "full"] {
        let mut game = base.clone();
        if blocked == "existing" {
            game.push_generated_actor("test.vidarr".into(), VIDARR, Position { x: 13, y: 10 });
        }
        if blocked == "dead" {
            game.defeated_limited_actor_counts.insert(VIDARR.into(), 1);
        }
        if blocked == "full" {
            game.terrain.fill("demo.terrain.wall".into());
            let player = game.player.position;
            replace_terrain(&mut game, player, "demo.terrain.floor");
            replace_terrain(&mut game, Position { x: 11, y: 10 }, "demo.terrain.floor");
        }
        game.push_generated_actor("test.odin".into(), ODIN, Position { x: 11, y: 10 });
        death(&mut game, "test.odin", false);
        assert_eq!(
            game.entities
                .iter()
                .filter(|actor| actor.kind_id == VIDARR)
                .count(),
            usize::from(blocked == "existing")
        );
    }
    let mut pet_game = base;
    pet_game.push_generated_actor("test.pet-odin".into(), ODIN, Position { x: 11, y: 10 });
    pet_game.entities.last_mut().unwrap().controller_id = Some(pet_game.player.id.clone());
    death(&mut pet_game, "test.pet-odin", true);
    let avenger = pet_game
        .entities
        .iter()
        .find(|actor| actor.kind_id == VIDARR)
        .unwrap();
    assert_eq!(avenger.controller_id, Some(pet_game.player.id.clone()));
    assert!(
        pet_game
            .items
            .iter()
            .all(|item| item.kind_id != "demo.item.runespear")
    );
    let saved = Game::from_save(pet_game.to_save()).unwrap();
    assert_eq!(saved.state_hash(), pet_game.state_hash());
}

#[test]
fn asgard_frigg_chest_is_independent_of_named_and_ordinary_rewards() {
    for rewards in [true, false] {
        let mut game = game();
        game.push_generated_actor("test.frigg".into(), FRIGG, Position { x: 11, y: 10 });
        game.entities.last_mut().unwrap().controller_id = Some(game.player.id.clone());
        let events = death(&mut game, "test.frigg", rewards);
        assert_eq!(
            game.items
                .iter()
                .filter(|item| item.kind_id == CHEST)
                .count(),
            1
        );
        let item = game
            .items
            .iter()
            .find(|item| item.kind_id == CHEST)
            .unwrap();
        assert_eq!(item.origin_actor_kind_id.as_deref(), Some(FRIGG));
        assert!((1..=15).contains(&item.chest.unwrap().difficulty));
        assert!(events.iter().any(|event| matches!(event, DomainEvent::LootDropped { target_kind_id, .. } if target_kind_id == CHEST)));
        assert!(
            game.items
                .iter()
                .all(|item| item.kind_id != "demo.item.frigg")
        );
        if !rewards {
            assert_eq!(game.items.len(), 1);
        }
    }
}

#[test]
fn asgard_chest_opening_uses_saved_state_and_cannot_repeat_after_restore() {
    let mut game = game();
    let id = chest(&mut game, -6);
    let before_turn = game.turn;
    let mut saved = Game::from_save(game.to_save()).unwrap();
    let command = GameCommand::OpenChest {
        item_id: id.clone(),
    };
    let left = dispatch_next(&mut game, command.clone());
    let right = dispatch_next(&mut saved, command.clone());
    assert_eq!(left, right);
    assert_eq!(game.turn, before_turn + 1);
    assert_eq!(game.state_hash(), saved.state_hash());
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .chest
            .unwrap()
            .difficulty,
        0
    );
    assert_eq!(game.items.iter().filter(|item| item.id != id).count(), 2);
    assert!(
        game.items
            .iter()
            .filter(|item| item.id != id)
            .all(|item| item.chest.is_none()
                && item.origin_kind == Some(rfb_protocol::ItemOriginKindDto::Chest))
    );
    assert_eq!(game.gold_piles.len(), 3);
    let ids = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let gold = game
        .gold_piles
        .iter()
        .map(|pile| (pile.id.clone(), pile.amount))
        .collect::<Vec<_>>();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    dispatch_next(&mut restored, command);
    assert_eq!(
        restored
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect::<BTreeSet<_>>(),
        ids
    );
    assert_eq!(
        restored
            .gold_piles
            .iter()
            .map(|pile| (pile.id.clone(), pile.amount))
            .collect::<Vec<_>>(),
        gold
    );
    let mut corrupted = restored.to_save();
    corrupted
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .chest
        .as_mut()
        .unwrap()
        .difficulty = 16;
    assert!(Game::from_save(corrupted).is_err());
}

#[test]
fn asgard_chest_disarm_requires_knowledge_and_preserves_treasure() {
    let mut game = game();
    let id = chest(&mut game, 13);
    assert!(
        !game
            .items_dto()
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .chest
            .unwrap()
            .can_disarm
    );
    let before = game.rng.clone();
    game.interact_chest(&id, true, &mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(game.rng, before);
    game.rng = RfbRng::seeded(unlock_seed());
    assert!(game.search_chest_traps(&mut Vec::new(), &mut BTreeSet::new()));
    assert!(
        game.items_dto()
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .chest
            .unwrap()
            .can_disarm
    );
    game.rng = RfbRng::seeded(unlock_seed());
    game.interact_chest(&id, true, &mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .chest
            .unwrap()
            .difficulty,
        -13
    );
    assert!(game.gold_piles.is_empty());
    let hp = game.player.hp;
    let attributes = game.progress.attributes.clone();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    restored
        .interact_chest(&id, false, &mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(restored.player.hp, hp);
    assert_eq!(restored.progress.attributes, attributes);
    assert_eq!(restored.gold_piles.len(), 3);
}

#[test]
fn asgard_chest_scatter_poison_needles_alarm_and_summoning_have_real_effects() {
    for difficulty in [1, 7, 12, 13, 15] {
        let mut game = game();
        let id = chest(&mut game, difficulty);
        game.player.hp = game.effective_player_max_hp();
        let hp = game.player.hp;
        if difficulty == 7 {
            game.push_generated_actor(
                "test.sleeper".into(),
                "demo.actor.software-bug",
                Position { x: 13, y: 10 },
            );
            game.entities.last_mut().unwrap().alerted = false;
        }
        game.rng = RfbRng::seeded(unlock_seed());
        let mut events = Vec::new();
        game.interact_chest(&id, false, &mut events, &mut BTreeSet::new())
            .unwrap();
        match difficulty {
            1 => assert!(
                game.player
                    .statuses
                    .iter()
                    .any(|status| status.kind_id == STATUS_POISON)
            ),
            7 => assert!(
                game.entities
                    .iter()
                    .find(|actor| actor.id == "test.sleeper")
                    .unwrap()
                    .alerted
            ),
            12 => {
                assert_eq!(game.gold_piles.len(), 3);
                assert_eq!(game.items.iter().filter(|item| item.id != id).count(), 2);
                assert!(
                    game.gold_piles
                        .iter()
                        .any(
                            |pile| chebyshev_distance(pile.position, Position { x: 11, y: 10 }) > 1
                        )
                );
            }
            13 => {
                assert!(game.player.hp < hp);
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(event, DomainEvent::ItemAttributeChanged { .. }))
                        .count(),
                    2
                );
            }
            15 => assert!(!game.entities.is_empty()),
            _ => unreachable!(),
        }
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .chest
                .unwrap()
                .difficulty,
            0
        );
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
    }
}

#[test]
fn asgard_mundanity_empties_chests_without_losing_valid_save_state() {
    let mut game = game();
    let id = chest(&mut game, 15);
    game.mundanify_item(&id);
    let empty = game.items.iter().find(|item| item.id == id).unwrap();
    assert_eq!(empty.chest.unwrap().difficulty, 0);
    assert_eq!(empty.chest.unwrap().opening_depth, 0);
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn asgard_chest_blind_confused_failures_keep_the_lock_and_can_trigger_disarm_traps() {
    let mut base = game();
    base.set_floor_glow_at(base.player.position, true);
    let id = chest(&mut base, 6);
    let chance = (base.player_derived_stats().disarm_skill.value - 6).max(2) as u64;
    assert!(chance > 2);
    let seed = (0..1000)
        .find(|seed| {
            let roll = RfbRng::seeded(*seed).bounded(100);
            roll >= 2 && roll < chance
        })
        .unwrap();
    let mut hindered = base.clone();
    for status in [STATUS_BLINDNESS, STATUS_CONFUSION] {
        hindered.resolve_item_status(
            CHEST,
            status,
            0,
            0,
            100,
            rfb_content::AbilityStatusStackingDefinition::Extend,
            None,
            &Default::default(),
            &Default::default(),
            &Default::default(),
            100,
            &mut Vec::new(),
        );
    }
    base.rng = RfbRng::seeded(seed);
    base.interact_chest(&id, false, &mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(
        base.items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .chest
            .unwrap()
            .difficulty,
        0
    );
    hindered.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    hindered
        .interact_chest(&id, false, &mut events, &mut BTreeSet::new())
        .unwrap();
    assert!(events.iter().any(|event| matches!(event, DomainEvent::ChestInteracted { message_key } if message_key == "chest-unlock-failed")));
    assert_eq!(
        hindered
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .chest
            .unwrap()
            .difficulty,
        6
    );
    assert!(hindered.gold_piles.is_empty());
    hindered
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .chest
        .as_mut()
        .unwrap()
        .difficulty = 2;
    hindered.identify_item_instance(&id, ItemIdentificationRequest::new(false));
    let hp = hindered.player.hp;
    hindered.rng = RfbRng::seeded(seed);
    hindered
        .interact_chest(&id, true, &mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert!(hindered.player.hp < hp);
    assert_eq!(
        hindered
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .chest
            .unwrap()
            .difficulty,
        2
    );
    assert!(hindered.gold_piles.is_empty());
}

#[test]
fn asgard_chest_id_exhaustion_rejects_before_time_rng_or_treasure_changes() {
    for gold in [false, true] {
        let mut game = game();
        let id = chest(&mut game, -6);
        if gold {
            game.next_gold_pile_serial = u64::MAX - 2;
        } else {
            game.next_item_instance_serial = u64::MAX - 1;
        }
        let before = game.to_save();
        let snapshot = game.snapshot();
        let result = game.dispatch(command(
            snapshot.last_command_seq + 1,
            snapshot.revision,
            GameCommand::OpenChest { item_id: id },
        ));
        assert!(matches!(
            result,
            Err(CoreError::ItemIdExhausted | CoreError::GoldPileIdExhausted)
        ));
        assert_eq!(game.to_save(), before);
    }
}

#[test]
fn asgard_aegir_extra_booze_is_split_to_valid_stacks_and_excludes_pets() {
    let mut game = game();
    let mut actor = game.generated_actor(
        "test.aegir".into(),
        "demo.actor.aegir-god-king-of-the-sea-giants",
        Position { x: 11, y: 10 },
    );
    let seed = (0..1000)
        .find(|seed| RfbRng::seeded(*seed).bounded(25) >= 20)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let drops = game.generate_norse_death_extras(&actor, true).unwrap();
    assert_eq!(drops.len(), 2);
    assert!(
        drops
            .iter()
            .all(|item| item.kind_id == "demo.item.booze-potion" && item.quantity <= 20)
    );
    assert!((21..=25).contains(&drops.iter().map(|item| item.quantity).sum::<u32>()));
    actor.controller_id = Some(game.player.id.clone());
    let before = game.rng.clone();
    assert!(
        game.generate_norse_death_extras(&actor, true)
            .unwrap()
            .is_empty()
    );
    assert_eq!(game.rng, before);
}

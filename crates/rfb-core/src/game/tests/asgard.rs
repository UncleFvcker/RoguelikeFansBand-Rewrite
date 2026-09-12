// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
mod deaths;
mod floors;
mod integration;
mod mead;

fn game() -> Game {
    let mut game = Game::new_with_build(493, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=14 {
        for x in 8..=20 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game
}

fn context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-100".into(),
        depth: 100,
        source: LootSource::ItemUse {
            item_id: "test.asgard".into(),
        },
    }
}

fn artifact(game: &mut Game, slug: &str) -> String {
    let draft = game.fixed_item_draft(&context(), format!("demo.item.{slug}"));
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    id
}

fn activate(
    game: &mut Game,
    id: &str,
    target: Option<&TargetSelection>,
) -> (Option<i32>, Vec<DomainEvent>) {
    let mut events = Vec::new();
    let cost = game
        .use_inventory_item(
            id,
            target,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    (cost, events)
}

fn successful_seed(game: &Game, id: &str, target: Option<&TargetSelection>) -> u64 {
    (0..1000)
        .find(|seed| {
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            activate(&mut trial, id, target).1.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::DeviceSkillChecked {
                        succeeded: true,
                        ..
                    }
                )
            })
        })
        .unwrap()
}

fn east() -> TargetSelection {
    TargetSelection::Direction {
        direction: Direction::East,
    }
}

#[test]
fn asgard_factory_records_all_fixed_identities_and_ordinary_eligibility_survives_save() {
    let mut game = game();
    let freyr = "demo.item.freyr";
    let base = game
        .content
        .item(freyr)
        .unwrap()
        .artifact_generation
        .as_ref()
        .unwrap()
        .base_item_kind_id
        .clone();
    assert!((0..30_000).any(|_| {
        game.roll_fixed_artifact_kind_id(&context(), Some(&base), false)
            .as_deref()
            == Some(freyr)
    }));
    for slug in [
        "ingwe",
        "runespear",
        "mjollnir",
        "aegir",
        "freyr",
        "gjallarhorn",
        "tyr",
        "vidarr",
        "freyja",
        "brisingamen",
        "njord",
        "frigg",
        "skadi",
        "ullur",
        "jarngreipr",
    ] {
        artifact(&mut game, slug);
    }
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.generated_artifact_ids, game.generated_artifact_ids);
    assert!((0..1000).all(|_| {
        restored
            .roll_fixed_artifact_kind_id(&context(), Some(&base), false)
            .as_deref()
            != Some(freyr)
    }));
}

#[test]
fn asgard_horn_requires_equipment_recalls_controlled_pets_and_preserves_cooldown() {
    let mut game = game();
    let id = artifact(&mut game, "gjallarhorn");
    assert!(
        !game
            .inventory_item_dto(game.items.iter().find(|item| item.id == id).unwrap())
            .usable
    );
    game.equip_inventory_item(&id, None).unwrap();
    replace_terrain(
        &mut game,
        Position { x: 20, y: 10 },
        "demo.terrain.surface-water-shallow",
    );
    game.push_generated_actor(
        "test.pet".into(),
        "demo.actor.piranha",
        Position { x: 20, y: 10 },
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    let seed = successful_seed(&game, &id, Some(&TargetSelection::SelfTarget));
    game.rng = RfbRng::seeded(seed);
    let (_, events) = activate(&mut game, &id, Some(&TargetSelection::SelfTarget));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterBlinked { .. }))
    );
    assert!(rfb_distance(game.entities[0].position, game.player.position) <= 2);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let before = restored.rng.clone();
    assert!(
        activate(&mut restored, &id, Some(&TargetSelection::SelfTarget))
            .1
            .iter()
            .any(|event| matches!(event, DomainEvent::ItemUseUnavailable))
    );
    assert_eq!(restored.rng, before);
    restored.entities[0].position = Position { x: -1, y: 10 };
    assert!(Game::from_save(restored.to_save()).is_err());
}

#[test]
fn asgard_kick_only_stuns_and_null_target_still_spends_activation() {
    let mut game = game();
    let id = artifact(&mut game, "vidarr");
    game.equip_inventory_item(&id, None).unwrap();
    game.push_generated_actor(
        "test.target".into(),
        "demo.actor.war-bear",
        Position { x: 11, y: 10 },
    );
    let hp = game.entities[0].hp;
    game.rng = RfbRng::seeded(successful_seed(&game, &id, Some(&east())));
    let mut cancelled = game.clone();
    activate(&mut game, &id, Some(&east()));
    assert_eq!(game.entities[0].hp, hp);
    assert_eq!(
        game.entities[0]
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_STUN)
            .unwrap()
            .remaining_ticks,
        100
    );
    activate(&mut cancelled, &id, None);
    assert!(cancelled.entities[0].statuses.is_empty());
    assert_eq!(
        cancelled
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
}

#[test]
fn asgard_fishing_start_block_save_cancel_and_ecology_resume() {
    let mut game = game();
    game.transition_floor("demo.floor.warrens-depth-3".into(), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.player.position = Position { x: 10, y: 10 };
    for position in [
        game.player.position,
        Position { x: 11, y: 10 },
        Position { x: 9, y: 10 },
    ] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    let id = artifact(&mut game, "njord");
    game.equip_inventory_item(&id, None).unwrap();
    let water = Position { x: 11, y: 10 };
    let seed = successful_seed(&game, &id, Some(&east()));
    game.rng = RfbRng::seeded(seed);
    assert!(
        activate(&mut game, &id, Some(&east()))
            .1
            .iter()
            .any(|e| matches!(e, DomainEvent::FishingNoWater))
    );
    replace_terrain(&mut game, water, "demo.terrain.surface-water-shallow");
    game.push_generated_actor("test.blocker".into(), "demo.actor.war-bear", water);
    game.rng = RfbRng::seeded(seed);
    assert_eq!(activate(&mut game, &id, Some(&east())).0, Some(0));
    clear_monsters(&mut game);
    game.rng = RfbRng::seeded(seed);
    activate(&mut game, &id, Some(&east()));
    assert_eq!(game.fishing_direction, Some(Direction::East));
    let mut saved = Game::from_save(game.to_save()).unwrap();
    assert_eq!(saved.state_hash(), game.state_hash());
    let tick = saved.world_tick;
    dispatch_next(&mut saved, GameCommand::CancelFishing);
    assert_eq!(saved.world_tick, tick);
    assert_eq!(saved.fishing_direction, None);
    let mut invalid = game.to_save();
    invalid.player.fishing_direction = Some(Direction::West);
    assert!(Game::from_save(invalid).is_err());
    let seed = (0..100_000)
        .find(|seed| {
            if RfbRng::seeded(*seed).bounded(1000) != 0 {
                return false;
            }
            let mut trial = game.clone();
            trial.rng = RfbRng::seeded(*seed);
            let mut events = Vec::new();
            trial.continue_fishing(&mut events, &mut BTreeSet::new());
            events
                .iter()
                .any(|e| matches!(e, DomainEvent::FishingCaught { .. }))
        })
        .expect("depth-3 allocation can actually catch an aquatic creature");
    game.rng = RfbRng::seeded(seed);
    saved = Game::from_save(game.to_save()).unwrap();
    let left = dispatch_next(&mut game, GameCommand::ContinueFishing);
    let right = dispatch_next(&mut saved, GameCommand::ContinueFishing);
    assert_eq!(left, right);
    assert_eq!(game.state_hash(), saved.state_hash());
    assert_eq!(game.fishing_direction, None);
    assert!(
        left.events
            .iter()
            .any(|event| event.message_key == "fishing-caught")
    );
    assert!(game.entities.iter().all(|actor| {
        let kind = game.content.actor(&actor.kind_id).unwrap();
        kind.movement
            .modes
            .contains(&rfb_content::ActorMovementMode::Aquatic)
            && matches!(kind.glyph.as_str(), "J" | "j" | "l" | "w")
            && !kind.tags.iter().any(|tag| tag == "unique")
    }));
}

#[test]
fn asgard_ullur_native_hit_and_damage_are_shooting_only() {
    let mut game = game();
    give_inventory_item(&mut game, "test.bow", "demo.item.long-bow");
    game.equip_inventory_item("test.bow", None).unwrap();
    give_inventory_item(&mut game, "test.arrow", "demo.item.arrow");
    let before = game.player_projectile_profile().unwrap();
    let ring = artifact(&mut game, "ullur");
    game.equip_inventory_item(&ring, None).unwrap();
    let after = game.player_projectile_profile().unwrap();
    assert_eq!(after.to_hit, before.to_hit + 21);
    assert_eq!(after.launcher_to_damage, before.launcher_to_damage + 36);
    let item = game.items.iter().find(|item| item.id == ring).unwrap();
    let bonuses = game.item_equipment_bonuses(item);
    assert_eq!((bonuses.melee_skill, bonuses.melee_damage), (0, 0));
    assert_eq!(bonuses.base_shot_delta_percent, 45);
    assert!(crate::game::item_value::obj_value_real(&game.content, item).unwrap() > 0);
}

#[test]
fn asgard_mjollnir_gloves_add_one_attack_and_throw_keeps_identity_or_drops_it() {
    let mut game = game();
    let hammer = artifact(&mut game, "mjollnir");
    game.equip_inventory_item(&hammer, None).unwrap();
    let without = game.player_melee_profile(&game.player_derived_stats());
    let gloves = artifact(&mut game, "jarngreipr");
    game.equip_inventory_item(&gloves, None).unwrap();
    let with = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(with.attacks, without.attacks + 1);
    let mut dual = game.clone();
    give_inventory_item(&mut dual, "test.offhand", "demo.item.dagger");
    let offhand_slot = dual
        .body_slots
        .iter()
        .find(|slot| slot.slot_type == "shield")
        .unwrap()
        .id
        .clone();
    dual.equip_inventory_item("test.offhand", Some(&offhand_slot))
        .unwrap();
    let profiles = dual.player_melee_profiles(&dual.player_derived_stats());
    let offhand = profiles
        .iter()
        .find(|profile| profile.source_item_id.as_deref() == Some("test.offhand"))
        .unwrap();
    assert!(
        !offhand
            .attack_sources
            .iter()
            .any(|source| source.source_id == hammer)
    );
    let glove_slot = match &dual
        .items
        .iter()
        .find(|item| item.id == gloves)
        .unwrap()
        .location
    {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => unreachable!(),
    };
    dual.unequip_slot(&glove_slot).unwrap();
    let profile = dual
        .player_melee_profiles(&dual.player_derived_stats())
        .into_iter()
        .find(|profile| profile.source_item_id.as_deref() == Some(hammer.as_str()))
        .unwrap();
    assert_eq!(profile.attacks, without.attacks);
    assert!(
        game.equipment_dto()
            .iter()
            .find(|item| item.id == hammer)
            .unwrap()
            .throw_target_spec
            .is_some()
    );
    let original = game
        .items
        .iter()
        .find(|item| item.id == hammer)
        .unwrap()
        .clone();
    let throw = |game: &mut Game| {
        game.throw_inventory_item(
            &hammer,
            Direction::East,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    };
    let mut caught = false;
    let mut dropped = false;
    for seed in 0..1000 {
        let mut trial = game.clone();
        trial.rng = RfbRng::seeded(seed);
        let mut saved = Game::from_save(trial.to_save()).unwrap();
        throw(&mut trial);
        throw(&mut saved);
        assert_eq!(trial.state_hash(), saved.state_hash());
        let item = trial.items.iter().find(|item| item.id == hammer).unwrap();
        assert_eq!(item.kind_id, original.kind_id);
        assert_eq!(item.charges, original.charges);
        caught |= item.location == original.location;
        dropped |= matches!(item.location, ItemLocation::Ground(_));
        if caught && dropped {
            break;
        }
    }
    assert!(caught && dropped);
}

// SPDX-License-Identifier: MPL-2.0
use super::pet_spells::arena_with_spell;
use super::support::*;
use super::*;

const FIXED: &str = "rfb-legacy.ability.kin-archlich";
const CATEGORY: &str = "rfb-legacy.ability.summon-hound-l38-1d2-1";
const RAISE: &str = "rfb-legacy.ability.animate-dead";

fn cast(game: &mut Game, ability_id: &str) -> MonsterAbilityPlanResolution {
    let ability = game.content.ability(ability_id).unwrap().clone();
    let plan = game.monster_ability_plan(0, ability, 1).unwrap();
    let kind = game.entities[0].kind_id.clone();
    game.resolve_monster_ability_plan(
        0,
        &kind,
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
}

#[test]
fn summon_option_is_free_saved_and_controls_the_live_casting_path() {
    let mut game = arena_with_spell(FIXED);
    assert!(game.summon_command.summon_spells);
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::SummonSpells,
            enabled: false,
        },
    );
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
    assert!(!game.resolve_monster_ability(0, &mut Vec::new()));
    assert_eq!(game.entities.len(), 2);
    let mut restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    assert!(!restored.summon_command.summon_spells);
    for state in [&mut game, &mut restored] {
        dispatch_next(
            state,
            GameCommand::SetPetOption {
                option: rfb_protocol::PetOptionDto::SummonSpells,
                enabled: true,
            },
        );
        assert!(
            state.resolve_monster_ability(
                state
                    .entities
                    .iter()
                    .position(|actor| actor.id == "pet")
                    .unwrap(),
                &mut Vec::new()
            )
        );
        assert!(state.entities.len() > 2);
        assert!(state.entities[2..].iter().all(|actor| {
            state.actor_is_player_aligned(actor)
                && actor
                    .summon
                    .as_ref()
                    .is_some_and(|summon| summon.owner_dependent && summon.owner_id == "pet")
        }));
    }
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn fixed_and_category_children_are_commandable_counted_and_saved() {
    for spell in [FIXED, CATEGORY] {
        let mut game = arena_with_spell(spell);
        let before = game.pet_upkeep();
        game.entities[0].custom_name = Some("召唤者".into());
        let resolution = cast(&mut game, spell).summon.unwrap();
        assert!(!resolution.entity_ids.is_empty());
        assert!(game.pet_upkeep().total_levels > before.total_levels);
        for id in &resolution.entity_ids {
            let child = game.entities.iter().find(|actor| actor.id == *id).unwrap();
            let unique = game
                .content
                .actor(&child.kind_id)
                .unwrap()
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
            assert_eq!(game.actor_is_player_aligned(child), !unique);
            assert!(game.actor_is_player_side(child));
            assert!(child.custom_name.is_none());
            assert_eq!(
                child.controller_id.as_deref(),
                (!unique).then_some(game.player.id.as_str())
            );
            assert_eq!(
                child.summon.as_ref().unwrap().remaining_turns,
                resolution.duration_turns
            );
            assert_eq!(child.summon.as_ref().unwrap().owner_dependent, !unique);
        }
        let restored = Game::from_save_with_content(
            game.to_save(),
            game.content.clone(),
            game.behavior_preferences(),
        )
        .unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        game.summon_command.target_actor_id = Some("enemy".into());
        let controlled_id = resolution
            .entity_ids
            .iter()
            .find(|id| {
                game.entities
                    .iter()
                    .any(|actor| actor.id == **id && game.actor_is_player_aligned(actor))
            })
            .unwrap()
            .clone();
        let index = game
            .entities
            .iter()
            .position(|actor| actor.id == controlled_id)
            .unwrap();
        assert_eq!(game.pet_hostile_target_ids(index), ["enemy"]);
        dispatch_next(
            &mut game,
            GameCommand::DismissPet {
                actor_id: controlled_id.clone(),
            },
        );
        assert!(!game.entities.iter().any(|actor| actor.id == controlled_id));
    }
}

#[test]
fn special_family_summons_inherit_control_and_unique_children_remain_friends() {
    for (kind, spell, controlled) in [
        (
            "demo.actor.hades-ruler-of-the-underworld",
            "rfb-legacy.ability.summon-family-hades-ruler-of-the-underworld",
            true,
        ),
        (
            "demo.actor.artemis-the-moon-goddess",
            "rfb-legacy.ability.summon-family-artemis-the-moon-goddess",
            false,
        ),
    ] {
        let mut game = arena_with_spell(FIXED);
        game.entities[0] = game.generated_actor("pet".into(), kind, Position { x: 71, y: 30 });
        game.entities[0].controller_id = Some(game.player.id.clone());
        let ability = game.content.ability(spell).unwrap().clone();
        game.summon_command.summon_spells = false;
        assert!(game.monster_ability_plan(0, ability.clone(), 1).is_err());
        game.summon_command.summon_spells = true;
        let resolution = cast(&mut game, spell).summon.unwrap();
        assert!(!resolution.entity_ids.is_empty());
        for id in &resolution.entity_ids {
            let actor = game.entities.iter().find(|actor| actor.id == *id).unwrap();
            assert_eq!(game.actor_is_player_aligned(actor), controlled);
            assert!(game.actor_is_player_side(actor));
            assert_eq!(actor.summon.as_ref().unwrap().owner_dependent, controlled);
        }
        Game::from_save_with_content(
            game.to_save(),
            game.content.clone(),
            game.behavior_preferences(),
        )
        .unwrap();
        if !controlled {
            assert!(
                game.monster_ability_plan(0, ability, 1).is_err(),
                "the living unique cannot be summoned twice"
            );
        }
    }
}

#[test]
fn source_disappearance_removes_child_and_cargo_on_its_next_action_after_restore() {
    let mut game = arena_with_spell(FIXED);
    let id = cast(&mut game, FIXED).summon.unwrap().entity_ids[0].clone();
    give_inventory_item(&mut game, "test.child-cargo", "demo.item.short-sword");
    game.items.last_mut().unwrap().location = ItemLocation::CarriedBy {
        actor_id: id.clone(),
    };
    game.remove_pet_at(0, &mut BTreeSet::new(), &mut Vec::new());
    let mut restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    for state in [&mut game, &mut restored] {
        let index = state
            .entities
            .iter()
            .position(|actor| actor.id == id)
            .unwrap();
        state.entities[index].energy_need = 0;
        state
            .continue_monster_energy_pulse(
                vec![id.clone()],
                BTreeSet::new(),
                false,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert!(!state.entities.iter().any(|actor| actor.id == id));
        assert!(!state.items.iter().any(|item| item.id == "test.child-cargo"));
    }
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn surviving_mount_releases_parent_dependency_without_renewing_its_lifetime() {
    const SPELL: &str = "rfb-legacy.ability.kin-great-wyrm-of-power";
    let mut game = arena_with_spell(SPELL);
    let id = cast(&mut game, SPELL).summon.unwrap().entity_ids[0].clone();
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    assert!(
        game.content
            .actor(&game.entities[index].kind_id)
            .unwrap()
            .rideable
    );
    game.entities[index].position = game.player.position;
    game.entities[index]
        .summon
        .as_mut()
        .unwrap()
        .remaining_turns = 7;
    game.riding_actor_id = Some(id.clone());
    game.remove_pet_at(0, &mut BTreeSet::new(), &mut Vec::new());
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    assert!(!game.resolve_missing_summon_owner(
        index,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new()
    ));
    let summon = game.entities[index].summon.as_ref().unwrap();
    assert!(!summon.owner_dependent);
    assert_eq!(summon.remaining_turns, 7);
    assert_eq!(summon.owner_id, "pet");
    Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    game.riding_actor_id = None;
    game.entities[index].position = Position { x: 72, y: 30 };
    assert!(!game.resolve_missing_summon_owner(
        index,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new()
    ));
}

#[test]
fn raised_pet_is_permanent_but_still_depends_on_its_caster_and_cyclic_saves_are_rejected() {
    let mut game = arena_with_spell(RAISE);
    game.summon_command.summon_spells = false; // Animate dead is an annoyance spell in RFB.
    give_inventory_item(&mut game, "test.corpse", "demo.item.corpse-remains");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(Position { x: 72, y: 31 });
    let mut ability = game.content.ability(RAISE).unwrap().clone();
    for effect in match &mut ability.effect {
        AbilityEffectDefinition::Sequence { effects } => effects,
        _ => panic!("source raise sequence"),
    } {
        if let AbilityEffectDefinition::AnimateDead {
            failure_chance_percent,
            ..
        } = effect
        {
            *failure_chance_percent = 0;
        }
    }
    let plan = game.monster_ability_plan(0, ability, 1).unwrap();
    game.resolve_monster_ability_plan(
        0,
        "demo.actor.horse",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    let id = game.entities.last().unwrap().id.clone();
    assert_eq!(
        game.entities.last().unwrap().kind_id,
        "demo.actor.risen-thrall"
    );
    assert_eq!(
        game.entities
            .last()
            .unwrap()
            .summon
            .as_ref()
            .unwrap()
            .remaining_turns,
        0
    );
    assert!(game.actor_is_player_aligned(game.entities.last().unwrap()));
    game.entities.retain(|actor| actor.id != "enemy");
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.entities.iter().any(|actor| actor.id == id));
    Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    game.entities[index].summon.as_mut().unwrap().owner_id = id;
    assert!(
        Game::from_save_with_content(
            game.to_save(),
            game.content.clone(),
            game.behavior_preferences()
        )
        .is_err()
    );
}

#[test]
fn no_space_and_anti_summoning_prevent_actual_spawns() {
    let mut game = arena_with_spell(FIXED);
    let spell = game.content.ability(FIXED).unwrap().clone();
    let origin = game.entities[0].position;
    for y in origin.y - 2..=origin.y + 2 {
        for x in origin.x - 2..=origin.x + 2 {
            let position = Position { x, y };
            if position != origin && position != game.player.position {
                replace_terrain(&mut game, position, "demo.terrain.permanent-wall");
            }
        }
    }
    assert!(game.monster_ability_target_plan(0, spell, 1).is_err());
    let mut game = arena_with_spell(FIXED);
    give_inventory_item(&mut game, "test.no-summon", "demo.item.chain-mail");
    let armor = game.items.last_mut().unwrap();
    armor.location = ItemLocation::Equipped {
        slot_id: "body".into(),
    };
    armor
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::AntiSummoning);
    let seed = (0..100)
        .find(|seed| {
            let mut rng = crate::rng::RfbRng::seeded(*seed);
            rng.bounded(3) != 0 && rng.bounded(3) != 0
        })
        .unwrap();
    game.rng = crate::rng::RfbRng::seeded(seed);
    let before = game.rng_draw_counter();
    assert!(cast(&mut game, FIXED).summon.unwrap().entity_ids.is_empty());
    assert_eq!(game.entities.len(), 2);
    assert_eq!(game.rng_draw_counter(), before + 2);
}

#[test]
fn control_is_not_revoked_when_source_turns_hostile_or_stays_on_another_floor() {
    let mut game = arena_with_spell(FIXED);
    let id = cast(&mut game, FIXED).summon.unwrap().entity_ids[0].clone();
    game.entities[0].controller_id = None;
    let hostile_children = cast(&mut game, FIXED).summon.unwrap().entity_ids;
    assert!(hostile_children.iter().all(|id| {
        !game.actor_is_player_side(game.entities.iter().find(|actor| actor.id == *id).unwrap())
    }));
    game.entities[0].position = Position { x: 80, y: 34 };
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    game.entities[index].position = Position { x: 70, y: 29 };
    assert!(game.entity_is_player_aligned(index));
    let old_floor = game.current_floor_id.clone();
    game.transition_floor("demo.floor.warrens-depth-1".into(), None, None, false)
        .unwrap()
        .unwrap();
    assert!(
        stored_floor(&game, &old_floor)
            .entities
            .iter()
            .any(|actor| actor.id == "pet")
    );
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    assert!(!game.resolve_missing_summon_owner(
        index,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new()
    ));
    let mut restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    for floor in restored.stored_floors.values_mut() {
        floor.entities.retain(|actor| actor.id != "pet");
    }
    let index = restored
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    assert!(restored.resolve_missing_summon_owner(
        index,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new()
    ));
}

#[test]
fn anti_summoning_also_reaches_category_family_and_dead_unique_attempts() {
    for spell in [
        CATEGORY,
        "rfb-legacy.ability.summon-family-artemis-the-moon-goddess",
        "rfb-legacy.ability.summon-dead-unique-l100-1d2",
    ] {
        let mut game = arena_with_spell(spell);
        let family = spell.contains("family");
        if family {
            let actor = game.generated_actor(
                "pet".into(),
                "demo.actor.artemis-the-moon-goddess",
                Position { x: 71, y: 30 },
            );
            game.entities[0] = actor;
            game.entities[0].controller_id = Some(game.player.id.clone());
        }
        give_inventory_item(&mut game, "test.no-summon", "demo.item.chain-mail");
        let armor = game.items.last_mut().unwrap();
        armor.location = ItemLocation::Equipped {
            slot_id: "body".into(),
        };
        armor
            .intrinsic_properties
            .passives
            .insert(EquipmentPassive::AntiSummoning);
        let seed = (0..1000)
            .find(|seed| {
                let mut rng = crate::rng::RfbRng::seeded(*seed);
                let attempts = if family {
                    1
                } else {
                    rng.bounded(2) + 1 + u64::from(spell == CATEGORY)
                };
                (0..attempts).all(|_| rng.bounded(3) != 0)
            })
            .unwrap();
        game.rng = crate::rng::RfbRng::seeded(seed);
        assert!(
            cast(&mut game, spell).summon.unwrap().entity_ids.is_empty(),
            "{spell}"
        );
        assert_eq!(game.entities.len(), 2);
    }
}

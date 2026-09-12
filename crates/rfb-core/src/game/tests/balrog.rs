// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: races_a.c, cmd6.c, spells_c.c, gf.c.
use super::support::*;
use super::*;

const BALROG: &str = "rfb-legacy.race.balrog";
const BREATH: &str = "rfb.ability.race.demon-breath";
const START: Position = Position { x: 99, y: 33 };
const EAST: Position = Position { x: 100, y: 33 };

fn prepared(level: u16) -> Game {
    let mut game =
        Game::new_with_build_race_and_name(83, "demo.build.warrior", BALROG, "test").unwrap();
    clear_monsters(&mut game);
    game.player.position = START;
    for y in 29..=37 {
        for x in 95..=118 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    game.player.hp = game.effective_player_max_hp();
    game.world_tick = 1;
    game
}

fn use_corpse(game: &mut Game, id: &str) {
    dispatch_next(
        game,
        GameCommand::UseItem {
            item_id: id.to_owned(),
            target: None,
        },
    );
}

#[test]
fn balrog_birth_uses_human_allocation_and_preserves_class_kits_and_save_identity() {
    for build in [
        "warrior",
        "berserker",
        "duelist",
        "archer",
        "sniper",
        "cavalry",
        "ranger-nature-sorcery",
        "mage-life-sorcery",
        "high-mage-death",
        "magic-eater",
        "priest-life-sorcery",
        "paladin-death",
        "warrior-mage-arcane-life",
        "mindcrafter",
    ] {
        let game =
            Game::new_with_build_race_and_name(83, &format!("demo.build.{build}"), BALROG, "test")
                .unwrap();
        let corpses: Vec<_> = game
            .items
            .iter()
            .filter(|item| game.item_is_human_corpse(item))
            .collect();
        assert!((3..=4).contains(&corpses.len()));
        for item in corpses {
            assert_eq!(item.quantity, 1);
            assert_eq!(item.location, ItemLocation::Inventory);
            assert!(game.inventory_item_dto(item).usable);
            let actor = game
                .content
                .actor(item.origin_actor_kind_id.as_deref().unwrap())
                .unwrap();
            assert!(
                !actor
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
            );
            assert!(actor.allocation.is_some());
        }
        assert!(
            game.virtues
                .iter()
                .any(|v| v.kind == VirtueKindDto::Justice)
        );
        assert!(
            !game
                .items
                .iter()
                .any(|item| item.kind_id == "demo.item.ration-of-food")
        );
        assert!(
            game.items
                .iter()
                .any(|item| item.kind_id == "demo.item.wooden-torch")
        );
        let (build, _, class, personality) = game.character_definitions().unwrap();
        for expected in class
            .starting_items
            .iter()
            .chain(&personality.starting_items)
            .chain(&build.starting_items)
        {
            assert!(game.items.iter().any(|item| {
                item.kind_id == expected.item_kind_id
                    && (expected.quantity..=expected.maximum_quantity.unwrap_or(expected.quantity))
                        .contains(&item.quantity)
                    && matches!(item.location, ItemLocation::Equipped { .. }) == expected.equipped
            }));
        }
        let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.snapshot(), game.snapshot());
        assert_eq!(restored.state_hash(), game.state_hash());
    }
}

#[test]
fn balrog_passive_thresholds_alignment_and_current_form_have_real_consumers() {
    for level in [9, 10, 44, 45] {
        let mut game = prepared(level);
        assert!(game.player_is_nonliving());
        assert_eq!(game.player_infravision_range(), 5);
        assert!(game.player_hold_life_sources() > 0);
        assert_eq!(game.player_see_invisible_sources() > 0, level >= 10);
        assert_eq!(
            game.effective_player_resistances().level(DamageType::Fire),
            if level >= 45 {
                ResistanceLevel::Strong
            } else {
                ResistanceLevel::Resistant
            }
        );
        assert_eq!(
            game.effective_player_resistances()
                .level(DamageType::Nether),
            ResistanceLevel::Resistant
        );
        for status in [STATUS_BLEEDING, STATUS_UNWELL] {
            game.apply_player_melee_status(status, 100, "test.nonliving");
            assert!(!game.player_has_status_kind(status));
        }
        game.virtues.iter_mut().for_each(|virtue| virtue.value = 0);
        assert_eq!(game.player_alignment(), -200);
        for (kind, expected) in [(DamageType::HolyFire, 260), (DamageType::HellFire, 50)] {
            let damage = game.resist_player_damage(resolve_damage(
                DamagePacket::new(100, kind),
                ResistanceLevel::Normal,
            ));
            assert_eq!(damage.applied, expected);
            assert_eq!(damage.raw, 100);
            assert_eq!(damage.resistance_delta, 100 - expected);
        }
        let mut form =
            monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.form").status;
        form.granted_race_id = Some("demo.race.rfb-human".to_owned());
        game.player.statuses.push(form);
        assert_eq!(game.player_alignment(), 0);
        assert!(!game.player_is_nonliving());
        assert!(
            !game.player_can_sacrifice_corpse(
                game.items
                    .iter()
                    .find(|item| game.item_is_human_corpse(item))
                    .unwrap()
            )
        );
        game.player.statuses.last_mut().unwrap().granted_race_id =
            Some("rfb-legacy.race.archon".to_owned());
        assert_eq!(game.player_alignment(), 200);
        for (kind, expected) in [(DamageType::HolyFire, 50), (DamageType::HellFire, 260)] {
            assert_eq!(
                game.resist_player_damage(resolve_damage(
                    DamagePacket::new(100, kind),
                    ResistanceLevel::Normal
                ))
                .applied,
                expected
            );
        }
    }
}

#[test]
fn balrog_sacrifices_one_pack_or_floor_corpse_without_healing_and_rejects_invalid_remains() {
    let mut game = prepared(15);
    let ids: Vec<_> = game
        .items
        .iter()
        .filter(|item| game.item_is_human_corpse(item))
        .map(|item| item.id.clone())
        .collect();
    for (id, floor) in ids.iter().take(2).zip([false, true]) {
        if floor {
            game.items
                .iter_mut()
                .find(|item| item.id == *id)
                .unwrap()
                .location = ItemLocation::Ground(START);
            game.mark_item_instances_discovered(std::slice::from_ref(id));
            assert!(
                game.items_dto()
                    .iter()
                    .find(|item| item.id == *id)
                    .unwrap()
                    .usable
            );
        }
        game.player.hp = 1;
        game.nutrition = if floor { 15_000 } else { 1000 };
        let count = game.items.len();
        // Inspect the item effect without the following world regeneration tick.
        game.use_inventory_item(
            id,
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.nutrition, 14_999);
        assert_eq!(game.player.hp, 1);
        assert_eq!(game.items.len(), count - 1);
        assert!(!game.items.iter().any(|item| item.id == *id));
    }
    let id = ids[2].clone();
    let valid = game
        .items
        .iter()
        .find(|item| item.id == id)
        .unwrap()
        .clone();
    for invalid in [0, 1, 2, 3] {
        let item = game.items.iter_mut().find(|item| item.id == id).unwrap();
        *item = valid.clone();
        match invalid {
            0 => item.origin_actor_kind_id = None,
            1 => item.origin_actor_kind_id = Some("demo.actor.sheep".to_owned()),
            2 => item.kind_id = "demo.item.skeleton-remains".to_owned(),
            _ => item.location = ItemLocation::Ground(EAST),
        }
        let before = game.to_save();
        use_corpse(&mut game, &id);
        // Dispatch sequencing changes independently of the rejected action.
        assert_eq!(game.nutrition, before.player.nutrition);
        assert!(game.world_tick > before.world_tick);
        assert!(game.items.iter().any(|item| item.id == id));
    }
    *game.items.iter_mut().find(|item| item.id == id).unwrap() = valid;
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let command = GameCommand::UseItem {
        item_id: id,
        target: None,
    };
    assert_eq!(
        dispatch_next(&mut game, command.clone()),
        dispatch_next(&mut restored, command)
    );
}

#[test]
fn ordinary_allocated_humans_drop_sacrificable_remains_after_real_melee() {
    let mut game = prepared(15);
    let policy = game
        .content
        .encounter_table("demo.encounter-table.warrens")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    // Existing early dungeon policy, including its nonpreferred-glyph penalty.
    // Isolate encounters on an open tile; no synthetic corpse or guaranteed drop.
    let mut found = None;
    for attempt in 0..2048 {
        let kind = game
            .select_original_allocated_monster(
                "demo.floor.warrens-depth-1",
                &policy,
                2,
                1,
                None,
                &[],
                None,
                None,
            )
            .unwrap();
        let definition = game.content.actor(&kind).unwrap();
        if !crate::game::hunger::actor_is_human_remains_source(definition)
            || definition.remains.is_none()
            || definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
        {
            continue;
        }
        game.push_generated_actor(format!("test.human.{attempt}"), &kind, EAST);
        for _ in 0..100 {
            if game.entities.is_empty() {
                break;
            }
            game.resolve_player_melee(
                0,
                false,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        }
        assert!(game.entities.is_empty());
        if let Some(item) = game.items.iter().find(|item| {
            item.location == ItemLocation::Ground(EAST) && game.item_is_human_corpse(item)
        }) {
            assert_eq!(item.origin_actor_kind_id.as_deref(), Some(kind.as_str()));
            found = Some(item.id.clone());
            break;
        }
    }
    let id = found.expect(
        "ordinary early dungeon allocation and probabilistic drops must supply human corpses",
    );
    game.player.position = EAST;
    game.nutrition = 1000;
    game.reveal_current_visibility();
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    let command = GameCommand::UseItem {
        item_id: id.clone(),
        target: None,
    };
    assert_eq!(
        dispatch_next(&mut game, command.clone()),
        dispatch_next(&mut restored, command)
    );
    assert_eq!(game.nutrition, 14_999);
    assert!(!game.items.iter().any(|item| item.id == id));
}

#[test]
fn balrog_native_food_potions_and_inn_keep_source_nutrition() {
    let mut game = prepared(15);
    for (kind, gain) in [
        ("demo.item.ration-of-food", 250),
        ("demo.item.water-potion", 10),
    ] {
        game.nutrition = 1000;
        give_inventory_item(&mut game, "test.food", kind);
        use_corpse(&mut game, "test.food");
        assert_eq!(game.nutrition, 1000 + gain);
    }
    let rng = game.rng.clone();
    assert_eq!(game.consume_inn_meal(&mut Vec::new()), "inn-food-meat");
    assert_eq!(game.nutrition, 14_999);
    assert_eq!(game.rng, rng);
}

#[test]
fn balrog_breath_unlocks_scales_branches_and_pays_on_failure() {
    let locked = prepared(14)
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == BREATH)
        .unwrap();
    assert!(!locked.can_cast);
    assert_eq!(locked.minimum_level, 15);
    for level in [15, 29, 30, 44, 45] {
        let mut game = prepared(level);
        let mut ability = game.content.ability(BREATH).unwrap().clone();
        Game::apply_player_level_scaling(&mut ability, level);
        assert!(ability.affects_ground_items);
        let AbilityEffectDefinition::RandomChoice { branches, .. } = &ability.effect else {
            panic!("random breath");
        };
        for branch in branches {
            let AbilityEffectDefinition::ConeDamage {
                damage_bonus,
                radius,
                ..
            } = branch.effect.as_ref()
            else {
                panic!("cone");
            };
            assert_eq!(*damage_bonus, level * 3);
            assert_eq!(u16::from(*radius), 1 + level / 15);
        }
        for branch in [None, Some(0), Some(1)] {
            game.entities.clear();
            game.entities.push(actor_from_runtime_spawn(
                "test.target",
                "demo.actor.sheep",
                EAST,
                10_000,
                1,
                100_000,
                true,
            ));
            game.player.hp = game.effective_player_max_hp();
            assert!(game.resources.is_empty());
            let seed = (0..10_000)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    let first = rng.bounded(100);
                    match branch {
                        None => first == 0,
                        Some(branch) => first == 99 && rng.bounded(2) == branch,
                    }
                })
                .unwrap();
            game.rng = RfbRng::seeded(seed);
            let hp = game.player.hp;
            game.resolve_player_ability(
                BREATH,
                TargetSelection::Direction {
                    direction: Direction::East,
                },
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(game.player.hp, hp - 10 - i32::from(level / 3));
            assert_eq!(
                game.entities[0].hp,
                10_000
                    - if branch.is_some() {
                        i32::from(level) * 3
                    } else {
                        0
                    }
            );
        }
        let before = game.to_save();
        game.resolve_player_ability(
            BREATH,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.to_save(), before);
        // Replace the damage-only synthetic target with a valid saved actor.
        clear_monsters(&mut game);
        game.push_generated_actor("test.saved-target".to_owned(), "demo.actor.sheep", EAST);
        game.reveal_current_visibility();
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(
            dispatch_next(&mut game, GameCommand::Wait),
            dispatch_next(&mut restored, GameCommand::Wait)
        );
    }
}

#[test]
fn balrog_breath_applies_spell_power_once_before_target_resistance() {
    let mut game = prepared(15);
    let mut status = monster_combat::melee_status("test.spell-power", 100, "test").status;
    status.granted_modifiers.spell_power_bonus = 7;
    game.player.statuses.push(status);
    for (roll, kind) in [(0, DamageType::Nether), (1, DamageType::Fire)] {
        game.entities.clear();
        let mut actor = actor_from_runtime_spawn(
            "test.target",
            "demo.actor.sheep",
            EAST,
            10_000,
            1,
            100_000,
            true,
        );
        actor.resistances.set(kind, ResistanceLevel::Resistant);
        game.entities.push(actor);
        game.player.hp = game.effective_player_max_hp();
        game.rng = RfbRng::seeded(
            (0..10_000)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    rng.bounded(100) == 99 && rng.bounded(2) == roll
                })
                .unwrap(),
        );
        game.resolve_player_ability(
            BREATH,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        let raw = ability_scaling::spell_power_value(45, 7) as i32;
        let expected =
            resolve_damage(DamagePacket::new(raw, kind), ResistanceLevel::Resistant).applied;
        assert_eq!(game.entities[0].hp, 10_000 - expected);
    }
}

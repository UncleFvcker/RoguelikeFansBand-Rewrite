// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

const BOLT: &str = "demo.ability.cinder-bolt";

fn arena() -> Game {
    arena_with_spell(BOLT)
}

pub(super) fn arena_with_spell(spell_id: &str) -> Game {
    let mut game = game_with_actor_definition(509, "demo.actor.horse", |actor| {
        actor.monster_casting = Some(rfb_content::MonsterCastingDefinition {
            frequency_percent: 100,
            smart: true,
            preferred_distance: None,
            flee_hp_percent: 0,
            abilities: vec![rfb_content::MonsterAbilityCandidateDefinition {
                ability_id: spell_id.into(),
                weight: 1,
                innate: false,
            }],
        });
    });
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 70, y: 30 };
    for y in 24..=36 {
        for x in 64..=80 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.push_generated_actor("pet".into(), "demo.actor.horse", Position { x: 71, y: 30 });
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.entities[0].nice = false;
    game.entities[0].statuses.clear();
    game.push_generated_actor(
        "enemy".into(),
        "demo.actor.adobe-golem",
        Position { x: 74, y: 30 },
    );
    game.glow.fill(true);
    game.reveal_current_visibility();
    game
}

fn ability(game: &Game, effect: AbilityEffectDefinition) -> AbilityDefinition {
    let mut ability = game.content.ability(BOLT).unwrap().clone();
    ability.effect = effect;
    ability
}

fn area() -> AbilityEffectDefinition {
    AbilityEffectDefinition::AreaDamage {
        damage_dice: 0,
        damage_sides: 0,
        damage_bonus: 12,
        damage_type: rfb_content::ActorDamageType::Physical,
        radius: 2,
        target_category: None,
    }
}

fn cast(game: &mut Game, ability: AbilityDefinition) -> MonsterAbilityPlanResolution {
    let plan = game.monster_ability_plan(0, ability, 1).unwrap();
    game.resolve_monster_ability_plan(
        0,
        "demo.actor.horse",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
}

#[test]
fn pet_casts_before_moving_and_obeys_selected_target_distance_and_obstruction() {
    let mut game = arena();
    game.push_generated_actor(
        "chosen".into(),
        "demo.actor.adobe-golem",
        Position { x: 71, y: 26 },
    );
    game.summon_command.target_actor_id = Some("chosen".into());
    let before = game.entities[0].position;
    let mut events = Vec::new();
    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(game.entities[0].position, before);
    assert!(events.iter().any(|event| matches!(event,
        DomainEvent::MonsterAbilityCast { resolution, .. } if resolution.target_entity_id == "chosen")));
    game.summon_command.target_actor_id = None;
    game.summon_command.mode = SummonCommandModeDto::StayClose;
    let bolt = game.content.ability(BOLT).unwrap().clone();
    assert!(game.monster_ability_plan(0, bolt.clone(), 1).is_err());
    game.summon_command.target_actor_id = Some("enemy".into());
    assert!(game.monster_ability_plan(0, bolt.clone(), 1).is_ok());
    replace_terrain(&mut game, Position { x: 72, y: 30 }, "demo.terrain.wall");
    assert!(game.monster_ability_plan(0, bolt, 1).is_err());
    assert_eq!(
        game.summon_command.target_actor_id.as_deref(),
        Some("enemy")
    );
}

#[test]
fn spell_options_are_free_saved_and_do_not_disable_self_healing() {
    let mut game = arena();
    assert!(game.summon_command.attack_spells && game.summon_command.teleport);
    assert!(!game.summon_command.allow_player_damage);
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    for (option, enabled) in [
        (rfb_protocol::PetOptionDto::AttackSpells, false),
        (rfb_protocol::PetOptionDto::Teleport, false),
        (rfb_protocol::PetOptionDto::AllowPlayerDamage, true),
    ] {
        dispatch_next(&mut game, GameCommand::SetPetOption { option, enabled });
    }
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
    let restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    assert_eq!(restored.summon_command, game.summon_command);
    assert_eq!(restored.state_hash(), game.state_hash());
    let bolt = game.content.ability(BOLT).unwrap().clone();
    assert!(game.monster_ability_plan(0, bolt, 1).is_err());
    for effect in [
        AbilityEffectDefinition::BlinkSelf {
            radius: 3,
            line_of_sight: false,
        },
        AbilityEffectDefinition::TeleportTarget,
    ] {
        assert!(
            game.monster_ability_plan(0, ability(&game, effect), 1)
                .is_err()
        );
    }
    game.entities[0].hp = 1;
    let heal = ability(&game, AbilityEffectDefinition::Heal { amount: 10 });
    cast(&mut game, heal.clone());
    assert_eq!(game.entities[0].hp, 11);
    let haste = game
        .content
        .ability("rfb-legacy.ability.haste-self")
        .unwrap()
        .clone();
    let status_count = game.entities[0].statuses.len();
    cast(&mut game, haste.clone());
    assert!(game.entities[0].statuses.len() > status_count);
    assert!(game.monster_ability_plan(0, haste, 1).is_err());
    game.entities.retain(|actor| actor.id == "pet");
    assert!(
        game.monster_ability_plan(0, heal, 1).is_err(),
        "self spells still need a projectable enemy"
    );
}

#[test]
fn collateral_permission_changes_real_area_damage_but_keeps_other_friends_safe() {
    let mut game = arena();
    game.player.position = Position { x: 74, y: 31 };
    let spell = ability(&game, area());
    assert_eq!(
        game.monster_ability_plan(0, spell.clone(), 1)
            .unwrap_err()
            .reason,
        MonsterAbilityRejectionReasonDto::FriendlyRisk
    );
    game.summon_command.allow_player_damage = true;
    game.push_generated_actor(
        "friend".into(),
        "demo.actor.horse",
        Position { x: 75, y: 30 },
    );
    game.entities.last_mut().unwrap().controller_id = Some(game.player.id.clone());
    assert_eq!(
        game.monster_ability_plan(0, spell.clone(), 1)
            .unwrap_err()
            .reason,
        MonsterAbilityRejectionReasonDto::FriendlyRisk
    );
    game.entities.pop();
    let hp = game.player.hp;
    let resolution = cast(&mut game, spell);
    assert!(game.player.hp < hp);
    assert!(
        resolution
            .targets
            .iter()
            .any(|target| target.target_entity_id == game.player.id)
    );
    assert!(
        resolution
            .targets
            .iter()
            .any(|target| target.target_entity_id == "enemy")
    );
}

#[test]
fn beam_and_breath_hit_player_only_with_permission_while_bolts_never_shoot_through_player() {
    let base = arena();
    for effect in [
        AbilityEffectDefinition::BeamDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 12,
            damage_type: rfb_content::ActorDamageType::Physical,
            maximum_range: None,
        },
        AbilityEffectDefinition::BreathDamage {
            hp_percent: 100,
            max_damage: 12,
            damage_type: rfb_content::ActorDamageType::Physical,
            radius: 2,
        },
    ] {
        let mut game = base.clone();
        game.player.position = Position { x: 73, y: 30 };
        let spell = ability(&game, effect);
        assert!(game.monster_ability_plan(0, spell.clone(), 1).is_err());
        game.summon_command.allow_player_damage = true;
        let hp = game.player.hp;
        cast(&mut game, spell);
        assert!(game.player.hp < hp);
        let bolt = game.content.ability(BOLT).unwrap().clone();
        assert!(game.monster_ability_plan(0, bolt, 1).is_err());
    }
}

#[test]
fn mount_casts_on_its_energy_schedule_and_self_teleport_keeps_rider_together() {
    let mut game = arena();
    game.entities[0].position = game.player.position;
    game.riding_actor_id = Some("pet".into());
    game.entities[0].energy_need = 0;
    let mut events = Vec::new();
    game.continue_monster_energy_pulse(
        vec!["pet".into()],
        BTreeSet::new(),
        false,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterAbilityCast { .. }))
    );
    assert!(game.entities[0].energy_need > 0);
    assert_eq!(game.entities[0].position, game.player.position);
    game.summon_command.target_actor_id = Some("enemy".into());
    let blink = ability(
        &game,
        AbilityEffectDefinition::BlinkSelf {
            radius: 3,
            line_of_sight: false,
        },
    );
    let from = game.player.position;
    cast(&mut game, blink);
    assert_ne!(game.player.position, from);
    assert_eq!(game.entities[0].position, game.player.position);
    assert_eq!(game.riding_actor_id.as_deref(), Some("pet"));
    give_inventory_item(&mut game, "test.no-teleport", "demo.item.chain-mail");
    let armor = game.items.last_mut().unwrap();
    armor.location = ItemLocation::Equipped {
        slot_id: "body".into(),
    };
    armor
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::AntiTeleport);
    assert!(game.player_has_anti_teleport());
    let blink = ability(
        &game,
        AbilityEffectDefinition::BlinkSelf {
            radius: 3,
            line_of_sight: false,
        },
    );
    assert_eq!(
        game.monster_ability_plan(0, blink, 1).unwrap_err().reason,
        MonsterAbilityRejectionReasonDto::NoSpace
    );
}

#[test]
fn mount_area_damage_never_hurts_its_rider_and_blast_hits_unselected_enemies() {
    let mut game = arena();
    game.entities[0].position = game.player.position;
    game.riding_actor_id = Some("pet".into());
    game.entities[1].position = Position { x: 72, y: 30 };
    game.push_generated_actor(
        "collateral".into(),
        "demo.actor.adobe-golem",
        Position { x: 73, y: 30 },
    );
    game.summon_command.target_actor_id = Some("enemy".into());
    game.summon_command.mode = SummonCommandModeDto::StayClose;
    for enabled in [false, true] {
        let mut copy = game.clone();
        copy.summon_command.allow_player_damage = enabled;
        let spell = ability(&copy, area());
        let hp = copy.player.hp;
        let resolution = cast(&mut copy, spell);
        assert_eq!(copy.player.hp, hp);
        assert!(
            resolution
                .targets
                .iter()
                .all(|target| target.target_entity_id != copy.player.id)
        );
        assert!(
            resolution
                .targets
                .iter()
                .any(|target| target.target_entity_id == "collateral")
        );
    }
}

#[test]
fn nice_cooldown_confusion_and_no_magic_preserve_casting_rng_boundaries() {
    let mut game = arena();
    game.entities[0].nice = true;
    let before = game.rng_draw_counter();
    assert!(!game.resolve_monster_ability(0, &mut Vec::new()));
    assert_eq!(game.rng_draw_counter(), before);
    game.entities[0].nice = false;
    game.entities[0].casting_cooldown_remaining = 1;
    assert!(!game.resolve_monster_ability(0, &mut Vec::new()));
    assert_eq!(game.rng_draw_counter(), before);
    assert_eq!(game.entities[0].casting_cooldown_remaining, 0);
    game.entities[0]
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 10, "test").status);
    assert!(!game.resolve_monster_ability(0, &mut Vec::new()));
    assert_eq!(game.rng_draw_counter(), before + 1);
    game.entities[0].statuses.clear();
    game.current_floor_id = "demo.floor.anti-magic-cave-depth-40".into();
    assert!(game.dungeon_blocks_magic());
    assert!(!game.resolve_monster_ability(0, &mut Vec::new()));
    assert_eq!(game.rng_draw_counter(), before + 2);
    assert_eq!(game.entities[0].casting_cooldown_remaining, 0);
}

#[test]
fn forbidden_spells_and_disabled_summons_are_filtered_inside_sequences() {
    let mut game = arena();
    for effect in [
        AbilityEffectDefinition::AggravateMonsters,
        AbilityEffectDefinition::DarkenRoom,
        AbilityEffectDefinition::Amnesia,
        AbilityEffectDefinition::TeleportLevel,
        AbilityEffectDefinition::NoOp {
            reason: "unsupported special".into(),
        },
    ] {
        let spell = ability(
            &game,
            AbilityEffectDefinition::Sequence {
                effects: vec![AbilityEffectDefinition::Heal { amount: 10 }, effect],
            },
        );
        assert!(!game.pet_spell_allowed(&spell));
    }
    let summon = game
        .content
        .abilities()
        .find(|ability| {
            matches!(
                ability.effect,
                AbilityEffectDefinition::Summon { .. }
                    | AbilityEffectDefinition::SummonCategory { .. }
            )
        })
        .unwrap();
    assert!(game.pet_spell_allowed(summon));
    let summon = summon.clone();
    game.summon_command.summon_spells = false;
    assert!(!game.pet_spell_allowed(&summon));
    let jump = ability(
        &game,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 12,
            damage_multiplier_numerator: 1,
            damage_multiplier_denominator: 1,
            damage_type: rfb_content::ActorDamageType::Physical,
            radius: 5,
            blink_radius: 3,
        },
    );
    game.summon_command.attack_spells = false;
    assert!(
        game.pet_spell_allowed(&jump),
        "jump attacks belong to the tactical group"
    );
    game.summon_command.teleport = false;
    assert!(!game.pet_spell_allowed(&jump));
}

#[test]
fn mounted_bird_escape_uses_the_same_rider_relocation_as_teleport_spells() {
    let mut game = arena();
    game.entities[0].position = game.player.position;
    game.riding_actor_id = Some("pet".into());
    let spell = ability(&game, AbilityEffectDefinition::BirdDrop);
    let mut plan = game.monster_ability_plan(0, spell, 1).unwrap();
    let MonsterAbilityTargetPlan::BirdDrop {
        escape_destinations,
        ..
    } = &mut plan.target
    else {
        panic!("bird plan");
    };
    let landing = escape_destinations[0];
    *escape_destinations = vec![landing];
    let seed = (0..100)
        .find(|seed| crate::rng::RfbRng::seeded(*seed).bounded(3) == 0)
        .unwrap();
    game.rng = crate::rng::RfbRng::seeded(seed);
    game.resolve_monster_ability_plan(
        0,
        "demo.actor.horse",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(game.player.position, landing);
    assert_eq!(game.entities[0].position, landing);
    assert_eq!(game.riding_actor_id.as_deref(), Some("pet"));
}

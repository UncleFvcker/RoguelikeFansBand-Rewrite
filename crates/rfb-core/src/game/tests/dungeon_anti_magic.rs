// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use std::sync::OnceLock;

const FLOOR: &str = "demo.floor.castle-depth-40";
const CASTER: &str = "demo.actor.ash-drake";
const BREATH: &str = "demo.ability.ash-breath";
const MAGIC: &str = "demo.ability.cinder-bolt";

fn catalog() -> Arc<ContentCatalog> {
    static CONTENT: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packs/rfb-demo-original");
            let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
            enable_test_caster(&mut artifact.content);
            artifact.content.worlds[0]
                .dungeons
                .iter_mut()
                .find(|d| d.id == "demo.dungeon.castle")
                .unwrap()
                .no_magic = true;
            let actor = artifact
                .content
                .actors
                .iter_mut()
                .find(|a| a.id == CASTER)
                .unwrap();
            let casting = actor.monster_casting.as_mut().unwrap();
            casting.frequency_percent = 100;
            let mut magic = casting.abilities[0].clone();
            magic.ability_id = MAGIC.to_owned();
            magic.innate = false;
            casting.abilities.push(magic);
            Arc::new(ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content).unwrap(),
            ))
        })
        .clone()
}

pub(super) fn enter_context(game: &mut Game) {
    game.content = catalog();
    game.current_floor_id = FLOOR.to_owned();
    assert!(game.dungeon_blocks_magic());
}

fn caster_game() -> Game {
    let mut game =
        Game::from_content_with_build(1, catalog(), DEFAULT_WORLD_ID, "demo.build.warrior")
            .unwrap();
    clear_monsters(&mut game);
    game.player.position = Position { x: 5, y: 5 };
    for y in 3..=12 {
        for x in 3..=12 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.push_generated_actor("test.caster".to_owned(), CASTER, Position { x: 9, y: 5 });
    game.entities[0].nice = false;
    game.entities[0].alerted = true;
    game
}

fn decision(events: &[DomainEvent]) -> &MonsterAbilityDecisionResolutionDto {
    events
        .iter()
        .find_map(|event| match event {
            DomainEvent::MonsterAbilityDecision { resolution } => Some(resolution),
            _ => None,
        })
        .unwrap()
}

#[test]
fn dungeon_anti_magic_player_gate_matches_projection_without_cost_and_restores() {
    let spell = "demo.ability.death-berserk";
    let mut game = prepare_death_caster(7, 40, spell);
    enter_context(&mut game);
    choose_human_talent_if_pending(&mut game);
    let mana = game.resources["demo.resource.mana"].current;
    let hp = game.player.hp;
    let draws = game.rng_draw_counter();
    let tick = game.world_tick;
    let mut events = Vec::new();
    game.resolve_player_ability(
        spell,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        matches!(events.as_slice(), [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "anti-magic")
    );
    assert_eq!(
        (
            game.resources["demo.resource.mana"].current,
            game.player.hp,
            game.rng_draw_counter(),
            game.world_tick
        ),
        (mana, hp, draws, tick)
    );
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|a| a.id == spell)
            .unwrap()
            .can_cast
    );
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: spell.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(
        (
            game.resources["demo.resource.mana"].current,
            game.player.hp,
            game.rng_draw_counter(),
            game.world_tick
        ),
        (mana, hp, draws, tick)
    );
    game.current_floor_id = "demo.floor.surface".to_owned();
    assert!(!game.dungeon_blocks_player_ability(spell));
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|a| a.id == spell)
            .unwrap()
            .can_cast
    );
    game.debug_set_ability_casts_succeed(true);
    game.resolve_player_ability(
        spell,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.player_has_status_kind(STATUS_BERSERK));

    let mut sniper =
        Game::from_content_with_build(1, catalog(), DEFAULT_WORLD_ID, "demo.build.sniper").unwrap();
    sniper.current_floor_id = FLOOR.to_owned();
    sniper.progress.level = 50;
    let class = sniper.content.class("demo.class.sniper").unwrap();
    assert_eq!(
        class
            .abilities
            .iter()
            .filter(|a| a.blocked_by_dungeon_anti_magic)
            .count(),
        16
    );
    for activation in &class.abilities {
        assert_eq!(
            sniper.dungeon_blocks_player_ability(&activation.ability_id),
            activation.ability_id != "demo.ability.sniper-probe-monsters"
        );
    }
    let mut events = Vec::new();
    let draws = sniper.rng_draw_counter();
    sniper
        .resolve_player_ability(
            "demo.ability.sniper-concentrate",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert!(
        matches!(events.as_slice(), [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "anti-magic")
    );
    assert_eq!(sniper.rng_draw_counter(), draws);
    assert!(
        !sniper
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|a| a.id == "demo.ability.sniper-concentrate")
            .unwrap()
            .can_cast
    );
    assert!(
        sniper
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|a| a.id == "demo.ability.sniper-probe-monsters")
            .unwrap()
            .can_cast
    );
    let mut events = Vec::new();
    sniper.debug_set_ability_casts_succeed(true);
    sniper
        .resolve_player_ability(
            "demo.ability.sniper-probe-monsters",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, DomainEvent::AbilityCastUnavailable { .. }))
    );
}

#[test]
fn dungeon_anti_magic_preserves_race_power_execution() {
    let mut game =
        Game::from_content_with_build(7, catalog(), DEFAULT_WORLD_ID, "demo.build.warrior")
            .unwrap();
    game.current_floor_id = FLOOR.to_owned();
    game.build.as_mut().unwrap().race_id = "rfb-legacy.race.barbarian".to_owned();
    game.progress.level = 20;
    game.debug_set_ability_casts_succeed(true);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|a| a.id == "rfb.ability.race.berserk")
            .unwrap()
            .can_cast
    );
    game.resolve_player_ability(
        "rfb.ability.race.berserk",
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(game.player_has_status_kind(STATUS_BERSERK));
}

#[test]
fn dungeon_anti_magic_enemy_draws_normally_and_rejects_without_reroll_or_cooldown() {
    let base = caster_game();
    let mut observed = BTreeSet::new();
    for seed in 0..32 {
        let mut normal = base.clone();
        normal.rng = RfbRng::seeded(seed);
        let mut blocked = normal.clone();
        blocked.current_floor_id = FLOOR.to_owned();
        let mut normal_events = Vec::new();
        let mut blocked_events = Vec::new();
        assert!(normal.resolve_monster_ability(0, &mut normal_events));
        let cast = blocked.resolve_monster_ability(0, &mut blocked_events);
        assert_eq!(decision(&normal_events), decision(&blocked_events));
        let chosen = decision(&blocked_events)
            .selected_ability_id
            .as_deref()
            .unwrap();
        observed.insert(chosen.to_owned());
        assert_eq!(cast, chosen == BREATH);
        if chosen == MAGIC {
            assert_eq!(blocked.rng_draw_counter(), 2);
            assert_eq!(blocked.entities[0].casting_cooldown_remaining, 0);
            assert_eq!(blocked.player.hp, base.player.hp);
            // The real action caller keeps its ordinary movement after rejection.
            let mut action = base.clone();
            action.current_floor_id = FLOOR.to_owned();
            action.rng = RfbRng::seeded(seed);
            action
                .resolve_monster_action(
                    0,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                )
                .unwrap();
            assert!(action.entities[0].position.x < base.entities[0].position.x);
        } else {
            assert_eq!(blocked.player.hp, normal.player.hp);
            assert_eq!(blocked.rng_draw_counter(), normal.rng_draw_counter());
        }
    }
    assert_eq!(
        observed,
        BTreeSet::from([BREATH.to_owned(), MAGIC.to_owned()])
    );
}

#[test]
fn dungeon_anti_magic_monster_targets_friends_and_runtime_forms_follow_source_ai() {
    for allegiance in 0..3 {
        let mut friendly = caster_game();
        friendly.current_floor_id = FLOOR.to_owned();
        match allegiance {
            0 => friendly.entities[0].controller_id = Some(friendly.player.id.clone()),
            1 => friendly.entities[0].friendly = true,
            _ => {}
        }
        friendly.push_generated_actor(
            "test.enemy".to_owned(),
            "demo.actor.cave-orc",
            Position { x: 5, y: 5 },
        );
        friendly.player.position = Position { x: 5, y: 10 };
        if allegiance == 2 {
            friendly.entities[1].controller_id = Some(friendly.player.id.clone());
        }
        let mut events = Vec::new();
        assert!(friendly.resolve_monster_ability(0, &mut events));
        assert_eq!(decision(&events).viable_ability_ids, [BREATH]);
        assert_eq!(
            decision(&events)
                .candidates
                .iter()
                .find(|c| c.ability_id == MAGIC)
                .unwrap()
                .effective_weight,
            0
        );
    }
    // STUPID is read from the current form. Blinking dot's magic survives
    // monster-to-monster AI/failure checks, but still cannot target the player.
    let mut stupid = caster_game();
    stupid.current_floor_id = FLOOR.to_owned();
    stupid.entities[0].kind_id = "demo.actor.chameleon".to_owned();
    stupid.entities[0].appearance_kind_id = Some("demo.actor.blinking-dot".to_owned());
    stupid.entities[0].controller_id = Some(stupid.player.id.clone());
    stupid.push_generated_actor(
        "test.enemy".to_owned(),
        "demo.actor.cave-orc",
        Position { x: 5, y: 5 },
    );
    stupid.player.position = Position { x: 5, y: 10 };
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 50)
        .unwrap();
    stupid.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    assert!(stupid.resolve_monster_ability(0, &mut events));
    let mut hostile = stupid.clone();
    hostile.entities[0].controller_id = None;
    hostile.entities.truncate(1);
    hostile.entities[0].casting_cooldown_remaining = 0;
    hostile.rng = RfbRng::seeded(seed);
    assert!(!hostile.resolve_monster_ability(0, &mut Vec::new()));

    let mut changed = caster_game();
    changed.current_floor_id = FLOOR.to_owned();
    changed.entities[0].kind_id = "demo.actor.chameleon".to_owned();
    changed.entities[0].appearance_kind_id = Some(CASTER.to_owned());
    changed.rng = RfbRng::seeded(0);
    let mut events = Vec::new();
    let cast = changed.resolve_monster_ability(0, &mut events);
    assert_eq!(
        cast,
        decision(&events).selected_ability_id.as_deref() == Some(BREATH)
    );
}

#[test]
fn dungeon_anti_magic_preserves_the_formal_rocket() {
    let mut normal = caster_game();
    normal.entities[0].kind_id = "demo.actor.cyberdemon".to_owned();
    normal.player.hp = 10_000;
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < 15)
        .unwrap();
    normal.rng = RfbRng::seeded(seed);
    let mut blocked = normal.clone();
    blocked.current_floor_id = FLOOR.to_owned();
    assert!(normal.resolve_monster_ability(0, &mut Vec::new()));
    assert!(blocked.resolve_monster_ability(0, &mut Vec::new()));
    assert!(blocked.player.hp < 10_000);
    assert_eq!(blocked.player.hp, normal.player.hp);
    assert_eq!(blocked.rng_draw_counter(), normal.rng_draw_counter());
}

#[test]
fn dungeon_anti_magic_generation_ambient_and_summon_allocation_share_target_context() {
    let mut game =
        Game::from_content_with_build(1, catalog(), DEFAULT_WORLD_ID, "demo.build.warrior")
            .unwrap();
    let floor = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|f| f.id == FLOOR)
        .unwrap()
        .clone();
    assert!(!game.dungeon_blocks_magic());
    let generated = game.generate_procedural_floor(&floor, None).unwrap();
    assert!(!generated.entities.is_empty());
    assert!(
        generated
            .entities
            .iter()
            .all(|a| game.dungeon_allows_monster(
                FLOOR,
                game.content.actor(&a.kind_id).unwrap(),
                false
            ))
    );
    game.current_floor_id = FLOOR.to_owned();
    for (kind, allowed) in [
        ("yeti", true),
        ("shrieker-mushroom-patch", true),
        ("chameleon", true),
        ("kobold", false),
        ("novice-mage", false),
    ] {
        let actor = game.content.actor(&format!("demo.actor.{kind}")).unwrap();
        assert_eq!(
            game.dungeon_allows_monster(FLOOR, actor, false),
            allowed,
            "{kind}"
        );
    }
    let yeti = game.content.actor("demo.actor.yeti").unwrap();
    assert!(yeti.monster_casting.is_none()); // Possessor-only BERSERK still qualifies.
    let candidates = game.summon_category_candidate_kind_ids("any-monster", None, 80, false, true);
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|id| game.dungeon_allows_monster(
        FLOOR,
        game.content.actor(id).unwrap(),
        true
    )));
    for scroll in [
        "demo.item.summoning-scroll",
        "demo.item.pet-summoning-scroll",
    ] {
        let mut summoned = game.clone();
        clear_monsters(&mut summoned);
        summoned.player.position = Position { x: 10, y: 10 };
        for y in 8..=12 {
            for x in 8..=12 {
                replace_terrain(&mut summoned, Position { x, y }, "demo.terrain.floor");
            }
        }
        give_inventory_item(&mut summoned, "test.summon", scroll);
        summoned
            .use_inventory_item(
                "test.summon",
                None,
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert!(!summoned.entities.is_empty(), "{scroll}");
        assert!(summoned.entities.iter().all(|a| {
            summoned.dungeon_allows_monster(
                FLOOR,
                summoned.content.actor(&a.kind_id).unwrap(),
                true,
            )
        }));
        assert!(
            summoned
                .entities
                .iter()
                .all(|a| summoned.actor_is_player_side(a)
                    == (scroll == "demo.item.pet-summoning-scroll"))
        );
    }
    clear_monsters(&mut game);
    let policy = game
        .content
        .encounter_table(floor.encounter_table_id.as_ref().unwrap())
        .unwrap()
        .global_allocation
        .as_ref()
        .unwrap();
    let chance = u32::from(policy.ambient_chance_one_in) * (u32::from(floor.depth) + 100) / 100;
    let seed = (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(u64::from(chance)) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.process_ambient_monster_allocation(&mut BTreeSet::new())
        .unwrap();
    assert!(!game.entities.is_empty());
    assert!(game.entities.iter().all(|a| game.dungeon_allows_monster(
        FLOOR,
        game.content.actor(&a.kind_id).unwrap(),
        false
    )));
}

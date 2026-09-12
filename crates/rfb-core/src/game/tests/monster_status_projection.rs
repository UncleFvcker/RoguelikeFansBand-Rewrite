// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_content::MonsterStatusProjectionDefinition::{self, Confusion, Fear, Sleep, Stasis};

fn control_game() -> Game {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut content = rfb_content::compile_pack_dir(&path).unwrap().content;
    let goblin = content
        .actors
        .iter()
        .find(|a| a.id == "demo.actor.goblin")
        .unwrap()
        .clone();
    for (suffix, tags, immunities) in [
        ("ordinary", vec![], vec![]),
        ("unique", vec!["unique"], vec![]),
        ("resist-all", vec!["resist-all"], vec![]),
        ("invisible", vec!["invisible"], vec![]),
        (
            "immune",
            vec![],
            vec![STATUS_SLEEP, STATUS_CONFUSION, STATUS_FEAR],
        ),
    ] {
        let mut actor = goblin.clone();
        actor.id = format!("test.actor.{suffix}");
        actor.allocation = None;
        actor.tags = tags.into_iter().map(str::to_owned).collect();
        actor.status_immunities = immunities.into_iter().map(str::to_owned).collect();
        content.actors.push(actor);
    }
    let mut game = Game::new_with_build(481, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(content).unwrap(),
    ));
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=15 {
        for x in 8..=30 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game
}

fn project(
    game: &mut Game,
    projection: MonsterStatusProjectionDefinition,
    power: u16,
) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_projected_monster_status(
        "test.control",
        projection,
        power,
        &mut events,
        &mut BTreeSet::new(),
    );
    events
}

fn change(events: &[DomainEvent]) -> AbilityStatusChangeDto {
    events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                resolution.effects.iter().find_map(|effect| {
                    if let AbilityEffectResolutionDto::ApplyStatus { change, .. } = effect {
                        Some(*change)
                    } else {
                        None
                    }
                })
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn project_hack_targets_dark_invisible_actors_but_obeys_walls_and_range_and_save_order() {
    let mut game = control_game();
    game.glow.fill(false);
    for (id, kind, position) in [
        ("test.z-near", "invisible", Position { x: 13, y: 10 }),
        ("test.a-range18", "ordinary", Position { x: 28, y: 10 }),
        ("test.range19", "ordinary", Position { x: 29, y: 10 }),
        ("test.behind-wall", "ordinary", Position { x: 10, y: 14 }),
    ] {
        game.push_generated_actor(id.into(), &format!("test.actor.{kind}"), position);
    }
    replace_terrain(&mut game, Position { x: 10, y: 12 }, "demo.terrain.wall");
    assert!(!game.entity_is_visible_to_player(&game.entities[0]));
    assert_eq!(
        game.projected_monster_status_targets(),
        ["test.a-range18", "test.z-near"]
    );
    game.entities[0].hp = 0;
    assert_eq!(game.projected_monster_status_targets(), ["test.a-range18"]);
    game.entities[0].hp = game.entities[0].max_hp;
    game.reveal_current_visibility();
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(
        project(&mut game, Sleep, 60),
        project(&mut restored, Sleep, 60)
    );
    assert_eq!(game.rng, restored.rng);
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn successful_sleep_targets_friends_and_pets_with_source_anger_rng() {
    let mut template = control_game();
    template.push_generated_actor(
        "test.target".into(),
        "test.actor.ordinary",
        Position { x: 12, y: 10 },
    );
    let seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(5) < rng.bounded(60)
        })
        .unwrap();
    template.rng = RfbRng::seeded(seed);
    let mut expected = template.rng.clone();
    expected.bounded(5);
    expected.bounded(60);
    let mut friendly = template.clone();
    friendly.entities[0].friendly = true;
    project(&mut friendly, Sleep, 60);
    assert!(!friendly.entities[0].friendly);
    assert_eq!(friendly.entities[0].statuses[0].remaining_ticks, 500);
    assert_eq!(friendly.rng, expected);
    let mut pet = template;
    pet.entities[0].controller_id = Some(pet.player.id.clone());
    expected.bounded(3);
    project(&mut pet, Sleep, 60);
    assert_eq!(pet.entities[0].controller_id.as_ref(), Some(&pet.player.id));
    assert_eq!(pet.entities[0].statuses[0].remaining_ticks, 500);
    assert_eq!(pet.rng, expected);
}

#[test]
fn projection_immunities_have_distinct_rng_and_stasis_ignores_no_sleep() {
    let template = control_game();
    for projection in [Fear, Confusion, Sleep, Stasis] {
        let mut game = template.clone();
        game.push_generated_actor(
            "test.target".into(),
            "test.actor.resist-all",
            Position { x: 12, y: 10 },
        );
        let rng = game.rng.clone();
        assert_eq!(
            change(&project(&mut game, projection, 90)),
            AbilityStatusChangeDto::Immune
        );
        assert_eq!(game.rng, rng, "RES_ALL rejects before all effect RNG");
    }
    let mut immune = template.clone();
    immune.push_generated_actor(
        "test.target".into(),
        "test.actor.immune",
        Position { x: 12, y: 10 },
    );
    for projection in [Fear, Confusion] {
        let mut expected = immune.rng.clone();
        for _ in 0..3 {
            expected.bounded(45);
        }
        assert_eq!(
            change(&project(&mut immune, projection, 90)),
            AbilityStatusChangeDto::Immune
        );
        assert_eq!(
            immune.rng, expected,
            "NO_FEAR/NO_CONF follow the duration draw"
        );
    }
    let rng = immune.rng.clone();
    assert_eq!(
        change(&project(&mut immune, Sleep, 60)),
        AbilityStatusChangeDto::Immune
    );
    assert_eq!(immune.rng, rng);
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(92) + 1 >= 5)
        .unwrap();
    immune.rng = RfbRng::seeded(seed);
    let mut expected = immune.rng.clone();
    expected.bounded(92);
    let ticks = if expected.bounded(15) == 0 { 30 } else { 20 };
    project(&mut immune, Stasis, 92);
    assert_eq!(immune.entities[0].statuses[0].remaining_ticks, ticks);
    assert_eq!(immune.rng, expected);
    project(&mut immune, Stasis, 1000);
    assert_eq!(
        immune.entities[0].statuses[0].remaining_ticks, ticks,
        "stasis never refreshes"
    );
    let mut unique = template;
    unique.push_generated_actor(
        "test.target".into(),
        "test.actor.unique",
        Position { x: 12, y: 10 },
    );
    for projection in [Sleep, Stasis] {
        let rng = unique.rng.clone();
        assert_eq!(
            change(&project(&mut unique, projection, 92)),
            AbilityStatusChangeDto::Immune
        );
        assert_eq!(unique.rng, rng);
    }
    let events = project(&mut unique, Confusion, 90);
    assert_ne!(
        change(&events),
        AbilityStatusChangeDto::Immune,
        "unique is not NO_CONF"
    );
    assert_ne!(
        change(&project(&mut unique, Fear, 90)),
        AbilityStatusChangeDto::Immune,
        "unique is not NO_FEAR"
    );
}

#[test]
fn sleep_save_rolls_monster_first_confusion_caps_strength_and_halves_repeat_fear_adds() {
    let mut template = control_game();
    template.push_generated_actor(
        "test.target".into(),
        "test.actor.ordinary",
        Position { x: 12, y: 10 },
    );
    let mut sleep = template.clone();
    sleep.rng = RfbRng::seeded(13);
    let mut expected = sleep.rng.clone();
    let target = expected.bounded(5) + 1;
    let power = expected.bounded(60) + 1;
    let events = project(&mut sleep, Sleep, 60);
    assert_eq!(sleep.rng, expected);
    if target >= power {
        assert_eq!(change(&events), AbilityStatusChangeDto::Resisted);
        assert!(sleep.entities[0].statuses.is_empty());
    } else {
        assert_eq!(sleep.entities[0].statuses[0].remaining_ticks, 500);
        sleep.entities[0].statuses[0].remaining_ticks = 999;
        sleep.rng = RfbRng::seeded(13);
        project(&mut sleep, Sleep, 60);
        assert_eq!(
            sleep.entities[0].statuses[0].remaining_ticks, 500,
            "sleep replaces its counter"
        );
    }
    let mut confusion = template.clone();
    let seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            for _ in 0..3 {
                rng.bounded(50);
            }
            rng.bounded(5) < rng.bounded(100)
        })
        .unwrap();
    confusion.rng = RfbRng::seeded(seed);
    let mut expected = confusion.rng.clone();
    let duration: u32 = (0..3).map(|_| expected.bounded(50) as u32 + 1).sum::<u32>() + 1;
    expected.bounded(5);
    expected.bounded(100);
    project(&mut confusion, Confusion, 1000);
    assert_eq!(confusion.rng, expected);
    assert_eq!(confusion.entities[0].statuses[0].remaining_ticks, duration);
    confusion.rng = RfbRng::seeded(seed);
    project(&mut confusion, Confusion, 1000);
    assert_eq!(
        confusion.entities[0].statuses[0].remaining_ticks,
        (duration + duration / 2).min(200)
    );
    confusion.entities[0].statuses[0].remaining_ticks = 199;
    confusion.rng = RfbRng::seeded(seed);
    project(&mut confusion, Confusion, 1000);
    assert_eq!(confusion.entities[0].statuses[0].remaining_ticks, 200);
    let seed = (0..100)
        .find(|seed| {
            let mut game = template.clone();
            game.rng = RfbRng::seeded(*seed);
            project(&mut game, Fear, 90);
            !game.entities[0].statuses.is_empty()
        })
        .unwrap();
    let mut fear = template.clone();
    fear.rng = RfbRng::seeded(seed);
    project(&mut fear, Fear, 90);
    let ticks = fear.entities[0].statuses[0].remaining_ticks;
    fear.rng = RfbRng::seeded(seed);
    project(&mut fear, Fear, 90);
    assert_eq!(
        fear.entities[0].statuses[0].remaining_ticks,
        (ticks * 2).min(200)
    );
    // Effect power changes fear duration, not the CHR/level save or its result.
    let mut weak = template;
    weak.rng = RfbRng::seeded(seed);
    project(&mut weak, Fear, 1);
    assert_eq!(weak.entities[0].statuses[0].remaining_ticks, 4);
    assert_eq!(weak.rng, fear.rng);
}

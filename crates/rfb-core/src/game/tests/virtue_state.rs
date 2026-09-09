// SPDX-License-Identifier: MPL-2.0

use super::*;

fn virtue_kinds(game: &Game) -> Vec<VirtueKindDto> {
    game.snapshot()
        .player
        .virtues
        .into_iter()
        .map(|virtue| virtue.kind)
        .collect()
}

fn seed_for_coin_rolls(expected: &[u64]) -> u64 {
    (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            expected.iter().all(|roll| rng.bounded(2) == *roll)
        })
        .expect("requested bounded coin sequence should be reachable")
}

fn set_virtue(game: &mut Game, slot: usize, kind: VirtueKindDto, value: i16) {
    game.virtues[slot] = VirtueDto { kind, value };
}

#[test]
fn hidden_draconian_subraces_keep_the_authoritative_enchantment_virtue() {
    let content = load_built_in_content().expect("built-in content should load");
    for suffix in [
        "red", "white", "blue", "black", "green", "bronze", "crystal", "gold", "shadow",
    ] {
        let identity = CharacterBuildIdentity {
            build_id: "demo.build.archer".to_owned(),
            race_id: format!("rfb-legacy.race.draconian-{suffix}"),
            class_id: "demo.class.archer".to_owned(),
            personality_id: "demo.personality.ordinary".to_owned(),
        };
        let mut rng = RfbRng::seeded(118);
        let virtues = virtues::initial_virtues(&content, Some(&identity), &mut rng);
        assert_eq!(
            std::array::from_fn(|index| virtues[index].kind),
            [
                VirtueKindDto::Nature,
                VirtueKindDto::Temperance,
                VirtueKindDto::Enchantment,
            ],
            "{suffix}"
        );
    }
}

#[test]
fn rfb_virtue_initialization_keeps_class_race_and_realm_order_then_fills_unique_slots() {
    for (build_id, expected_prefix) in [
        (
            "demo.build.warrior",
            vec![
                VirtueKindDto::Valour,
                VirtueKindDto::Honour,
                VirtueKindDto::Individualism,
            ],
        ),
        (
            "demo.build.archer",
            vec![
                VirtueKindDto::Nature,
                VirtueKindDto::Temperance,
                VirtueKindDto::Individualism,
            ],
        ),
        (
            "demo.build.high-mage-death",
            vec![
                VirtueKindDto::Enlightenment,
                VirtueKindDto::Enchantment,
                VirtueKindDto::Knowledge,
                VirtueKindDto::Individualism,
                VirtueKindDto::Unlife,
            ],
        ),
    ] {
        let left = Game::new_with_build(42, build_id).expect("build should create");
        let right =
            Game::new_with_build(42, build_id).expect("build should create deterministically");
        let kinds = virtue_kinds(&left);
        assert_eq!(&kinds[..expected_prefix.len()], expected_prefix);
        assert_eq!(kinds.len(), super::super::virtues::VIRTUE_SLOT_COUNT);
        assert!(
            kinds
                .iter()
                .enumerate()
                .all(|(index, kind)| !kinds[..index].contains(kind))
        );
        assert!(left.virtues.iter().all(|virtue| virtue.value == 0));
        assert_eq!(left.virtues, right.virtues);
    }
}

#[test]
fn formal_races_receive_their_original_race_virtues() {
    use VirtueKindDto::*;

    for (seed, build_id, race_id, expected) in [
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.barbarian",
            [Nature, Temperance, Valour],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.half-orc",
            [Nature, Temperance, Valour],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.high-elf",
            [Nature, Temperance, Vitality],
        ),
        (
            43,
            "demo.build.warrior",
            "rfb-legacy.race.hobbit",
            [Valour, Honour, Temperance],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.kobold",
            [Nature, Temperance, Honour],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.dwarf",
            [Nature, Temperance, Diligence],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.nibelung",
            [Nature, Temperance, Patience],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.gnome",
            [Nature, Temperance, Knowledge],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.half-giant",
            [Nature, Temperance, Justice],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.half-troll",
            [Nature, Temperance, Valour],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.half-titan",
            [Nature, Temperance, Harmony],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.cyclops",
            [Nature, Temperance, Knowledge],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.yeek",
            [Nature, Temperance, Sacrifice],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.klackon",
            [Nature, Temperance, Diligence],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.dark-elf",
            [Nature, Temperance, Enchantment],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.mindflayer",
            [Nature, Temperance, Enlightenment],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.imp",
            [Nature, Temperance, Faith],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.dunadan",
            [Nature, Temperance, Individualism],
        ),
        (
            43,
            "demo.build.archer",
            "rfb-legacy.race.snotling",
            [Nature, Temperance, Honour],
        ),
        (
            405,
            "demo.build.warrior",
            "rfb-legacy.race.boit",
            [Valour, Honour, Sacrifice],
        ),
        (
            409,
            "demo.build.warrior",
            "rfb-legacy.race.einheri",
            [Valour, Honour, Unlife],
        ),
        (
            411,
            "demo.build.warrior",
            "rfb-legacy.race.kutar",
            [Valour, Honour, Vitality],
        ),
        (
            413,
            "demo.build.archer",
            "rfb-legacy.race.amberite",
            [Nature, Temperance, Honour],
        ),
        (
            419,
            "demo.build.archer",
            "rfb-legacy.race.beastman",
            [Nature, Temperance, Chance],
        ),
        (
            421,
            "demo.build.warrior",
            "rfb-legacy.race.shadow-fairy",
            [Valour, Honour, Enchantment],
        ),
        (
            367,
            "demo.build.warrior",
            "rfb-legacy.race.golem",
            [Valour, Honour, Justice],
        ),
        (
            376,
            "demo.build.warrior",
            "rfb-legacy.race.zombie",
            [Valour, Honour, Unlife],
        ),
        (
            383,
            "demo.build.warrior",
            "rfb-legacy.race.skeleton",
            [Valour, Honour, Unlife],
        ),
        (
            384,
            "demo.build.warrior",
            "rfb-legacy.race.wood-elf",
            [Valour, Honour, Nature],
        ),
        (
            387,
            "demo.build.warrior",
            "rfb-legacy.race.archon",
            [Valour, Honour, Justice],
        ),
        (
            389,
            "demo.build.warrior",
            "rfb-legacy.race.sprite",
            [Valour, Honour, Nature],
        ),
    ] {
        let game =
            Game::new_with_build_race_and_name(seed, build_id, race_id, Game::DEFAULT_PLAYER_NAME)
                .unwrap_or_else(|error| panic!("{race_id}: {error}"));
        assert_eq!(&virtue_kinds(&game)[..3], expected, "{race_id}");
    }
}

#[test]
fn virtue_changes_use_the_three_rfb_soft_caps_and_hard_bounds() {
    let mut game = Game::new(0);
    for (current, amount, coin_rolls, expected) in [
        (49, 2, &[0][..], 50),
        (79, 2, &[1, 0][..], 80),
        (99, 2, &[1, 1, 0][..], 100),
        (124, 10, &[1, 1, 1][..], 125),
        (-49, -2, &[0][..], -50),
        (-79, -2, &[1, 0][..], -80),
        (-99, -2, &[1, 1, 0][..], -100),
        (-124, -10, &[1, 1, 1][..], -125),
    ] {
        set_virtue(&mut game, 0, VirtueKindDto::Valour, current);
        game.rng = RfbRng::seeded(seed_for_coin_rolls(coin_rolls));
        game.add_virtue(VirtueKindDto::Valour, amount);
        assert_eq!(game.virtue_current(VirtueKindDto::Valour), expected);
    }
}

#[test]
fn chance_virtue_adjusts_rolls_with_the_authoritative_repeated_d400_rule() {
    for (chance, direction) in [(125, 1), (-125, -1)] {
        let seed = (0..10_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(400) + 1 < 125 && rng.bounded(400) + 1 >= 125
            })
            .expect("one Chance adjustment should be reachable");
        let mut game = Game::new(0);
        set_virtue(&mut game, 0, VirtueKindDto::Chance, chance);
        game.rng = RfbRng::seeded(seed);
        assert_eq!(game.adjust_roll_by_chance_virtue(50), 50 + direction);
    }
}

#[test]
fn virtue_state_round_trips_and_rejects_invalid_slots() {
    let mut game = Game::new_with_build(17, "demo.build.high-mage-death")
        .expect("Death High-Mage should create");
    let initial_hash = game.state_hash();
    game.add_virtue(VirtueKindDto::Enchantment, 2);
    game.add_virtue(VirtueKindDto::Unlife, 1);
    assert_ne!(game.state_hash(), initial_hash);
    let save = game.to_save();
    let restored = Game::from_save(save.clone()).expect("valid virtues should round-trip");
    assert_eq!(restored.virtues, game.virtues);
    assert_eq!(restored.snapshot().player.virtues, save.player.virtues);

    let mut duplicate = save.clone();
    duplicate.player.virtues[1].kind = duplicate.player.virtues[0].kind;
    assert!(Game::from_save(duplicate).is_err());

    let mut out_of_range = save;
    out_of_range.player.virtues[0].value = 126;
    assert!(Game::from_save(out_of_range).is_err());
}

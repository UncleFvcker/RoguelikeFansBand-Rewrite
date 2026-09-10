// SPDX-License-Identifier: MPL-2.0

use rfb_content::{ItemDefinition, LootEntryDefinition, RfbDropTheme};

use super::{Game, ItemGenerationMode, LootContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Weapon,
    Shield,
    BowQuiver,
    Ring,
    Amulet,
    Light,
    BodyArmor,
    Cloak,
    Helm,
    Gloves,
    Boots,
    Device,
    Consumable,
    Ammo,
    Book,
    Bag,
    Misc,
}

// object2.c::_kind_alloc_table: preserve source order and signed adjustments.
const CATEGORIES: [(Category, i32, i32, i32, Option<&str>); 17] = [
    (Category::Weapon, 195, 0, 0, Some("weapon")),
    (Category::Shield, 30, 0, 0, Some("shield")),
    (Category::BowQuiver, 60, 0, 0, Some("launcher")),
    (Category::Ring, 20, 0, 0, Some("ring")),
    (Category::Amulet, 20, 0, 0, Some("amulet")),
    (Category::Light, 10, 0, 0, Some("light")),
    (Category::BodyArmor, 195, 0, 0, Some("body")),
    (Category::Cloak, 30, 0, 0, Some("cloak")),
    (Category::Helm, 30, 0, 0, Some("head")),
    (Category::Gloves, 30, 0, 0, Some("gloves")),
    (Category::Boots, 30, 0, 0, Some("boots")),
    (Category::Device, 95, -40, -60, None),
    (Category::Consumable, 100, -50, -90, None),
    (Category::Ammo, 80, 0, 0, Some("launcher")),
    (Category::Book, 25, 10, 15, None),
    (Category::Bag, 30, 0, 0, Some("container")),
    (Category::Misc, 50, -50, -50, None),
];

impl Category {
    fn accepts(self, tval: u16, sval: u16) -> bool {
        match self {
            Self::Weapon => matches!(tval, 21..=23),
            Self::Shield => tval == 34,
            Self::BowQuiver => tval == 19 || (tval, sval) == (46, 0),
            Self::Ring => tval == 45,
            Self::Amulet => tval == 40,
            Self::Light => tval == 39,
            Self::BodyArmor => matches!(tval, 36..=38),
            Self::Cloak => tval == 35,
            Self::Helm => matches!(tval, 32 | 33),
            Self::Gloves => tval == 31,
            Self::Boots => tval == 30,
            Self::Device => matches!(tval, 55 | 65 | 66),
            Self::Consumable => matches!(tval, 70 | 75),
            Self::Ammo => matches!(tval, 16..=18),
            Self::Book => matches!(tval, 90..=109),
            Self::Bag => (tval, sval) == (46, 1),
            Self::Misc => matches!(tval, 1..=5 | 7..=9 | 11 | 77 | 80),
        }
    }
}

fn is_great(mode: ItemGenerationMode) -> bool {
    matches!(
        mode,
        ItemGenerationMode::Great
            | ItemGenerationMode::GreatOnly
            | ItemGenerationMode::TailoredGreat
            | ItemGenerationMode::Artifact { .. }
    )
}

fn is_good(mode: ItemGenerationMode) -> bool {
    matches!(
        mode,
        ItemGenerationMode::Good
            | ItemGenerationMode::Great
            | ItemGenerationMode::TailoredGreat
            | ItemGenerationMode::Artifact { .. }
    )
}

fn category_weights(game: &Game, mode: ItemGenerationMode) -> Vec<u32> {
    CATEGORIES
        .iter()
        .map(|(_, base, good, great, slot)| {
            let mut weight = base
                + if is_great(mode) {
                    *great
                } else if is_good(mode) {
                    *good
                } else {
                    0
                };
            if slot.is_some_and(|slot| {
                !game
                    .body_slots
                    .iter()
                    .any(|body| crate::game::item_can_occupy_slot_type(slot, &body.slot_type))
            }) {
                weight /= 2;
            }
            weight.max(0) as u32
        })
        .collect()
}

fn allocation_level(game: &mut Game, context: &LootContext, mode: ItemGenerationMode) -> u16 {
    // get_obj_num clamps before GREAT_OBJ, never after it. The current source
    // d_info has no BEGINNER dungeon, and no runtime option sets that flag.
    let mut level = context
        .depth
        .saturating_add(if is_good(mode) { 10 } else { 0 })
        .min(127);
    if level > 0 && game.rng.bounded(8) == 0 {
        let boost = level.max(20);
        level += boost / 4 + game.rng.bounded(u64::from(boost / 2 - boost / 4 + 1)) as u16;
    }
    level
}

fn quality_candidate(game: &Game, mode: ItemGenerationMode, item: &ItemDefinition) -> bool {
    if !is_good(mode) && !is_great(mode) {
        return true;
    }
    let base = item
        .rfb_base_kind
        .expect("source pool validates kind identities");
    match base.tval {
        30..=38 => {
            item.rfb_value
                .as_ref()
                .expect("source armor retains base +AC")
                .to_armor
                >= 0
        }
        19..=23 => {
            // P-line hit/damage are already preserved in these profiles; the
            // source harp has no P-line and neither combat profile.
            item.melee_profile
                .as_ref()
                .is_none_or(|profile| profile.to_hit >= 0 && profile.to_damage >= 0)
                && item
                    .projectile_profile
                    .as_ref()
                    .is_none_or(|profile| profile.to_hit >= 0 && profile.to_damage >= 0)
        }
        16 => !is_great(mode),
        17 | 18 | 40 | 45 | 55 | 65 | 66 => true,
        // Rage Mage's limit of eight has no current class/build entry.
        90..=95 | 97..=101 | 104..=109 => {
            base.sval >= 2
                && game
                    .item_knowledge
                    .get(&item.id)
                    .map_or(0, |state| state.found_count)
                    < 2
        }
        75 if is_great(mode) => matches!(base.sval, 38 | 39 | 55),
        75 => matches!(base.sval, 37..=39 | 48..=53 | 55 | 60),
        70 if is_great(mode) => matches!(base.sval, 41 | 44..=47 | 52 | 61),
        70 => matches!(base.sval, 41 | 44..=50 | 58 | 59 | 61 | 63),
        _ => false,
    }
}

fn book_weight(game: &Game, item: &ItemDefinition, weight: u32) -> u32 {
    let base = item
        .rfb_base_kind
        .expect("source pool validates kind identities");
    if weight == 0 || item.ability_book_id.is_none() {
        return weight;
    }
    let limit = if base.tval == 96 {
        10
    } else {
        [10, 10, 3, 2][usize::from(base.sval)]
    };
    let found = game
        .item_knowledge
        .get(&item.id)
        .map_or(0, |state| state.found_count);
    // Once a positive u32 weight has been shifted 31 times, max(1) is stable.
    (weight >> found.saturating_sub(limit).min(31)).max(1)
}

/// One _choose_obj_kind/get_obj_num attempt. Empty categories fail normally;
/// retry policy belongs to the calling generation/reward operation.
pub(super) fn select_entry(
    game: &mut Game,
    context: &LootContext,
    mode: ItemGenerationMode,
    entries: &[LootEntryDefinition],
    theme: Option<RfbDropTheme>,
) -> Option<usize> {
    let category = theme.is_none().then(|| {
        let weights = category_weights(game, mode);
        CATEGORIES[game.roll_weighted_index(&weights)].0
    });
    let tomte_headgear = mode == ItemGenerationMode::TailoredGreat
        && game
            .build
            .as_ref()
            .is_some_and(|build| build.race_id == "rfb-legacy.race.tomte");
    // get_obj_num_prep applies the hook before get_obj_num rolls its boost.
    let mut weights = entries
        .iter()
        .map(|entry| {
            let item = game
                .content
                .item(&entry.item_kind_id)
                .expect("validated source item");
            let base = item
                .rfb_base_kind
                .expect("source pool validates kind identities");
            if !category.is_none_or(|category| category.accepts(base.tval, base.sval))
                || !theme.is_none_or(|theme| theme_candidate(theme, item))
                || !quality_candidate(game, mode, item)
                || (tomte_headgear
                    && matches!(base.tval, 32 | 33)
                    && (base.tval, base.sval) != (32, 1))
            {
                0
            } else {
                entry.weight
            }
        })
        .collect::<Vec<_>>();
    let level = allocation_level(game, context, mode);
    let dungeon = game
        .content
        .world(&game.world_id)
        .and_then(|world| {
            let floor = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == context.floor_id)?;
            world
                .dungeons
                .iter()
                .find(|dungeon| Some(&dungeon.id) == floor.dungeon_id.as_ref())
        })
        .and_then(|dungeon| dungeon.legacy_index);
    for (entry, weight) in entries.iter().zip(&mut weights) {
        let item = game.content.item(&entry.item_kind_id).unwrap();
        let base = item.rfb_base_kind.unwrap();
        *weight = if entry.min_depth > level
            || entry.max_depth < level
            || ((base.tval, base.sval) == (80, 40) && dungeon != Some(22))
            || ((base.tval, base.sval) == (75, 14) && dungeon != Some(39))
        {
            0
        } else {
            book_weight(game, item, *weight)
        };
    }
    weights
        .iter()
        .any(|weight| *weight > 0)
        .then(|| game.roll_weighted_index(&weights))
}

/// Static candidate domain of object2.c::_kind_theme_* at RFB master a0d92b6378.
/// Warrior's random jewelry acceptance belongs to the allocation scheduler (B3).
pub(super) fn theme_candidate(theme: RfbDropTheme, item: &ItemDefinition) -> bool {
    let base = item
        .rfb_base_kind
        .expect("source pool validates kind identities");
    let (tval, sval) = (base.tval, base.sval);
    let warrior = || {
        matches!(tval, 22 | 30..=32 | 34 | 37 | 38 | 40 | 45)
            || (tval == 23 && (11..32).contains(&sval))
            || (tval == 75 && matches!(sval, 32 | 33))
    };
    match theme {
        RfbDropTheme::Warrior => warrior(),
        RfbDropTheme::WarriorShoot => {
            warrior() || tval == 18 || (tval == 19 && matches!(sval, 23 | 24))
        }
        RfbDropTheme::Archer => matches!(tval, 17 | 19 | 45 | 46),
        RfbDropTheme::Mage => {
            matches!(tval, 40 | 45 | 55 | 65 | 66 | 91..=98 | 101)
                || matches!((tval, sval), (21, 21) | (36, 2) | (32, 10))
                || (tval == 75 && matches!(sval, 40 | 48..=53 | 56..=58 | 62 | 70 | 71))
                || (tval == 70 && matches!(sval, 8..=13 | 22 | 41 | 44..=47 | 52 | 57..=62))
        }
        RfbDropTheme::Priest => {
            matches!(tval, 21 | 40 | 90 | 99)
                || (tval == 75 && matches!(sval, 34..=39 | 41..=47))
                || (tval == 70 && matches!(sval, 14 | 15 | 33..=35 | 37 | 38 | 42))
        }
        RfbDropTheme::PriestEvil => {
            matches!(tval, 21 | 40 | 94 | 98) || (tval == 70 && matches!(sval, 44 | 45))
        }
        RfbDropTheme::Paladin => {
            warrior()
                || matches!(tval, 90 | 99)
                || (tval == 75 && matches!(sval, 34..=39))
                || (tval == 70 && matches!(sval, 33..=35 | 37 | 50))
        }
        RfbDropTheme::PaladinEvil => {
            warrior()
                || matches!(tval, 94 | 98)
                || (tval == 70 && matches!(sval, 41 | 44 | 45 | 50))
        }
        RfbDropTheme::Samurai => {
            matches!(tval, 45 | 106) || matches!((tval, sval), (23, 13 | 20) | (37, 14))
        }
        RfbDropTheme::Ninja => {
            matches!(tval, 5 | 40) || matches!((tval, sval), (23, 4 | 19 | 32 | 33) | (36, 13))
        }
        RfbDropTheme::Rogue => {
            matches!(tval, 16 | 31 | 35 | 36 | 40 | 109)
                || (tval, sval) == (19, 2)
                || (tval == 23 && item.weight_tenths_pound < 50)
        }
        RfbDropTheme::Hobbit => tval == 80 || matches!((tval, sval), (19, 2) | (16, 1)),
        RfbDropTheme::Dwarf => {
            tval == 40
                || matches!(
                    (tval, sval),
                    (22, 10 | 11 | 22 | 25 | 28)
                        | (30, 5)
                        | (20, 3 | 6 | 7)
                        | (32, 5 | 6)
                        | (34, 3)
                        | (37, 20 | 25)
                )
        }
        RfbDropTheme::Junk => matches!(tval, 1 | 2 | 3 | 5 | 10 | 39 | 77 | 80),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{
        Game,
        loot::{ItemGenerationMode, LootContext, LootSource},
    };

    fn context(depth: u16) -> LootContext {
        LootContext {
            table_id: "demo.loot-table.base-items".into(),
            floor_id: "demo.floor.warrens-depth-1".into(),
            depth,
            source: LootSource::ItemUse {
                item_id: "test.allocation".into(),
            },
        }
    }

    #[test]
    fn category_weights_use_great_precedence_and_body_slots_not_equipped_items() {
        let mut game = Game::new_with_build(411, "demo.build.warrior").unwrap();
        let ordinary = category_weights(&game, ItemGenerationMode::Ordinary);
        let good = category_weights(&game, ItemGenerationMode::Good);
        let great = category_weights(&game, ItemGenerationMode::Great);
        assert_eq!(ordinary.iter().sum::<u32>(), 1030);
        assert_eq!(good.iter().sum::<u32>(), 900);
        assert_eq!(great.iter().sum::<u32>(), 845);
        assert_eq!(
            category_weights(&game, ItemGenerationMode::GreatOnly),
            great
        );
        game.items.clear();
        assert_eq!(
            category_weights(&game, ItemGenerationMode::Ordinary),
            ordinary
        );
        game.body_slots = crate::game::dragon_body_slots();
        let dragon = category_weights(&game, ItemGenerationMode::Ordinary);
        assert_eq!(
            dragon[0], 97,
            "missing weapon slot halves 195 with integer truncation"
        );
        assert_eq!(
            dragon[3], 20,
            "multiple ring slots do not multiply frequency"
        );
        assert_eq!(dragon[13], 40, "ammunition depends on the bow slot");
        game.body_slots = vec![crate::game::BodySlot {
            id: "left-hand".into(),
            slot_type: "shield".into(),
        }];
        assert_eq!(
            category_weights(&game, ItemGenerationMode::Ordinary)[0],
            195
        );
        assert!(
            !Category::Weapon.accepts(20, 1),
            "diggers are not in the weapon category"
        );
        assert!(
            Category::BowQuiver.accepts(19, 70),
            "ordinary bows include harps"
        );
        assert!(Category::BowQuiver.accepts(46, 0));
        assert!(!Category::BowQuiver.accepts(46, 1));
        assert!(Category::Bag.accepts(46, 1));
        assert!(
            !Category::Misc.accepts(10, 0),
            "corpse is distinct from generic skeleton"
        );
    }

    #[test]
    fn good_and_great_distinguish_damaged_bases_ammo_consumables_and_books() {
        let mut game = Game::new_with_build(412, "demo.build.warrior").unwrap();
        let accepts = |mode, id| quality_candidate(&game, mode, game.content.item(id).unwrap());
        assert!(!accepts(
            ItemGenerationMode::Good,
            "demo.item.broken-dagger"
        ));
        assert!(accepts(ItemGenerationMode::Great, "demo.item.harp"));
        assert!(accepts(ItemGenerationMode::Good, "demo.item.iron-shot"));
        assert!(!accepts(
            ItemGenerationMode::GreatOnly,
            "demo.item.iron-shot"
        ));
        assert!(accepts(ItemGenerationMode::Great, "demo.item.arrow"));
        assert!(accepts(
            ItemGenerationMode::Good,
            "demo.item.healing-potion"
        ));
        assert!(!accepts(
            ItemGenerationMode::Great,
            "demo.item.healing-potion"
        ));
        assert!(accepts(
            ItemGenerationMode::Great,
            "demo.item.star-healing-potion"
        ));
        assert!(accepts(
            ItemGenerationMode::Great,
            "demo.item.magic-missile-wand"
        ));
        assert!(!accepts(
            ItemGenerationMode::Good,
            "demo.item.manual-of-mastery"
        ));
        assert!(!accepts(ItemGenerationMode::Good, "demo.item.black-mass"));
        let mut armor = game.content.item("demo.item.robe").unwrap().clone();
        armor.rfb_value.as_mut().unwrap().to_armor = -1;
        assert!(!quality_candidate(&game, ItemGenerationMode::Good, &armor));
        for kind in ["demo.item.black-channels", "demo.item.necronomicon"] {
            game.item_knowledge
                .entry(kind.into())
                .or_default()
                .found_count = 1;
            assert!(quality_candidate(
                &game,
                ItemGenerationMode::Great,
                game.content.item(kind).unwrap()
            ));
            game.item_knowledge.get_mut(kind).unwrap().found_count = 2;
            assert!(!quality_candidate(
                &game,
                ItemGenerationMode::Great,
                game.content.item(kind).unwrap()
            ));
        }
    }

    #[test]
    fn ordinary_book_weights_decay_past_each_limit_without_promoting_zero_rows() {
        let mut game = Game::new_with_build(413, "demo.build.warrior").unwrap();
        for (id, limit) in [
            ("demo.item.black-prayers", 10),
            ("demo.item.black-mass", 10),
            ("demo.item.black-channels", 3),
            ("demo.item.necronomicon", 2),
            ("demo.item.manual-of-mastery", 10),
        ] {
            let item = game.content.item(id).unwrap().clone();
            game.item_knowledge
                .entry(id.into())
                .or_default()
                .found_count = limit;
            assert_eq!(book_weight(&game, &item, 100), 100);
            game.item_knowledge.get_mut(id).unwrap().found_count += 1;
            assert_eq!(book_weight(&game, &item, 100), 50);
            game.item_knowledge.get_mut(id).unwrap().found_count += 3;
            assert_eq!(book_weight(&game, &item, 100), 6);
            game.item_knowledge.get_mut(id).unwrap().found_count =
                crate::game::inventory::MAX_BOOK_FOUND_COUNT;
            assert_eq!(book_weight(&game, &item, u32::MAX), 1);
            assert_eq!(book_weight(&game, &item, 0), 0);
        }
    }

    #[test]
    fn allocation_depth_uses_only_good_bonus_then_clamps_then_boosts() {
        let mut game = Game::new(414);
        let no_boost = (0..100)
            .find(|seed| crate::rng::RfbRng::seeded(*seed).bounded(8) != 0)
            .unwrap();
        for (mode, expected) in [
            (ItemGenerationMode::Ordinary, 0),
            (ItemGenerationMode::Good, 10),
            (ItemGenerationMode::GreatOnly, 0),
            (ItemGenerationMode::Great, 10),
        ] {
            game.rng = crate::rng::RfbRng::seeded(no_boost);
            assert_eq!(allocation_level(&mut game, &context(0), mode), expected);
            assert_eq!(game.rng.draw_counter, u64::from(expected > 0));
        }
        let boost = (0..100)
            .find(|seed| crate::rng::RfbRng::seeded(*seed).bounded(8) == 0)
            .unwrap();
        let mut expected_rng = crate::rng::RfbRng::seeded(boost);
        assert_eq!(expected_rng.bounded(8), 0);
        let expected = 127 + 31 + expected_rng.bounded(33) as u16;
        game.rng = crate::rng::RfbRng::seeded(boost);
        assert_eq!(
            allocation_level(&mut game, &context(u16::MAX), ItemGenerationMode::Great),
            expected
        );
        assert!(expected > 127);
        assert_eq!(game.rng, expected_rng);
        game.rng = crate::rng::RfbRng::seeded(boost);
        expected_rng = game.rng.clone();
        expected_rng.bounded(8);
        assert_eq!(
            allocation_level(&mut game, &context(1), ItemGenerationMode::Ordinary),
            6 + expected_rng.bounded(6) as u16
        );
        assert_eq!(game.rng, expected_rng);
    }

    #[test]
    fn empty_category_and_boosted_max_depth_fail_without_weighted_draw_or_fallback() {
        let mut game = Game::new(415);
        let rows = vec![LootEntryDefinition {
            item_kind_id: "demo.item.dagger".into(),
            weight: 100,
            quantity: 1,
            min_depth: 0,
            max_depth: 127,
        }];
        let seed = (0..100)
            .find(|seed| {
                let mut trial = crate::rng::RfbRng::seeded(*seed);
                trial.bounded(1030) >= 195
            })
            .unwrap();
        game.rng = crate::rng::RfbRng::seeded(seed);
        assert_eq!(
            select_entry(
                &mut game,
                &context(0),
                ItemGenerationMode::Ordinary,
                &rows,
                None
            ),
            None
        );
        assert_eq!(
            game.rng.draw_counter, 1,
            "category draw remains even when no kind can match"
        );
        let boost = (0..100)
            .find(|seed| crate::rng::RfbRng::seeded(*seed).bounded(8) == 0)
            .unwrap();
        game.rng = crate::rng::RfbRng::seeded(boost);
        assert_eq!(
            select_entry(
                &mut game,
                &context(127),
                ItemGenerationMode::Good,
                &rows,
                Some(RfbDropTheme::Warrior)
            ),
            None
        );
        assert_eq!(
            game.rng.draw_counter, 2,
            "theme skips category; boost precedes max-level exclusion"
        );
    }

    #[test]
    fn duplicate_allocation_rows_keep_their_order_and_weight_after_filtering() {
        let mut game = Game::new(416);
        let rows = [0, 100, 25].map(|weight| LootEntryDefinition {
            item_kind_id: "demo.item.black-prayers".into(),
            weight,
            quantity: 1,
            min_depth: 0,
            max_depth: u16::MAX,
        });
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..32 {
            game.rng = crate::rng::RfbRng::seeded(seed);
            let mut expected = game.rng.clone();
            let index = if expected.bounded(125) < 100 { 1 } else { 2 };
            assert_eq!(
                select_entry(
                    &mut game,
                    &context(0),
                    ItemGenerationMode::Ordinary,
                    &rows,
                    Some(RfbDropTheme::Mage)
                ),
                Some(index)
            );
            assert_eq!(
                game.rng, expected,
                "theme at level zero consumes only the weighted kind draw"
            );
            seen.insert(index);
        }
        assert_eq!(seen, [1, 2].into());
    }

    #[test]
    fn source_good_books_devices_and_consumables_materialize_pick_up_and_restore() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
        let mut content = rfb_content::compile_pack_dir(&path).unwrap().content;
        let expected = [
            "demo.item.black-channels",
            "demo.item.magic-missile-wand",
            "demo.item.might-tonic",
            "demo.item.destruction-scroll",
        ];
        let mut pool = content
            .loot_tables
            .iter()
            .find(|table| table.id == "demo.loot-table.base-items")
            .unwrap()
            .clone();
        pool.id = "test.loot-table.non-equipment-base".into();
        pool.entries
            .retain(|row| expected.contains(&row.item_kind_id.as_str()));
        let mut theme = content
            .loot_tables
            .iter()
            .find(|table| table.id == "demo.loot-table.mage")
            .unwrap()
            .clone();
        theme.id = "test.loot-table.non-equipment-mage".into();
        theme.kind_selection = Some(rfb_content::LootKindSelectionDefinition::RfbTheme {
            pool_id: pool.id.clone(),
            theme: RfbDropTheme::Mage,
        });
        content.loot_tables.extend([pool, theme]);
        let mut game = Game::new_with_build(417, "demo.build.warrior").unwrap();
        game.content = std::sync::Arc::new(rfb_content::ContentCatalog::from_artifact(
            rfb_content::encode_content(content).unwrap(),
        ));
        let mut context = context(70);
        context.table_id = "test.loot-table.non-equipment-mage".into();
        let mut found = std::collections::BTreeSet::new();
        for _ in 0..128 {
            let Some(draft) = game.generate_one_loot_draft(&context, ItemGenerationMode::Good)
            else {
                continue;
            };
            if !expected.contains(&draft.kind_id.as_str()) || !found.insert(draft.kind_id.clone()) {
                continue;
            }
            assert_eq!(draft.quantity, 1);
            if draft.kind_id == "demo.item.magic-missile-wand" {
                assert!(draft.charges.unwrap().maximum > 0);
            }
            let item = game
                .commit_generated_item_draft(
                    draft,
                    crate::state::ItemLocation::Ground(game.player.position),
                )
                .unwrap();
            let id = item.id.clone();
            game.items.push(item);
            game.pick_up_item_at_player(Some(&id)).unwrap();
            if found.len() == expected.len() {
                break;
            }
        }
        assert_eq!(found, expected.into_iter().map(str::to_owned).collect());
        assert_eq!(
            game.item_knowledge["demo.item.black-channels"].found_count,
            1
        );
        game.reveal_current_visibility();
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(
            game.generate_one_loot_draft(&context, ItemGenerationMode::Great),
            restored.generate_one_loot_draft(&context, ItemGenerationMode::Great)
        );
        assert_eq!(restored.rng, game.rng);
    }

    #[test]
    fn themes_read_source_identity_including_previously_missing_domains() {
        let content = crate::game::load_built_in_content().unwrap();
        let accepts = |theme, id| theme_candidate(theme, content.item(id).unwrap());
        assert!(accepts(RfbDropTheme::Archer, "demo.item.harp"));
        assert!(accepts(RfbDropTheme::Mage, "demo.item.magic-missile-wand"));
        assert!(accepts(RfbDropTheme::Mage, "demo.item.beginners-handbook"));
        assert!(!accepts(
            RfbDropTheme::Mage,
            "demo.item.book-of-common-prayer"
        ));
        assert!(accepts(RfbDropTheme::Dwarf, "demo.item.mattock"));
        assert!(accepts(RfbDropTheme::Hobbit, "demo.item.ration-of-food"));
        assert!(!accepts(RfbDropTheme::Hobbit, "demo.item.iron-shot"));
        let mut sword = content.item("demo.item.dagger").unwrap().clone();
        sword.id = "test.item.unrelated-name".into();
        sword.weight_tenths_pound = 49;
        assert!(theme_candidate(RfbDropTheme::Rogue, &sword));
        sword.weight_tenths_pound = 50;
        assert!(!theme_candidate(RfbDropTheme::Rogue, &sword));
    }

    #[test]
    fn renamed_theme_uses_shared_pool_and_empty_or_zero_weight_does_not_fall_back() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
        let mut content = rfb_content::compile_pack_dir(&path).unwrap().content;
        let base = content
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.base-items")
            .unwrap();
        base.entries
            .retain(|row| row.item_kind_id == "demo.item.ration-of-food");
        base.entries.truncate(1);
        base.entries[0].weight = 1;
        base.entries[0].min_depth = 10;
        let mut zero = base.entries[0].clone();
        zero.min_depth = 0;
        zero.max_depth = 9;
        zero.weight = 0;
        base.entries.push(zero);
        let mut theme = content
            .loot_tables
            .iter()
            .find(|table| table.id == "demo.loot-table.hobbit")
            .unwrap()
            .clone();
        theme.id = "test.loot-table.renamed".into();
        content.loot_tables.push(theme);
        let mut game = Game::new(403);
        game.content = std::sync::Arc::new(rfb_content::ContentCatalog::from_artifact(
            rfb_content::encode_content(content).unwrap(),
        ));
        let mut context = LootContext {
            table_id: "test.loot-table.renamed".into(),
            floor_id: game.current_floor_id.clone(),
            depth: 9,
            source: LootSource::MonsterDeath {
                actor_id: "test.actor.drop".into(),
                themed: true,
            },
        };
        assert_eq!(context.drop_theme(&game.content), "hobbit");
        assert!(
            game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .is_none()
        );
        context.depth = 10;
        assert_eq!(
            game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .unwrap()
                .kind_id,
            "demo.item.ration-of-food"
        );
        context.table_id = "demo.loot-table.mage".into();
        assert!(
            game.generate_one_loot_draft(&context, ItemGenerationMode::Ordinary)
                .is_none()
        );
    }
}

// SPDX-License-Identifier: MPL-2.0

use rfb_content::{ItemDefinition, RfbDropTheme};

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

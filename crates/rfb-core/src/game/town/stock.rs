// SPDX-License-Identifier: MPL-2.0
// Zul shop.c stock hooks, using the existing source allocator and equipment materializer.

use super::*;
use crate::game::loot::{LootContext, LootSource, allocation};

pub(super) fn generates_equipment(category: ShopCategory) -> bool {
    matches!(category, ShopCategory::Jeweler | ShopCategory::Dragon)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jewelry_level_growth_and_scale_discount_preserve_source_draw_boundaries() {
        let mut rng = RfbRng::seeded(42);
        assert_eq!(jewelry_spread(1), 0);
        assert_eq!(jewelry_level(&mut rng, jewelry_spread(1)), 20);
        assert_eq!(rng.draw_counter, 0);
        assert_eq!(jewelry_spread(25), 14);
        assert_eq!(jewelry_spread(49), 47);
        assert_eq!(jewelry_spread(50), 50);
        for _ in 0..64 {
            assert!((32..=68).contains(&jewelry_level(&mut rng, 50)));
        }
        let before = rng.draw_counter;
        assert_eq!(stock_discount(&mut rng, 50_000, false, true), 0);
        assert_eq!(rng.draw_counter, before);
        assert_eq!(stock_discount(&mut rng, 50_000, true, false), 0);
        assert!(rng.draw_counter > before);
        let before = rng.draw_counter;
        assert!(accepts_jewelry_artifact(&mut rng, false));
        assert_eq!(rng.draw_counter, before);
        let mut accepted = 0;
        for _ in 0..64 {
            accepted += usize::from(accepts_jewelry_artifact(&mut rng, true));
        }
        assert!((1..64).contains(&accepted));
        assert_eq!(rng.draw_counter, before + 64);
    }

    #[test]
    fn zul_price_hooks_keep_integer_surcharge_rounding_and_owner_cap() {
        let game = Game::new(42);
        let jeweler = game.content.shop("demo.shop.zul-jeweler").unwrap();
        let dragon = game.content.shop("demo.shop.zul-dragonskin").unwrap();
        assert_eq!(player_purchase_unit_price(&game, jeweler, 1_234, 140), 3_460);
        assert_eq!(player_sale_unit_price(&game, jeweler, 1_234, 140), 440);
        assert_eq!(player_sale_unit_price(&game, jeweler, 1_000_000, 140), 20_000);
        assert_eq!(player_purchase_unit_price(&game, dragon, 31_000, 100), 31_000);
        assert_eq!(player_purchase_unit_price(&game, dragon, 31_001, 100), 31_300);
        assert_eq!(player_purchase_unit_price(&game, dragon, 31_199, 100), 31_600);
        assert_eq!(player_purchase_unit_price(&game, dragon, 31_200, 100), 31_700);
    }

    #[test]
    fn generated_candidates_use_source_hooks_and_allocate_ids_only_when_stocked() {
        let mut game = Game::new(42);
        game.progress.level = 50;
        let serial = game.next_item_instance_serial;
        let artifacts = game.generated_artifact_ids.clone();
        let mut dragon_part = false;
        for id in ["demo.shop.zul-jeweler", "demo.shop.zul-dragonskin"] {
            let shop = game.content.shop(id).unwrap().clone();
            for _ in 0..512 {
                let Some(item) = game.special_shop_candidate(&shop) else { continue };
                assert!(item.id.is_empty());
                assert!(item.curse.is_none());
                let kind = game.content.item(&item.kind_id).unwrap();
                assert!(kind.artifact_generation.is_none());
                let base = kind.rfb_base_kind.unwrap();
                assert!(stock_kind(shop.category, base.tval, base.sval, true));
                assert!(crate::game::item_value::obj_value_real(&game.content, &item).unwrap() > 0);
                if shop.category == ShopCategory::Dragon {
                    if base.tval == 38 {
                        assert_eq!(item.discount_percent, 0);
                    } else {
                        dragon_part = true;
                        assert!(!stock_kind(shop.category, base.tval, base.sval, false));
                    }
                }
            }
        }
        assert!(dragon_part);
        assert_eq!(game.next_item_instance_serial, serial);
        assert_eq!(game.generated_artifact_ids, artifacts);
    }
}

fn jewelry_spread(level: u16) -> u16 {
    // xtra1.c::py_prorata_level(50), including the level-50 rounding exception.
    let level = u32::from(level);
    if level == 50 {
        50
    } else {
        (level / 3 + level * level / 150 + level * level * level / 7_500) as u16
    }
}

fn jewelry_level(rng: &mut RfbRng, spread: u16) -> u16 {
    let bound = 3 * spread / 4;
    let random = if bound <= 1 { 0 } else { rng.bounded(u64::from(bound)) as u16 };
    20 + spread / 4 + random
}

fn stock_kind(category: ShopCategory, tval: u16, sval: u16, dragon_parts: bool) -> bool {
    match category {
        ShopCategory::Jeweler => matches!(tval, 40 | 45),
        ShopCategory::Dragon => tval == 38 || (dragon_parts && matches!(
            (tval, sval), (35, 7) | (32, 8) | (30, 4) | (31, 6) | (34, 6)
        )),
        _ => false,
    }
}

fn stock_discount(rng: &mut RfbRng, value: u32, artifact: bool, dragon_scale: bool) -> u8 {
    // _create skips _discount entirely for TV_DRAG_ARMOR. Other artifacts still consume its rolls.
    if dragon_scale || value < 5 {
        return 0;
    }
    let discount = if rng.bounded(25) == 0 {
        25
    } else if rng.bounded(150) == 0 {
        50
    } else if rng.bounded(300) == 0 {
        75
    } else if rng.bounded(500) == 0 {
        90
    } else {
        0
    };
    if artifact { 0 } else { discount }
}

fn accepts_jewelry_artifact(rng: &mut RfbRng, artifact: bool) -> bool {
    !artifact || rng.bounded(6) == 0
}

impl Game {
    fn special_shop_candidate(&mut self, shop: &ShopDefinition) -> Option<ItemInstance> {
        let table_id = shop.stock_generation_table_id.as_ref()
            .expect("generated shop stock requires a validated source pool");
        let (kind_level, magic_level, dragon_parts) = if shop.category == ShopCategory::Jeweler {
            let spread = jewelry_spread(self.progress.level);
            (
                jewelry_level(&mut self.rng, spread),
                Some(jewelry_level(&mut self.rng, spread)),
                false,
            )
        } else {
            (50, None, self.rng.bounded(4) == 0)
        };
        let entries = self.content.loot_table(table_id).unwrap().entries.iter()
            .filter(|entry| {
                let item = self.content.item(&entry.item_kind_id).unwrap();
                item.artifact_generation.is_none() && item.rfb_base_kind.is_some_and(|base| {
                    stock_kind(shop.category, base.tval, base.sval, dragon_parts)
                })
            }).cloned().collect::<Vec<_>>();
        let mut context = LootContext {
            table_id: table_id.clone(),
            floor_id: self.current_floor_id.clone(),
            depth: kind_level,
            source: LootSource::Shop { shop_id: shop.id.clone() },
        };
        let index = allocation::select_shop_entry(self, &context, &entries)?;
        // _dragon_create rolls its magic level after kind selection; the jeweler rolls both first.
        context.depth = magic_level.unwrap_or_else(|| 1 + self.rng.bounded(25) as u16);
        let draft = self.generate_shop_loot_draft(&context, &entries[index])?;
        let mut item = draft.into_item_instance(
            String::new(), ItemLocation::Shop { shop_id: shop.id.clone() },
        );
        if item.curse.is_some() {
            return None;
        }
        let value = crate::game::item_value::obj_value_real(&self.content, &item)
            .expect("source shop equipment must have authoritative value inputs");
        if value <= 0 {
            return None;
        }
        let artifact = item.is_artifact(&self.content);
        let dragon_scale = self.content.item(&item.kind_id).unwrap().rfb_base_kind.unwrap().tval == 38;
        item.discount_percent = stock_discount(&mut self.rng, value as u32, artifact, dragon_scale);
        if shop.category == ShopCategory::Jeweler && !accepts_jewelry_artifact(&mut self.rng, artifact) {
            return None;
        }
        Some(item)
    }

    pub(in crate::game::town) fn roll_special_shop_stock(
        &mut self,
        shop: &ShopDefinition,
        current_count: usize,
    ) -> Result<Vec<ItemInstance>, CoreError> {
        let target = 12 + self.rng.bounded(8) as usize;
        let mut additions = Vec::new();
        // Source _restock permits 99 generation attempts; rejected drafts allocate no item ID.
        for _ in 1..100 {
            if current_count + additions.len() >= target {
                break;
            }
            if let Some(mut item) = self.special_shop_candidate(shop) {
                item.id = self.allocate_item_instance_id()?;
                additions.push(item);
            }
        }
        Ok(additions)
    }
}

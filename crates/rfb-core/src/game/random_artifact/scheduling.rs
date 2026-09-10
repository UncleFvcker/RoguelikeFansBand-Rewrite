// Adapted from RFB master ego.c at a0d92b6378d148c5262cc236b8fa6ed2ca06a54c.
use super::*;
use crate::game::loot::{GeneratedItemDraft, ItemGenerationMode, LootContext};
use crate::game::{Game, ItemLocation};
use rfb_content::RfbBaseKindDefinition;

#[cfg(test)]
mod tests;

fn check(rng: &mut RfbRng, base: i32, level: i32, power: i16) -> bool {
    power > 2 || rng.bounded((base - base * level / 200).max(1) as u64) == 0
}

/// Called only after base treatment and enchantment; Craft uses its existing
/// direct Ego path and never enters this scheduler. Robe selection precedes it.
pub(in crate::game) fn select(
    rng: &mut RfbRng,
    base: RfbBaseKindDefinition,
    level: i32,
    power: i16,
    mode: ItemGenerationMode,
) -> Option<(i32, bool)> {
    if matches!(base.tval, 40 | 45) {
        if power == 0 {
            return None;
        }
        if matches!(mode, ItemGenerationMode::Artifact { .. }) {
            return Some((level, false));
        }
        if base.tval == 40 && rng.bounded(6) == 0 {
            return None;
        }
        let adjusted = trim(rng, level.min(100), 20, 52, 64);
        return (check(rng, 50, adjusted + 10, power) && (power <= 2 || rng.bounded(3) == 0))
            .then_some((adjusted, true));
    }
    if (-1..=1).contains(&power) {
        return None;
    }
    if base.tval == 39 {
        return (base.sval == 2 && (rng.bounded(7) == 0 || power > 2)).then_some((level, false));
    }
    let denominator = match base.tval {
        19 => 20,
        20 => 30,
        21..=23 => 40,
        30..=37 => 20,
        38 => 50,
        _ => return None,
    };
    check(rng, denominator, level, power).then_some((level, false))
}

impl Game {
    pub(in crate::game) fn materialize_random_artifact_draft(
        &mut self,
        draft: GeneratedItemDraft,
        context: &LootContext,
        value_level: i32,
        power: i16,
        mut jewelry_level_adjusted: bool,
    ) -> GeneratedItemDraft {
        let original = draft.into_item_instance(String::new(), ItemLocation::Inventory);
        let class_id = self
            .build
            .as_ref()
            .map_or("", |build| build.class_id.as_str());
        let theme = match context.table_id.as_str() {
            "demo.loot-table.warrior" => "warrior",
            "demo.loot-table.archer" => "archer",
            "demo.loot-table.mage" => "mage",
            "demo.loot-table.priest" => "priest",
            "demo.loot-table.evil-priest" => "priest-evil",
            "demo.loot-table.paladin" => "paladin",
            "demo.loot-table.evil-paladin" => "paladin-evil",
            "demo.loot-table.samurai" => "samurai",
            "demo.loot-table.ninja" => "ninja",
            "demo.loot-table.rogue" => "rogue",
            "demo.loot-table.dwarf" => "dwarf",
            "demo.loot-table.hobbit" => "hobbit",
            _ => "",
        };
        let bad_luck = self
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.bad-luck");
        let (item, _) = materialize(
            &self.content,
            &mut self.rng,
            &original,
            Creation {
                // create_artifact uses global object_level, before Bad Luck's
                // local apply_magic level reduction and jewelry's value trim.
                level: i32::from(context.depth),
                class_id,
                theme,
                ..Default::default()
            },
            &mut self.random_artifact_names,
            value_level,
            power,
            &mut jewelry_level_adjusted,
            bad_luck,
        )
        .expect("eligible RFB base and validated artifact generation data");
        item.into()
    }
}

// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{GoldAppearanceDto, GoldPileDto, Position};

use crate::{error::CoreError, rng::RfbRng, state::GoldPile, stats::CharacterBuildIdentity};

use super::Game;

pub(super) const MAX_PLAYER_GOLD: u32 = 999_999_999;
const GENERATED_GOLD_ID_PREFIX: &str = "generated.gold.";
const GOLD_BASE_VALUES: [u32; 18] = [
    3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 16, 18, 20, 24, 28, 32, 40, 80,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GoldPickupOutcome {
    pub(super) gained: u32,
    pub(super) balance: u32,
}

pub(super) fn starting_gold(build: Option<&CharacterBuildIdentity>, rng: &mut RfbRng) -> u32 {
    if build.is_none() {
        return 0;
    }
    let first = u32::try_from(rng.bounded(300) + 1).expect("birth gold roll must fit u32");
    let second = u32::try_from(rng.bounded(300) + 1).expect("birth gold roll must fit u32");
    first + second + 200
}

impl Game {
    pub(super) fn generate_gold_pile(
        &mut self,
        position: Position,
        object_level: u16,
        boosted: bool,
    ) -> Result<GoldPile, CoreError> {
        let level = u32::from(object_level);
        let variety_roll = u32::try_from(self.rng.bounded(u64::from(level + 2)) + 1)
            .expect("gold variety roll must fit u32");
        let variety_index = usize::try_from(((variety_roll + 2) / 2).saturating_sub(1))
            .expect("gold variety index must fit usize")
            .min(GOLD_BASE_VALUES.len() - 1);
        let base = GOLD_BASE_VALUES[variety_index];
        let base_roll = u32::try_from(self.rng.bounded(u64::from(base)) + 1)
            .expect("gold base roll must fit u32");
        let small_roll =
            u32::try_from(self.rng.bounded(8) + 1).expect("gold small roll must fit u32");
        let mut amount = base
            .saturating_add(8_u32.saturating_mul(base_roll))
            .saturating_add(small_roll);
        if boosted {
            amount = amount.saturating_add(amount.saturating_mul(level) / 7);
        }
        Ok(GoldPile {
            id: self.allocate_gold_pile_id()?,
            position,
            amount,
            appearance: gold_appearance(variety_index),
            discovered: false,
        })
    }

    pub(super) fn generate_mining_gold_pile(
        &mut self,
        position: Position,
        depth: u16,
        mining: u16,
    ) -> Result<GoldPile, CoreError> {
        let object_level = depth
            .saturating_add((mining / 500).min(20))
            .saturating_add(depth / 10)
            .clamp(1, 100);
        let mut pile = self.generate_gold_pile(position, object_level, false)?;
        let multiplier = 100_u32
            .saturating_add(u32::from(depth).saturating_mul(2).min(200))
            .saturating_add(u32::from(mining / 80));
        let amount = pile.amount.saturating_mul(multiplier).saturating_div(100);
        pile.amount = if amount > i16::MAX as u32 {
            (i16::MAX as u32).saturating_sub(
                u32::try_from(self.rng.bounded(1_000)).expect("gold cap roll must fit u32"),
            )
        } else {
            amount
        };
        Ok(pile)
    }

    pub(super) fn pick_up_gold_at_player(
        &mut self,
        target_id: Option<&str>,
    ) -> Option<GoldPickupOutcome> {
        let mut indices = self
            .gold_piles
            .iter()
            .enumerate()
            .filter(|(_, pile)| {
                pile.position == self.player.position
                    && target_id.is_none_or(|target_id| pile.id == target_id)
            })
            .map(|(index, pile)| (index, pile.id.clone(), pile.amount))
            .collect::<Vec<_>>();
        if indices.is_empty() {
            return None;
        }
        indices.sort_by(|left, right| left.1.cmp(&right.1));
        let before = self.gold;
        for (_, _, amount) in &indices {
            self.gold = self.gold.saturating_add(*amount).min(MAX_PLAYER_GOLD);
        }
        let remove = indices
            .into_iter()
            .map(|(index, _, _)| index)
            .collect::<std::collections::BTreeSet<_>>();
        self.gold_piles = std::mem::take(&mut self.gold_piles)
            .into_iter()
            .enumerate()
            .filter_map(|(index, pile)| (!remove.contains(&index)).then_some(pile))
            .collect();
        Some(GoldPickupOutcome {
            gained: self.gold - before,
            balance: self.gold,
        })
    }

    pub(super) fn gold_pile_dtos(&self) -> Vec<GoldPileDto> {
        let mut piles = self
            .gold_piles
            .iter()
            .filter(|pile| pile.discovered)
            .map(|pile| GoldPileDto {
                id: pile.id.clone(),
                position: pile.position,
                amount: pile.amount,
                appearance: pile.appearance,
                discovered: pile.discovered,
            })
            .collect::<Vec<_>>();
        piles.sort_by(|left, right| left.id.cmp(&right.id));
        piles
    }

    pub(super) fn mark_gold_piles_discovered(&mut self, pile_ids: &[String]) {
        for pile in &mut self.gold_piles {
            if pile_ids.contains(&pile.id) {
                pile.discovered = true;
            }
        }
    }

    pub(super) fn allocate_gold_pile_id(&mut self) -> Result<String, CoreError> {
        loop {
            let serial = self.next_gold_pile_serial;
            let next = serial
                .checked_add(1)
                .ok_or(CoreError::GoldPileIdExhausted)?;
            let candidate = format!("{GENERATED_GOLD_ID_PREFIX}{serial}");
            self.next_gold_pile_serial = next;
            let exists = self.gold_piles.iter().any(|pile| pile.id == candidate)
                || self
                    .stored_floors
                    .values()
                    .any(|floor| floor.gold_piles.iter().any(|pile| pile.id == candidate));
            if !exists {
                return Ok(candidate);
            }
        }
    }
}

pub(super) fn derive_next_gold_pile_serial<'a>(
    piles: impl IntoIterator<Item = &'a GoldPile>,
) -> Result<u64, CoreError> {
    let maximum = piles
        .into_iter()
        .filter_map(|pile| generated_gold_serial(&pile.id))
        .max()
        .unwrap_or(0);
    maximum.checked_add(1).ok_or(CoreError::GoldPileIdExhausted)
}

pub(super) fn generated_gold_serial(id: &str) -> Option<u64> {
    id.strip_prefix(GENERATED_GOLD_ID_PREFIX)?
        .parse()
        .ok()
        .filter(|serial| *serial > 0)
}

fn gold_appearance(index: usize) -> GoldAppearanceDto {
    match index {
        0..=2 => GoldAppearanceDto::Copper,
        3..=5 => GoldAppearanceDto::Silver,
        6..=7 => GoldAppearanceDto::Garnets,
        8..=10 => GoldAppearanceDto::Gold,
        11 => GoldAppearanceDto::Opals,
        12 => GoldAppearanceDto::Sapphires,
        13 => GoldAppearanceDto::Rubies,
        14 => GoldAppearanceDto::Diamonds,
        15 => GoldAppearanceDto::Emeralds,
        16 => GoldAppearanceDto::Mithril,
        _ => GoldAppearanceDto::Adamantite,
    }
}

pub(super) const fn gold_visual_id(appearance: GoldAppearanceDto) -> &'static str {
    match appearance {
        GoldAppearanceDto::Copper => "core.gold.copper",
        GoldAppearanceDto::Silver => "core.gold.silver",
        GoldAppearanceDto::Garnets => "core.gold.garnets",
        GoldAppearanceDto::Gold => "core.gold.gold",
        GoldAppearanceDto::Opals => "core.gold.opals",
        GoldAppearanceDto::Sapphires => "core.gold.sapphires",
        GoldAppearanceDto::Rubies => "core.gold.rubies",
        GoldAppearanceDto::Diamonds => "core.gold.diamonds",
        GoldAppearanceDto::Emeralds => "core.gold.emeralds",
        GoldAppearanceDto::Mithril => "core.gold.mithril",
        GoldAppearanceDto::Adamantite => "core.gold.adamantite",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_birth_gold_stays_in_the_rfb_range() {
        for seed in 0..64 {
            let build = CharacterBuildIdentity {
                build_id: "test.build.character".to_owned(),
                race_id: String::new(),
                class_id: String::new(),
                personality_id: String::new(),
            };
            let amount = starting_gold(Some(&build), &mut RfbRng::seeded(seed));
            assert!((202..=800).contains(&amount));
        }
    }

    #[test]
    fn mining_gold_uses_original_level_and_amount_bonuses() {
        let position = Position { x: 4, y: 5 };
        let depth = 32;
        let mining = 4_075;
        let object_level = 32 + 8 + 3;
        let mut ordinary = Game::new(1);
        ordinary.rng = RfbRng::seeded(99);
        let ordinary = ordinary
            .generate_gold_pile(position, object_level, false)
            .expect("ordinary gold should generate");
        let mut mined = Game::new(1);
        mined.rng = RfbRng::seeded(99);
        let mined = mined
            .generate_mining_gold_pile(position, depth, mining)
            .expect("mining gold should generate");

        assert_eq!(mined.appearance, ordinary.appearance);
        assert_eq!(mined.amount, ordinary.amount * 214 / 100);
    }
}

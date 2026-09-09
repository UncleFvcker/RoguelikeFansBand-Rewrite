// SPDX-License-Identifier: MPL-2.0
// bldg.c::gamble_comm/do_poker, RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c.

use rfb_protocol::{
    CasinoActionDto, CasinoDto, CasinoGameDto, CasinoRoundDto, CasinoRoundSaveDto,
    CasinoSessionDto, CasinoStateSaveDto, VirtueKindDto,
};

use super::{DomainEvent, Game};

impl Game {
    pub(super) fn casino_dto(&self, facility_id: &str) -> CasinoDto {
        CasinoDto {
            maximum_wager: (u32::from(self.progress.level) * 200).min(self.gold),
            session: self
                .casino
                .as_ref()
                .filter(|s| s.facility_id == facility_id)
                .map(|s| CasinoSessionDto {
                    game: s.game,
                    wager: s.wager,
                    starting_gold: s.starting_gold,
                    round: match &s.round {
                        CasinoRoundSaveDto::Poker { deck } => CasinoRoundDto::Poker {
                            cards: deck[..5].to_vec(),
                        },
                        CasinoRoundSaveDto::Craps { point, dice } => CasinoRoundDto::Craps {
                            point: *point,
                            dice: *dice,
                        },
                        CasinoRoundSaveDto::Finished { values, odds } => CasinoRoundDto::Finished {
                            values: values.clone(),
                            odds: *odds,
                            payout: s.wager * u32::from(*odds),
                            result_key: if s.game == CasinoGameDto::Poker {
                                poker_result(values).1
                            } else if *odds > 0 {
                                "casino-win"
                            } else {
                                "casino-loss"
                            }
                            .to_owned(),
                        },
                    },
                }),
        }
    }

    pub(super) fn casino_action(
        &mut self,
        facility_id: &str,
        action: CasinoActionDto,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), &'static str> {
        if !self
            .content
            .town_facility(facility_id)
            .is_some_and(|f| f.casino)
        {
            return Err("service-unavailable");
        }
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        if let CasinoActionDto::Start {
            game,
            wager,
            roulette_choice,
        } = action
        {
            if self.casino.is_some() {
                return Err("casino-in-progress");
            }
            let mut state = CasinoStateSaveDto {
                facility_id: facility_id.to_owned(),
                game,
                wager,
                starting_gold: self.gold,
                round: CasinoRoundSaveDto::Finished {
                    values: Vec::new(),
                    odds: 0,
                },
            };
            self.start_casino_round(&mut state, roulette_choice, events)?;
            self.casino = Some(state);
            return Ok(());
        }
        let mut state = self
            .casino
            .as_ref()
            .filter(|s| s.facility_id == facility_id)
            .ok_or("casino-not-started")?
            .clone();
        match (&state.round, action) {
            (CasinoRoundSaveDto::Finished { .. }, CasinoActionDto::Leave) => {
                self.add_virtue(
                    VirtueKindDto::Chance,
                    if self.gold >= state.starting_gold {
                        3
                    } else {
                        -3
                    },
                );
                self.casino = None;
                events.push(DomainEvent::CasinoSessionEnded {
                    facility_id: facility_id.to_owned(),
                    gold_balance: self.gold,
                });
                return Ok(());
            }
            (CasinoRoundSaveDto::Finished { .. }, CasinoActionDto::Again { roulette_choice }) => {
                self.start_casino_round(&mut state, roulette_choice, events)?;
            }
            (CasinoRoundSaveDto::Craps { point, .. }, CasinoActionDto::Roll) => {
                let point = *point;
                let dice = self.casino_dice();
                if dice[0] + dice[1] == point || dice[0] + dice[1] == 7 {
                    self.finish_casino_round(
                        &mut state,
                        vec![point, dice[0], dice[1]],
                        if dice[0] + dice[1] == point { 2 } else { 0 },
                        events,
                    );
                } else {
                    state.round = CasinoRoundSaveDto::Craps { point, dice };
                }
            }
            (CasinoRoundSaveDto::Poker { deck }, CasinoActionDto::Draw { replace_mask })
                if replace_mask < 32 =>
            {
                let mut cards = deck[..5].to_vec();
                let mut next = 5;
                for (index, card) in cards.iter_mut().enumerate() {
                    if replace_mask & (1 << index) != 0 {
                        *card = deck[next];
                        next += 1;
                    }
                }
                let odds = poker_result(&cards).0;
                self.finish_casino_round(&mut state, cards, odds, events);
            }
            _ => return Err("casino-action-unavailable"),
        }
        self.casino = Some(state);
        Ok(())
    }

    fn start_casino_round(
        &mut self,
        state: &mut CasinoStateSaveDto,
        choice: Option<u8>,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), &'static str> {
        use CasinoGameDto::*;
        if state.wager == 0 || state.wager > u32::from(self.progress.level) * 200 {
            return Err("invalid-wager");
        }
        if self.gold < state.wager {
            return Err("insufficient-gold");
        }
        if (state.game == Roulette && choice.is_none_or(|n| n > 9))
            || (state.game != Roulette && choice.is_some())
        {
            return Err("invalid-choice");
        }
        // Validate the representable bankroll before paying or drawing any randomness.
        let maximum_odds = match state.game {
            InBetween => 4,
            Craps => 2,
            Roulette => 9,
            DiceSlots => 1000,
            Poker => 3000,
        };
        if u64::from(self.gold - state.wager) + u64::from(state.wager) * maximum_odds
            > u64::from(u32::MAX)
        {
            return Err("gold-limit");
        }
        self.gold -= state.wager;
        match state.game {
            InBetween => {
                let a = self.rng.bounded(10) as u8 + 1;
                let b = self.rng.bounded(10) as u8 + 1;
                let red = self.rng.bounded(10) as u8 + 1;
                self.finish_casino_round(
                    state,
                    vec![a, b, red],
                    if red > a.min(b) && red < a.max(b) {
                        4
                    } else {
                        0
                    },
                    events,
                );
            }
            Craps => {
                let dice = self.casino_dice();
                match dice[0] + dice[1] {
                    7 | 11 => self.finish_casino_round(state, vec![0, dice[0], dice[1]], 2, events),
                    2 | 3 | 12 => {
                        self.finish_casino_round(state, vec![0, dice[0], dice[1]], 0, events)
                    }
                    point => state.round = CasinoRoundSaveDto::Craps { point, dice },
                }
            }
            Roulette => {
                let number = self.rng.bounded(10) as u8;
                let chosen = choice.expect("roulette choice was validated");
                self.finish_casino_round(
                    state,
                    vec![chosen, number],
                    if chosen == number { 9 } else { 0 },
                    events,
                );
            }
            DiceSlots => {
                let values = (0..3)
                    .map(|_| slot_symbol(self.rng.bounded(21) as u8))
                    .collect::<Vec<_>>();
                let odds = slot_odds(&values);
                self.finish_casino_round(state, values, odds, events);
            }
            Poker => {
                let mut deck = (0..53).collect::<Vec<u8>>();
                for index in 0..53 {
                    let other = index + self.rng.bounded((53 - index) as u64) as usize;
                    deck.swap(index, other);
                }
                state.round = CasinoRoundSaveDto::Poker { deck };
            }
        }
        Ok(())
    }

    fn casino_dice(&mut self) -> [u8; 2] {
        [self.rng.bounded(6) as u8 + 1, self.rng.bounded(6) as u8 + 1]
    }

    fn finish_casino_round(
        &mut self,
        state: &mut CasinoStateSaveDto,
        values: Vec<u8>,
        odds: u16,
        events: &mut Vec<DomainEvent>,
    ) {
        let payout = state.wager * u32::from(odds);
        self.gold += payout;
        state.round = CasinoRoundSaveDto::Finished { values, odds };
        events.push(DomainEvent::CasinoRoundCompleted {
            facility_id: state.facility_id.clone(),
            wager: state.wager,
            payout,
            gold_balance: self.gold,
        });
    }

    pub(super) fn casino_state_is_valid(&self) -> bool {
        use CasinoGameDto::*;
        self.casino.as_ref().is_none_or(|s| {
            if s.wager == 0
                || s.wager > u32::from(self.progress.level) * 200
                || s.starting_gold < s.wager
                || !self
                    .content
                    .town_facility(&s.facility_id)
                    .is_some_and(|f| f.casino)
                || !self.town_facility_accessible(&s.facility_id)
            {
                return false;
            }
            let valid_dice = |v: &[u8]| v.iter().all(|n| (1..=6).contains(n));
            match &s.round {
                CasinoRoundSaveDto::Poker { deck } => {
                    s.game == Poker
                        && deck.len() == 53
                        && {
                            let mut sorted = deck.clone();
                            sorted.sort_unstable();
                            sorted == (0..53).collect::<Vec<_>>()
                        }
                        && self.gold.checked_add(s.wager * 3000).is_some()
                }
                CasinoRoundSaveDto::Craps { point, dice } => {
                    s.game == Craps
                        && [4, 5, 6, 8, 9, 10].contains(point)
                        && valid_dice(dice)
                        && dice[0] + dice[1] != 7
                        && self.gold.checked_add(s.wager * 2).is_some()
                }
                CasinoRoundSaveDto::Finished { values: v, odds } => {
                    let expected = match s.game {
                        InBetween if v.len() == 3 && v.iter().all(|n| (1..=10).contains(n)) => {
                            Some(if v[2] > v[0].min(v[1]) && v[2] < v[0].max(v[1]) {
                                4
                            } else {
                                0
                            })
                        }
                        Roulette if v.len() == 2 && v.iter().all(|n| *n < 10) => {
                            Some(if v[0] == v[1] { 9 } else { 0 })
                        }
                        DiceSlots if v.len() == 3 && valid_dice(v) => Some(slot_odds(v)),
                        Poker
                            if v.len() == 5
                                && v.iter().all(|n| *n < 53)
                                && v.iter().enumerate().all(|(i, n)| !v[..i].contains(n)) =>
                        {
                            Some(poker_result(v).0)
                        }
                        Craps if v.len() == 3 && valid_dice(&v[1..]) => {
                            let sum = v[1] + v[2];
                            if v[0] == 0 && [2, 3, 7, 11, 12].contains(&sum) {
                                Some(if sum == 7 || sum == 11 { 2 } else { 0 })
                            } else if [4, 5, 6, 8, 9, 10].contains(&v[0])
                                && (sum == v[0] || sum == 7)
                            {
                                Some(if sum == v[0] { 2 } else { 0 })
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    expected == Some(*odds)
                }
            }
        })
    }
}

fn slot_symbol(mut roll: u8) -> u8 {
    for weight in (1..=6).rev() {
        if roll < weight {
            return 7 - weight;
        }
        roll -= weight;
    }
    unreachable!("slot draw is in 0..21")
}

fn slot_odds(values: &[u8]) -> u16 {
    if values[0] == values[1] && values[1] == values[2] {
        [5, 10, 20, 50, 200, 1000][usize::from(values[0] - 1)]
    } else if values[0] == 1 && values[1] == 1 {
        2
    } else {
        0
    }
}

fn poker_result(cards: &[u8]) -> (u16, &'static str) {
    let joker = cards.contains(&52);
    let rank = |n| cards.iter().any(|card| *card != 52 && card % 13 == n);
    let suit = cards.iter().find(|card| **card != 52).unwrap() / 13;
    let flush = cards.iter().all(|card| *card == 52 || card / 13 == suit);
    let lowest = cards
        .iter()
        .filter(|card| **card != 52)
        .map(|card| card % 13)
        .min()
        .unwrap();
    if flush
        && ((lowest == 0 && (9..13).filter(|n| !rank(*n)).count() <= usize::from(joker))
            || (lowest == 9 && joker && (10..13).all(rank)))
    {
        return (200, "casino-poker-royal-flush");
    }
    let straight = (lowest..lowest + 5).filter(|n| !rank(*n)).count() <= usize::from(joker);
    if straight {
        return if flush {
            (80, "casino-poker-straight-flush")
        } else {
            (4, "casino-poker-straight")
        };
    }
    if flush {
        return (8, "casino-poker-flush");
    }
    let mut pairs = cards
        .iter()
        .enumerate()
        .map(|(i, card)| {
            cards[i + 1..]
                .iter()
                .filter(|other| *card != 52 && **other != 52 && *card % 13 == **other % 13)
                .count()
        })
        .sum::<usize>();
    if joker {
        pairs = match pairs {
            0 => 1,
            1 => 3,
            2 => 4,
            3 => 6,
            6 => 7,
            n => n,
        };
    }
    match pairs {
        1 => (0, "casino-poker-pair"),
        2 => (1, "casino-poker-two-pair"),
        3 => (1, "casino-poker-three"),
        4 => (12, "casino-poker-full-house"),
        6 => (16, "casino-poker-four"),
        7 if rank(0) => (3000, "casino-poker-five-aces"),
        7 => (400, "casino-poker-five"),
        _ => (0, "casino-poker-no-pair"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casino_source_paytables_include_joker_and_weighted_slots() {
        let mut counts = [0; 6];
        for roll in 0..21 {
            counts[usize::from(slot_symbol(roll) - 1)] += 1;
        }
        assert_eq!(counts, [6, 5, 4, 3, 2, 1]);
        assert_eq!(slot_odds(&[1, 1, 2]), 2);
        assert_eq!(slot_odds(&[6, 6, 6]), 1000);
        assert_eq!(slot_odds(&[2, 1, 1]), 0);
        for (cards, odds) in [
            ([0, 9, 10, 11, 12], 200),
            ([52, 9, 10, 11, 12], 200),
            ([52, 0, 10, 11, 12], 200),
            ([2, 3, 4, 5, 52], 80),
            ([2, 16, 30, 5, 52], 4),
            ([0, 2, 5, 8, 11], 8),
            ([0, 13, 26, 39, 52], 3000),
            ([1, 14, 27, 40, 52], 400),
            ([1, 14, 27, 40, 8], 16),
            ([1, 14, 2, 15, 52], 12),
            ([1, 14, 27, 2, 15], 12),
            ([1, 14, 2, 15, 8], 1),
            ([1, 14, 27, 4, 8], 1),
            ([1, 14, 3, 5, 8], 0),
            ([1, 16, 5, 20, 52], 0),
        ] {
            assert_eq!(poker_result(&cards).0, odds, "{cards:?}");
        }
    }
}

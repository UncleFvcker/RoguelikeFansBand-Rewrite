// SPDX-License-Identifier: MPL-2.0

use crate::error::CoreError;
use rfb_protocol::RngSaveDto;

pub const RNG_ALGORITHM: &str = "rfb-rng-xoshiro256ss-v1";

const RFB_RANDNOR_TABLE: [u16; 256] = [
    206, 613, 1022, 1430, 1838, 2245, 2652, 3058, 3463, 3867, 4271, 4673, 5075, 5475, 5874, 6271,
    6667, 7061, 7454, 7845, 8234, 8621, 9006, 9389, 9770, 10148, 10524, 10898, 11269, 11638, 12004,
    12367, 12727, 13085, 13440, 13792, 14140, 14486, 14828, 15168, 15504, 15836, 16166, 16492,
    16814, 17133, 17449, 17761, 18069, 18374, 18675, 18972, 19266, 19556, 19842, 20124, 20403,
    20678, 20949, 21216, 21479, 21738, 21994, 22245, 22493, 22737, 22977, 23213, 23446, 23674,
    23899, 24120, 24336, 24550, 24759, 24965, 25166, 25365, 25559, 25750, 25937, 26120, 26300,
    26476, 26649, 26818, 26983, 27146, 27304, 27460, 27612, 27760, 27906, 28048, 28187, 28323,
    28455, 28585, 28711, 28835, 28955, 29073, 29188, 29299, 29409, 29515, 29619, 29720, 29818,
    29914, 30007, 30098, 30186, 30272, 30356, 30437, 30516, 30593, 30668, 30740, 30810, 30879,
    30945, 31010, 31072, 31133, 31192, 31249, 31304, 31358, 31410, 31460, 31509, 31556, 31601,
    31646, 31688, 31730, 31770, 31808, 31846, 31882, 31917, 31950, 31983, 32014, 32044, 32074,
    32102, 32129, 32155, 32180, 32205, 32228, 32251, 32273, 32294, 32314, 32333, 32352, 32370,
    32387, 32404, 32420, 32435, 32450, 32464, 32477, 32490, 32503, 32515, 32526, 32537, 32548,
    32558, 32568, 32577, 32586, 32595, 32603, 32611, 32618, 32625, 32632, 32639, 32645, 32651,
    32657, 32662, 32667, 32672, 32677, 32682, 32686, 32690, 32694, 32698, 32702, 32705, 32708,
    32711, 32714, 32717, 32720, 32722, 32725, 32727, 32729, 32731, 32733, 32735, 32737, 32739,
    32740, 32742, 32743, 32745, 32746, 32747, 32748, 32749, 32750, 32751, 32752, 32753, 32754,
    32755, 32756, 32757, 32757, 32758, 32758, 32759, 32760, 32760, 32761, 32761, 32761, 32762,
    32762, 32763, 32763, 32763, 32764, 32764, 32764, 32764, 32765, 32765, 32765, 32765, 32766,
    32766, 32766, 32766, 32767,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfbRng {
    pub(crate) state: [u64; 4],
    pub(crate) draw_counter: u64,
}

impl RfbRng {
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        let mut splitmix_state = seed;
        let mut state = [0_u64; 4];
        for value in &mut state {
            *value = splitmix64(&mut splitmix_state);
        }
        if state == [0; 4] {
            state[0] = 1;
        }
        Self {
            state,
            draw_counter: 0,
        }
    }

    pub(crate) fn from_save(save: &RngSaveDto) -> Result<Self, CoreError> {
        if save.algorithm != RNG_ALGORITHM {
            return Err(CoreError::UnsupportedRng(save.algorithm.clone()));
        }
        if save.state == [0; 4] {
            return Err(CoreError::InvalidSave("RNG state cannot be all zero"));
        }
        Ok(Self {
            state: save.state,
            draw_counter: save.draw_counter,
        })
    }

    pub(crate) fn to_save(&self) -> RngSaveDto {
        RngSaveDto {
            algorithm: RNG_ALGORITHM.to_owned(),
            state: self.state,
            draw_counter: self.draw_counter,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let result = self.state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.state[1] << 17;

        self.state[2] ^= self.state[0];
        self.state[3] ^= self.state[1];
        self.state[1] ^= self.state[2];
        self.state[0] ^= self.state[3];
        self.state[2] ^= t;
        self.state[3] = self.state[3].rotate_left(45);
        self.draw_counter = self.draw_counter.wrapping_add(1);
        result
    }

    pub(crate) fn bounded(&mut self, upper_exclusive: u64) -> u64 {
        assert!(upper_exclusive > 0, "RNG bound must be positive");
        let threshold = upper_exclusive.wrapping_neg() % upper_exclusive;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % upper_exclusive;
            }
        }
    }
}

/// Exact RFB `m_bonus(maximum, level)` roll with an explicit generation level.
pub(crate) fn rfb_m_bonus(rng: &mut RfbRng, maximum: u16, generation_level: u16) -> u16 {
    let level = generation_level.min(127);
    let product = u32::from(maximum).saturating_mul(u32::from(level));
    let mut mean = i32::try_from(product / 128).expect("RFB bonus mean must fit i32");
    if rng.bounded(128) < u64::from(product % 128) {
        mean += 1;
    }
    let mut deviation = maximum / 4;
    if rng.bounded(4) < u64::from(maximum % 4) {
        deviation += 1;
    }
    let value = if deviation == 0 {
        mean
    } else {
        let roll = u16::try_from(rng.bounded(32_768)).expect("d32768 roll must fit u16");
        let category = RFB_RANDNOR_TABLE.partition_point(|threshold| *threshold < roll);
        let offset = i32::try_from(usize::from(deviation).saturating_mul(category) / 64)
            .expect("RFB normal offset must fit i32");
        if rng.bounded(100) < 50 {
            mean.saturating_sub(offset)
        } else {
            mean.saturating_add(offset)
        }
    };
    u16::try_from(value.clamp(0, i32::from(maximum))).expect("bounded RFB bonus must fit u16")
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::{RfbRng, rfb_m_bonus};

    #[test]
    fn m_bonus_uses_explicit_level_and_supports_large_deviations() {
        let mut low = RfbRng::seeded(17);
        let mut high = RfbRng::seeded(17);
        assert_eq!(rfb_m_bonus(&mut low, 10, 0), 0);
        assert_eq!(rfb_m_bonus(&mut high, 10, 127), 6);
        assert_eq!(low.draw_counter, 4);
        assert_eq!(high.draw_counter, 4);

        let mut broad = RfbRng::seeded(91);
        let value = rfb_m_bonus(&mut broad, 75, 50);
        assert!(value <= 75);
        assert_eq!(broad.draw_counter, 4);
    }
}

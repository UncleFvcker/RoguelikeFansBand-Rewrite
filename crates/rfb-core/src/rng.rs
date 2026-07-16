// SPDX-License-Identifier: MPL-2.0

use crate::error::CoreError;
use rfb_protocol::RngSaveDto;

pub const RNG_ALGORITHM: &str = "rfb-rng-xoshiro256ss-v1";

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

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

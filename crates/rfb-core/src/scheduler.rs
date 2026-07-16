// SPDX-License-Identifier: MPL-2.0

pub(crate) const STANDARD_ACTION_COST: i32 = 100;
pub(crate) const INITIAL_PLAYER_ENERGY_NEED: i32 = 0;
pub(crate) const INITIAL_MONSTER_ENERGY_NEED: i32 = STANDARD_ACTION_COST;

const ENERGY_CURVE: [(u16, i32); 14] = [
    (0, 1),
    (70, 2),
    (80, 2),
    (90, 3),
    (100, 5),
    (110, 10),
    (120, 20),
    (130, 30),
    (140, 38),
    (150, 42),
    (160, 45),
    (170, 47),
    (180, 49),
    (199, 49),
];

pub(crate) fn energy_gain(speed: u16) -> i32 {
    let speed = speed.min(199);
    for window in ENERGY_CURVE.windows(2) {
        let (left_speed, left_gain) = window[0];
        let (right_speed, right_gain) = window[1];
        if speed <= right_speed {
            let speed_span = i32::from(right_speed - left_speed);
            let speed_offset = i32::from(speed.saturating_sub(left_speed));
            let gain_span = right_gain - left_gain;
            return left_gain + gain_span * speed_offset / speed_span.max(1);
        }
    }
    49
}

pub(crate) fn gain_energy(energy_need: &mut i32, speed: u16) {
    *energy_need = energy_need.saturating_sub(energy_gain(speed));
}

pub(crate) fn spend_energy(energy_need: &mut i32, cost: i32) {
    *energy_need = energy_need.saturating_add(cost.max(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_speed_gains_ten_energy_per_world_tick() {
        assert_eq!(energy_gain(110), 10);
        assert_eq!(energy_gain(120), 20);
        assert_eq!(energy_gain(100), 5);
    }

    #[test]
    fn energy_curve_is_monotonic_and_bounded() {
        let gains = (0..=u16::MAX).map(energy_gain).collect::<Vec<_>>();
        assert!(gains.windows(2).all(|window| window[0] <= window[1]));
        assert!(gains.iter().all(|gain| (1..=49).contains(gain)));
    }
}

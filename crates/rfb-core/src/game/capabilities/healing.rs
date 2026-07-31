// SPDX-License-Identifier: MPL-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::game) struct HealingRequest {
    amount: i32,
}

impl HealingRequest {
    pub(in crate::game) const fn amount(amount: i32) -> Self {
        Self { amount }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::game) struct HealingOutcome {
    pub(in crate::game) requested: i32,
    pub(in crate::game) applied: i32,
}

pub(in crate::game) fn apply_healing(
    hit_points: &mut i32,
    maximum_hit_points: i32,
    request: HealingRequest,
) -> HealingOutcome {
    let requested = request.amount.max(0);
    let before = *hit_points;
    *hit_points = hit_points.saturating_add(requested).min(maximum_hit_points);
    HealingOutcome {
        requested,
        applied: hit_points.saturating_sub(before),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healing_clamps_requested_and_applied_amounts() {
        let mut hit_points = 3;
        let bounded = apply_healing(&mut hit_points, 10, HealingRequest::amount(9));
        assert_eq!(
            bounded,
            HealingOutcome {
                requested: 9,
                applied: 7,
            }
        );
        assert_eq!(hit_points, 10);

        let full = apply_healing(&mut hit_points, 10, HealingRequest::amount(4));
        assert_eq!(
            full,
            HealingOutcome {
                requested: 4,
                applied: 0,
            }
        );
        assert_eq!(hit_points, 10);

        hit_points = 8;
        let negative = apply_healing(&mut hit_points, 10, HealingRequest::amount(-5));
        assert_eq!(
            negative,
            HealingOutcome {
                requested: 0,
                applied: 0,
            }
        );
        assert_eq!(hit_points, 8);
    }
}

// SPDX-License-Identifier: MPL-2.0

use crate::{rng::RfbRng, stats::DerivedStat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckKind {
    MeleeHit,
    ProjectileHit,
    ThrowHit,
    FearAction,
    UnlockDoor,
    BashDoor,
    SearchTerrain,
    DisarmTrap,
    DigTerrain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckContext {
    pub kind: CheckKind,
    pub actor_id: String,
    pub target_id: Option<String>,
    pub ability: DerivedStat,
    pub difficulty: DerivedStat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    AutomaticSuccess,
    AutomaticFailure,
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub context: CheckContext,
    pub outcome: CheckOutcome,
    pub percentile_roll: u8,
    pub contest_roll: Option<i32>,
    pub threshold: i32,
}

impl CheckResult {
    #[must_use]
    pub const fn succeeded(&self) -> bool {
        matches!(
            self.outcome,
            CheckOutcome::AutomaticSuccess | CheckOutcome::Success
        )
    }
}

#[must_use]
pub fn resolve_check(rng: &mut RfbRng, context: CheckContext) -> CheckResult {
    let percentile_roll = u8::try_from(rng.bounded(100)).expect("percentile roll must fit u8");
    let threshold = context.difficulty.value.max(0).saturating_mul(3) / 4;
    if percentile_roll < 5 {
        return CheckResult {
            context,
            outcome: CheckOutcome::AutomaticSuccess,
            percentile_roll,
            contest_roll: None,
            threshold,
        };
    }
    if percentile_roll < 10 || context.ability.value <= 0 {
        return CheckResult {
            context,
            outcome: if percentile_roll < 10 {
                CheckOutcome::AutomaticFailure
            } else {
                CheckOutcome::Failure
            },
            percentile_roll,
            contest_roll: None,
            threshold,
        };
    }
    let contest_roll = i32::try_from(rng.bounded(context.ability.value as u64)).unwrap_or(i32::MAX);
    CheckResult {
        context,
        outcome: if contest_roll >= threshold {
            CheckOutcome::Success
        } else {
            CheckOutcome::Failure
        },
        percentile_roll,
        contest_roll: Some(contest_roll),
        threshold,
    }
}

#[cfg(test)]
mod tests {
    use crate::stats::{DerivedStatsPipeline, StatBounds, StatKind, StatLayer};

    use super::*;

    fn context(skill: i32, armor_class: i32) -> CheckContext {
        let mut stats = DerivedStatsPipeline::new();
        stats.add(
            StatKind::MeleeSkill,
            StatLayer::Base,
            "demo.actor.attacker",
            skill,
        );
        stats.add(
            StatKind::ArmorClass,
            StatLayer::Base,
            "demo.actor.target",
            armor_class,
        );
        CheckContext {
            kind: CheckKind::MeleeHit,
            actor_id: "demo.actor.attacker.1".to_owned(),
            target_id: Some("demo.actor.target.1".to_owned()),
            ability: stats.resolve(StatKind::MeleeSkill, StatBounds::NON_NEGATIVE),
            difficulty: stats.resolve(StatKind::ArmorClass, StatBounds::NON_NEGATIVE),
        }
    }

    #[test]
    fn check_result_keeps_context_rolls_and_threshold() {
        let mut rng = RfbRng::seeded(42);
        let result = resolve_check(&mut rng, context(60, 20));

        assert_eq!(result.context.actor_id, "demo.actor.attacker.1");
        assert_eq!(
            result.context.target_id.as_deref(),
            Some("demo.actor.target.1")
        );
        assert_eq!(result.threshold, 15);
        assert!(result.percentile_roll < 100);
        assert_eq!(result.contest_roll.is_some(), result.percentile_roll >= 10);
    }

    #[test]
    fn non_positive_ability_fails_without_a_contest_roll() {
        let mut seed = 0_u64;
        let result = loop {
            let mut rng = RfbRng::seeded(seed);
            let result = resolve_check(&mut rng, context(0, 20));
            if result.percentile_roll >= 10 {
                break result;
            }
            seed += 1;
        };

        assert_eq!(result.outcome, CheckOutcome::Failure);
        assert_eq!(result.contest_roll, None);
    }
}

// SPDX-License-Identifier: MPL-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatKind {
    MaxHp,
    Attack,
    Defense,
    Speed,
    MeleeSkill,
    ArmorClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatLayer {
    Base,
    Species,
    Class,
    Personality,
    Equipment,
    Status,
    Stance,
    Environment,
}

impl StatLayer {
    const fn priority(self) -> i16 {
        match self {
            Self::Base => 0,
            Self::Species => 100,
            Self::Class => 200,
            Self::Personality => 300,
            Self::Equipment => 400,
            Self::Status => 500,
            Self::Stance => 600,
            Self::Environment => 700,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatContribution {
    pub source_id: String,
    pub origin_id: Option<String>,
    pub layer: StatLayer,
    pub priority: i16,
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedStat {
    pub kind: StatKind,
    pub value: i32,
    pub contributions: Vec<StatContribution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatBounds {
    minimum: i32,
    maximum: i32,
}

impl StatBounds {
    pub const UNBOUNDED: Self = Self {
        minimum: i32::MIN,
        maximum: i32::MAX,
    };
    pub const NON_NEGATIVE: Self = Self {
        minimum: 0,
        maximum: i32::MAX,
    };
    pub const ACTOR_SPEED: Self = Self {
        minimum: 0,
        maximum: 199,
    };
}

#[derive(Debug, Clone, Default)]
pub struct DerivedStatsPipeline {
    entries: Vec<(StatKind, StatContribution)>,
}

impl DerivedStatsPipeline {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        source_id: impl Into<String>,
        amount: i32,
    ) {
        self.add_detailed(kind, layer, layer.priority(), source_id, None, amount);
    }

    pub fn add_with_priority(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        priority: i16,
        source_id: impl Into<String>,
        amount: i32,
    ) {
        self.add_detailed(kind, layer, priority, source_id, None, amount);
    }

    pub fn add_with_origin(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        source_id: impl Into<String>,
        origin_id: Option<String>,
        amount: i32,
    ) {
        self.add_detailed(kind, layer, layer.priority(), source_id, origin_id, amount);
    }

    fn add_detailed(
        &mut self,
        kind: StatKind,
        layer: StatLayer,
        priority: i16,
        source_id: impl Into<String>,
        origin_id: Option<String>,
        amount: i32,
    ) {
        self.entries.push((
            kind,
            StatContribution {
                source_id: source_id.into(),
                origin_id,
                layer,
                priority,
                amount,
            },
        ));
    }

    #[must_use]
    pub fn resolve(&self, kind: StatKind, bounds: StatBounds) -> DerivedStat {
        let mut contributions = self
            .entries
            .iter()
            .filter(|(entry_kind, _)| *entry_kind == kind)
            .map(|(_, contribution)| contribution.clone())
            .collect::<Vec<_>>();
        contributions.sort_by(|left, right| {
            (
                left.priority,
                left.layer,
                left.source_id.as_str(),
                left.origin_id.as_deref(),
            )
                .cmp(&(
                    right.priority,
                    right.layer,
                    right.source_id.as_str(),
                    right.origin_id.as_deref(),
                ))
        });
        let value = contributions
            .iter()
            .fold(0_i32, |total, contribution| {
                total.saturating_add(contribution.amount)
            })
            .clamp(bounds.minimum, bounds.maximum);
        DerivedStat {
            kind,
            value,
            contributions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_orders_sources_by_layer_priority_and_id() {
        let mut pipeline = DerivedStatsPipeline::new();
        pipeline.add(
            StatKind::Attack,
            StatLayer::Equipment,
            "demo.item.zeta.1",
            2,
        );
        pipeline.add(StatKind::Attack, StatLayer::Base, "demo.actor.explorer", 3);
        pipeline.add(
            StatKind::Attack,
            StatLayer::Equipment,
            "demo.item.alpha.1",
            -1,
        );

        let result = pipeline.resolve(StatKind::Attack, StatBounds::NON_NEGATIVE);

        assert_eq!(result.value, 4);
        assert_eq!(
            result
                .contributions
                .iter()
                .map(|contribution| contribution.source_id.as_str())
                .collect::<Vec<_>>(),
            [
                "demo.actor.explorer",
                "demo.item.alpha.1",
                "demo.item.zeta.1"
            ]
        );
    }

    #[test]
    fn pipeline_clamps_only_the_final_saturating_total() {
        let mut pipeline = DerivedStatsPipeline::new();
        pipeline.add(StatKind::Speed, StatLayer::Base, "demo.actor.explorer", 190);
        pipeline.add(StatKind::Speed, StatLayer::Status, "rfb.status.haste", 20);

        let result = pipeline.resolve(StatKind::Speed, StatBounds::ACTOR_SPEED);

        assert_eq!(result.value, 199);
        assert_eq!(result.contributions.len(), 2);
    }
}

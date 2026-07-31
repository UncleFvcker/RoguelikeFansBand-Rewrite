// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::state::ResourcePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceRestoration {
    Amount(u32),
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::game) struct ResourceRestorationRequest<'a> {
    resource_id: &'a str,
    restoration: ResourceRestoration,
}

impl<'a> ResourceRestorationRequest<'a> {
    pub(in crate::game) const fn amount(resource_id: &'a str, amount: u32) -> Self {
        Self {
            resource_id,
            restoration: ResourceRestoration::Amount(amount),
        }
    }

    pub(in crate::game) const fn full(resource_id: &'a str) -> Self {
        Self {
            resource_id,
            restoration: ResourceRestoration::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::game) struct ResourceRestorationOutcome {
    pub(in crate::game) resource_id: String,
    pub(in crate::game) before: u32,
    pub(in crate::game) after: u32,
    pub(in crate::game) recovered: u32,
}

pub(in crate::game) fn apply_resource_restoration(
    resources: &mut BTreeMap<String, ResourcePool>,
    resources_touched: &mut BTreeSet<String>,
    request: ResourceRestorationRequest<'_>,
) -> ResourceRestorationOutcome {
    let (before, after) = if let Some(pool) = resources.get_mut(request.resource_id) {
        let before = pool.current;
        pool.current = match request.restoration {
            ResourceRestoration::Amount(amount) => {
                pool.current.saturating_add(amount).min(pool.maximum)
            }
            ResourceRestoration::Full => pool.maximum,
        };
        (before, pool.current)
    } else {
        (0, 0)
    };
    let recovered = after.saturating_sub(before);
    if recovered > 0 {
        resources_touched.insert(request.resource_id.to_owned());
    }
    ResourceRestorationOutcome {
        resource_id: request.resource_id.to_owned(),
        before,
        after,
        recovered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restoration_reports_missing_bounded_and_full_resource_outcomes() {
        let mut resources = BTreeMap::from([(
            "test.resource.mana".to_owned(),
            ResourcePool {
                current: 3,
                maximum: 10,
            },
        )]);
        let mut touched = BTreeSet::new();

        let missing = apply_resource_restoration(
            &mut resources,
            &mut touched,
            ResourceRestorationRequest::amount("test.resource.missing", 4),
        );
        assert_eq!(
            missing,
            ResourceRestorationOutcome {
                resource_id: "test.resource.missing".to_owned(),
                before: 0,
                after: 0,
                recovered: 0,
            }
        );
        assert!(touched.is_empty());

        let bounded = apply_resource_restoration(
            &mut resources,
            &mut touched,
            ResourceRestorationRequest::amount("test.resource.mana", 9),
        );
        assert_eq!(
            bounded,
            ResourceRestorationOutcome {
                resource_id: "test.resource.mana".to_owned(),
                before: 3,
                after: 10,
                recovered: 7,
            }
        );
        assert!(touched.contains("test.resource.mana"));

        resources
            .get_mut("test.resource.mana")
            .expect("test resource should remain available")
            .current = 4;
        let full = apply_resource_restoration(
            &mut resources,
            &mut touched,
            ResourceRestorationRequest::full("test.resource.mana"),
        );
        assert_eq!(full.before, 4);
        assert_eq!(full.after, 10);
        assert_eq!(full.recovered, 6);
    }
}

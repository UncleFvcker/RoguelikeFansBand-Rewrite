// SPDX-License-Identifier: MPL-2.0

mod items;

use std::collections::BTreeMap;

use crate::{ContentError, ItemDefinition, effect_programs::ResolvedEffectProgram};

pub(crate) use items::SourceItemDefinition;
#[cfg(test)]
pub(crate) use items::SourceItemUseActionDefinition;

pub(super) fn compile_source_item(
    item: SourceItemDefinition,
    programs: &BTreeMap<String, ResolvedEffectProgram>,
) -> Result<ItemDefinition, ContentError> {
    item.into_compiled(programs)
}

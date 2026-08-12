// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{
    ActorRole, AffixDefinition, ContentError, ContentPosition, ENCOUNTER_TABLE_SCHEMA,
    EncounterTableDefinition, ItemDefinition, LOOT_TABLE_SCHEMA, LootQualityPolicyDefinition,
    LootTableDefinition, MonsterPackBehavior, REGION_TABLE_SCHEMA, RegionTableDefinition,
    TERRAIN_FEATURE_TABLE_SCHEMA, THEME_TABLE_SCHEMA, TerrainDefinition, TerrainFeaturePlacement,
    TerrainFeatureTableDefinition, ThemeTableDefinition, VAULT_SCHEMA, VaultDefinition,
    affix_is_compatible_with_item,
};

use super::shared::{
    insert_definition_id, normalize_tags, require_actor_role, require_format_version,
    require_reference, require_schema, validate_definition_id, validate_glyph, validate_id,
    validate_message_key,
};

pub(super) struct TableDefinitions<'a> {
    pub(super) loot_tables: &'a mut [LootTableDefinition],
    pub(super) encounter_tables: &'a mut [EncounterTableDefinition],
    pub(super) vaults: &'a mut [VaultDefinition],
    pub(super) theme_tables: &'a mut [ThemeTableDefinition],
    pub(super) region_tables: &'a mut [RegionTableDefinition],
    pub(super) terrain_feature_tables: &'a mut [TerrainFeatureTableDefinition],
}

pub(super) struct TableValidationRefs<'a> {
    pub(super) item_limits: &'a BTreeMap<String, (u32, bool)>,
    pub(super) affix_ids: &'a BTreeSet<String>,
    pub(super) items: &'a [ItemDefinition],
    pub(super) affixes: &'a [AffixDefinition],
    pub(super) actor_loot_table_ids: Vec<(String, String)>,
    pub(super) actor_roles: &'a BTreeMap<String, ActorRole>,
    pub(super) actor_tag_values: &'a BTreeSet<String>,
    pub(super) actor_levels: &'a BTreeMap<String, u32>,
    pub(super) terrain_ids: &'a BTreeSet<String>,
    pub(super) terrain_walkability: &'a BTreeMap<String, bool>,
    pub(super) terrain_connectability: &'a BTreeMap<String, bool>,
    pub(super) terrain: &'a [TerrainDefinition],
}

pub(super) struct TableValidationOutputs {
    pub(super) loot_table_ids: BTreeSet<String>,
    pub(super) loot_tables_by_id: BTreeMap<String, LootTableDefinition>,
    pub(super) encounter_tables_by_id: BTreeMap<String, EncounterTableDefinition>,
    pub(super) vaults_by_id: BTreeMap<String, VaultDefinition>,
    pub(super) theme_tables_by_id: BTreeMap<String, ThemeTableDefinition>,
    pub(super) region_tables_by_id: BTreeMap<String, RegionTableDefinition>,
    pub(super) terrain_feature_tables_by_id: BTreeMap<String, TerrainFeatureTableDefinition>,
}

pub(super) fn validate_tables(
    definitions: TableDefinitions<'_>,
    refs: TableValidationRefs<'_>,
    all_ids: &mut BTreeSet<String>,
) -> Result<TableValidationOutputs, ContentError> {
    let TableValidationRefs {
        item_limits,
        affix_ids,
        items,
        affixes,
        actor_loot_table_ids,
        actor_roles,
        actor_tag_values,
        actor_levels,
        terrain_ids,
        terrain_walkability,
        terrain_connectability,
        terrain,
    } = refs;
    let mut loot_table_ids = BTreeSet::new();
    let mut loot_tables_by_id = BTreeMap::new();
    for table in definitions.loot_tables.iter_mut() {
        require_schema(&table.schema, LOOT_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "loot-table")?;
        let maximum_rolls = table.roll_dice.map_or(u32::from(table.rolls), |dice| {
            u32::from(table.rolls) + u32::from(dice.dice) * u32::from(dice.sides)
        });
        let has_quality_weights = !table.quality_weights.is_empty();
        if maximum_rolls == 0
            || maximum_rolls > 16
            || table
                .roll_chance_percent
                .is_some_and(|chance| !(1..=100).contains(&chance))
            || table
                .roll_dice
                .is_some_and(|dice| dice.dice == 0 || dice.sides == 0)
            || table.entries.is_empty()
            || table.entries.len() > 128
            || table.quality_weights.len() > 3
            || has_quality_weights == table.quality_policy.is_some()
            || table.quality_policy.is_some_and(|policy| match policy {
                LootQualityPolicyDefinition::RfbDepth {
                    good_cap_percent,
                    great_cap_percent,
                } => good_cap_percent > 100 || great_cap_percent > 100,
            })
            || table.affix_weights.is_empty()
            || table.affix_weights.len() > 64
        {
            return Err(ContentError::InvalidLootTable(table.id.clone()));
        }

        table.entries.sort_by(|left, right| {
            left.item_kind_id
                .cmp(&right.item_kind_id)
                .then(left.quantity.cmp(&right.quantity))
        });
        table.quality_weights.sort_by_key(|entry| entry.quality);
        table
            .affix_weights
            .sort_by(|left, right| left.affix_id.as_deref().cmp(&right.affix_id.as_deref()));

        let mut entry_ids = BTreeSet::new();
        let mut quality_ids = BTreeSet::new();
        let mut affix_entries = BTreeSet::new();
        let mut entry_weight = 0_u64;
        let mut quality_weight = 0_u64;
        let mut affix_weight = 0_u64;
        for entry in &table.entries {
            let Some((max_stack, _)) = item_limits.get(&entry.item_kind_id) else {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: entry.item_kind_id.clone(),
                });
            };
            if entry.weight == 0
                || entry.quantity == 0
                || entry.quantity > *max_stack
                || entry.min_depth > entry.max_depth
                || !entry_ids.insert(entry.item_kind_id.as_str())
            {
                return Err(ContentError::InvalidLootTable(table.id.clone()));
            }
            entry_weight = entry_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidLootTable(table.id.clone()))?;
        }
        for entry in &table.quality_weights {
            if entry.weight == 0 || !quality_ids.insert(entry.quality) {
                return Err(ContentError::InvalidLootTable(table.id.clone()));
            }
            quality_weight = quality_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidLootTable(table.id.clone()))?;
        }
        for entry in &table.affix_weights {
            if entry.weight == 0 || !affix_entries.insert(entry.affix_id.as_deref()) {
                return Err(ContentError::InvalidLootTable(table.id.clone()));
            }
            if let Some(affix_id) = &entry.affix_id
                && !affix_ids.contains(affix_id)
            {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: affix_id.clone(),
                });
            }
            affix_weight = affix_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidLootTable(table.id.clone()))?;
        }
        let named_affixes = table
            .affix_weights
            .iter()
            .filter_map(|entry| entry.affix_id.as_deref())
            .map(|affix_id| {
                affixes
                    .iter()
                    .find(|affix| affix.id == affix_id)
                    .expect("validated affix reference must remain available")
            })
            .collect::<Vec<_>>();
        let entry_accepts_affix = |entry: &crate::LootEntryDefinition, affix: &AffixDefinition| {
            let Some(item) = items.iter().find(|item| item.id == entry.item_kind_id) else {
                return false;
            };
            let generation_depth = entry.min_depth.max(affix.generation_level);
            generation_depth <= entry.max_depth.min(affix.generation_max_level)
                && affix_is_compatible_with_item(affix, item, generation_depth)
        };
        if named_affixes.iter().any(|affix| {
            !table
                .entries
                .iter()
                .any(|entry| entry_accepts_affix(entry, affix))
        }) || (table
            .affix_weights
            .iter()
            .all(|entry| entry.affix_id.is_some())
            && table.entries.iter().any(|entry| {
                !named_affixes
                    .iter()
                    .any(|affix| entry_accepts_affix(entry, affix))
            }))
        {
            return Err(ContentError::InvalidLootTable(table.id.clone()));
        }
        if entry_weight == 0
            || (table.quality_policy.is_none() && quality_weight == 0)
            || affix_weight == 0
        {
            return Err(ContentError::InvalidLootTable(table.id.clone()));
        }
        insert_definition_id(all_ids, &table.id)?;
        loot_table_ids.insert(table.id.clone());
        loot_tables_by_id.insert(table.id.clone(), table.clone());
    }

    for (actor_id, loot_table_id) in actor_loot_table_ids {
        require_reference(&loot_table_ids, &loot_table_id, &actor_id)?;
    }

    let mut encounter_tables_by_id = BTreeMap::new();
    for table in definitions.encounter_tables.iter_mut() {
        require_schema(&table.schema, ENCOUNTER_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "encounter-table")?;
        if table.rolls == 0 || table.rolls > 16 || table.entries.len() > 64 {
            return Err(ContentError::InvalidEncounterTable(table.id.clone()));
        }
        if let Some(allocation) = &mut table.global_allocation {
            allocation.preferred_glyphs.sort();
            normalize_tags(&table.id, &mut allocation.preferred_tags)?;
            let mut glyphs = BTreeSet::new();
            if !table.entries.is_empty()
                || (allocation.preferred_glyphs.is_empty() && allocation.preferred_tags.is_empty())
                || allocation.preferred_glyphs.len() > 64
                || allocation.preferred_tags.len() > 64
                || allocation.special_div > 64
                || allocation.ambient_chance_one_in == 0
                || allocation.ambient_chance_one_in > 10_000
                || allocation
                    .preferred_tags
                    .iter()
                    .any(|tag| !actor_tag_values.contains(tag))
                || allocation.preferred_glyphs.iter().any(|glyph| {
                    validate_glyph(&table.id, glyph).is_err() || !glyphs.insert(glyph.clone())
                })
            {
                return Err(ContentError::InvalidEncounterTable(table.id.clone()));
            }
        } else if table.entries.is_empty() {
            return Err(ContentError::InvalidEncounterTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.actor_kind_id
                .cmp(&right.actor_kind_id)
                .then(left.min_depth.cmp(&right.min_depth))
                .then(left.max_depth.cmp(&right.max_depth))
        });
        let mut actor_ids = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &mut table.entries {
            require_actor_role(
                actor_roles,
                &entry.actor_kind_id,
                ActorRole::Monster,
                &table.id,
            )?;
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || actor_levels
                    .get(&entry.actor_kind_id)
                    .is_none_or(|level| *level > u32::from(entry.max_depth))
                || !actor_ids.insert(entry.actor_kind_id.clone())
            {
                return Err(ContentError::InvalidEncounterTable(table.id.clone()));
            }
            if let Some(group) = &mut entry.group {
                let friends_are_valid = group.friends.as_ref().is_none_or(|friends| {
                    friends.max_count > 0
                        && friends.min_count <= friends.max_count
                        && friends.max_count <= 7
                });
                let escort_is_valid = group.escort.as_ref().is_none_or(|escort| {
                    escort.max_count > 0
                        && escort.min_count <= escort.max_count
                        && escort.max_count <= 7
                        && !escort.entries.is_empty()
                        && escort.entries.len() <= 64
                });
                if !friends_are_valid
                    || !escort_is_valid
                    || group.min_companion_count() == 0
                    || group.max_companion_count() > 7
                    || group.pack_ai.leader == MonsterPackBehavior::GuardLeader
                {
                    return Err(ContentError::InvalidEncounterTable(table.id.clone()));
                }
                if let Some(escort) = &mut group.escort {
                    escort.entries.sort_by(|left, right| {
                        left.actor_kind_id
                            .cmp(&right.actor_kind_id)
                            .then(left.min_depth.cmp(&right.min_depth))
                            .then(left.max_depth.cmp(&right.max_depth))
                    });
                    let mut escort_actor_ids = BTreeSet::new();
                    let mut escort_weight = 0_u64;
                    for escort_entry in &escort.entries {
                        require_actor_role(
                            actor_roles,
                            &escort_entry.actor_kind_id,
                            ActorRole::Monster,
                            &table.id,
                        )?;
                        if escort_entry.weight == 0
                            || escort_entry.min_depth < entry.min_depth
                            || escort_entry.min_depth > escort_entry.max_depth
                            || escort_entry.max_depth > entry.max_depth
                            || actor_levels
                                .get(&escort_entry.actor_kind_id)
                                .is_none_or(|level| *level > u32::from(escort_entry.max_depth))
                            || !escort_actor_ids.insert(escort_entry.actor_kind_id.clone())
                        {
                            return Err(ContentError::InvalidEncounterTable(table.id.clone()));
                        }
                        escort_weight = escort_weight
                            .checked_add(u64::from(escort_entry.weight))
                            .ok_or_else(|| {
                            ContentError::InvalidEncounterTable(table.id.clone())
                        })?;
                    }
                    if escort_weight == 0
                        || (entry.min_depth..=entry.max_depth).any(|depth| {
                            !escort.entries.iter().any(|escort_entry| {
                                escort_entry.min_depth <= depth
                                    && depth <= escort_entry.max_depth
                                    && actor_levels
                                        .get(&escort_entry.actor_kind_id)
                                        .is_some_and(|level| *level <= u32::from(depth))
                            })
                        })
                    {
                        return Err(ContentError::InvalidEncounterTable(table.id.clone()));
                    }
                }
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidEncounterTable(table.id.clone()))?;
        }
        if table.global_allocation.is_none() && total_weight == 0 {
            return Err(ContentError::InvalidEncounterTable(table.id.clone()));
        }
        insert_definition_id(all_ids, &table.id)?;
        encounter_tables_by_id.insert(table.id.clone(), table.clone());
    }

    let mut vaults_by_id = BTreeMap::new();
    for vault in definitions.vaults.iter_mut() {
        require_schema(&vault.schema, VAULT_SCHEMA, &vault.id)?;
        require_format_version(vault.format_version, &vault.id)?;
        validate_definition_id(&vault.id, "vault")?;
        validate_message_key(&vault.name_key)?;
        validate_definition_id(&vault.theme_id, "theme")?;
        if vault.entrance_positions.is_empty() {
            if let Some(legacy_position) = vault.entrance_position.take() {
                vault.entrance_positions.push(legacy_position);
            }
        } else if vault.entrance_position.is_some() {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        vault.entrance_positions.sort();
        vault.transforms.sort();
        let transform_count = vault
            .transforms
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len();
        if !(2..=12).contains(&vault.width)
            || !(2..=12).contains(&vault.height)
            || !(1..=8).contains(&vault.entrance_positions.len())
            || vault
                .entrance_positions
                .windows(2)
                .any(|positions| positions[0] == positions[1])
            || vault.entrance_positions.iter().any(|position| {
                position.x >= vault.width
                    || position.y >= vault.height
                    || !(position.x == 0
                        || position.x + 1 == vault.width
                        || position.y == 0
                        || position.y + 1 == vault.height)
            })
            || transform_count != vault.transforms.len()
        {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        require_reference(terrain_ids, &vault.base_terrain_id, &vault.id)?;
        if terrain_walkability.get(&vault.base_terrain_id) != Some(&true) {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }

        for terrain_override in &mut vault.terrain_overrides {
            terrain_override.positions.sort();
        }
        vault.terrain_overrides.sort_by(|left, right| {
            left.terrain_id
                .cmp(&right.terrain_id)
                .then(left.positions.cmp(&right.positions))
        });
        let mut terrain_by_position = BTreeMap::new();
        let mut terrain_override_ids = BTreeSet::new();
        for terrain_override in &mut vault.terrain_overrides {
            require_reference(terrain_ids, &terrain_override.terrain_id, &vault.id)?;
            if terrain_override.positions.is_empty()
                || !terrain_override_ids.insert(terrain_override.terrain_id.clone())
            {
                return Err(ContentError::InvalidVault(vault.id.clone()));
            }
            for position in &terrain_override.positions {
                if position.x >= vault.width
                    || position.y >= vault.height
                    || terrain_by_position
                        .insert(*position, terrain_override.terrain_id.clone())
                        .is_some()
                {
                    return Err(ContentError::InvalidVault(vault.id.clone()));
                }
            }
        }

        let connectable_positions = (0..vault.height)
            .flat_map(|y| (0..vault.width).map(move |x| ContentPosition { x, y }))
            .filter(|position| {
                let terrain_id = terrain_by_position
                    .get(position)
                    .unwrap_or(&vault.base_terrain_id);
                terrain_connectability
                    .get(terrain_id)
                    .copied()
                    .unwrap_or(false)
            })
            .collect::<BTreeSet<_>>();
        if vault
            .entrance_positions
            .iter()
            .any(|position| !connectable_positions.contains(position))
        {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        let mut reached = BTreeSet::new();
        let mut pending = VecDeque::from([vault.entrance_positions[0]]);
        while let Some(position) = pending.pop_front() {
            if !connectable_positions.contains(&position) || !reached.insert(position) {
                continue;
            }
            for (dx, dy) in [(0_i32, -1_i32), (1, 0), (0, 1), (-1, 0)] {
                let x = i32::from(position.x) + dx;
                let y = i32::from(position.y) + dy;
                if x >= 0 && y >= 0 && x < i32::from(vault.width) && y < i32::from(vault.height) {
                    pending.push_back(ContentPosition {
                        x: u16::try_from(x).expect("bounded Vault x must fit u16"),
                        y: u16::try_from(y).expect("bounded Vault y must fit u16"),
                    });
                }
            }
        }
        if reached != connectable_positions {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }

        vault
            .encounter_groups
            .sort_by(|left, right| left.id.cmp(&right.id));
        vault
            .loot_spawns
            .sort_by(|left, right| left.id.cmp(&right.id));
        if vault.encounter_groups.is_empty()
            || vault.encounter_groups.len() > 16
            || vault.loot_spawns.is_empty()
            || vault.loot_spawns.len() > 16
        {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        let mut section_ids = BTreeSet::new();
        let mut occupied_positions = BTreeSet::new();
        for group in &mut vault.encounter_groups {
            validate_id(&group.id)?;
            group.member_positions.sort();
            group.entries.sort_by(|left, right| {
                left.actor_kind_id
                    .cmp(&right.actor_kind_id)
                    .then(left.min_depth.cmp(&right.min_depth))
                    .then(left.max_depth.cmp(&right.max_depth))
            });
            if !section_ids.insert(group.id.clone())
                || group.member_positions.is_empty()
                || group.member_positions.len() > 16
                || group.entries.is_empty()
                || group.entries.len() > 64
            {
                return Err(ContentError::InvalidVault(vault.id.clone()));
            }
            let mut entry_ids = BTreeSet::new();
            for entry in &group.entries {
                require_actor_role(
                    actor_roles,
                    &entry.actor_kind_id,
                    ActorRole::Monster,
                    &vault.id,
                )?;
                if entry.weight == 0
                    || entry.min_depth == 0
                    || entry.min_depth > entry.max_depth
                    || entry.max_depth > 1_000
                    || actor_levels
                        .get(&entry.actor_kind_id)
                        .is_none_or(|level| *level > u32::from(entry.max_depth))
                    || !entry_ids.insert(entry.actor_kind_id.clone())
                {
                    return Err(ContentError::InvalidVault(vault.id.clone()));
                }
            }
            for position in &group.member_positions {
                let terrain_id = terrain_by_position
                    .get(position)
                    .unwrap_or(&vault.base_terrain_id);
                if position.x >= vault.width
                    || position.y >= vault.height
                    || terrain_walkability.get(terrain_id) != Some(&true)
                    || !occupied_positions.insert(*position)
                {
                    return Err(ContentError::InvalidVault(vault.id.clone()));
                }
            }
        }
        for spawn in &vault.loot_spawns {
            validate_id(&spawn.id)?;
            require_reference(&loot_table_ids, &spawn.loot_table_id, &vault.id)?;
            let terrain_id = terrain_by_position
                .get(&spawn.position)
                .unwrap_or(&vault.base_terrain_id);
            if !section_ids.insert(spawn.id.clone())
                || spawn.position.x >= vault.width
                || spawn.position.y >= vault.height
                || terrain_walkability.get(terrain_id) != Some(&true)
                || !occupied_positions.insert(spawn.position)
            {
                return Err(ContentError::InvalidVault(vault.id.clone()));
            }
        }
        insert_definition_id(all_ids, &vault.id)?;
        vaults_by_id.insert(vault.id.clone(), vault.clone());
    }

    let mut theme_tables_by_id = BTreeMap::new();
    for table in definitions.theme_tables.iter_mut() {
        require_schema(&table.schema, THEME_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "theme-table")?;
        if table.entries.is_empty() || table.entries.len() > 64 {
            return Err(ContentError::InvalidThemeTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.min_depth
                .cmp(&right.min_depth)
                .then(left.max_depth.cmp(&right.max_depth))
                .then(left.theme_id.cmp(&right.theme_id))
                .then(left.floor_terrain_id.cmp(&right.floor_terrain_id))
        });
        let mut entry_keys = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &mut table.entries {
            validate_definition_id(&entry.theme_id, "theme")?;
            require_reference(terrain_ids, &entry.floor_terrain_id, &table.id)?;
            entry.vault_candidates.sort_by(|left, right| {
                left.vault_id
                    .cmp(&right.vault_id)
                    .then(left.min_depth.cmp(&right.min_depth))
                    .then(left.max_depth.cmp(&right.max_depth))
            });
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || terrain_walkability.get(&entry.floor_terrain_id) != Some(&true)
                || entry.vault_candidates.len() > 64
                || !entry_keys.insert((entry.theme_id.clone(), entry.min_depth, entry.max_depth))
            {
                return Err(ContentError::InvalidThemeTable(table.id.clone()));
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidThemeTable(table.id.clone()))?;
            let mut vault_ids = BTreeSet::new();
            let mut vault_weight = 0_u64;
            for candidate in &entry.vault_candidates {
                let Some(vault) = vaults_by_id.get(&candidate.vault_id) else {
                    return Err(ContentError::DanglingReference {
                        owner: table.id.clone(),
                        target: candidate.vault_id.clone(),
                    });
                };
                if candidate.weight == 0
                    || candidate.min_depth < entry.min_depth
                    || candidate.min_depth > candidate.max_depth
                    || candidate.max_depth > entry.max_depth
                    || vault.theme_id != entry.theme_id
                    || !vault_ids.insert(candidate.vault_id.clone())
                {
                    return Err(ContentError::InvalidThemeTable(table.id.clone()));
                }
                vault_weight = vault_weight
                    .checked_add(u64::from(candidate.weight))
                    .ok_or_else(|| ContentError::InvalidThemeTable(table.id.clone()))?;
            }
            if !entry.vault_candidates.is_empty() && vault_weight == 0 {
                return Err(ContentError::InvalidThemeTable(table.id.clone()));
            }
        }
        if total_weight == 0 {
            return Err(ContentError::InvalidThemeTable(table.id.clone()));
        }
        insert_definition_id(all_ids, &table.id)?;
        theme_tables_by_id.insert(table.id.clone(), table.clone());
    }

    let mut region_tables_by_id = BTreeMap::new();
    for table in definitions.region_tables.iter_mut() {
        require_schema(&table.schema, REGION_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "region-table")?;
        if table.entries.len() < 2 || table.entries.len() > 32 {
            return Err(ContentError::InvalidRegionTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.region_id
                .cmp(&right.region_id)
                .then(left.min_depth.cmp(&right.min_depth))
                .then(left.max_depth.cmp(&right.max_depth))
        });
        let mut region_ids = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &table.entries {
            validate_definition_id(&entry.region_id, "region")?;
            let Some(theme_table) = theme_tables_by_id.get(&entry.theme_table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: entry.theme_table_id.clone(),
                });
            };
            if !encounter_tables_by_id.contains_key(&entry.encounter_table_id) {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: entry.encounter_table_id.clone(),
                });
            }
            require_reference(&loot_table_ids, &entry.loot_table_id, &table.id)?;
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || !region_ids.insert(entry.region_id.clone())
                || !theme_table.entries.iter().any(|theme| {
                    theme.theme_id == entry.theme_id
                        && theme.min_depth <= entry.min_depth
                        && entry.max_depth <= theme.max_depth
                })
            {
                return Err(ContentError::InvalidRegionTable(table.id.clone()));
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidRegionTable(table.id.clone()))?;
        }
        if total_weight == 0 {
            return Err(ContentError::InvalidRegionTable(table.id.clone()));
        }
        insert_definition_id(all_ids, &table.id)?;
        region_tables_by_id.insert(table.id.clone(), table.clone());
    }

    let mut terrain_feature_tables_by_id = BTreeMap::new();
    for table in definitions.terrain_feature_tables.iter_mut() {
        require_schema(&table.schema, TERRAIN_FEATURE_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "terrain-feature-table")?;
        if !(1..=8).contains(&table.rolls) || table.entries.is_empty() || table.entries.len() > 64 {
            return Err(ContentError::InvalidTerrainFeatureTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.min_depth
                .cmp(&right.min_depth)
                .then(left.max_depth.cmp(&right.max_depth))
                .then(left.placement.cmp(&right.placement))
                .then(left.terrain_id.cmp(&right.terrain_id))
        });
        let mut entry_keys = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &table.entries {
            require_reference(terrain_ids, &entry.terrain_id, &table.id)?;
            let terrain = terrain
                .iter()
                .find(|terrain| terrain.id == entry.terrain_id)
                .expect("validated terrain feature must remain available");
            let placement_matches_terrain = match entry.placement {
                TerrainFeaturePlacement::Room => {
                    terrain.trap.is_some() || terrain.dig_to_terrain_id.is_some()
                }
                TerrainFeaturePlacement::Corridor => terrain.open_to_terrain_id.is_some(),
            };
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || !placement_matches_terrain
                || !entry_keys.insert((
                    entry.terrain_id.clone(),
                    entry.placement,
                    entry.min_depth,
                    entry.max_depth,
                ))
            {
                return Err(ContentError::InvalidTerrainFeatureTable(table.id.clone()));
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidTerrainFeatureTable(table.id.clone()))?;
        }
        if total_weight == 0 {
            return Err(ContentError::InvalidTerrainFeatureTable(table.id.clone()));
        }
        insert_definition_id(all_ids, &table.id)?;
        terrain_feature_tables_by_id.insert(table.id.clone(), table.clone());
    }
    Ok(TableValidationOutputs {
        loot_table_ids,
        loot_tables_by_id,
        encounter_tables_by_id,
        vaults_by_id,
        theme_tables_by_id,
        region_tables_by_id,
        terrain_feature_tables_by_id,
    })
}

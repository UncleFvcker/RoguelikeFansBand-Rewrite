// Adapted from RFB master spells3.c/artifact.c/cmd6.c at
// a0d92b6378d148c5262cc236b8fa6ed2ca06a54c; upstream terms are preserved in NOTICE.
use super::*;

impl Game {
    pub(in crate::game) fn artifact_creation_plan(
        &self,
        source_item_id: &str,
        target: &TargetSelection,
    ) -> Option<ItemUsePlan> {
        let TargetSelection::ArtifactCreationItem {
            item_id,
            quantity,
            name,
        } = target
        else {
            return None;
        };
        if source_item_id == item_id
            || name
                .as_ref()
                .is_some_and(|name| name.len() >= 80 || name.chars().any(char::is_control))
        {
            return None;
        }
        let item = self.items.iter().find(|item| {
            item.id == *item_id && item.quantity > 0 && item.quantity == *quantity
                && (matches!(item.location, ItemLocation::Inventory | ItemLocation::Equipped { .. })
                    || matches!(item.location, ItemLocation::Ground(position) if position == self.player.position))
        })?;
        let definition = self.content.item(&item.kind_id)?;
        let base = definition.rfb_base_kind?;
        let eligible = matches!(base.tval, 16..=23 | 30..=38 | 40 | 45)
            || (base.tval == 39 && base.sval >= 2)
            || (self.player_is_snotling() && definition.tags.iter().any(|tag| tag == "mushroom"));
        if !eligible
            || matches!((base.tval, base.sval), (23, 32 | 34))
            || (definition.rfb_value.is_none()
                && !definition.tags.iter().any(|tag| tag == "mushroom"))
            || (self.item_identification(item) == ItemIdentificationDto::Identified
                && (item.is_artifact(&self.content) || !item.affix_ids.is_empty()))
        {
            return None;
        }
        Some(ItemUsePlan::ArtifactCreation {
            item_id: item_id.clone(),
            name: name.clone(),
        })
    }

    /// False is a source-level failed attempt: retain the scroll and refund its turn.
    pub(super) fn resolve_artifact_creation(
        &mut self,
        source_kind_id: &str,
        target_item_id: &str,
        name: Option<&str>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<bool, CoreError> {
        let index = self
            .items
            .iter()
            .position(|item| item.id == target_item_id)
            .expect("preflighted artifact target remains available");
        let original = self.items[index].clone();
        let definition = self
            .content
            .item(&original.kind_id)
            .expect("preflighted artifact target has a definition");
        let no_remove = original
            .intrinsic_properties
            .rfb_flags
            .contains("NO_REMOVE")
            || definition
                .rfb_value
                .as_ref()
                .is_some_and(|raw| raw.flags.contains("NO_REMOVE"));
        let mushroom = definition.tags.iter().any(|tag| tag == "mushroom");
        self.mark_item_aware(source_kind_id);
        if original.is_artifact(&self.content) || !original.affix_ids.is_empty() || no_remove {
            events.push(DomainEvent::ItemArtifactCreation {
                target_item_id: original.id,
                target_kind_id: original.kind_id,
                succeeded: false,
                destroyed_quantity: 0,
            });
            return Ok(false);
        }
        let (mut created, succeeded) = if mushroom {
            let mut item = original.clone();
            self.random_artifact_names.insert(String::new());
            item.artifact_name = Some(super::super::random_artifact::intern_name(
                &mut self.random_artifact_names,
                "(永恒蘑菇)".to_owned(),
            ));
            item.quantity = 1;
            (item, true)
        } else {
            let class_id = self
                .build
                .as_ref()
                .map_or("", |build| build.class_id.as_str());
            let depth = self.floor_depth(&self.current_floor_id);
            super::super::random_artifact::materialize_scroll(
                &self.content,
                &mut self.rng,
                &original,
                super::super::random_artifact::Creation {
                    level: i32::from(depth),
                    class_id,
                    requested_name: name,
                    ..Default::default()
                },
                &mut self.random_artifact_names,
            )
            .ok_or_else(|| {
                CoreError::Invariant("eligible artifact target could not materialize".into())
            })?
        };
        if succeeded {
            created.origin_kind = Some(ItemOriginKindDto::ArtifactCreation);
        }
        if !super::super::validation::item_creation_state_is_valid(
            &created,
            self.content.item(&created.kind_id).unwrap(),
        ) {
            return Err(CoreError::Invariant(
                "artifact creation produced invalid item state".into(),
            ));
        }
        if let ItemLocation::Ground(position) = created.location {
            changed.insert(position);
        }
        self.items[index] = created;
        if !mushroom {
            self.identify_item_instance(target_item_id, ItemIdentificationRequest::new(true));
            self.add_virtue(VirtueKindDto::Individualism, 2);
            self.add_virtue(VirtueKindDto::Enchantment, 5);
        }
        if succeeded {
            self.add_virtue(VirtueKindDto::Enchantment, 1);
        }
        events.push(DomainEvent::ItemArtifactCreation {
            target_item_id: original.id,
            target_kind_id: original.kind_id,
            succeeded,
            destroyed_quantity: original.quantity - 1,
        });
        Ok(succeeded)
    }
}

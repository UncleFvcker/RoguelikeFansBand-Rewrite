// SPDX-License-Identifier: MPL-2.0
//! Current-floor inquiry, adapted from RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c,
//! dungeon.c:get_dungeon_feeling. Queries never mutate knowledge, time or RNG.
// Copyright (c) 1997 Ben Harrison, James E. Wilson, Robert A. Koeneke.
// This software may be copied and distributed for educational, research,
// and not for profit purposes provided that this copyright and statement
// are included in all such copies. Other copyrights may also apply.
use super::Game;
use crate::state::ItemLocation;
use rfb_protocol::{ItemIdentificationDto, ResearchMonsterDto};

impl Game {
    pub(super) fn monster_recall_dtos(&self) -> Vec<ResearchMonsterDto> {
        // Use projected identities: telepathy and disguises must not reveal true kinds.
        let visible = if self.map_scale == rfb_protocol::MapScaleDto::World
            || self.player_has_status_kind(crate::effect::STATUS_HALLUCINATION)
        {
            Vec::new()
        } else {
            self.entities_dto()
        };
        self.research_monster_dtos()
            .into_iter()
            .filter(|monster| {
                self.discovery_knows_monster(&monster.kind_id)
                    || visible
                        .iter()
                        .any(|entity| entity.kind_id == monster.kind_id)
            })
            .collect()
    }

    pub(super) fn floor_feeling_message_key(&self) -> String {
        if self.map_scale == rfb_protocol::MapScaleDto::World {
            return "floor-feeling-wilderness".into();
        }
        if self.current_floor_task_id().is_some() {
            return "floor-feeling-quest".into();
        }
        let depth = i64::from(self.floor_depth(&self.current_floor_id));
        if depth == 0 {
            return if self.current_town_dto().is_some() {
                "floor-feeling-town"
            } else {
                "floor-feeling-wilderness"
            }
            .into();
        }
        let boost = |delta: i64| delta * delta + 50 * delta;
        let mut rating = 0;
        let mut incomplete_value = false;
        for entity in &self.entities {
            if entity.hp <= 0 || self.actor_is_player_aligned(entity) {
                continue;
            }
            let kind = self
                .content
                .actor(&entity.kind_id)
                .expect("actor kind validated");
            let mut delta = if kind.tags.iter().any(|tag| tag == "unique") {
                (i64::from(kind.level) + 10 - depth).max(0) * 20
            } else {
                (i64::from(kind.level) - depth).max(0) * 10
            };
            let crowd = self
                .entities
                .iter()
                .filter(|other| {
                    other.hp > 0
                        && other.id != entity.id
                        && (other.position.x - entity.position.x)
                            .abs()
                            .max((other.position.y - entity.position.y).abs())
                            <= 1
                })
                .count();
            let threshold = if kind
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.friends.is_some())
            {
                5
            } else {
                2
            };
            if crowd >= threshold {
                delta += 1;
            }
            rating += boost(delta);
        }
        for item in &self.items {
            if !matches!(
                item.location,
                ItemLocation::Ground(_) | ItemLocation::CarriedBy { .. }
            ) || self.item_identification(item) != ItemIdentificationDto::Unexamined
                || self.item_feeling(item).is_some()
            {
                continue;
            }
            if item.is_artifact(&self.content) {
                return "floor-feeling-1".into();
            }
            let kind = self
                .content
                .item(&item.kind_id)
                .expect("item kind validated");
            let mut delta = (i64::from(kind.generation_level) - depth).max(0) * 10;
            if !item.affix_ids.is_empty() || kind.rfb_base_kind.is_some_and(|base| base.tval == 38)
            {
                delta += 100;
                if let Some(value) = super::item_value::obj_value_real(&self.content, item) {
                    delta += [10_000, 50_000, 100_000]
                        .iter()
                        .filter(|threshold| value > **threshold)
                        .count() as i64
                        * 100;
                } else {
                    // The shared COST_REAL adapter explicitly excludes unsupported properties.
                    // Keep checking for special artifacts, but do not fabricate a value band.
                    incomplete_value = true;
                }
            }
            rating += boost(delta);
        }
        if incomplete_value {
            return "floor-feeling-incomplete".into();
        }
        let feeling = [1000, 800, 600, 400, 300, 200, 100, 0]
            .iter()
            .position(|threshold| rating > boost(*threshold))
            .map_or(10, |index| index + 2);
        format!("floor-feeling-{feeling}")
    }
}

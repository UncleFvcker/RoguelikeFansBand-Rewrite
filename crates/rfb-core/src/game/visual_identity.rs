// SPDX-License-Identifier: MPL-2.0
//! Knowledge-gated visual identities. Browsing and rendering never discover content.
use super::Game;
use crate::state::ItemInstance;
use rfb_protocol::{
    EditableVisualDto, ItemIdentificationDto, ItemKnowledgeDto, MapScaleDto, Position,
    ResearchMonsterDto, VisualCategoryDto,
};
use std::collections::BTreeMap;

impl Game {
    pub(super) fn item_visual(&self, item: &ItemInstance) -> EditableVisualDto {
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("item definition exists");
        let hidden_artifact = definition.tags.iter().any(|tag| tag == "artifact")
            && self.item_identification(item) == ItemIdentificationDto::Unexamined;
        if hidden_artifact && let Some(generation) = &definition.artifact_generation {
            let base = self
                .content
                .item(&generation.base_item_kind_id)
                .expect("artifact base validated");
            let name_key = self.item_display_name_key(&base.id);
            let id = if self.item_knowledge_dto(&base.id) == ItemKnowledgeDto::Aware {
                base.id.clone()
            } else {
                format!("core.appearance.{name_key}")
            };
            return EditableVisualDto {
                prf: None,
                id,
                name_key,
                glyph: base.glyph.clone(),
                category: VisualCategoryDto::Item,
            };
        }
        let (id, name_key) = if hidden_artifact {
            // Hand-authored artifacts without a declared base expose only their public symbol.
            (
                format!(
                    "core.appearance.symbol-{:x}",
                    definition.glyph.chars().next().expect("validated glyph") as u32
                ),
                "visual-unidentified-item".to_owned(),
            )
        } else if self.item_knowledge_dto(&item.kind_id) != ItemKnowledgeDto::Aware {
            let name = self.item_display_name_key(&item.kind_id);
            (format!("core.appearance.{name}"), name)
        } else {
            (item.kind_id.clone(), definition.name_key.clone())
        };
        EditableVisualDto {
            prf: None,
            id,
            name_key,
            glyph: definition.glyph.clone(),
            category: VisualCategoryDto::Item,
        }
    }

    pub(super) fn visual_catalog(&self, monsters: &[ResearchMonsterDto]) -> Vec<EditableVisualDto> {
        let mut entries = BTreeMap::new();
        for id in &self.discovery.objects {
            let item = self.content.item(id).expect("discovery item exists");
            entries.insert(
                id.clone(),
                EditableVisualDto {
                    prf: None,
                    id: id.clone(),
                    glyph: item.glyph.clone(),
                    name_key: item.name_key.clone(),
                    category: VisualCategoryDto::Item,
                },
            );
        }
        for item in self
            .items
            .iter()
            .filter(|item| self.item_is_discovered(&item.id))
        {
            let visual = self.item_visual(item);
            entries.insert(visual.id.clone(), visual);
        }
        for monster in monsters {
            entries.insert(
                monster.kind_id.clone(),
                EditableVisualDto {
                    prf: None,
                    id: monster.kind_id.clone(),
                    glyph: monster.glyph.clone(),
                    name_key: monster.name_key.clone(),
                    category: VisualCategoryDto::Monster,
                },
            );
        }
        let (width, height) = self.projected_dimensions();
        for y in 0..height {
            for x in 0..width {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if self.map_scale == MapScaleDto::World {
                    let cell = self.wilderness_cell_dto(position);
                    let glyph = super::snapshot::WILDERNESS_VISUALS
                        .iter()
                        .find(|(id, _)| *id == cell.terrain_id)
                        .expect("world visual exists")
                        .1;
                    entries
                        .entry(cell.terrain_id.clone())
                        .or_insert(EditableVisualDto {
                            prf: None,
                            id: cell.terrain_id,
                            glyph: glyph.into(),
                            name_key: "visual-world-terrain".into(),
                            category: VisualCategoryDto::Terrain,
                        });
                } else if self
                    .index(position)
                    .is_some_and(|index| self.explored[index])
                {
                    let id = self.known_terrain_at(position);
                    let terrain = self.content.terrain(id).expect("known terrain exists");
                    entries.insert(
                        id.to_owned(),
                        EditableVisualDto {
                            prf: None,
                            id: id.to_owned(),
                            glyph: terrain.glyph.clone(),
                            name_key: terrain.name_key.clone(),
                            category: VisualCategoryDto::Terrain,
                        },
                    );
                }
            }
        }
        let player = self
            .content
            .actor(&self.player.kind_id)
            .expect("player definition exists");
        entries.insert(
            player.id.clone(),
            EditableVisualDto {
                prf: None,
                id: player.id.clone(),
                glyph: player.glyph.clone(),
                name_key: "visual-player".into(),
                category: VisualCategoryDto::Marker,
            },
        );
        for pile in self.gold_piles.iter().filter(|pile| pile.discovered) {
            let id = super::gold::gold_visual_id(pile.appearance).to_owned();
            entries.insert(
                id.clone(),
                EditableVisualDto {
                    prf: None,
                    id,
                    glyph: "$".into(),
                    name_key: "visual-gold".into(),
                    category: VisualCategoryDto::Marker,
                },
            );
        }
        // Resolve numeric sources only for identities already admitted above. In particular,
        // appearances never gain a base-kind identity through an imported preference.
        let mut kinds = BTreeMap::new();
        for item in self.content.item_definitions() {
            if let Some(kind) = item.rfb_base_kind {
                *kinds.entry((kind.tval, kind.sval)).or_insert(0_u32) += 1;
            }
        }
        for visual in entries.values_mut() {
            visual.prf = match visual.category {
                VisualCategoryDto::Monster => self
                    .content
                    .actor(&visual.id)
                    .and_then(|actor| actor.allocation.as_ref())
                    .map(|allocation| format!("R:{}", allocation.legacy_index)),
                VisualCategoryDto::Item => self.content.item(&visual.id).and_then(|item| {
                    let base = item
                        .artifact_generation
                        .as_ref()
                        .and_then(|generation| self.content.item(&generation.base_item_kind_id))
                        .unwrap_or(item);
                    let kind = base.rfb_base_kind?;
                    (kinds.get(&(kind.tval, kind.sval)) == Some(&1))
                        .then(|| format!("K:{}:{}", kind.tval, kind.sval))
                }),
                _ => None,
            };
        }
        entries.into_values().collect()
    }
}

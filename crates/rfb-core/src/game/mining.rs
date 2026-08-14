use std::collections::BTreeSet;

use rfb_content::{TerrainDefinition, TerrainVeinYield};
use rfb_protocol::{MaterialDto, MiningProficiencyDto, Position};

use crate::{state::ItemLocation, stats::CharacterProgress};

use super::{
    DomainEvent, Game, GeneratedItemDraft, ItemGenerationMode, LootContext, LootSource,
    terrain::TerrainChangeSource, weapon_proficiency::proficiency_rank,
};

pub(super) const MINING_PROFICIENCY_MAXIMUM: u16 = 8_000;

pub(super) struct MaterialDefinition {
    pub(super) id: &'static str,
    pub(super) name_key: &'static str,
}

pub(super) const MATERIAL_DEFINITIONS: [MaterialDefinition; 10] = [
    MaterialDefinition {
        id: "rfb.material.iron-ore",
        name_key: "material-iron-ore",
    },
    MaterialDefinition {
        id: "rfb.material.silver-ore",
        name_key: "material-silver-ore",
    },
    MaterialDefinition {
        id: "rfb.material.mithril-dust",
        name_key: "material-mithril-dust",
    },
    MaterialDefinition {
        id: "rfb.material.crystal-shard",
        name_key: "material-crystal-shard",
    },
    MaterialDefinition {
        id: "rfb.material.herb",
        name_key: "material-herb",
    },
    MaterialDefinition {
        id: "rfb.material.beast-meat",
        name_key: "material-beast-meat",
    },
    MaterialDefinition {
        id: "rfb.material.dragon-scale",
        name_key: "material-dragon-scale",
    },
    MaterialDefinition {
        id: "rfb.material.demon-ichor",
        name_key: "material-demon-ichor",
    },
    MaterialDefinition {
        id: "rfb.material.arcane-essence",
        name_key: "material-arcane-essence",
    },
    MaterialDefinition {
        id: "rfb.material.rare-catalyst",
        name_key: "material-rare-catalyst",
    },
];

pub(super) fn mining_progress_is_valid(progress: &CharacterProgress) -> bool {
    progress.mining_proficiency <= MINING_PROFICIENCY_MAXIMUM
        && progress.materials.iter().all(|(id, quantity)| {
            *quantity > 0
                && MATERIAL_DEFINITIONS
                    .iter()
                    .any(|material| material.id == id)
        })
}

impl Game {
    pub(super) fn train_mining_proficiency(
        &mut self,
        vein_yield: TerrainVeinYield,
        power: u16,
    ) -> bool {
        let depth = self.floor_depth(&self.current_floor_id);
        let gain = match vein_yield {
            TerrainVeinYield::Ordinary => 8 + power / 2 + depth / 8,
            TerrainVeinYield::Treasure => 50 + power + depth / 2,
        };
        let previous = self.progress.mining_proficiency;
        self.progress.mining_proficiency = previous
            .saturating_add(gain)
            .min(MINING_PROFICIENCY_MAXIMUM);
        proficiency_rank(previous) != proficiency_rank(self.progress.mining_proficiency)
    }

    pub(super) fn player_mining_proficiency(&self) -> MiningProficiencyDto {
        let current = self.progress.mining_proficiency;
        MiningProficiencyDto {
            digging_power: self.player_derived_stats().dig_skill.value,
            rank: proficiency_rank(current),
            current,
            maximum: MINING_PROFICIENCY_MAXIMUM,
        }
    }

    pub(super) fn player_materials(&self) -> Vec<MaterialDto> {
        MATERIAL_DEFINITIONS
            .iter()
            .map(|material| MaterialDto {
                material_id: material.id.to_owned(),
                name_key: material.name_key.to_owned(),
                quantity: self
                    .progress
                    .materials
                    .get(material.id)
                    .copied()
                    .unwrap_or(0),
            })
            .collect()
    }

    fn add_material(&mut self, material_id: &str, quantity: u32) {
        self.progress
            .materials
            .entry(material_id.to_owned())
            .and_modify(|current| *current = current.saturating_add(quantity))
            .or_insert(quantity);
    }

    fn grant_mining_materials(&mut self, vein_yield: TerrainVeinYield) {
        let depth = self.floor_depth(&self.current_floor_id);
        let mining = self.progress.mining_proficiency;
        let amount = 1_u32
            .saturating_add(u32::from(depth / 25))
            .saturating_add(u32::from(mining / 1_600));
        match vein_yield {
            TerrainVeinYield::Treasure => {
                self.add_material("rfb.material.iron-ore", amount);
                if depth >= 20 && self.rng.bounded(3) == 0 {
                    self.add_material(
                        "rfb.material.silver-ore",
                        1_u32.saturating_add(u32::from(depth / 50)),
                    );
                }
                if depth >= 40
                    && self.rng.bounded(100) < u64::from(12_u16.saturating_add(mining / 500))
                {
                    self.add_material("rfb.material.mithril-dust", 1);
                }
            }
            TerrainVeinYield::Ordinary => {
                if self.rng.bounded(3) != 0 {
                    self.add_material("rfb.material.iron-ore", (amount / 2).max(1));
                }
                if depth >= 30 && self.rng.bounded(6) == 0 {
                    self.add_material("rfb.material.crystal-shard", 1);
                }
            }
        }
    }

    fn mining_object_level(&self) -> u16 {
        let depth = self.floor_depth(&self.current_floor_id);
        depth
            .saturating_add((self.progress.mining_proficiency / 500).min(20))
            .saturating_add(depth / 10)
            .clamp(1, 100)
    }

    fn current_floor_loot_table_id(&self) -> Option<String> {
        self.content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .and_then(|floor| floor.loot_table_id.clone())
    }

    fn mining_item_generation_mode(&mut self) -> ItemGenerationMode {
        let roll = u32::try_from(self.rng.bounded(100)).expect("d100 roll must fit u32");
        Self::mining_item_generation_mode_for_roll(self.progress.mining_proficiency, roll)
    }

    fn mining_item_generation_mode_for_roll(
        mining_proficiency: u16,
        mut roll: u32,
    ) -> ItemGenerationMode {
        let mining = u32::from(mining_proficiency.min(MINING_PROFICIENCY_MAXIMUM));
        let scaled = |maximum: u32| (maximum.saturating_mul(mining) + 4_000) / 8_000;
        let artifact = scaled(5);
        let great = scaled(20);
        let good = scaled(40);
        if roll < artifact {
            return ItemGenerationMode::Artifact;
        }
        roll -= artifact;
        if roll < great {
            return ItemGenerationMode::Great;
        }
        roll -= great;
        if roll < good {
            return ItemGenerationMode::Good;
        }
        ItemGenerationMode::Ordinary
    }

    fn generate_mining_item_draft(
        &mut self,
        context: &LootContext,
        mode: ItemGenerationMode,
    ) -> Option<GeneratedItemDraft> {
        if mode != ItemGenerationMode::Artifact {
            return self.generate_one_loot_draft(context, mode);
        }
        for _ in 0..20 {
            let Some(draft) = self.generate_one_loot_draft(context, mode) else {
                continue;
            };
            if self
                .content
                .item(&draft.kind_id)
                .is_some_and(|item| item.artifact_generation.is_some())
            {
                return Some(draft);
            }
        }
        self.generate_one_loot_draft(context, ItemGenerationMode::Great)
    }

    fn place_rubble_item(
        &mut self,
        position: Position,
        generation_level: u16,
        mode: ItemGenerationMode,
    ) -> bool {
        let Some(table_id) = self.current_floor_loot_table_id() else {
            return false;
        };
        let context = LootContext {
            table_id,
            floor_id: self.current_floor_id.clone(),
            depth: generation_level,
            source: LootSource::Rubble { position },
        };
        self.next_item_instance_serial
            .checked_add(1)
            .expect("validated rubble loot must remain generatable");
        let Some(draft) = self.generate_mining_item_draft(&context, mode) else {
            return false;
        };
        let item = self
            .commit_generated_item_draft(draft, ItemLocation::Ground(position))
            .expect("validated rubble loot must remain generatable");
        self.items.push(item);
        true
    }

    fn current_dungeon_minimum_depth(&self) -> Option<u16> {
        let world = self.content.world(&self.world_id)?;
        let dungeon_id = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == self.current_floor_id)?
            .dungeon_id
            .as_ref()?;
        world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_ref() == Some(dungeon_id))
            .map(|floor| floor.depth)
            .min()
    }

    pub(super) fn resolve_terrain_change_rewards(
        &mut self,
        source: &TerrainDefinition,
        target: &TerrainDefinition,
        position: Position,
        change_source: TerrainChangeSource,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        if change_source == TerrainChangeSource::Projectile {
            return false;
        }
        let source_yield = source
            .digging
            .as_ref()
            .and_then(|digging| digging.vein_yield);
        let target_yield = target
            .digging
            .as_ref()
            .and_then(|digging| digging.vein_yield);
        let mut proficiency_improved = false;
        if change_source == TerrainChangeSource::Dig
            && let Some(vein_yield) = source_yield
            && target_yield.is_none()
        {
            let power = source
                .digging
                .as_ref()
                .expect("mining vein must retain digging data")
                .power;
            proficiency_improved = self.train_mining_proficiency(vein_yield, power);
            self.grant_mining_materials(vein_yield);
        }

        let depth = self.floor_depth(&self.current_floor_id);
        let mining = self.progress.mining_proficiency;
        let mut found = false;
        if source_yield == Some(TerrainVeinYield::Treasure)
            && target_yield != Some(TerrainVeinYield::Treasure)
        {
            let pile = if change_source == TerrainChangeSource::Dig {
                self.generate_mining_gold_pile(position, depth, mining)
            } else {
                self.generate_gold_pile(position, depth.max(1), false)
            }
            .expect("validated terrain gold allocator must remain available");
            self.gold_piles.push(pile);
            found = true;

            if change_source == TerrainChangeSource::Dig {
                let chance = (3_u16
                    .saturating_add(depth / 15)
                    .saturating_add(mining / 1_000))
                .min(20);
                if self.rng.bounded(100) < u64::from(chance) {
                    let mode = self.mining_item_generation_mode();
                    found |= self.place_rubble_item(position, self.mining_object_level(), mode);
                }
            }
        }

        if source.tags.iter().any(|tag| tag == "rubble")
            && !target.tags.iter().any(|tag| tag == "rubble")
            && let Some(minimum_depth) = self.current_dungeon_minimum_depth()
            && depth > minimum_depth
        {
            let chance = 36_i32.saturating_sub(i32::from(depth)).clamp(1, 24);
            if self.rng.bounded(200) < u64::try_from(chance).expect("positive chance") {
                found |=
                    self.place_rubble_item(position, depth.max(1), ItemGenerationMode::Ordinary);
            }
        }

        if found && self.is_visible(position) {
            events.push(DomainEvent::TerrainFoundSomething);
        }
        if found {
            changed.insert(position);
        }
        proficiency_improved
    }
}

#[cfg(test)]
mod tests {
    use crate::rng::RfbRng;
    use rfb_protocol::ItemQualityDto;

    use super::*;

    fn artifact_context(game: &Game) -> LootContext {
        LootContext {
            table_id: "demo.loot-table.paladin".to_owned(),
            floor_id: "test.floor.depth-100".to_owned(),
            depth: 100,
            source: LootSource::Rubble {
                position: game.player.position,
            },
        }
    }

    fn formal_artifact(game: &Game, draft: &GeneratedItemDraft) -> bool {
        game.content
            .item(&draft.kind_id)
            .is_some_and(|item| item.artifact_generation.is_some())
    }

    #[test]
    fn original_mining_gain_formulas_use_feature_power_and_floor_depth() {
        let mut game = Game::new(42);
        game.current_floor_id = "demo.floor.warrens-depth-8".to_owned();

        assert!(!game.train_mining_proficiency(TerrainVeinYield::Ordinary, 20));
        assert_eq!(game.progress.mining_proficiency, 19);
        assert!(!game.train_mining_proficiency(TerrainVeinYield::Treasure, 20));
        assert_eq!(game.progress.mining_proficiency, 93);

        game.progress.mining_proficiency = 7_999;
        assert!(game.train_mining_proficiency(TerrainVeinYield::Treasure, 20));
        assert_eq!(game.progress.mining_proficiency, MINING_PROFICIENCY_MAXIMUM);
        assert!(!game.train_mining_proficiency(TerrainVeinYield::Treasure, 20));
    }

    #[test]
    fn mining_item_mode_uses_original_scaled_adjacent_thresholds() {
        for roll in [0, 1, 50, 99] {
            assert_eq!(
                Game::mining_item_generation_mode_for_roll(0, roll),
                ItemGenerationMode::Ordinary
            );
        }
        for (roll, expected) in [
            (0, ItemGenerationMode::Artifact),
            (4, ItemGenerationMode::Artifact),
            (5, ItemGenerationMode::Great),
            (24, ItemGenerationMode::Great),
            (25, ItemGenerationMode::Good),
            (64, ItemGenerationMode::Good),
            (65, ItemGenerationMode::Ordinary),
            (99, ItemGenerationMode::Ordinary),
        ] {
            assert_eq!(
                Game::mining_item_generation_mode_for_roll(8_000, roll),
                expected
            );
        }
    }

    #[test]
    fn mining_artifact_mode_accepts_a_twentieth_attempt_without_allocating_discarded_drafts() {
        let mut probe = Game::new(9);
        let context = artifact_context(&probe);
        let seed = (0..100_000)
            .find(|seed| {
                probe.rng = RfbRng::seeded(*seed);
                (1..=20).find(|_| {
                    probe
                        .generate_one_loot_draft(&context, ItemGenerationMode::Artifact)
                        .is_some_and(|draft| formal_artifact(&probe, &draft))
                }) == Some(20)
            })
            .expect("a seed with its first fixed artifact on attempt twenty should exist");

        let mut game = Game::new(9);
        let context = artifact_context(&game);
        game.rng = RfbRng::seeded(seed);
        let serial_before = game.next_item_instance_serial;
        let draft = game
            .generate_mining_item_draft(&context, ItemGenerationMode::Artifact)
            .expect("the twentieth attempt should produce a fixed artifact");
        assert!(formal_artifact(&game, &draft));
        assert_eq!(game.next_item_instance_serial, serial_before);
        let position = game.player.position;
        let item = game
            .commit_generated_item_draft(draft, ItemLocation::Ground(position))
            .expect("accepted mining artifact should commit once");
        assert_eq!(
            item.origin_kind,
            Some(rfb_protocol::ItemOriginKindDto::Rubble)
        );
        assert_eq!(item.quality, ItemQualityDto::Ordinary);
        assert_eq!(game.next_item_instance_serial, serial_before + 1);
        assert!(game.generated_artifact_ids.contains(&item.kind_id));
    }

    #[test]
    fn mining_artifact_mode_falls_back_once_to_great_when_all_artifacts_are_ineligible() {
        let seed = 17;
        let run = || {
            let mut game = Game::new(11);
            game.generated_artifact_ids.extend(
                game.content
                    .item_definitions()
                    .filter(|item| item.artifact_generation.is_some())
                    .map(|item| item.id.clone()),
            );
            game.rng = RfbRng::seeded(seed);
            let context = artifact_context(&game);
            let serial_before = game.next_item_instance_serial;
            let draft = game
                .generate_mining_item_draft(&context, ItemGenerationMode::Artifact)
                .expect("Great fallback should produce an item");
            assert!(!formal_artifact(&game, &draft));
            assert!(
                game.content
                    .item(&draft.kind_id)
                    .is_some_and(|item| !item.tags.iter().any(|tag| tag == "artifact"))
            );
            assert_eq!(draft.quality, ItemQualityDto::Exceptional);
            assert_eq!(game.next_item_instance_serial, serial_before);
            let position = game.player.position;
            let item = game
                .commit_generated_item_draft(draft, ItemLocation::Ground(position))
                .expect("Great fallback should commit once");
            assert_eq!(
                item.origin_kind,
                Some(rfb_protocol::ItemOriginKindDto::Rubble)
            );
            game.items.push(item);
            game
        };

        let first = run();
        let second = run();
        assert_eq!(first.rng, second.rng);
        assert_eq!(
            first.next_item_instance_serial,
            second.next_item_instance_serial
        );
        assert_eq!(first.state_hash(), second.state_hash());
    }
}

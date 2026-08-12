use rfb_content::TerrainVeinYield;
use rfb_protocol::{MaterialDto, MiningProficiencyDto};

use crate::stats::CharacterProgress;

use super::{Game, weapon_proficiency::proficiency_rank};

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

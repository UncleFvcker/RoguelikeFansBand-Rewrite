// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    AbilityBookDefinition, AbilityDefinition, ActorDefinition, AffixDefinition,
    CharacterBuildDefinition, ClassDefinition, CompiledArtifact, ContentError,
    EncounterTableDefinition, ItemDefinition, LootTableDefinition, MutationDefinition,
    PackDependency, PersonalityDefinition, RaceDefinition, RegionTableDefinition,
    ResourceDefinition, ShopDefinition, SkillDefinition, SkillKind, SkillSetDefinition,
    TerrainDefinition, TerrainFeatureTableDefinition, ThemeTableDefinition, TownDefinition,
    TownFacilityDefinition, VaultDefinition, WorldDefinition, decode_content,
};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompiledContentV1 {
    pub format: String,
    pub format_version: u16,
    pub pack_id: String,
    pub pack_version: String,
    pub title_key: String,
    pub dependencies: Vec<PackDependency>,
    pub load_after: Vec<String>,
    pub terrain: Vec<TerrainDefinition>,
    pub actors: Vec<ActorDefinition>,
    pub affixes: Vec<AffixDefinition>,
    pub items: Vec<ItemDefinition>,
    #[serde(default)]
    pub resources: Vec<ResourceDefinition>,
    #[serde(default)]
    pub abilities: Vec<AbilityDefinition>,
    #[serde(default)]
    pub ability_books: Vec<AbilityBookDefinition>,
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
    #[serde(default)]
    pub skill_sets: Vec<SkillSetDefinition>,
    #[serde(default)]
    pub races: Vec<RaceDefinition>,
    #[serde(default)]
    pub classes: Vec<ClassDefinition>,
    #[serde(default)]
    pub personalities: Vec<PersonalityDefinition>,
    #[serde(default)]
    pub builds: Vec<CharacterBuildDefinition>,
    #[serde(default)]
    pub mutations: Vec<MutationDefinition>,
    #[serde(default)]
    pub encounter_tables: Vec<EncounterTableDefinition>,
    #[serde(default)]
    pub loot_tables: Vec<LootTableDefinition>,
    #[serde(default)]
    pub theme_tables: Vec<ThemeTableDefinition>,
    #[serde(default)]
    pub region_tables: Vec<RegionTableDefinition>,
    #[serde(default)]
    pub terrain_feature_tables: Vec<TerrainFeatureTableDefinition>,
    #[serde(default)]
    pub vaults: Vec<VaultDefinition>,
    #[serde(default)]
    pub towns: Vec<TownDefinition>,
    #[serde(default)]
    pub town_facilities: Vec<TownFacilityDefinition>,
    #[serde(default)]
    pub shops: Vec<ShopDefinition>,
    pub worlds: Vec<WorldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCatalog {
    pack_id: String,
    pack_version: String,
    content_hash: String,
    terrain: BTreeMap<String, TerrainDefinition>,
    actors: BTreeMap<String, ActorDefinition>,
    affixes: BTreeMap<String, AffixDefinition>,
    items: BTreeMap<String, ItemDefinition>,
    resources: BTreeMap<String, ResourceDefinition>,
    abilities: BTreeMap<String, AbilityDefinition>,
    ability_books: BTreeMap<String, AbilityBookDefinition>,
    skills: BTreeMap<String, SkillDefinition>,
    skill_sets: BTreeMap<String, SkillSetDefinition>,
    races: BTreeMap<String, RaceDefinition>,
    classes: BTreeMap<String, ClassDefinition>,
    personalities: BTreeMap<String, PersonalityDefinition>,
    builds: BTreeMap<String, CharacterBuildDefinition>,
    mutations: BTreeMap<String, MutationDefinition>,
    encounter_tables: BTreeMap<String, EncounterTableDefinition>,
    loot_tables: BTreeMap<String, LootTableDefinition>,
    theme_tables: BTreeMap<String, ThemeTableDefinition>,
    region_tables: BTreeMap<String, RegionTableDefinition>,
    terrain_feature_tables: BTreeMap<String, TerrainFeatureTableDefinition>,
    vaults: BTreeMap<String, VaultDefinition>,
    towns: BTreeMap<String, TownDefinition>,
    town_facilities: BTreeMap<String, TownFacilityDefinition>,
    shops: BTreeMap<String, ShopDefinition>,
    worlds: BTreeMap<String, WorldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSummary {
    pub pack_id: String,
    pub pack_version: String,
    pub content_hash: String,
    pub terrain_count: usize,
    pub actor_count: usize,
    pub affix_count: usize,
    pub item_count: usize,
    pub resource_count: usize,
    pub ability_count: usize,
    pub ability_book_count: usize,
    pub skill_count: usize,
    pub skill_set_count: usize,
    pub race_count: usize,
    pub class_count: usize,
    pub personality_count: usize,
    pub build_count: usize,
    pub mutation_count: usize,
    pub encounter_table_count: usize,
    pub loot_table_count: usize,
    pub theme_table_count: usize,
    pub region_table_count: usize,
    pub terrain_feature_table_count: usize,
    pub vault_count: usize,
    pub town_count: usize,
    pub town_facility_count: usize,
    pub shop_count: usize,
    pub world_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentLockV1 {
    pub schema_version: u16,
    pub pack_id: String,
    pub pack_version: String,
    pub content_hash: String,
}

impl ContentCatalog {
    #[must_use]
    pub fn from_artifact(artifact: CompiledArtifact) -> Self {
        let CompiledArtifact {
            content,
            content_hash,
            ..
        } = artifact;
        Self {
            pack_id: content.pack_id,
            pack_version: content.pack_version,
            content_hash,
            terrain: content
                .terrain
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            actors: content
                .actors
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            affixes: content
                .affixes
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            items: content
                .items
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            resources: content
                .resources
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            abilities: content
                .abilities
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            ability_books: content
                .ability_books
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            skills: content
                .skills
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            skill_sets: content
                .skill_sets
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            races: content
                .races
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            classes: content
                .classes
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            personalities: content
                .personalities
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            builds: content
                .builds
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            mutations: content
                .mutations
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            encounter_tables: content
                .encounter_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            loot_tables: content
                .loot_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            theme_tables: content
                .theme_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            region_tables: content
                .region_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            terrain_feature_tables: content
                .terrain_feature_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            vaults: content
                .vaults
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            towns: content
                .towns
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            town_facilities: content
                .town_facilities
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            shops: content
                .shops
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            worlds: content
                .worlds
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ContentError> {
        Ok(Self::from_artifact(decode_content(bytes)?))
    }

    #[must_use]
    pub fn pack_id(&self) -> &str {
        &self.pack_id
    }

    #[must_use]
    pub fn pack_version(&self) -> &str {
        &self.pack_version
    }

    #[must_use]
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    #[must_use]
    pub fn terrain(&self, id: &str) -> Option<&TerrainDefinition> {
        self.terrain.get(id)
    }

    #[must_use]
    pub fn actor(&self, id: &str) -> Option<&ActorDefinition> {
        self.actors.get(id)
    }

    /// All actor definitions in stable id order (BTree iteration), so
    /// category filters enumerate candidates deterministically.
    pub fn actor_definitions(&self) -> impl Iterator<Item = &ActorDefinition> {
        self.actors.values()
    }

    #[must_use]
    pub fn item(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.get(id)
    }

    /// All item definitions in stable id order.
    pub fn item_definitions(&self) -> impl Iterator<Item = &ItemDefinition> {
        self.items.values()
    }

    #[must_use]
    pub fn affix(&self, id: &str) -> Option<&AffixDefinition> {
        self.affixes.get(id)
    }

    #[must_use]
    pub fn resource(&self, id: &str) -> Option<&ResourceDefinition> {
        self.resources.get(id)
    }

    #[must_use]
    pub fn ability(&self, id: &str) -> Option<&AbilityDefinition> {
        self.abilities.get(id)
    }

    pub fn abilities(&self) -> impl Iterator<Item = &AbilityDefinition> {
        self.abilities.values()
    }

    #[must_use]
    pub fn ability_book(&self, id: &str) -> Option<&AbilityBookDefinition> {
        self.ability_books.get(id)
    }

    #[must_use]
    pub fn skill(&self, id: &str) -> Option<&SkillDefinition> {
        self.skills.get(id)
    }

    #[must_use]
    pub fn skill_by_kind(&self, kind: SkillKind) -> Option<&SkillDefinition> {
        self.skills.values().find(|skill| skill.kind == kind)
    }

    #[must_use]
    pub fn skill_set(&self, id: &str) -> Option<&SkillSetDefinition> {
        self.skill_sets.get(id)
    }

    #[must_use]
    pub fn race(&self, id: &str) -> Option<&RaceDefinition> {
        self.races.get(id)
    }

    #[must_use]
    pub fn class(&self, id: &str) -> Option<&ClassDefinition> {
        self.classes.get(id)
    }

    #[must_use]
    pub fn personality(&self, id: &str) -> Option<&PersonalityDefinition> {
        self.personalities.get(id)
    }

    #[must_use]
    pub fn build(&self, id: &str) -> Option<&CharacterBuildDefinition> {
        self.builds.get(id)
    }

    pub fn builds(&self) -> impl Iterator<Item = &CharacterBuildDefinition> {
        self.builds.values()
    }

    #[must_use]
    pub fn mutation(&self, id: &str) -> Option<&MutationDefinition> {
        self.mutations.get(id)
    }

    pub fn mutations(&self) -> impl Iterator<Item = &MutationDefinition> {
        self.mutations.values()
    }

    #[must_use]
    pub fn loot_table(&self, id: &str) -> Option<&LootTableDefinition> {
        self.loot_tables.get(id)
    }

    #[must_use]
    pub fn encounter_table(&self, id: &str) -> Option<&EncounterTableDefinition> {
        self.encounter_tables.get(id)
    }

    #[must_use]
    pub fn theme_table(&self, id: &str) -> Option<&ThemeTableDefinition> {
        self.theme_tables.get(id)
    }

    #[must_use]
    pub fn region_table(&self, id: &str) -> Option<&RegionTableDefinition> {
        self.region_tables.get(id)
    }

    #[must_use]
    pub fn terrain_feature_table(&self, id: &str) -> Option<&TerrainFeatureTableDefinition> {
        self.terrain_feature_tables.get(id)
    }

    #[must_use]
    pub fn vault(&self, id: &str) -> Option<&VaultDefinition> {
        self.vaults.get(id)
    }

    #[must_use]
    pub fn town(&self, id: &str) -> Option<&TownDefinition> {
        self.towns.get(id)
    }

    #[must_use]
    pub fn town_facility(&self, id: &str) -> Option<&TownFacilityDefinition> {
        self.town_facilities.get(id)
    }

    #[must_use]
    pub fn shop(&self, id: &str) -> Option<&ShopDefinition> {
        self.shops.get(id)
    }

    #[must_use]
    pub fn world(&self, id: &str) -> Option<&WorldDefinition> {
        self.worlds.get(id)
    }

    #[must_use]
    pub fn visual_glyphs(&self) -> BTreeMap<String, String> {
        self.terrain
            .iter()
            .map(|(id, definition)| (id.clone(), definition.glyph.clone()))
            .chain(
                self.actors
                    .iter()
                    .map(|(id, definition)| (id.clone(), definition.glyph.clone())),
            )
            .chain(
                self.items
                    .iter()
                    .map(|(id, definition)| (id.clone(), definition.glyph.clone())),
            )
            .collect()
    }
}

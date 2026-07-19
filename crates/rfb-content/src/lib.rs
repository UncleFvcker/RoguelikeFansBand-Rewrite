// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

#[cfg(feature = "schemas")]
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const CONTENT_FORMAT: &str = "rfb-content";
pub const CONTENT_FORMAT_VERSION: u16 = 1;
pub const PACK_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/pack.schema.json";
pub const TERRAIN_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/terrain.schema.json";
pub const ACTOR_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/actor.schema.json";
pub const ITEM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/item.schema.json";
pub const WORLD_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/world.schema.json";

const fn default_actor_speed() -> u16 {
    110
}

const MAGIC: &[u8; 8] = b"RFBCONT\0";
const CONTAINER_VERSION: u16 = 1;
const FIXED_HEADER_LENGTH: usize = 8 + 2 + 2 + 8 + 32;
const MAX_SOURCE_FILE_LENGTH: usize = 1024 * 1024;
const MAX_SOURCE_TOTAL_LENGTH: usize = 16 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 2048;
const MAX_COMPILED_PAYLOAD_LENGTH: usize = 32 * 1024 * 1024;
const SUPPORTED_ROOTS: [&str; 4] = ["actors", "items", "terrain", "worlds"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub version: String,
    pub title_key: String,
    pub dependencies: Vec<PackDependency>,
    pub load_after: Vec<String>,
    pub content_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackDependency {
    pub id: String,
    pub version_requirement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub glyph: String,
    pub walkable: bool,
    pub blocks_sight: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorRole {
    Player,
    Monster,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorDamageType {
    #[default]
    Physical,
    Acid,
    Electricity,
    Fire,
    Cold,
    Poison,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeleeBlowDefinition {
    pub method_id: String,
    pub to_hit: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeleeRoutineDefinition {
    pub blows: Vec<MeleeBlowDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub role: ActorRole,
    pub name_key: String,
    pub description_key: String,
    pub glyph: String,
    pub level: u32,
    pub max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub speed: u16,
    pub attack: i32,
    pub defense: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
    #[serde(default)]
    pub melee_routine: Option<MeleeRoutineDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatModifiers {
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttackProfileDefinition {
    pub attacks: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectileProfileDefinition {
    pub range: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub glyph: String,
    pub max_stack: u32,
    #[serde(default)]
    pub equipment_slot: Option<String>,
    #[serde(default)]
    pub modifiers: StatModifiers,
    #[serde(default)]
    pub melee_profile: Option<AttackProfileDefinition>,
    #[serde(default)]
    pub projectile_profile: Option<ProjectileProfileDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentPosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainOverride {
    pub terrain_id: String,
    pub positions: Vec<ContentPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorSpawn {
    pub instance_id: String,
    pub kind_id: String,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemSpawn {
    pub instance_id: String,
    pub kind_id: String,
    pub position: ContentPosition,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub width: u16,
    pub height: u16,
    pub fill_terrain_id: String,
    pub border_terrain_id: String,
    pub terrain_overrides: Vec<TerrainOverride>,
    pub player: ActorSpawn,
    pub actors: Vec<ActorSpawn>,
    pub items: Vec<ItemSpawn>,
}

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
    pub items: Vec<ItemDefinition>,
    pub worlds: Vec<WorldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifact {
    pub content: CompiledContentV1,
    pub content_hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCatalog {
    pack_id: String,
    pack_version: String,
    content_hash: String,
    terrain: BTreeMap<String, TerrainDefinition>,
    actors: BTreeMap<String, ActorDefinition>,
    items: BTreeMap<String, ItemDefinition>,
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
    pub item_count: usize,
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

impl CompiledArtifact {
    #[must_use]
    pub fn summary(&self) -> ContentSummary {
        ContentSummary {
            pack_id: self.content.pack_id.clone(),
            pack_version: self.content.pack_version.clone(),
            content_hash: self.content_hash.clone(),
            terrain_count: self.content.terrain.len(),
            actor_count: self.content.actors.len(),
            item_count: self.content.items.len(),
            world_count: self.content.worlds.len(),
        }
    }
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
            items: content
                .items
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

    #[must_use]
    pub fn item(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.get(id)
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

pub fn compile_pack_dir(root: &Path) -> Result<CompiledArtifact, ContentError> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidPackRoot(root.to_path_buf()));
    }

    let mut budget = SourceBudget::default();
    let manifest: PackManifest = read_json(&root.join("pack.json"), &mut budget)?;
    validate_manifest(&manifest)?;

    let roots = manifest
        .content_roots
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let content = CompiledContentV1 {
        format: CONTENT_FORMAT.to_owned(),
        format_version: CONTENT_FORMAT_VERSION,
        pack_id: manifest.id,
        pack_version: manifest.version,
        title_key: manifest.title_key,
        dependencies: manifest.dependencies,
        load_after: manifest.load_after,
        terrain: load_root(root, "terrain", &roots, &mut budget)?,
        actors: load_root(root, "actors", &roots, &mut budget)?,
        items: load_root(root, "items", &roots, &mut budget)?,
        worlds: load_root(root, "worlds", &roots, &mut budget)?,
    };
    encode_content(content)
}

pub fn verify_pack_lock(root: &Path) -> Result<CompiledArtifact, ContentError> {
    let artifact = compile_pack_dir(root)?;
    let mut budget = SourceBudget::default();
    let content_lock: ContentLockV1 = read_json(&root.join("content.lock.json"), &mut budget)?;
    if content_lock.schema_version != 1
        || content_lock.pack_id != artifact.content.pack_id
        || content_lock.pack_version != artifact.content.pack_version
        || content_lock.content_hash != artifact.content_hash
    {
        return Err(ContentError::ContentLockMismatch);
    }
    Ok(artifact)
}

pub fn encode_content(mut content: CompiledContentV1) -> Result<CompiledArtifact, ContentError> {
    validate_and_normalize(&mut content)?;
    let payload = rmp_serde::to_vec_named(&content)?;
    if payload.len() > MAX_COMPILED_PAYLOAD_LENGTH {
        return Err(ContentError::CompiledPayloadTooLarge(payload.len()));
    }
    let content_hash = sha256(&payload);
    let payload_length = u64::try_from(payload.len()).map_err(|_| ContentError::LengthOverflow)?;
    let capacity = FIXED_HEADER_LENGTH
        .checked_add(payload.len())
        .ok_or(ContentError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&Sha256::digest(&payload));
    bytes.extend_from_slice(&payload);
    Ok(CompiledArtifact {
        content,
        content_hash,
        bytes,
    })
}

pub fn decode_content(bytes: &[u8]) -> Result<CompiledArtifact, ContentError> {
    if bytes.len() < FIXED_HEADER_LENGTH || &bytes[..8] != MAGIC {
        return Err(ContentError::InvalidContainer);
    }
    let version = read_u16(bytes, 8)?;
    if version != CONTAINER_VERSION {
        return Err(ContentError::UnsupportedContainerVersion(version));
    }
    let flags = read_u16(bytes, 10)?;
    if flags != 0 {
        return Err(ContentError::UnsupportedContainerFlags(flags));
    }
    let payload_length =
        usize::try_from(read_u64(bytes, 12)?).map_err(|_| ContentError::LengthOverflow)?;
    if payload_length > MAX_COMPILED_PAYLOAD_LENGTH {
        return Err(ContentError::CompiledPayloadTooLarge(payload_length));
    }
    let expected_length = FIXED_HEADER_LENGTH
        .checked_add(payload_length)
        .ok_or(ContentError::LengthOverflow)?;
    if bytes.len() != expected_length {
        return Err(ContentError::InvalidContainer);
    }
    let payload = &bytes[FIXED_HEADER_LENGTH..];
    let actual_checksum = Sha256::digest(payload);
    if bytes[20..52] != actual_checksum[..] {
        return Err(ContentError::ChecksumMismatch);
    }
    let content: CompiledContentV1 = rmp_serde::from_slice(payload)?;
    let mut normalized = content.clone();
    validate_and_normalize(&mut normalized)?;
    if normalized != content {
        return Err(ContentError::NonCanonicalCompiledContent);
    }
    Ok(CompiledArtifact {
        content,
        content_hash: sha256(payload),
        bytes: bytes.to_vec(),
    })
}

pub fn read_compiled_file(path: &Path) -> Result<CompiledArtifact, ContentError> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((FIXED_HEADER_LENGTH + MAX_COMPILED_PAYLOAD_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)?;
    decode_content(&bytes)
}

fn validate_manifest(manifest: &PackManifest) -> Result<(), ContentError> {
    require_schema(&manifest.schema, PACK_SCHEMA, "pack.json")?;
    require_format_version(manifest.format_version, "pack.json")?;
    validate_id(&manifest.id)?;
    validate_semver(&manifest.version)?;
    validate_message_key(&manifest.title_key)?;

    let mut roots = BTreeSet::new();
    for root in &manifest.content_roots {
        if !SUPPORTED_ROOTS.contains(&root.as_str()) {
            return Err(ContentError::UnsupportedContentRoot(root.clone()));
        }
        if !roots.insert(root.as_str()) {
            return Err(ContentError::DuplicateContentRoot(root.clone()));
        }
    }
    validate_pack_relations(&manifest.id, &manifest.dependencies, &manifest.load_after)
}

fn validate_and_normalize(content: &mut CompiledContentV1) -> Result<(), ContentError> {
    if content.format != CONTENT_FORMAT || content.format_version != CONTENT_FORMAT_VERSION {
        return Err(ContentError::InvalidCompiledMetadata);
    }
    validate_id(&content.pack_id)?;
    validate_semver(&content.pack_version)?;
    validate_message_key(&content.title_key)?;
    validate_pack_relations(&content.pack_id, &content.dependencies, &content.load_after)?;
    content
        .dependencies
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.load_after.sort();
    content
        .terrain
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.actors.sort_by(|left, right| left.id.cmp(&right.id));
    content.items.sort_by(|left, right| left.id.cmp(&right.id));
    content.worlds.sort_by(|left, right| left.id.cmp(&right.id));

    let mut all_ids = BTreeSet::new();
    let mut terrain_ids = BTreeSet::new();
    let mut terrain_walkability = BTreeMap::new();
    for terrain in &mut content.terrain {
        require_schema(&terrain.schema, TERRAIN_SCHEMA, &terrain.id)?;
        require_format_version(terrain.format_version, &terrain.id)?;
        validate_definition_id(&terrain.id, "terrain")?;
        validate_definition_text(&terrain.id, &terrain.name_key, &terrain.description_key)?;
        validate_glyph(&terrain.id, &terrain.glyph)?;
        normalize_tags(&terrain.id, &mut terrain.tags)?;
        insert_definition_id(&mut all_ids, &terrain.id)?;
        terrain_ids.insert(terrain.id.clone());
        terrain_walkability.insert(terrain.id.clone(), terrain.walkable);
    }

    let mut actor_roles = BTreeMap::new();
    for actor in &mut content.actors {
        require_schema(&actor.schema, ACTOR_SCHEMA, &actor.id)?;
        require_format_version(actor.format_version, &actor.id)?;
        validate_definition_id(&actor.id, "actor")?;
        validate_definition_text(&actor.id, &actor.name_key, &actor.description_key)?;
        validate_glyph(&actor.id, &actor.glyph)?;
        if actor.level > 10_000
            || actor.max_hp <= 0
            || actor.max_hp > 1_000_000
            || actor.speed > 199
            || actor.attack <= 0
            || actor.attack > 1_000_000
            || actor.defense < 0
            || actor.defense > 1_000_000
            || actor.damage_dice == 0
            || actor.damage_dice > 100
            || actor.damage_sides == 0
            || actor.damage_sides > 10_000
        {
            return Err(ContentError::InvalidActorStats(actor.id.clone()));
        }
        if let Some(routine) = &actor.melee_routine {
            if actor.role != ActorRole::Monster
                || routine.blows.is_empty()
                || routine.blows.len() > 8
                || routine.blows.iter().any(|blow| {
                    validate_id(&blow.method_id).is_err()
                        || blow.to_hit < -1_000_000
                        || blow.to_hit > 1_000_000
                        || blow.damage_dice == 0
                        || blow.damage_dice > 100
                        || blow.damage_sides == 0
                        || blow.damage_sides > 10_000
                })
            {
                return Err(ContentError::InvalidMeleeRoutine(actor.id.clone()));
            }
        }
        normalize_tags(&actor.id, &mut actor.tags)?;
        insert_definition_id(&mut all_ids, &actor.id)?;
        actor_roles.insert(actor.id.clone(), actor.role);
    }

    let mut item_limits = BTreeMap::new();
    for item in &mut content.items {
        require_schema(&item.schema, ITEM_SCHEMA, &item.id)?;
        require_format_version(item.format_version, &item.id)?;
        validate_definition_id(&item.id, "item")?;
        validate_definition_text(&item.id, &item.name_key, &item.description_key)?;
        validate_glyph(&item.id, &item.glyph)?;
        if item.max_stack == 0 || item.max_stack > 1_000_000 {
            return Err(ContentError::InvalidItemStack(item.id.clone()));
        }
        if let Some(slot) = &item.equipment_slot
            && (item.max_stack != 1 || validate_equipment_slot(slot).is_err())
        {
            return Err(ContentError::InvalidEquipmentSlot(item.id.clone()));
        }
        if item.modifiers.max_hp < 0
            || item.modifiers.max_hp > 1_000_000
            || item.modifiers.attack < -1_000_000
            || item.modifiers.attack > 1_000_000
            || item.modifiers.defense < -1_000_000
            || item.modifiers.defense > 1_000_000
            || (item.equipment_slot.is_none() && item.modifiers != StatModifiers::default())
        {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
        if let Some(profile) = &item.melee_profile
            && (item.max_stack != 1
                || item.equipment_slot.as_deref() != Some("weapon")
                || profile.attacks == 0
                || profile.attacks > 8
                || profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000
                || profile.damage_dice == 0
                || profile.damage_dice > 100
                || profile.damage_sides == 0
                || profile.damage_sides > 10_000)
        {
            return Err(ContentError::InvalidAttackProfile(item.id.clone()));
        }
        if let Some(profile) = &item.projectile_profile
            && (item.max_stack != 1
                || item.equipment_slot.as_deref() != Some("launcher")
                || profile.range == 0
                || profile.range > 32
                || profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000
                || profile.damage_dice == 0
                || profile.damage_dice > 100
                || profile.damage_sides == 0
                || profile.damage_sides > 10_000)
        {
            return Err(ContentError::InvalidProjectileProfile(item.id.clone()));
        }
        normalize_tags(&item.id, &mut item.tags)?;
        insert_definition_id(&mut all_ids, &item.id)?;
        item_limits.insert(item.id.clone(), item.max_stack);
    }

    for world in &mut content.worlds {
        require_schema(&world.schema, WORLD_SCHEMA, &world.id)?;
        require_format_version(world.format_version, &world.id)?;
        validate_definition_id(&world.id, "world")?;
        validate_message_key(&world.name_key)?;
        insert_definition_id(&mut all_ids, &world.id)?;
        validate_world(
            world,
            &terrain_ids,
            &terrain_walkability,
            &actor_roles,
            &item_limits,
        )?;
    }
    Ok(())
}

fn validate_world(
    world: &mut WorldDefinition,
    terrain_ids: &BTreeSet<String>,
    terrain_walkability: &BTreeMap<String, bool>,
    actor_roles: &BTreeMap<String, ActorRole>,
    item_limits: &BTreeMap<String, u32>,
) -> Result<(), ContentError> {
    if world.width < 3 || world.height < 3 || world.width > 512 || world.height > 512 {
        return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
    }
    require_reference(terrain_ids, &world.fill_terrain_id, &world.id)?;
    require_reference(terrain_ids, &world.border_terrain_id, &world.id)?;
    require_actor_role(
        actor_roles,
        &world.player.kind_id,
        ActorRole::Player,
        &world.id,
    )?;
    validate_position(world.player.position, world.width, world.height, &world.id)?;
    validate_id(&world.player.instance_id)?;

    let mut instance_ids = BTreeSet::new();
    instance_ids.insert(world.player.instance_id.clone());
    let mut actor_positions = BTreeSet::new();
    actor_positions.insert(world.player.position);

    world
        .actors
        .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    for actor in &world.actors {
        validate_id(&actor.instance_id)?;
        if !instance_ids.insert(actor.instance_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(actor.instance_id.clone()));
        }
        require_actor_role(actor_roles, &actor.kind_id, ActorRole::Monster, &world.id)?;
        validate_position(actor.position, world.width, world.height, &world.id)?;
        if !actor_positions.insert(actor.position) {
            return Err(ContentError::DuplicateActorPosition(world.id.clone()));
        }
    }

    world
        .items
        .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    for item in &world.items {
        validate_id(&item.instance_id)?;
        if !instance_ids.insert(item.instance_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(item.instance_id.clone()));
        }
        let max_stack =
            item_limits
                .get(&item.kind_id)
                .ok_or_else(|| ContentError::DanglingReference {
                    owner: world.id.clone(),
                    target: item.kind_id.clone(),
                })?;
        if item.quantity == 0 || item.quantity > *max_stack {
            return Err(ContentError::InvalidItemQuantity(item.instance_id.clone()));
        }
        validate_position(item.position, world.width, world.height, &world.id)?;
    }

    world
        .terrain_overrides
        .sort_by(|left, right| left.terrain_id.cmp(&right.terrain_id));
    let mut override_terrain = BTreeMap::new();
    for terrain_override in &mut world.terrain_overrides {
        require_reference(terrain_ids, &terrain_override.terrain_id, &world.id)?;
        terrain_override.positions.sort();
        for position in &terrain_override.positions {
            validate_position(*position, world.width, world.height, &world.id)?;
            if position.x == 0
                || position.y == 0
                || position.x == world.width - 1
                || position.y == world.height - 1
                || override_terrain
                    .insert(*position, terrain_override.terrain_id.clone())
                    .is_some()
            {
                return Err(ContentError::InvalidTerrainOverride(world.id.clone()));
            }
        }
    }

    require_walkable_spawn(
        world,
        world.player.position,
        &override_terrain,
        terrain_walkability,
    )?;
    for actor in &world.actors {
        require_walkable_spawn(
            world,
            actor.position,
            &override_terrain,
            terrain_walkability,
        )?;
    }
    for item in &world.items {
        require_walkable_spawn(world, item.position, &override_terrain, terrain_walkability)?;
    }
    Ok(())
}

fn require_walkable_spawn(
    world: &WorldDefinition,
    position: ContentPosition,
    override_terrain: &BTreeMap<ContentPosition, String>,
    terrain_walkability: &BTreeMap<String, bool>,
) -> Result<(), ContentError> {
    let terrain_id = if position.x == 0
        || position.y == 0
        || position.x == world.width - 1
        || position.y == world.height - 1
    {
        &world.border_terrain_id
    } else {
        override_terrain
            .get(&position)
            .unwrap_or(&world.fill_terrain_id)
    };
    if terrain_walkability.get(terrain_id) != Some(&true) {
        return Err(ContentError::SpawnOnBlockedTerrain(world.id.clone()));
    }
    Ok(())
}

fn load_root<T: DeserializeOwned>(
    pack_root: &Path,
    root: &str,
    enabled_roots: &BTreeSet<&str>,
    budget: &mut SourceBudget,
) -> Result<Vec<T>, ContentError> {
    if !enabled_roots.contains(root) {
        return Ok(Vec::new());
    }
    let directory = pack_root.join(root);
    let metadata = fs::symlink_metadata(&directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidContentDirectory(directory));
    }
    let mut paths = fs::read_dir(&directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut definitions = Vec::with_capacity(paths.len());
    for path in paths {
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || path.extension().and_then(|value| value.to_str()) != Some("json")
        {
            return Err(ContentError::InvalidContentFile(path));
        }
        definitions.push(read_json(&path, budget)?);
    }
    Ok(definitions)
}

fn read_json<T: DeserializeOwned>(
    path: &Path,
    budget: &mut SourceBudget,
) -> Result<T, ContentError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidContentFile(path.to_path_buf()));
    }
    budget.files = budget
        .files
        .checked_add(1)
        .ok_or(ContentError::LengthOverflow)?;
    if budget.files > MAX_SOURCE_FILES {
        return Err(ContentError::TooManySourceFiles(budget.files));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_SOURCE_FILE_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SOURCE_FILE_LENGTH {
        return Err(ContentError::SourceFileTooLarge(path.to_path_buf()));
    }
    budget.bytes = budget
        .bytes
        .checked_add(bytes.len())
        .ok_or(ContentError::LengthOverflow)?;
    if budget.bytes > MAX_SOURCE_TOTAL_LENGTH {
        return Err(ContentError::SourcePackTooLarge(budget.bytes));
    }
    serde_json::from_slice(&bytes).map_err(|source| ContentError::InvalidJson {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_definition_id(id: &str, category: &str) -> Result<(), ContentError> {
    validate_id(id)?;
    if id.split('.').nth(1) != Some(category) {
        return Err(ContentError::WrongIdCategory {
            id: id.to_owned(),
            expected: category.to_owned(),
        });
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), ContentError> {
    if id.is_empty()
        || id.len() > 128
        || id.split('.').count() < 3
        || id.split('.').any(str::is_empty)
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        return Err(ContentError::InvalidStableId(id.to_owned()));
    }
    Ok(())
}

fn validate_semver(version: &str) -> Result<(), ContentError> {
    if version.is_empty() || version.len() > 64 || !version.is_ascii() {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let (core_and_prerelease, build) = version
        .split_once('+')
        .map_or((version, None), |(core, build)| (core, Some(build)));
    if version.matches('+').count() > 1
        || build.is_some_and(|value| !valid_semver_identifiers(value, false))
    {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let (core, prerelease) = core_and_prerelease
        .split_once('-')
        .map_or((core_and_prerelease, None), |(core, prerelease)| {
            (core, Some(prerelease))
        });
    if prerelease.is_some_and(|value| !valid_semver_identifiers(value, true)) {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let parts = core.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    Ok(())
}

fn valid_semver_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && !(reject_numeric_leading_zero
                    && identifier.len() > 1
                    && identifier.starts_with('0')
                    && identifier.bytes().all(|byte| byte.is_ascii_digit()))
        })
}

fn validate_pack_relations(
    pack_id: &str,
    dependencies: &[PackDependency],
    load_after_entries: &[String],
) -> Result<(), ContentError> {
    let mut dependency_ids = BTreeSet::new();
    for dependency in dependencies {
        validate_id(&dependency.id)?;
        if dependency.id == pack_id || !dependency_ids.insert(&dependency.id) {
            return Err(ContentError::InvalidDependency(dependency.id.clone()));
        }
        if dependency.version_requirement.trim().is_empty()
            || dependency.version_requirement.len() > 64
        {
            return Err(ContentError::InvalidVersionRequirement(
                dependency.version_requirement.clone(),
            ));
        }
    }
    let mut load_after = BTreeSet::new();
    for id in load_after_entries {
        validate_id(id)?;
        if id == pack_id || !load_after.insert(id) {
            return Err(ContentError::InvalidLoadAfter(id.clone()));
        }
    }
    Ok(())
}

fn validate_message_key(key: &str) -> Result<(), ContentError> {
    if key.is_empty()
        || key.len() > 128
        || !key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(ContentError::InvalidMessageKey(key.to_owned()));
    }
    Ok(())
}

fn validate_equipment_slot(slot: &str) -> Result<(), ContentError> {
    if slot.is_empty()
        || slot.len() > 64
        || !slot.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(ContentError::InvalidEquipmentSlot(slot.to_owned()));
    }
    Ok(())
}

fn validate_definition_text(
    id: &str,
    name_key: &str,
    description_key: &str,
) -> Result<(), ContentError> {
    validate_message_key(name_key)
        .map_err(|_| ContentError::InvalidDefinitionText(id.to_owned()))?;
    validate_message_key(description_key)
        .map_err(|_| ContentError::InvalidDefinitionText(id.to_owned()))?;
    Ok(())
}

fn validate_glyph(id: &str, glyph: &str) -> Result<(), ContentError> {
    let mut characters = glyph.chars();
    if characters.next().is_none_or(char::is_control) || characters.next().is_some() {
        return Err(ContentError::InvalidGlyph(id.to_owned()));
    }
    Ok(())
}

fn normalize_tags(id: &str, tags: &mut [String]) -> Result<(), ContentError> {
    for tag in tags.iter() {
        if tag.is_empty()
            || tag.len() > 64
            || !tag.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            })
        {
            return Err(ContentError::InvalidTag {
                id: id.to_owned(),
                tag: tag.clone(),
            });
        }
    }
    tags.sort();
    if tags.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ContentError::DuplicateTag(id.to_owned()));
    }
    Ok(())
}

fn insert_definition_id(ids: &mut BTreeSet<String>, id: &str) -> Result<(), ContentError> {
    if !ids.insert(id.to_owned()) {
        return Err(ContentError::DuplicateDefinitionId(id.to_owned()));
    }
    Ok(())
}

fn require_schema(actual: &str, expected: &str, owner: &str) -> Result<(), ContentError> {
    if actual != expected {
        return Err(ContentError::SchemaMismatch(owner.to_owned()));
    }
    Ok(())
}

fn require_format_version(actual: u16, owner: &str) -> Result<(), ContentError> {
    if actual != CONTENT_FORMAT_VERSION {
        return Err(ContentError::UnsupportedSourceVersion {
            owner: owner.to_owned(),
            version: actual,
        });
    }
    Ok(())
}

fn require_reference(
    ids: &BTreeSet<String>,
    target: &str,
    owner: &str,
) -> Result<(), ContentError> {
    if !ids.contains(target) {
        return Err(ContentError::DanglingReference {
            owner: owner.to_owned(),
            target: target.to_owned(),
        });
    }
    Ok(())
}

fn require_actor_role(
    roles: &BTreeMap<String, ActorRole>,
    target: &str,
    expected: ActorRole,
    owner: &str,
) -> Result<(), ContentError> {
    match roles.get(target) {
        Some(actual) if *actual == expected => Ok(()),
        Some(_) => Err(ContentError::WrongActorRole(target.to_owned())),
        None => Err(ContentError::DanglingReference {
            owner: owner.to_owned(),
            target: target.to_owned(),
        }),
    }
}

fn validate_position(
    position: ContentPosition,
    width: u16,
    height: u16,
    owner: &str,
) -> Result<(), ContentError> {
    if position.x >= width || position.y >= height {
        return Err(ContentError::PositionOutOfBounds(owner.to_owned()));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ContentError> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or(ContentError::InvalidContainer)?
            .try_into()
            .map_err(|_| ContentError::InvalidContainer)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ContentError> {
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or(ContentError::InvalidContainer)?
            .try_into()
            .map_err(|_| ContentError::InvalidContainer)?,
    ))
}

#[cfg(feature = "schemas")]
pub fn generated_schema_documents() -> Result<Vec<(&'static str, String)>, serde_json::Error> {
    Ok(vec![
        schema_document("pack.schema.json", PACK_SCHEMA, schema_for!(PackManifest))?,
        schema_document(
            "terrain.schema.json",
            TERRAIN_SCHEMA,
            schema_for!(TerrainDefinition),
        )?,
        schema_document(
            "actor.schema.json",
            ACTOR_SCHEMA,
            schema_for!(ActorDefinition),
        )?,
        schema_document("item.schema.json", ITEM_SCHEMA, schema_for!(ItemDefinition))?,
        schema_document(
            "world.schema.json",
            WORLD_SCHEMA,
            schema_for!(WorldDefinition),
        )?,
    ])
}

#[cfg(feature = "schemas")]
fn schema_document<T: Serialize>(
    file_name: &'static str,
    schema_id: &str,
    schema: T,
) -> Result<(&'static str, String), serde_json::Error> {
    let mut value = serde_json::to_value(schema)?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "$id".to_owned(),
            serde_json::Value::String(schema_id.to_owned()),
        );
    }
    let mut output = serde_json::to_string_pretty(&value)?;
    output.push('\n');
    Ok((file_name, output))
}

#[derive(Debug, Default)]
struct SourceBudget {
    files: usize,
    bytes: usize,
}

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("content pack root is invalid or is a symlink: {0}")]
    InvalidPackRoot(PathBuf),
    #[error("content directory is invalid or is a symlink: {0}")]
    InvalidContentDirectory(PathBuf),
    #[error("content entry must be a regular .json file: {0}")]
    InvalidContentFile(PathBuf),
    #[error("content source file exceeds the 1 MiB limit: {0}")]
    SourceFileTooLarge(PathBuf),
    #[error("content source pack exceeds the 16 MiB limit: {0} bytes")]
    SourcePackTooLarge(usize),
    #[error("content source pack exceeds the file-count limit: {0}")]
    TooManySourceFiles(usize),
    #[error("invalid JSON in {path}: {source}")]
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("schema identifier does not match for {0}")]
    SchemaMismatch(String),
    #[error("unsupported source format version {version} in {owner}")]
    UnsupportedSourceVersion { owner: String, version: u16 },
    #[error("unsupported content root {0}")]
    UnsupportedContentRoot(String),
    #[error("duplicate content root {0}")]
    DuplicateContentRoot(String),
    #[error("invalid stable content ID {0}")]
    InvalidStableId(String),
    #[error("content ID {id} must use category {expected}")]
    WrongIdCategory { id: String, expected: String },
    #[error("invalid pack semantic version {0}")]
    InvalidPackVersion(String),
    #[error("invalid dependency {0}")]
    InvalidDependency(String),
    #[error("invalid dependency version requirement {0}")]
    InvalidVersionRequirement(String),
    #[error("invalid loadAfter entry {0}")]
    InvalidLoadAfter(String),
    #[error("invalid localization message key {0}")]
    InvalidMessageKey(String),
    #[error("definition name or description key is invalid: {0}")]
    InvalidDefinitionText(String),
    #[error("definition glyph must contain one non-control Unicode scalar: {0}")]
    InvalidGlyph(String),
    #[error("invalid tag {tag} in {id}")]
    InvalidTag { id: String, tag: String },
    #[error("duplicate tag in {0}")]
    DuplicateTag(String),
    #[error("duplicate definition ID {0}")]
    DuplicateDefinitionId(String),
    #[error("actor stats are outside supported limits: {0}")]
    InvalidActorStats(String),
    #[error("actor melee routine is invalid or requires the monster role: {0}")]
    InvalidMeleeRoutine(String),
    #[error("item stack limit is outside supported limits: {0}")]
    InvalidItemStack(String),
    #[error("item equipment slot is invalid or requires maxStack 1: {0}")]
    InvalidEquipmentSlot(String),
    #[error("item stat modifiers are invalid or require an equipment slot: {0}")]
    InvalidItemModifiers(String),
    #[error("item attack profile is invalid or requires the weapon slot: {0}")]
    InvalidAttackProfile(String),
    #[error("item projectile profile is invalid or requires the launcher slot: {0}")]
    InvalidProjectileProfile(String),
    #[error("world dimensions are outside supported limits: {0}")]
    InvalidWorldDimensions(String),
    #[error("content reference from {owner} to {target} is unresolved")]
    DanglingReference { owner: String, target: String },
    #[error("actor has the wrong role for this spawn: {0}")]
    WrongActorRole(String),
    #[error("duplicate runtime instance ID {0}")]
    DuplicateInstanceId(String),
    #[error("two actors occupy the same world position: {0}")]
    DuplicateActorPosition(String),
    #[error("content position is outside world bounds: {0}")]
    PositionOutOfBounds(String),
    #[error("world spawn is placed on non-walkable terrain: {0}")]
    SpawnOnBlockedTerrain(String),
    #[error("terrain override is duplicated or touches the generated border: {0}")]
    InvalidTerrainOverride(String),
    #[error("item spawn quantity is invalid: {0}")]
    InvalidItemQuantity(String),
    #[error("compiled content metadata is invalid")]
    InvalidCompiledMetadata,
    #[error("compiled content payload exceeds the 32 MiB limit: {0} bytes")]
    CompiledPayloadTooLarge(usize),
    #[error("compiled content container is invalid or truncated")]
    InvalidContainer,
    #[error("unsupported compiled content container version {0}")]
    UnsupportedContainerVersion(u16),
    #[error("unsupported compiled content container flags 0x{0:04x}")]
    UnsupportedContainerFlags(u16),
    #[error("compiled content checksum does not match")]
    ChecksumMismatch,
    #[error("compiled content is not in canonical sorted form")]
    NonCanonicalCompiledContent,
    #[error("content.lock.json does not match the deterministic compiled pack")]
    ContentLockMismatch,
    #[error("content length overflow")]
    LengthOverflow,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("MessagePack encode error: {0}")]
    MessagePackEncode(#[from] rmp_serde::encode::Error),
    #[error("MessagePack decode error: {0}")]
    MessagePackDecode(#[from] rmp_serde::decode::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original_pack_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace")
            .join("packs/rfb-demo-original")
    }

    #[test]
    fn original_pack_compiles_deterministically_and_round_trips() {
        let first = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
        let second = compile_pack_dir(&original_pack_path()).expect("recompile should succeed");
        let decoded = decode_content(&first.bytes).expect("compiled pack should decode");

        assert_eq!(first.content_hash, second.content_hash);
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(decoded, first);
        assert_eq!(first.content.pack_id, "rfb.demo.original-v1");
        assert_eq!(first.content.terrain.len(), 2);
        assert_eq!(first.content.actors.len(), 7);
        assert_eq!(first.content.items.len(), 4);
        assert_eq!(first.content.worlds.len(), 1);
    }

    #[test]
    fn compiled_catalog_exposes_stable_runtime_indexes() {
        let artifact =
            verify_pack_lock(&original_pack_path()).expect("original pack should verify");
        let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

        assert_eq!(catalog.pack_id(), "rfb.demo.original-v1");
        assert_eq!(catalog.pack_version(), "1.10.0");
        assert_eq!(
            catalog
                .actor("demo.actor.echo-hound")
                .and_then(|actor| actor.melee_routine.as_ref())
                .map(|routine| routine
                    .blows
                    .iter()
                    .map(|blow| blow.method_id.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["rfb.blow.echo-bite", "rfb.blow.echo-rake"])
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-blade")
                .and_then(|item| item.melee_profile.as_ref())
                .map(|profile| (profile.attacks, profile.to_hit, profile.to_damage)),
            Some((2, 10, 1))
        );
        assert_eq!(
            catalog
                .item("demo.item.resonance-sling")
                .and_then(|item| item.projectile_profile.as_ref())
                .map(|profile| (profile.range, profile.to_hit, profile.to_damage)),
            Some((6, 30, 1))
        );
        assert_eq!(catalog.content_hash(), artifact.content_hash);
        assert_eq!(
            catalog
                .terrain("demo.terrain.wall")
                .map(|terrain| terrain.walkable),
            Some(false)
        );
        assert_eq!(
            catalog
                .actor("demo.actor.ember-mote")
                .map(|actor| actor.max_hp),
            Some(3)
        );
        assert_eq!(
            catalog
                .actor("demo.actor.ember-mote")
                .map(|actor| actor.damage_type),
            Some(ActorDamageType::Fire)
        );
        assert_eq!(
            catalog.actor("demo.actor.explorer").map(|actor| (
                actor.attack,
                actor.defense,
                actor.damage_dice,
                actor.damage_sides,
                actor.speed,
            )),
            Some((2, 1, 1, 2, 110))
        );
        assert_eq!(
            catalog
                .item("demo.item.luminous-shard")
                .map(|item| item.max_stack),
            Some(20)
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-charm")
                .and_then(|item| item.equipment_slot.as_deref()),
            Some("charm")
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-charm")
                .map(|item| item.modifiers.max_hp),
            Some(4)
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-charm")
                .map(|item| (item.modifiers.attack, item.modifiers.defense)),
            Some((1, 1))
        );
        assert!(catalog.world("demo.world.original-v1").is_some());
        assert_eq!(
            catalog.visual_glyphs().get("demo.item.luminous-shard"),
            Some(&"!".to_owned())
        );
    }

    #[test]
    fn dangling_references_and_checksum_corruption_are_rejected() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        invalid.worlds[0].fill_terrain_id = "demo.terrain.missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut blocked_spawn = artifact.content.clone();
        blocked_spawn.worlds[0].player.position = ContentPosition { x: 11, y: 3 };
        assert!(matches!(
            validate_and_normalize(&mut blocked_spawn),
            Err(ContentError::SpawnOnBlockedTerrain(_))
        ));

        let mut corrupted = artifact.bytes;
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x01;
        assert!(matches!(
            decode_content(&corrupted),
            Err(ContentError::ChecksumMismatch)
        ));
    }

    #[test]
    fn equippable_items_require_a_valid_slot_and_single_item_stack() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the shard");
        shard.equipment_slot = Some("charm".to_owned());

        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidEquipmentSlot(_))
        ));

        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the shard");
        shard.modifiers.max_hp = 1;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemModifiers(_))
        ));

        let mut invalid = artifact.content.clone();
        let blade = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.echo-blade")
            .expect("fixture should contain the blade");
        blade.equipment_slot = Some("charm".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidAttackProfile(_))
        ));

        let mut invalid = artifact.content.clone();
        let sling = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-sling")
            .expect("fixture should contain the sling");
        sling.equipment_slot = Some("weapon".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidProjectileProfile(_))
        ));
    }

    #[test]
    fn melee_routines_require_monsters_and_valid_blow_profiles() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        let hound = invalid
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-hound")
            .expect("fixture should contain the echo hound");
        hound.role = ActorRole::Player;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidMeleeRoutine(_))
        ));

        let mut invalid = artifact.content;
        let hound = invalid
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-hound")
            .expect("fixture should contain the echo hound");
        hound
            .melee_routine
            .as_mut()
            .expect("hound should have a melee routine")
            .blows[0]
            .damage_dice = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidMeleeRoutine(_))
        ));
    }

    #[test]
    fn semantic_versions_are_checked_strictly() {
        assert!(validate_semver("1.2.3-alpha.1+build.5").is_ok());
        for invalid in ["01.2.3", "1.2", "1.2.3-", "1.2.3+", "1.2.3-alpha..1"] {
            assert!(matches!(
                validate_semver(invalid),
                Err(ContentError::InvalidPackVersion(_))
            ));
        }
    }
}

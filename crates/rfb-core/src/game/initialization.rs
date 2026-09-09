// SPDX-License-Identifier: MPL-2.0

use crate::error::CoreError;
use crate::game::ego::materialize_ego_with_rng;
use crate::game::inventory::{ItemKnowledgeState, ItemPropertyKnowledgeState};
use crate::game::mogaminator::MogaminatorState;
use crate::game::progression::{
    build_definitions, character_skill_progress, initial_character_attributes,
    resolve_character_build,
};
use crate::game::tasks::{CampaignState, initial_task_states};
use crate::game::{
    BodySlot, DEFAULT_WORLD_ID, DungeonState, Game, actor_starts_alerted, base_dungeon_states,
    body_slot_instance_for_type, body_slots_for_race, bounty, chaos_patron, gold, hunger,
    initial_item_curse, initial_item_runtime_state, item_quality_dto, lighting,
    load_built_in_content, normalize_player_name, spawn_actor_from_definition, standard_body_slots,
    town, virtues, wilderness,
};
use crate::rng::RfbRng;
use crate::save::{
    GENERATED_ITEM_ID_PREFIX, actor_from_spawn, derive_next_item_instance_serial,
    initial_item_fuel, position_from_content,
};
use crate::scheduler::{INITIAL_MONSTER_ENERGY_NEED, INITIAL_PLAYER_ENERGY_NEED};
use crate::state::{ItemInstance, ItemLocation, MonsterPackIdentity};
use crate::stats::{CharacterBuildIdentity, CharacterProgress};
use rfb_content::{ContentCatalog, StartingItemDefinition};
use rfb_protocol::{
    ItemEnchantmentsDto, ItemQualityDto, LocaleDto, MapScaleDto, MonsterPackBehaviorDto,
    MonsterPackRoleDto, Position, SummonCommandDto,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) fn dungeon_substitution_uses_alternate(
    primary: &rfb_content::DungeonDefinition,
    alternate: &rfb_content::DungeonDefinition,
    seed: u64,
) -> bool {
    let primary_index = u64::from(
        primary
            .legacy_index
            .expect("validated substituted dungeon must retain a legacy index"),
    );
    let alternate_index = u64::from(
        alternate
            .legacy_index
            .expect("validated alternate dungeon must retain a legacy index"),
    );
    let pair_modulus = primary_index * 48 + alternate_index;
    let selected = seed % (pair_modulus * 2) >= pair_modulus;
    selected
        && primary
            .substitution
            .as_ref()
            .and_then(|substitution| substitution.alternate_gate_one_in)
            .is_none_or(|one_in| seed.is_multiple_of(u64::from(one_in)))
}

fn initial_dungeon_states(
    world: &rfb_content::WorldDefinition,
    seed: u64,
) -> BTreeMap<String, DungeonState> {
    let mut states = base_dungeon_states(world);
    for primary in &world.dungeons {
        let Some(substitution) = &primary.substitution else {
            continue;
        };
        let alternate = world
            .dungeons
            .iter()
            .find(|dungeon| dungeon.id == substitution.alternate_dungeon_id)
            .expect("validated dungeon substitution must retain its alternate");
        let suppressed_id = if dungeon_substitution_uses_alternate(primary, alternate, seed) {
            &primary.id
        } else {
            &alternate.id
        };
        states
            .get_mut(suppressed_id)
            .expect("substituted dungeon state must remain available")
            .suppressed = true;
    }
    states
}

/// Body slots come from the build's race when it declares any, otherwise
/// the standard template applies. Games without a build use the standard
/// template as well.
pub(super) fn resolve_body_slots(
    content: &ContentCatalog,
    identity: Option<&CharacterBuildIdentity>,
) -> Result<Vec<BodySlot>, CoreError> {
    let Some(identity) = identity else {
        return Ok(standard_body_slots());
    };
    let (_, race, _, _) = build_definitions(content, identity)?;
    Ok(body_slots_for_race(race))
}

fn append_starting_items(
    content: &ContentCatalog,
    identity: Option<&CharacterBuildIdentity>,
    body_slots: &[BodySlot],
    items: &mut Vec<ItemInstance>,
    next_serial: &mut u64,
    rng: &mut RfbRng,
) -> Result<(), CoreError> {
    let Some(identity) = identity else {
        return Ok(());
    };
    let (build, race, class, personality) = build_definitions(content, identity)?;
    for starting_item in race
        .starting_items
        .iter()
        .chain(class.starting_items.iter())
        .chain(personality.starting_items.iter())
        .chain(build.starting_items.iter())
    {
        append_starting_item(content, starting_item, body_slots, items, next_serial, rng)?;
    }
    Ok(())
}

fn append_starting_item(
    content: &ContentCatalog,
    starting_item: &StartingItemDefinition,
    body_slots: &[BodySlot],
    items: &mut Vec<ItemInstance>,
    next_serial: &mut u64,
    rng: &mut RfbRng,
) -> Result<(), CoreError> {
    let definition = content
        .item(&starting_item.item_kind_id)
        .ok_or_else(|| CoreError::UnknownItem(starting_item.item_kind_id.clone()))?;
    let location = if starting_item.equipped {
        let slot_type = definition
            .equipment_slot
            .as_deref()
            .ok_or(CoreError::InvalidSave("starting equipment is invalid"))?;
        let occupied = |slot_id: &str| {
            items.iter().any(|item| {
                matches!(
                    &item.location,
                    ItemLocation::Equipped { slot_id: equipped } if equipped == slot_id
                )
            })
        };
        let slot = body_slot_instance_for_type(body_slots, slot_type, occupied)
            .ok_or(CoreError::InvalidSave("starting equipment is invalid"))?;
        ItemLocation::Equipped {
            slot_id: slot.id.clone(),
        }
    } else {
        ItemLocation::Inventory
    };
    let id = format!("{GENERATED_ITEM_ID_PREFIX}{next_serial}");
    *next_serial = next_serial
        .checked_add(1)
        .ok_or(CoreError::ItemIdExhausted)?;
    let (activation, mut charges) =
        initial_item_runtime_state(content, rng, &starting_item.item_kind_id, &[], 1);
    if starting_item.fully_charged {
        let charges = charges
            .as_mut()
            .expect("validated fully charged starting item must be a device");
        charges.current = charges.maximum;
    }
    let quantity = starting_item
        .maximum_quantity
        .map_or(starting_item.quantity, |maximum| {
            starting_item.quantity
                + u32::try_from(rng.bounded(u64::from(maximum - starting_item.quantity + 1)))
                    .expect("validated birth item quantity must fit u32")
        });
    items.push(ItemInstance {
        artifact_name: None,
        intrinsic_melee_damage_dice: None,
        intrinsic_weight_tenths_pound: None,
        intrinsic_weapon_traits: Default::default(),
        intrinsic_curse_effects: Default::default(),
        id,
        kind_id: starting_item.item_kind_id.clone(),
        quantity,
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        intrinsic_properties: Default::default(),
        enchantments: ItemEnchantmentsDto::default(),
        curse: initial_item_curse(content, &starting_item.item_kind_id),
        permanent_destruction_immunities: Default::default(),
        activation,
        charges,
        fuel: initial_item_fuel(content, &starting_item.item_kind_id),
        device_recovery_progress: 0,
        captured_actor: None,
        location,
    });
    Ok(())
}

impl Game {
    pub const DEFAULT_PLAYER_NAME: &'static str = "RFB Demo Character";

    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::from_content(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
        )
        .expect("built-in world should create a game")
    }

    pub fn new_with_build(seed: u64, build_id: &str) -> Result<Self, CoreError> {
        Self::from_content_with_build(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
            build_id,
        )
    }

    pub fn new_with_build_and_name(
        seed: u64,
        build_id: &str,
        player_name: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
            Some(build_id),
            None,
            player_name,
        )
    }

    pub fn new_with_build_race_and_name(
        seed: u64,
        build_id: &str,
        race_id: &str,
        player_name: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            load_built_in_content().expect("built-in content should decode"),
            DEFAULT_WORLD_ID,
            Some(build_id),
            Some(race_id),
            player_name,
        )
    }

    pub fn from_content(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            content,
            world_id,
            None,
            None,
            Self::DEFAULT_PLAYER_NAME,
        )
    }

    pub fn from_content_with_build(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
        build_id: &str,
    ) -> Result<Self, CoreError> {
        Self::from_content_internal(
            seed,
            content,
            world_id,
            Some(build_id),
            None,
            Self::DEFAULT_PLAYER_NAME,
        )
    }

    pub(super) fn from_content_internal(
        seed: u64,
        content: Arc<ContentCatalog>,
        world_id: &str,
        build_id: Option<&str>,
        race_id: Option<&str>,
        player_name: &str,
    ) -> Result<Self, CoreError> {
        let player_name = normalize_player_name(player_name).ok_or(CoreError::InvalidPlayerName)?;
        let world = content
            .world(world_id)
            .ok_or_else(|| CoreError::UnknownWorld(world_id.to_owned()))?;
        let build = resolve_character_build(
            &content,
            build_id.or(world.player_build_id.as_deref()),
            race_id,
        )?;
        let starts_at_night = build
            .as_ref()
            .and_then(|identity| content.race(&identity.race_id))
            .is_some_and(|race| race.tags.iter().any(|tag| tag == "night-start"));
        let starting_world_tick = if starts_at_night {
            wilderness::WILDERNESS_NIGHT_START_TICK
        } else {
            0
        };
        let width = world.width;
        let height = world.height;
        let mut terrain =
            vec![world.fill_terrain_id.clone(); usize::from(width) * usize::from(height)];
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    terrain[usize::from(y) * usize::from(width) + usize::from(x)] =
                        world.border_terrain_id.clone();
                }
            }
        }
        for terrain_override in &world.terrain_overrides {
            for position in &terrain_override.positions {
                terrain[usize::from(position.y) * usize::from(width) + usize::from(position.x)] =
                    terrain_override.terrain_id.clone();
            }
        }
        let player_kind_id = build
            .as_ref()
            .and_then(|identity| content.build(&identity.build_id))
            .and_then(|build| build.player_actor_id.as_deref())
            .unwrap_or(&world.player.kind_id);
        let player_definition = content
            .actor(player_kind_id)
            .ok_or_else(|| CoreError::UnknownActor(player_kind_id.to_owned()))?;
        let player = actor_from_spawn(
            &world.player.instance_id,
            player_kind_id,
            world.player.position,
            player_definition.max_hp,
            player_definition.speed,
            INITIAL_PLAYER_ENERGY_NEED,
            true,
        );
        let mut rng = RfbRng::seeded(seed);
        let virtues = virtues::initial_virtues(&content, build.as_ref(), &mut rng);
        let gold = gold::starting_gold(build.as_ref(), &mut rng);
        let starting_ration_quantity = hunger::starting_ration_quantity(build.as_ref(), &mut rng);
        let starting_torches = lighting::starting_torch_supply(build.as_ref(), &mut rng);
        let mut progress = CharacterProgress::new(seed, player_definition.max_hp);
        if let Some(identity) = build.as_ref() {
            let (definition, _, class, _) = build_definitions(&content, identity)?;
            progress.attributes = initial_character_attributes(definition);
            progress.maximum_attributes = progress.attributes;
            progress.riding_proficiency = class.riding_proficiency.initial;
        }
        progress.replace_skills(character_skill_progress(
            &content,
            build.as_ref(),
            progress.level,
        )?);
        let mut entities = world
            .actors
            .iter()
            .map(|spawn| {
                let definition = content
                    .actor(&spawn.kind_id)
                    .ok_or_else(|| CoreError::UnknownActor(spawn.kind_id.clone()))?;
                Ok(spawn_actor_from_definition(
                    &mut rng,
                    definition,
                    &spawn.instance_id,
                    position_from_content(spawn.position),
                    INITIAL_MONSTER_ENERGY_NEED,
                    actor_starts_alerted(definition),
                ))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        for dungeon in world.dungeons.iter().filter(|_| world.wilderness.is_none()) {
            let Some(guardian) = &dungeon.entrance_guardian else {
                continue;
            };
            let definition = content
                .actor(&guardian.actor_kind_id)
                .ok_or_else(|| CoreError::UnknownActor(guardian.actor_kind_id.clone()))?;
            let mut actor = spawn_actor_from_definition(
                &mut rng,
                definition,
                &guardian.instance_id,
                position_from_content(guardian.position),
                INITIAL_MONSTER_ENERGY_NEED,
                actor_starts_alerted(definition),
            );
            actor.pack = Some(MonsterPackIdentity {
                id: guardian.instance_id.clone(),
                leader_id: guardian.instance_id.clone(),
                role: MonsterPackRoleDto::Leader,
                behavior: MonsterPackBehaviorDto::GuardPosition,
            });
            entities.push(actor);
        }
        let mut items = world
            .items
            .iter()
            .map(|spawn| {
                let materialization = materialize_ego_with_rng(
                    &content,
                    &mut rng,
                    &spawn.kind_id,
                    spawn.affix_ids.clone(),
                    |_| 1,
                    1,
                    2,
                );
                let mut item = ItemInstance {
                    artifact_name: None,
                    intrinsic_melee_damage_dice: None,
                    intrinsic_weight_tenths_pound: None,
                    intrinsic_weapon_traits: Default::default(),
                    intrinsic_curse_effects: Default::default(),
                    id: spawn.instance_id.clone(),
                    kind_id: spawn.kind_id.clone(),
                    quantity: spawn.quantity,
                    inscription: None,
                    origin_actor_kind_id: None,
                    origin_kind: None,
                    damage_dice_override: None,
                    discount_percent: 0,
                    quality: item_quality_dto(spawn.quality),
                    affix_ids: Vec::new(),
                    rolled_affixes: Vec::new(),
                    intrinsic_properties: Default::default(),
                    enchantments: ItemEnchantmentsDto::default(),
                    curse: initial_item_curse(&content, &spawn.kind_id),
                    permanent_destruction_immunities: Default::default(),
                    activation: None,
                    charges: None,
                    fuel: initial_item_fuel(&content, &spawn.kind_id),
                    device_recovery_progress: 0,
                    captured_actor: None,
                    location: ItemLocation::Ground(position_from_content(spawn.position)),
                };
                materialization.apply_to(&mut item);
                item
            })
            .collect::<Vec<_>>();
        let body_slots = resolve_body_slots(&content, build.as_ref())?;
        let mut next_item_instance_serial =
            derive_next_item_instance_serial(&player, &entities, &items)?;
        if let Some(quantity) = starting_ration_quantity {
            append_starting_item(
                &content,
                &StartingItemDefinition {
                    item_kind_id: hunger::RATION_ITEM_KIND_ID.to_owned(),
                    quantity,
                    maximum_quantity: None,
                    equipped: false,
                    fully_charged: false,
                },
                &body_slots,
                &mut items,
                &mut next_item_instance_serial,
                &mut rng,
            )?;
        }
        if let Some(supply) = starting_torches {
            for _ in 0..supply.quantity {
                append_starting_item(
                    &content,
                    &StartingItemDefinition {
                        item_kind_id: lighting::WOODEN_TORCH_ITEM_KIND_ID.to_owned(),
                        quantity: 1,
                        maximum_quantity: None,
                        equipped: false,
                        fully_charged: false,
                    },
                    &body_slots,
                    &mut items,
                    &mut next_item_instance_serial,
                    &mut rng,
                )?;
                items
                    .last_mut()
                    .and_then(|item| item.fuel.as_mut())
                    .expect("validated birth torch must have fuel")
                    .current = supply.fuel;
            }
        }
        append_starting_items(
            &content,
            build.as_ref(),
            &body_slots,
            &mut items,
            &mut next_item_instance_serial,
            &mut rng,
        )?;
        let initial_floor_id = world.initial_floor_id.clone();
        let wilderness_position = world
            .wilderness
            .as_ref()
            .map(|wilderness| position_from_content(wilderness.start_position));
        let task_states = initial_task_states(world, seed);
        let dungeon_states = initial_dungeon_states(world, seed);
        let (town_states, shop_states) = town::initial_town_and_shop_states(
            world,
            &content,
            &mut rng,
            &mut next_item_instance_serial,
        )?;
        let home_states = town::initial_home_states(world, &content);
        let chaos_patron_id = chaos_patron::initial_chaos_patron_id(&content, &mut rng);
        let mogaminator = MogaminatorState::for_character(&content, seed);
        let generated_artifact_ids = items
            .iter()
            .filter(|item| {
                content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.artifact_generation.is_some())
            })
            .map(|item| item.kind_id.clone())
            .collect();
        let mut game = Self {
            content,
            world_id: world_id.to_owned(),
            map_scale: MapScaleDto::Local,
            wilderness_position,
            wilderness_view_offset: Position::default(),
            wilderness_seed: seed,
            wilderness_terrain_cache: BTreeMap::new(),
            world_travel_destination: None,
            interface_locale: LocaleDto::ZhCn,
            mogaminator,
            current_floor_id: initial_floor_id,
            current_dungeon_instance_id: None,
            reproduction_suppressed: false,
            stored_floors: BTreeMap::new(),
            width,
            height,
            terrain,
            glow: vec![false; usize::from(width) * usize::from(height)],
            player_name,
            player,
            riding_actor_id: None,
            riding_bond: None,
            build,
            body_slots,
            progress,
            virtues,
            resources: BTreeMap::new(),
            last_visual_cells: None,
            bonus_spell_learning_capacity: 0,
            learned_abilities: BTreeSet::new(),
            ability_progress: BTreeMap::new(),
            entities,
            items,
            gold,
            nutrition: rfb_protocol::PLAYER_NUTRITION_BIRTH,
            fasting: false,
            gold_piles: Vec::new(),
            item_knowledge: BTreeMap::new(),
            item_property_knowledge: BTreeMap::new(),
            task_states,
            bounty_state: bounty::BountyState::default(),
            command_actor_deaths: Vec::new(),
            dungeon_states,
            defeated_limited_actor_counts: BTreeMap::new(),
            generated_artifact_ids,
            town_states,
            shop_states,
            home_states,
            campaign_state: CampaignState::default(),
            summon_command: SummonCommandDto::default(),
            recall: None,
            confusing_strike_ready: false,
            sniper_concentration: 0,
            probed_actor_kind_ids: BTreeSet::new(),
            minor_slow: 0,
            minor_slow_energy: 0,
            chaos_patron_id,
            reality_change_ticks: 0,
            pending_mutation_direction: None,
            pending_ability_direction: None,
            next_item_instance_serial,
            next_gold_pile_serial: 1,
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            floor_connections: Vec::new(),
            floor_regions: Vec::new(),
            rng,
            revision: 0,
            turn: 0,
            world_tick: starting_world_tick,
            last_non_melee_fear_aura_tick: None,
            last_command_seq: 0,
            debug_ability_casts_succeed: false,
            debug_recharge_attempts_succeed: false,
            debug_recharge_attempts_fail: false,
            debug_recharge_sources_survive: false,
            debug_recall_delay_turns: None,
            debug_item_curses_land: false,
            debug_item_curses_resisted: false,
            monster_division_remainders: BTreeMap::new(),
        };
        game.initialize_birth_race_mutations();
        game.initialize_player_ability_state();
        game.initialize_starting_item_knowledge();
        let mut initial_entities = std::mem::take(&mut game.entities);
        for actor in &mut initial_entities {
            game.maybe_initialize_chameleon_form(actor);
        }
        game.entities = initial_entities;
        game.player.hp = game.effective_player_max_hp();
        game.initialize_carried_loot()?;
        game.initialize_continuous_wilderness_surface()?;
        game.refresh_daily_bounty_target();
        game.refresh_invisible_visibility(true, &BTreeMap::new());
        game.refresh_weird_mind_visibility(true, &BTreeMap::new());
        game.reveal_current_visibility();
        Ok(game)
    }

    fn initialize_starting_item_knowledge(&mut self) {
        for item in self.items.iter().filter(|item| {
            matches!(
                item.location,
                ItemLocation::Inventory | ItemLocation::Equipped { .. }
            )
        }) {
            if self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.appearance_name_key.is_some())
            {
                self.item_knowledge.insert(
                    item.kind_id.clone(),
                    ItemKnowledgeState {
                        tried: true,
                        aware: true,
                    },
                );
            }
            if matches!(item.location, ItemLocation::Equipped { .. }) {
                self.item_property_knowledge.insert(
                    item.id.clone(),
                    ItemPropertyKnowledgeState {
                        discovered: true,
                        appraised: true,
                        identified: true,
                        known_affix_ids: item.affix_ids.iter().cloned().collect(),
                    },
                );
            }
        }
    }
}

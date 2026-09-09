// SPDX-License-Identifier: MPL-2.0

use super::support::*;
use super::*;
use rfb_content::ActorResistanceLevel;

mod casting;
mod compound;
mod control;
mod damage;
mod items;
mod restoration;
mod scaling;
mod summoning;
mod terrain;
mod travel;

fn set_test_virtue(game: &mut Game, slot: usize, kind: VirtueKindDto, value: i16) {
    game.virtues[slot] = VirtueDto { kind, value };
}

const MUTATION_CONTRACT_ABILITY_ID: &str = "demo.ability.mutation-contract";

const MUTATION_CONTRACT_ID: &str = "rfb.mutation.spit-acid";

const RACE_BERSERK_ABILITY_ID: &str = "rfb.ability.race.berserk";

const RACE_CREATE_FOOD_ABILITY_ID: &str = "rfb.ability.race.create-food";

const RACE_DETECT_DOORS_ABILITY_ID: &str = "rfb.ability.race.detect-doors-stairs-traps";

const RACE_DETECT_TREASURE_ABILITY_ID: &str = "rfb.ability.race.detect-treasure";

const RACE_POISON_DART_ABILITY_ID: &str = "rfb.ability.race.poison-dart";

fn mutation_ability_catalog_with_effect(
    minimum_level: u16,
    cost: u32,
    base_failure_percent: u8,
    effect: AbilityEffectDefinition,
) -> Arc<rfb_content::ContentCatalog> {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let mut ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.warrens-scare")
        .expect("monster scare ability should exist")
        .clone();
    ability.id = MUTATION_CONTRACT_ABILITY_ID.to_owned();
    ability.name_key = "ability-mutation-contract-name".to_owned();
    ability.description_key = "ability-mutation-contract-description".to_owned();
    ability.target = AbilityTargetDefinition {
        modes: vec![AbilityTargetModeDefinition::SelfTarget],
        range: 0,
        requires_line_of_effect: false,
    };
    ability.effect = effect;
    ability.level_scaling.clear();
    ability.player = None;
    artifact.content.abilities.push(ability);
    artifact
        .content
        .mutations
        .iter_mut()
        .find(|mutation| mutation.id == MUTATION_CONTRACT_ID)
        .expect("Spit Acid mutation should exist")
        .activation = Some(InnatePowerDefinition {
        minimum_level,
        governing_attribute: TechniqueAttribute::Constitution,
        cost,
        cost_scaling: None,
        base_failure_percent,
        minimum_failure_percent: None,
        ability_id: MUTATION_CONTRACT_ABILITY_ID.to_owned(),
    });
    enable_test_caster(&mut artifact.content);
    Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("mutation ability contract content should remain valid"),
    ))
}

fn mutation_ability_game(catalog: Arc<rfb_content::ContentCatalog>, build_id: &str) -> Game {
    let mut game = Game::from_content_with_build(0, catalog, DEFAULT_WORLD_ID, build_id)
        .expect("mutation ability test build should create");
    clear_monsters(&mut game);
    game.progress
        .active_mutation_ids
        .insert(MUTATION_CONTRACT_ID.to_owned());
    game
}

fn mutation_cast_resolution(events: &[DomainEvent]) -> &AbilityCastResolutionDto {
    events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityCastSucceeded { resolution }
            | DomainEvent::AbilityCastFailed { resolution } => Some(resolution),
            _ => None,
        })
        .expect("mutation cast should produce a resolution")
}

fn active_source_mutation_game(seed: u64, suffix: &str, level: u16) -> Game {
    let mut game = test_caster_game(seed);
    clear_monsters(&mut game);
    game.progress.level = level;
    game.progress.max_level = level;
    game.refresh_character_skills();
    game.debug_set_ability_casts_succeed(true);
    assert!(game.gain_mutation(&format!("rfb.mutation.{suffix}"), &mut Vec::new()));
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana");
    mana.current = mana.maximum;
    game
}

fn place_test_ground_item(game: &mut Game, id: &str, kind_id: &str, position: Position) {
    give_inventory_item(game, id, kind_id);
    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .expect("test ground item should exist")
        .location = ItemLocation::Ground(position);
}

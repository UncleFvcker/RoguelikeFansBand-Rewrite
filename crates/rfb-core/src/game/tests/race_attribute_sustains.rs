// SPDX-License-Identifier: MPL-2.0

use super::*;

fn constitution_sustain_game(seed: u64) -> Game {
    static CONTENT: OnceLock<Arc<rfb_content::ContentCatalog>> = OnceLock::new();
    let content = CONTENT
        .get_or_init(|| {
            let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(std::path::Path::parent)
                .expect("core crate should be inside the workspace")
                .join("packs/rfb-demo-original");
            let mut artifact =
                rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
            artifact
                .content
                .races
                .iter_mut()
                .find(|race| race.id == "demo.race.rfb-human")
                .expect("Human race should exist")
                .attribute_sustains
                .insert(ItemAttributeDefinition::Constitution);
            Arc::new(rfb_content::ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content)
                    .expect("race sustain test content should remain valid"),
            ))
        })
        .clone();
    Game::from_content_with_build(seed, content, DEFAULT_WORLD_ID, "demo.build.warrior")
        .expect("race sustain test game should create")
}

fn seed_matching(mut predicate: impl FnMut(&mut RfbRng) -> bool) -> u64 {
    (0..1_000_000)
        .find(|seed| predicate(&mut RfbRng::seeded(*seed)))
        .expect("bounded deterministic seed search should find a match")
}

#[test]
fn race_and_equipment_attribute_sustains_use_the_effective_race() {
    let mut game = constitution_sustain_game(0);
    assert!(game.player_sustains_attribute(AttributeKind::Constitution));
    assert!(!game.player_sustains_attribute(AttributeKind::Strength));

    let mut polymorph =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.setup").status;
    polymorph.granted_race_id = Some("rfb-legacy.race.small-kobold".to_owned());
    game.player.statuses.push(polymorph);
    assert!(!game.player_sustains_attribute(AttributeKind::Constitution));
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert!(game.player_sustains_attribute(AttributeKind::Constitution));

    let mut equipped = Game::new_with_build(0, "demo.build.warrior")
        .expect("equipment sustain test game should create");
    equipped
        .items
        .iter_mut()
        .find(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .expect("Warrior should begin with equipment")
        .kind_id = "demo.item.warding-band".to_owned();
    assert!(equipped.player_sustains_attribute(AttributeKind::Strength));
}

#[test]
fn race_sustain_guards_every_attribute_drain_entry_point_without_extra_rng() {
    let base = constitution_sustain_game(7);
    let constitution_before = base.progress.attributes.constitution;

    let mut item = base.clone();
    let rng_before = item.rng.clone();
    let mut events = Vec::new();
    assert!(item.resolve_item_drain_attribute(
        "demo.item.frailty-tonic",
        AttributeKind::Constitution,
        &mut events,
    ));
    assert_eq!(item.progress.attributes.constitution, constitution_before);
    assert_eq!(item.rng, rng_before);
    assert!(
        item.item_knowledge
            .get("demo.item.frailty-tonic")
            .is_some_and(|knowledge| knowledge.aware)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::ItemAttributeChanged {
            change: ItemAttributeChange::Sustained,
            ..
        }
    )));

    let mut melee = base.clone();
    let rng_before = melee.rng.clone();
    melee.resolve_monster_attribute_drain(AttributeKind::Constitution);
    assert_eq!(melee.progress.attributes.constitution, constitution_before);
    assert_eq!(melee.rng, rng_before);

    let mut eldritch = base.clone();
    let rng_before = eldritch.rng.clone();
    assert!(!eldritch.drain_eldritch_attribute(AttributeKind::Constitution));
    assert_eq!(
        eldritch.progress.attributes.constitution,
        constitution_before
    );
    assert_eq!(eldritch.rng, rng_before);

    let mut patron = base.clone();
    let rng_before = patron.rng.clone();
    patron.drain_patron_attribute(AttributeKind::Constitution, 10, true);
    assert_eq!(patron.progress.attributes.constitution, constitution_before);
    assert_eq!(patron.rng, rng_before);

    let mut wasting = base;
    wasting.progress.active_mutation_ids.clear();
    wasting
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.wasting".to_owned());
    let seed = seed_matching(|rng| rng.bounded(3_000) == 0 && rng.bounded(6) == 4);
    wasting.rng = RfbRng::seeded(seed);
    let mut expected_rng = wasting.rng.clone();
    expected_rng.bounded(3_000);
    expected_rng.bounded(6);
    wasting
        .process_periodic_mutations(
            true,
            false,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Wasting should resolve");
    assert_eq!(
        wasting.progress.attributes.constitution,
        constitution_before
    );
    assert_eq!(wasting.rng, expected_rng);

    let mut unsustained = constitution_sustain_game(8);
    let strength_before = unsustained.progress.attributes.strength;
    assert!(unsustained.resolve_item_drain_attribute(
        "demo.item.frailty-tonic",
        AttributeKind::Strength,
        &mut Vec::new(),
    ));
    assert!(unsustained.progress.attributes.strength < strength_before);
}

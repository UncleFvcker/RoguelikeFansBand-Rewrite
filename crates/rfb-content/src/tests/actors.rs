use std::collections::BTreeSet;

use super::*;

#[test]
fn capture_policies_distinguish_normal_unique_and_immune_monsters() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let policy = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("fixture should contain {id}"))
            .capture_policy
    };
    assert_eq!(
        policy("demo.actor.horse"),
        ActorCapturePolicyDefinition::Normal
    );
    assert_eq!(
        policy("demo.actor.smeagol"),
        ActorCapturePolicyDefinition::PetOnly
    );
    assert_eq!(
        policy("demo.actor.golden-angel"),
        ActorCapturePolicyDefinition::Immune
    );
    assert_eq!(
        policy("demo.actor.serpent-of-chaos"),
        ActorCapturePolicyDefinition::Immune
    );

    let mut invalid = artifact.content.clone();
    invalid
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain a player actor")
        .capture_policy = ActorCapturePolicyDefinition::PetOnly;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidActorStats(_))
    ));
}

#[test]
fn rfb_pet_evolution_relations_use_stable_actor_ids() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let evolution_count = artifact
        .content
        .actors
        .iter()
        .filter(|actor| actor.evolution.is_some())
        .count();
    assert_eq!(evolution_count, 322);
    let horse = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.horse")
        .expect("Horse should be imported");
    let evolution = horse.evolution.as_ref().expect("Horse should evolve");
    assert_eq!(evolution.required_experience, 70);
    assert_eq!(evolution.next_actor_kind_id, "demo.actor.unruly-horse");
}

#[test]
fn p11_actor_facts_remain_explicit_and_narrow() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = |id: &str| {
        artifact
            .content
            .actors
            .iter()
            .find(|actor| actor.id == id)
            .unwrap_or_else(|| panic!("fixture should contain {id}"))
    };

    assert!(actor("demo.actor.blubbering-icky-thing").kills_weaker_bodies);
    assert!(actor("demo.actor.giant-slug").kills_weaker_bodies);
    assert!(actor("demo.actor.novice-archaeologist").ranged_melee);
    assert!(actor("demo.actor.creeping-silver-coins").made_of_silver);
    for id in [
        "demo.actor.horse",
        "demo.actor.unruly-horse",
        "demo.actor.sheep",
        "demo.actor.chiokovo",
    ] {
        assert!(actor(id).rideable, "{id} should retain RIDING");
    }
}

#[test]
fn p61_nazgul_has_a_five_instance_lifetime_limit() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let nazgul = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.nazgul")
        .expect("P61 should import the Nazgul");

    assert_eq!(nazgul.lifetime_instance_limit, Some(5));
    assert_eq!(nazgul.finite_lifetime_instance_limit(), Some(5));
    assert!(nazgul.tags.iter().any(|tag| tag == "unique"));
}

#[test]
fn p62_lord_of_change_has_two_percent_gated_player_polymorph_claws() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.lord-of-change")
        .expect("P62 should import the Lord of Change");
    assert_eq!(
        actor
            .allocation
            .as_ref()
            .map(|allocation| allocation.legacy_index),
        Some(745)
    );
    let routine = actor
        .melee_routine
        .as_ref()
        .expect("Lord of Change should retain its melee routine");
    assert_eq!(
        routine
            .blows
            .iter()
            .flat_map(|blow| &blow.effects)
            .filter(|effect| matches!(
                effect,
                MeleeBlowEffectDefinition::PolymorphPlayer {
                    chance_percent: Some(20)
                }
            ))
            .count(),
        2
    );
}

#[test]
fn p24_slime_mold_retains_move_body_regeneration_and_existing_spells() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actor = artifact
        .content
        .actors
        .iter()
        .find(|actor| actor.id == "demo.actor.slime-mold")
        .expect("P24 should contain the Slime Mold");

    assert_eq!(
        (actor.level, actor.experience_value, actor.max_hp),
        (12, 10, 9)
    );
    assert!(actor.moves_weaker_bodies);
    assert!(actor.regenerates);
    assert_eq!(actor.movement.modes, vec![ActorMovementMode::Swim]);
    assert_eq!(
        actor
            .monster_casting
            .as_ref()
            .expect("Slime Mold should cast")
            .abilities
            .iter()
            .map(|candidate| candidate.ability_id.as_str())
            .collect::<BTreeSet<_>>(),
        [
            "rfb-legacy.ability.mind-blast-7d7",
            "rfb-legacy.ability.shriek",
        ]
        .into_iter()
        .collect()
    );
    let allocation = actor
        .allocation
        .as_ref()
        .expect("Slime Mold should remain allocatable");
    assert!(allocation.multiplies);
    assert_eq!(allocation.habitats, vec![ActorHabitat::Swamp]);
}

#[test]
fn player_inventory_capacity_is_positive_and_monsters_cannot_declare_one() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    let player = invalid
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Player)
        .expect("fixture should contain a player actor");
    player.inventory_slot_capacity = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidActorInventoryCapacity(_))
    ));

    let mut invalid = artifact.content.clone();
    let monster = invalid
        .actors
        .iter_mut()
        .find(|actor| actor.role == ActorRole::Monster)
        .expect("fixture should contain a monster actor");
    monster.inventory_slot_capacity = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidActorInventoryCapacity(_))
    ));
}

#[test]
fn probabilistic_monster_remains_require_valid_weights_and_corpse_items() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut zero_chance = artifact.content.clone();
    zero_chance
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.small-kobold")
        .and_then(|actor| actor.remains.as_mut())
        .expect("small kobold should define probabilistic remains")
        .chance_denominator = 0;
    assert!(matches!(
        validate_and_normalize(&mut zero_chance),
        Err(ContentError::InvalidActorStats(_))
    ));

    let mut mismatched_weight = artifact.content.clone();
    mismatched_weight
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.small-kobold")
        .and_then(|actor| actor.remains.as_mut())
        .expect("small kobold should define probabilistic remains")
        .skeleton_weight = 0;
    assert!(matches!(
        validate_and_normalize(&mut mismatched_weight),
        Err(ContentError::InvalidActorStats(_))
    ));

    let mut dangling_remains = artifact.content;
    dangling_remains
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.small-kobold")
        .and_then(|actor| actor.remains.as_mut())
        .expect("small kobold should define probabilistic remains")
        .skeleton_item_kind_id = Some("demo.item.missing-remains".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut dangling_remains),
        Err(ContentError::DanglingReference { .. })
    ));
}

use super::*;

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

#[test]
fn melee_routines_require_monsters_and_valid_blow_profiles() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    let hound = invalid
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-hound")
        .expect("fixture should contain the echo hound");
    hound.role = ActorRole::Player;
    hound.experience_value = 0;
    hound.inventory_slot_capacity = 26;
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
    let blow = hound
        .melee_routine
        .as_mut()
        .expect("hound should have a melee routine")
        .blows
        .first_mut()
        .expect("hound should have a melee blow");
    let MeleeBlowEffectDefinition::Damage { damage_dice, .. } = blow
        .effects
        .first_mut()
        .expect("hound should have a damage effect")
    else {
        panic!("hound's first melee effect should deal damage");
    };
    *damage_dice = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidMeleeRoutine(_))
    ));
}

#[test]
fn monster_casting_requires_weighted_supported_abilities() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid_frequency = artifact.content.clone();
    invalid_frequency
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast")
        .frequency_percent = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_frequency),
        Err(ContentError::InvalidMonsterCasting(_))
    ));

    let mut invalid_tactics = artifact.content.clone();
    let casting = invalid_tactics
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast");
    casting.preferred_distance = Some(1);
    casting.flee_hp_percent = 100;
    assert!(matches!(
        validate_and_normalize(&mut invalid_tactics),
        Err(ContentError::InvalidMonsterCasting(_))
    ));

    let mut duplicate_ability = artifact.content.clone();
    let casting = duplicate_ability
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast");
    casting.abilities.push(casting.abilities[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut duplicate_ability),
        Err(ContentError::InvalidMonsterCasting(_))
    ));

    let mut dangling_ability = artifact.content.clone();
    dangling_ability
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast")
        .abilities[0]
        .ability_id = "demo.ability.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut dangling_ability),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut unsupported_ability = artifact.content;
    unsupported_ability
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.echo-cantor")
        .expect("fixture should contain the echo cantor")
        .monster_casting
        .as_mut()
        .expect("echo cantor should cast")
        .abilities[0]
        .ability_id = "demo.ability.echo-step".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut unsupported_ability),
        Err(ContentError::InvalidMonsterCasting(_))
    ));
}

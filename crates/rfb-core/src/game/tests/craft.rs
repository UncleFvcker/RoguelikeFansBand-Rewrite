// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn caster() -> Game {
    let mut game = Game::new_with_build(415, "demo.build.high-mage-craft").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.progress.level = 50;
    game.progress.max_level = 50;
    game.progress.experience = game.experience_required_for_level(50);
    game.progress.maximum_experience = game.progress.experience;
    game.refresh_character_skills();
    choose_human_talent_if_pending(&mut game);
    game.refresh_player_resource_maxima();
    game.debug_set_ability_casts_succeed(true);
    game
}

fn learn(game: &mut Game, slug: &str) -> String {
    let id = format!("demo.ability.craft-{slug}");
    let kind = game
        .content
        .item_definitions()
        .find(|item| {
            item.ability_book_id
                .as_ref()
                .and_then(|id| game.content.ability_book(id))
                .is_some_and(|book| book.ability_ids.contains(&id))
        })
        .unwrap()
        .id
        .clone();
    let item_id = format!("test.book.{slug}");
    give_inventory_item(game, &item_id, &kind);
    game.study_player_ability(&item_id, &id).unwrap();
    game.ability_progress.get_mut(&id).unwrap().proficiency = SPELL_EXP_MASTER;
    id
}

fn cast(game: &mut Game, id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    for resource in game.resources.values_mut() {
        resource.current = resource.maximum;
    }
    let mut events = Vec::new();
    game.resolve_player_ability(
        id,
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. })),
        "{id}: {events:#?}"
    );
    events
}

fn item(id: &str) -> TargetSelection {
    TargetSelection::Item { item_id: id.into() }
}

#[test]
fn generated_craft_book_is_counted_studied_cast_and_restored() {
    let newborn = Game::new_with_build(415, "demo.build.high-mage-craft").unwrap();
    let abilities: Vec<_> = newborn
        .snapshot()
        .player
        .abilities
        .into_iter()
        .filter(|a| a.source == rfb_protocol::AbilitySourceDto::Learned)
        .collect();
    assert_eq!(abilities.len(), 32);
    assert!(
        abilities
            .iter()
            .all(|a| a.id.starts_with("demo.ability.craft-"))
    );
    assert!(
        newborn
            .items
            .iter()
            .any(|i| i.kind_id == "demo.item.handbook-for-pupils")
    );
    let mut game = caster();
    let context = LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth: 60,
        source: LootSource::ItemUse {
            item_id: "test.acquirement".into(),
        },
    };
    let draft = (0..512)
        .find_map(|_| {
            game.generate_one_loot_draft(&context, ItemGenerationMode::TailoredGreat)
                .filter(|d| d.kind_id == "demo.item.note-of-acting-master")
        })
        .expect("Craft advanced book must be reachable through full tailored allocation");
    let generated = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = generated.id.clone();
    game.items.push(generated);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    assert_eq!(
        game.item_knowledge["demo.item.note-of-acting-master"].found_count,
        1
    );
    let ability = "demo.ability.craft-telepathy".to_owned();
    game.study_player_ability(&id, &ability).unwrap();
    cast(&mut game, &ability, TargetSelection::SelfTarget);
    assert!(game.player_has_status_kind(STATUS_TELEPATHY));
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(restored.learned_abilities.contains(&ability));
}

#[test]
fn enchantment_caps_bonuses_and_rejects_forbidden_targets_before_payment() {
    let mut game = caster();
    let id = learn(&mut game, "enchantment");
    give_inventory_item(&mut game, "test.weapon", "demo.item.dagger");
    let target = game
        .items
        .iter_mut()
        .find(|i| i.id == "test.weapon")
        .unwrap();
    target.enchantments.to_hit = 14;
    target.enchantments.to_damage = 13;
    cast(&mut game, &id, item("test.weapon"));
    let target = game.items.iter().find(|i| i.id == "test.weapon").unwrap();
    assert_eq!(
        (target.enchantments.to_hit, target.enchantments.to_damage),
        (15, 15)
    );
    assert_eq!(target.discount_percent, 99);
    let events = cast(&mut game, &id, item("test.weapon"));
    assert!(events.iter().any(|e| matches!(e, DomainEvent::AbilityEffectsResolved { resolution, .. } if matches!(resolution.effects[0], rfb_protocol::AbilityEffectResolutionDto::ItemMagic { succeeded: false, .. }))));
    give_inventory_item(&mut game, "test.needle", "demo.item.poison-needle");
    let before = (game.rng.clone(), game.resources.clone(), game.items.clone());
    game.resolve_player_ability(
        &id,
        item("test.needle"),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        (game.rng.clone(), game.resources.clone(), game.items.clone()),
        before
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn polish_shield_grants_real_reflection_and_repeated_polish_fails() {
    let mut game = caster();
    let id = learn(&mut game, "polish-shield");
    let kind = game
        .content
        .item_definitions()
        .find(|i| i.rfb_base_kind.is_some_and(|b| b.tval == 34 && b.sval == 2))
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.shield", &kind);
    cast(&mut game, &id, item("test.shield"));
    let shield = game.items.iter().find(|i| i.id == "test.shield").unwrap();
    assert_eq!(shield.affix_ids, ["rfb-legacy.affix.reflection"]);
    assert_eq!(shield.discount_percent, 99);
    game.equip_inventory_item("test.shield", None).unwrap();
    assert!(game.player_reflects_bolts());
    let virtue = game.virtue_current(VirtueKindDto::Enchantment);
    cast(&mut game, &id, item("test.shield"));
    assert_eq!(game.virtue_current(VirtueKindDto::Enchantment), virtue - 2);
    assert!(
        Game::from_save(game.to_save())
            .unwrap()
            .player_reflects_bolts()
    );
}

#[test]
fn mundanity_converts_fixed_artifact_preserves_ledger_and_erases_device_magic() {
    let mut game = caster();
    let id = learn(&mut game, "mundanity");
    give_inventory_item(&mut game, "test.fixed", "demo.item.pippin");
    game.generated_artifact_ids
        .insert("demo.item.pippin".into());
    let ledger = game.generated_artifact_ids.clone();
    cast(&mut game, &id, item("test.fixed"));
    let mundane = game.items.iter().find(|i| i.id == "test.fixed").unwrap();
    assert_eq!(mundane.kind_id, "demo.item.leather-gloves");
    assert!(!mundane.is_artifact(&game.content));
    assert_eq!(game.generated_artifact_ids, ledger);
    give_inventory_item(&mut game, "test.wand", "demo.item.magic-missile-wand");
    cast(&mut game, &id, item("test.wand"));
    let mundane = game.items.iter().find(|i| i.id == "test.wand").unwrap();
    assert!(mundane.activation.is_none() && mundane.charges.is_none());
    assert!(
        game.inventory_item_use_context("test.wand")
            .unwrap()
            .is_none()
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .inventory_item_use_context("test.wand")
            .unwrap()
            .is_none()
    );
}

#[test]
fn mundane_dragon_confirmation_and_crafted_ego_rejection_are_atomic() {
    let mut game = caster();
    let id = learn(&mut game, "mundanity");
    let kind = game
        .content
        .item_definitions()
        .find(|i| i.rfb_base_kind.is_some_and(|b| (b.tval, b.sval) == (32, 8)))
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.dragon", &kind);
    let ability = game.content.ability(&id).unwrap();
    assert!(
        game.ability_target_plan(ability, &item("test.dragon"))
            .is_none()
    );
    let options = game.craft_ability_item_targets(ability).unwrap();
    let option = options.iter().find(|o| o.item_id == "test.dragon").unwrap();
    assert_eq!(
        option.confirmation_key.as_deref(),
        Some("item-mundanity-resistance-confirm")
    );
    cast(&mut game, &id, option.target.clone());
    give_inventory_item(&mut game, "test.ego", "demo.item.dagger");
    let target = game.items.iter_mut().find(|i| i.id == "test.ego").unwrap();
    target.affix_ids.push("rfb-legacy.affix.slaying".into());
    target.discount_percent = 99;
    target.origin_kind = Some(ItemOriginKindDto::PlayerMade);
    assert!(
        game.ability_target_plan(game.content.ability(&id).unwrap(), &item("test.ego"))
            .is_none()
    );
}

#[test]
fn elemental_choices_are_level_gated_replace_previous_element_and_survive_save() {
    let mut game = caster();
    let brand = learn(&mut game, "elemental-brand");
    let immunity = learn(&mut game, "elemental-immunity");
    let definition = game.content.ability(&brand).unwrap().clone();
    game.progress.level = 35;
    assert_eq!(
        game.ability_element_targets(&definition),
        [
            DamageTypeDto::Fire,
            DamageTypeDto::Cold,
            DamageTypeDto::Poison
        ]
    );
    assert!(
        game.ability_target_plan(
            &definition,
            &TargetSelection::Element {
                element: DamageTypeDto::Acid
            }
        )
        .is_none()
    );
    game.progress.level = 50;
    cast(
        &mut game,
        &brand,
        TargetSelection::Element {
            element: DamageTypeDto::Fire,
        },
    );
    cast(
        &mut game,
        &brand,
        TargetSelection::Element {
            element: DamageTypeDto::Cold,
        },
    );
    let status = game
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == "rfb.status.elemental-brand")
        .unwrap();
    assert_eq!(status.granted_brands, BTreeSet::from([WeaponBrand::Cold]));
    assert!((26..=50).contains(&status.remaining_ticks));
    cast(
        &mut game,
        &immunity,
        TargetSelection::Element {
            element: DamageTypeDto::Fire,
        },
    );
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Immune
    );
    cast(
        &mut game,
        &immunity,
        TargetSelection::Element {
            element: DamageTypeDto::Cold,
        },
    );
    assert_ne!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Immune
    );
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Immune
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn armor_and_weapon_mastery_change_actual_stats_without_stacking_stone_skin() {
    let mut game = caster();
    let armor = learn(&mut game, "magic-armor");
    let stone = learn(&mut game, "stone-skin");
    let mastery = learn(&mut game, "weapon-mastery");
    give_inventory_item(&mut game, "test.weapon", "demo.item.dagger");
    game.equip_inventory_item("test.weapon", None).unwrap();
    let before = game.snapshot().player;
    cast(&mut game, &armor, TargetSelection::SelfTarget);
    let armored = game.snapshot().player;
    assert_eq!(armored.armor_class, before.armor_class + 50);
    assert!(game.player_reflects_bolts() && game.player_levitates());
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Confusion),
        ResistanceLevel::Resistant
    );
    cast(&mut game, &stone, TargetSelection::SelfTarget);
    assert_eq!(game.snapshot().player.armor_class, armored.armor_class);
    let dice = game
        .player_melee_profile(&game.player_derived_stats())
        .damage_dice;
    cast(&mut game, &mastery, TargetSelection::SelfTarget);
    assert_eq!(
        game.player_melee_profile(&game.player_derived_stats())
            .damage_dice,
        dice + 2
    );
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn living_trump_on_surface_grants_usable_mutation_and_restores_it() {
    let mut game = caster();
    let id = learn(&mut game, "living-trump");
    assert_eq!(game.floor_depth(&game.current_floor_id), 0);
    cast(&mut game, &id, TargetSelection::SelfTarget);
    assert!(
        game.progress
            .active_mutation_ids
            .contains("rfb.mutation.teleport")
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|a| a.source == rfb_protocol::AbilitySourceDto::Mutation)
    );
}

#[test]
fn mana_brand_consumes_mana_on_real_weapon_hits_and_elemental_brand_changes_damage() {
    let mut game = caster();
    let mana_brand = learn(&mut game, "mana-brand");
    let element = learn(&mut game, "elemental-brand");
    give_inventory_item(&mut game, "test.weapon", "demo.item.dagger");
    game.equip_inventory_item("test.weapon", None).unwrap();
    game.push_generated_actor(
        "test.target".into(),
        "demo.actor.goblin",
        Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        },
    );
    cast(
        &mut game,
        &element,
        TargetSelection::Element {
            element: DamageTypeDto::Fire,
        },
    );
    let profile = game.player_melee_profile(&game.player_derived_stats());
    let target = &game.entities[0];
    let definition = game.content.actor(&target.kind_id).unwrap();
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, target, definition),
        24
    );
    cast(&mut game, &mana_brand, TargetSelection::SelfTarget);
    let before = game.resources["demo.resource.mana"].current;
    let mut hit = None;
    for seed in 0..100 {
        let mut trial = game.clone();
        trial.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        trial
            .resolve_player_melee(0, false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .unwrap();
        if events
            .iter()
            .any(|e| matches!(e, DomainEvent::PlayerMeleeHit { .. }))
        {
            hit = Some(trial);
            break;
        }
    }
    let hit = hit.expect("fixed seed range includes a weapon hit");
    assert!(hit.resources["demo.resource.mana"].current < before);
    assert_eq!(
        Game::from_save(hit.to_save()).unwrap().state_hash(),
        hit.state_hash()
    );
}

#[test]
fn crafting_uses_common_ego_factory_and_requires_exact_risky_stack_confirmation() {
    let mut game = caster();
    let id = learn(&mut game, "crafting");
    give_inventory_item(&mut game, "test.ammo", "demo.item.arrow");
    game.items
        .iter_mut()
        .find(|i| i.id == "test.ammo")
        .unwrap()
        .quantity = 31;
    let definition = game.content.ability(&id).unwrap().clone();
    assert!(
        game.ability_target_plan(&definition, &item("test.ammo"))
            .is_none()
    );
    assert!(
        game.ability_target_plan(
            &definition,
            &TargetSelection::CraftingItem {
                item_id: "test.ammo".into(),
                quantity: 30
            }
        )
        .is_none()
    );
    cast(
        &mut game,
        &id,
        TargetSelection::CraftingItem {
            item_id: "test.ammo".into(),
            quantity: 31,
        },
    );
    let ammo = game.items.iter().find(|i| i.id == "test.ammo").unwrap();
    assert_eq!(ammo.quantity, 31);
    assert!(!ammo.affix_ids.is_empty());
    assert_eq!(ammo.discount_percent, 99);
    assert_eq!(
        Game::from_save(game.to_save()).unwrap().state_hash(),
        game.state_hash()
    );
}

#[test]
fn curing_removes_only_source_statuses_and_reduces_severe_poison() {
    let mut game = caster();
    for kind in [
        STATUS_FEAR,
        STATUS_POISON,
        STATUS_STUN,
        STATUS_BLEEDING,
        STATUS_HALLUCINATION,
        STATUS_BLINDNESS,
    ] {
        game.player
            .statuses
            .push(monster_combat::melee_status(kind, 900, "test.curing").status);
    }
    let ability = game
        .content
        .ability("demo.ability.craft-curing")
        .unwrap()
        .clone();
    game.resolve_player_ability_effect(
        ability,
        crate::game::abilities::AbilityTargetPlan::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|s| s.kind_id == STATUS_POISON)
            .unwrap()
            .remaining_ticks,
        600
    );
    assert!(game.player_has_status_kind(STATUS_BLINDNESS));
    for kind in [
        STATUS_FEAR,
        STATUS_STUN,
        STATUS_BLEEDING,
        STATUS_HALLUCINATION,
    ] {
        assert!(!game.player_has_status_kind(kind));
    }
}

use std::sync::Arc;

use rfb_content::{AbilityStatusStackingDefinition, MutationPeriodicEffectDefinition};

use super::support::{
    clear_monsters, dispatch_next, game_with_actor_definition, give_inventory_item,
    test_caster_game,
};
use super::*;
use crate::game::hunger::NUTRITION_WEAK;

const EARLY_MUTATION_ID: &str = "rfb.mutation.bers-rage";
const LATE_MUTATION_ID: &str = "rfb.mutation.attract-demon";
const PERIODIC_STATUS_ID: &str = "rfb.status.periodic-contract";

fn periodic_catalog(
    early_trigger_one_in: u32,
    late_trigger_one_in: u32,
) -> Arc<rfb_content::ContentCatalog> {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    for (mutation_id, trigger_one_in, duration_ticks) in [
        (EARLY_MUTATION_ID, early_trigger_one_in, 2),
        (LATE_MUTATION_ID, late_trigger_one_in, 3),
    ] {
        artifact
            .content
            .mutations
            .iter_mut()
            .find(|mutation| mutation.id == mutation_id)
            .expect("periodic contract mutation should exist")
            .periodic_effect = Some(MutationPeriodicEffectDefinition::ApplyStatus {
            trigger_one_in,
            skip_if_present: false,
            status_kind_id: PERIODIC_STATUS_ID.to_owned(),
            intensity: 1,
            duration_ticks,
            duration_dice: 0,
            duration_sides: 0,
            stacking: AbilityStatusStackingDefinition::Replace,
        });
    }
    Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("periodic contract content should remain valid"),
    ))
}

fn periodic_game(early_trigger_one_in: u32, late_trigger_one_in: u32) -> Game {
    let mut game = Game::from_content_with_build(
        0,
        periodic_catalog(early_trigger_one_in, late_trigger_one_in),
        DEFAULT_WORLD_ID,
        "demo.build.warrior",
    )
    .expect("periodic contract game should create");
    game.progress
        .active_mutation_ids
        .extend([EARLY_MUTATION_ID.to_owned(), LATE_MUTATION_ID.to_owned()]);
    game.player
        .statuses
        .retain(|status| status.kind_id != PERIODIC_STATUS_ID);
    game
}

fn m6_game(mutation_id: &str, build_id: &str) -> Game {
    let mut game = Game::new_with_build(0, build_id).expect("M6 test game should create");
    game.progress.active_mutation_ids.clear();
    game.progress
        .active_mutation_ids
        .insert(mutation_id.to_owned());
    game
}

fn polymorph_game(random_candidate_ids: &[&str], active_ids: &[&str], locked_ids: &[&str]) -> Game {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    for mutation in &mut artifact.content.mutations {
        mutation.random_selection_enabled = random_candidate_ids.contains(&mutation.id.as_str());
    }
    let content = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("polymorph contract content should remain valid"),
    ));
    let mut game =
        Game::from_content_with_build(0, content, DEFAULT_WORLD_ID, "demo.build.warrior")
            .expect("polymorph contract game should create");
    game.progress.active_mutation_ids = active_ids.iter().map(|id| (*id).to_owned()).collect();
    game.progress.locked_mutation_ids = locked_ids.iter().map(|id| (*id).to_owned()).collect();
    game
}

fn seed_matching(mut predicate: impl FnMut(&mut RfbRng) -> bool) -> u64 {
    (0..u64::MAX)
        .find(|seed| predicate(&mut RfbRng::seeded(*seed)))
        .expect("a matching deterministic seed should exist")
}

#[test]
fn demigod_passives_apply_level_hp_spell_power_and_attribute_costs() {
    let mut baseline = m6_game("rfb.mutation.fast-learner", "demo.build.high-mage-death");
    baseline.progress.active_mutation_ids.clear();
    baseline.progress.level = 20;
    let baseline_max_hp = baseline.effective_player_max_hp();
    let baseline_attributes = baseline.effective_player_attributes();

    let mut unyielding = baseline.clone();
    unyielding
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.unyielding".to_owned());
    assert_eq!(unyielding.effective_player_max_hp(), baseline_max_hp + 20);

    let mut fell = baseline;
    fell.progress
        .active_mutation_ids
        .insert("rfb.mutation.fell-sorcery".to_owned());
    let attributes = fell.effective_player_attributes();
    assert_eq!(fell.effective_player_spell_power_bonus(), 2);
    assert_eq!(attributes.strength, baseline_attributes.strength - 1);
    assert_eq!(attributes.dexterity, baseline_attributes.dexterity - 1);
    assert_eq!(
        attributes.constitution,
        baseline_attributes.constitution - 1
    );
}

#[test]
fn demigod_passives_scale_player_healing_and_potion_energy() {
    const POTION_ID: &str = "test.item.demigod-potion";
    let mut game = m6_game("rfb.mutation.sacred-vitality", "demo.build.warrior");
    game.player.hp = 1;
    let outcome = game.apply_player_healing(10);
    assert_eq!((outcome.requested, outcome.applied), (12, 12));

    give_inventory_item(&mut game, POTION_ID, "demo.item.water-potion");
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.potion-chugger".to_owned());
    assert_eq!(
        game.player_mutation_action_energy_cost(
            &GameAction::UseItem {
                item_id: POTION_ID.to_owned(),
                target: None,
                target_glyph: None,
            },
            STANDARD_ACTION_COST,
        ),
        STANDARD_ACTION_COST / 2
    );
}

#[test]
fn weapon_skills_raises_every_available_weapon_cap_to_master() {
    let mut game = m6_game("rfb.mutation.fast-learner", "demo.build.high-mage-death");
    game.progress.active_mutation_ids.clear();
    let limited = game
        .player_weapon_proficiencies()
        .into_iter()
        .find(|proficiency| proficiency.maximum < 8_000)
        .expect("High-Mage should have a non-master weapon cap");

    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.weapon-skills".to_owned());
    let promoted = game
        .player_weapon_proficiencies()
        .into_iter()
        .find(|proficiency| proficiency.item_kind_id == limited.item_kind_id)
        .expect("the selected base weapon should remain projected");
    assert_eq!(promoted.maximum, 8_000);
}

#[test]
fn infernal_deal_recovers_hp_or_hp_and_casting_resource_on_visible_death() {
    fn kill_wolf(game: &mut Game) {
        clear_monsters(game);
        let actor = game.generated_actor(
            "test.infernal-deal-wolf".to_owned(),
            "demo.actor.wolf",
            game.player.position,
        );
        game.entities.push(actor);
        game.resolve_actor_death_without_credit(
            0,
            DomainEvent::Waited,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("visible hostile death should resolve");
    }

    let mut warrior = m6_game("rfb.mutation.infernal-deal", "demo.build.warrior");
    warrior.player.hp = 1;
    kill_wolf(&mut warrior);
    assert_eq!(warrior.player.hp, 1 + 10 * 2 / 3);

    let mut caster = m6_game("rfb.mutation.infernal-deal", "demo.build.high-mage-death");
    caster.player.hp = 1;
    let mana = caster
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have Mana");
    mana.current = 0;
    mana.maximum = 100;
    kill_wolf(&mut caster);
    assert_eq!(caster.player.hp, 1 + 10 * 4 / 9);
    assert_eq!(caster.resources["demo.resource.mana"].current, 10 * 2 / 9);
}

#[test]
fn human_strength_stops_later_criticals_and_adds_one_fifth_action_energy() {
    let mut game = m6_game(HUMAN_STR_MUTATION_ID, "demo.build.warrior");
    game.player.energy_need = 0;
    let mut allow_criticals = true;

    let first = game.roll_player_melee_critical_multiplier(10_000, 0, &mut allow_criticals);
    let draws_after_first = game.rng_draw_counter();
    let second = game.roll_player_melee_critical_multiplier(10_000, 0, &mut allow_criticals);

    assert!(first > 100);
    assert_eq!(second, 100);
    assert!(!allow_criticals);
    assert_eq!(game.player.energy_need, STANDARD_ACTION_COST / 5);
    assert_eq!(game.rng_draw_counter(), draws_after_first);
}

#[test]
fn human_dexterity_sprain_applies_one_speed_penalty_and_still_rolls_while_slow() {
    let mut game = m6_game(HUMAN_DEX_MUTATION_ID, "demo.build.warrior");
    let speed_before = game.player_derived_stats().speed.value;
    let mut events = Vec::new();

    game.check_human_dexterity_sprain(1, &mut events);
    let slow = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_SLOW)
        .expect("a forced sprain should slow the player")
        .clone();
    assert!((51..=100).contains(&slow.remaining_ticks));
    assert_eq!(game.player_derived_stats().speed.value, speed_before - 10);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationPeriodicTriggered { mutation_id, .. }
            if mutation_id == HUMAN_DEX_MUTATION_ID
    )));

    let draws_before = game.rng_draw_counter();
    game.check_human_dexterity_sprain(1, &mut events);
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_SLOW)
            .unwrap()
            .remaining_ticks,
        slow.remaining_ticks
    );
}

#[test]
fn human_constitution_only_rolls_for_unwell_when_the_status_is_absent() {
    let mut game = m6_game("rfb.mutation.human-con", "demo.build.warrior");
    let seed = seed_matching(|rng| rng.bounded(200) == 0);
    game.rng = RfbRng::seeded(seed);
    game.process_periodic_mutations(
        true,
        false,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Human illness should process");
    let unwell = game
        .player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == STATUS_UNWELL)
        .expect("the forced Human illness roll should apply unwell");
    assert_eq!(unwell.remaining_ticks, 50);
    unwell.remaining_ticks = 100;

    let draws_before = game.rng_draw_counter();
    game.process_periodic_mutations(
        true,
        false,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("an existing illness should skip its mutation roll");
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn human_intelligence_only_reduces_fear_checks() {
    let base = m6_game(HUMAN_INT_MUTATION_ID, "demo.build.warrior");
    let difficulty = u32::try_from(base.player_derived_stats().saving_throw_skill.value.max(1))
        .expect("saving throw skill must fit u32");
    let seed = (0..4_096)
        .find(|seed| {
            let mut ordinary = base.clone();
            ordinary.rng = RfbRng::seeded(*seed);
            let ordinary_saved =
                ordinary.monster_saving_throw("demo.actor.fearmaster", difficulty, &mut Vec::new());
            let mut fear = base.clone();
            fear.rng = RfbRng::seeded(*seed);
            let fear_saved = fear.monster_fear_saving_throw(
                "demo.actor.fearmaster",
                difficulty,
                &mut Vec::new(),
            );
            ordinary_saved && !fear_saved
        })
        .expect("the ten-point Human fear penalty should change a deterministic check");

    let mut ordinary = base.clone();
    ordinary.rng = RfbRng::seeded(seed);
    assert!(ordinary.monster_saving_throw("demo.actor.fearmaster", difficulty, &mut Vec::new()));
    let mut fear = base;
    fear.rng = RfbRng::seeded(seed);
    assert!(!fear.monster_fear_saving_throw("demo.actor.fearmaster", difficulty, &mut Vec::new()));
}

#[test]
fn human_charisma_applies_skill_spell_and_forced_hit_penalties() {
    let mut normal = m6_game(HUMAN_CHR_MUTATION_ID, "demo.build.high-mage-death");
    normal.progress.active_mutation_ids.clear();
    let careless = m6_game(HUMAN_CHR_MUTATION_ID, "demo.build.high-mage-death");
    let normal_stats = normal.player_derived_stats();
    let careless_stats = careless.player_derived_stats();
    assert_eq!(
        careless_stats.device_skill.value,
        normal_stats.device_skill.value - 10
    );
    assert_eq!(
        careless_stats.melee_skill.value,
        normal_stats.melee_skill.value - 16
    );
    assert_eq!(
        careless_stats.ranged_skill.value,
        normal_stats.ranged_skill.value - 10
    );
    assert_eq!(careless.player_spell_failure_minimum_percent(), 1);

    let failure_for = |game: &Game| {
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == "demo.ability.death-detect-unlife")
            .expect("the first Death spell should be projected")
            .failure_percent
    };
    assert_eq!(failure_for(&careless), failure_for(&normal) + 10);

    let seed = seed_matching(|rng| rng.bounded(100) >= 10 && rng.bounded(20) == 0);
    let mut forced = careless;
    forced.rng = RfbRng::seeded(seed);
    let mut stats = DerivedStatsPipeline::new();
    stats.add(StatKind::MeleeSkill, StatLayer::Base, "test", 100);
    stats.add(StatKind::ArmorClass, StatLayer::Base, "test", 0);
    let result = forced.resolve_player_hit_check(CheckContext {
        kind: CheckKind::MeleeHit,
        actor_id: forced.player.id.clone(),
        target_id: Some("test.target".to_owned()),
        ability: stats.resolve(StatKind::MeleeSkill, StatBounds::NON_NEGATIVE),
        difficulty: stats.resolve(StatKind::ArmorClass, StatBounds::NON_NEGATIVE),
    });
    assert!(!result.succeeded());
    assert_eq!(result.contest_roll, None);
}

#[test]
fn chaos_gift_assigns_and_persists_one_authoritative_patron() {
    let game = Game::new(42);
    let patron_id = game
        .chaos_patron_id
        .clone()
        .expect("new characters should receive a chaos patron");
    assert_eq!(chaos_patron::chaos_patrons(&game.content).len(), 16);
    assert!(
        game.chaos_patron()
            .is_some_and(|patron| patron.id == patron_id)
    );

    let restored = Game::from_save(game.to_save()).expect("chaos patron should reload");
    assert_eq!(
        restored.chaos_patron_id.as_deref(),
        Some(patron_id.as_str())
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn chaos_gift_rewards_only_a_new_highest_level() {
    let mut game = m6_game(chaos_patron::CHAOS_GIFT_MUTATION_ID, "demo.build.warrior");
    clear_monsters(&mut game);
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(6) == 0));
    let mut events = Vec::new();
    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(2),
        &mut events,
    );
    let mut event_cursor = 0;
    game.process_chaos_patron_level_rewards(
        &mut events,
        &mut event_cursor,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("new maximum reward should resolve");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::ChaosPatronReward { .. }))
            .count(),
        1
    );

    let mut regained = Game::new(43);
    regained.progress.active_mutation_ids.clear();
    let level_two = crate::stats::experience_required_for_level(2);
    regained.apply_unscaled_player_experience(level_two, &mut Vec::new());
    regained.apply_player_experience_drain(level_two, "test", &mut Vec::new());
    regained
        .progress
        .active_mutation_ids
        .insert(chaos_patron::CHAOS_GIFT_MUTATION_ID.to_owned());
    let mut events = Vec::new();
    regained.apply_unscaled_player_experience(level_two, &mut events);
    let mut event_cursor = 0;
    regained
        .process_chaos_patron_level_rewards(
            &mut events,
            &mut event_cursor,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("regained level should remain valid");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::ChaosPatronReward { .. }))
    );
}

#[test]
fn chaos_weapon_table_preserves_original_level_bands() {
    assert_eq!(chaos_patron::chaos_weapon_kind_id(1), "demo.item.dagger");
    assert_eq!(chaos_patron::chaos_weapon_kind_id(30), "demo.item.falchion");
    assert_eq!(
        chaos_patron::chaos_weapon_kind_id(38),
        "demo.item.falcon-sword"
    );
    assert_eq!(
        chaos_patron::chaos_weapon_kind_id(39),
        "demo.item.blade-of-chaos"
    );
}

fn process_m6(game: &mut Game) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.process_periodic_mutations(
        true,
        false,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("M6 periodic mutation should resolve");
    events
}

#[test]
fn m4f_c_luck_bias_adjusts_quality_depth_and_attribute_thresholds() {
    let mut game = m6_game("rfb.mutation.good-luck", "demo.build.warrior");
    assert_eq!(game.player_luck_bias(), LuckBias::Good);
    assert_eq!(game.player_luck_bias().attribute_increase_threshold(16), 70);
    assert_eq!(game.player_luck_bias().attribute_increase_threshold(17), 58);
    let mut locked_luck = polymorph_game(
        &["rfb.mutation.bad-luck"],
        &["rfb.mutation.good-luck"],
        &["rfb.mutation.good-luck"],
    );
    assert_eq!(locked_luck.gain_random_mutation(&mut Vec::new()), None);
    let weights = game
        .content
        .loot_table("demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist")
        .quality_weights
        .clone();
    let raw_weights = weights.iter().map(|entry| entry.weight).collect::<Vec<_>>();
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(100) == 96));
    assert_eq!(
        game.roll_loot_quality(&weights, &raw_weights, rfb_content::ItemQuality::Ordinary,),
        ItemQualityDto::Fine
    );
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(100) == 99));
    assert_eq!(
        game.roll_loot_quality(&weights, &raw_weights, rfb_content::ItemQuality::Ordinary,),
        ItemQualityDto::Exceptional
    );

    game.progress.active_mutation_ids.clear();
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.bad-luck".to_owned());
    assert_eq!(game.player_luck_bias(), LuckBias::Bad);
    assert_eq!(game.player_luck_bias().attribute_increase_threshold(16), 80);
    let weights = game
        .content
        .loot_table("demo.loot-table.warrens-final-reward")
        .expect("static loot table should exist")
        .quality_weights
        .clone();
    let raw_weights = weights.iter().map(|entry| entry.weight).collect::<Vec<_>>();
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(100) == 0));
    assert_eq!(
        game.roll_loot_quality(&weights, &raw_weights, rfb_content::ItemQuality::Ordinary,),
        ItemQualityDto::Ordinary
    );
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(20) != 0));
    assert_eq!(game.luck_adjusted_item_generation_depth(90, false), 75);
}

#[test]
fn rfb_depth_quality_uses_original_thresholds_and_one_draw() {
    let policy = rfb_content::LootQualityPolicyDefinition::RfbDepth {
        good_cap_percent: 75,
        great_cap_percent: 20,
    };
    for (depth, expected) in [(1, (11, 7)), (9, (19, 12)), (15, (25, 16)), (32, (42, 20))] {
        assert_eq!(
            rfb_depth_quality_percentages(policy, depth, false, LuckBias::Neutral),
            expected,
            "depth {depth}"
        );
    }
    assert_eq!(
        rfb_depth_quality_percentages(policy, 15, true, LuckBias::Neutral),
        (55, 16)
    );
    assert_eq!(
        rfb_depth_quality_percentages(policy, 99, false, LuckBias::Neutral),
        (75, 20)
    );
    assert_eq!(
        rfb_depth_quality_percentages(policy, 15, false, LuckBias::Good),
        (30, 18)
    );
    assert_eq!(
        rfb_depth_quality_percentages(policy, 15, false, LuckBias::Bad),
        (25, 15)
    );
    assert!(quality_allows_natural_affix(
        Some(policy),
        ItemQualityDto::Exceptional
    ));
    assert!(!quality_allows_natural_affix(
        Some(policy),
        ItemQualityDto::Fine
    ));
    assert!(quality_allows_natural_affix(None, ItemQualityDto::Fine));

    let mut game = m6_game("rfb.mutation.good-luck", "demo.build.warrior");
    game.progress.active_mutation_ids.clear();
    for (roll, minimum, expected) in [
        (
            399,
            rfb_content::ItemQuality::Ordinary,
            ItemQualityDto::Exceptional,
        ),
        (
            400,
            rfb_content::ItemQuality::Ordinary,
            ItemQualityDto::Fine,
        ),
        (
            2_500,
            rfb_content::ItemQuality::Ordinary,
            ItemQualityDto::Ordinary,
        ),
        (
            1_599,
            rfb_content::ItemQuality::Fine,
            ItemQualityDto::Exceptional,
        ),
        (1_600, rfb_content::ItemQuality::Fine, ItemQualityDto::Fine),
        (
            9_999,
            rfb_content::ItemQuality::Exceptional,
            ItemQualityDto::Exceptional,
        ),
    ] {
        game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(10_000) == roll));
        let draws_before = game.rng_draw_counter();
        assert_eq!(
            game.roll_rfb_depth_loot_quality(policy, 15, false, minimum),
            expected
        );
        assert_eq!(game.rng_draw_counter(), draws_before + 1);
    }
}

#[test]
fn m4f_c_easy_tiring_accumulates_and_recovers_shared_minor_slow() {
    let mut game = m6_game("rfb.mutation.easy-tiring", "demo.build.warrior");
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(16) == 0));
    game.apply_easy_tiring_fatigue(50);
    assert_eq!(game.minor_slow, 1);
    assert_eq!(game.minor_slow_energy, 50);

    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(15) == 0));
    game.apply_easy_tiring_fatigue(50);
    assert_eq!(game.minor_slow, 1);
    assert_eq!(game.minor_slow_energy, 0);

    game.minor_slow_energy = 99;
    game.process_minor_slow_recovery();
    assert_eq!(game.minor_slow, 0);
    assert_eq!(game.minor_slow_energy, 0);

    game.minor_slow = 2;
    game.minor_slow_energy = 41;
    let restored = Game::from_save(game.to_save()).expect("fatigue should reload");
    assert_eq!(restored.minor_slow, 2);
    assert_eq!(restored.minor_slow_energy, 41);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn m4f_c_impotence_penalizes_staffs_and_rods_but_not_wands() {
    let mut game = m6_game("rfb.mutation.impotence", "demo.build.warrior");
    clear_monsters(&mut game);
    for (item_id, kind_id, expected_penalty) in [
        ("test.staff", "demo.item.detect-objects-staff", 10),
        ("test.rod", "demo.item.resonance-rod", 10),
        ("test.wand", "demo.item.magic-missile-wand", 0),
    ] {
        give_inventory_item(&mut game, item_id, kind_id);
        let item = game
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("test device should exist");
        let definition = game
            .content
            .item(kind_id)
            .expect("test device definition should exist");
        let activation = item
            .activation
            .as_ref()
            .expect("test device should activate");
        let effect = &definition
            .device_generation
            .as_ref()
            .expect("test device should generate")
            .activations
            .iter()
            .find(|candidate| candidate.id == activation.profile_id)
            .expect("test activation should exist")
            .effect;
        let base = game.player_derived_stats().device_skill;
        let adjusted = game.apply_impotence_device_skill_modifier(&base, item, definition, effect);
        assert_eq!(adjusted.value, (base.value - expected_penalty).max(0));
    }

    let base = game.player_derived_stats().device_skill;
    let staff = game
        .items
        .iter()
        .find(|item| item.id == "test.staff")
        .expect("test staff should exist");
    let definition = game
        .content
        .item(&staff.kind_id)
        .expect("test staff definition should exist");
    let speed = ItemUseEffectDefinition::ApplySpeed {
        duration_dice: 1,
        duration_sides: 1,
        duration_bonus: 0,
    };
    assert_eq!(
        game.apply_impotence_device_skill_modifier(&base, staff, definition, &speed)
            .value,
        (base.value - 30).max(0)
    );
}

#[test]
fn periodic_mutations_use_source_order_and_exact_trigger_draws() {
    let mut game = periodic_game(1, 1);
    game.rng = RfbRng::seeded(7);
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(1);
    expected_rng.bounded(1);

    game.process_periodic_mutations(
        true,
        false,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("periodic mutations should resolve");

    assert_eq!(game.rng, expected_rng);
    let status = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == PERIODIC_STATUS_ID)
        .expect("both periodic mutations should apply the shared status");
    assert_eq!(status.remaining_ticks, 3);
    assert_eq!(status.source_id.as_deref(), Some(LATE_MUTATION_ID));
}

#[test]
fn periodic_mutations_skip_world_map_without_rng_and_consume_one_draw_on_miss() {
    let mut game = periodic_game(2, 1);
    game.progress.active_mutation_ids.remove(LATE_MUTATION_ID);
    let seed = (0..u64::MAX)
        .find(|seed| RfbRng::seeded(*seed).bounded(2) != 0)
        .expect("a periodic miss seed should exist");
    game.rng = RfbRng::seeded(seed);
    let untouched_rng = game.rng.clone();

    game.process_periodic_mutations(
        false,
        false,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("world map should skip periodic mutations");
    assert_eq!(game.rng, untouched_rng);
    assert!(
        game.player
            .statuses
            .iter()
            .all(|status| status.kind_id != PERIODIC_STATUS_ID)
    );

    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(2);
    game.process_periodic_mutations(
        true,
        false,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("periodic miss should resolve");
    assert_eq!(game.rng, expected_rng);
    assert!(
        game.player
            .statuses
            .iter()
            .all(|status| status.kind_id != PERIODIC_STATUS_ID)
    );
}

#[test]
fn m6_a_berserk_and_invulnerability_reuse_authoritative_status_payloads() {
    let mut berserk = m6_game("rfb.mutation.bers-rage", "demo.build.warrior");
    berserk.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(3_000) == 0));
    process_m6(&mut berserk);
    let rage = berserk
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_BERSERK)
        .expect("Berserk Rage should apply the shared berserk status");
    assert_eq!(rage.granted_modifiers.max_hp, 30);
    assert!(rage.granted_status_immunities.contains(STATUS_FEAR));

    let mut invulnerable = m6_game("rfb.mutation.invuln", "demo.build.warrior");
    invulnerable.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(5_000) == 0));
    process_m6(&mut invulnerable);
    let status = invulnerable
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_INVULNERABILITY)
        .expect("Invulnerability should apply its shared protection status");
    assert!((9..=16).contains(&status.remaining_ticks));
    assert_eq!(status.incoming_damage_percent, 0);
}

#[test]
fn m6_a_speed_flux_minor_slow_round_trips_and_feeds_speed() {
    let mut game = m6_game("rfb.mutation.speed-flux", "demo.build.warrior");
    let speed_before = game.snapshot().player.speed;
    let seed =
        seed_matching(|rng| rng.bounded(6_000) == 0 && rng.bounded(2) == 0 && rng.bounded(2) == 1);
    game.rng = RfbRng::seeded(seed);

    process_m6(&mut game);

    assert_eq!(game.minor_slow, 10);
    assert_eq!(game.snapshot().player.speed, speed_before - 10);
    let restored = Game::from_save(game.to_save()).expect("minor slow should reload");
    assert_eq!(restored.minor_slow, 10);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn m6_a_resource_conversion_and_hypochondria_use_existing_stat_resources() {
    let mut conversion = test_caster_game(0);
    conversion.progress.active_mutation_ids.clear();
    conversion
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.sp-to-hp".to_owned());
    conversion.player.hp = conversion.effective_player_max_hp() - 5;
    let mana = conversion
        .resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana");
    mana.current = 7;
    conversion.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(2_000) == 0));

    process_m6(&mut conversion);

    assert_eq!(conversion.player.hp, conversion.effective_player_max_hp());
    assert_eq!(conversion.resources["demo.resource.mana"].current, 2);

    let mut hypochondria = m6_game("rfb.mutation.hypochondria", "demo.build.warrior");
    let attributes_before = hypochondria.effective_player_attributes();
    hypochondria.rng = RfbRng::seeded(seed_matching(|rng| {
        rng.bounded(1_815) == 0 && rng.bounded(2) == 1
    }));
    process_m6(&mut hypochondria);
    let attributes_after = hypochondria.effective_player_attributes();
    assert_eq!(attributes_after.dexterity, attributes_before.dexterity - 4);
    assert_eq!(
        attributes_after.constitution,
        attributes_before.constitution - 4
    );
    assert!(
        hypochondria
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_UNWELL && status.remaining_ticks == 50)
    );
}

#[test]
fn m6_a_produce_mana_persists_its_prompt_then_resolves_a_directional_ball() {
    let mut game = m6_game("rfb.mutation.prod-mana", "demo.build.warrior");
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(9_000) == 0));
    let events = process_m6(&mut game);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationPeriodicTriggered { mutation_id, .. }
            if mutation_id == "rfb.mutation.prod-mana"
    )));
    assert_eq!(
        game.pending_mutation_direction
            .as_ref()
            .map(|pending| pending.mutation_id.as_str()),
        Some("rfb.mutation.prod-mana")
    );
    let restored = Game::from_save(game.to_save()).expect("pending direction should reload");
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut events = Vec::new();
    let pending = game
        .resolve_pending_mutation_direction(
            Direction::East,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Produce Mana direction should resolve");
    assert_eq!(pending.mutation_id, "rfb.mutation.prod-mana");
    assert!(game.pending_mutation_direction.is_none());
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage { ability_id, resolution, .. }
            if ability_id == "rfb.mutation.prod-mana"
                && resolution.radius == 3
                && resolution.base_raw_damage == 2
    )));
}

#[test]
fn m6_b_random_teleport_and_banish_reuse_existing_displacement_rules() {
    let mut teleport = m6_game("rfb.mutation.teleport-rnd", "demo.build.warrior");
    teleport.rng = RfbRng::seeded(seed_matching(|rng| {
        rng.bounded(33);
        rng.bounded(5_000) == 87
    }));
    let position_before = teleport.player.position;
    process_m6(&mut teleport);
    assert_ne!(teleport.player.position, position_before);
    assert!(chebyshev_distance(position_before, teleport.player.position) <= 40);

    let mut banish = m6_game("rfb.mutation.banish-all-rnd", "demo.build.warrior");
    let target_position = banish
        .open_positions_around(banish.player.position, 1)
        .into_iter()
        .next()
        .expect("the test player should have an adjacent open tile");
    let definition = banish
        .content
        .actor("demo.actor.bandit")
        .cloned()
        .expect("the content should contain the Bandit");
    banish.entities = vec![spawn_actor_from_definition(
        &mut banish.rng,
        &definition,
        "m6-b-banish-target",
        target_position,
        INITIAL_MONSTER_ENERGY_NEED,
        false,
    )];
    banish.reveal_current_visibility();
    banish.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(9_000) == 0));
    process_m6(&mut banish);
    assert_ne!(banish.entities[0].position, target_position);
}

#[test]
fn m6_b_fumbling_deals_one_d_twenty_five_and_drops_a_removable_weapon() {
    let mut game = m6_game("rfb.mutation.fumbling", "demo.build.warrior");
    let weapon_id = game
        .items
        .iter()
        .find(|item| {
            matches!(item.location, ItemLocation::Equipped { .. })
                && item.curse.is_none()
                && game
                    .content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "weapon"))
        })
        .map(|item| item.id.clone())
        .expect("the Warrior should begin with a removable melee weapon");
    let hp_before = game.player.hp;
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(10_000) == 0));

    let events = process_m6(&mut game);

    let damage = hp_before.saturating_sub(game.player.hp);
    assert!((1..=25).contains(&damage));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == weapon_id)
            .map(|item| &item.location),
        Some(&ItemLocation::Ground(game.player.position))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationFumbled {
            dropped_item_kind_id: Some(_),
            ..
        }
    )));
}

#[test]
fn m6_b_shadow_walk_persists_then_only_regenerates_ordinary_procedural_dungeons() {
    let mut surface = m6_game("rfb.mutation.shadow-walk", "demo.build.warrior");
    surface.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(12_000) == 0));
    process_m6(&mut surface);
    assert!((15..=35).contains(&surface.reality_change_ticks));
    let restored = Game::from_save(surface.to_save()).expect("reality countdown should reload");
    assert_eq!(restored.reality_change_ticks, surface.reality_change_ticks);
    assert_eq!(restored.state_hash(), surface.state_hash());

    let wilderness_seed = surface.wilderness_seed;
    let terrain = surface.terrain.clone();
    surface.reality_change_ticks = 1;
    let mut events = Vec::new();
    assert!(
        !surface
            .advance_reality_change(&mut events, &mut BTreeSet::new(), &mut Vec::new())
            .expect("surface reality countdown should resolve")
    );
    assert_eq!(surface.wilderness_seed, wilderness_seed);
    assert_eq!(surface.terrain, terrain);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::RealityChangeResolved { regenerated: false }
    )));

    let mut dungeon = m6_game("rfb.mutation.shadow-walk", "demo.build.warrior");
    let definition = dungeon
        .content
        .world(DEFAULT_WORLD_ID)
        .and_then(|world| {
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        })
        .cloned()
        .expect("Warrens depth one should exist");
    let floor = dungeon
        .generate_procedural_floor(&definition, None)
        .expect("Warrens depth one should generate");
    dungeon.activate_floor(floor, Vec::new());
    let terrain_before = dungeon.terrain.clone();
    let connections_before = dungeon
        .floor_connections
        .iter()
        .map(|connection| {
            (
                connection.id.clone(),
                connection.target_floor_id.clone(),
                connection.target_connection_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    dungeon.reality_change_ticks = 1;
    let mut events = Vec::new();
    assert!(
        dungeon
            .advance_reality_change(&mut events, &mut BTreeSet::new(), &mut Vec::new())
            .expect("procedural reality countdown should resolve")
    );
    assert_eq!(dungeon.current_floor_id, definition.id);
    assert_ne!(dungeon.terrain, terrain_before);
    assert_eq!(
        dungeon
            .floor_connections
            .iter()
            .map(|connection| {
                (
                    connection.id.clone(),
                    connection.target_floor_id.clone(),
                    connection.target_connection_id.clone(),
                )
            })
            .collect::<Vec<_>>(),
        connections_before
    );
}

#[test]
fn m6_c_flatulence_and_raw_chaos_reuse_centered_area_damage() {
    for (mutation_id, trigger_bound, trigger_hit, damage_type, radius) in [
        (
            "rfb.mutation.flatulent",
            3_000,
            12,
            DamageTypeDto::Poison,
            3,
        ),
        ("rfb.mutation.raw-chaos", 8_000, 0, DamageTypeDto::Chaos, 8),
    ] {
        let mut game = m6_game(mutation_id, "demo.build.warrior");
        game.rng = RfbRng::seeded(seed_matching(|rng| {
            rng.bounded(trigger_bound) == trigger_hit
        }));

        let events = process_m6(&mut game);

        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::AbilityAreaDamage { ability_id, resolution, .. }
                if ability_id == mutation_id
                    && resolution.center == game.player.position
                    && resolution.radius == radius
                    && resolution.base_raw_damage == i32::from(game.progress.level)
                    && resolution.damage_type == damage_type
        )));
    }
}

#[test]
fn m6_c_attractions_reuse_category_summons_and_original_friendliness() {
    for (mutation_id, category, trigger_bound, trigger_hit, friendly_one_in, floor_id) in [
        (
            "rfb.mutation.attract-demon",
            "demon",
            6_666,
            665,
            6,
            "demo.floor.warrens-depth-8",
        ),
        (
            "rfb.mutation.attract-animal",
            "animal",
            7_000,
            0,
            3,
            "demo.floor.warrens-depth-1",
        ),
        (
            "rfb.mutation.attract-dragon",
            "dragon",
            3_000,
            0,
            5,
            "demo.floor.warrens-depth-4",
        ),
    ] {
        let mut game = m6_game(mutation_id, "demo.build.warrior");
        game.current_floor_id = floor_id.to_owned();
        game.rng = RfbRng::seeded(seed_matching(|rng| {
            rng.bounded(trigger_bound) == trigger_hit && rng.bounded(friendly_one_in) == 0
        }));

        let events = process_m6(&mut game);
        let resolution = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilitySummoned {
                    ability_id,
                    resolution,
                } if ability_id == mutation_id => Some(resolution),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{mutation_id} should summon its category"));

        assert!(!resolution.hostile);
        assert!(!resolution.entity_ids.is_empty());
        assert!(resolution.summoned_kind_ids.iter().all(|kind_id| {
            game.content
                .actor(kind_id)
                .is_some_and(|definition| definition.tags.iter().any(|tag| tag == category))
        }));
        assert!(resolution.entity_ids.iter().all(|entity_id| {
            game.entities.iter().any(|entity| {
                &entity.id == entity_id
                    && entity.controller_id.as_deref() == Some(game.player.id.as_str())
            })
        }));
    }
}

#[test]
fn m6_c_eat_light_heals_halves_fuel_damages_and_extinguishes_the_area() {
    let mut game = m6_game("rfb.mutation.eat-light", "demo.build.warrior");
    let maximum_hp = game.effective_player_max_hp();
    game.player.hp = maximum_hp - 30;
    let light_id = game
        .items
        .iter_mut()
        .find(|item| item.fuel.is_some())
        .map(|item| {
            item.location = ItemLocation::Equipped {
                slot_id: "light".to_owned(),
            };
            item.fuel
                .as_mut()
                .expect("the selected test light should use fuel")
                .current = 100;
            item.id.clone()
        })
        .expect("the Warrior should begin with a fuelled light");
    let adjacent = game
        .open_positions_around(game.player.position, 1)
        .into_iter()
        .next()
        .expect("the player should have an adjacent open tile");
    let player_index = game
        .index(game.player.position)
        .expect("player is in bounds");
    let adjacent_index = game.index(adjacent).expect("adjacent tile is in bounds");
    game.glow[player_index] = true;
    game.glow[adjacent_index] = true;
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(3_000) == 0));

    let events = process_m6(&mut game);

    assert_eq!(game.player.hp, maximum_hp - 15);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == light_id)
            .and_then(|item| item.fuel)
            .map(|fuel| fuel.current),
        Some(50)
    );
    assert!(!game.glow[player_index]);
    assert!(!game.glow[adjacent_index]);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage { ability_id, resolution, .. }
            if ability_id == "rfb.mutation.eat-light"
                && resolution.radius == 10
                && resolution.base_raw_damage == 50
                && resolution.damage_type == DamageTypeDto::Dark
    )));
}

#[test]
fn m6_d_normality_respects_locks_and_wasting_respects_sustains() {
    let mut normality = m6_game("rfb.mutation.normality", "demo.build.warrior");
    normality
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.pultitis".to_owned());
    normality
        .progress
        .locked_mutation_ids
        .insert("rfb.mutation.normality".to_owned());
    normality.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(5_000) == 0));

    process_m6(&mut normality);

    assert!(
        normality
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.normality")
    );
    assert!(
        !normality
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.pultitis")
    );

    let mut sustained = m6_game("rfb.mutation.wasting", "demo.build.warrior");
    sustained
        .items
        .iter_mut()
        .find(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .expect("the Warrior should begin with equipment")
        .kind_id = "demo.item.warding-band".to_owned();
    let strength_before = sustained.progress.attributes.strength;
    let seed = seed_matching(|rng| rng.bounded(3_000) == 0 && rng.bounded(6) == 0);
    sustained.rng = RfbRng::seeded(seed);
    let mut expected_rng = sustained.rng.clone();
    expected_rng.bounded(3_000);
    expected_rng.bounded(6);

    let events = process_m6(&mut sustained);

    assert_eq!(sustained.progress.attributes.strength, strength_before);
    assert_eq!(sustained.rng, expected_rng);
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationPeriodicTriggered { mutation_id, .. }
            if mutation_id == "rfb.mutation.wasting"
    )));

    let mut wasting = m6_game("rfb.mutation.wasting", "demo.build.warrior");
    let intelligence_before = wasting.progress.attributes.intelligence;
    let maximum_before = wasting.progress.maximum_attributes.intelligence;
    wasting.rng = RfbRng::seeded(seed_matching(|rng| {
        rng.bounded(3_000) == 0 && rng.bounded(6) == 1 && {
            rng.bounded(6);
            rng.bounded(6) != 0
        }
    }));

    process_m6(&mut wasting);

    assert!(wasting.progress.attributes.intelligence < intelligence_before);
    assert_eq!(
        wasting.progress.maximum_attributes.intelligence,
        maximum_before
    );
}

#[test]
fn m6_d_wraithform_and_polymorph_wounds_reuse_shared_status_transactions() {
    let mut wraith = m6_game("rfb.mutation.wraith", "demo.build.warrior");
    wraith.progress.level = 10;
    let seed = seed_matching(|rng| rng.bounded(3_000) == 0);
    wraith.rng = RfbRng::seeded(seed);
    let mut expected_rng = wraith.rng.clone();
    expected_rng.bounded(3_000);
    let expected_duration = u32::try_from(expected_rng.bounded(5) + 1).unwrap() + 5;

    process_m6(&mut wraith);

    let status = wraith
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_WRAITHFORM)
        .expect("Wraithform should apply the shared status");
    assert_eq!(status.remaining_ticks, expected_duration);
    assert!(status.grants_wall_passage);
    assert_eq!(status.incoming_damage_percent, 50);

    let mut wounds = m6_game("rfb.mutation.poly-wound", "demo.build.warrior");
    let maximum_hp = wounds.effective_player_max_hp();
    wounds.player.hp = maximum_hp - 5;
    wounds.player.statuses.push(StatusInstance {
        kind_id: STATUS_BLEEDING.to_owned(),
        intensity: 1,
        remaining_ticks: 10,
        source_id: None,
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let seed = seed_matching(|rng| {
        rng.bounded(3_000) == 0 && {
            rng.bounded(5);
            rng.bounded(5) != 0
        }
    });
    wounds.rng = RfbRng::seeded(seed);
    let mut expected_rng = wounds.rng.clone();
    expected_rng.bounded(3_000);
    let healing = i32::try_from(expected_rng.bounded(5) + 1).unwrap();
    expected_rng.bounded(5);

    process_m6(&mut wounds);

    assert_eq!(wounds.player.hp, (maximum_hp - 5 + healing).min(maximum_hp));
    assert_eq!(
        wounds
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_BLEEDING)
            .map(|status| status.remaining_ticks),
        Some(10_u32.saturating_sub(u32::try_from(healing / 2).unwrap()))
    );
    assert_eq!(wounds.rng, expected_rng);
}

#[test]
fn m6_d_random_telepathy_toggles_and_nausea_sets_the_original_weak_threshold() {
    let mut telepathy = m6_game("rfb.mutation.random-telepathy", "demo.build.warrior");
    telepathy.progress.level = 12;
    let trigger_seed = seed_matching(|rng| rng.bounded(3_000) == 0);
    telepathy.rng = RfbRng::seeded(trigger_seed);

    process_m6(&mut telepathy);

    assert!(telepathy.player_has_telepathy());
    assert!(
        telepathy
            .player
            .statuses
            .iter()
            .any(|status| { status.kind_id == STATUS_TELEPATHY && status.remaining_ticks == 12 })
    );

    telepathy.rng = RfbRng::seeded(trigger_seed);
    process_m6(&mut telepathy);
    assert!(!telepathy.player_has_telepathy());

    let mut nausea = m6_game("rfb.mutation.nausea", "demo.build.warrior");
    nausea.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM;
    nausea.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(9_000) == 0));

    let events = process_m6(&mut nausea);

    assert_eq!(nausea.nutrition, NUTRITION_WEAK);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::NutritionStateChanged { nutrition, .. } if *nutrition == NUTRITION_WEAK
    )));
}

#[test]
fn m6_d_warning_sums_every_living_monster_at_or_above_player_level() {
    let mut game = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.level = 120;
    });
    game.progress.active_mutation_ids.clear();
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.warning".to_owned());
    clear_monsters(&mut game);
    let definition = game
        .content
        .actor("demo.actor.small-kobold")
        .cloned()
        .expect("the warning test actor should exist");
    game.entities.push(spawn_actor_from_definition(
        &mut game.rng,
        &definition,
        "m6-d-warning-target",
        game.player.position,
        INITIAL_MONSTER_ENERGY_NEED,
        false,
    ));
    game.rng = RfbRng::seeded(seed_matching(|rng| rng.bounded(1_000) == 0));

    let events = process_m6(&mut game);

    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationWarning { danger_amount } if *danger_amount == 120
    )));
    let warning = project_events(events)
        .into_iter()
        .find(|event| event.kind == "mutation.warning.extreme")
        .expect("danger above one hundred should project the extreme warning band");
    assert_eq!(warning.message_key, "mutation-warning-extreme");
    assert_eq!(warning.args["danger"], "120");
}

#[test]
fn m7_polymorph_rare_cure_preserves_locks_and_loses_in_source_order() {
    let mut game = polymorph_game(
        &[],
        &[
            "rfb.mutation.pultitis",
            "rfb.mutation.infravision",
            "rfb.mutation.vuln-elem",
        ],
        &["rfb.mutation.infravision"],
    );
    let seed = seed_matching(|rng| rng.bounded(23) == 0);
    game.rng = RfbRng::seeded(seed);
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(23);
    let mut events = Vec::new();

    assert!(game.resolve_polymorph_mutations(&mut events));

    assert_eq!(game.rng, expected_rng);
    assert!(matches!(
        events.first(),
        Some(DomainEvent::MutationAllCured)
    ));
    assert_eq!(
        project_events(events.clone())[0].message_key,
        "mutation-all-cured"
    );
    assert_eq!(
        game.progress.active_mutation_ids,
        BTreeSet::from(["rfb.mutation.infravision".to_owned()])
    );
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::MutationLost { mutation_id, .. } => Some(mutation_id.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        ["rfb.mutation.vuln-elem", "rfb.mutation.pultitis"]
    );
}

#[test]
fn m7_polymorph_empty_candidate_set_terminates_without_rng() {
    let mut game = polymorph_game(
        &[],
        &["rfb.mutation.infravision"],
        &["rfb.mutation.infravision"],
    );
    game.rng = RfbRng::seeded(17);
    let untouched_rng = game.rng.clone();

    assert!(!game.resolve_polymorph_mutations(&mut Vec::new()));

    assert_eq!(game.rng, untouched_rng);
    assert_eq!(
        game.progress.active_mutation_ids,
        BTreeSet::from(["rfb.mutation.infravision".to_owned()])
    );
}

#[test]
fn m7_polymorph_chains_gains_and_orders_conflict_before_gain() {
    let mut game = polymorph_game(
        &["rfb.mutation.infravision", "rfb.mutation.regen"],
        &["rfb.mutation.flesh-rot"],
        &[],
    );
    let seed = seed_matching(|rng| {
        rng.bounded(2) == 0
            && rng.bounded(10) >= 6
            && rng.bounded(2) == 0
            && rng.bounded(2) == 0
            && {
                rng.bounded(6);
                true
            }
            && rng.bounded(2) == 1
    });
    game.rng = RfbRng::seeded(seed);
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(2);
    expected_rng.bounded(10);
    expected_rng.bounded(2);
    expected_rng.bounded(2);
    expected_rng.bounded(6);
    expected_rng.bounded(2);
    let mut events = Vec::new();

    assert!(game.resolve_polymorph_mutations(&mut events));

    assert_eq!(game.rng, expected_rng);
    assert_eq!(
        game.progress.active_mutation_ids,
        BTreeSet::from([
            "rfb.mutation.infravision".to_owned(),
            "rfb.mutation.regen".to_owned(),
        ])
    );
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::MutationLost { mutation_id, .. } => {
                    Some(("lost", mutation_id.as_str()))
                }
                DomainEvent::MutationGained { mutation_id, .. } => {
                    Some(("gained", mutation_id.as_str()))
                }
                _ => None,
            })
            .collect::<Vec<_>>(),
        [
            ("lost", "rfb.mutation.flesh-rot"),
            ("gained", "rfb.mutation.regen"),
            ("gained", "rfb.mutation.infravision"),
        ]
    );
}

#[test]
fn m7_polymorph_loss_threshold_changes_after_five_unlocked_mutations() {
    const IDS: [&str; 6] = [
        "rfb.mutation.normality",
        "rfb.mutation.flesh-rot",
        "rfb.mutation.xtra-eyes",
        "rfb.mutation.infravision",
        "rfb.mutation.regen",
        "rfb.mutation.vuln-elem",
    ];

    for count in [5_usize, 6] {
        let mut game = polymorph_game(&IDS[..count], &IDS[..count], &[]);
        let total = IDS[..count]
            .iter()
            .map(|id| {
                u64::from(
                    game.content
                        .mutation(id)
                        .expect("loss candidate should exist")
                        .random_weight,
                )
            })
            .sum::<u64>();
        let seed = seed_matching(|rng| {
            rng.bounded(23) != 0
                && rng.bounded(2) == 1
                && (count > 5 || rng.bounded(1) == 0)
                && {
                    rng.bounded(total);
                    true
                }
                && rng.bounded(2) == 1
        });
        game.rng = RfbRng::seeded(seed);
        let mut expected_rng = game.rng.clone();
        expected_rng.bounded(23);
        expected_rng.bounded(2);
        if count <= 5 {
            expected_rng.bounded(1);
        }
        expected_rng.bounded(total);
        expected_rng.bounded(2);

        assert!(game.resolve_polymorph_mutations(&mut Vec::new()));

        assert_eq!(game.rng, expected_rng, "count {count} RNG contract");
        assert_eq!(game.progress.active_mutation_ids.len(), count - 1);
    }
}

#[test]
fn m7_polymorph_potion_consumes_and_becomes_aware_after_a_change() {
    const ITEM_ID: &str = "test.item.polymorph.1";
    const KIND_ID: &str = "demo.item.polymorph-potion";
    let mut game = polymorph_game(&["rfb.mutation.infravision"], &[], &[]);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    let seed = seed_matching(|rng| {
        rng.bounded(2) == 0
            && {
                rng.bounded(6);
                true
            }
            && rng.bounded(2) == 1
    });
    game.rng = RfbRng::seeded(seed);

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(
        game.item_knowledge_dto(KIND_ID),
        rfb_protocol::ItemKnowledgeDto::Aware
    );
    assert!(
        game.progress
            .active_mutation_ids
            .contains("rfb.mutation.infravision")
    );
    assert!(update.events.iter().any(|event| {
        event.kind == "mutation.gained"
            && event.args.get("target").map(String::as_str) == Some("rfb.mutation.infravision")
    }));
}

// SPDX-License-Identifier: MPL-2.0
use super::*;

fn same_save(game: &Game, restored: &Game) {
    let a = serde_json::to_value(game.to_save()).unwrap();
    let b = serde_json::to_value(restored.to_save()).unwrap();
    let differences = a
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| b.get(*key) != Some(*value))
        .map(|(key, _)| key.as_str())
        .collect::<Vec<_>>();
    assert!(
        differences.is_empty(),
        "save fields differ: {differences:?}"
    );
}

fn ready(seed: u64, slug: &str) -> (Game, String, String, String) {
    let mut game = Game::new_with_build(seed, BUILD).unwrap();
    clear_monsters(&mut game);
    let id = format!("demo.task.{slug}");
    let task = game
        .content
        .world(&game.world_id)
        .unwrap()
        .tasks
        .iter()
        .find(|task| task.id == id)
        .unwrap()
        .clone();
    let facility = task.source_facility_id.unwrap();
    crate::game::tests::town::enter_town_facility(&mut game, &facility);
    game.mark_shop_visited_at_player().unwrap();
    game.reveal_current_visibility();
    game.task_states.insert(
        id.clone(),
        TaskState {
            status: TaskStatusKindDto::RewardAvailable,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    (game, id, facility, task.reward.unwrap().item_instance_id)
}

#[test]
fn rapier_reward_and_fixed_old_castle_choice_survive_rng_changes_and_loading() {
    let (mut game, id, facility, item_id) = ready(923, "thieves-hideout");
    game.claim_task_reward(&facility, &id).unwrap();
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == item_id)
            .unwrap()
            .kind_id,
        "demo.item.rapier"
    );
    let mut seen = BTreeSet::new();
    for seed in 0..16 {
        let (mut game, id, facility, item_id) = ready(seed, "old-castle");
        let mut changed_rng = game.clone();
        changed_rng.rng.bounded(987);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(
            game.claim_task_reward(&facility, &id),
            restored.claim_task_reward(&facility, &id)
        );
        same_save(&game, &restored);
        changed_rng.claim_task_reward(&facility, &id).unwrap();
        let item = game.items.iter().find(|item| item.id == item_id).unwrap();
        assert_eq!(
            item.kind_id,
            changed_rng
                .items
                .iter()
                .find(|item| item.id == item_id)
                .unwrap()
                .kind_id
        );
        assert!(game.generated_artifact_ids.contains(&item.kind_id));
        assert!(Game::from_save(game.to_save()).is_ok());
        seen.insert(item.kind_id.clone());
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(
        seen,
        BTreeSet::from([
            "demo.item.duelist".to_owned(),
            "demo.item.quickthorn".to_owned()
        ])
    );
}

#[test]
fn existing_old_castle_artifact_becomes_a_named_replacement_without_duplicate_or_failed_claim_rng()
{
    let (mut game, id, facility, item_id) = ready(923, "old-castle");
    game.generated_artifact_ids.extend([
        "demo.item.duelist".to_owned(),
        "demo.item.quickthorn".to_owned(),
    ]);
    let mut full = game.clone();
    while full.inventory_used_slots() < full.inventory_slot_capacity() {
        let id = format!("test.filler.{}", full.items.len());
        give_inventory_item(&mut full, &id, "demo.item.dagger");
    }
    let before = full.to_save();
    assert_eq!(
        full.claim_task_reward(&facility, &id),
        Err("inventory-full")
    );
    assert_eq!(full.to_save(), before);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    game.claim_task_reward(&facility, &id).unwrap();
    restored.claim_task_reward(&facility, &id).unwrap();
    same_save(&game, &restored);
    let item = game.items.iter().find(|item| item.id == item_id).unwrap();
    assert_eq!(item.kind_id, "demo.item.rapier");
    assert!(item.artifact_name.is_some());
    assert!(matches!(item.intrinsic_weight_tenths_pound, Some(30 | 55)));
    Game::from_save(game.to_save()).expect("replacement reward must reload");
    assert_eq!(
        game.claim_task_reward(&facility, &id),
        Err("reward-unavailable")
    );
}

#[test]
fn reward_artifact_strafing_executes_and_quickthorn_extra_blows_remain_capped() {
    let mut game = at_level(35);
    let weapon = game
        .items
        .iter()
        .position(|item| item.kind_id == "demo.item.rapier")
        .unwrap();
    let slot = game.items[weapon].location.clone();
    game.items[weapon].location = ItemLocation::Inventory;
    give_inventory_item(&mut game, "test.quickthorn", "demo.item.quickthorn");
    game.items.last_mut().unwrap().location = slot.clone();
    let profile = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        (profile.attacks, profile.extra_attack_chance_percent),
        (1, 0)
    );
    game.items.last_mut().unwrap().location = ItemLocation::Inventory;
    give_inventory_item(&mut game, "test.duelist", "demo.item.duelist");
    game.items.last_mut().unwrap().location = slot;
    game.identify_item_instance("test.duelist", ItemIdentificationRequest::new(true));
    let from = game.player.position;
    let hp = game.player.hp;
    let mut events = Vec::new();
    for _ in 0..100 {
        game.use_inventory_item(
            "test.duelist",
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.items.last().unwrap().charges.unwrap().current == 0 {
            break;
        }
    }
    assert_eq!(game.items.last().unwrap().charges.unwrap().current, 0);
    assert_ne!(game.player.position, from);
    assert!(crate::game::projectile_geometry::rfb_distance(from, game.player.position) <= 10);
    assert!(crate::game::visibility::has_line_of_sight(
        &game,
        from,
        game.player.position
    ));
    assert_eq!(game.player.hp, hp);
    for id in [
        "demo.town-facility.anambar-warrior-guild",
        "demo.town-facility.morivant-thieves-guild",
    ] {
        assert_eq!(
            game.town_facility_membership(game.content.town_facility(id).unwrap()),
            rfb_protocol::FacilityMembershipDto::Visitor
        );
    }
}

// SPDX-License-Identifier: MPL-2.0
use super::*;

const SERPENT: &str = "demo.actor.the-serpent-of-chaos";
const TASK: &str = "demo.task.angband-serpent-of-chaos";

#[test]
fn angband_old_test_serpent_death_does_not_win_the_formal_campaign() {
    let mut game = entrance_game();
    let id = "test.ag6.old-serpent";
    let position = Position {
        x: game.player.position.x - 1,
        y: game.player.position.y,
    };
    game.push_generated_actor(id.into(), "demo.actor.serpent-of-chaos", position);
    let mut events = task_death(&mut game, id, true);
    game.apply_campaign_events(&mut events);
    assert_eq!(game.campaign_state.status, CampaignStatusDto::Active);
    assert_eq!(game.task_states[TASK].current, 0);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::CampaignVictorious { .. }))
    );
}

#[test]
fn angband_full_hp_serpent_combat_wins_once_then_explores_saves_and_retires() {
    let mut game = entrance_game();
    game.debug_prepare_angband_e2e("arrival", None).unwrap();
    choose_human_talent_if_pending(&mut game);
    let surface = game.current_floor_id.clone();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    // Earlier quest gates, player XP, survival and melee power are explicit fixture
    // preparation. The final quest and its source boss HP, defenses and AI stay intact.
    complete_task_gates(&mut game);
    choose_human_talent_if_pending(&mut game);
    assert_eq!(game.progress.level, 50);
    game.transition_floor(floor_id(100), None, None, false)
        .unwrap()
        .unwrap();
    game.entities.retain(|actor| actor.kind_id == SERPENT);
    assert_eq!(game.entities[0].hp, 30_000);
    let mut killing_update = None;
    for attack in 0..128 {
        let Some(boss) = game.entities.iter().find(|actor| actor.kind_id == SERPENT) else {
            break;
        };
        let id = boss.id.clone();
        game.debug_prepare_angband_e2e("battle", Some(&id)).unwrap();
        let target = game
            .entities
            .iter()
            .find(|actor| actor.id == id)
            .unwrap()
            .position;
        let direction = [
            Direction::East,
            Direction::West,
            Direction::South,
            Direction::North,
            Direction::SouthEast,
            Direction::SouthWest,
            Direction::NorthEast,
            Direction::NorthWest,
        ]
        .into_iter()
        .find(|direction| {
            let (dx, dy) = direction.delta();
            game.player.position.x + dx == target.x && game.player.position.y + dy == target.y
        })
        .unwrap();
        let update = dispatch_next(&mut game, GameCommand::Move { direction });
        assert!(
            game.player.hp >= 0,
            "prepared player must survive the source boss"
        );
        if update.campaign.status == CampaignStatusDto::Victorious {
            assert!(attack > 0, "full-HP boss must require multiple attacks");
            killing_update = Some(update);
            break;
        }
    }
    let victory = killing_update.expect("source serpent must die through combat commands");
    assert_eq!(game.task_states[TASK].status, TaskStatusKindDto::Completed);
    assert_eq!(game.defeated_limited_actor_counts.get(SERPENT), Some(&1));
    let won = victory
        .events
        .iter()
        .position(|event| event.kind == "campaign.victorious")
        .unwrap();
    let completed = victory
        .events
        .iter()
        .position(|event| event.kind == "task.completed")
        .unwrap();
    let loot = victory
        .events
        .iter()
        .rposition(|event| event.kind == "loot.drop")
        .unwrap();
    assert!(completed < won && loot < won);
    assert_eq!(
        victory
            .events
            .iter()
            .filter(|event| event.kind == "campaign.victorious")
            .count(),
        1
    );
    assert!(game.progress.level > 50);
    let progress = &victory.player.progress;
    assert_eq!(progress.level_cap, CharacterProgress::level_cap(true));
    assert_eq!(
        progress.attribute_index_cap,
        CharacterProgress::attribute_index_cap(true)
    );
    assert!(!victory.campaign.can_retire);
    for kind in ["demo.item.grond", "demo.item.crown-of-chaos"] {
        assert_eq!(
            game.items
                .iter()
                .filter(|item| item.kind_id == kind)
                .count(),
            1
        );
    }
    clear_monsters(&mut game);
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_BERSERK);
    game.player.hp = game.effective_player_max_hp();
    game.reveal_current_visibility();
    let fame = game.fame;
    let victory_turn = game.campaign_state.victory_turn;
    let mut loaded = restored(&game);
    let next = dispatch_next(&mut game, GameCommand::Wait);
    let replay = dispatch_next(&mut loaded, GameCommand::Wait);
    assert_eq!(next, replay);
    assert_eq!(game.fame, fame);
    assert_eq!(game.campaign_state.victory_turn, victory_turn);
    assert!(
        !next
            .events
            .iter()
            .any(|event| event.kind == "campaign.victorious")
    );
    traverse(&mut game, "demo.terrain.stairs-down", &floor_id(101));
    assert_eq!(game.snapshot().dungeon.unwrap().current_depth, 101);
    game.transition_floor(floor_id(127), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.reveal_current_visibility();
    let mut game = restored(&game);
    assert!(!game.snapshot().campaign.can_retire);
    game.start_recall(0);
    game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(game.current_floor_id, surface);
    assert_eq!(game.wilderness_position, Some(Position { x: 57, y: 40 }));
    clear_monsters(&mut game);
    game.reveal_current_visibility();
    assert!(game.snapshot().campaign.can_retire);
    let mut dead = game.clone();
    dead.player.hp = -1;
    assert!(!dead.snapshot().campaign.can_retire);
    assert_eq!(
        restored(&dead).campaign_state.status,
        CampaignStatusDto::Victorious
    );
    let expected = game.campaign_score_at(game.turn + 1);
    let retired = dispatch_next(&mut game, GameCommand::Retire);
    assert_eq!(retired.campaign.status, CampaignStatusDto::Retired);
    assert_eq!(retired.campaign.score, expected);
    assert_eq!(restored(&game).state_hash(), game.state_hash());
    assert!(matches!(
        game.dispatch(command(
            game.last_command_seq + 1,
            game.revision,
            GameCommand::Wait
        )),
        Err(CoreError::CampaignEnded)
    ));
}

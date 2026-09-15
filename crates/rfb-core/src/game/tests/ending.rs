// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn abandon_is_zero_time_frozen_and_round_trips_on_both_map_scales() {
    for world_map in [false, true] {
        let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        if world_map {
            dispatch_next(
                &mut game,
                GameCommand::EnterWorldMap {
                    cancel_recall: false,
                },
            );
        }
        let before = game.snapshot();
        let draws = game.rng.draw_counter;
        let update = dispatch_next(&mut game, GameCommand::EndCharacter);
        assert_eq!(update.campaign.status, CampaignStatusDto::Abandoned);
        assert_eq!(update.campaign.score, before.campaign.score);
        assert_eq!(
            (
                update.turn,
                update.world_tick,
                update.player.hp,
                game.rng.draw_counter
            ),
            (before.turn, before.world_tick, before.player.hp, draws)
        );
        assert!(!update.command_repeatable);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "campaign.abandoned")
        );
        let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert!(matches!(
            restored.dispatch(command(
                update.command_seq + 1,
                update.revision,
                GameCommand::Wait
            )),
            Err(CoreError::CampaignEnded)
        ));
        let mut invalid = game.to_save();
        invalid.campaign_state.as_mut().unwrap().final_score = Some(before.campaign.score + 1);
        assert!(Game::from_save(invalid, Game::default_behavior_preferences()).is_err());
    }
}

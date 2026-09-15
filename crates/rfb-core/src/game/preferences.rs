// SPDX-License-Identifier: MPL-2.0
use super::*;
use rfb_protocol::{BehaviorContextDto, BehaviorPreferencesDto, MogaminatorPreferencesDto};

impl Game {
    pub fn default_behavior_preferences() -> BehaviorPreferencesDto {
        let state = MogaminatorState::default();
        BehaviorPreferencesDto {
            locale: LocaleDto::ZhCn,
            travel: rfb_protocol::TravelOptionsDto::default(),
            operations: rfb_protocol::OperationOptionsDto::default(),
            mogaminator: MogaminatorPreferencesDto {
                enabled: state.enabled,
                leave_destroyed_items: state.leave_destroyed_items,
                auto_get_mode: state.auto_get_mode,
                zh_cn_source: state.zh_cn_source,
                en_us_source: state.en_us_source,
            },
        }
    }

    pub fn validate_behavior_preferences(
        preferences: &BehaviorPreferencesDto,
    ) -> Result<(), CoreError> {
        for source in [
            &preferences.mogaminator.zh_cn_source,
            &preferences.mogaminator.en_us_source,
        ] {
            crate::mogaminator::compile_mogaminator(source)
                .map_err(|errors| CoreError::InvalidPreferences(format!("{errors:?}")))?;
        }
        Ok(())
    }

    pub fn behavior_preferences(&self) -> BehaviorPreferencesDto {
        BehaviorPreferencesDto {
            locale: self.interface_locale,
            travel: self.travel_options,
            operations: self.operation_options,
            mogaminator: MogaminatorPreferencesDto {
                enabled: self.mogaminator.enabled,
                leave_destroyed_items: self.mogaminator.leave_destroyed_items,
                auto_get_mode: self.mogaminator.auto_get_mode,
                zh_cn_source: self.mogaminator.zh_cn_source.clone(),
                en_us_source: self.mogaminator.en_us_source.clone(),
            },
        }
    }

    // No turns, RNG, item processing, or changes to character-owned wanted targets.
    pub fn apply_behavior_preferences(
        &mut self,
        preferences: BehaviorPreferencesDto,
    ) -> Result<(), CoreError> {
        Self::validate_behavior_preferences(&preferences)?;
        self.interface_locale = preferences.locale;
        self.travel_options = preferences.travel;
        self.operation_options = preferences.operations;
        let p = preferences.mogaminator;
        self.mogaminator.enabled = p.enabled;
        self.mogaminator.leave_destroyed_items = p.leave_destroyed_items;
        self.mogaminator.auto_get_mode = p.auto_get_mode;
        self.mogaminator.zh_cn_source = p.zh_cn_source;
        self.mogaminator.en_us_source = p.en_us_source;
        self.mogaminator.pending_query = None;
        self.mogaminator.dismissed_query_item_ids.clear();
        Ok(())
    }

    pub fn behavior_context(&self) -> BehaviorContextDto {
        let state = self.mogaminator.to_context();
        BehaviorContextDto {
            preferences: self.behavior_preferences(),
            pending_query: state.pending_query,
            dismissed_query_item_ids: state.dismissed_query_item_ids,
        }
    }

    pub fn restore_behavior_context(
        &mut self,
        context: BehaviorContextDto,
    ) -> Result<(), CoreError> {
        if context
            .dismissed_query_item_ids
            .iter()
            .any(|id| !self.items.iter().any(|item| item.id == *id))
            || context.pending_query.as_ref().is_some_and(|query| {
                query.rule_line == 0
                    || !self.items.iter().any(|item| {
                        item.id == query.item_id
                            && item.location == ItemLocation::Ground(self.player.position)
                    })
            })
        {
            return Err(CoreError::InvalidPreferences(
                "invalid replay query context".into(),
            ));
        }
        self.apply_behavior_preferences(context.preferences)?;
        self.mogaminator.pending_query =
            context
                .pending_query
                .map(|q| super::mogaminator::MogaminatorPendingQuery {
                    item_id: q.item_id,
                    rule_line: q.rule_line,
                });
        self.mogaminator.dismissed_query_item_ids =
            context.dismissed_query_item_ids.into_iter().collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rfb_protocol::GameCommand;

    #[test]
    fn reloading_mogaminator_is_atomic_and_preserves_other_preferences_and_character_facts() {
        let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
        game.operation_options.cut_corners = true;
        game.travel_options.always_pickup = true;
        let mut before_save = game.to_save();
        let before_preferences = game.behavior_preferences();
        let before_hash = game.state_hash();
        let mut rules = before_preferences.mogaminator.clone();
        rules.enabled = true;
        rules.en_us_source = "?:[EQU $SYS windows]\nweapons".into();
        assert!(
            game.dispatch(GameCommandEnvelope {
                command_seq: game.last_command_seq() + 1,
                expected_revision: game.revision(),
                command: GameCommand::ConfigureMogaminatorPreferences {
                    preferences: rules.clone()
                },
            })
            .is_err()
        );
        assert_eq!(game.state_hash(), before_hash);
        assert_eq!(game.to_save(), before_save);
        rules.zh_cn_source = "!物品".into();
        rules.en_us_source = "!items".into();
        game.dispatch(GameCommandEnvelope {
            command_seq: game.last_command_seq() + 1,
            expected_revision: game.revision(),
            command: GameCommand::ConfigureMogaminatorPreferences {
                preferences: rules.clone(),
            },
        })
        .unwrap();
        let after = game.behavior_preferences();
        assert_eq!(after.locale, before_preferences.locale);
        assert_eq!(after.travel, before_preferences.travel);
        assert_eq!(after.operations, before_preferences.operations);
        assert_eq!(after.mogaminator, rules);
        before_save.revision = game.revision();
        before_save.last_command_seq = game.last_command_seq();
        assert_eq!(
            game.to_save(),
            before_save,
            "reload must not process items, spend time, or advance RNG"
        );
    }

    #[test]
    fn preferences_are_injected_on_load_and_never_serialized_as_character_facts() {
        let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
        let before = game.to_save();
        let hash = game.state_hash();
        let mut p = game.behavior_preferences();
        p.locale = LocaleDto::EnUs;
        p.travel.always_pickup = true;
        p.operations.cut_corners = true;
        p.operations.default_target = rfb_protocol::DefaultTargetModeDto::OldThenNearest;
        p.mogaminator.enabled = true;
        p.mogaminator.zh_cn_source = "!物品".into();
        p.mogaminator.en_us_source = "!items".into();
        game.apply_behavior_preferences(p.clone()).unwrap();
        assert_eq!(game.to_save(), before);
        assert_ne!(
            game.state_hash(),
            hash,
            "execution hash includes behavior context"
        );
        let json = serde_json::to_value(game.to_save()).unwrap();
        for field in [
            "interfaceLocale",
            "travelOptions",
            "operationOptions",
            "operations",
            "mogaminator",
            "preferences",
        ] {
            assert!(
                json.get(field).is_none(),
                "{field} leaked into character save"
            );
        }
        let restored = Game::from_save(before.clone(), p.clone()).unwrap();
        assert_eq!(restored.behavior_preferences(), p);
        assert_eq!(restored.to_save(), before);
        let defaults =
            Game::from_save(before.clone(), Game::default_behavior_preferences()).unwrap();
        assert_ne!(defaults.behavior_preferences(), p);
        assert_eq!(defaults.to_save(), before);
    }

    #[test]
    fn applying_destructive_rules_is_a_zero_time_context_change_not_an_item_action() {
        let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
        assert!(!game.to_save().inventory.is_empty());
        let mut before = game.to_save();
        let mut p = game.behavior_preferences();
        p.mogaminator.enabled = true;
        p.mogaminator.zh_cn_source = "!物品".into();
        game.dispatch(GameCommandEnvelope {
            command_seq: game.last_command_seq() + 1,
            expected_revision: game.revision(),
            command: GameCommand::ConfigurePreferences { preferences: p },
        })
        .unwrap();
        // Only command bookkeeping changes, not world/character facts or RNG.
        before.revision = game.revision();
        before.last_command_seq = game.last_command_seq();
        assert_eq!(game.to_save(), before);
        assert!(game.mogaminator.pending_query.is_none());
        assert!(game.mogaminator.dismissed_query_item_ids.is_empty());
    }
}

// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(super) enum PlayerTrapOutcome {
    Resisted,
    Triggered {
        source_kind_id: String,
        damage: DamageOutcome,
    },
}

impl Game {
    pub(super) fn trigger_player_trap(
        &mut self,
        position: Position,
        events: &mut Vec<DomainEvent>,
    ) -> Option<PlayerTrapOutcome> {
        let index = self.index(position)?;
        let terrain = self.content.terrain(&self.terrain[index])?;
        let source_kind_id = terrain.id.clone();
        let trap = terrain.trap.clone()?;
        self.revealed_terrain.insert(position);
        if let Some(difficulty) = trap.saving_throw_difficulty {
            let ability = self.player_derived_stats().saving_throw_skill;
            let mut difficulty_pipeline = DerivedStatsPipeline::new();
            difficulty_pipeline.add(
                StatKind::ActionDifficulty,
                StatLayer::Environment,
                &source_kind_id,
                difficulty,
            );
            let check = resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::SavingThrow,
                    actor_id: self.player.id.clone(),
                    target_id: Some(source_kind_id.clone()),
                    ability,
                    difficulty: difficulty_pipeline
                        .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                },
            );
            let succeeded = check.succeeded();
            let skill_id = self
                .content
                .skill_by_kind(SkillKind::SavingThrow)
                .expect("validated saving throw skill must remain available")
                .id
                .clone();
            events.push(DomainEvent::SavingThrowChecked {
                source_kind_id: source_kind_id.clone(),
                position,
                succeeded,
                resolution: check.to_dto(skill_id),
            });
            if succeeded {
                return Some(PlayerTrapOutcome::Resisted);
            }
        }
        let damage = self.reduce_player_damage(resolve_damage(
            DamagePacket::new(trap.damage, trap.damage_type.into()),
            self.effective_player_resistances()
                .level(trap.damage_type.into()),
        ));
        let application = plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
        commit_damage_application(&mut self.player, &application);
        Some(PlayerTrapOutcome::Triggered {
            source_kind_id,
            damage,
        })
    }
}

// SPDX-License-Identifier: MPL-2.0

use super::movement::actor_avoids_terrain_trap;
use super::*;

pub(super) enum PlayerTrapOutcome {
    Resisted,
    Triggered {
        source_kind_id: String,
        damage: DamageOutcome,
    },
}

impl Game {
    pub(super) fn trigger_actor_trap(
        &mut self,
        index: usize,
        position: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let Some(terrain_index) = self.index(position) else {
            return Ok(true);
        };
        let Some(terrain) = self.content.terrain(&self.terrain[terrain_index]) else {
            return Ok(true);
        };
        let Some(trap) = terrain.trap.clone() else {
            return Ok(true);
        };
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("moving actor definition must remain available")
            .clone();
        if actor_avoids_terrain_trap(&definition, terrain) {
            return Ok(true);
        }
        let target_kind_id = self.entities[index].kind_id.clone();
        let resistance = self.entities[index]
            .resistances
            .level(trap.damage_type.into());
        let damage = resolve_damage(
            DamagePacket::new(trap.damage, trap.damage_type.into()),
            resistance,
        );
        let application =
            plan_damage_application(&self.entities[index], damage, FatalityPolicy::AtOrBelowZero);
        commit_damage_application(&mut self.entities[index], &application);
        let event = DomainEvent::ActorTrapTriggered {
            position,
            target_kind_id,
            damage,
        };
        if application.fatal {
            self.resolve_actor_death(index, event, events, changed, removed_entities)?;
            Ok(false)
        } else {
            self.wake_entity_after_damage(index, damage.applied, events);
            events.push(event);
            Ok(true)
        }
    }

    pub(super) fn trigger_player_trap(
        &mut self,
        position: Position,
        events: &mut Vec<DomainEvent>,
    ) -> Option<PlayerTrapOutcome> {
        let index = self.index(position)?;
        let terrain = self.content.terrain(&self.terrain[index])?;
        let source_kind_id = terrain.id.clone();
        let trap = terrain.trap.clone()?;
        if self.active_traveler_has_mode(rfb_content::ActorMovementMode::Fly)
            && trap
                .avoided_by_movement_modes
                .contains(&rfb_content::ActorMovementMode::Fly)
        {
            return None;
        }
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

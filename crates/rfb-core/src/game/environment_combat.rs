// SPDX-License-Identifier: MPL-2.0

use super::movement::actor_avoids_terrain_trap;
use super::*;

const EXPLOSIVE_RUNE_ABILITY_ID: &str = "rfb.ability.race.explosive-rune";
const EXPLOSIVE_RUNE_BREAK_POWER: u64 = 299;

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
        if terrain.tags.iter().any(|tag| tag == "explosive-rune") {
            let terrain_kind_id = terrain.id.clone();
            let replacement_terrain_kind_id = terrain
                .monster_destroy_to_terrain_id
                .clone()
                .expect("validated explosive rune must define a destruction target");
            return self.trigger_explosive_rune(
                index,
                terrain_index,
                position,
                terrain_kind_id,
                replacement_terrain_kind_id,
                events,
                changed,
                removed_entities,
            );
        }
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

    #[allow(clippy::too_many_arguments)]
    fn trigger_explosive_rune(
        &mut self,
        index: usize,
        terrain_index: usize,
        position: Position,
        terrain_kind_id: String,
        replacement_terrain_kind_id: String,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let target_entity_id = self.entities[index].id.clone();
        let source_kind_id = self.entities[index].kind_id.clone();
        let monster_level = self
            .actor_runtime_definition(&self.entities[index])
            .expect("moving actor definition must remain available")
            .level;
        let break_roll_sides = EXPLOSIVE_RUNE_BREAK_POWER * u64::from(self.progress.level) / 50;
        let explodes = self.rng.bounded(break_roll_sides) + 1 > u64::from(monster_level);

        self.terrain[terrain_index] = replacement_terrain_kind_id.clone();
        self.revealed_terrain.remove(&position);
        changed.insert(position);
        if !explodes {
            events.push(DomainEvent::MonsterTerrainDestroyed {
                source_kind_id,
                terrain_kind_id,
                replacement_terrain_kind_id,
                position,
            });
            return Ok(true);
        }

        let base_raw_damage =
            (i32::from(self.progress.level) + self.roll_damage(7, 7)).saturating_mul(2);
        self.resolve_player_area_damage_with_base(
            EXPLOSIVE_RUNE_ABILITY_ID,
            vec![position],
            false,
            DamageType::Mana,
            2,
            None,
            base_raw_damage,
            true,
            events,
            changed,
            removed_entities,
        )?;
        Ok(self
            .entities
            .iter()
            .any(|entity| entity.id == target_entity_id && entity.hp > 0))
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
        let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
        let damage = application.damage;
        self.damage_player_inventory(
            &source_kind_id,
            trap.damage_type.into(),
            false,
            damage.applied,
            events,
        );
        Some(PlayerTrapOutcome::Triggered {
            source_kind_id,
            damage,
        })
    }
}

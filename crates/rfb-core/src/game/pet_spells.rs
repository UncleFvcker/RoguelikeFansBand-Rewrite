// SPDX-License-Identifier: MPL-2.0
use super::*;

// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c:
// monspell.c _ai_think_pet / _avoid_hurting_player; spells1.c project_p.
impl Game {
    pub(super) fn monster_spell_targets(&self, index: usize) -> Vec<MonsterHostileTarget> {
        let mut targets = self.monster_hostile_targets(index);
        if self.entity_is_player_aligned(index) {
            let commanded = self.pet_hostile_target_ids(index);
            targets.retain(|target| {
                commanded.iter().any(|id| id == target.entity_id())
                    && has_line_of_effect(self, self.entities[index].position, target.position())
            });
        }
        targets
    }

    pub(super) fn pet_spell_allowed(&self, ability: &AbilityDefinition) -> bool {
        if ability.tags.iter().any(|tag| tag == "monster-world") {
            return false;
        }
        ability
            .effect
            .ordered_effects()
            .iter()
            .all(|effect| match effect {
                AbilityEffectDefinition::Summon { .. }
                | AbilityEffectDefinition::SummonCategory { .. } => {
                    self.summon_command.summon_spells
                }
                AbilityEffectDefinition::NoOp { .. } => {
                    self.summon_command.summon_spells
                        && ability
                            .tags
                            .iter()
                            .any(|tag| tag == monster_abilities::MONSTER_FAMILY_SUMMON_TAG)
                }
                AbilityEffectDefinition::AggravateMonsters
                | AbilityEffectDefinition::DarkenRoom
                | AbilityEffectDefinition::Amnesia
                | AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::TeleportLevel => false,
                AbilityEffectDefinition::TransformTerrain {
                    target_terrain_id, ..
                } if self
                    .content
                    .terrain(target_terrain_id)
                    .is_some_and(|terrain| terrain.trap.is_some()) =>
                {
                    false
                }
                AbilityEffectDefinition::Damage { .. }
                | AbilityEffectDefinition::AreaDamage { .. }
                | AbilityEffectDefinition::BeamDamage { .. }
                | AbilityEffectDefinition::ConeDamage { .. }
                | AbilityEffectDefinition::BreathDamage { .. }
                | AbilityEffectDefinition::CurseDamage { .. } => self.summon_command.attack_spells,
                AbilityEffectDefinition::BirdDrop => {
                    self.summon_command.attack_spells && self.summon_command.teleport
                }
                AbilityEffectDefinition::JumpDamage { .. }
                | AbilityEffectDefinition::BlinkSelf { .. }
                | AbilityEffectDefinition::TeleportSelf { .. }
                | AbilityEffectDefinition::BlinkTarget { .. }
                | AbilityEffectDefinition::TeleportAway { .. }
                | AbilityEffectDefinition::TeleportTarget => self.summon_command.teleport,
                _ => true,
            })
    }

    pub(super) fn pet_spell_risk_is_allowed(
        &self,
        index: usize,
        positions: &[Position],
        friendly_risk_count: u16,
    ) -> bool {
        if !self.entity_is_player_aligned(index) {
            return friendly_risk_count == 0;
        }
        let mounted = self.riding_actor_id.as_deref() == Some(self.entities[index].id.as_str());
        let allowed_player = (mounted || self.summon_command.allow_player_damage)
            && !self.player_is_dead()
            && positions.contains(&self.player.position);
        // The option concerns the player; preserve the existing protection of
        // other friendly monsters. A mount cannot damage its own rider.
        friendly_risk_count <= u16::from(allowed_player)
    }

    pub(super) fn monster_area_targets(
        &self,
        index: usize,
        positions: &[Position],
    ) -> Vec<MonsterHostileTarget> {
        // Collateral enemies are determined by the footprint, not follow range
        // or the single commanded target used when selecting the spell.
        let mut targets = self.monster_hostile_targets(index);
        if self.entity_is_player_aligned(index)
            && self.summon_command.allow_player_damage
            && self.riding_actor_id.as_deref() != Some(self.entities[index].id.as_str())
            && !self.player_is_dead()
        {
            targets.push(MonsterHostileTarget::Player {
                entity_id: self.player.id.clone(),
                kind_id: self.player.kind_id.clone(),
                position: self.player.position,
            });
        }
        targets.retain(|target| positions.contains(&target.position()));
        targets
    }

    pub(super) fn restrict_mounted_spell_destinations(
        &self,
        index: usize,
        plan: &mut MonsterAbilityPlan,
    ) -> Result<(), MonsterAbilityPlanRejection> {
        if self.riding_actor_id.as_deref() != Some(self.entities[index].id.as_str()) {
            return Ok(());
        }
        match &mut plan.target {
            MonsterAbilityTargetPlan::BirdDrop {
                escape_destinations,
                ..
            } => {
                // Flying away is only one branch; the drop remains usable
                // even when the rider prevents that optional displacement.
                escape_destinations.retain(|position| {
                    !self.player_has_anti_teleport()
                        && self.player_can_teleport_to(*position, false)
                });
            }
            MonsterAbilityTargetPlan::BlinkSelf { destinations }
            | MonsterAbilityTargetPlan::EscapeSelf { destinations }
            | MonsterAbilityTargetPlan::JumpDamage { destinations, .. } => {
                destinations.retain(|position| {
                    !self.player_has_anti_teleport()
                        && self.player_can_teleport_to(*position, false)
                });
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn relocate_spell_caster(
        &mut self,
        index: usize,
        destination: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.riding_actor_id.as_deref() == Some(self.entities[index].id.as_str()) {
            events.extend(self.relocate_player(destination, changed));
        } else {
            changed.insert(self.entities[index].position);
            self.entities[index].position = destination;
            changed.insert(destination);
        }
    }
}

// SPDX-License-Identifier: MPL-2.0

use crate::game::damage::FatalityPolicy;
use crate::game::item_value::interpolate;
use crate::game::*;
use rfb_protocol::{AbilityControlOutcomeDto, AbilityStatusChangeDto};

// cmd5.c / gf.c: terrain projections are utility even when they also deal damage.
fn attack_projection(effect: &AbilityEffectDefinition, self_target: bool) -> Option<bool> {
    use AbilityEffectDefinition::*;
    match effect {
        Sequence { effects } => effects
            .iter()
            .find_map(|effect| attack_projection(effect, self_target)),
        LightLine { .. }
        | LightArea { .. }
        | DarkenRoom
        | TerrainBeam { .. }
        | LavaFlow { .. }
        | CreateDoor { .. }
        | CreateAdjacentTerrain { .. } => Some(false),
        Damage { damage_type, .. }
        | AreaDamage { damage_type, .. }
        | BeamDamage { damage_type, .. }
        | BoltOrBeamDamage { damage_type, .. }
        | BoltOrAreaDamage { damage_type, .. }
        | ConeDamage { damage_type, .. }
        | VisibleDamage { damage_type, .. } => Some(*damage_type != ActorDamageType::Disintegrate),
        ApplyStatus { .. } => (!self_target).then_some(true),
        Malediction { .. }
        | CallSunlight { .. }
        | Stardust { .. }
        | CurseDamage { .. }
        | DeathRay { .. }
        | TeleportAway { .. }
        | PolymorphTarget
        | Control { .. }
        | DrainLife { .. }
        | TurnUndead { .. }
        | VisibleApplyStatus { .. }
        | Entangle { .. }
        | MassSleepOrStasis { .. }
        | Sanctuary { .. }
        | NatureWrath
        | WrathOfGod { .. }
        | DivineIntervention
        | Crusade
        | InsanityCircle { .. }
        | Hellfire { .. }
        | DoomHand
        | BanishEvil => Some(true),
        _ => None,
    }
}

impl Game {
    pub(in crate::game) fn book_spell_realm(&self, ability_id: &str) -> Option<&str> {
        self.active_casting_realm_profiles()
            .into_iter()
            .find(|realm| {
                realm.ability_book_ids.iter().any(|book_id| {
                    self.content
                        .ability_book(book_id)
                        .is_some_and(|book| book.ability_ids.iter().any(|id| id == ability_id))
                })
            })
            .map(|realm| realm.realm_id.as_str())
    }

    pub(super) fn spell_practice_targets(&self) -> BTreeMap<String, Position> {
        self.entities
            .iter()
            .filter(|actor| {
                actor.hp > 0
                    && !self.actor_is_player_side(actor)
                    && self
                        .actor_runtime_definition(actor)
                        .is_some_and(|kind| !kind.movement.never_moves)
            })
            .map(|actor| (actor.id.clone(), actor.position))
            .collect()
    }

    pub(super) fn grow_mage_spell(
        &mut self,
        ability: &AbilityDefinition,
        targets: &BTreeMap<String, Position>,
        events: &[DomainEvent],
    ) -> AbilityProgress {
        let depth = if self.is_wilderness_floor() && self.current_town().is_none() {
            self.wilderness_danger_level(
                self.wilderness_position.expect("wilderness has a position"),
            )
        } else {
            self.floor_depth(&self.current_floor_id)
        };
        let attack = attack_projection(
            &ability.effect,
            ability.target.modes == [AbilityTargetModeDefinition::SelfTarget],
        )
        .unwrap_or(false);
        let useful = !attack
            || events.iter().any(|event| match event {
                DomainEvent::AbilityHit {
                    ability_id,
                    damage,
                    trace,
                    ..
                } => {
                    ability_id == &ability.id
                        && damage.applied > 0
                        && targets.values().any(|pos| *pos == trace.impact)
                }
                DomainEvent::AbilityEffectsResolved {
                    ability_id,
                    resolution,
                    ..
                } if ability_id == &ability.id => {
                    resolution.effects.iter().any(|effect| match effect {
                        AbilityEffectResolutionDto::ApplyStatus { change, .. } => {
                            *change != AbilityStatusChangeDto::Immune
                                && resolution
                                    .target_entity_id
                                    .as_ref()
                                    .is_some_and(|id| targets.contains_key(id))
                        }
                        AbilityEffectResolutionDto::Control {
                            target_entity_id,
                            outcome,
                            ..
                        } => {
                            matches!(
                                outcome,
                                AbilityControlOutcomeDto::Controlled
                                    | AbilityControlOutcomeDto::Resisted
                            ) && targets.contains_key(target_entity_id)
                        }
                        AbilityEffectResolutionDto::DeathRay { living, .. } => {
                            *living
                                && resolution
                                    .target_entity_id
                                    .as_ref()
                                    .is_some_and(|id| targets.contains_key(id))
                        }
                        AbilityEffectResolutionDto::TeleportAway {
                            target_entity_id, ..
                        }
                        | AbilityEffectResolutionDto::PolymorphTarget {
                            target_entity_id, ..
                        } => targets.contains_key(target_entity_id),
                        _ => false,
                    })
                }
                _ => false,
            });
        let difficulty = Self::player_ability_parameters(ability).minimum_level;
        let progress = self
            .ability_progress
            .get_mut(&ability.id)
            .expect("learned spell has progress");
        if depth > 0 && useful {
            let ratio = (17 + i32::from(difficulty)) * 100 / (10 + i32::from(depth));
            let maximum =
                interpolate(ratio, &[(60, 1600), (100, 1200), (200, 900), (300, 0)]) as u16;
            if progress.proficiency < maximum {
                let gain = interpolate(
                    i32::from(progress.proficiency),
                    &[
                        (0, 128),
                        (200, 64),
                        (400, 32),
                        (600, 16),
                        (800, 8),
                        (1000, 4),
                        (1200, 2),
                        (1400, 1),
                        (1600, 1),
                    ],
                ) as u16
                    * if attack { 1 } else { 3 };
                progress.proficiency = (progress.proficiency + gain)
                    .min(maximum)
                    .min(progress.proficiency_cap);
            }
        }
        *progress
    }

    pub(super) fn resolve_mage_spell_failure(
        &mut self,
        ability: &AbilityDefinition,
        failure: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let realm = self
            .book_spell_realm(&ability.id)
            .expect("book spell has a realm")
            .to_owned();
        self.apply_book_spell_failure_virtues(&realm, failure);
        if realm != "death" {
            return;
        }
        let realm_profile = self
            .active_casting_realm_profiles()
            .into_iter()
            .find(|profile| profile.realm_id == realm)
            .expect("active death realm");
        let (rank, index) = realm_profile
            .ability_book_ids
            .iter()
            .find_map(|id| {
                self.content.ability_book(id).and_then(|book| {
                    let rank = usize::from(book.rank.expect("realm book has a rank") - 1);
                    book.ability_ids
                        .iter()
                        .position(|id| id == &ability.id)
                        .map(|slot| (rank, rank * 8 + slot))
                })
            })
            .expect("book contains its spell");
        if self.rng.bounded(100) + 1 >= index as u64 {
            return;
        }
        if rank == 3 && self.rng.bounded(2) == 0 {
            let outcome = self.resolve_necromantic_sanity_blast(&ability.id, events, changed);
            events.push(DomainEvent::SpellSanityBlasted {
                ability_id: ability.id.clone(),
                outcome,
            });
        } else {
            let raw = self.roll_damage((rank + 1) as u16, 6);
            let damage = resolve_damage(
                DamagePacket::new(raw, DamageType::Physical),
                ResistanceLevel::Normal,
            );
            let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                trace: None,
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(self.player.id.clone()),
                    target_kind_id: Some(self.player.kind_id.clone()),
                    effects: vec![AbilityEffectResolutionDto::SelfDamage {
                        effect_index: 0,
                        damage: application.damage.applied,
                        fatal: application.fatal,
                    }],
                },
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: ability.id.clone(),
                    method_id: Some(ability.id.clone()),
                    damage: application.damage,
                });
            }
            if index > 15 && self.rng.bounded(6) == 0 && self.player_hold_life_sources() == 0 {
                self.apply_player_experience_drain(index as u64 * 250, &ability.id, events);
            }
        }
    }
}

// SPDX-License-Identifier: MPL-2.0

use super::movement::actor_can_cross_terrain;
use super::*;
use rfb_content::{
    ActorAllocationDefinition, ActorDamageType, ActorDefinition, ActorHabitat, ActorMovementMode,
    ActorResistanceLevel, GlobalMonsterAllocationDefinition,
};

const ORIGINAL_NASTY_MON_ONE_IN: u64 = 40;
const ORIGINAL_GROUP_MAX: u16 = 32;
const ORIGINAL_ESCORT_ATTEMPTS: u16 = 32;
const ORIGINAL_MAX_REPRODUCERS: usize = 100;
const ORIGINAL_MULTIPLY_ADJACENCY_FACTOR: u64 = 8;
const ORIGINAL_MAX_SIGHT: u32 = 20;
const SHADOWER_APPEARANCE_KIND_ID: &str = "demo.actor.shadower";
const SHADOWER_ONE_IN: u64 = 333;
const CHAMELEON_CHANGE_ONE_IN: u64 = 13;
const ELDRITCH_HORROR_TAG: &str = "eldritch-horror";
const ELDRITCH_HORROR_IMMUNITY: &str = "rfb.status.eldritch-horror";
const ELDRITCH_HORROR_REPEAT_ONE_IN: u64 = 5;
const ELDRITCH_HIGH_RESISTANCE_ROLL: u64 = 33;
const MORONIC_MUTATION_ID: &str = "rfb.mutation.moronic";
const COWARDICE_MUTATION_ID: &str = "rfb.mutation.cowardice";
const HALLUCINATION_MUTATION_ID: &str = "rfb.mutation.hallucination";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OriginalGroupRole {
    Friend,
    Escort,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalGroupMember {
    pub(super) kind_id: String,
    pub(super) position: Position,
    pub(super) role: OriginalGroupRole,
}

pub(super) fn actor_allocation_matches_legacy_dungeon(
    allocation: &ActorAllocationDefinition,
    current_legacy_dungeon_index: Option<u16>,
) -> bool {
    allocation.legacy_dungeon_indices.is_empty()
        || current_legacy_dungeon_index
            .is_some_and(|index| allocation.legacy_dungeon_indices.contains(&index))
}

pub(super) fn actor_allocation_matches_task(
    allocation: &ActorAllocationDefinition,
    current_task_id: Option<&str>,
) -> bool {
    allocation
        .task_id
        .as_deref()
        .is_none_or(|task_id| current_task_id == Some(task_id))
}

impl Game {
    pub(super) fn actor_runtime_definition<'a>(
        &'a self,
        actor: &Actor,
    ) -> Option<&'a ActorDefinition> {
        let definition = self.content.actor(&actor.kind_id)?;
        if !definition.tags.iter().any(|tag| tag == "chameleon") {
            return Some(definition);
        }
        actor
            .appearance_kind_id
            .as_deref()
            .and_then(|kind_id| self.content.actor(kind_id))
            .or(Some(definition))
    }

    pub(super) fn visible_eldritch_horror_entity_ids(&self) -> BTreeSet<String> {
        if self.map_scale != MapScaleDto::Local {
            return BTreeSet::new();
        }
        self.entities
            .iter()
            .filter(|actor| {
                actor.hp > 0
                    && !self.actor_is_player_aligned(actor)
                    && self.entity_is_visible_to_player(actor)
                    && self
                        .actor_runtime_definition(actor)
                        .is_some_and(|definition| {
                            definition.tags.iter().any(|tag| tag == ELDRITCH_HORROR_TAG)
                        })
            })
            .map(|actor| actor.id.clone())
            .collect()
    }

    pub(super) fn resolve_newly_visible_eldritch_horrors(
        &mut self,
        previously_visible: &BTreeSet<String>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let newly_visible = self
            .visible_eldritch_horror_entity_ids()
            .difference(previously_visible)
            .cloned()
            .collect::<Vec<_>>();
        for entity_id in newly_visible {
            let Some(index) = self.entities.iter().position(|actor| actor.id == entity_id) else {
                continue;
            };
            self.resolve_eldritch_horror(index, events, changed);
        }
    }

    fn eldritch_percent_save(&mut self, chance: i32) -> bool {
        self.rng.bounded(100) < u64::try_from(chance.clamp(0, 100)).unwrap_or(0)
    }

    fn eldritch_saving_throw(&mut self, power: i32) -> bool {
        let chance = self
            .player_derived_stats()
            .saving_throw_skill
            .value
            .saturating_sub(power);
        self.eldritch_percent_save(chance)
    }

    fn eldritch_resistance_save(&mut self, damage_type: DamageType, status_kind_id: &str) -> bool {
        if self.player_status_immunities().contains(status_kind_id) {
            return true;
        }
        let percent = self
            .effective_player_resistances()
            .level(damage_type)
            .reduction_percent()
            .max(0);
        self.rng.bounded(ELDRITCH_HIGH_RESISTANCE_ROLL) < u64::try_from(percent).unwrap_or(0)
    }

    fn apply_eldritch_status(&mut self, status_kind_id: &str, duration: u32, source_kind_id: &str) {
        self.apply_player_melee_status(
            status_kind_id,
            i32::try_from(duration).unwrap_or(i32::MAX),
            source_kind_id,
        );
    }

    fn apply_eldritch_confusion(&mut self, source_kind_id: &str) {
        if !self.eldritch_resistance_save(DamageType::Confusion, STATUS_CONFUSION) {
            let duration = u32::try_from(self.rng.bounded(4) + 4).unwrap_or(u32::MAX);
            self.apply_eldritch_status(STATUS_CONFUSION, duration, source_kind_id);
        }
    }

    fn apply_eldritch_hallucination(&mut self, source_kind_id: &str, one_in_three: bool) {
        if self.eldritch_resistance_save(DamageType::Chaos, STATUS_HALLUCINATION)
            || (one_in_three && self.rng.bounded(3) != 0)
        {
            return;
        }
        let duration = u32::try_from(self.rng.bounded(25) + 15).unwrap_or(u32::MAX);
        self.apply_eldritch_status(STATUS_HALLUCINATION, duration, source_kind_id);
    }

    fn drain_eldritch_attribute(&mut self, attribute: AttributeKind) -> bool {
        if self
            .player_equipment_passives()
            .contains(&attribute_sustain_passive(attribute))
        {
            return false;
        }
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let changed = apply_attribute_drain(&mut self.progress, attribute, &mut self.rng).changed;
        if changed {
            self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        }
        changed
    }

    fn apply_eldritch_brain_smash(&mut self, source_kind_id: &str) {
        self.apply_eldritch_confusion(source_kind_id);
        if !self.player_status_immunities().contains(STATUS_PARALYSIS) {
            let duration = u32::try_from(self.rng.bounded(4) + 1).unwrap_or(u32::MAX);
            self.apply_eldritch_status(STATUS_PARALYSIS, duration, source_kind_id);
        }
        for attribute in [AttributeKind::Intelligence, AttributeKind::Wisdom] {
            while self.progress.attributes.value(attribute) > 3
                && self.rng.bounded(100)
                    > u64::try_from(
                        self.player_derived_stats()
                            .saving_throw_skill
                            .value
                            .clamp(0, 100),
                    )
                    .unwrap_or(0)
            {
                if !self.drain_eldritch_attribute(attribute) {
                    break;
                }
            }
        }
        self.apply_eldritch_hallucination(source_kind_id, false);
    }

    fn resolve_eldritch_permanent_insanity(
        &mut self,
        events: &mut Vec<DomainEvent>,
    ) -> &'static str {
        let moronic = self
            .progress
            .active_mutation_ids
            .contains(MORONIC_MUTATION_ID);
        let cowardly = self
            .progress
            .active_mutation_ids
            .contains(COWARDICE_MUTATION_ID);
        let hallucinating = self
            .progress
            .active_mutation_ids
            .contains(HALLUCINATION_MUTATION_ID);
        if moronic
            && (cowardly || self.eldritch_resistance_save(DamageType::Fear, STATUS_FEAR))
            && (hallucinating
                || self.eldritch_resistance_save(DamageType::Chaos, STATUS_HALLUCINATION))
        {
            return "unaffected";
        }
        loop {
            match self.rng.bounded(21) + 1 {
                1 if !self
                    .progress
                    .active_mutation_ids
                    .contains(MORONIC_MUTATION_ID) =>
                {
                    if self.rng.bounded(5) == 0 {
                        self.gain_mutation(MORONIC_MUTATION_ID, events);
                        return "permanent-insanity";
                    }
                    return "fleeting-insanity";
                }
                2..=11
                    if !self
                        .progress
                        .active_mutation_ids
                        .contains(COWARDICE_MUTATION_ID)
                        && !self.eldritch_resistance_save(DamageType::Fear, STATUS_FEAR) =>
                {
                    self.gain_mutation(COWARDICE_MUTATION_ID, events);
                    return "permanent-insanity";
                }
                12..=21
                    if !self
                        .progress
                        .active_mutation_ids
                        .contains(HALLUCINATION_MUTATION_ID)
                        && !self
                            .eldritch_resistance_save(DamageType::Chaos, STATUS_HALLUCINATION) =>
                {
                    self.gain_mutation(HALLUCINATION_MUTATION_ID, events);
                    return "permanent-insanity";
                }
                _ => {}
            }
        }
    }

    pub(super) fn resolve_eldritch_horror(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self
            .player_status_immunities()
            .contains(ELDRITCH_HORROR_IMMUNITY)
            || self.entity_is_player_aligned(index)
        {
            return;
        }
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("eldritch horror definition must remain available")
            .clone();
        if !definition.tags.iter().any(|tag| tag == ELDRITCH_HORROR_TAG) {
            return;
        }
        let mut power = i32::try_from(definition.level / 2).unwrap_or(i32::MAX);
        if definition.tags.iter().any(|tag| tag == "unique") {
            power = power.saturating_mul(2);
        } else if definition
            .allocation
            .as_ref()
            .is_some_and(|allocation| allocation.friends.is_some())
        {
            power /= 2;
        }
        if !self.eldritch_percent_save(power)
            || (self.entities[index].eldritch_horror_triggered
                && self.rng.bounded(ELDRITCH_HORROR_REPEAT_ONE_IN) != 0)
            || self.eldritch_saving_throw(power)
        {
            return;
        }
        let source_entity_id = self.entities[index].id.clone();
        let source_kind_id = definition.id.clone();
        if self.player_has_status_kind(STATUS_HALLUCINATION) {
            if self.rng.bounded(3) == 0 {
                let duration =
                    u32::try_from(self.rng.bounded(u64::from(definition.level.max(1))) + 1)
                        .unwrap_or(u32::MAX);
                self.apply_eldritch_status(STATUS_HALLUCINATION, duration, &source_kind_id);
            }
            events.push(DomainEvent::EldritchHorror {
                source_entity_id,
                source_kind_id,
                power: u16::try_from(power).unwrap_or(u16::MAX),
                outcome: "hallucinating",
            });
            return;
        }
        let (is_demon, is_undead) = self
            .character_definitions()
            .map(|(_, race, _, _)| {
                (
                    race.tags.iter().any(|tag| tag == "demon"),
                    race.tags.iter().any(|tag| tag == "undead"),
                )
            })
            .unwrap_or_default();
        if is_demon
            || (is_undead && self.eldritch_percent_save(25 + i32::from(self.progress.level)))
        {
            events.push(DomainEvent::EldritchHorror {
                source_entity_id,
                source_kind_id,
                power: u16::try_from(power).unwrap_or(u16::MAX),
                outcome: "unaffected",
            });
            return;
        }
        self.entities[index].eldritch_horror_triggered = true;
        let outcome = if !self.eldritch_saving_throw(power) {
            self.apply_eldritch_confusion(&source_kind_id);
            self.apply_eldritch_hallucination(&source_kind_id, true);
            "mind-blast"
        } else if !self.eldritch_saving_throw(power) {
            for attribute in [
                AttributeKind::Intelligence,
                AttributeKind::Wisdom,
                AttributeKind::Charisma,
            ] {
                self.drain_eldritch_attribute(attribute);
            }
            "attribute-drain"
        } else if !self.eldritch_saving_throw(power) {
            self.apply_eldritch_brain_smash(&source_kind_id);
            "brain-smash"
        } else if !self.eldritch_saving_throw(power) {
            self.clear_current_floor_memory(changed);
            "amnesia"
        } else if self.eldritch_saving_throw(power) {
            "resisted"
        } else {
            self.resolve_eldritch_permanent_insanity(events)
        };
        events.push(DomainEvent::EldritchHorror {
            source_entity_id,
            source_kind_id,
            power: u16::try_from(power).unwrap_or(u16::MAX),
            outcome,
        });
    }

    fn roll_chameleon_form(&mut self, actor_kind_id: &str, position: Position) -> Option<String> {
        let terrain_id = self
            .index(position)
            .map(|index| self.terrain[index].clone());
        self.roll_chameleon_form_on_terrain(actor_kind_id, terrain_id.as_deref())
    }

    fn roll_chameleon_form_on_terrain(
        &mut self,
        actor_kind_id: &str,
        terrain_id: Option<&str>,
    ) -> Option<String> {
        let level = self.content.actor(actor_kind_id)?.level;
        let terrain = terrain_id.and_then(|terrain_id| self.content.terrain(terrain_id));
        let mut candidates =
            self.content
                .actor_definitions()
                .filter_map(|definition| {
                    let allocation = definition.allocation.as_ref()?;
                    let explodes = definition.melee_routine.as_ref().is_some_and(|routine| {
                        routine.blows.iter().any(|blow| blow.self_destructs)
                    });
                    (definition.role == ActorRole::Monster
                        && definition.id != actor_kind_id
                        && definition.level <= level
                        && (allocation.max_depth == 0
                            || allocation.max_depth >= u16::try_from(level).unwrap_or(u16::MAX))
                        && !definition.friendly
                        && !actor_is_unique(definition)
                        && !allocation.multiplies
                        && !definition.tags.iter().any(|tag| tag == "chameleon")
                        && !explodes
                        && terrain
                            .as_ref()
                            .is_none_or(|terrain| actor_can_cross_terrain(definition, terrain)))
                    .then(|| OriginalAllocationCandidate {
                        kind_id: definition.id.clone(),
                        level: definition.level,
                        legacy_index: allocation.legacy_index,
                        weight: 100 / allocation.rarity,
                    })
                })
                .filter(|candidate| candidate.weight > 0)
                .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| (candidate.level, candidate.legacy_index));
        let total = candidates
            .iter()
            .map(|candidate| u64::from(candidate.weight))
            .sum::<u64>();
        if total == 0 {
            return None;
        }
        let mut roll = self.rng.bounded(total);
        for candidate in candidates {
            if roll < u64::from(candidate.weight) {
                return Some(candidate.kind_id);
            }
            roll -= u64::from(candidate.weight);
        }
        None
    }

    pub(super) fn apply_chameleon_form(&mut self, index: usize, form_kind_id: &str) {
        let definition = self
            .content
            .actor(form_kind_id)
            .expect("selected chameleon form must remain available")
            .clone();
        let new_max_hp = actor_spawn_max_hp(&mut self.rng, &definition).max(1);
        let actor = &mut self.entities[index];
        let old_max_hp = actor.max_hp.max(1);
        actor.hp = i32::try_from(
            i64::from(actor.hp)
                .saturating_mul(i64::from(new_max_hp))
                .saturating_div(i64::from(old_max_hp)),
        )
        .unwrap_or(i32::MAX)
        .max(1);
        actor.max_hp = new_max_hp;
        actor.speed = definition.speed;
        actor.resistances = definition_resistance_profile(&definition);
        actor.casting_cooldown_remaining = 0;
        if !definition
            .monster_casting
            .as_ref()
            .is_some_and(|casting| casting.smart)
        {
            actor.observed_player_resistances.clear();
        }
        actor.visible_invisible = false;
        actor.appearance_kind_id = Some(definition.id);
    }

    pub(super) fn maybe_change_chameleon_form(&mut self, index: usize) -> bool {
        let actor_kind_id = self.entities[index].kind_id.clone();
        if !self
            .content
            .actor(&actor_kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "chameleon"))
            || self.rng.bounded(CHAMELEON_CHANGE_ONE_IN) != 0
        {
            return false;
        }
        let position = self.entities[index].position;
        let Some(form_kind_id) = self.roll_chameleon_form(&actor_kind_id, position) else {
            return false;
        };
        self.apply_chameleon_form(index, &form_kind_id);
        true
    }

    pub(super) fn current_floor_task_id(&self) -> Option<&str> {
        self.content.world(&self.world_id).and_then(|world| {
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == self.current_floor_id)?
                .task_id
                .as_deref()
        })
    }

    pub(super) fn maybe_initialize_chameleon_form(&mut self, actor: &mut Actor) -> bool {
        if !self
            .content
            .actor(&actor.kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "chameleon"))
        {
            return false;
        }
        let terrain_id = self
            .index(actor.position)
            .and_then(|index| self.terrain.get(index).cloned());
        self.maybe_initialize_chameleon_form_on_terrain(actor, terrain_id.as_deref())
    }

    pub(super) fn maybe_initialize_chameleon_form_on_terrain(
        &mut self,
        actor: &mut Actor,
        terrain_id: Option<&str>,
    ) -> bool {
        if self
            .content
            .actor(&actor.kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "chameleon"))
        {
            if actor.appearance_kind_id.is_some() {
                return true;
            }
            let actor_kind_id = actor.kind_id.clone();
            if let Some(form_kind_id) =
                self.roll_chameleon_form_on_terrain(&actor_kind_id, terrain_id)
            {
                let definition = self
                    .content
                    .actor(&form_kind_id)
                    .expect("selected chameleon form must remain available")
                    .clone();
                let new_max_hp = actor_spawn_max_hp(&mut self.rng, &definition).max(1);
                actor.hp = new_max_hp;
                actor.max_hp = new_max_hp;
                actor.speed = definition.speed;
                actor.resistances = definition_resistance_profile(&definition);
                actor.casting_cooldown_remaining = 0;
                actor.observed_player_resistances.clear();
                actor.visible_invisible = false;
                actor.appearance_kind_id = Some(form_kind_id);
            }
            return true;
        }
        false
    }

    pub(super) fn maybe_apply_shadower_appearance(&mut self, actor: &mut Actor) {
        if self.maybe_initialize_chameleon_form(actor) {
            return;
        }
        if self
            .content
            .actor(&actor.kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "shapechanger"))
        {
            actor.appearance_kind_id = self.roll_shapechanger_appearance(&actor.kind_id);
            return;
        }
        let eligible = self
            .content
            .actor(&actor.kind_id)
            .is_some_and(|definition| {
                definition.level >= 10
                    && !definition.tags.iter().any(|tag| tag == "unique")
                    && definition.id != SHADOWER_APPEARANCE_KIND_ID
            })
            && self
                .content
                .actor(SHADOWER_APPEARANCE_KIND_ID)
                .is_some_and(|definition| {
                    definition
                        .tags
                        .iter()
                        .any(|tag| tag == "shadower-appearance")
                });
        if eligible && self.rng.bounded(SHADOWER_ONE_IN) == 0 {
            actor.appearance_kind_id = Some(SHADOWER_APPEARANCE_KIND_ID.to_owned());
        }
    }

    fn roll_shapechanger_appearance(&mut self, actor_kind_id: &str) -> Option<String> {
        let candidates = self
            .content
            .actor_definitions()
            .filter(|definition| {
                definition.role == ActorRole::Monster
                    && definition.id != actor_kind_id
                    && !definition
                        .tags
                        .iter()
                        .any(|tag| tag == "shadower-appearance")
            })
            .map(|definition| definition.id.clone())
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let index = usize::try_from(self.rng.bounded(candidates.len() as u64)).ok()?;
        candidates.get(index).cloned()
    }

    pub(super) fn reroll_shapechanger_appearance(&mut self, index: usize) {
        let kind_id = self.entities[index].kind_id.clone();
        if !self
            .content
            .actor(&kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "shapechanger"))
        {
            return;
        }
        self.entities[index].appearance_kind_id = self.roll_shapechanger_appearance(&kind_id);
    }

    fn original_pack_spell_flags(&self, leader: &ActorDefinition) -> (bool, bool) {
        fn classify(effect: &AbilityEffectDefinition) -> (bool, bool) {
            match effect {
                AbilityEffectDefinition::Damage { .. }
                | AbilityEffectDefinition::AreaDamage { .. }
                | AbilityEffectDefinition::BeamDamage { .. }
                | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                | AbilityEffectDefinition::ConeDamage { .. }
                | AbilityEffectDefinition::BreathDamage { .. }
                | AbilityEffectDefinition::CurseDamage { .. }
                | AbilityEffectDefinition::DeathRay { .. }
                | AbilityEffectDefinition::DrainLife { .. } => (true, false),
                AbilityEffectDefinition::Summon { .. }
                | AbilityEffectDefinition::SummonCategory { .. } => (false, true),
                AbilityEffectDefinition::Sequence { effects } => effects
                    .iter()
                    .map(classify)
                    .fold((false, false), |left, right| {
                        (left.0 || right.0, left.1 || right.1)
                    }),
                AbilityEffectDefinition::RandomChoice { branches, .. } => branches
                    .iter()
                    .map(|branch| classify(&branch.effect))
                    .fold((false, false), |left, right| {
                        (left.0 || right.0, left.1 || right.1)
                    }),
                _ => (false, false),
            }
        }

        leader
            .monster_casting
            .iter()
            .flat_map(|casting| &casting.abilities)
            .filter_map(|candidate| self.content.ability(&candidate.ability_id))
            .map(|ability| classify(&ability.effect))
            .fold((false, false), |left, right| {
                (left.0 || right.0, left.1 || right.1)
            })
    }

    pub(super) fn original_pack_behavior(
        &mut self,
        leader: &ActorDefinition,
        has_leader: bool,
        count: usize,
    ) -> MonsterPackBehaviorDto {
        let (has_attack_spell, has_summon_spell) = self.original_pack_spell_flags(leader);
        let roll = self.rng.bounded(if count == 1 {
            if leader.damage_dice > 0 && leader.damage_sides > 0 {
                100
            } else {
                25
            }
        } else {
            10
        });
        if count == 1 {
            return if roll < 5 && has_attack_spell {
                MonsterPackBehaviorDto::Shoot
            } else if roll < 15 && has_summon_spell {
                MonsterPackBehaviorDto::Lure
            } else if (roll < 25 && has_attack_spell)
                || leader.damage_dice == 0
                || leader.damage_sides == 0
            {
                MonsterPackBehaviorDto::MaintainDistance
            } else {
                MonsterPackBehaviorDto::Seek
            };
        }
        if leader.tags.iter().any(|tag| tag == "animal") {
            return match roll {
                0..=2 => MonsterPackBehaviorDto::Seek,
                3 if has_attack_spell => MonsterPackBehaviorDto::Shoot,
                _ => MonsterPackBehaviorDto::Lure,
            };
        }
        match roll {
            0..=5 => MonsterPackBehaviorDto::Seek,
            6 | 7 => MonsterPackBehaviorDto::Lure,
            8 if has_leader => MonsterPackBehaviorDto::GuardLeader,
            8 if has_attack_spell => MonsterPackBehaviorDto::GuardPosition,
            8 => MonsterPackBehaviorDto::Lure,
            9 if has_attack_spell => MonsterPackBehaviorDto::Shoot,
            _ => MonsterPackBehaviorDto::Seek,
        }
    }
}

#[cfg(test)]
mod w3_tests {
    use super::*;

    #[test]
    fn daylight_excludes_light_vulnerable_wilderness_monsters() {
        let game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
            .expect("Warrens journey should create");
        let cave_spider = game
            .content
            .actor("demo.actor.cave-spider")
            .expect("light-vulnerable actor should remain available");

        assert!(!actor_matches_wilderness_daytime(cave_spider, true));
        assert!(actor_matches_wilderness_daytime(cave_spider, false));
    }
}

#[derive(Debug, Clone)]
struct OriginalAllocationCandidate {
    kind_id: String,
    level: u32,
    legacy_index: u32,
    weight: u32,
}

fn actor_is_unique(definition: &ActorDefinition) -> bool {
    definition.tags.iter().any(|tag| tag == "unique")
}

fn actor_is_guardian(definition: &ActorDefinition) -> bool {
    definition.tags.iter().any(|tag| tag == "guardian")
}

fn habitat_tag(habitat: ActorHabitat) -> Option<&'static str> {
    match habitat {
        ActorHabitat::All => None,
        ActorHabitat::Grass => Some("grass"),
        ActorHabitat::Mountain => Some("mountain"),
        ActorHabitat::Shore => Some("shore"),
        ActorHabitat::Snow => Some("snow"),
        ActorHabitat::Swamp => Some("swamp"),
        ActorHabitat::Town => Some("town"),
        ActorHabitat::Volcano => Some("volcano"),
        ActorHabitat::Waste => Some("waste"),
        ActorHabitat::Wood => Some("wood"),
    }
}

fn actor_matches_surface_habitat(
    definition: &ActorDefinition,
    terrain: &rfb_content::TerrainDefinition,
) -> bool {
    let aquatic = definition
        .movement
        .modes
        .contains(&ActorMovementMode::Aquatic);
    if aquatic && terrain.tags.iter().any(|tag| tag == "water") {
        return true;
    }
    let Some(allocation) = &definition.allocation else {
        return false;
    };
    allocation.wild_only
        && allocation.habitats.iter().any(|habitat| {
            habitat_tag(*habitat)
                .is_none_or(|required| terrain.tags.iter().any(|tag| tag == required))
        })
}

fn actor_matches_wilderness_daytime(definition: &ActorDefinition, daytime: bool) -> bool {
    !daytime
        || definition.resistances.get(&ActorDamageType::Light)
            != Some(&ActorResistanceLevel::Vulnerable)
}

fn escort_alignment_is_compatible(leader: &ActorDefinition, escort: &ActorDefinition) -> bool {
    let leader_good = leader.tags.iter().any(|tag| tag == "good");
    let leader_evil = leader.tags.iter().any(|tag| tag == "evil");
    let escort_good = escort.tags.iter().any(|tag| tag == "good");
    let escort_evil = escort.tags.iter().any(|tag| tag == "evil");
    !(leader_good && escort_evil || leader_evil && escort_good)
}

fn terrain_at_generated_position<'a>(
    content: &'a ContentCatalog,
    terrain: &[String],
    width: u16,
    height: u16,
    position: Position,
) -> Option<&'a rfb_content::TerrainDefinition> {
    if position.x < 0
        || position.y < 0
        || position.x >= i32::from(width)
        || position.y >= i32::from(height)
    {
        return None;
    }
    let index = usize::try_from(position.y).ok()? * usize::from(width)
        + usize::try_from(position.x).ok()?;
    content.terrain(terrain.get(index)?)
}

fn generated_line_of_effect(
    content: &ContentCatalog,
    terrain: &[String],
    width: u16,
    height: u16,
    from: Position,
    to: Position,
) -> bool {
    let mut x = from.x;
    let mut y = from.y;
    let dx = (to.x - from.x).abs();
    let dy = (to.y - from.y).abs();
    let step_x = if from.x < to.x { 1 } else { -1 };
    let step_y = if from.y < to.y { 1 } else { -1 };
    let mut error = dx - dy;
    loop {
        if x == to.x && y == to.y {
            return true;
        }
        let position = Position { x, y };
        if position != from
            && !terrain_at_generated_position(content, terrain, width, height, position)
                .is_some_and(|tile| tile.walkable)
        {
            return false;
        }
        let double_error = error * 2;
        if double_error > -dy {
            error -= dy;
            x += step_x;
        }
        if double_error < dx {
            error += dx;
            y += step_y;
        }
        if terrain_at_generated_position(content, terrain, width, height, Position { x, y })
            .is_none()
        {
            return false;
        }
    }
}

impl Game {
    fn select_surface_allocated_monster(
        &mut self,
        level: u16,
        terrain: &rfb_content::TerrainDefinition,
        target_floor_kind_ids: &[String],
    ) -> Option<String> {
        let daytime = self.wilderness_is_daytime();
        let mut candidates = self
            .content
            .actor_definitions()
            .filter(|definition| {
                let Some(allocation) = &definition.allocation else {
                    return false;
                };
                definition.role == ActorRole::Monster
                    && !actor_is_guardian(definition)
                    && definition.level <= u32::from(level)
                    && (allocation.max_depth == 0 || allocation.max_depth >= level)
                    && (!actor_is_unique(definition)
                        || (self.unique_actor_kind_is_available(&definition.id)
                            && !target_floor_kind_ids
                                .iter()
                                .any(|kind_id| kind_id == &definition.id)))
                    && actor_matches_surface_habitat(definition, terrain)
                    && actor_matches_wilderness_daytime(definition, daytime)
                    && actor_can_cross_terrain(definition, terrain)
            })
            .map(|definition| {
                let allocation = definition
                    .allocation
                    .as_ref()
                    .expect("surface candidate must retain allocation metadata");
                OriginalAllocationCandidate {
                    kind_id: definition.id.clone(),
                    level: definition.level,
                    legacy_index: allocation.legacy_index,
                    weight: 100 / allocation.rarity,
                }
            })
            .filter(|candidate| candidate.weight > 0)
            .collect::<Vec<_>>();
        candidates.sort_by_key(|candidate| (candidate.level, candidate.legacy_index));
        let total = candidates
            .iter()
            .map(|candidate| u64::from(candidate.weight))
            .sum::<u64>();
        if total == 0 {
            return None;
        }
        let mut roll = self.rng.bounded(total);
        for candidate in candidates {
            if roll < u64::from(candidate.weight) {
                return Some(candidate.kind_id);
            }
            roll -= u64::from(candidate.weight);
        }
        None
    }

    pub(super) fn initialize_surface_monsters(&mut self) {
        let Some(allocation) = self
            .content
            .world(&self.world_id)
            .and_then(|world| world.surface_actor_allocation)
        else {
            return;
        };
        self.initialize_surface_monsters_with(allocation.rolls, allocation.level, "surface");
    }

    pub(super) fn initialize_wilderness_monsters(
        &mut self,
        level: u16,
        rolls: u16,
        spawn_kind: &str,
    ) {
        self.initialize_surface_monsters_with(rolls, level, spawn_kind);
    }

    fn initialize_surface_monsters_with(&mut self, rolls: u16, level: u16, spawn_kind: &str) {
        let mut occupied = self
            .entities
            .iter()
            .map(|entity| entity.position)
            .chain(std::iter::once(self.player.position))
            .collect::<BTreeSet<_>>();
        let mut target_floor_kind_ids = self
            .entities
            .iter()
            .map(|entity| entity.kind_id.clone())
            .collect::<Vec<_>>();
        let terrain = self.terrain.clone();
        let group_policy = GlobalMonsterAllocationDefinition {
            preferred_glyphs: Vec::new(),
            special_div: 64,
            ambient_chance_one_in: 1,
        };
        for ordinal in 0..rolls {
            let mut positions = (1..self.height.saturating_sub(1))
                .flat_map(|y| {
                    (1..self.width.saturating_sub(1)).map(move |x| Position {
                        x: i32::from(x),
                        y: i32::from(y),
                    })
                })
                .filter(|position| {
                    !occupied.contains(position)
                        && rfb_distance(*position, self.player.position) > 10
                        && terrain_at_generated_position(
                            &self.content,
                            &terrain,
                            self.width,
                            self.height,
                            *position,
                        )
                        .is_some_and(|tile| tile.tags.iter().any(|tag| tag == "surface"))
                })
                .collect::<Vec<_>>();
            positions.sort_by_key(|position| (position.y, position.x));
            if positions.is_empty() {
                break;
            }
            let position = positions[usize::try_from(
                self.rng
                    .bounded(u64::try_from(positions.len()).unwrap_or(u64::MAX)),
            )
            .unwrap_or(0)];
            let Some(required_terrain) = terrain_at_generated_position(
                &self.content,
                &terrain,
                self.width,
                self.height,
                position,
            )
            .cloned() else {
                continue;
            };
            let Some(kind_id) = self.select_surface_allocated_monster(
                level,
                &required_terrain,
                &target_floor_kind_ids,
            ) else {
                continue;
            };
            occupied.insert(position);
            let mut members = self.plan_original_group(
                &group_policy,
                &kind_id,
                position,
                level,
                None,
                &terrain,
                self.width,
                self.height,
                &mut occupied,
            );
            let daytime = self.wilderness_is_daytime();
            members.retain(|member| {
                let eligible = self
                    .content
                    .actor(&member.kind_id)
                    .is_some_and(|definition| {
                        actor_matches_wilderness_daytime(definition, daytime)
                    });
                if !eligible {
                    occupied.remove(&member.position);
                }
                eligible
            });
            let definition = self
                .content
                .actor(&kind_id)
                .expect("surface actor definition must remain available")
                .clone();
            let pack_behavior = (!members.is_empty()).then(|| {
                self.original_pack_behavior(
                    &definition,
                    members
                        .iter()
                        .any(|member| member.role == OriginalGroupRole::Escort),
                    members.len() + 1,
                )
            });
            let surface_id = if self.is_wilderness_floor() {
                let position = self
                    .wilderness_position
                    .expect("local wilderness must retain its world position");
                format!("{}.{}.{}", self.current_floor_id, position.x, position.y)
            } else {
                self.current_floor_id.clone()
            };
            let leader_id = format!("{surface_id}.{spawn_kind}.{}", ordinal + 1);
            let pack_id = format!("{leader_id}.pack");
            let mut leader = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &leader_id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                actor_starts_alerted(&definition),
            );
            self.maybe_apply_shadower_appearance(&mut leader);
            if let Some(behavior) = pack_behavior {
                leader.pack = Some(MonsterPackIdentity {
                    id: pack_id.clone(),
                    leader_id: leader_id.clone(),
                    role: MonsterPackRoleDto::Leader,
                    behavior,
                });
            }
            target_floor_kind_ids.push(kind_id);
            self.entities.push(leader);
            for (member_ordinal, member) in members.into_iter().enumerate() {
                let member_definition = self
                    .content
                    .actor(&member.kind_id)
                    .expect("surface companion definition must remain available")
                    .clone();
                let mut actor = spawn_actor_from_definition(
                    &mut self.rng,
                    &member_definition,
                    &format!("{leader_id}.companion.{}", member_ordinal + 1),
                    member.position,
                    INITIAL_MONSTER_ENERGY_NEED,
                    actor_starts_alerted(&member_definition),
                );
                self.maybe_apply_shadower_appearance(&mut actor);
                actor.pack = Some(MonsterPackIdentity {
                    id: pack_id.clone(),
                    leader_id: leader_id.clone(),
                    role: MonsterPackRoleDto::Member,
                    behavior: pack_behavior.expect("surface pack must retain behavior"),
                });
                self.entities.push(actor);
            }
        }
    }

    pub(super) fn unique_actor_kind_is_available(&self, kind_id: &str) -> bool {
        if self.defeated_unique_actor_kind_ids.contains(kind_id)
            || self
                .entities
                .iter()
                .any(|actor| actor.kind_id == kind_id && actor.hp > 0)
        {
            return false;
        }
        !self.stored_floors.values().any(|floor| {
            floor
                .entities
                .iter()
                .any(|actor| actor.kind_id == kind_id && actor.hp > 0)
        })
    }

    pub(super) fn original_allocation_level(&mut self, base_level: u16) -> u16 {
        let mut level = base_level;
        if level == 0 {
            return level;
        }
        for _ in 0..2 {
            if self.rng.bounded(ORIGINAL_NASTY_MON_ONE_IN) == 0 {
                let bonus = level.saturating_div(10).min(5).saturating_add(2);
                level = level.saturating_add(bonus);
            }
        }
        level
    }

    pub(super) fn original_dungeon_weight(
        &mut self,
        definition: &ActorDefinition,
        policy: &GlobalMonsterAllocationDefinition,
    ) -> u32 {
        let allocation = definition
            .allocation
            .as_ref()
            .expect("allocation candidate must retain metadata");
        let base = 100 / allocation.rarity;
        if policy
            .preferred_glyphs
            .iter()
            .any(|glyph| glyph == &definition.glyph)
        {
            return base;
        }
        let scaled = u64::from(base) * u64::from(policy.special_div);
        let mut weight = u32::try_from(scaled / 64).unwrap_or(u32::MAX);
        let rounded = *self
            .monster_division_remainders
            .entry(definition.id.clone())
            .or_insert_with(|| self.rng.bounded(64) < scaled % 64);
        if rounded {
            weight = weight.saturating_add(1);
        }
        weight
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn select_original_allocated_monster(
        &mut self,
        policy: &GlobalMonsterAllocationDefinition,
        base_level: u16,
        floor_depth: u16,
        current_task_id: Option<&str>,
        target_floor_kind_ids: &[String],
        escort_leader_kind_id: Option<&str>,
        required_terrain: Option<&rfb_content::TerrainDefinition>,
    ) -> Option<String> {
        let current_legacy_dungeon_index = self.content.world(&self.world_id).and_then(|world| {
            let dungeon_id = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == self.current_floor_id)?
                .dungeon_id
                .as_deref()?;
            world
                .dungeons
                .iter()
                .find(|dungeon| dungeon.id == dungeon_id)?
                .legacy_index
        });
        let unique_count = target_floor_kind_ids
            .iter()
            .filter(|kind_id| self.content.actor(kind_id).is_some_and(actor_is_unique))
            .count();
        let allow_uniques = unique_count == 0 || {
            let odds = u64::try_from(unique_count.saturating_add(1))
                .unwrap_or(u64::MAX)
                .saturating_pow(2);
            self.rng.bounded(odds) == 0
        };
        let selection_level = self.original_allocation_level(base_level);
        let escort_leader = escort_leader_kind_id
            .and_then(|kind_id| self.content.actor(kind_id))
            .cloned();
        let mut definitions = self
            .content
            .actor_definitions()
            .filter(|definition| {
                let Some(allocation) = &definition.allocation else {
                    return false;
                };
                if definition.role != ActorRole::Monster
                    || allocation.wild_only
                    || actor_is_guardian(definition)
                    || definition.level > u32::from(selection_level)
                    || (allocation.max_depth != 0 && allocation.max_depth < selection_level)
                    || (allocation.force_depth && definition.level > u32::from(floor_depth))
                    || !actor_allocation_matches_task(allocation, current_task_id)
                    || !actor_allocation_matches_legacy_dungeon(
                        allocation,
                        current_legacy_dungeon_index,
                    )
                    || (actor_is_unique(definition)
                        && (!allow_uniques
                            || !self.unique_actor_kind_is_available(&definition.id)
                            || target_floor_kind_ids
                                .iter()
                                .any(|kind_id| kind_id == &definition.id)))
                {
                    return false;
                }
                if required_terrain
                    .is_some_and(|terrain| !actor_can_cross_terrain(definition, terrain))
                {
                    return false;
                }
                escort_leader.as_ref().is_none_or(|leader| {
                    definition.id != leader.id
                        && definition.glyph == leader.glyph
                        && definition.level <= leader.level
                        && !actor_is_unique(definition)
                        && escort_alignment_is_compatible(leader, definition)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        definitions.sort_by_key(|definition| {
            let allocation = definition
                .allocation
                .as_ref()
                .expect("filtered allocation candidate must retain metadata");
            (
                definition.level,
                allocation.legacy_index,
                definition.id.clone(),
            )
        });
        let mut candidates = Vec::new();
        for definition in definitions {
            let allocation = definition
                .allocation
                .as_ref()
                .expect("filtered allocation candidate must retain metadata");
            let mut weight = self.original_dungeon_weight(&definition, policy);
            if weight > 0
                && allocation.max_depth != 999
                && u32::from(selection_level) > definition.level.saturating_add(9)
                && !actor_is_unique(&definition)
            {
                let shift = (u32::from(selection_level) - definition.level) / 10;
                weight = weight.checked_shr(shift).unwrap_or(0).max(1);
            }
            if weight > 0 {
                candidates.push(OriginalAllocationCandidate {
                    kind_id: definition.id,
                    level: definition.level,
                    legacy_index: allocation.legacy_index,
                    weight,
                });
            }
        }
        candidates.sort_by_key(|candidate| (candidate.level, candidate.legacy_index));
        let total = candidates
            .iter()
            .map(|candidate| u64::from(candidate.weight))
            .sum::<u64>();
        if total == 0 {
            return None;
        }
        let mut roll = self.rng.bounded(total);
        for candidate in candidates {
            if roll < u64::from(candidate.weight) {
                return Some(candidate.kind_id);
            }
            roll -= u64::from(candidate.weight);
        }
        None
    }

    fn original_scatter_position(
        &mut self,
        terrain: &[String],
        width: u16,
        height: u16,
        origin: Position,
        radius: i32,
    ) -> Position {
        loop {
            let span = u64::try_from(radius.saturating_mul(2).saturating_add(1))
                .expect("positive scatter span must fit u64");
            let y = origin.y + i32::try_from(self.rng.bounded(span)).unwrap_or(0) - radius;
            let x = origin.x + i32::try_from(self.rng.bounded(span)).unwrap_or(0) - radius;
            let position = Position { x, y };
            if x <= 0
                || y <= 0
                || x + 1 >= i32::from(width)
                || y + 1 >= i32::from(height)
                || (radius > 1 && rfb_distance(origin, position) > radius as u32)
                || !generated_line_of_effect(
                    &self.content,
                    terrain,
                    width,
                    height,
                    origin,
                    position,
                )
            {
                continue;
            }
            return position;
        }
    }

    pub(super) fn original_friend_total(
        &mut self,
        definition: &ActorDefinition,
        depth: u16,
    ) -> u16 {
        let friends = definition
            .allocation
            .as_ref()
            .and_then(|allocation| allocation.friends)
            .expect("friend group must retain dice metadata");
        if friends.chance_percent > 0
            && self.rng.bounded(100).saturating_add(1) > u64::from(friends.chance_percent)
        {
            return 1;
        }
        let total = if friends.dice > 0 {
            self.roll_damage(u16::from(friends.dice), u16::from(friends.sides))
        } else {
            let mut total = i32::try_from(self.rng.bounded(10).saturating_add(1)).unwrap_or(1);
            let mut extra = 0_i32;
            let actor_level = i32::try_from(definition.level).unwrap_or(i32::MAX);
            let depth = i32::from(depth);
            if actor_level > depth {
                let difference = actor_level - depth;
                extra = -i32::try_from(
                    self.rng
                        .bounded(u64::try_from(difference).unwrap_or(u64::MAX))
                        .saturating_add(1),
                )
                .unwrap_or(i32::MAX);
            } else if actor_level < depth {
                let difference = depth - actor_level;
                extra = i32::try_from(
                    self.rng
                        .bounded(u64::try_from(difference).unwrap_or(u64::MAX))
                        .saturating_add(1),
                )
                .unwrap_or(i32::MAX)
                .min(9);
            }
            total += extra;
            total
        };
        u16::try_from(total.clamp(1, i32::from(ORIGINAL_GROUP_MAX))).unwrap_or(1)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn plan_original_group(
        &mut self,
        policy: &GlobalMonsterAllocationDefinition,
        leader_kind_id: &str,
        leader_position: Position,
        depth: u16,
        current_task_id: Option<&str>,
        terrain: &[String],
        width: u16,
        height: u16,
        occupied: &mut BTreeSet<Position>,
    ) -> Vec<OriginalGroupMember> {
        let leader = self
            .content
            .actor(leader_kind_id)
            .expect("placed leader definition must remain available")
            .clone();
        let allocation = leader
            .allocation
            .as_ref()
            .expect("globally allocated leader must retain metadata")
            .clone();
        let mut members = Vec::new();
        if allocation.friends.is_some() {
            let total = self.original_friend_total(&leader, depth);
            let mut placed = vec![leader_position];
            let mut source_index = 0;
            while source_index < placed.len() && placed.len() < usize::from(total) {
                let origin = placed[source_index];
                source_index += 1;
                for _ in 0..8 {
                    if placed.len() >= usize::from(total) {
                        break;
                    }
                    let position =
                        self.original_scatter_position(terrain, width, height, origin, 4);
                    if occupied.contains(&position)
                        || !terrain_at_generated_position(
                            &self.content,
                            terrain,
                            width,
                            height,
                            position,
                        )
                        .is_some_and(|tile| actor_can_cross_terrain(&leader, tile))
                    {
                        continue;
                    }
                    occupied.insert(position);
                    placed.push(position);
                    members.push(OriginalGroupMember {
                        kind_id: leader_kind_id.to_owned(),
                        position,
                        role: OriginalGroupRole::Friend,
                    });
                }
            }
        } else if allocation.escort {
            for _ in 0..ORIGINAL_ESCORT_ATTEMPTS {
                let position =
                    self.original_scatter_position(terrain, width, height, leader_position, 3);
                if occupied.contains(&position) {
                    continue;
                }
                let required_terrain =
                    terrain_at_generated_position(&self.content, terrain, width, height, position)
                        .cloned();
                let Some(required_terrain) = required_terrain else {
                    continue;
                };
                if !required_terrain.walkable && required_terrain.movement_modes.is_empty() {
                    continue;
                }
                let target_floor_kind_ids = members
                    .iter()
                    .map(|member| member.kind_id.clone())
                    .collect::<Vec<_>>();
                // RFB prepares the terrain-filtered allocation table again for
                // every escort position before drawing a candidate.
                self.monster_division_remainders.clear();
                let Some(kind_id) = self.select_original_allocated_monster(
                    policy,
                    u16::try_from(leader.level).unwrap_or(u16::MAX),
                    depth,
                    current_task_id,
                    &target_floor_kind_ids,
                    Some(leader_kind_id),
                    Some(&required_terrain),
                ) else {
                    break;
                };
                occupied.insert(position);
                members.push(OriginalGroupMember {
                    kind_id,
                    position,
                    role: OriginalGroupRole::Escort,
                });
            }
        }
        members
    }

    fn ecology_entity_id(&self, prefix: &str) -> String {
        if self.entities.iter().all(|entity| entity.id != prefix) {
            return prefix.to_owned();
        }
        let mut suffix = 1_u32;
        loop {
            let candidate = format!("{prefix}.{suffix}");
            if self.entities.iter().all(|entity| entity.id != candidate) {
                return candidate;
            }
            suffix = suffix.saturating_add(1);
        }
    }

    pub(super) fn try_original_reproduction(
        &mut self,
        index: usize,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let kind_id = self.entities[index].kind_id.clone();
        if !self
            .content
            .actor(&kind_id)
            .and_then(|definition| definition.allocation.as_ref())
            .is_some_and(|allocation| allocation.multiplies)
            || self
                .entities
                .iter()
                .filter(|entity| {
                    self.content
                        .actor(&entity.kind_id)
                        .and_then(|definition| definition.allocation.as_ref())
                        .is_some_and(|allocation| allocation.multiplies)
                })
                .count()
                >= ORIGINAL_MAX_REPRODUCERS
            || self
                .entities
                .iter()
                .filter(|entity| entity.hp > 0 && entity.kind_id == kind_id)
                .count()
                >= ORIGINAL_MAX_REPRODUCERS
        {
            return false;
        }
        // Original neutral Harmony always passes this check, but randint1(375)
        // still advances the RNG before crowding and placement are considered.
        let _harmony_roll = self.rng.bounded(375).saturating_add(1);
        let origin = self.entities[index].position;
        let adjacent_monsters = self
            .entities
            .iter()
            .filter(|entity| adjacent(origin, entity.position) || entity.position == origin)
            .count();
        if adjacent_monsters >= 4
            || (adjacent_monsters > 0
                && self.rng.bounded(
                    u64::try_from(adjacent_monsters)
                        .unwrap_or(u64::MAX)
                        .saturating_mul(ORIGINAL_MULTIPLY_ADJACENCY_FACTOR),
                ) != 0)
        {
            return false;
        }
        let mut selected = None;
        let mut candidate_count = 0_u64;
        for x in origin.x - 1..=origin.x + 1 {
            for y in origin.y - 1..=origin.y + 1 {
                let position = Position { x, y };
                if position == origin
                    || position == self.player.position
                    || !self.actor_can_enter_position(index, position)
                    || self
                        .entities
                        .iter()
                        .any(|entity| entity.hp > 0 && entity.position == position)
                {
                    continue;
                }
                candidate_count = candidate_count.saturating_add(1);
                if self.rng.bounded(candidate_count) == 0 {
                    selected = Some(position);
                }
            }
        }
        let Some(position) = selected else {
            return false;
        };
        let definition = self
            .content
            .actor(&kind_id)
            .expect("reproducer definition must remain available")
            .clone();
        let id = self.ecology_entity_id(&format!(
            "{}.multiply.{}",
            self.entities[index].id, self.world_tick
        ));
        let mut offspring = spawn_actor_from_definition(
            &mut self.rng,
            &definition,
            &id,
            position,
            INITIAL_MONSTER_ENERGY_NEED,
            true,
        );
        offspring.controller_id = self.entities[index].controller_id.clone();
        offspring.summon = self.entities[index].summon.clone();
        self.entities.push(offspring);
        changed.insert(position);
        true
    }

    pub(super) fn resolve_original_random_movement(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        const DELTAS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        let chance = self
            .actor_runtime_definition(&self.entities[index])
            .and_then(|definition| definition.allocation.as_ref())
            .map_or(0, |allocation| allocation.random_movement_percent);
        if chance == 0 || self.rng.bounded(100) >= u64::from(chance) {
            return Ok(false);
        }
        for _ in 0..4 {
            let delta = DELTAS[usize::try_from(self.rng.bounded(8)).unwrap_or(0)];
            let position = Position {
                x: self.entities[index].position.x + delta.0,
                y: self.entities[index].position.y + delta.1,
            };
            if let Some(target) = self
                .monster_hostile_targets(index)
                .into_iter()
                .find(|target| target.position() == position)
            {
                self.resolve_monster_melee_target(
                    index,
                    &target,
                    events,
                    changed,
                    removed_entities,
                )?;
                return Ok(true);
            }
            if position == self.player.position
                || !self.actor_can_traverse_or_interact(index, position)
                || self.entities.iter().enumerate().any(|(other, entity)| {
                    other != index && entity.hp > 0 && entity.position == position
                })
            {
                continue;
            }
            self.move_entity(index, position, events, changed, removed_entities)?;
            return Ok(true);
        }
        Ok(true)
    }

    pub(super) fn process_ambient_monster_allocation(
        &mut self,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        self.monster_division_remainders.clear();
        let Some((depth, table)) = self
            .content
            .world(&self.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == self.current_floor_id)
            })
            .and_then(|floor| {
                floor.encounter_table_id.as_ref().and_then(|table_id| {
                    self.content
                        .encounter_table(table_id)
                        .map(|table| (floor.depth, table.clone()))
                })
            })
        else {
            return Ok(());
        };
        let Some(policy) = table.global_allocation.as_ref() else {
            return Ok(());
        };
        let chance = u32::from(policy.ambient_chance_one_in)
            .saturating_mul(u32::from(depth).saturating_add(100))
            / 100;
        // Patience is not yet a player system. Its neutral value is zero, so
        // the original final multiplier (375 - Patience) / 375 is exactly one.
        if self.rng.bounded(u64::from(chance.max(1))) != 0 {
            return Ok(());
        }
        let mut leader_position = None;
        for _ in 0..10_000 {
            let position = Position {
                y: i32::try_from(self.rng.bounded(u64::from(self.height))).unwrap_or(0),
                x: i32::try_from(self.rng.bounded(u64::from(self.width))).unwrap_or(0),
            };
            let supports_monster_movement = self
                .index(position)
                .and_then(|index| self.content.terrain(&self.terrain[index]))
                .is_some_and(|terrain| terrain.walkable || !terrain.movement_modes.is_empty());
            if rfb_distance(position, self.player.position) <= ORIGINAL_MAX_SIGHT + 5
                || !supports_monster_movement
                || self
                    .entities
                    .iter()
                    .any(|entity| entity.hp > 0 && entity.position == position)
            {
                continue;
            }
            leader_position = Some(position);
            break;
        }
        let Some(leader_position) = leader_position else {
            return Ok(());
        };
        let target_floor_kind_ids = self
            .entities
            .iter()
            .map(|entity| entity.kind_id.clone())
            .collect::<Vec<_>>();
        let required_terrain = self
            .index(leader_position)
            .and_then(|index| self.content.terrain(&self.terrain[index]))
            .cloned();
        let Some(required_terrain) = required_terrain else {
            return Ok(());
        };
        let current_task_id = self.current_floor_task_id().map(str::to_owned);
        let Some(kind_id) = self.select_original_allocated_monster(
            policy,
            depth,
            depth,
            current_task_id.as_deref(),
            &target_floor_kind_ids,
            None,
            Some(&required_terrain),
        ) else {
            return Ok(());
        };
        let mut occupied = self
            .entities
            .iter()
            .map(|entity| entity.position)
            .chain(std::iter::once(self.player.position))
            .collect::<BTreeSet<_>>();
        occupied.insert(leader_position);
        let terrain = self.terrain.clone();
        let members = self.plan_original_group(
            policy,
            &kind_id,
            leader_position,
            depth,
            current_task_id.as_deref(),
            &terrain,
            self.width,
            self.height,
            &mut occupied,
        );
        let definition = self
            .content
            .actor(&kind_id)
            .expect("ambient actor definition must remain available")
            .clone();
        let pack_behavior = (!members.is_empty()).then(|| {
            self.original_pack_behavior(
                &definition,
                members
                    .iter()
                    .any(|member| member.role == OriginalGroupRole::Escort),
                members.len() + 1,
            )
        });
        let leader_id = self.ecology_entity_id(&format!(
            "{}.ambient.{}",
            self.current_floor_id, self.world_tick
        ));
        let pack_id = format!("{leader_id}.pack");
        let mut leader = spawn_actor_from_definition(
            &mut self.rng,
            &definition,
            &leader_id,
            leader_position,
            INITIAL_MONSTER_ENERGY_NEED,
            actor_starts_alerted(&definition),
        );
        self.maybe_apply_shadower_appearance(&mut leader);
        if let Some(behavior) = pack_behavior {
            leader.pack = Some(MonsterPackIdentity {
                id: pack_id.clone(),
                leader_id: leader_id.clone(),
                role: MonsterPackRoleDto::Leader,
                behavior,
            });
        }
        let mut spawned = vec![leader];
        for (ordinal, member) in members.into_iter().enumerate() {
            let member_definition = self
                .content
                .actor(&member.kind_id)
                .expect("ambient companion definition must remain available")
                .clone();
            let id = format!("{leader_id}.companion.{}", ordinal + 1);
            let mut actor = spawn_actor_from_definition(
                &mut self.rng,
                &member_definition,
                &id,
                member.position,
                INITIAL_MONSTER_ENERGY_NEED,
                actor_starts_alerted(&member_definition),
            );
            self.maybe_apply_shadower_appearance(&mut actor);
            actor.pack = Some(MonsterPackIdentity {
                id: pack_id.clone(),
                leader_id: leader_id.clone(),
                role: MonsterPackRoleDto::Member,
                behavior: pack_behavior.expect("non-empty pack must retain behavior"),
            });
            spawned.push(actor);
        }
        let floor_id = self.current_floor_id.clone();
        let carried = self.generate_carried_loot_for_actors(&spawned, &floor_id, depth)?;
        changed.extend(spawned.iter().map(|actor| actor.position));
        self.entities.extend(spawned);
        self.items.extend(carried);
        Ok(())
    }
}

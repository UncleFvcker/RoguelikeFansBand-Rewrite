// SPDX-License-Identifier: MPL-2.0
use super::*;

impl Game {
    // RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, cmd5.c do_name_pet.
    // Adapt the 16-position input limit to Unicode scalar values.
    pub(crate) fn pet_name_is_valid(name: &str) -> bool {
        !name.is_empty() && name.trim() == name && name.chars().count() <= 16
            && !name.chars().any(|c| c.is_control() || (c.is_whitespace() && c != ' ') || matches!(c,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    }

    pub(super) fn pet_can_be_named(&self, actor: &Actor) -> bool {
        actor.hp > 0
            && self.actor_is_player_aligned(actor)
            && self.entity_is_visible_to_player(actor)
            && !self.entity_is_fuzzy_to_player(actor)
            && self
                .content
                .actor(&actor.kind_id)
                .is_some_and(|kind| !kind.tags.iter().any(|tag| tag == "unique"))
    }

    pub(super) fn set_pet_name(
        &mut self,
        id: &str,
        name: Option<String>,
        events: &mut Vec<DomainEvent>,
    ) {
        let Some(index) = self
            .entities
            .iter()
            .position(|actor| actor.id == id && self.pet_can_be_named(actor))
        else {
            events.push(DomainEvent::PetCommandUnavailable);
            return;
        };
        let name = name.filter(|name| !name.is_empty());
        if name
            .as_deref()
            .is_some_and(|name| !Self::pet_name_is_valid(name))
        {
            events.push(DomainEvent::PetNameInvalid);
            return;
        }
        self.entities[index].custom_name = name.clone();
        events.push(DomainEvent::PetNameChanged {
            name: name.unwrap_or_default(),
        });
    }

    pub(super) fn pet_door_interaction_allowed(&self, index: usize) -> bool {
        !self.entity_is_player_aligned(index) || self.summon_command.open_doors
    }

    pub(super) fn set_pet_option(
        &mut self,
        option: rfb_protocol::PetOptionDto,
        enabled: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        match option {
            rfb_protocol::PetOptionDto::RidingTwoHands => {
                if self.riding_mount_level().is_none() {
                    events.push(DomainEvent::PetCommandUnavailable);
                    return;
                }
                self.summon_command.riding_two_hands = enabled;
            }
            rfb_protocol::PetOptionDto::HighlightMap => {
                self.summon_command.highlight_map = enabled;
                changed.extend(
                    self.entities
                        .iter()
                        .filter(|actor| self.entity_is_visible_to_player(actor))
                        .map(|actor| actor.position),
                );
            }
            rfb_protocol::PetOptionDto::HighlightLists => {
                self.summon_command.highlight_lists = enabled
            }
            rfb_protocol::PetOptionDto::SummonSpells => self.summon_command.summon_spells = enabled,
            rfb_protocol::PetOptionDto::AttackSpells => self.summon_command.attack_spells = enabled,
            rfb_protocol::PetOptionDto::Teleport => self.summon_command.teleport = enabled,
            rfb_protocol::PetOptionDto::AllowPlayerDamage => {
                self.summon_command.allow_player_damage = enabled
            }
            rfb_protocol::PetOptionDto::NoBreeding => self.summon_command.no_breeding = enabled,
            rfb_protocol::PetOptionDto::OpenDoors => self.summon_command.open_doors = enabled,
            rfb_protocol::PetOptionDto::PickupItems => {
                let drop_items = self.summon_command.pickup_items && !enabled;
                self.summon_command.pickup_items = enabled;
                if drop_items {
                    self.drop_pet_carried_items(events, changed);
                }
            }
        }
        events.push(DomainEvent::PetOptionChanged);
    }

    // cmd5.c PET_TAKE_ITEMS drops the current level's pets' carried objects.
    // Keep the existing deterministic drop placement and no-space destruction.
    fn drop_pet_carried_items(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let pets = self
            .entities
            .iter()
            .filter(|actor| actor.hp > 0 && self.actor_is_player_aligned(actor))
            .map(|actor| (actor.id.clone(), (actor.position, actor.kind_id.clone())))
            .collect::<BTreeMap<_, _>>();
        let mut index = 0;
        while index < self.items.len() {
            let item = &self.items[index];
            let pet = match &item.location {
                ItemLocation::CarriedBy { actor_id } => pets.get(actor_id),
                _ => None,
            };
            let Some((origin, source_kind_id)) = pet else {
                index += 1;
                continue;
            };
            if let Some(position) =
                self.ground_drop_position(*origin, item.is_artifact(&self.content))
            {
                let target_kind_id = item.kind_id.clone();
                let quantity = item.quantity;
                self.items[index].location = ItemLocation::Ground(position);
                changed.insert(position);
                events.push(DomainEvent::LootDropped {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id,
                    quantity,
                });
                index += 1;
            } else {
                let item = self.items.remove(index);
                self.item_property_knowledge.remove(&item.id);
                events.push(DomainEvent::ItemDestroyed {
                    target_kind_id: item.kind_id,
                    quantity: item.quantity,
                    rule_line: None,
                });
            }
        }
    }

    // RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c,
    // cmd5.c do_cmd_pet and defines.h PET_*_DIST. Commands do not spend energy.
    pub(super) fn pet_target_exists(&self, id: &str) -> bool {
        self.entities
            .iter()
            .any(|actor| actor.id == id && actor.hp > 0 && !self.actor_is_player_side(actor))
    }

    fn report_pet_command(&self, events: &mut Vec<DomainEvent>) {
        events.push(DomainEvent::SummonCommandChanged {
            resolution: SummonCommandResolutionDto {
                command: self.summon_command.clone(),
                affected_summons: self.pet_upkeep().controlled_pets,
            },
        });
    }

    pub(super) fn set_pet_mode(
        &mut self,
        mode: SummonCommandModeDto,
        events: &mut Vec<DomainEvent>,
    ) {
        self.summon_command.mode = mode;
        self.summon_command.guard_position =
            (mode == SummonCommandModeDto::Guard).then_some(self.player.position);
        if matches!(
            mode,
            SummonCommandModeDto::Follow
                | SummonCommandModeDto::StayClose
                | SummonCommandModeDto::Guard
        ) {
            self.summon_command.target_actor_id = None;
        }
        self.report_pet_command(events);
    }

    pub(super) fn set_pet_target(
        &mut self,
        actor_id: Option<String>,
        events: &mut Vec<DomainEvent>,
    ) {
        if actor_id.as_deref().is_some_and(|id| {
            !self.pet_target_exists(id)
                || !self
                    .entities
                    .iter()
                    .any(|actor| actor.id == id && self.entity_is_visible_to_player(actor))
        }) {
            events.push(DomainEvent::PetCommandUnavailable);
            return;
        }
        self.summon_command.target_actor_id = actor_id;
        self.report_pet_command(events);
    }

    pub(super) fn dismiss_selected_pet(
        &mut self,
        id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) {
        let Some(index) = self.entities.iter().position(|actor| {
            actor.id == id && actor.hp > 0 && self.actor_is_player_aligned(actor)
        }) else {
            events.push(DomainEvent::PetCommandUnavailable);
            return;
        };
        let actor = &self.entities[index];
        if self.entity_is_visible_to_player(actor)
            && !self.entity_is_fuzzy_to_player(actor)
            && let Some(name) = &actor.custom_name
        {
            events.push(DomainEvent::NamedPetDismissed { name: name.clone() });
        }
        self.remove_pet_at(index, changed, removed);
        events.push(DomainEvent::PetsDismissed {
            count: 1,
            upkeep_percent: self.pet_upkeep().percent,
        });
    }

    fn pet_follow_distance(&self) -> i32 {
        match self.summon_command.mode {
            SummonCommandModeDto::StayClose => 1,
            SummonCommandModeDto::Follow => 6,
            SummonCommandModeDto::Attack => 255,
            SummonCommandModeDto::GiveSpace => -10,
            SummonCommandModeDto::KeepDistance => -25,
            SummonCommandModeDto::Guard => 0,
        }
    }

    pub(super) fn pet_hostile_target_ids(&self, index: usize) -> Vec<String> {
        let origin = self.entities[index].position;
        let owner = self.player.position;
        let distance = chebyshev_distance(origin, owner);
        let mode = self.summon_command.mode;
        let follow_distance = self.pet_follow_distance();
        let locked = self
            .summon_command
            .target_actor_id
            .as_deref()
            .filter(|id| self.pet_target_exists(id));
        if let Some(id) = locked {
            vec![id.to_owned()]
        } else {
            self.player_summon_hostile_targets(index)
                .into_iter()
                .filter(|id| {
                    let target = self.entities.iter().find(|actor| actor.id == *id).unwrap();
                    let target_distance = chebyshev_distance(target.position, owner);
                    if mode == SummonCommandModeDto::Guard {
                        adjacent(origin, target.position)
                    } else if follow_distance < 0 {
                        target_distance > follow_distance.unsigned_abs()
                    } else {
                        target_distance <= follow_distance as u32 || target_distance <= distance
                    }
                })
                .collect()
        }
    }

    pub(super) fn resolve_player_summon_action(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let origin = self.entities[index].position;
        let owner = self.player.position;
        let distance = chebyshev_distance(origin, owner);
        let mode = self.summon_command.mode;
        let follow_distance = self.pet_follow_distance();
        let targets = self.pet_hostile_target_ids(index);
        let never_moves = self
            .actor_runtime_definition(&self.entities[index])
            .is_some_and(|definition| definition.movement.never_moves);
        if let Some(id) = targets.first() {
            let position = self
                .entities
                .iter()
                .find(|actor| actor.id == *id)
                .unwrap()
                .position;
            if self.monster_attempts_melee(index) && adjacent(origin, position) {
                self.resolve_player_summon_melee(index, id, events, changed, removed_entities)?;
                return Ok(());
            }
            if !never_moves && let Some(next) = self.next_monster_step_toward(index, position, true)
            {
                self.move_entity(index, next, events, changed, removed_entities)?;
            }
            return Ok(());
        }
        if never_moves {
            return Ok(());
        }
        let next = match mode {
            SummonCommandModeDto::Follow
            | SummonCommandModeDto::StayClose
            | SummonCommandModeDto::Attack => (distance > follow_distance.min(10) as u32)
                .then(|| self.next_monster_step_toward(index, owner, true))
                .flatten(),
            SummonCommandModeDto::KeepDistance | SummonCommandModeDto::GiveSpace => {
                let limit = follow_distance.unsigned_abs();
                if distance <= limit {
                    self.next_player_summon_step_away_from_owner(index)
                } else if distance > limit + 1 {
                    self.next_monster_step_toward(index, owner, true)
                } else {
                    None
                }
            }
            SummonCommandModeDto::Guard => {
                let guard = self.summon_command.guard_position.unwrap_or(owner);
                (!adjacent(origin, guard) && origin != guard)
                    .then(|| self.next_monster_step_toward(index, guard, true))
                    .flatten()
            }
        };
        if let Some(next) = next {
            self.move_entity(index, next, events, changed, removed_entities)?;
        }
        Ok(())
    }
}

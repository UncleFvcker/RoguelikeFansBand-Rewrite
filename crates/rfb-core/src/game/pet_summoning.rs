// SPDX-License-Identifier: MPL-2.0
use super::*;

// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c:
// monster2.c place_monster_one; melee2.c process_monster parent check.
impl Game {
    pub(super) fn initialize_monster_summon(
        &mut self,
        source_index: usize,
        entity: &mut Actor,
        ability_id: &str,
        duration: u16,
    ) {
        let source = &self.entities[source_index];
        let owner_id = source.id.clone();
        let pet = self.actor_is_player_aligned(source);
        let friendly = self.actor_is_friendly(source);
        let stupid = self
            .actor_runtime_definition(source)
            .is_some_and(|kind| kind.tags.iter().any(|tag| tag == "stupid"));
        let unique = self.content.actor(&entity.kind_id).is_some_and(|kind| {
            kind.tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
        });
        if pet {
            if unique {
                // Monster-summoned uniques/Nazgul are friends, not controlled pets.
                entity.friendly = true;
                entity.controller_id = None;
            } else {
                entity.controller_id = Some(self.player.id.clone());
            }
        } else if friendly && !stupid && self.rng.bounded(10) != 0 {
            entity.friendly = true;
        }
        entity.summon = Some(SummonIdentity {
            owner_dependent: pet && !unique,
            owner_id,
            source_ability_id: ability_id.to_owned(),
            remaining_turns: duration,
        });
    }

    pub(super) fn resolve_missing_summon_owner(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed: &mut Vec<String>,
    ) -> bool {
        let Some(summon) = self.entities[index]
            .summon
            .as_ref()
            .filter(|summon| summon.owner_dependent)
        else {
            return false;
        };
        // Floors persist in this implementation: a living source left on another
        // floor is not dead. owner_id remains provenance after releasing the link.
        let alive = self
            .entities
            .iter()
            .chain(
                self.stored_floors
                    .values()
                    .flat_map(|floor| floor.entities.iter()),
            )
            .any(|actor| actor.id == summon.owner_id && actor.hp > 0);
        if alive {
            return false;
        }
        if !self.entity_is_player_aligned(index)
            || self.riding_actor_id.as_deref() == Some(self.entities[index].id.as_str())
        {
            self.entities[index]
                .summon
                .as_mut()
                .unwrap()
                .owner_dependent = false;
            return false;
        }
        let entity_id = self.entities[index].id.clone();
        let target_kind_id = self.entities[index].kind_id.clone();
        self.remove_pet_at(index, changed, removed);
        events.push(DomainEvent::SummonExpired {
            entity_id,
            target_kind_id,
        });
        true
    }

    pub(super) fn summon_owner_chain_is_valid(&self, actor: &Actor) -> bool {
        let mut visited = BTreeSet::new();
        let mut current = actor;
        while let Some(summon) = current
            .summon
            .as_ref()
            .filter(|summon| summon.owner_dependent)
        {
            if !visited.insert(current.id.as_str()) || summon.owner_id == self.player.id {
                return false;
            }
            let Some(owner) = self
                .entities
                .iter()
                .chain(
                    self.stored_floors
                        .values()
                        .flat_map(|floor| floor.entities.iter()),
                )
                .find(|candidate| candidate.id == summon.owner_id)
            else {
                // A source can die before the child's next action. Loading
                // preserves this pending disappearance instead of repairing it.
                return true;
            };
            current = owner;
        }
        true
    }
}

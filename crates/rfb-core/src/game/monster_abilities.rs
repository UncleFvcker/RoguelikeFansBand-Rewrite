// SPDX-License-Identifier: MPL-2.0

use super::ground_item_effects::ground_item_damage_type_for_ability_effect;
use super::monster_ecology::{BANOR_KIND_ID, BANOR_RUPART_COMBINED_KIND_ID, RUPART_KIND_ID};
use super::*;

const BANOR_RUPART_TRANSFORM_TAG: &str = "monster-banor-rupart-transform";
const MONSTER_AIR_BREATH_TAG: &str = "monster-air-breath";
const MONSTER_CHICKEN_TAG: &str = "monster-chicken";
const MONSTER_FAMILY_SUMMON_TAG: &str = "monster-family-summon";
const MONSTER_WATER_FLOW_TAG: &str = "monster-water-flow";
const MONSTER_WATER_FLOW_TERRAIN_ID: &str = "demo.terrain.surface-water-deep";
const MONSTER_WATER_FLOW_RADIUS: u8 = 8;
const MONSTER_LAVA_FLOW_TERRAIN_ID: &str = "demo.terrain.surface-lava-deep";
const MONSTER_FAMILY_SUMMON_DURATION_TURNS: u16 = 10_000;

fn monster_family_summon_candidates(source_kind_id: &str) -> Option<&'static [&'static str]> {
    Some(match source_kind_id {
        "demo.actor.athena-the-goddess-of-wisdom" => &[
            "demo.actor.zeus-king-of-the-olympians",
            "demo.actor.ultimate-magus",
        ],
        "demo.actor.ares-the-god-of-war" => &[
            "demo.actor.zeus-king-of-the-olympians",
            "demo.actor.hera-queen-of-the-gods",
        ],
        "demo.actor.apollo-the-sun-god" => &[
            "demo.actor.artemis-the-moon-goddess",
            "demo.actor.fenghuang",
        ],
        "demo.actor.artemis-the-moon-goddess" => &["demo.actor.apollo-the-sun-god"],
        "demo.actor.hephaestus-the-smith-god" => &[
            "demo.actor.zeus-king-of-the-olympians",
            "demo.actor.hera-queen-of-the-gods",
            "demo.actor.spellwarp-automaton",
        ],
        "demo.actor.hades-ruler-of-the-underworld" => {
            &["demo.actor.greater-balrog", "demo.actor.archlich"]
        }
        "demo.actor.hera-queen-of-the-gods" => &[
            "demo.actor.ares-the-god-of-war",
            "demo.actor.hephaestus-the-smith-god",
            "demo.actor.death-beast",
        ],
        "demo.actor.osiris-the-reborn" => &[
            "demo.actor.horus-the-ancient",
            "demo.actor.isis-the-great-goddess",
        ],
        "demo.actor.lakshmi-the-goddess-of-prosperity" => &["demo.actor.vishnu-the-preserver"],
        "demo.actor.vishnu-the-preserver" => &[
            "demo.actor.rama-the-exiled-prince",
            "demo.actor.krishna-avatar-of-vishnu",
            "demo.actor.lakshmi-the-goddess-of-prosperity",
            "demo.actor.shesha-the-infinite",
        ],
        "demo.actor.shiva-the-destroyer" => &[
            "demo.actor.parvati-the-goddess-of-hidden-power",
            "demo.actor.ganesha-the-elephant-god",
            "demo.actor.karthikeya-the-six-headed-warrior",
            "demo.actor.vasuki-the-serpent-king",
        ],
        "demo.actor.parvati-the-goddess-of-hidden-power" => &[
            "demo.actor.karthikeya-the-six-headed-warrior",
            "demo.actor.ganesha-the-elephant-god",
            "demo.actor.shiva-the-destroyer",
        ],
        _ => return None,
    })
}

fn prepare_curse_damage(
    rolled: i32,
    current_hp: i32,
    damage_is_current_hp_percent: bool,
    nonlethal: bool,
) -> i32 {
    if !damage_is_current_hp_percent {
        return rolled.max(0);
    }
    let current_hp = current_hp.max(0);
    let damage = i64::from(current_hp)
        .saturating_mul(i64::from(rolled.max(0)))
        .saturating_div(100)
        .clamp(0, i64::from(i32::MAX)) as i32;
    if nonlethal {
        damage.min(current_hp.saturating_sub(1))
    } else {
        damage
    }
}

fn rfb_monster_stun_amount(damage: i32) -> i32 {
    let damage = damage.max(1);
    match damage {
        1..=10 => damage,
        11..=100 => 10 + (damage - 10) * 15 / 90,
        101..=500 => 25 + (damage - 100) * 25 / 400,
        _ => 50,
    }
}

impl Game {
    fn resolve_monster_chicken_riders(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        target: &MonsterHostileTarget,
    ) {
        if !target.is_player() || self.player_is_dead() {
            return;
        }
        let sound_resistance = self.effective_player_resistances().level(DamageType::Sound);
        self.record_monster_player_resistance(
            source_entity_id,
            DamageType::Sound,
            sound_resistance,
        );
        if matches!(
            sound_resistance,
            ResistanceLevel::Vulnerable | ResistanceLevel::Normal
        ) {
            let duration = self.roll_damage(1, 20);
            self.apply_player_melee_status(STATUS_STUN, duration, source_kind_id);
        }
        let fear_resistance = self.effective_player_resistances().level(DamageType::Fear);
        self.record_monster_player_resistance(source_entity_id, DamageType::Fear, fear_resistance);
        let fear_duration = self
            .actor_runtime_definition(&self.entities[source_index])
            .map_or(1, super::monster_combat::melee_terrify_duration);
        let duration = resisted_status_duration(
            u32::try_from(fear_duration).unwrap_or(u32::MAX),
            fear_resistance,
        );
        self.apply_player_melee_status(
            STATUS_FEAR,
            i32::try_from(duration).unwrap_or(i32::MAX),
            source_kind_id,
        );
    }

    fn remove_banor_rupart_form(
        &mut self,
        entity_id: &str,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Actor {
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == entity_id)
            .expect("planned Banor/Rupart form must remain available");
        let removed = self.entities.remove(index);
        if self.riding_actor_id.as_deref() == Some(removed.id.as_str()) {
            self.riding_actor_id = None;
        }
        if let Some(pack_id) = removed
            .pack
            .as_ref()
            .and_then(|pack| (pack.role == MonsterPackRoleDto::Leader).then(|| pack.id.clone()))
        {
            for entity in &mut self.entities {
                if entity.pack.as_ref().is_some_and(|pack| pack.id == pack_id) {
                    entity.pack = None;
                }
            }
        }
        let carried_item_ids = self
            .items
            .iter()
            .filter_map(|item| match &item.location {
                ItemLocation::CarriedBy { actor_id } if actor_id == entity_id => {
                    Some(item.id.clone())
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        self.items
            .retain(|item| !carried_item_ids.contains(item.id.as_str()));
        for item_id in carried_item_ids {
            self.item_property_knowledge.remove(&item_id);
        }
        changed.insert(removed.position);
        removed_entities.push(removed.id.clone());
        removed
    }

    fn push_banor_rupart_form(
        &mut self,
        id: &str,
        kind_id: &str,
        position: Position,
        hp: i32,
        max_hp: i32,
    ) -> String {
        let definition = self
            .content
            .actor(kind_id)
            .expect("Banor/Rupart form definition must remain available")
            .clone();
        let mut actor = spawn_actor_from_definition(
            &mut self.rng,
            &definition,
            id,
            position,
            INITIAL_MONSTER_ENERGY_NEED,
            true,
        );
        actor.max_hp = max_hp.max(1);
        actor.hp = hp.clamp(1, actor.max_hp);
        self.entities.push(actor);
        id.to_owned()
    }

    fn resolve_banor_rupart_transform_plan(
        &mut self,
        source_index: usize,
        plan: &MonsterAbilityPlan,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let source = self.entities[source_index].clone();
        let (entity_ids, positions, summoned_kind_ids) = match &plan.target {
            MonsterAbilityTargetPlan::BanorRupartSplit { positions } => {
                let hp = source.hp.saturating_add(1) / 2;
                let max_hp = source.max_hp / 2;
                let ids = [
                    self.summon_entity_id(&plan.ability.id, 0),
                    self.summon_entity_id(&plan.ability.id, 1),
                ];
                self.remove_banor_rupart_form(&source.id, changed, removed_entities);
                let kinds = [BANOR_KIND_ID, RUPART_KIND_ID];
                let entity_ids = kinds
                    .iter()
                    .zip(positions)
                    .zip(ids)
                    .map(|((kind_id, position), id)| {
                        let id = self.push_banor_rupart_form(&id, kind_id, *position, hp, max_hp);
                        changed.insert(*position);
                        id
                    })
                    .collect::<Vec<_>>();
                (
                    entity_ids,
                    positions.clone(),
                    kinds.into_iter().map(str::to_owned).collect(),
                )
            }
            MonsterAbilityTargetPlan::BanorRupartMerge {
                counterpart_entity_id,
                destination,
            } => {
                let counterpart = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == *counterpart_entity_id)
                    .expect("planned Banor/Rupart counterpart must remain available")
                    .clone();
                let hp = source.hp.saturating_add(counterpart.hp);
                let max_hp = source.max_hp.saturating_add(counterpart.max_hp);
                let id = self.summon_entity_id(&plan.ability.id, 0);
                self.remove_banor_rupart_form(&source.id, changed, removed_entities);
                self.remove_banor_rupart_form(&counterpart.id, changed, removed_entities);
                let id = self.push_banor_rupart_form(
                    &id,
                    BANOR_RUPART_COMBINED_KIND_ID,
                    *destination,
                    hp,
                    max_hp,
                );
                changed.insert(*destination);
                (
                    vec![id],
                    vec![*destination],
                    vec![BANOR_RUPART_COMBINED_KIND_ID.to_owned()],
                )
            }
            _ => unreachable!("Banor/Rupart executor requires a transformation plan"),
        };
        MonsterAbilityPlanResolution {
            target_entity_id: source.id.clone(),
            target_kind_id: source.kind_id.clone(),
            affected_positions: positions.clone(),
            summon: Some(AbilitySummonResolutionDto {
                owner_id: source.id.clone(),
                actor_kind_id: BANOR_RUPART_COMBINED_KIND_ID.to_owned(),
                entity_ids,
                positions,
                duration_turns: 0,
                hostile: true,
                group: false,
                summoned_kind_ids,
            }),
            effects: Vec::new(),
            targets: Vec::new(),
            trace: None,
        }
    }

    fn monster_flow_positions(
        &self,
        center: Position,
        terrain_id: &str,
        radius: u8,
    ) -> Vec<Position> {
        let connections = self
            .floor_connections
            .iter()
            .map(|connection| connection.position)
            .collect::<BTreeSet<_>>();
        self.area_damage_cells(center, radius)
            .into_iter()
            .map(|(_, position)| position)
            .filter(|position| !connections.contains(position))
            .filter(|position| {
                let index = self
                    .index(*position)
                    .expect("water-flow footprint positions must remain in bounds");
                self.terrain[index] != terrain_id
                    && self
                        .content
                        .terrain(&self.terrain[index])
                        .is_some_and(|terrain| !terrain.tags.iter().any(|tag| tag == "permanent"))
            })
            .collect()
    }

    fn monster_family_actor_is_available(&self, actor_kind_id: &str) -> bool {
        self.content.actor(actor_kind_id).is_some()
            && self.actor_kind_available_instance_count(actor_kind_id) > 0
    }

    fn resolve_monster_family_summon_plan(
        &mut self,
        source_index: usize,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> MonsterAbilityPlanResolution {
        let source_kind_id = self.entities[source_index].kind_id.clone();
        let owner_id = self.entities[source_index].id.clone();
        let origin = self.entities[source_index].position;
        let one_in = |game: &mut Self, sides: u64| game.rng.bounded(sides) == 0;
        let available =
            |game: &Self, kind_id: &str| game.monster_family_actor_is_available(kind_id);
        let repeated = |kind_ids: &[&'static str], count: usize| {
            (0..count)
                .flat_map(|_| kind_ids.iter().copied())
                .collect::<Vec<_>>()
        };

        let requested_kind_ids = match source_kind_id.as_str() {
            "demo.actor.hades-ruler-of-the-underworld" => {
                let count = usize::try_from(self.roll_damage(1, 2)).unwrap_or(1);
                repeated(&["demo.actor.greater-balrog", "demo.actor.archlich"], count)
            }
            "demo.actor.athena-the-goddess-of-wisdom" => {
                if one_in(self, 3) && available(self, "demo.actor.zeus-king-of-the-olympians") {
                    vec!["demo.actor.zeus-king-of-the-olympians"]
                } else {
                    let count = usize::try_from(self.roll_damage(1, 2)).unwrap_or(1);
                    repeated(&["demo.actor.ultimate-magus"], count)
                }
            }
            "demo.actor.ares-the-god-of-war" => vec![
                "demo.actor.zeus-king-of-the-olympians",
                "demo.actor.hera-queen-of-the-gods",
            ],
            "demo.actor.apollo-the-sun-god" => {
                if one_in(self, 3) && available(self, "demo.actor.artemis-the-moon-goddess") {
                    vec!["demo.actor.artemis-the-moon-goddess"]
                } else {
                    let initial_count = usize::try_from(self.roll_damage(1, 4)).unwrap_or(1);
                    repeated(&["demo.actor.fenghuang"], initial_count)
                }
            }
            "demo.actor.artemis-the-moon-goddess" => vec!["demo.actor.apollo-the-sun-god"],
            "demo.actor.hephaestus-the-smith-god" => {
                if one_in(self, 3) && available(self, "demo.actor.zeus-king-of-the-olympians") {
                    vec!["demo.actor.zeus-king-of-the-olympians"]
                } else if one_in(self, 3) && available(self, "demo.actor.hera-queen-of-the-gods") {
                    vec!["demo.actor.hera-queen-of-the-gods"]
                } else {
                    let initial_count = usize::try_from(self.roll_damage(1, 4)).unwrap_or(1);
                    repeated(&["demo.actor.spellwarp-automaton"], initial_count)
                }
            }
            "demo.actor.hera-queen-of-the-gods" => {
                if one_in(self, 3) && available(self, "demo.actor.ares-the-god-of-war") {
                    vec!["demo.actor.ares-the-god-of-war"]
                } else if one_in(self, 3) && available(self, "demo.actor.hephaestus-the-smith-god")
                {
                    vec!["demo.actor.hephaestus-the-smith-god"]
                } else {
                    let initial_count = usize::try_from(self.roll_damage(1, 4)).unwrap_or(1);
                    repeated(&["demo.actor.death-beast"], initial_count)
                }
            }
            "demo.actor.osiris-the-reborn" => vec![
                "demo.actor.horus-the-ancient",
                "demo.actor.isis-the-great-goddess",
            ],
            "demo.actor.lakshmi-the-goddess-of-prosperity" => {
                vec!["demo.actor.vishnu-the-preserver"]
            }
            "demo.actor.vishnu-the-preserver" => {
                let avatar_available = available(self, "demo.actor.rama-the-exiled-prince")
                    || available(self, "demo.actor.krishna-avatar-of-vishnu");
                if one_in(self, 3) && avatar_available {
                    vec![
                        "demo.actor.rama-the-exiled-prince",
                        "demo.actor.krishna-avatar-of-vishnu",
                    ]
                } else if one_in(self, 2)
                    && available(self, "demo.actor.lakshmi-the-goddess-of-prosperity")
                {
                    vec!["demo.actor.lakshmi-the-goddess-of-prosperity"]
                } else {
                    vec!["demo.actor.shesha-the-infinite"]
                }
            }
            "demo.actor.shiva-the-destroyer" => {
                let family_available = [
                    "demo.actor.parvati-the-goddess-of-hidden-power",
                    "demo.actor.ganesha-the-elephant-god",
                    "demo.actor.karthikeya-the-six-headed-warrior",
                ]
                .into_iter()
                .any(|kind_id| available(self, kind_id));
                if !one_in(self, 3) && family_available {
                    let mut first = "demo.actor.karthikeya-the-six-headed-warrior";
                    let mut second = "demo.actor.ganesha-the-elephant-god";
                    if !available(self, first) {
                        first = "demo.actor.parvati-the-goddess-of-hidden-power";
                    } else if !available(self, second) {
                        second = "demo.actor.parvati-the-goddess-of-hidden-power";
                    } else if available(self, "demo.actor.parvati-the-goddess-of-hidden-power")
                        && !one_in(self, 3)
                    {
                        if one_in(self, 2) {
                            first = "demo.actor.parvati-the-goddess-of-hidden-power";
                        } else {
                            second = "demo.actor.parvati-the-goddess-of-hidden-power";
                        }
                    }
                    vec![first, second]
                } else {
                    // The authoritative source assigns Nandi and immediately
                    // overwrites it with Vasuki; preserve the resulting target.
                    vec!["demo.actor.vasuki-the-serpent-king"]
                }
            }
            "demo.actor.parvati-the-goddess-of-hidden-power" => {
                let mut first = "demo.actor.karthikeya-the-six-headed-warrior";
                let mut second = "demo.actor.ganesha-the-elephant-god";
                if !available(self, first) && one_in(self, 2) {
                    first = "demo.actor.shiva-the-destroyer";
                } else if !available(self, second) && one_in(self, 2) {
                    second = "demo.actor.shiva-the-destroyer";
                } else if available(self, "demo.actor.shiva-the-destroyer") && one_in(self, 4) {
                    if one_in(self, 2) {
                        first = "demo.actor.shiva-the-destroyer";
                    } else {
                        second = "demo.actor.shiva-the-destroyer";
                    }
                }
                vec![first, second]
            }
            _ => unreachable!("validated family summoner must retain a runtime branch"),
        };

        let mut affected_positions = Vec::new();
        if source_kind_id == "demo.actor.hades-ruler-of-the-underworld" {
            let transformed_positions = self.monster_flow_positions(
                origin,
                MONSTER_LAVA_FLOW_TERRAIN_ID,
                MONSTER_WATER_FLOW_RADIUS,
            );
            let source_terrain_ids = transformed_positions
                .iter()
                .filter_map(|position| {
                    self.index(*position)
                        .map(|index| self.terrain[index].clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            for position in &transformed_positions {
                self.replace_terrain_from_source(
                    *position,
                    MONSTER_LAVA_FLOW_TERRAIN_ID,
                    super::terrain::TerrainChangeSource::Monster,
                    events,
                    changed,
                );
            }
            events.push(DomainEvent::AbilityTerrainTransformed {
                ability_id: plan.ability.id.clone(),
                resolution: AbilityTerrainTransformResolutionDto {
                    center: origin,
                    radius: MONSTER_WATER_FLOW_RADIUS,
                    source_terrain_ids,
                    target_terrain_id: MONSTER_LAVA_FLOW_TERRAIN_ID.to_owned(),
                    transformed_positions: transformed_positions.clone(),
                },
            });
            affected_positions = transformed_positions;
        }

        let mut entity_ids = Vec::new();
        let mut summoned_kind_ids = Vec::new();
        let mut positions = Vec::new();
        for kind_id in requested_kind_ids {
            if !self.monster_family_actor_is_available(kind_id) {
                continue;
            }
            let Some(position) = self
                .open_positions_around_for_actor_kind(origin, 2, kind_id)
                .into_iter()
                .next()
            else {
                continue;
            };
            let definition = self
                .content
                .actor(kind_id)
                .expect("validated family summon candidate must remain available")
                .clone();
            let id = self.summon_entity_id(&plan.ability.id, entity_ids.len());
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            self.maybe_initialize_chameleon_form(&mut entity);
            entity.summon = Some(SummonIdentity {
                owner_id: owner_id.clone(),
                source_ability_id: plan.ability.id.clone(),
                remaining_turns: MONSTER_FAMILY_SUMMON_DURATION_TURNS,
            });
            changed.insert(position);
            affected_positions.push(position);
            entity_ids.push(id);
            summoned_kind_ids.push(kind_id.to_owned());
            positions.push(position);
            self.entities.push(entity);
        }
        affected_positions.sort_unstable_by_key(|position| (position.y, position.x));
        affected_positions.dedup();
        MonsterAbilityPlanResolution {
            target_entity_id: owner_id.clone(),
            target_kind_id: source_kind_id,
            affected_positions,
            summon: Some(AbilitySummonResolutionDto {
                owner_id,
                actor_kind_id: "family".to_owned(),
                entity_ids,
                positions,
                duration_turns: MONSTER_FAMILY_SUMMON_DURATION_TURNS,
                hostile: false,
                group: false,
                summoned_kind_ids,
            }),
            effects: Vec::new(),
            targets: Vec::new(),
            trace: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_monster_bird_drop_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::BirdDrop {
            target,
            trace,
            destination,
            escape_destinations,
        } = &plan.target
        else {
            unreachable!("bird drop executor requires a bird drop target plan")
        };
        let source_position = self.entities[source_index].position;
        if self.rng.bounded(3) == 0 {
            let mut affected_positions = vec![source_position];
            if !escape_destinations.is_empty() {
                let choice = usize::try_from(self.rng.bounded(
                    u64::try_from(escape_destinations.len()).expect("candidate count fits"),
                ))
                .expect("bounded draw fits usize");
                let destination = escape_destinations[choice];
                self.entities[source_index].position = destination;
                changed.insert(source_position);
                changed.insert(destination);
                affected_positions.push(destination);
                events.push(DomainEvent::MonsterTeleported {
                    source_kind_id: source_kind_id.to_owned(),
                    resolution: MonsterDisplacementResolutionDto {
                        actor_id: source_entity_id.to_owned(),
                        from: source_position,
                        to: destination,
                    },
                });
            }
            return MonsterAbilityPlanResolution {
                target_entity_id: target.entity_id().to_owned(),
                target_kind_id: target.kind_id().to_owned(),
                affected_positions,
                summon: None,
                effects: Vec::new(),
                targets: vec![MonsterAbilityTargetResolutionDto {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    target_position: target.position(),
                    effects: Vec::new(),
                }],
                trace: Some(trace.clone()),
            };
        }

        let target_levitates = if target.is_player() {
            self.player_levitates()
        } else {
            self.content
                .actor(target.kind_id())
                .is_some_and(|definition| {
                    definition.movement.modes.contains(&ActorMovementMode::Fly)
                })
        };
        let target_from = target.position();
        events.push(DomainEvent::MonsterDraggedTarget {
            source_kind_id: source_kind_id.to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            resolution: MonsterDisplacementResolutionDto {
                actor_id: target.entity_id().to_owned(),
                from: target_from,
                to: *destination,
            },
        });
        match target {
            MonsterHostileTarget::Player { .. } => {
                events.extend(self.relocate_player(*destination, changed));
            }
            MonsterHostileTarget::Summon { entity_id, .. } => {
                if let Some(target_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == *entity_id && entity.hp > 0)
                {
                    self.entities[target_index].position = *destination;
                    changed.insert(target_from);
                    changed.insert(*destination);
                }
            }
        }

        let mut raw_damage = self.roll_damage(4, 8);
        if !target_levitates {
            raw_damage = raw_damage.saturating_add(self.roll_damage(6, 8));
        }
        let effect = self.resolve_monster_damage_to_hostile(
            source_entity_id,
            source_kind_id,
            &plan.ability.id,
            0,
            raw_damage,
            raw_damage,
            DamageType::Physical,
            target,
            events,
        );
        let effects = vec![effect];
        self.remove_defeated_monster_targets(
            std::iter::once(target.entity_id()),
            source_entity_id,
            events,
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: vec![target_from, *destination],
            summon: None,
            effects: effects.clone(),
            targets: vec![MonsterAbilityTargetResolutionDto {
                target_entity_id: target.entity_id().to_owned(),
                target_kind_id: target.kind_id().to_owned(),
                target_position: *destination,
                effects,
            }],
            trace: Some(trace.clone()),
        }
    }

    fn resolve_monster_animate_dead_effect(
        &mut self,
        source_index: usize,
        ability_id: &str,
        effect_index: u8,
        effect: &AbilityEffectDefinition,
        changed: &mut BTreeSet<Position>,
    ) -> (AbilityEffectResolutionDto, Vec<Position>) {
        let AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            radius,
            count,
            failure_chance_percent,
        } = effect
        else {
            unreachable!("monster animate dead executor requires an animate dead effect")
        };
        let origin = self.entities[source_index].position;
        let owner_id = self.entities[source_index].id.clone();
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated animated actor must remain available")
            .clone();
        let corpses = self.animate_dead_candidates(
            origin,
            actor_kind_id,
            corpse_item_kind_id,
            *radius,
            *count,
        );
        let consumed_corpse_item_ids = corpses
            .iter()
            .map(|(item_id, _)| item_id.clone())
            .collect::<Vec<_>>();
        self.items
            .retain(|item| !consumed_corpse_item_ids.contains(&item.id));
        for item_id in &consumed_corpse_item_ids {
            self.item_property_knowledge.remove(item_id);
        }
        let affected_positions = corpses
            .iter()
            .map(|(_, position)| *position)
            .collect::<Vec<_>>();
        changed.extend(affected_positions.iter().copied());
        let mut entity_ids = Vec::with_capacity(corpses.len());
        let mut positions = Vec::with_capacity(corpses.len());
        for (ordinal, (_, position)) in corpses.into_iter().enumerate() {
            if *failure_chance_percent > 0
                && self.rng.bounded(100) < u64::from(*failure_chance_percent)
            {
                continue;
            }
            let id = self.summon_entity_id(ability_id, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.summon = Some(SummonIdentity {
                owner_id: owner_id.clone(),
                source_ability_id: ability_id.to_owned(),
                remaining_turns: 0,
            });
            self.entities.push(entity);
            entity_ids.push(id);
            positions.push(position);
        }
        (
            AbilityEffectResolutionDto::AnimateDead {
                effect_index,
                actor_kind_id: actor_kind_id.clone(),
                consumed_corpse_item_ids,
                entity_ids,
                positions,
            },
            affected_positions,
        )
    }

    fn monster_has_animatable_remains(
        &self,
        source_index: usize,
        effect: &AbilityEffectDefinition,
    ) -> bool {
        let origin = self.entities[source_index].position;
        effect.ordered_effects().iter().any(|effect| {
            let AbilityEffectDefinition::AnimateDead {
                actor_kind_id,
                corpse_item_kind_id,
                radius,
                count,
                ..
            } = effect
            else {
                return false;
            };
            !self
                .animate_dead_candidates(
                    origin,
                    actor_kind_id,
                    corpse_item_kind_id,
                    *radius,
                    *count,
                )
                .is_empty()
        })
    }

    pub(super) fn resolve_monster_fixed_summon_plan(
        &mut self,
        source_index: usize,
        plan: &MonsterAbilityPlan,
        changed: &mut BTreeSet<Position>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Summon { positions } = &plan.target else {
            unreachable!("monster fixed summon executor requires a summon target plan")
        };
        let AbilityEffectDefinition::Summon {
            ref actor_kind_id,
            duration_turns,
            ..
        } = plan.ability.effect
        else {
            unreachable!("monster summon plan must retain a summon effect");
        };
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated summoned actor must remain available")
            .clone();
        let owner_id = self.entities[source_index].id.clone();
        let mut entity_ids = Vec::with_capacity(positions.len());
        for (ordinal, position) in positions.iter().copied().enumerate() {
            let id = self.summon_entity_id(&plan.ability.id, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            self.maybe_initialize_chameleon_form(&mut entity);
            entity.summon = Some(SummonIdentity {
                owner_id: owner_id.clone(),
                source_ability_id: plan.ability.id.clone(),
                remaining_turns: duration_turns,
            });
            changed.insert(position);
            entity_ids.push(id);
            self.entities.push(entity);
        }
        let summon = AbilitySummonResolutionDto {
            owner_id: owner_id.clone(),
            actor_kind_id: actor_kind_id.clone(),
            entity_ids,
            positions: positions.clone(),
            duration_turns,
            hostile: false,
            group: false,
            summoned_kind_ids: Vec::new(),
        };
        MonsterAbilityPlanResolution {
            target_entity_id: owner_id,
            target_kind_id: self.entities[source_index].kind_id.clone(),
            affected_positions: positions.clone(),
            summon: Some(summon),
            effects: Vec::new(),
            targets: Vec::new(),
            trace: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_cone_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Cone {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster cone executor requires a cone target plan")
        };
        let (raw_damage, damage_type, radius) = match &plan.ability.effect {
            AbilityEffectDefinition::ConeDamage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
                radius,
            } => (
                self.roll_damage(*damage_dice, *damage_sides)
                    .saturating_add(i32::from(*damage_bonus))
                    .max(0),
                damage_type,
                radius,
            ),
            AbilityEffectDefinition::BreathDamage {
                hp_percent,
                max_damage,
                damage_type,
                radius,
            } => {
                // Breath scales with the caster's current vigor: no damage
                // dice are rolled, and the elemental cap bounds a healthy caster.
                let caster_hp = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == source_entity_id)
                    .map_or(0, |entity| entity.hp)
                    .max(0);
                let scaled = caster_hp
                    .saturating_mul(i32::from(*hp_percent))
                    .div_euclid(100);
                (scaled.min(i32::from(*max_damage)), damage_type, radius)
            }
            _ => unreachable!("monster cone plan must retain a cone or breath effect"),
        };
        let origin = self
            .entities
            .iter()
            .find(|entity| entity.id == source_entity_id)
            .map_or(trace.origin, |entity| entity.position);
        let direction = direction_toward(origin, target.position())
            .expect("validated monster cone retains a direction");
        let lateral_distances = self
            .cone_damage_cells(origin, &trace.traversed, direction, *radius)
            .into_iter()
            .map(|(_, lateral, position)| (position, lateral))
            .collect::<BTreeMap<_, _>>();
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let air_breath = plan
            .ability
            .tags
            .iter()
            .any(|tag| tag == MONSTER_AIR_BREATH_TAG);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let lateral_distance = lateral_distances
                .get(&affected_target.position())
                .copied()
                .unwrap_or(0);
            let mut prepared = rfb_area_damage(raw_damage, lateral_distance);
            if air_breath && affected_target.is_player() && self.player_levitates() {
                prepared = prepared.saturating_sub(prepared / 4);
            }
            let resolved_damage_type = if air_breath && affected_target.is_player() {
                DamageType::Physical
            } else {
                DamageType::from(*damage_type)
            };
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                prepared,
                resolved_damage_type,
                &affected_target,
                events,
            );
            if air_breath && !self.player_is_dead() {
                if affected_target.is_player() {
                    let sound_resistance =
                        self.effective_player_resistances().level(DamageType::Sound);
                    self.record_monster_player_resistance(
                        source_entity_id,
                        DamageType::Sound,
                        sound_resistance,
                    );
                    if matches!(
                        sound_resistance,
                        ResistanceLevel::Vulnerable | ResistanceLevel::Normal
                    ) {
                        let duration = self.roll_damage(1, 20);
                        self.apply_player_melee_status(STATUS_STUN, duration, source_kind_id);
                    }
                } else if let Some(target_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == affected_target.entity_id() && entity.hp > 0)
                    && matches!(
                        self.entities[target_index]
                            .resistances
                            .level(DamageType::Force),
                        ResistanceLevel::Vulnerable | ResistanceLevel::Normal
                    )
                {
                    self.apply_actor_melee_status(
                        target_index,
                        STATUS_STUN,
                        rfb_monster_stun_amount(prepared),
                        source_kind_id,
                    );
                }
            }
            changed.insert(affected_target.position());
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: affected_target.position(),
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_monster_targets(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            source_entity_id,
            events,
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_beam_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Beam {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster beam executor requires a beam target plan")
        };
        let AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &plan.ability.effect
        else {
            unreachable!("monster beam plan must retain a beam effect");
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        if plan.ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &plan.ability.id,
                affected_positions,
                DamageType::from(*damage_type),
                false,
                events,
                changed,
                removed_entities,
            );
        }
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                raw_damage,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(affected_target.position());
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: affected_target.position(),
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_monster_targets(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            source_entity_id,
            events,
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_area_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Area {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster area executor requires an area target plan")
        };
        let AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            ..
        } = &plan.ability.effect
        else {
            unreachable!("monster area plan must retain an area effect");
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        if plan.ability.affects_ground_items {
            self.resolve_ground_item_projectile_effects(
                &plan.ability.id,
                affected_positions,
                DamageType::from(*damage_type),
                false,
                events,
                changed,
                removed_entities,
            );
        }
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let position = affected_target.position();
            let distance = target
                .position()
                .x
                .abs_diff(position.x)
                .max(target.position().y.abs_diff(position.y));
            let prepared = rfb_area_damage(raw_damage, distance);
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                prepared,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(position);
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: position,
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_monster_targets(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            source_entity_id,
            events,
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_jump_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::JumpDamage {
            affected_positions,
            destinations,
        } = &plan.target
        else {
            unreachable!("monster jump damage executor requires a jump target plan")
        };
        let AbilityEffectDefinition::JumpDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_multiplier_numerator,
            damage_multiplier_denominator,
            damage_type,
            ..
        } = &plan.ability.effect
        else {
            unreachable!("monster jump damage plan must retain a jump damage effect")
        };
        let origin = self.entities[source_index].position;
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .saturating_mul(i32::from(*damage_multiplier_numerator))
            .saturating_div(i32::from(*damage_multiplier_denominator))
            .max(0);
        let target_actors = self
            .monster_hostile_targets(source_index)
            .into_iter()
            .filter(|target| affected_positions.contains(&target.position()))
            .collect::<Vec<_>>();
        let mut targets = Vec::with_capacity(target_actors.len());
        for target in target_actors {
            let position = target.position();
            let distance = origin
                .x
                .abs_diff(position.x)
                .max(origin.y.abs_diff(position.y));
            let prepared = rfb_area_damage(raw_damage, distance);
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                prepared,
                DamageType::from(*damage_type),
                &target,
                events,
            );
            changed.insert(position);
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: target.entity_id().to_owned(),
                target_kind_id: target.kind_id().to_owned(),
                target_position: position,
                effects: vec![effect],
            });
        }
        self.remove_defeated_monster_targets(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            source_entity_id,
            events,
            changed,
            removed_entities,
        );

        let choice = usize::try_from(
            self.rng
                .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
        )
        .expect("bounded draw fits usize");
        let destination = destinations[choice];
        let source_index = self
            .entities
            .iter()
            .position(|entity| entity.id == source_entity_id)
            .expect("jumping monster must remain on the floor");
        self.entities[source_index].position = destination;
        changed.insert(origin);
        changed.insert(destination);
        events.push(DomainEvent::MonsterBlinked {
            source_kind_id: source_kind_id.to_owned(),
            resolution: MonsterDisplacementResolutionDto {
                actor_id: source_entity_id.to_owned(),
                from: origin,
                to: destination,
            },
        });

        MonsterAbilityPlanResolution {
            target_entity_id: source_entity_id.to_owned(),
            target_kind_id: source_kind_id.to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects: Vec::new(),
            targets,
            trace: None,
        }
    }

    pub(super) fn resolve_monster_self_effects(
        &mut self,
        source_index: usize,
        ability: &AbilityDefinition,
        changed: &mut BTreeSet<Position>,
    ) -> (Vec<AbilityEffectResolutionDto>, Vec<Position>) {
        let source_entity_id = self.entities[source_index].id.clone();
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        let mut affected_positions = Vec::new();
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            let resolution = match effect {
                AbilityEffectDefinition::Heal { amount } => {
                    let amount =
                        i32::try_from(*amount).expect("validated healing amount must fit i32");
                    let actor = &mut self.entities[source_index];
                    let outcome = apply_effect(
                        &mut EffectTarget {
                            hp: &mut actor.hp,
                            max_hp: actor.max_hp,
                            resistances: &actor.resistances,
                            statuses: &mut actor.statuses,
                        },
                        EffectSpec::Heal { amount },
                    );
                    let EffectOutcome::Healed { requested, applied } = outcome else {
                        unreachable!("monster healing must produce a healing outcome");
                    };
                    AbilityEffectResolutionDto::Heal {
                        effect_index,
                        resolution: HealingResolutionDto { requested, applied },
                    }
                }
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    stacking,
                    resistance_type,
                    power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id,
                    grants_wall_passage,
                    incoming_damage_percent,
                } => apply_ability_status_effect(
                    &mut self.entities[source_index],
                    &ability.id,
                    effect_index,
                    status_kind_id,
                    *intensity,
                    *duration_ticks,
                    *duration_dice,
                    *duration_sides,
                    *stacking,
                    *resistance_type,
                    *power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id.as_deref(),
                    *grants_wall_passage,
                    *incoming_damage_percent,
                    None,
                    None,
                    &mut self.rng,
                ),
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    remove_ability_status_effect(
                        &mut self.entities[source_index],
                        effect_index,
                        status_kind_id,
                    )
                }
                AbilityEffectDefinition::AggravateMonsters => {
                    let (awakened, hastened, positions) =
                        self.aggravate_monsters(Some(&source_entity_id), &ability.id, changed);
                    affected_positions.extend(positions);
                    AbilityEffectResolutionDto::AggravateMonsters {
                        effect_index,
                        awakened,
                        hastened,
                    }
                }
                AbilityEffectDefinition::NoOp { reason } => AbilityEffectResolutionDto::NoOp {
                    effect_index,
                    reason: reason.clone(),
                },
                effect @ AbilityEffectDefinition::AnimateDead { .. } => {
                    let (resolution, positions) = self.resolve_monster_animate_dead_effect(
                        source_index,
                        &ability.id,
                        effect_index,
                        effect,
                        changed,
                    );
                    affected_positions.extend(positions);
                    resolution
                }
                _ => unreachable!("validated monster self effects must remain actor effects"),
            };
            resolutions.push(resolution);
        }
        (resolutions, affected_positions)
    }

    pub(super) fn resolve_monster_ability_plan(
        &mut self,
        source_index: usize,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let source_entity_id = self.entities[source_index].id.clone();
        let player_hp_before = self.player.hp;
        let mounted_hp_before = self.riding_actor_id.as_deref().and_then(|mount_id| {
            self.entities
                .iter()
                .find(|entity| entity.id == mount_id)
                .map(|entity| (mount_id.to_owned(), entity.hp))
        });
        let resolution = match &plan.target {
            MonsterAbilityTargetPlan::BanorRupartSplit { .. }
            | MonsterAbilityTargetPlan::BanorRupartMerge { .. } => self
                .resolve_banor_rupart_transform_plan(source_index, plan, changed, removed_entities),
            MonsterAbilityTargetPlan::SelfTarget
                if plan
                    .ability
                    .tags
                    .iter()
                    .any(|tag| tag == MONSTER_FAMILY_SUMMON_TAG) =>
            {
                self.resolve_monster_family_summon_plan(source_index, plan, events, changed)
            }
            MonsterAbilityTargetPlan::SelfTarget => {
                let target_entity_id = self.entities[source_index].id.clone();
                let target_kind_id = self.entities[source_index].kind_id.clone();
                let target_position = self.entities[source_index].position;
                let (effects, affected_positions) =
                    self.resolve_monster_self_effects(source_index, &plan.ability, changed);
                if !matches!(
                    plan.ability.effect,
                    AbilityEffectDefinition::AggravateMonsters
                ) {
                    changed.insert(target_position);
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target_entity_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    affected_positions,
                    summon: None,
                    effects: effects.clone(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id,
                        target_kind_id,
                        target_position,
                        effects,
                    }],
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::Projectile { target, trace } => {
                if plan.ability.affects_ground_items
                    && let Some(damage_type) =
                        ground_item_damage_type_for_ability_effect(&plan.ability.effect)
                {
                    self.resolve_ground_item_projectile_effects(
                        &plan.ability.id,
                        &[trace.landing],
                        damage_type,
                        false,
                        events,
                        changed,
                        removed_entities,
                    );
                }
                let floor_id = self.current_floor_id.clone();
                let effects = self.resolve_monster_hostile_effects(
                    &source_entity_id,
                    source_kind_id,
                    &plan.ability,
                    target,
                    events,
                    changed,
                );
                if plan
                    .ability
                    .tags
                    .iter()
                    .any(|tag| tag == MONSTER_CHICKEN_TAG)
                {
                    self.resolve_monster_chicken_riders(
                        source_index,
                        &source_entity_id,
                        source_kind_id,
                        target,
                    );
                }
                let transitioned = self.current_floor_id != floor_id;
                if !transitioned {
                    changed.insert(target.position());
                }
                let targets = vec![MonsterAbilityTargetResolutionDto {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    target_position: target.position(),
                    effects: effects.clone(),
                }];
                if !transitioned {
                    self.remove_defeated_monster_targets(
                        targets
                            .iter()
                            .map(|target| target.target_entity_id.as_str()),
                        &source_entity_id,
                        events,
                        changed,
                        removed_entities,
                    );
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: if transitioned {
                        Vec::new()
                    } else {
                        vec![target.position()]
                    },
                    summon: None,
                    effects,
                    targets,
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::BirdDrop { .. } => self.resolve_monster_bird_drop_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::Area { .. } => self.resolve_monster_area_damage_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::JumpDamage { .. } => self.resolve_monster_jump_damage_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::Beam { .. } => self.resolve_monster_beam_damage_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::Cone { .. } => self.resolve_monster_cone_damage_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::TerrainTransform {
                target,
                trace,
                center,
                positions,
            } => {
                self.resolve_terrain_transform_effect(
                    &plan.ability,
                    *center,
                    positions.clone(),
                    super::terrain::TerrainChangeSource::Monster,
                    events,
                    changed,
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: positions.clone(),
                    summon: None,
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::Summon { .. } => {
                self.resolve_monster_fixed_summon_plan(source_index, plan, changed)
            }
            MonsterAbilityTargetPlan::SummonCategory {
                candidate_kind_ids,
                positions,
            } => {
                let mut candidate_kind_ids = candidate_kind_ids.clone();
                let AbilityEffectDefinition::SummonCategory {
                    ref category,
                    count_dice,
                    count_sides,
                    count_bonus,
                    maximum_count,
                    ref batch_candidates,
                    duration_turns,
                    ..
                } = plan.ability.effect
                else {
                    unreachable!("monster category summon plan must retain its effect");
                };
                // The count dice roll comes first. Batch candidates then use
                // one weighted draw for the whole group; ordinary category
                // summons retain one kind draw per summon.
                let rolled = self
                    .roll_damage(u16::from(count_dice), u16::from(count_sides))
                    .saturating_add(i32::from(count_bonus))
                    .max(1);
                let count = usize::try_from(rolled)
                    .unwrap_or(1)
                    .min(maximum_count.map_or(usize::MAX, usize::from))
                    .min(positions.len());
                let mut affected_positions = Vec::new();
                if plan
                    .ability
                    .tags
                    .iter()
                    .any(|tag| tag == MONSTER_WATER_FLOW_TAG)
                {
                    let center = self.entities[source_index].position;
                    let transformed_positions = self.monster_flow_positions(
                        center,
                        MONSTER_WATER_FLOW_TERRAIN_ID,
                        MONSTER_WATER_FLOW_RADIUS,
                    );
                    let source_terrain_ids = transformed_positions
                        .iter()
                        .filter_map(|position| {
                            self.index(*position)
                                .map(|index| self.terrain[index].clone())
                        })
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>();
                    for position in &transformed_positions {
                        self.replace_terrain_from_source(
                            *position,
                            MONSTER_WATER_FLOW_TERRAIN_ID,
                            super::terrain::TerrainChangeSource::Monster,
                            events,
                            changed,
                        );
                    }
                    events.push(DomainEvent::AbilityTerrainTransformed {
                        ability_id: plan.ability.id.clone(),
                        resolution: AbilityTerrainTransformResolutionDto {
                            center,
                            radius: MONSTER_WATER_FLOW_RADIUS,
                            source_terrain_ids,
                            target_terrain_id: MONSTER_WATER_FLOW_TERRAIN_ID.to_owned(),
                            transformed_positions: transformed_positions.clone(),
                        },
                    });
                    affected_positions = transformed_positions;
                }
                let owner_id = self.entities[source_index].id.clone();
                let mut entity_ids = Vec::with_capacity(count);
                let mut summoned_kind_ids = Vec::with_capacity(count);
                let mut used_positions = Vec::with_capacity(count);
                let planned_positions = positions.iter().copied().take(count).collect::<Vec<_>>();
                let batch_kind_id = if batch_candidates.is_empty() {
                    None
                } else {
                    let eligible = batch_candidates
                        .iter()
                        .filter(|candidate| candidate_kind_ids.contains(&candidate.actor_kind_id))
                        .filter(|candidate| {
                            planned_positions.iter().any(|position| {
                                self.actor_kind_can_enter_position(
                                    &candidate.actor_kind_id,
                                    *position,
                                )
                            })
                        })
                        .map(|candidate| {
                            (candidate.actor_kind_id.clone(), u32::from(candidate.weight))
                        })
                        .collect::<Vec<_>>();
                    let weights = eligible
                        .iter()
                        .map(|(_, weight)| *weight)
                        .collect::<Vec<_>>();
                    Some(eligible[self.roll_weighted_index(&weights)].0.clone())
                };
                for position in planned_positions {
                    if candidate_kind_ids.is_empty() {
                        break;
                    }
                    let kind_id = if let Some(kind_id) = &batch_kind_id {
                        if self.actor_kind_available_instance_count(kind_id) == 0
                            || !self.actor_kind_can_enter_position(kind_id, position)
                        {
                            continue;
                        }
                        kind_id.clone()
                    } else {
                        let eligible_choices = candidate_kind_ids
                            .iter()
                            .enumerate()
                            .filter_map(|(index, kind_id)| {
                                (self.actor_kind_available_instance_count(kind_id) > 0
                                    && self.actor_kind_can_enter_position(kind_id, position))
                                .then_some(index)
                            })
                            .collect::<Vec<_>>();
                        if eligible_choices.is_empty() {
                            continue;
                        }
                        let eligible_choice = usize::try_from(
                            self.rng.bounded(
                                u64::try_from(eligible_choices.len())
                                    .expect("eligible candidate count fits"),
                            ),
                        )
                        .expect("bounded draw fits usize");
                        candidate_kind_ids[eligible_choices[eligible_choice]].clone()
                    };
                    let definition = self
                        .content
                        .actor(&kind_id)
                        .expect("validated summon candidate must remain available")
                        .clone();
                    let id = self.summon_entity_id(&plan.ability.id, entity_ids.len());
                    let mut entity = spawn_actor_from_definition(
                        &mut self.rng,
                        &definition,
                        &id,
                        position,
                        INITIAL_MONSTER_ENERGY_NEED,
                        true,
                    );
                    self.maybe_initialize_chameleon_form(&mut entity);
                    entity.summon = Some(SummonIdentity {
                        owner_id: owner_id.clone(),
                        source_ability_id: plan.ability.id.clone(),
                        remaining_turns: duration_turns,
                    });
                    changed.insert(position);
                    entity_ids.push(id);
                    summoned_kind_ids.push(kind_id.clone());
                    used_positions.push(position);
                    self.entities.push(entity);
                    if self.actor_kind_available_instance_count(&kind_id) == 0 {
                        candidate_kind_ids.retain(|candidate| candidate != &kind_id);
                    }
                }
                let summon = AbilitySummonResolutionDto {
                    owner_id: owner_id.clone(),
                    actor_kind_id: category.clone(),
                    entity_ids,
                    positions: used_positions.clone(),
                    duration_turns,
                    hostile: false,
                    group: false,
                    summoned_kind_ids,
                };
                affected_positions.extend(used_positions.iter().copied());
                affected_positions.sort_unstable_by_key(|position| (position.y, position.x));
                affected_positions.dedup();
                MonsterAbilityPlanResolution {
                    target_entity_id: owner_id,
                    target_kind_id: self.entities[source_index].kind_id.clone(),
                    affected_positions,
                    summon: Some(summon),
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::BlinkSelf { destinations }
            | MonsterAbilityTargetPlan::EscapeSelf { destinations } => {
                // The candidate list was collected without RNG at planning
                // time; the actual landing cell consumes one bounded draw.
                let choice = usize::try_from(
                    self.rng
                        .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
                )
                .expect("bounded draw fits usize");
                let destination = destinations[choice];
                let from = self.entities[source_index].position;
                self.entities[source_index].position = destination;
                changed.insert(from);
                changed.insert(destination);
                let resolution = MonsterDisplacementResolutionDto {
                    actor_id: source_entity_id.clone(),
                    from,
                    to: destination,
                };
                events.push(
                    if matches!(plan.target, MonsterAbilityTargetPlan::BlinkSelf { .. }) {
                        DomainEvent::MonsterBlinked {
                            source_kind_id: source_kind_id.to_owned(),
                            resolution,
                        }
                    } else {
                        DomainEvent::MonsterTeleported {
                            source_kind_id: source_kind_id.to_owned(),
                            resolution,
                        }
                    },
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: source_entity_id.clone(),
                    target_kind_id: self.entities[source_index].kind_id.clone(),
                    affected_positions: vec![from, destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::BlinkTarget {
                target,
                trace,
                destinations,
            }
            | MonsterAbilityTargetPlan::BanishTarget {
                target,
                trace,
                destinations,
            } => {
                let blink_target =
                    matches!(&plan.target, MonsterAbilityTargetPlan::BlinkTarget { .. });
                // One bounded draw picks the landing cell from the candidates
                // collected during planning.
                let choice = usize::try_from(
                    self.rng
                        .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
                )
                .expect("bounded draw fits usize");
                let destination = destinations[choice];
                match target {
                    MonsterHostileTarget::Player { .. } => {
                        let from = self.player.position;
                        let resolution = MonsterDisplacementResolutionDto {
                            actor_id: target.entity_id().to_owned(),
                            from,
                            to: destination,
                        };
                        events.push(if blink_target {
                            DomainEvent::MonsterBlinkedTarget {
                                source_kind_id: source_kind_id.to_owned(),
                                target_kind_id: target.kind_id().to_owned(),
                                resolution,
                            }
                        } else {
                            DomainEvent::MonsterBanishedTarget {
                                source_kind_id: source_kind_id.to_owned(),
                                target_kind_id: target.kind_id().to_owned(),
                                resolution,
                            }
                        });
                        let relocation = self.relocate_player(destination, changed);
                        events.extend(relocation);
                    }
                    MonsterHostileTarget::Summon { entity_id, .. } => {
                        if let Some(banished_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *entity_id && entity.hp > 0)
                        {
                            let from = self.entities[banished_index].position;
                            self.entities[banished_index].position = destination;
                            changed.insert(from);
                            changed.insert(destination);
                            let resolution = MonsterDisplacementResolutionDto {
                                actor_id: entity_id.clone(),
                                from,
                                to: destination,
                            };
                            events.push(if blink_target {
                                DomainEvent::MonsterBlinkedTarget {
                                    source_kind_id: source_kind_id.to_owned(),
                                    target_kind_id: target.kind_id().to_owned(),
                                    resolution,
                                }
                            } else {
                                DomainEvent::MonsterBanishedTarget {
                                    source_kind_id: source_kind_id.to_owned(),
                                    target_kind_id: target.kind_id().to_owned(),
                                    resolution,
                                }
                            });
                        }
                    }
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id: target.entity_id().to_owned(),
                        target_kind_id: target.kind_id().to_owned(),
                        target_position: destination,
                        effects: Vec::new(),
                    }],
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::DragTarget {
                target,
                trace,
                destination,
            } => {
                let destination = *destination;
                match target {
                    MonsterHostileTarget::Player { .. } => {
                        let from = self.player.position;
                        events.push(DomainEvent::MonsterDraggedTarget {
                            source_kind_id: source_kind_id.to_owned(),
                            target_kind_id: target.kind_id().to_owned(),
                            resolution: MonsterDisplacementResolutionDto {
                                actor_id: target.entity_id().to_owned(),
                                from,
                                to: destination,
                            },
                        });
                        let relocation = self.relocate_player(destination, changed);
                        events.extend(relocation);
                    }
                    MonsterHostileTarget::Summon { entity_id, .. } => {
                        if let Some(dragged_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *entity_id && entity.hp > 0)
                        {
                            let from = self.entities[dragged_index].position;
                            self.entities[dragged_index].position = destination;
                            changed.insert(from);
                            changed.insert(destination);
                            events.push(DomainEvent::MonsterDraggedTarget {
                                source_kind_id: source_kind_id.to_owned(),
                                target_kind_id: target.kind_id().to_owned(),
                                resolution: MonsterDisplacementResolutionDto {
                                    actor_id: entity_id.clone(),
                                    from,
                                    to: destination,
                                },
                            });
                        }
                    }
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id: target.entity_id().to_owned(),
                        target_kind_id: target.kind_id().to_owned(),
                        target_position: destination,
                        effects: Vec::new(),
                    }],
                    trace: Some(trace.clone()),
                }
            }
        };
        let player_damage = player_hp_before
            .saturating_sub(self.player.hp)
            .clamp(0, 200);
        let mounted_damage = mounted_hp_before
            .and_then(|(mount_id, hp_before)| {
                self.entities
                    .iter()
                    .find(|entity| entity.id == mount_id)
                    .map(|entity| hp_before.saturating_sub(entity.hp).clamp(0, 200))
            })
            .unwrap_or(0);
        if mounted_damage > 0 && !self.player_is_dead() {
            self.resolve_riding_fall(mounted_damage, false, events, changed);
        }
        if player_damage > 0 && !self.player_is_dead() {
            self.resolve_riding_fall(player_damage, false, events, changed);
        }
        resolution
    }

    fn monster_targets_in_footprint(
        &self,
        source_index: usize,
        primary: &MonsterHostileTarget,
        affected_positions: &[Position],
    ) -> Vec<MonsterHostileTarget> {
        let mut targets = self
            .monster_hostile_targets(source_index)
            .into_iter()
            .filter(|target| affected_positions.contains(&target.position()))
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| {
            (left.entity_id() != primary.entity_id())
                .cmp(&(right.entity_id() != primary.entity_id()))
                .then_with(|| left.entity_id().cmp(right.entity_id()))
        });
        targets
    }

    fn remove_defeated_monster_targets<'a>(
        &mut self,
        target_entity_ids: impl Iterator<Item = &'a str>,
        source_entity_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut defeated = target_entity_ids
            .filter_map(|entity_id| {
                self.entities
                    .iter()
                    .position(|entity| entity.id == entity_id && entity.hp <= 0)
                    .map(|index| self.entities[index].id.clone())
            })
            .collect::<Vec<_>>();
        defeated.sort();
        defeated.dedup();
        for entity_id in defeated {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let slain = self.entities[index].clone();
            self.reward_controlled_actor_kill(source_entity_id, &slain, events);
            self.resolve_actor_death_without_rewards(
                index,
                None,
                events,
                changed,
                removed_entities,
            )
            .expect("defeated monster target death must resolve");
        }
    }

    fn resolve_monster_hostile_effects(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability: &AbilityDefinition,
        target: &MonsterHostileTarget,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Vec<AbilityEffectResolutionDto> {
        if target.is_player() {
            return self.resolve_monster_player_effects(
                source_entity_id,
                source_kind_id,
                ability,
                events,
                changed,
            );
        }
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            let Some(target_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target.entity_id())
            else {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            };
            if self.entities[target_index].hp <= 0 {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            }
            let resolution = match effect {
                AbilityEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                } => {
                    let raw_damage = self
                        .roll_damage(*damage_dice, *damage_sides)
                        .saturating_add(i32::from(*damage_bonus))
                        .max(0);
                    let damage_type = DamageType::from(*damage_type);
                    let definition = self
                        .content
                        .actor(&self.entities[target_index].kind_id)
                        .expect("monster target definition must remain available");
                    let armor_class = self
                        .actor_derived_stats(&self.entities[target_index], definition, false)
                        .armor_class
                        .value;
                    let prepared = if damage_type == DamageType::Physical
                        && !ability.tags.iter().any(|tag| tag == MONSTER_CHICKEN_TAG)
                    {
                        apply_melee_armor_reduction(raw_damage, armor_class)
                    } else {
                        raw_damage
                    };
                    self.resolve_monster_damage_to_hostile(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        prepared,
                        damage_type,
                        target,
                        events,
                    )
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_is_current_hp_percent,
                    nonlethal,
                } => {
                    // Summoned targets have no saving-throw skill; the curse
                    // lands in full (documented v98 simplification).
                    let target_definition = self
                        .content
                        .actor(&self.entities[target_index].kind_id)
                        .expect("monster target definition must remain available");
                    if *damage_is_current_hp_percent
                        && target_definition
                            .tags
                            .iter()
                            .any(|tag| matches!(tag.as_str(), "unique" | "unique2" | "guardian"))
                    {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Ineligible,
                        }
                    } else {
                        let rolled = self
                            .roll_damage(*damage_dice, *damage_sides)
                            .saturating_add(i32::from(*damage_bonus))
                            .max(0);
                        let raw_damage = prepare_curse_damage(
                            rolled,
                            self.entities[target_index].hp,
                            *damage_is_current_hp_percent,
                            *nonlethal,
                        );
                        self.resolve_monster_damage_to_hostile(
                            source_entity_id,
                            source_kind_id,
                            &ability.id,
                            effect_index,
                            raw_damage,
                            raw_damage,
                            DamageType::Curse,
                            target,
                            events,
                        )
                    }
                }
                AbilityEffectDefinition::PolymorphTarget => {
                    let caster_level = self
                        .content
                        .actor(source_kind_id)
                        .map_or(1, |definition| definition.level);
                    self.resolve_actor_polymorph_target(
                        target_index,
                        caster_level,
                        effect_index,
                        events,
                        changed,
                    )
                }
                AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::Amnesia
                | AbilityEffectDefinition::DarkenRoom => {
                    // Summons carry no resource pools or map knowledge; both
                    // effects fizzle against them (documented v99 boundary).
                    AbilityEffectResolutionDto::Skipped {
                        effect_index,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }
                }
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    stacking,
                    resistance_type,
                    power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id,
                    grants_wall_passage,
                    incoming_damage_percent,
                } => {
                    let target_level = self
                        .content
                        .actor(&self.entities[target_index].kind_id)
                        .map(|definition| definition.level);
                    apply_ability_status_effect(
                        &mut self.entities[target_index],
                        &ability.id,
                        effect_index,
                        status_kind_id,
                        *intensity,
                        *duration_ticks,
                        *duration_dice,
                        *duration_sides,
                        *stacking,
                        *resistance_type,
                        *power,
                        granted_resistances,
                        granted_brands,
                        granted_modifiers,
                        granted_equipment_bonuses,
                        granted_status_immunities,
                        granted_race_id.as_deref(),
                        *grants_wall_passage,
                        *incoming_damage_percent,
                        target_level,
                        None,
                        &mut self.rng,
                    )
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    remove_ability_status_effect(
                        &mut self.entities[target_index],
                        effect_index,
                        status_kind_id,
                    )
                }
                _ => unreachable!(
                    "validated monster abilities contain only direct actor-target effects"
                ),
            };
            resolutions.push(resolution);
        }
        resolutions
    }

    pub(super) fn monster_curse_save(
        &mut self,
        source_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let caster_level = self
            .content
            .actor(source_kind_id)
            .map_or(1, |definition| definition.level);
        self.monster_saving_throw(source_kind_id, caster_level, events)
    }

    pub(super) fn monster_saving_throw(
        &mut self,
        source_kind_id: &str,
        difficulty: u32,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let ability_stat = self.player_derived_stats().saving_throw_skill;
        let mut difficulty_pipeline = DerivedStatsPipeline::new();
        difficulty_pipeline.add(
            StatKind::ActionDifficulty,
            StatLayer::Environment,
            source_kind_id,
            i32::try_from(difficulty).unwrap_or(i32::MAX),
        );
        let check = resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::SavingThrow,
                actor_id: self.player.id.clone(),
                target_id: Some(source_kind_id.to_owned()),
                ability: ability_stat,
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
            source_kind_id: source_kind_id.to_owned(),
            position: self.player.position,
            succeeded,
            resolution: check.to_dto(skill_id),
        });
        succeeded
    }

    pub(super) fn resolve_monster_player_effects(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Vec<AbilityEffectResolutionDto> {
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            if self.player_is_dead() {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            }
            let resolution = match effect {
                AbilityEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                } => {
                    let raw_damage = self
                        .roll_damage(*damage_dice, *damage_sides)
                        .saturating_add(i32::from(*damage_bonus))
                        .max(0);
                    let damage_type = DamageType::from(*damage_type);
                    let target = self.player_derived_stats();
                    let prepared = if damage_type == DamageType::Physical
                        && !ability.tags.iter().any(|tag| tag == MONSTER_CHICKEN_TAG)
                    {
                        apply_melee_armor_reduction(raw_damage, target.armor_class.value)
                    } else {
                        raw_damage
                    };
                    self.resolve_monster_damage_to_player(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        prepared,
                        damage_type,
                        events,
                    )
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_is_current_hp_percent,
                    nonlethal,
                } => {
                    // A successful saving throw negates the curse before any
                    // damage dice are drawn; difficulty follows the caster's
                    // definition level.
                    if self.monster_curse_save(source_kind_id, events) {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        let rolled = self
                            .roll_damage(*damage_dice, *damage_sides)
                            .saturating_add(i32::from(*damage_bonus))
                            .max(0);
                        let raw_damage = prepare_curse_damage(
                            rolled,
                            self.player.hp,
                            *damage_is_current_hp_percent,
                            *nonlethal,
                        );
                        self.resolve_monster_damage_to_player(
                            source_entity_id,
                            source_kind_id,
                            &ability.id,
                            effect_index,
                            raw_damage,
                            raw_damage,
                            DamageType::Curse,
                            events,
                        )
                    }
                }
                AbilityEffectDefinition::PolymorphTarget => {
                    if self.monster_curse_save(source_kind_id, events) {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        let target_entity_id = self.player.id.clone();
                        let changed = self.resolve_polymorph_mutations(events);
                        AbilityEffectResolutionDto::PolymorphTarget {
                            effect_index,
                            target_entity_id,
                            form_kind_id: None,
                            changed,
                        }
                    }
                }
                AbilityEffectDefinition::TeleportLevel => {
                    let nexus = self.effective_player_resistances().level(DamageType::Nexus);
                    self.record_monster_player_resistance(
                        source_entity_id,
                        DamageType::Nexus,
                        nexus,
                    );
                    let nexus_resisted = self.rng.bounded(55)
                        < u64::try_from(nexus.reduction_percent().max(0)).unwrap_or(0);
                    if nexus_resisted || self.monster_curse_save(source_kind_id, events) {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        let (upward_targets, downward_targets) = self.teleport_level_targets();
                        let prefer_upward = self.rng.bounded(2) == 0;
                        let targets = if prefer_upward {
                            if upward_targets.is_empty() {
                                downward_targets
                            } else {
                                upward_targets
                            }
                        } else if downward_targets.is_empty() {
                            upward_targets
                        } else {
                            downward_targets
                        };
                        let target_index = if targets.len() == 1 {
                            0
                        } else {
                            usize::try_from(self.rng.bounded(targets.len() as u64))
                                .expect("bounded floor target index must fit usize")
                        };
                        let target = targets[target_index].clone();
                        let from_floor_id = self.current_floor_id.clone();
                        let transition = self
                            .transition_floor(
                                target.floor_id,
                                target.arrival_connection_id,
                                target.departure_connection_id,
                                false,
                            )
                            .expect("planned monster level teleport must remain valid")
                            .expect("planned monster level teleport must remain available");
                        let to_floor_id = transition.to_floor_id.clone();
                        self.record_floor_transition(transition, events, changed);
                        AbilityEffectResolutionDto::TeleportLevel {
                            effect_index,
                            from_floor_id,
                            to_floor_id,
                        }
                    }
                }
                AbilityEffectDefinition::DrainResource { amount } => {
                    // The casting-profile pool is drained when present; other
                    // players lose their first non-empty pool in id order.
                    let pool_id = self
                        .casting_profile()
                        .map(|profile| profile.resource_id.clone())
                        .filter(|id| self.resources.contains_key(id))
                        .or_else(|| {
                            self.resources
                                .iter()
                                .find(|(_, pool)| pool.current > 0)
                                .map(|(id, _)| id.clone())
                        });
                    let requested = *amount;
                    let immune = self.player_has_resource_drain_immunity();
                    let (resource_id, drained) = match pool_id {
                        Some(id) if immune => (id, 0),
                        Some(id) => {
                            let pool = self
                                .resources
                                .get_mut(&id)
                                .expect("selected drain pool must remain available");
                            let drained = pool.current.min(requested);
                            pool.current -= drained;
                            (id, drained)
                        }
                        None => (String::new(), 0),
                    };
                    // The caster feeds on the stolen power, capped at its
                    // own maximum life.
                    let mut caster_healed = 0_u32;
                    if drained > 0
                        && let Some(caster_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *source_entity_id)
                    {
                        let caster = &mut self.entities[caster_index];
                        let missing = caster.max_hp.saturating_sub(caster.hp).max(0);
                        let healed = i32::try_from(drained).unwrap_or(i32::MAX).min(missing);
                        caster.hp += healed;
                        caster_healed = u32::try_from(healed).unwrap_or(0);
                        changed.insert(caster.position);
                    }
                    AbilityEffectResolutionDto::DrainResource {
                        effect_index,
                        resource_id,
                        requested,
                        drained,
                        caster_healed,
                    }
                }
                AbilityEffectDefinition::Amnesia => {
                    // The saving throw gates the memory wipe exactly like the
                    // curse family; success costs no further RNG.
                    if self.monster_curse_save(source_kind_id, events) {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        // Only the current floor map memory fades; item
                        // knowledge stays authoritative per the long-term
                        // design constraints.
                        let cleared_cells = self.clear_current_floor_memory(changed);
                        AbilityEffectResolutionDto::Amnesia {
                            effect_index,
                            cleared_cells,
                        }
                    }
                }
                AbilityEffectDefinition::DarkenRoom => {
                    let positions = self.darken_room(self.player.position);
                    changed.extend(positions.iter().copied());
                    AbilityEffectResolutionDto::DarkenRoom {
                        effect_index,
                        cleared_cells: u32::try_from(positions.len()).unwrap_or(u32::MAX),
                    }
                }
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    stacking,
                    resistance_type,
                    power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id,
                    grants_wall_passage,
                    incoming_damage_percent,
                } => {
                    if status_kind_id == STATUS_NO_AIR
                        && (self.player_is_nonliving()
                            || self.player_has_status_kind(STATUS_NO_AIR))
                    {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Ineligible,
                        }
                    } else {
                        let effective = self.effective_player_resistances();
                        let immunities = self.player_status_immunities();
                        let target_level = u32::from(self.progress.level);
                        let resolution = apply_ability_status_effect(
                            &mut self.player,
                            &ability.id,
                            effect_index,
                            status_kind_id,
                            *intensity,
                            *duration_ticks,
                            *duration_dice,
                            *duration_sides,
                            *stacking,
                            *resistance_type,
                            *power,
                            granted_resistances,
                            granted_brands,
                            granted_modifiers,
                            granted_equipment_bonuses,
                            granted_status_immunities,
                            granted_race_id.as_deref(),
                            *grants_wall_passage,
                            *incoming_damage_percent,
                            Some(target_level),
                            Some((&effective, &immunities)),
                            &mut self.rng,
                        );
                        if let Some(damage_type) = resistance_type.map(DamageType::from) {
                            let level = effective.level(damage_type);
                            self.record_monster_player_resistance(
                                source_entity_id,
                                damage_type,
                                level,
                            );
                        }
                        resolution
                    }
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    if self.player_resists_dispel() {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        remove_ability_status_effect(&mut self.player, effect_index, status_kind_id)
                    }
                }
                _ => unreachable!(
                    "validated monster abilities contain only direct actor-target effects"
                ),
            };
            resolutions.push(resolution);
        }
        resolutions
    }
    pub(super) fn monster_ability_target_plan(
        &self,
        index: usize,
        ability: AbilityDefinition,
        base_weight: u32,
    ) -> Result<MonsterAbilityPlan, MonsterAbilityPlanRejection> {
        let origin = self.entities[index].position;
        let (target, enemy_target_count, friendly_risk_count) = match &ability.effect {
            AbilityEffectDefinition::Heal { .. } | AbilityEffectDefinition::AggravateMonsters => {
                (MonsterAbilityTargetPlan::SelfTarget, 0, 0)
            }
            AbilityEffectDefinition::NoOp { .. }
                if ability
                    .tags
                    .iter()
                    .any(|tag| tag == BANOR_RUPART_TRANSFORM_TAG) =>
            {
                if self.banor_rupart_group_is_defeated() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let source_kind_id = self.entities[index].kind_id.as_str();
                let target =
                    match source_kind_id {
                        BANOR_RUPART_COMBINED_KIND_ID => {
                            if self.banor_rupart_living_count() != 1 {
                                return Err(MonsterAbilityPlanRejection {
                                    reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                                    enemy_target_count: 0,
                                    friendly_risk_count: 0,
                                });
                            }
                            let Some(adjacent) = self
                                .open_positions_around_for_actor_kind(origin, 1, RUPART_KIND_ID)
                                .into_iter()
                                .next()
                            else {
                                return Err(MonsterAbilityPlanRejection {
                                    reason: MonsterAbilityRejectionReasonDto::NoSpace,
                                    enemy_target_count: 0,
                                    friendly_risk_count: 0,
                                });
                            };
                            MonsterAbilityTargetPlan::BanorRupartSplit {
                                positions: vec![origin, adjacent],
                            }
                        }
                        BANOR_KIND_ID | RUPART_KIND_ID => {
                            let counterpart_kind_id = if source_kind_id == BANOR_KIND_ID {
                                RUPART_KIND_ID
                            } else {
                                BANOR_KIND_ID
                            };
                            let Some(counterpart) = self.entities.iter().find(|entity| {
                                entity.hp > 0 && entity.kind_id == counterpart_kind_id
                            }) else {
                                return Err(MonsterAbilityPlanRejection {
                                    reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                                    enemy_target_count: 0,
                                    friendly_risk_count: 0,
                                });
                            };
                            if self.banor_rupart_living_count() != 2 {
                                return Err(MonsterAbilityPlanRejection {
                                    reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                                    enemy_target_count: 0,
                                    friendly_risk_count: 0,
                                });
                            }
                            MonsterAbilityTargetPlan::BanorRupartMerge {
                                counterpart_entity_id: counterpart.id.clone(),
                                destination: counterpart.position,
                            }
                        }
                        _ => {
                            return Err(MonsterAbilityPlanRejection {
                                reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                                enemy_target_count: 0,
                                friendly_risk_count: 0,
                            });
                        }
                    };
                (target, 0, 0)
            }
            AbilityEffectDefinition::NoOp { .. }
                if ability.tags.iter().any(|tag| tag == "monster-world") =>
            {
                (MonsterAbilityTargetPlan::SelfTarget, 0, 0)
            }
            AbilityEffectDefinition::NoOp { .. }
                if ability
                    .tags
                    .iter()
                    .any(|tag| tag == MONSTER_FAMILY_SUMMON_TAG) =>
            {
                let candidate_kind_ids = monster_family_summon_candidates(
                    self.entities[index].kind_id.as_str(),
                )
                .ok_or(MonsterAbilityPlanRejection {
                    reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                    enemy_target_count: 0,
                    friendly_risk_count: 0,
                })?;
                let available = candidate_kind_ids
                    .iter()
                    .copied()
                    .filter(|kind_id| self.monster_family_actor_is_available(kind_id));
                if available.clone().next().is_none() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                if !available.into_iter().any(|kind_id| {
                    !self
                        .open_positions_around_for_actor_kind(origin, 2, kind_id)
                        .is_empty()
                }) {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (MonsterAbilityTargetPlan::SelfTarget, 0, 0)
            }
            AbilityEffectDefinition::Summon {
                actor_kind_id,
                count,
                radius,
                ..
            } => {
                let available_count = self
                    .actor_kind_available_instance_count(actor_kind_id)
                    .min(usize::from(*count));
                if available_count == 0 {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let positions = self
                    .summon_positions_around(
                        origin,
                        u8::try_from(available_count)
                            .expect("summon count is bounded by its u8 content field"),
                        *radius,
                        actor_kind_id,
                    )
                    .ok_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })?;
                (MonsterAbilityTargetPlan::Summon { positions }, 0, 0)
            }
            AbilityEffectDefinition::SummonCategory {
                category,
                maximum_level,
                count_dice,
                count_sides,
                count_bonus,
                maximum_count,
                batch_candidates,
                radius,
                ..
            } => {
                // Candidate kinds are filtered without RNG. Ordinary
                // categories enumerate in stable id order; batch candidates
                // preserve their declared weighted order for execution.
                let current_task_id = self.current_floor_task_id();
                let eligible = |definition: &rfb_content::ActorDefinition| {
                    let unique = definition
                        .tags
                        .iter()
                        .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
                    definition.role == ActorRole::Monster
                        && (category != "unique"
                            || definition.level >= u32::from(maximum_level.saturating_sub(40)))
                        && definition.level <= u32::from(*maximum_level)
                        && definition.tags.iter().any(|tag| tag == category)
                        && !definition.tags.iter().any(|tag| tag == "guardian")
                        && actor_answers_summons(definition)
                        && definition.allocation.as_ref().is_none_or(|allocation| {
                            monster_ecology::actor_allocation_matches_task(
                                allocation,
                                current_task_id,
                            )
                        })
                        && (!unique || self.unique_actor_kind_is_available(&definition.id))
                };
                let candidate_kind_ids = if batch_candidates.is_empty() {
                    self.content
                        .actor_definitions()
                        .filter(|definition| eligible(definition))
                        .map(|definition| definition.id.clone())
                        .collect::<Vec<_>>()
                } else {
                    batch_candidates
                        .iter()
                        .filter_map(|candidate| self.content.actor(&candidate.actor_kind_id))
                        .filter(|definition| eligible(definition))
                        .map(|definition| definition.id.clone())
                        .collect::<Vec<_>>()
                };
                if candidate_kind_ids.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let maximum_count = (usize::from(*count_dice) * usize::from(*count_sides)
                    + usize::from(*count_bonus))
                .min(maximum_count.map_or(usize::MAX, usize::from));
                let positions = self
                    .open_positions_around_matching(origin, *radius, |position| {
                        if !ability.tags.iter().any(|tag| tag == MONSTER_WATER_FLOW_TAG) {
                            return candidate_kind_ids.iter().any(|kind_id| {
                                self.actor_kind_can_enter_position(kind_id, position)
                            });
                        }
                        let will_be_water = self
                            .index(position)
                            .and_then(|terrain_index| {
                                self.content.terrain(&self.terrain[terrain_index])
                            })
                            .is_some_and(|terrain| {
                                !self
                                    .floor_connections
                                    .iter()
                                    .any(|connection| connection.position == position)
                                    && (terrain.id == MONSTER_WATER_FLOW_TERRAIN_ID
                                        || !terrain.tags.iter().any(|tag| tag == "permanent"))
                            });
                        will_be_water
                            && self
                                .content
                                .terrain(MONSTER_WATER_FLOW_TERRAIN_ID)
                                .is_some_and(|water| {
                                    candidate_kind_ids.iter().any(|kind_id| {
                                        self.content.actor(kind_id).is_some_and(|actor| {
                                            super::movement::actor_can_cross_terrain(actor, water)
                                        })
                                    })
                                })
                    })
                    .into_iter()
                    .take(maximum_count)
                    .collect::<Vec<_>>();
                if positions.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (
                    MonsterAbilityTargetPlan::SummonCategory {
                        candidate_kind_ids,
                        positions,
                    },
                    0,
                    0,
                )
            }
            effect @ AbilityEffectDefinition::AnimateDead { .. }
                if self.monster_has_animatable_remains(index, effect) =>
            {
                (MonsterAbilityTargetPlan::SelfTarget, 0, 0)
            }
            effect @ AbilityEffectDefinition::Sequence { effects }
                if effects.iter().any(|effect| {
                    matches!(effect, AbilityEffectDefinition::AnimateDead { .. })
                }) =>
            {
                if !self.monster_has_animatable_remains(index, effect) {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (MonsterAbilityTargetPlan::SelfTarget, 0, 0)
            }
            AbilityEffectDefinition::AnimateDead { .. } => {
                return Err(MonsterAbilityPlanRejection {
                    reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                    enemy_target_count: 0,
                    friendly_risk_count: 0,
                });
            }
            AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Sequence { .. }
                if ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget) =>
            {
                (MonsterAbilityTargetPlan::SelfTarget, 0, 0)
            }
            AbilityEffectDefinition::BlinkSelf { radius } => {
                let radius = u32::from(*radius);
                let destinations = self.displacement_destinations(index, |position| {
                    origin
                        .x
                        .abs_diff(position.x)
                        .max(origin.y.abs_diff(position.y))
                        <= radius
                });
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (MonsterAbilityTargetPlan::BlinkSelf { destinations }, 0, 0)
            }
            AbilityEffectDefinition::JumpDamage {
                radius,
                blink_radius,
                ..
            } => {
                let affected_positions = self
                    .area_damage_cells(origin, *radius)
                    .into_iter()
                    .map(|(_, position)| position)
                    .collect::<Vec<_>>();
                let enemy_target_count = u16::try_from(
                    self.monster_hostile_targets(index)
                        .into_iter()
                        .filter(|target| affected_positions.contains(&target.position()))
                        .count(),
                )
                .unwrap_or(u16::MAX);
                if enemy_target_count == 0 {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let blink_radius = u32::from(*blink_radius);
                let destinations = self.displacement_destinations(index, |position| {
                    origin
                        .x
                        .abs_diff(position.x)
                        .max(origin.y.abs_diff(position.y))
                        <= blink_radius
                });
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count,
                        friendly_risk_count: 0,
                    });
                }
                (
                    MonsterAbilityTargetPlan::JumpDamage {
                        affected_positions,
                        destinations,
                    },
                    enemy_target_count,
                    0,
                )
            }
            AbilityEffectDefinition::TeleportSelf { minimum_distance } => {
                let player = self.player.position;
                let escape_candidates = |minimum: u32| {
                    self.displacement_destinations(index, |position| {
                        player
                            .x
                            .abs_diff(position.x)
                            .max(player.y.abs_diff(position.y))
                            >= minimum
                    })
                };
                let minimum = u32::from(*minimum_distance);
                let mut destinations = escape_candidates(minimum);
                if destinations.is_empty() {
                    // The half-distance fallback keeps cramped floors escapable.
                    destinations = escape_candidates(minimum.div_ceil(2));
                }
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (MonsterAbilityTargetPlan::EscapeSelf { destinations }, 0, 0)
            }
            AbilityEffectDefinition::Damage { .. }
            | AbilityEffectDefinition::AreaDamage { .. }
            | AbilityEffectDefinition::BeamDamage { .. }
            | AbilityEffectDefinition::ConeDamage { .. }
            | AbilityEffectDefinition::BreathDamage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::TeleportAway { .. }
            | AbilityEffectDefinition::BirdDrop
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::TeleportLevel
            | AbilityEffectDefinition::PolymorphTarget
            | AbilityEffectDefinition::DarkenRoom
            | AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Sequence { .. }
            | AbilityEffectDefinition::BlinkTarget { .. }
            | AbilityEffectDefinition::TeleportTarget
            | AbilityEffectDefinition::TransformTerrain { .. } => {
                let mut first_rejection = None;
                let mut selected = None;
                for hostile_target in self.monster_hostile_targets(index) {
                    match self.monster_targeted_ability_plan(index, &ability, hostile_target) {
                        Ok(plan) => {
                            selected = Some(plan);
                            break;
                        }
                        Err(rejection) => {
                            first_rejection.get_or_insert(rejection);
                        }
                    }
                }
                selected.ok_or_else(|| {
                    first_rejection.unwrap_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })
                })?
            }
            _ => {
                return Err(MonsterAbilityPlanRejection {
                    reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                    enemy_target_count: 0,
                    friendly_risk_count: 0,
                });
            }
        };
        Ok(MonsterAbilityPlan {
            ability,
            base_weight,
            effective_weight: base_weight,
            enemy_target_count,
            friendly_risk_count,
            target,
        })
    }

    fn monster_targeted_ability_plan(
        &self,
        source_index: usize,
        ability: &AbilityDefinition,
        target: MonsterHostileTarget,
    ) -> Result<(MonsterAbilityTargetPlan, u16, u16), MonsterAbilityPlanRejection> {
        let origin = self.entities[source_index].position;
        let target_position = target.position();
        let (plan, affected_positions) = match &ability.effect {
            AbilityEffectDefinition::TransformTerrain {
                source_terrain_ids,
                target_terrain_id,
                radius,
            } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, false, false)?;
                let positions = self
                    .terrain_transform_positions_from(
                        ability,
                        Some(origin),
                        target_position,
                        source_terrain_ids,
                        target_terrain_id,
                        *radius,
                    )
                    .filter(|positions| !positions.is_empty())
                    .ok_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })?;
                (
                    MonsterAbilityTargetPlan::TerrainTransform {
                        target,
                        trace,
                        center: target_position,
                        positions: positions.clone(),
                    },
                    positions,
                )
            }
            AbilityEffectDefinition::TeleportLevel => {
                if !target.is_player() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let (upward_targets, downward_targets) = self.teleport_level_targets();
                if upward_targets.is_empty() && downward_targets.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                (
                    MonsterAbilityTargetPlan::Projectile { target, trace },
                    vec![target_position],
                )
            }
            AbilityEffectDefinition::AreaDamage { radius, .. } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, false, false)?;
                let affected_positions = self
                    .area_damage_cells(target_position, *radius)
                    .into_iter()
                    .map(|(_, position)| position)
                    .collect::<Vec<_>>();
                (
                    MonsterAbilityTargetPlan::Area {
                        target,
                        trace,
                        affected_positions: affected_positions.clone(),
                    },
                    affected_positions,
                )
            }
            AbilityEffectDefinition::BeamDamage { .. } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, false, true)?;
                let affected_positions = trace.traversed.clone();
                (
                    MonsterAbilityTargetPlan::Beam {
                        target,
                        trace,
                        affected_positions: affected_positions.clone(),
                    },
                    affected_positions,
                )
            }
            AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::BreathDamage { radius, .. } => {
                let direction = direction_toward(origin, target_position).ok_or(
                    MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    },
                )?;
                let (dx, dy) = direction.delta();
                let path = (1..=ability.target.range)
                    .map(|step| Position {
                        x: origin.x + dx * i32::from(step),
                        y: origin.y + dy * i32::from(step),
                    })
                    .collect::<Vec<_>>();
                let trace = self.trace_monster_path(origin, path);
                let cells = self.cone_damage_cells(origin, &trace.traversed, direction, *radius);
                if !cells
                    .iter()
                    .any(|(_, _, position)| *position == target_position)
                {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::OutOfRange,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                let affected_positions = cells
                    .into_iter()
                    .map(|(_, _, position)| position)
                    .collect::<Vec<_>>();
                (
                    MonsterAbilityTargetPlan::Cone {
                        target,
                        trace,
                        affected_positions: affected_positions.clone(),
                    },
                    affected_positions,
                )
            }
            AbilityEffectDefinition::Damage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::PolymorphTarget
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::DarkenRoom
            | AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Sequence { .. } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                (
                    MonsterAbilityTargetPlan::Projectile { target, trace },
                    vec![target_position],
                )
            }
            AbilityEffectDefinition::BirdDrop => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
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
                let destination = DELTAS
                    .iter()
                    .map(|(dx, dy)| Position {
                        x: origin.x + dx,
                        y: origin.y + dy,
                    })
                    .find(|position| {
                        self.index(*position).is_some()
                            && self.monster_hostile_target_can_enter_position(&target, *position)
                            && *position != self.player.position
                            && !self
                                .entities
                                .iter()
                                .any(|entity| entity.hp > 0 && entity.position == *position)
                    })
                    .ok_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })?;
                let escape_destinations_at_least = |minimum_distance| {
                    self.displacement_destinations(source_index, |position| {
                        let distance = origin
                            .x
                            .abs_diff(position.x)
                            .max(origin.y.abs_diff(position.y));
                        (minimum_distance..=10).contains(&distance)
                    })
                };
                let mut escape_destinations = escape_destinations_at_least(5);
                if escape_destinations.is_empty() {
                    escape_destinations = escape_destinations_at_least(0);
                }
                (
                    MonsterAbilityTargetPlan::BirdDrop {
                        target,
                        trace,
                        destination,
                        escape_destinations,
                    },
                    vec![target_position],
                )
            }
            AbilityEffectDefinition::BlinkTarget { radius } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                let radius = u32::from(*radius);
                let mut destinations = Vec::new();
                for y in 0..self.height {
                    for x in 0..self.width {
                        let position = Position {
                            x: i32::from(x),
                            y: i32::from(y),
                        };
                        if position == self.player.position
                            || target_position
                                .x
                                .abs_diff(position.x)
                                .max(target_position.y.abs_diff(position.y))
                                > radius
                            || !self.monster_hostile_target_can_enter_position(&target, position)
                            || self
                                .entities
                                .iter()
                                .any(|entity| entity.hp > 0 && entity.position == position)
                        {
                            continue;
                        }
                        destinations.push(position);
                    }
                }
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (
                    MonsterAbilityTargetPlan::BlinkTarget {
                        target,
                        trace,
                        destinations,
                    },
                    vec![target_position],
                )
            }
            AbilityEffectDefinition::TeleportAway {
                minimum_distance, ..
            } => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                // The banished target lands away from the caster; candidates
                // collect without RNG and the halved fallback mirrors
                // teleport-self on cramped floors.
                let banish_candidates = |minimum: u32| {
                    let mut candidates = Vec::new();
                    for y in 0..self.height {
                        for x in 0..self.width {
                            let position = Position {
                                x: i32::from(x),
                                y: i32::from(y),
                            };
                            if position == self.player.position
                                || !self
                                    .monster_hostile_target_can_enter_position(&target, position)
                                || origin
                                    .x
                                    .abs_diff(position.x)
                                    .max(origin.y.abs_diff(position.y))
                                    < minimum
                                || self
                                    .entities
                                    .iter()
                                    .any(|entity| entity.hp > 0 && entity.position == position)
                            {
                                continue;
                            }
                            candidates.push(position);
                        }
                    }
                    candidates
                };
                let minimum = u32::from(*minimum_distance);
                let mut destinations = banish_candidates(minimum);
                if destinations.is_empty() {
                    destinations = banish_candidates(minimum.div_ceil(2));
                }
                if destinations.is_empty() {
                    return Err(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    });
                }
                (
                    MonsterAbilityTargetPlan::BanishTarget {
                        target,
                        trace,
                        destinations,
                    },
                    vec![target_position],
                )
            }
            AbilityEffectDefinition::TeleportTarget => {
                let trace =
                    self.monster_projectile_trace(source_index, ability, &target, true, false)?;
                // The dragged target lands on the first open cell adjacent to
                // the caster, in the canonical eight-direction order.
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
                let destination = DELTAS
                    .iter()
                    .map(|(dx, dy)| Position {
                        x: origin.x + dx,
                        y: origin.y + dy,
                    })
                    .find(|position| {
                        self.index(*position).is_some()
                            && self.monster_hostile_target_can_enter_position(&target, *position)
                            && *position != self.player.position
                            && !self
                                .entities
                                .iter()
                                .any(|entity| entity.hp > 0 && entity.position == *position)
                    })
                    .ok_or(MonsterAbilityPlanRejection {
                        reason: MonsterAbilityRejectionReasonDto::NoSpace,
                        enemy_target_count: 0,
                        friendly_risk_count: 0,
                    })?;
                (
                    MonsterAbilityTargetPlan::DragTarget {
                        target,
                        trace,
                        destination,
                    },
                    vec![target_position],
                )
            }
            _ => {
                return Err(MonsterAbilityPlanRejection {
                    reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
                    enemy_target_count: 0,
                    friendly_risk_count: 0,
                });
            }
        };
        let (enemy_target_count, friendly_risk_count) =
            self.monster_footprint_faction_counts(source_index, &affected_positions);
        if friendly_risk_count > 0 {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::FriendlyRisk,
                enemy_target_count,
                friendly_risk_count,
            });
        }
        Ok((plan, enemy_target_count, friendly_risk_count))
    }

    fn monster_footprint_faction_counts(
        &self,
        source_index: usize,
        affected_positions: &[Position],
    ) -> (u16, u16) {
        let source_is_player_side = self.entity_is_player_side(source_index);
        let player_is_affected =
            affected_positions.contains(&self.player.position) && !self.player_is_dead();
        let mut enemies = u16::from(player_is_affected && !source_is_player_side);
        let mut friendlies = u16::from(player_is_affected && source_is_player_side);
        for (index, entity) in self.entities.iter().enumerate() {
            if index == source_index
                || entity.hp <= 0
                || !affected_positions.contains(&entity.position)
            {
                continue;
            }
            if self.entity_is_player_side(index) == source_is_player_side {
                friendlies = friendlies.saturating_add(1);
            } else {
                enemies = enemies.saturating_add(1);
            }
        }
        (enemies, friendlies)
    }

    fn monster_projectile_trace(
        &self,
        index: usize,
        ability: &AbilityDefinition,
        hostile_target: &MonsterHostileTarget,
        clean_shot: bool,
        continue_through_target: bool,
    ) -> Result<ProjectileTrace, MonsterAbilityPlanRejection> {
        let origin = self.entities[index].position;
        let target = hostile_target.position();
        if target == origin
            || self.index(target).is_none()
            || origin.x.abs_diff(target.x).max(origin.y.abs_diff(target.y))
                > u32::from(ability.target.range)
        {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::OutOfRange,
                enemy_target_count: 0,
                friendly_risk_count: 0,
            });
        }
        let path = if continue_through_target {
            projectile_path_through_target(origin, target, ability.target.range)
        } else {
            projectile_path_between(origin, target, ability.target.range)
        }
        .ok_or(MonsterAbilityPlanRejection {
            reason: MonsterAbilityRejectionReasonDto::InvalidTarget,
            enemy_target_count: 0,
            friendly_risk_count: 0,
        })?;
        let trace = self.trace_monster_path(origin, path);
        if !trace.traversed.contains(&target) {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::Blocked,
                enemy_target_count: 0,
                friendly_risk_count: 0,
            });
        }
        if clean_shot {
            for position in trace
                .traversed
                .iter()
                .filter(|position| **position != target)
            {
                if let Some((candidate_index, _)) =
                    self.entities
                        .iter()
                        .enumerate()
                        .find(|(candidate_index, entity)| {
                            *candidate_index != index
                                && entity.hp > 0
                                && entity.position == *position
                        })
                {
                    let enemy = self.entity_is_player_side(candidate_index)
                        != self.entity_is_player_side(index);
                    return Err(MonsterAbilityPlanRejection {
                        reason: if enemy {
                            MonsterAbilityRejectionReasonDto::Blocked
                        } else {
                            MonsterAbilityRejectionReasonDto::FriendlyRisk
                        },
                        enemy_target_count: u16::from(enemy),
                        friendly_risk_count: u16::from(!enemy),
                    });
                }
            }
        }
        Ok(trace)
    }

    fn trace_monster_path(&self, origin: Position, path: Vec<Position>) -> ProjectileTrace {
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        for position in path {
            if self.index(position).is_none() || !self.is_walkable(position) {
                impact = position;
                break;
            }
            impact = position;
            landing = position;
            traversed.push(position);
        }
        ProjectileTrace {
            origin,
            impact,
            landing,
            traversed,
        }
    }
}

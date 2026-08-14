// SPDX-License-Identifier: MPL-2.0

use super::{player_stats::ResolvedProjectileProfile, *};

fn projectile_raw_damage(
    rolled_ammunition_damage: i32,
    ammunition_slay_multiplier: i32,
    ammunition_to_damage: i32,
    ammunition_critical_multiplier_percent: i32,
    concentration_bonus_percent: i32,
    damage_multiplier_percent: u16,
    launcher_to_damage: i32,
) -> i32 {
    let ammunition_damage = rolled_ammunition_damage
        .saturating_mul(ammunition_slay_multiplier)
        .saturating_div(10)
        .saturating_add(ammunition_to_damage)
        .saturating_mul(ammunition_critical_multiplier_percent)
        / 100;
    let ammunition_damage =
        ammunition_damage.saturating_mul(100_i32.saturating_add(concentration_bonus_percent)) / 100;
    ammunition_damage.saturating_mul(i32::from(damage_multiplier_percent)) / 100
        + launcher_to_damage
}

fn concentrated_target_armor_class(armor_class: i32, concentration: u8) -> i32 {
    armor_class.saturating_mul(10_i32.saturating_sub(i32::from(concentration))) / 10
}

fn sniper_explosion_radius(concentration: u8) -> u8 {
    (concentration.saturating_add(1) / 2).saturating_add(1)
}

fn mana_brand_cost(damage_dice: u16, damage_sides: u16) -> u32 {
    1_u32.saturating_add(
        u32::from(damage_dice)
            .saturating_mul(u32::from(damage_sides))
            .saturating_div(7),
    )
}

fn mana_brand_multiplier(multiplier: i32) -> i32 {
    multiplier
        .saturating_mul(3)
        .saturating_div(2)
        .saturating_add(14)
        .min(150)
}

fn roll_sniper_needle_vital_hit(
    rng: &mut RfbRng,
    target_level: u32,
    concentration: u8,
    unique: bool,
) -> bool {
    let inner_bound = u64::from(target_level) / u64::from(3_u8.saturating_add(concentration));
    let inner = if inner_bound <= 1 {
        1
    } else {
        rng.bounded(inner_bound) + 1
    };
    let outer_bound = inner.saturating_add(u64::from(8_u8.saturating_sub(concentration)));
    let vital_hit = if outer_bound <= 1 {
        true
    } else {
        rng.bounded(outer_bound) == 0
    };
    vital_hit && !unique
}

fn projectile_critical_chance(
    to_hit: i32,
    ranged_skill: i32,
    level: u16,
    class_bonus_percent_per_level: u8,
    concentration_bonus_percent: i32,
    ammunition_critical_chance_percent: u16,
) -> i64 {
    let base = i64::from(to_hit)
        .saturating_mul(3)
        .saturating_add(i64::from(ranged_skill).saturating_mul(2))
        .max(0);
    let chance = base.saturating_add(
        base.saturating_mul(i64::from(level))
            .saturating_mul(i64::from(class_bonus_percent_per_level))
            / 100,
    );
    chance
        .saturating_add(chance.saturating_mul(i64::from(concentration_bonus_percent)) / 100)
        .saturating_mul(i64::from(ammunition_critical_chance_percent))
        / 100
}

#[allow(clippy::too_many_arguments)]
fn sniper_shot_damage_multiplier(
    mode: ProjectileMode,
    concentration: u8,
    slays: &BTreeMap<SlayTarget, SlayLevel>,
    brands: &BTreeSet<WeaponBrand>,
    light: ResistanceLevel,
    fire: ResistanceLevel,
    cold: ResistanceLevel,
    electricity: ResistanceLevel,
    disintegrate: ResistanceLevel,
    good: bool,
    evil: bool,
    nonliving: bool,
) -> i32 {
    let focus = i32::from(concentration);
    match mode {
        ProjectileMode::Sniper(SniperShotModeDefinition::Shining)
            if light == ResistanceLevel::Vulnerable =>
        {
            20 + focus
        }
        ProjectileMode::Sniper(SniperShotModeDefinition::Burning)
            if fire != ResistanceLevel::Immune =>
        {
            let mut multiplier = 15 + 3 * focus;
            if brands.contains(&WeaponBrand::Fire) {
                multiplier += 5;
            }
            if fire == ResistanceLevel::Vulnerable {
                multiplier *= 2;
            }
            multiplier
        }
        ProjectileMode::Sniper(SniperShotModeDefinition::Freezing)
            if cold != ResistanceLevel::Immune =>
        {
            let mut multiplier = 15 + 3 * focus;
            if brands.contains(&WeaponBrand::Cold) {
                multiplier += 5;
            }
            if cold == ResistanceLevel::Vulnerable {
                multiplier *= 2;
            }
            multiplier
        }
        ProjectileMode::Sniper(SniperShotModeDefinition::Thunder)
            if electricity != ResistanceLevel::Immune =>
        {
            let mut multiplier = 18 + 4 * focus;
            if brands.contains(&WeaponBrand::Electricity) {
                multiplier += 7;
            }
            multiplier
        }
        ProjectileMode::Sniper(SniperShotModeDefinition::Shatter)
            if disintegrate == ResistanceLevel::Vulnerable || nonliving =>
        {
            15 + 2 * focus
        }
        ProjectileMode::Sniper(SniperShotModeDefinition::Evil) if good => {
            15 + 4 * focus + sniper_alignment_slay_bonus(slays, SlayTarget::Good)
        }
        ProjectileMode::Sniper(SniperShotModeDefinition::Holy) if evil => {
            15 + 4 * focus
                + if light == ResistanceLevel::Vulnerable {
                    3 * focus
                } else {
                    0
                }
                + sniper_alignment_slay_bonus(slays, SlayTarget::Evil)
        }
        ProjectileMode::Sniper(SniperShotModeDefinition::Final) => 50,
        _ => 10,
    }
}

fn sniper_alignment_slay_bonus(slays: &BTreeMap<SlayTarget, SlayLevel>, target: SlayTarget) -> i32 {
    match slays.get(&target) {
        Some(SlayLevel::Slay) => 5,
        Some(SlayLevel::Kill) => 10,
        None => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectileMode {
    Normal,
    Sniper(SniperShotModeDefinition),
}

impl ProjectileMode {
    const fn continues_through_target(self) -> bool {
        matches!(
            self,
            Self::Sniper(SniperShotModeDefinition::Knockback | SniperShotModeDefinition::Piercing)
        )
    }

    const fn break_chance_override(self) -> Option<u8> {
        match self {
            Self::Sniper(
                SniperShotModeDefinition::Shatter
                | SniperShotModeDefinition::Piercing
                | SniperShotModeDefinition::Exploding
                | SniperShotModeDefinition::Needle
                | SniperShotModeDefinition::Final,
            ) => Some(100),
            Self::Sniper(SniperShotModeDefinition::Evil | SniperShotModeDefinition::Holy) => {
                Some(40)
            }
            _ => None,
        }
    }
}

struct ProjectileShotOutcome {
    trace: ProjectileTrace,
    hit_body: bool,
    fatal: bool,
}

#[derive(Default)]
struct ProjectileCollisionOutcome {
    knockback_landing: Option<Position>,
    fatal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PlayerMeleeOutcome {
    pub(super) attacks_used: u16,
    pub(super) attacks_available: u16,
    pub(super) killed: bool,
}

impl Game {
    fn actor_can_be_angered_by_ranged_damage(&self, index: usize) -> bool {
        let Some(casting) = self
            .actor_runtime_definition(&self.entities[index])
            .and_then(|definition| definition.monster_casting.as_ref())
        else {
            return false;
        };
        casting.abilities.iter().any(|candidate| {
            self.content
                .ability(&candidate.ability_id)
                .is_some_and(|ability| {
                    ability.effect.ordered_effects().iter().any(|effect| {
                        matches!(
                            effect,
                            AbilityEffectDefinition::Damage { .. }
                                | AbilityEffectDefinition::Malediction { .. }
                                | AbilityEffectDefinition::AreaDamage { .. }
                                | AbilityEffectDefinition::JumpDamage { .. }
                                | AbilityEffectDefinition::BeamDamage { .. }
                                | AbilityEffectDefinition::LightLine { .. }
                                | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                                | AbilityEffectDefinition::BoltOrAreaDamage { .. }
                                | AbilityEffectDefinition::ConeDamage { .. }
                                | AbilityEffectDefinition::BreathDamage { .. }
                                | AbilityEffectDefinition::CurseDamage { .. }
                                | AbilityEffectDefinition::DeathRay { .. }
                                | AbilityEffectDefinition::Summon { .. }
                                | AbilityEffectDefinition::SummonCategory { .. }
                        )
                    })
                })
        })
    }

    pub(super) fn anger_monster_from_spell_damage(&mut self, index: usize, damage: i32) {
        if damage <= 0
            || self.player_suppresses_distant_spell_anger()
            || rfb_distance(self.player.position, self.entities[index].position) <= 1
            || !self.actor_can_be_angered_by_ranged_damage(index)
        {
            return;
        }
        let current = u32::from(self.entities[index].anger);
        let mut increase = 10_u32.saturating_add(current / 2);
        if damage < 450 {
            increase = increase
                .saturating_mul(u32::try_from(damage.saturating_add(50)).unwrap_or(0))
                / 500;
        }
        self.entities[index].anger = u8::try_from(current.saturating_add(increase).min(100))
            .expect("bounded monster anger must fit u8");
    }

    pub(super) fn anger_monster_from_projectile_damage(&mut self, index: usize, damage: i32) {
        if damage <= 0
            || self.player_suppresses_distant_projectile_anger()
            || rfb_distance(self.player.position, self.entities[index].position) <= 1
            || !self.actor_can_be_angered_by_ranged_damage(index)
        {
            return;
        }
        let current = u32::from(self.entities[index].anger);
        let mut increase = 5_u32.saturating_add(current / 4);
        if damage < 125 {
            increase = increase
                .saturating_mul(u32::try_from(damage.saturating_add(25)).unwrap_or(0))
                / 150;
        }
        self.entities[index].anger = u8::try_from(current.saturating_add(increase).min(100))
            .expect("bounded monster anger must fit u8");
    }

    pub(super) fn player_projectile_path_for_mode(
        &self,
        target: &TargetSelection,
        range: u16,
        mode: ProjectileMode,
    ) -> Option<Vec<Position>> {
        if !mode.continues_through_target() {
            return self.projectile_path(target, range);
        }
        match target {
            TargetSelection::Direction { .. } => self.projectile_path(target, range),
            TargetSelection::Position { position } => {
                self.targeted_projectile_path_through_target(*position, range)
            }
            TargetSelection::Entity { entity_id } => self
                .entities
                .iter()
                .find(|entity| entity.id == *entity_id && self.entity_is_visible_to_player(entity))
                .and_then(|entity| {
                    self.targeted_projectile_path_through_target(entity.position, range)
                }),
            TargetSelection::SelfTarget
            | TargetSelection::Item { .. }
            | TargetSelection::Town { .. } => None,
        }
    }
}

fn monster_stun_amount(damage: i32) -> i32 {
    let damage = damage.max(0);
    if damage < 1 {
        return 1;
    }
    for ((left_damage, left_stun), (right_damage, right_stun)) in
        [(1, 1), (10, 10), (100, 25), (500, 50)]
            .into_iter()
            .zip([(10, 10), (100, 25), (500, 50)])
    {
        if damage < right_damage {
            return left_stun
                + (damage - left_damage) * (right_stun - left_stun) / (right_damage - left_damage);
        }
    }
    50
}

impl Game {
    fn passive_teleport_actor(
        &mut self,
        index: usize,
        distance: u32,
        changed: &mut BTreeSet<Position>,
    ) {
        let from = self.entities[index].position;
        let mut minimum = distance / 2;
        let mut maximum = distance.max(1);
        for _ in 0..8 {
            let candidates = (0..self.height)
                .flat_map(|y| {
                    (0..self.width).map(move |x| Position {
                        x: i32::from(x),
                        y: i32::from(y),
                    })
                })
                .filter(|position| {
                    let actual_distance = rfb_distance(from, *position);
                    actual_distance >= minimum
                        && actual_distance <= maximum
                        && *position != self.player.position
                        && self.actor_can_enter_position(index, *position)
                        && !self
                            .entities
                            .iter()
                            .enumerate()
                            .any(|(other_index, entity)| {
                                other_index != index
                                    && entity.hp > 0
                                    && entity.position == *position
                            })
                })
                .collect::<Vec<_>>();
            if !candidates.is_empty() {
                let candidate_index = if candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(self.rng.bounded(candidates.len() as u64))
                        .expect("bounded passive teleport destination must fit usize")
                };
                self.entities[index].position = candidates[candidate_index];
                changed.insert(from);
                changed.insert(candidates[candidate_index]);
                return;
            }
            minimum /= 2;
            maximum = maximum.saturating_mul(2);
        }
    }

    pub(super) fn resolve_ability_damage_rider(
        &mut self,
        index: usize,
        ability_id: &str,
        damage_type: DamageType,
        raw_damage: i32,
        resistance: ResistanceLevel,
        changed: &mut BTreeSet<Position>,
    ) {
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("ability target definition must remain available")
            .clone();
        let has_tag = |tag: &str| definition.tags.iter().any(|candidate| candidate == tag);
        let level = u64::from(definition.level);
        let stun_immune = self.actor_has_status_immunity(index, STATUS_STUN);
        let stun_amount = monster_stun_amount(raw_damage);

        match damage_type {
            DamageType::Rock
                if !stun_immune
                    && matches!(
                        self.entities[index].resistances.level(DamageType::Sound),
                        ResistanceLevel::Vulnerable | ResistanceLevel::Normal
                    ) =>
            {
                let save_maximum = (1 + level / 12).saturating_mul(level).max(1);
                if self.rng.bounded(save_maximum)
                    < u64::try_from(raw_damage.max(0)).unwrap_or(u64::MAX)
                {
                    self.apply_actor_melee_status(index, STATUS_STUN, stun_amount, ability_id);
                }
            }
            DamageType::Ice if !stun_immune => {
                let intensity =
                    u16::try_from(self.rng.bounded(15) + 1).expect("ice stun roll must fit u16");
                self.apply_actor_melee_status(index, STATUS_STUN, i32::from(intensity), ability_id);
            }
            DamageType::Plasma | DamageType::Water | DamageType::Sound
                if !stun_immune
                    && !matches!(
                        resistance,
                        ResistanceLevel::Resistant
                            | ResistanceLevel::Strong
                            | ResistanceLevel::Immune
                    ) =>
            {
                let save_maximum = (1 + level / 12).saturating_mul(level).max(1);
                if self.rng.bounded(save_maximum)
                    < u64::try_from(raw_damage.max(0)).unwrap_or(u64::MAX)
                {
                    self.apply_actor_melee_status(index, STATUS_STUN, stun_amount, ability_id);
                }
            }
            DamageType::Inertia
                if !matches!(
                    resistance,
                    ResistanceLevel::Resistant | ResistanceLevel::Strong | ResistanceLevel::Immune
                ) && !has_tag("unique") =>
            {
                let save_sides = u64::try_from((raw_damage - 10).max(1)).unwrap_or(1);
                if level <= self.rng.bounded(save_sides) + 11 {
                    self.entities[index].minor_slow =
                        self.entities[index].minor_slow.saturating_add(5).min(10);
                }
            }
            DamageType::Gravity
                if !matches!(
                    resistance,
                    ResistanceLevel::Resistant | ResistanceLevel::Strong | ResistanceLevel::Immune
                ) =>
            {
                let teleport_resisted = has_tag("guardian")
                    || (has_tag("resist-teleport")
                        && (has_tag("unique") || level > self.rng.bounded(100) + 1));
                if !has_tag("unique") {
                    let save_sides = u64::try_from((raw_damage - 10).max(1)).unwrap_or(1);
                    if level <= self.rng.bounded(save_sides) + 11 {
                        self.apply_actor_melee_status(index, STATUS_SLOW, 50, ability_id);
                    }
                    if level <= self.rng.bounded(save_sides) + 11 {
                        self.apply_actor_melee_status(index, STATUS_STUN, stun_amount, ability_id);
                    }
                }
                if !teleport_resisted {
                    self.passive_teleport_actor(index, 10, changed);
                }
            }
            DamageType::Telekinesis => {
                let moves = self.rng.bounded(4) == 0 && !has_tag("guardian");
                let level_multiplier = if has_tag("unique") { 2_u64 } else { 1 };
                let save_sides = u64::try_from(raw_damage.max(1)).unwrap_or(1);
                if level_multiplier * level <= 5 + self.rng.bounded(save_sides) + 1 {
                    self.apply_actor_melee_status(index, STATUS_STUN, stun_amount, ability_id);
                }
                if moves {
                    self.passive_teleport_actor(index, 7, changed);
                }
            }
            _ => {}
        }
    }

    pub(super) fn resolve_player_projectile(
        &mut self,
        target: TargetSelection,
        mode: ProjectileMode,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(profile) = self.player_projectile_profile() else {
            events.push(DomainEvent::ProjectileUnavailable);
            return Ok(());
        };
        let Some(ammo_item_id) = &profile.ammo_item_id else {
            events.push(DomainEvent::ProjectileAmmoUnavailable {
                ammo_kind_id: profile.ammo_kind_id,
            });
            return Ok(());
        };
        let ammo_item_id = ammo_item_id.clone();
        let ammo_quantity = self
            .items
            .iter()
            .find(|item| item.id == ammo_item_id && item.location == ItemLocation::Inventory)
            .map_or(0, |item| item.quantity);
        if ammo_quantity == 0 {
            events.push(DomainEvent::ProjectileAmmoUnavailable {
                ammo_kind_id: profile.ammo_kind_id,
            });
            return Ok(());
        }
        let mode = if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Double)
            && ammo_quantity < 2
        {
            ProjectileMode::Normal
        } else {
            mode
        };
        let Some(path) = self.player_projectile_path_for_mode(&target, profile.range, mode) else {
            events.push(DomainEvent::ProjectileTargetUnavailable);
            return Ok(());
        };
        let starting_concentration = self.sniper_concentration;
        let shot_concentration = if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Double)
        {
            starting_concentration.saturating_add(1) / 2
        } else {
            starting_concentration
        };
        let shot_count = if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Double) {
            2
        } else {
            1
        };
        for _ in 0..shot_count {
            let Some(ammunition) = self.take_inventory_item(&ammo_item_id)? else {
                break;
            };
            let outcome = self.resolve_one_player_projectile(
                &profile,
                &path,
                mode,
                shot_concentration,
                events,
                changed,
                removed_entities,
            )?;
            self.settle_projectile_ammunition(
                ammunition,
                outcome.trace.landing,
                outcome.hit_body,
                mode.break_chance_override()
                    .unwrap_or(profile.ammo_break_chance_percent),
                events,
                changed,
            );
            if outcome.fatal {
                break;
            }
        }
        self.sniper_concentration = 0;
        self.apply_ranged_easy_tiring_fatigue(7_500_i32.saturating_div(profile.base_shot));
        self.check_human_dexterity_sprain(250, events);
        if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Retreat) {
            let radius = 10_u16.saturating_add(u16::from(starting_concentration).saturating_mul(2));
            let candidates = self.random_teleport_candidates(radius);
            if !candidates.is_empty() {
                let index = usize::try_from(self.rng.bounded(candidates.len() as u64))
                    .expect("bounded teleport candidate index must fit usize");
                events.extend(self.relocate_player(candidates[index], changed));
            }
        }
        if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Final) {
            let slow_duration = i32::try_from(self.rng.bounded(7) + 7).unwrap_or(i32::MAX);
            let stun_duration = i32::try_from(self.rng.bounded(25) + 1).unwrap_or(i32::MAX);
            let source_id = self.player.kind_id.clone();
            self.apply_player_melee_status(STATUS_SLOW, slow_duration, &source_id);
            self.apply_player_melee_status(STATUS_STUN, stun_duration, &source_id);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_one_player_projectile(
        &mut self,
        profile: &ResolvedProjectileProfile,
        path: &[Position],
        mode: ProjectileMode,
        concentration: u8,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<ProjectileShotOutcome, CoreError> {
        let origin = self.player.position;
        let mut active_concentration = concentration;
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut collided = false;
        let mut broke_wall = false;
        let mut fatal = false;
        for (path_index, position) in path.iter().copied().enumerate() {
            impact = position;
            let Some(terrain_index) = self.index(position) else {
                break;
            };
            let target_index = self
                .entities
                .iter()
                .position(|entity| entity.hp > 0 && entity.position == position);
            if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Shatter)
                && target_index.is_none()
            {
                let replacement = self
                    .content
                    .terrain(&self.terrain[terrain_index])
                    .filter(|terrain| !terrain.walkable)
                    .and_then(|terrain| terrain.digging.as_ref())
                    .filter(|digging| digging.resolution != TerrainDiggingResolution::Permanent)
                    .and_then(|digging| digging.result_terrain_id.clone());
                if let Some(replacement) = replacement {
                    self.replace_terrain_from_source(
                        position,
                        &replacement,
                        super::terrain::TerrainChangeSource::Projectile,
                        events,
                        changed,
                    );
                    broke_wall = true;
                    break;
                }
            }
            if !self.is_walkable(position) {
                break;
            }
            landing = position;
            traversed.push(position);
            if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Evil) {
                self.glow[terrain_index] = false;
                self.revealed_terrain.remove(&position);
                changed.insert(position);
            }
            if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Shining)
                && !self.glow[terrain_index]
            {
                self.glow[terrain_index] = true;
                changed.insert(position);
            }
            if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Disarm) {
                let replacement = self
                    .content
                    .terrain(&self.terrain[terrain_index])
                    .and_then(|terrain| terrain.trap.as_ref())
                    .map(|trap| trap.disarm_to_terrain_id.clone());
                if let Some(replacement) = replacement {
                    self.replace_terrain_from_source(
                        position,
                        &replacement,
                        super::terrain::TerrainChangeSource::Projectile,
                        events,
                        changed,
                    );
                }
            }
            let Some(target_index) = target_index else {
                continue;
            };
            collided = true;
            let trace = ProjectileTrace {
                origin,
                impact,
                landing,
                traversed: traversed.clone(),
            };
            let outcome = self.resolve_player_projectile_collision(
                target_index,
                profile,
                mode,
                active_concentration,
                trace,
                &path[path_index.saturating_add(1)..],
                events,
                changed,
                removed_entities,
            )?;
            if let Some(knockback_landing) = outcome.knockback_landing {
                landing = knockback_landing;
            }
            fatal = outcome.fatal;
            if mode != ProjectileMode::Sniper(SniperShotModeDefinition::Piercing) {
                break;
            }
            if active_concentration == 0 {
                break;
            }
            active_concentration -= 1;
        }
        let trace = ProjectileTrace {
            origin,
            impact,
            landing,
            traversed,
        };
        if !collided && !broke_wall {
            events.push(DomainEvent::ProjectileLanded {
                trace: trace.clone(),
            });
        }
        Ok(ProjectileShotOutcome {
            trace,
            hit_body: collided || broke_wall,
            fatal,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_player_projectile_collision(
        &mut self,
        index: usize,
        profile: &ResolvedProjectileProfile,
        mode: ProjectileMode,
        concentration: u8,
        trace: ProjectileTrace,
        remaining_path: &[Position],
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<ProjectileCollisionOutcome, CoreError> {
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("projectile target definition must remain available")
            .clone();
        let target_kind_id = definition.id.clone();
        let target_entity_id = self.entities[index].id.clone();
        self.entities[index].alerted = true;
        let attacker = self.player_derived_stats();
        let proficiency_modifier = self
            .items
            .iter()
            .find(|item| item.id == profile.source_item_id)
            .and_then(|item| self.weapon_proficiency_hit_modifier(&item.kind_id));
        if let Some(item_kind_id) =
            self.train_weapon_proficiency(&profile.source_item_id, definition.level)
        {
            events.push(DomainEvent::WeaponProficiencyImproved { item_kind_id });
        }
        if let Some(event) = self.train_riding_from_archery() {
            events.push(event);
        }
        let mut ranged_skill = attacker.ranged_skill.with_modifier(
            StatLayer::Equipment,
            profile.ammo_kind_id.clone(),
            profile.to_hit,
            StatBounds::NON_NEGATIVE,
        );
        if let Some((base_item_id, modifier)) = proficiency_modifier
            && modifier != 0
        {
            ranged_skill = ranged_skill.with_modifier(
                StatLayer::Class,
                base_item_id,
                modifier,
                StatBounds::NON_NEGATIVE,
            );
        }
        let target = self.actor_derived_stats(&self.entities[index], &definition, false);
        let concentration_bonus = self.sniper_concentration_bonus_percent(concentration);
        let focused_armor_class = if concentration == 0 {
            target.armor_class.clone()
        } else {
            let value = concentrated_target_armor_class(target.armor_class.value, concentration);
            target.armor_class.with_modifier(
                StatLayer::Class,
                "sniper-concentration",
                value.saturating_sub(target.armor_class.value),
                StatBounds::NON_NEGATIVE,
            )
        };
        changed.insert(self.entities[index].position);
        if !self
            .resolve_player_hit_check(CheckContext {
                kind: CheckKind::ProjectileHit,
                actor_id: self.player.id.clone(),
                target_id: Some(target_entity_id.clone()),
                ability: ranged_skill,
                difficulty: focused_armor_class,
            })
            .succeeded()
        {
            events.push(DomainEvent::ProjectileMissed {
                target_kind_id,
                trace,
            });
            return Ok(ProjectileCollisionOutcome::default());
        }
        if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Holy)
            && let Some(terrain_index) = self.index(self.entities[index].position)
        {
            self.glow[terrain_index] = true;
            changed.insert(self.entities[index].position);
        }
        if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Needle) {
            let unique = definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
            let vital_hit = roll_sniper_needle_vital_hit(
                &mut self.rng,
                definition.level,
                concentration,
                unique,
            );
            let damage = resolve_damage(
                DamagePacket::new(
                    if vital_hit {
                        self.entities[index].hp.saturating_add(1)
                    } else {
                        1
                    },
                    DamageType::Physical,
                ),
                ResistanceLevel::Normal,
            );
            return self.commit_player_projectile_damage(
                index,
                target_kind_id,
                target_entity_id,
                damage,
                trace,
                mode,
                remaining_path,
                events,
                changed,
                removed_entities,
            );
        }
        let ammunition_critical_multiplier = self.roll_projectile_critical_multiplier(
            profile.ammunition_weight_tenths_pound,
            profile.to_hit,
            attacker.ranged_skill.value,
            profile.ammunition_type,
            concentration,
        );
        let ammunition_slay_multiplier = self
            .player_projectile_damage_multiplier(profile, &self.entities[index], &definition)
            .max(sniper_shot_damage_multiplier(
                mode,
                concentration,
                &profile.ammunition_slays,
                &profile.ammunition_brands,
                self.entities[index].resistances.level(DamageType::Light),
                self.entities[index].resistances.level(DamageType::Fire),
                self.entities[index].resistances.level(DamageType::Cold),
                self.entities[index]
                    .resistances
                    .level(DamageType::Electricity),
                self.entities[index]
                    .resistances
                    .level(DamageType::Disintegrate),
                actor_matches_category(&definition, "good"),
                actor_matches_category(&definition, "evil"),
                actor_matches_category(&definition, "nonliving"),
            ));
        let raw_damage = projectile_raw_damage(
            self.roll_damage(profile.damage_dice, profile.damage_sides),
            ammunition_slay_multiplier,
            profile.ammunition_to_damage,
            ammunition_critical_multiplier,
            concentration_bonus,
            profile.damage_multiplier_percent,
            profile.launcher_to_damage,
        )
        .max(0);
        if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Exploding) {
            return self.resolve_sniper_explosion(
                self.entities[index].position,
                sniper_explosion_radius(concentration),
                raw_damage,
                trace,
                events,
                changed,
                removed_entities,
            );
        }
        let resistance = self.entities[index].resistances.level(profile.damage_type);
        let damage = resolve_armored_damage(
            raw_damage,
            profile.damage_type,
            target.armor_class.value,
            resistance,
        );
        self.commit_player_projectile_damage(
            index,
            target_kind_id,
            target_entity_id,
            damage,
            trace,
            mode,
            remaining_path,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_player_projectile_damage(
        &mut self,
        index: usize,
        target_kind_id: String,
        target_entity_id: String,
        damage: DamageOutcome,
        trace: ProjectileTrace,
        mode: ProjectileMode,
        remaining_path: &[Position],
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<ProjectileCollisionOutcome, CoreError> {
        let application =
            plan_damage_application(&self.entities[index], damage, FatalityPolicy::AtOrBelowZero);
        commit_damage_application(&mut self.entities[index], &application);
        events.push(DomainEvent::ProjectileHit {
            target_kind_id: target_kind_id.clone(),
            damage,
            trace: trace.clone(),
        });
        self.wake_entity_after_damage(index, damage.applied, events);
        if !application.fatal {
            self.anger_monster_from_projectile_damage(index, damage.applied);
            self.resolve_monster_fear_aura(index, "hurt", true, events);
        }
        if application.fatal {
            self.resolve_actor_death(
                index,
                DomainEvent::ProjectileSlew {
                    target_kind_id,
                    damage,
                    trace,
                },
                events,
                changed,
                removed_entities,
            )?;
            return Ok(ProjectileCollisionOutcome {
                fatal: true,
                ..ProjectileCollisionOutcome::default()
            });
        }
        if mode == ProjectileMode::Sniper(SniperShotModeDefinition::Knockback) {
            let distance = 3_usize.saturating_add(
                usize::try_from(self.rng.bounded(5) + 1)
                    .expect("knockback distance must fit usize"),
            );
            return Ok(ProjectileCollisionOutcome {
                knockback_landing: self.knockback_projectile_target(
                    &target_entity_id,
                    remaining_path,
                    distance,
                    changed,
                ),
                fatal: false,
            });
        }
        Ok(ProjectileCollisionOutcome::default())
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_sniper_explosion(
        &mut self,
        center: Position,
        radius: u8,
        raw_damage: i32,
        trace: ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<ProjectileCollisionOutcome, CoreError> {
        let (affected_positions, targets) = self.area_damage_targets(center, radius, None);
        changed.extend(affected_positions);
        let mut original_target_fatal = false;
        for (target_entity_id, distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_entity_id)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("explosion target definition must remain available")
                .clone();
            let target = self.actor_derived_stats(&self.entities[index], &definition, false);
            let prepared = rfb_area_damage(raw_damage, distance);
            let damage = resolve_armored_damage(
                prepared,
                DamageType::Physical,
                target.armor_class.value,
                self.entities[index].resistances.level(DamageType::Physical),
            );
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            events.push(DomainEvent::ProjectileHit {
                target_kind_id: definition.id.clone(),
                damage,
                trace: trace.clone(),
            });
            self.wake_entity_after_damage(index, damage.applied, events);
            if application.fatal {
                original_target_fatal |= self.entities[index].position == center;
                self.resolve_actor_death(
                    index,
                    DomainEvent::ProjectileSlew {
                        target_kind_id: definition.id,
                        damage,
                        trace: trace.clone(),
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            } else {
                self.anger_monster_from_projectile_damage(index, damage.applied);
                self.resolve_monster_fear_aura(index, "hurt", true, events);
            }
        }
        Ok(ProjectileCollisionOutcome {
            fatal: original_target_fatal,
            ..ProjectileCollisionOutcome::default()
        })
    }

    fn knockback_projectile_target(
        &mut self,
        actor_id: &str,
        remaining_path: &[Position],
        distance: usize,
        changed: &mut BTreeSet<Position>,
    ) -> Option<Position> {
        let mut landing = None;
        for position in remaining_path.iter().copied().take(distance) {
            let index = self
                .entities
                .iter()
                .position(|entity| entity.id == actor_id)?;
            if !self.actor_can_enter_position(index, position)
                || self
                    .entities
                    .iter()
                    .any(|entity| entity.hp > 0 && entity.position == position)
            {
                break;
            }
            let previous = self.entities[index].position;
            self.entities[index].position = position;
            changed.insert(previous);
            changed.insert(position);
            landing = Some(position);
        }
        landing
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_ability_damage_to_entity(
        &mut self,
        index: usize,
        ability_id: &str,
        damage_type: DamageType,
        raw_damage: i32,
        trace: ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<DamageOutcome, CoreError> {
        self.resolve_ability_damage_to_entity_with_resistance(
            index,
            ability_id,
            damage_type,
            raw_damage,
            trace,
            None,
            true,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_weak_light_damage_to_entity(
        &mut self,
        index: usize,
        ability_id: &str,
        raw_damage: i32,
        trace: ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<DamageOutcome, CoreError> {
        self.resolve_ability_damage_to_entity_with_resistance(
            index,
            ability_id,
            DamageType::Light,
            raw_damage,
            trace,
            Some(ResistanceLevel::Normal),
            true,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_stone_to_mud_damage_to_entity(
        &mut self,
        index: usize,
        ability_id: &str,
        raw_damage: i32,
        trace: ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<DamageOutcome, CoreError> {
        self.resolve_ability_damage_to_entity_with_resistance(
            index,
            ability_id,
            DamageType::Disintegrate,
            raw_damage,
            trace,
            Some(ResistanceLevel::Normal),
            true,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn resolve_ability_damage_to_entity_with_resistance(
        &mut self,
        index: usize,
        ability_id: &str,
        damage_type: DamageType,
        raw_damage: i32,
        trace: ProjectileTrace,
        resistance_override: Option<ResistanceLevel>,
        award_player_kill: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<DamageOutcome, CoreError> {
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("ability target definition must remain available")
            .clone();
        let target_kind_id = definition.id.clone();
        let has_tag = |tag: &str| definition.tags.iter().any(|candidate| candidate == tag);
        let raw_damage = match damage_type {
            DamageType::HellFire if has_tag("good") => raw_damage.saturating_mul(2),
            DamageType::HolyFire if has_tag("good") => 0,
            DamageType::HolyFire if has_tag("evil") => raw_damage.saturating_mul(2),
            DamageType::HolyFire => {
                let divisor =
                    i32::try_from(self.rng.bounded(6) + 7).expect("holy-fire divisor must fit i32");
                raw_damage.saturating_mul(3) / divisor
            }
            _ => raw_damage,
        };
        self.entities[index].alerted = true;
        changed.insert(self.entities[index].position);
        let target = self.actor_derived_stats(&self.entities[index], &definition, false);
        let resistance = resistance_override.unwrap_or_else(|| {
            let direct = self.entities[index].resistances.level(match damage_type {
                DamageType::Ice => DamageType::Cold,
                damage_type => damage_type,
            });
            if damage_type == DamageType::Rocket && direct == ResistanceLevel::Normal {
                match self.entities[index].resistances.level(DamageType::Shards) {
                    ResistanceLevel::Resistant
                    | ResistanceLevel::Strong
                    | ResistanceLevel::Immune => ResistanceLevel::Resistant,
                    ResistanceLevel::Vulnerable | ResistanceLevel::Normal => {
                        ResistanceLevel::Normal
                    }
                }
            } else {
                direct
            }
        });
        let damage = resolve_armored_damage(
            raw_damage,
            damage_type,
            target.armor_class.value,
            resistance,
        );
        let application =
            plan_damage_application(&self.entities[index], damage, FatalityPolicy::AtOrBelowZero);
        commit_damage_application(&mut self.entities[index], &application);
        events.push(DomainEvent::AbilityHit {
            ability_id: ability_id.to_owned(),
            target_kind_id: target_kind_id.clone(),
            damage,
            trace: trace.clone(),
        });
        self.wake_entity_after_damage(index, damage.applied, events);
        if !application.fatal {
            self.anger_monster_from_spell_damage(index, damage.applied);
            self.resolve_ability_damage_rider(
                index,
                ability_id,
                damage_type,
                raw_damage,
                resistance,
                changed,
            );
            self.resolve_monster_fear_aura(index, "hurt", true, events);
        }
        if application.fatal && award_player_kill {
            self.resolve_actor_death(
                index,
                DomainEvent::AbilitySlew {
                    ability_id: ability_id.to_owned(),
                    target_kind_id,
                    damage,
                    trace,
                },
                events,
                changed,
                removed_entities,
            )?;
        } else if application.fatal {
            self.resolve_actor_death_without_rewards(
                index,
                Some(DomainEvent::AbilitySlew {
                    ability_id: ability_id.to_owned(),
                    target_kind_id,
                    damage,
                    trace,
                }),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(damage)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_ability_damage_to_entity_without_rewards(
        &mut self,
        index: usize,
        ability_id: &str,
        damage_type: DamageType,
        raw_damage: i32,
        trace: ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<DamageOutcome, CoreError> {
        self.resolve_ability_damage_to_entity_with_resistance(
            index,
            ability_id,
            damage_type,
            raw_damage,
            trace,
            None,
            false,
            events,
            changed,
            removed_entities,
        )
    }

    pub(super) fn throw_inventory_item(
        &mut self,
        item_id: &str,
        direction: rfb_protocol::Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let Some(item) = self.items.iter().find(|item| {
            item.id == item_id && item.location == ItemLocation::Inventory && item.quantity > 0
        }) else {
            events.push(DomainEvent::ItemThrowUnavailable);
            return Ok(());
        };
        let definition = self
            .content
            .item(&item.kind_id)
            .expect("throwable item definition must remain available");
        let mighty_throw = self.player_has_mighty_throw();
        let range = throw_range(definition.weight_tenths_pound, mighty_throw);
        let profile = definition
            .throw_profile
            .as_ref()
            .map(|profile| ResolvedThrowProfile {
                to_hit: profile
                    .to_hit
                    .saturating_add(i32::from(item.enchantments.to_hit)),
                to_damage: profile
                    .to_damage
                    .saturating_add(i32::from(item.enchantments.to_damage)),
                damage_dice: profile.damage_dice,
                damage_sides: profile.damage_sides,
                damage_type: DamageType::from(profile.damage_type),
            });
        let Some(mut thrown) = self.take_inventory_item(item_id)? else {
            events.push(DomainEvent::ItemThrowUnavailable);
            return Ok(());
        };
        let source_kind_id = thrown.kind_id.clone();
        self.mark_item_tried(&source_kind_id);
        let path = self
            .projectile_path(&TargetSelection::Direction { direction }, range)
            .expect("direction targeting must always produce a path");
        let (trace, target_index) = self.trace_projectile_path(path);
        let landing = trace.landing;
        if let (Some(profile), Some(index)) = (profile, target_index) {
            let target_definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("throw target definition must remain available")
                .clone();
            let target_kind_id = target_definition.id.clone();
            self.entities[index].alerted = true;
            let attacker = self.player_derived_stats();
            let target = self.actor_derived_stats(&self.entities[index], &target_definition, false);
            let ability = attacker.throwing_skill.with_modifier(
                StatLayer::Equipment,
                &thrown.id,
                profile.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            changed.insert(self.entities[index].position);
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::ThrowHit,
                    actor_id: self.player.id.clone(),
                    target_id: Some(self.entities[index].id.clone()),
                    ability,
                    difficulty: target.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::ItemThrowMissed {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id,
                    trace: trace.clone(),
                });
            } else {
                let raw_damage = self
                    .roll_damage(profile.damage_dice, profile.damage_sides)
                    .saturating_add(profile.to_damage)
                    .saturating_mul(if mighty_throw { 2 } else { 1 })
                    .max(0);
                let resistance = self.entities[index].resistances.level(profile.damage_type);
                let damage = resolve_armored_damage(
                    raw_damage,
                    profile.damage_type,
                    target.armor_class.value,
                    resistance,
                );
                let application = plan_damage_application(
                    &self.entities[index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[index], &application);
                events.push(DomainEvent::ItemThrowHit {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    damage,
                    trace: trace.clone(),
                });
                self.wake_entity_after_damage(index, damage.applied, events);
                if !application.fatal {
                    self.anger_monster_from_projectile_damage(index, damage.applied);
                    self.resolve_monster_fear_aura(index, "hurt", true, events);
                }
                if application.fatal {
                    self.resolve_actor_death(
                        index,
                        DomainEvent::ItemThrowSlew {
                            source_kind_id: source_kind_id.clone(),
                            target_kind_id,
                            damage,
                            trace: trace.clone(),
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
        } else {
            events.push(DomainEvent::ItemThrown {
                target_kind_id: source_kind_id,
                trace,
            });
        }
        thrown.location = ItemLocation::Ground(landing);
        let thrown_id = thrown.id.clone();
        self.items.push(thrown);
        changed.insert(landing);
        self.force_open_capture_ball(&thrown_id, landing, true, events, changed);
        self.apply_easy_tiring_fatigue(STANDARD_ACTION_COST);
        Ok(())
    }

    fn draconian_strike_damage_multiplier(
        &self,
        profile: &super::player_stats::ResolvedAttackProfile,
        target: &Actor,
        definition: &rfb_content::ActorDefinition,
        strike_mode: Option<DraconianStrikeModeDefinition>,
    ) -> i32 {
        let multiplier = self.player_melee_damage_multiplier(profile, target, definition);
        let damage_type = match strike_mode {
            Some(DraconianStrikeModeDefinition::Fire) => DamageType::Fire,
            Some(DraconianStrikeModeDefinition::Cold) => DamageType::Cold,
            Some(DraconianStrikeModeDefinition::Electricity) => DamageType::Electricity,
            Some(DraconianStrikeModeDefinition::Acid) => DamageType::Acid,
            Some(DraconianStrikeModeDefinition::Poison) => DamageType::Poison,
            _ => return multiplier,
        };
        if target.resistances.level(damage_type) == ResistanceLevel::Immune {
            multiplier
        } else if profile.source_item_id.is_some() {
            multiplier.max(24)
        } else {
            multiplier.max(17)
        }
    }

    fn resolve_draconian_stunning_strike(
        &mut self,
        index: usize,
        raw_damage: i32,
        definition: &rfb_content::ActorDefinition,
    ) {
        if self.actor_has_status_immunity(index, STATUS_STUN)
            || definition.tags.iter().any(|tag| tag == "resist-all")
        {
            return;
        }
        let unique = definition
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
        let save_sides = (1 + u64::from(definition.level) / 12)
            .saturating_mul(u64::from(definition.level))
            .max(1);
        if unique
            && self.rng.bounded(save_sides) + 1
                > u64::try_from(raw_damage.max(0)).unwrap_or(u64::MAX)
        {
            return;
        }
        self.apply_actor_melee_status(
            index,
            STATUS_STUN,
            monster_stun_amount(raw_damage),
            "rfb.mutation.draconian-strike",
        );
    }

    fn resolve_draconian_confusing_strike(
        &mut self,
        index: usize,
        definition: &rfb_content::ActorDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let was_ready = self.confusing_strike_ready;
        self.confusing_strike_ready = true;
        self.resolve_confusing_strike(index, definition, events);
        self.confusing_strike_ready = was_ready;
    }

    pub(super) fn resolve_player_melee(
        &mut self,
        index: usize,
        train_weapon: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<PlayerMeleeOutcome, CoreError> {
        self.resolve_player_melee_with_draconian_strike(
            index,
            train_weapon,
            None,
            events,
            changed,
            removed_entities,
        )
    }

    pub(super) fn resolve_player_draconian_strike(
        &mut self,
        index: usize,
        mode: DraconianStrikeModeDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<PlayerMeleeOutcome, CoreError> {
        self.resolve_player_melee_with_draconian_strike(
            index,
            false,
            Some(mode),
            events,
            changed,
            removed_entities,
        )
    }

    fn resolve_player_melee_with_draconian_strike(
        &mut self,
        index: usize,
        train_weapon: bool,
        strike_mode: Option<DraconianStrikeModeDefinition>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<PlayerMeleeOutcome, CoreError> {
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("monster actor definition must remain available")
            .clone();
        let target_entity_id = self.entities[index].id.clone();
        let target_kind = self.entities[index].kind_id.clone();
        self.entities[index].alerted = true;
        let attacker = self.player_derived_stats();
        let target = self.actor_derived_stats(&self.entities[index], &definition, false);
        let weapon_profile = self.player_melee_profile(&attacker);
        let equipped_weapon_id = weapon_profile.source_item_id.clone();
        if train_weapon
            && let Some(item_id) = equipped_weapon_id.as_deref()
            && let Some(item_kind_id) = self.train_weapon_proficiency(item_id, definition.level)
        {
            events.push(DomainEvent::WeaponProficiencyImproved { item_kind_id });
        }
        if let Some(event) = self.train_riding_from_melee(definition.level) {
            events.push(event);
        }
        let mut profiles = if self.player_has_draconian_metamorphosis() {
            Vec::new()
        } else {
            vec![weapon_profile]
        };
        profiles.extend(
            self.player_mutation_innate_attack_profiles(&attacker, equipped_weapon_id.as_deref()),
        );
        let profiles = profiles
            .into_iter()
            .map(|profile| {
                let attacks = profile.attacks.saturating_add(u16::from(
                    profile.extra_attack_chance_percent > 0
                        && self.rng.bounded(100) < u64::from(profile.extra_attack_chance_percent),
                ));
                (profile, attacks)
            })
            .collect::<Vec<_>>();
        let attacks_available = profiles
            .iter()
            .fold(0_u16, |total, (_, attacks)| total.saturating_add(*attacks))
            .max(1);
        let mut attacks_used = 0_u16;
        let mut killed = false;
        let mut vampiric_drain_remaining = 50_i32;
        let mut retaliation_blow_index = 0_usize;
        let mut touched_surviving_target = false;
        let mut allow_criticals = true;
        'profiles: for (profile, profile_attacks) in profiles {
            let vampiric_weapon =
                matches!(strike_mode, Some(DraconianStrikeModeDefinition::Vampiric))
                    || profile.source_item_id.as_ref().is_some_and(|item_id| {
                        self.items
                            .iter()
                            .find(|item| &item.id == item_id)
                            .is_some_and(|item| {
                                self.item_passives(item)
                                    .contains(&EquipmentPassive::Vampiric)
                            })
                    });
            let base_damage_multiplier = self.draconian_strike_damage_multiplier(
                &profile,
                &self.entities[index],
                &definition,
                strike_mode,
            );
            for _ in 0..profile_attacks {
                attacks_used = attacks_used.saturating_add(1);
                self.apply_easy_tiring_fatigue(50);
                if profile.melee_skill.value <= 0
                    || !self
                        .resolve_player_hit_check(CheckContext {
                            kind: CheckKind::MeleeHit,
                            actor_id: self.player.id.clone(),
                            target_id: Some(self.entities[index].id.clone()),
                            ability: profile.melee_skill.clone(),
                            difficulty: target.armor_class.clone(),
                        })
                        .succeeded()
                {
                    events.push(profile.miss_event(&target_kind));
                    self.check_human_dexterity_sprain(
                        if profile.source_item_id.is_some() {
                            250
                        } else {
                            300
                        },
                        events,
                    );
                    continue;
                }

                let source_weapon_index = profile
                    .source_item_id
                    .as_deref()
                    .and_then(|item_id| self.items.iter().position(|item| item.id == item_id));
                let has_trait = |trait_| {
                    source_weapon_index.is_some_and(|item_index| {
                        Self::item_has_weapon_trait(&self.items[item_index], trait_)
                    })
                };
                let order = has_trait(WeaponTraitDto::Order);
                let vorpal_chance = if has_trait(WeaponTraitDto::Vorpal2) {
                    Some(2_u64)
                } else if has_trait(WeaponTraitDto::Vorpal) {
                    Some(4_u64)
                } else {
                    None
                };
                let mut damage_multiplier = base_damage_multiplier;
                if has_trait(WeaponTraitDto::ManaBrand)
                    && let Some(resource_id) = self
                        .casting_profile()
                        .map(|profile| profile.resource_id.clone())
                    && let Some(pool) = self.resources.get_mut(&resource_id)
                {
                    let cost = mana_brand_cost(profile.damage_dice, profile.damage_sides);
                    if pool.current >= cost {
                        pool.current -= cost;
                        damage_multiplier = mana_brand_multiplier(damage_multiplier);
                    }
                }
                let weapon_damage = if order {
                    i32::from(profile.damage_dice).saturating_mul(i32::from(profile.damage_sides))
                } else {
                    self.roll_damage(profile.damage_dice, profile.damage_sides)
                };
                let mut base_damage = weapon_damage
                    .saturating_mul(damage_multiplier)
                    .saturating_div(10);
                if !order && let Some(weight) = profile.critical_weight_tenths_pound {
                    base_damage = base_damage
                        .saturating_mul(self.roll_player_melee_critical_multiplier(
                            weight,
                            profile.to_hit,
                            &mut allow_criticals,
                        ))
                        .saturating_div(100);
                }
                if let Some(chance) = vorpal_chance
                    && self.rng.bounded(chance.saturating_mul(3).saturating_div(2)) == 0
                {
                    let mut multiplier = 2;
                    while self.rng.bounded(chance) == 0 {
                        multiplier += 1;
                    }
                    base_damage = base_damage.saturating_mul(multiplier);
                }
                let mut rolled_damage = base_damage.saturating_add(profile.to_damage).max(0);
                if matches!(strike_mode, Some(DraconianStrikeModeDefinition::Vorpal))
                    && self.rng.bounded(6) == 0
                {
                    let mut multiplier = 2;
                    while self.rng.bounded(4) == 0 {
                        multiplier += 1;
                    }
                    rolled_damage = rolled_damage.saturating_mul(multiplier);
                }
                self.check_human_dexterity_sprain(
                    if profile.source_item_id.is_some() {
                        500
                    } else {
                        300
                    },
                    events,
                );
                let damage_type = profile.damage_type;
                let resistance = self.entities[index].resistances.level(damage_type);
                let damage =
                    resolve_damage(DamagePacket::new(rolled_damage, damage_type), resistance);
                let application = plan_damage_application(
                    &self.entities[index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[index], &application);
                events.push(profile.hit_event(&target_kind, damage));
                self.wake_entity_after_damage(index, damage.applied, events);
                if !application.fatal {
                    match strike_mode {
                        Some(DraconianStrikeModeDefinition::Stun) => {
                            self.resolve_draconian_stunning_strike(
                                index,
                                rolled_damage,
                                &definition,
                            );
                        }
                        Some(DraconianStrikeModeDefinition::Confusion) => {
                            self.resolve_draconian_confusing_strike(index, &definition, events);
                        }
                        _ => {}
                    }
                }
                let mut revenge_stop = false;
                if !application.fatal
                    && let Some(stop) = self.resolve_monster_revenge_aura(
                        index,
                        retaliation_blow_index,
                        events,
                        changed,
                        removed_entities,
                    )?
                {
                    retaliation_blow_index = retaliation_blow_index.saturating_add(1);
                    revenge_stop = stop;
                }
                let contact_aura_fatal = !revenge_stop
                    && self.resolve_monster_contact_auras(index, &definition, events, changed);
                if contact_aura_fatal || revenge_stop {
                    if application.fatal {
                        killed = true;
                        self.resolve_actor_death(
                            index,
                            profile.slew_event(&target_kind, damage),
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                    break 'profiles;
                }
                if vampiric_weapon
                    && vampiric_drain_remaining > 0
                    && damage.applied > 5
                    && actor_matches_category(&definition, "living")
                {
                    let raw_requested = self
                        .roll_damage(
                            2,
                            u16::try_from(damage.applied / 6)
                                .expect("positive vampiric healing die must fit u16"),
                        )
                        .min(vampiric_drain_remaining);
                    vampiric_drain_remaining =
                        vampiric_drain_remaining.saturating_sub(raw_requested);
                    let requested = raw_requested.saturating_mul(
                        i32::try_from(self.mutation_regeneration_percent())
                            .expect("mutation regeneration percent must fit i32"),
                    ) / 100;
                    let outcome = self.apply_player_healing(requested);
                    events.push(DomainEvent::PlayerVampiricHealed {
                        resolution: HealingResolutionDto {
                            requested: outcome.requested,
                            applied: outcome.applied,
                        },
                    });
                }
                if application.fatal {
                    killed = true;
                    self.resolve_actor_death(
                        index,
                        profile.slew_event(&target_kind, damage),
                        events,
                        changed,
                        removed_entities,
                    )?;
                    break 'profiles;
                }
                touched_surviving_target = true;
                self.resolve_confusing_strike(index, &definition, events);
            }
        }
        if touched_surviving_target
            && !self.player_is_dead()
            && let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_entity_id)
        {
            self.resolve_monster_fear_aura(index, "contact", false, events);
        }
        Ok(PlayerMeleeOutcome {
            attacks_used,
            attacks_available,
            killed,
        })
    }

    pub(super) fn resolve_monster_revenge_aura(
        &mut self,
        index: usize,
        blow_index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<bool>, CoreError> {
        let definition = self
            .content
            .actor(&self.entities[index].kind_id)
            .expect("revenge aura actor definition must remain available");
        let incapacitated = self.entities[index]
            .statuses
            .iter()
            .any(|status| matches!(status.kind_id.as_str(), STATUS_CONFUSION | STATUS_PARALYSIS));
        if self.entities[index].hp <= 0
            || incapacitated
            || !definition.tags.iter().any(|tag| tag == "aura-revenge")
            || self.rng.bounded(150) >= u64::from(definition.level)
        {
            return Ok(None);
        }
        let source_entity_id = self.entities[index].id.clone();
        let self_destructs = self.resolve_monster_revenge_blow(
            index,
            blow_index,
            events,
            changed,
            removed_entities,
        )?;
        if self_destructs
            && let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == source_entity_id)
        {
            let source_kind_id = self.entities[index].kind_id.clone();
            self.resolve_actor_death(
                index,
                DomainEvent::MonsterSelfDestructed { source_kind_id },
                events,
                changed,
                removed_entities,
            )?;
        }
        let source_still_adjacent = self
            .entities
            .iter()
            .find(|entity| entity.id == source_entity_id)
            .is_some_and(|entity| adjacent(entity.position, self.player.position));
        Ok(Some(self.player_is_dead() || !source_still_adjacent))
    }

    pub(super) fn roll_projectile_critical_multiplier(
        &mut self,
        ammunition_weight_tenths_pound: u16,
        to_hit: i32,
        ranged_skill: i32,
        ammunition_type: AmmunitionTypeDefinition,
        concentration: u8,
    ) -> i32 {
        let bonus_per_level = self.character_definitions().map_or(0, |(_, _, class, _)| {
            class.projectile_critical_chance_bonus_percent_per_level
        });
        let sniping_profile = self.sniping_profile().copied();
        if bonus_per_level == 0 && sniping_profile.is_none() {
            return 100;
        }
        let concentration_bonus = self.sniper_concentration_bonus_percent(concentration);
        let ammunition_critical_percent = sniping_profile
            .filter(|profile| ammunition_type == profile.preferred_ammunition_type)
            .map_or(100, |profile| {
                profile.preferred_ammunition_critical_chance_percent
            });
        let chance = projectile_critical_chance(
            to_hit,
            ranged_skill,
            self.progress.level,
            bonus_per_level,
            concentration_bonus,
            ammunition_critical_percent,
        );
        let roll = i64::try_from(self.rng.bounded(5_000) + 1)
            .expect("projectile critical roll must fit i64");
        if roll > chance {
            return 100;
        }
        let quality =
            u64::from(ammunition_weight_tenths_pound).saturating_mul(self.rng.bounded(500) + 1);
        150_i32
            .saturating_add(i32::try_from(quality.saturating_mul(200) / 2_000).unwrap_or(i32::MAX))
    }

    pub(super) fn roll_innate_critical_multiplier(
        &mut self,
        weight_tenths_pound: u16,
        to_hit: i32,
    ) -> i32 {
        let chance = i64::from(weight_tenths_pound)
            .saturating_add(i64::from(to_hit).saturating_mul(5))
            .saturating_add(i64::from(self.progress.level).saturating_mul(3));
        let roll =
            i64::try_from(self.rng.bounded(5_000) + 1).expect("critical-hit roll must fit i64");
        if roll > chance {
            return 100;
        }
        let quality = u64::from(weight_tenths_pound) + self.rng.bounded(650) + 1;
        match quality {
            0..=399 => 200,
            400..=699 => 250,
            700..=899 => 300,
            900..=1_299 => 350,
            _ => 400,
        }
    }

    pub(super) fn roll_player_melee_critical_multiplier(
        &mut self,
        weight_tenths_pound: u16,
        to_hit: i32,
        allow_criticals: &mut bool,
    ) -> i32 {
        if !*allow_criticals {
            return 100;
        }
        let multiplier = self.roll_innate_critical_multiplier(weight_tenths_pound, to_hit);
        if multiplier > 100 && self.player_has_mutation(HUMAN_STR_MUTATION_ID) {
            *allow_criticals = false;
            self.player.energy_need = self
                .player
                .energy_need
                .saturating_add(STANDARD_ACTION_COST / 5);
        }
        multiplier
    }

    pub(super) fn resolve_monster_contact_auras(
        &mut self,
        source_index: usize,
        definition: &rfb_content::ActorDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        for aura in &definition.contact_auras {
            if aura
                .chance_percent
                .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
            {
                continue;
            }
            let raw = self.roll_damage(aura.damage_dice, aura.damage_sides);
            if aura.ravages_time {
                self.resolve_time_melee(&definition.id, events);
            }
            if aura.damage_type == rfb_content::ActorDamageType::Curse
                && self.monster_curse_save(&definition.id, events)
            {
                continue;
            }
            let damage_type = DamageType::from(aura.damage_type);
            let damage = resolve_damage(
                DamagePacket::new(raw, damage_type),
                self.effective_player_resistances().level(damage_type),
            );
            if aura.damage_type != rfb_content::ActorDamageType::Poison {
                if damage.applied <= 0 {
                    continue;
                }
                let application = self.apply_final_player_damage(damage, FatalityPolicy::BelowZero);
                let damage = application.damage;
                self.damage_player_inventory(
                    &definition.id,
                    damage_type,
                    true,
                    damage.applied,
                    events,
                );
                events.push(DomainEvent::MonsterMeleeHit {
                    source_kind_id: definition.id.clone(),
                    method_id: None,
                    damage,
                });
                if application.fatal {
                    events.push(DomainEvent::PlayerDied {
                        source_kind_id: definition.id.clone(),
                        method_id: None,
                        damage,
                    });
                    return true;
                }
                continue;
            }
            let duration = damage.applied.saturating_mul(7) / 4;
            if duration <= 0 || self.player_status_immunities().contains(STATUS_POISON) {
                continue;
            }
            let duration = u32::try_from(duration).unwrap_or(u32::MAX);
            apply_status(
                &mut self.player.statuses,
                monster_combat::melee_status(STATUS_POISON, duration, &definition.id),
            );
            events.push(DomainEvent::MonsterContactAuraApplied {
                source_kind_id: definition.id.clone(),
                status_kind_id: STATUS_POISON.to_owned(),
                duration,
            });
        }
        for effect in &definition.contact_effects {
            if monster_combat::melee_effect_chance(effect)
                .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
            {
                continue;
            }
            match effect {
                MeleeBlowEffectDefinition::Unlife {
                    amount_dice,
                    amount_sides,
                    ..
                } => {
                    let amount =
                        u16::try_from(self.roll_damage(*amount_dice, *amount_sides).max(0))
                            .unwrap_or(u16::MAX);
                    self.resolve_monster_unlife_against_player(
                        source_index,
                        amount,
                        events,
                        changed,
                    );
                }
                MeleeBlowEffectDefinition::Stun {
                    duration_dice,
                    duration_sides,
                    ..
                } => {
                    let duration = self.roll_damage(*duration_dice, *duration_sides);
                    self.apply_player_melee_status(STATUS_STUN, duration, &definition.id);
                    events.push(DomainEvent::MonsterContactAuraApplied {
                        source_kind_id: definition.id.clone(),
                        status_kind_id: STATUS_STUN.to_owned(),
                        duration: u32::try_from(duration.max(0)).unwrap_or(u32::MAX),
                    });
                }
                _ => unreachable!("validated contact effects stay narrow"),
            }
        }
        false
    }

    pub(super) fn resolve_confusing_strike(
        &mut self,
        index: usize,
        definition: &rfb_content::ActorDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.confusing_strike_ready {
            return;
        }
        self.confusing_strike_ready = false;
        let target_kind_id = self.entities[index].kind_id.clone();
        if definition
            .status_immunities
            .iter()
            .any(|status| status == STATUS_CONFUSION)
        {
            events.push(DomainEvent::ConfusingStrikeImmune { target_kind_id });
            return;
        }
        if self.rng.bounded(100) < u64::from(definition.level) {
            events.push(DomainEvent::ConfusingStrikeResisted { target_kind_id });
            return;
        }
        let duration = 10_u32.saturating_add(
            u32::try_from(self.rng.bounded(u64::from(self.progress.level.max(1))))
                .expect("confusing strike duration roll must fit u32")
                / 5,
        );
        apply_status(
            &mut self.entities[index].statuses,
            StatusApplication {
                status: StatusInstance {
                    kind_id: STATUS_CONFUSION.to_owned(),
                    intensity: 1,
                    remaining_ticks: duration,
                    source_id: None,
                    granted_resistances: BTreeMap::new(),
                    granted_brands: BTreeSet::new(),
                    granted_modifiers: StatModifiersDto::default(),
                    granted_equipment_bonuses: EquipmentBonusesDto::default(),
                    granted_status_immunities: BTreeSet::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                },
                stacking: StatusStacking::Extend,
            },
        );
        events.push(DomainEvent::ConfusingStrikeApplied {
            target_kind_id,
            duration,
        });
    }

    pub(super) fn resolve_player_summon_melee(
        &mut self,
        source_index: usize,
        target_entity_id: &str,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let source_entity_id = self.entities[source_index].id.clone();
        let source_kind_id = self.entities[source_index].kind_id.clone();
        let definition = self
            .actor_runtime_definition(&self.entities[source_index])
            .expect("summon actor definition must remain available")
            .clone();
        let attacker = self.actor_derived_stats(&self.entities[source_index], &definition, false);
        for blow in resolved_melee_blows(&definition) {
            let Some(target_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_entity_id && entity.hp > 0)
            else {
                break;
            };
            let target_kind_id = self.entities[target_index].kind_id.clone();
            let target_position = self.entities[target_index].position;
            let target_definition = self
                .actor_runtime_definition(&self.entities[target_index])
                .expect("summon melee target definition must remain available");
            let target_is_nonliving = target_definition.tags.iter().any(|tag| tag == "nonliving");
            let target_stats =
                self.actor_derived_stats(&self.entities[target_index], target_definition, false);
            let ability = attacker.melee_skill.with_modifier(
                StatLayer::Base,
                blow.method_id.as_deref().unwrap_or(definition.id.as_str()),
                blow.to_hit,
                StatBounds::NON_NEGATIVE,
            );
            if !resolve_check(
                &mut self.rng,
                CheckContext {
                    kind: CheckKind::MeleeHit,
                    actor_id: source_entity_id.clone(),
                    target_id: Some(target_entity_id.to_owned()),
                    ability,
                    difficulty: target_stats.armor_class.clone(),
                },
            )
            .succeeded()
            {
                events.push(DomainEvent::SummonMeleeMissed {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id,
                    method_id: blow.method_id,
                });
                continue;
            }

            if blow.self_destructs {
                if let Some(source_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == source_entity_id)
                {
                    self.resolve_actor_death_without_rewards(
                        source_index,
                        Some(DomainEvent::MonsterSelfDestructed {
                            source_kind_id: source_kind_id.clone(),
                        }),
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                break;
            }

            self.entities[target_index].alerted = true;
            for effect in &blow.effects {
                if monster_combat::melee_effect_chance(effect)
                    .is_some_and(|chance| self.rng.bounded(100) >= u64::from(chance))
                {
                    continue;
                }
                let Some(target_index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == target_entity_id && entity.hp > 0)
                else {
                    break;
                };
                let vampiric = matches!(
                    effect,
                    MeleeBlowEffectDefinition::Damage { vampiric: true, .. }
                ) && !target_is_nonliving;
                let damage = match effect {
                    MeleeBlowEffectDefinition::Damage {
                        damage_dice,
                        damage_sides,
                        damage_type,
                        armor_mitigated,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
                        let damage_type = DamageType::from(*damage_type);
                        let resistance = self.entities[target_index].resistances.level(damage_type);
                        Some(if *armor_mitigated {
                            resolve_armored_damage(
                                raw,
                                damage_type,
                                target_stats.armor_class.value,
                                resistance,
                            )
                        } else {
                            resolve_damage(DamagePacket::new(raw, damage_type), resistance)
                        })
                    }
                    MeleeBlowEffectDefinition::Shatter {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
                        Some(resolve_armored_damage(
                            raw,
                            DamageType::Physical,
                            target_stats.armor_class.value,
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Physical),
                        ))
                    }
                    MeleeBlowEffectDefinition::Poison {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
                        let duration = resolve_damage(
                            DamagePacket::new(raw, DamageType::Poison),
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Poison),
                        )
                        .applied
                        .saturating_mul(7)
                            / 4;
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_POISON,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Disease {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let raw = self.roll_monster_melee_effect(
                            source_index,
                            *damage_dice,
                            *damage_sides,
                            false,
                        );
                        Some(resolve_damage(
                            DamagePacket::new(raw, DamageType::Poison),
                            self.entities[target_index]
                                .resistances
                                .level(DamageType::Poison),
                        ))
                    }
                    MeleeBlowEffectDefinition::Bomb { .. }
                    | MeleeBlowEffectDefinition::DrainAttributes { .. }
                    | MeleeBlowEffectDefinition::DrainResource { .. }
                    | MeleeBlowEffectDefinition::DrainExperience { .. }
                    | MeleeBlowEffectDefinition::Disenchant { .. }
                    | MeleeBlowEffectDefinition::Amnesia { .. }
                    | MeleeBlowEffectDefinition::Time { .. }
                    | MeleeBlowEffectDefinition::PolymorphPlayer { .. } => None,
                    MeleeBlowEffectDefinition::Unlife {
                        amount_dice,
                        amount_sides,
                        ..
                    } => {
                        let amount = u16::try_from(
                            self.roll_monster_melee_effect(
                                source_index,
                                *amount_dice,
                                *amount_sides,
                                false,
                            )
                            .max(0),
                        )
                        .unwrap_or(u16::MAX);
                        self.resolve_monster_unlife_against_actor(
                            &source_kind_id,
                            target_index,
                            amount,
                            events,
                            changed,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Bleeding {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = self.roll_monster_melee_effect(
                            source_index,
                            *duration_dice,
                            *duration_sides,
                            false,
                        );
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_BLEEDING,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Blind { .. } => {
                        let duration = 12 + i32::try_from(self.rng.bounded(4)).unwrap_or(0);
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_BLINDNESS,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Confusion {
                        damage_dice,
                        damage_sides,
                        ..
                    } => {
                        let resistance = self.entities[target_index]
                            .resistances
                            .level(DamageType::Confusion);
                        let raw = (*damage_dice > 0).then(|| {
                            self.roll_monster_melee_effect(
                                source_index,
                                *damage_dice,
                                *damage_sides,
                                false,
                            )
                        });
                        let duration = resisted_status_duration(
                            u32::try_from(10 + self.roll_damage(1, 20)).unwrap_or(u32::MAX),
                            resistance,
                        );
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_CONFUSION,
                            i32::try_from(duration).unwrap_or(i32::MAX),
                            &source_kind_id,
                        );
                        raw.map(|raw| {
                            resolve_damage(
                                DamagePacket::new(raw, DamageType::Confusion),
                                resistance,
                            )
                        })
                    }
                    MeleeBlowEffectDefinition::Paralysis { .. } => {
                        let duration = self.roll_damage(1, 3);
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_PARALYSIS,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Slow { .. } => {
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_SLOW,
                            25,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Inertia { .. } => {
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_SLOW,
                            25,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Stun {
                        duration_dice,
                        duration_sides,
                        ..
                    } => {
                        let duration = self.roll_monster_melee_effect(
                            source_index,
                            *duration_dice,
                            *duration_sides,
                            false,
                        );
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_STUN,
                            duration,
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::Terrify { .. } => {
                        self.apply_actor_melee_status(
                            target_index,
                            STATUS_FEAR,
                            monster_combat::melee_terrify_duration(&definition),
                            &source_kind_id,
                        );
                        None
                    }
                    MeleeBlowEffectDefinition::EatGold { .. }
                    | MeleeBlowEffectDefinition::EatItem { .. }
                    | MeleeBlowEffectDefinition::EatFood { .. }
                    | MeleeBlowEffectDefinition::EatLight { .. }
                    | MeleeBlowEffectDefinition::DrainCharges { .. } => None,
                };
                let Some(damage) = damage else {
                    continue;
                };
                let shatters = matches!(effect, MeleeBlowEffectDefinition::Shatter { .. })
                    && damage.applied > 23;
                let quake_center = self.entities[source_index].position;
                let application = plan_damage_application(
                    &self.entities[target_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[target_index], &application);
                changed.insert(target_position);
                self.wake_entity_after_damage(target_index, damage.applied, events);
                if application.fatal {
                    let slain = self.entities[target_index].clone();
                    self.reward_controlled_actor_kill(&source_entity_id, &slain, events);
                    self.resolve_actor_death_without_rewards(
                        target_index,
                        Some(DomainEvent::SummonSlew {
                            source_kind_id: source_kind_id.clone(),
                            target_kind_id: target_kind_id.clone(),
                            method_id: blow.method_id.clone(),
                            damage,
                        }),
                        events,
                        changed,
                        removed_entities,
                    )?;
                    if shatters {
                        self.resolve_monster_shatter_earthquake(
                            quake_center,
                            source_kind_id.clone(),
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                    break;
                }
                if vampiric {
                    self.heal_vampiric_melee_source(&source_entity_id, damage.applied, changed);
                }
                if matches!(effect, MeleeBlowEffectDefinition::Disease { .. }) {
                    let duration = resolve_damage(
                        DamagePacket::new(damage.applied, DamageType::Poison),
                        self.entities[target_index]
                            .resistances
                            .level(DamageType::Poison),
                    )
                    .applied;
                    self.apply_actor_melee_status(
                        target_index,
                        STATUS_POISON,
                        duration,
                        &source_kind_id,
                    );
                }
                events.push(DomainEvent::SummonMeleeHit {
                    source_kind_id: source_kind_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    method_id: blow.method_id.clone(),
                    damage,
                });
                if shatters {
                    self.resolve_monster_shatter_earthquake(
                        quake_center,
                        source_kind_id.clone(),
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use rfb_content::{SlayLevel, SlayTarget, SniperShotModeDefinition, WeaponBrand};

    use crate::resistance::ResistanceLevel;
    use crate::rng::RfbRng;

    use super::{
        ProjectileMode, concentrated_target_armor_class, projectile_critical_chance,
        projectile_raw_damage, roll_sniper_needle_vital_hit, sniper_explosion_radius,
        sniper_shot_damage_multiplier,
    };

    #[test]
    fn ammunition_damage_and_bonus_are_scaled_before_launcher_bonus() {
        assert_eq!(projectile_raw_damage(7, 10, 2, 100, 0, 250, 3), 25);
        assert_eq!(projectile_raw_damage(7, 10, 2, 100, 0, 350, 3), 34);
        assert_eq!(projectile_raw_damage(7, 10, 2, 200, 0, 250, 3), 48);
        assert_eq!(projectile_raw_damage(7, 24, 2, 100, 0, 250, 3), 48);
        assert_eq!(projectile_raw_damage(7, 10, 2, 100, 30, 250, 3), 30);
    }

    #[test]
    fn concentration_reduces_armor_and_scales_critical_chance_before_bolt_bonus() {
        assert_eq!(concentrated_target_armor_class(50, 3), 35);
        assert_eq!(projectile_critical_chance(10, 100, 50, 0, 0, 150), 345);
        assert_eq!(projectile_critical_chance(10, 100, 50, 0, 30, 150), 448);
    }

    #[test]
    fn elemental_and_shining_sniper_multipliers_follow_original_focus_rules() {
        let no_brands = BTreeSet::new();
        let fire_brand = BTreeSet::from([WeaponBrand::Fire]);
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Shining),
                3,
                &BTreeMap::new(),
                &no_brands,
                ResistanceLevel::Vulnerable,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                false,
                false,
                false,
            ),
            23
        );
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Burning),
                3,
                &BTreeMap::new(),
                &fire_brand,
                ResistanceLevel::Normal,
                ResistanceLevel::Vulnerable,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                false,
                false,
                false,
            ),
            58
        );
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Burning),
                3,
                &BTreeMap::new(),
                &fire_brand,
                ResistanceLevel::Normal,
                ResistanceLevel::Immune,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                false,
                false,
                false,
            ),
            10
        );
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Freezing),
                2,
                &BTreeMap::new(),
                &BTreeSet::from([WeaponBrand::Cold]),
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Vulnerable,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                false,
                false,
                false,
            ),
            52
        );
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Shatter),
                3,
                &BTreeMap::new(),
                &no_brands,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                ResistanceLevel::Normal,
                false,
                false,
                true,
            ),
            21
        );
    }

    #[test]
    fn advanced_sniper_multipliers_use_the_stronger_special_or_ammunition_modifier() {
        let slays = BTreeMap::from([
            (SlayTarget::Good, SlayLevel::Kill),
            (SlayTarget::Evil, SlayLevel::Slay),
        ]);
        let brands = BTreeSet::from([WeaponBrand::Electricity]);
        let normal = ResistanceLevel::Normal;
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Evil),
                4,
                &slays,
                &brands,
                normal,
                normal,
                normal,
                normal,
                normal,
                true,
                false,
                false,
            ),
            41
        );
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Holy),
                4,
                &slays,
                &brands,
                ResistanceLevel::Vulnerable,
                normal,
                normal,
                normal,
                normal,
                false,
                true,
                false,
            ),
            48
        );
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Thunder),
                3,
                &slays,
                &brands,
                normal,
                normal,
                normal,
                normal,
                normal,
                false,
                false,
                false,
            ),
            37
        );
        assert_eq!(
            sniper_shot_damage_multiplier(
                ProjectileMode::Sniper(SniperShotModeDefinition::Final),
                7,
                &slays,
                &brands,
                normal,
                normal,
                normal,
                normal,
                normal,
                false,
                false,
                false,
            ),
            50
        );
    }

    #[test]
    fn explosion_radius_and_needle_nested_rng_follow_original_boundaries() {
        assert_eq!(
            [0, 1, 2, 3, 4, 7].map(sniper_explosion_radius),
            [1, 2, 2, 3, 3, 5]
        );

        let mut low_level = RfbRng::seeded(1);
        let draws_before = low_level.draw_counter;
        let _ = roll_sniper_needle_vital_hit(&mut low_level, 1, 7, false);
        assert_eq!(low_level.draw_counter, draws_before + 1);

        let seed = (0..10_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                roll_sniper_needle_vital_hit(&mut rng, 50, 7, false)
            })
            .expect("a deterministic vital-hit seed should exist");
        let mut ordinary = RfbRng::seeded(seed);
        assert!(roll_sniper_needle_vital_hit(&mut ordinary, 50, 7, false));
        let ordinary_draws = ordinary.draw_counter;
        let mut unique = RfbRng::seeded(seed);
        assert!(!roll_sniper_needle_vital_hit(&mut unique, 50, 7, true));
        assert_eq!(unique.draw_counter, ordinary_draws);
    }
}

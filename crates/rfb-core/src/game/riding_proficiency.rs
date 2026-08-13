use rfb_protocol::{ProficiencyRankDto, RidingProficiencyDto};

use super::*;

const RIDING_EXP_BEGINNER: u16 = 2_000;
const RIDING_EXP_SKILLED: u16 = 4_000;
const RIDING_EXP_EXPERT: u16 = 6_000;
const RIDING_EXP_MASTER: u16 = 8_000;
const RFB_BASE_SPEED: i32 = 110;

pub(super) fn riding_attempt_range(current: u16, level: u16) -> u16 {
    current / 50 + level / 2 + 20
}

pub(super) fn mounted_speed(mount_speed: u16, current: u16, level: u16) -> i32 {
    let mount_speed = i32::from(mount_speed);
    let mut speed = if mount_speed > RFB_BASE_SPEED {
        let control = i64::from(current) * 3 + i64::from(level) * 160 - 10_000;
        let extra = i64::from(mount_speed - RFB_BASE_SPEED) * control / 22_000;
        RFB_BASE_SPEED
            .saturating_add(i32::try_from(extra).unwrap_or_else(|_| {
                if extra.is_negative() {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }))
            .max(RFB_BASE_SPEED)
    } else {
        mount_speed
    };
    speed = speed.saturating_add(i32::from(current).saturating_add(i32::from(level) * 160) / 3_200);
    speed
}

pub(super) fn mounted_melee_adjustment(
    expert: bool,
    weapon_kind: Option<RidingWeaponKindDefinition>,
    mount_level: u32,
    current: u16,
) -> (i32, u16) {
    match weapon_kind {
        Some(RidingWeaponKindDefinition::Lance) => (15, 2),
        Some(RidingWeaponKindDefinition::Compatible) => (0, 0),
        None if expert => (-5, 0),
        None => {
            let mount_level = i32::try_from(mount_level).unwrap_or(i32::MAX);
            let penalty = mount_level
                .saturating_sub(i32::from(current) / 80)
                .saturating_add(30)
                .max(30);
            (penalty.saturating_neg(), 0)
        }
    }
}

pub(super) fn mounted_projectile_to_hit_adjustment(
    expert: bool,
    ammunition_type: AmmunitionTypeDefinition,
    mount_level: u32,
    current: u16,
) -> i32 {
    let penalty = if expert {
        if ammunition_type == AmmunitionTypeDefinition::Arrow {
            0
        } else {
            5
        }
    } else {
        i32::try_from(mount_level)
            .unwrap_or(i32::MAX)
            .saturating_sub(i32::from(current) / 80)
            .saturating_add(30)
            .max(30)
    };
    if ammunition_type == AmmunitionTypeDefinition::Bolt {
        penalty.saturating_mul(-2)
    } else {
        penalty.saturating_neg()
    }
}

pub(super) fn riding_proficiency_rank(current: u16) -> ProficiencyRankDto {
    match current {
        ..RIDING_EXP_BEGINNER => ProficiencyRankDto::Unskilled,
        RIDING_EXP_BEGINNER..RIDING_EXP_SKILLED => ProficiencyRankDto::Beginner,
        RIDING_EXP_SKILLED..RIDING_EXP_EXPERT => ProficiencyRankDto::Skilled,
        RIDING_EXP_EXPERT..RIDING_EXP_MASTER => ProficiencyRankDto::Expert,
        _ => ProficiencyRankDto::Master,
    }
}

pub(super) fn riding_proficiency_progress_is_valid(
    content: &ContentCatalog,
    class_id: Option<&str>,
    current: u16,
) -> bool {
    class_id
        .and_then(|id| content.class(id))
        .map_or(current == 0, |class| {
            let proficiency = class.riding_proficiency;
            (proficiency.initial..=proficiency.maximum).contains(&current)
        })
}

impl Game {
    fn riding_proficiency_bounds(&self) -> Option<(u16, u16)> {
        let class = self
            .build
            .as_ref()
            .and_then(|build| self.content.class(&build.class_id))?;
        Some((
            class.riding_proficiency.initial,
            class.riding_proficiency.maximum,
        ))
    }

    pub(super) fn riding_mount_level(&self) -> Option<u32> {
        let mount_id = self.riding_actor_id.as_deref()?;
        let mount = self
            .entities
            .iter()
            .find(|actor| actor.id == mount_id && actor.hp > 0)?;
        self.actor_runtime_definition(mount)
            .map(|definition| definition.level)
    }

    fn gain_riding_proficiency(&mut self, increase: u16) -> Option<DomainEvent> {
        let (_, maximum) = self.riding_proficiency_bounds()?;
        let previous = self.progress.riding_proficiency;
        let current = previous.saturating_add(increase).min(maximum);
        self.progress.riding_proficiency = current;
        (current / 100 > previous / 100)
            .then_some(DomainEvent::RidingProficiencyImproved { current })
    }

    pub(super) fn train_riding_from_melee(&mut self, target_level: u32) -> Option<DomainEvent> {
        let mount_level = i32::try_from(self.riding_mount_level()?).unwrap_or(i32::MAX);
        let current = self.progress.riding_proficiency;
        let (_, maximum) = self.riding_proficiency_bounds()?;
        if current >= maximum {
            return None;
        }
        let current = i32::from(current);
        let target_level = i32::try_from(target_level).unwrap_or(i32::MAX);
        let mut increase = if current / 200 - 5 < target_level {
            1
        } else {
            0
        };
        if current / 100 < mount_level {
            increase += if current / 100 + 15 < mount_level {
                1 + mount_level - (current / 100 + 15)
            } else {
                1
            };
        }
        (increase > 0)
            .then(|| self.gain_riding_proficiency(u16::try_from(increase).unwrap_or(u16::MAX)))
            .flatten()
    }

    pub(super) fn train_riding_from_archery(&mut self) -> Option<DomainEvent> {
        let mount_level = i32::try_from(self.riding_mount_level()?).unwrap_or(i32::MAX);
        let current = self.progress.riding_proficiency;
        let (_, maximum) = self.riding_proficiency_bounds()?;
        if current >= maximum
            || (i32::from(current) - i32::from(RIDING_EXP_BEGINNER) * 2) / 200 >= mount_level
            || self.rng.bounded(2) != 0
        {
            return None;
        }
        self.gain_riding_proficiency(1)
    }

    pub(super) fn train_riding_from_fall_check(&mut self, damage: i32) -> Option<DomainEvent> {
        if damage < 0 {
            return None;
        }
        let mount_level = i32::try_from(self.riding_mount_level()?).unwrap_or(i32::MAX);
        let current = self.progress.riding_proficiency;
        let (_, maximum) = self.riding_proficiency_bounds()?;
        if current >= maximum
            || maximum <= 1_000
            || damage / 2 + mount_level <= i32::from(current) / 30 + 10
        {
            return None;
        }
        let current = i32::from(current);
        let increase = if mount_level > current / 100 + 15 {
            1 + mount_level - current / 100 - 15
        } else {
            1
        };
        self.gain_riding_proficiency(u16::try_from(increase).unwrap_or(u16::MAX))
    }

    pub(super) fn resolve_riding_fall(
        &mut self,
        damage: i32,
        force: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> bool {
        let Some(mount_id) = self.riding_actor_id.clone() else {
            return false;
        };
        let Some(mount_index) = self
            .entities
            .iter()
            .position(|actor| actor.id == mount_id && actor.hp > 0)
        else {
            self.riding_actor_id = None;
            return false;
        };
        let target_kind_id = self.entities[mount_index].kind_id.clone();
        let mount_level = self
            .actor_runtime_definition(&self.entities[mount_index])
            .map_or(0, |definition| definition.level);

        if !force {
            let current = self.progress.riding_proficiency;
            let maximum = self
                .riding_proficiency_bounds()
                .map_or(0, |(_, maximum)| maximum);
            if let Some(event) = self.train_riding_from_fall_check(damage) {
                events.push(event);
            }
            let range = i64::from(damage.max(0) / 2)
                .saturating_add(i64::from(mount_level).saturating_mul(2))
                .max(1);
            let held = self.rng.bounded(u64::try_from(range).unwrap_or(u64::MAX))
                < u64::from(current / 33 + 25);
            if held {
                if maximum == RIDING_EXP_MASTER {
                    return false;
                }
                let second_range = u64::from(self.progress.level)
                    .saturating_mul(3)
                    .saturating_add(30);
                if self.rng.bounded(second_range) != 0 {
                    return false;
                }
            }
        }

        let origin = self.player.position;
        let mut destination = None;
        let mut safe_count = 0_u64;
        for direction in TERRAIN_INTERACTION_DIRECTIONS {
            let (dx, dy) = direction.delta();
            let candidate = Position {
                x: origin.x + dx,
                y: origin.y + dy,
            };
            let safe = self.index(candidate).is_some()
                && (self.is_walkable(candidate) || self.player_can_pass_walls())
                && !self
                    .entities
                    .iter()
                    .any(|actor| actor.hp > 0 && actor.position == candidate);
            if !safe {
                continue;
            }
            safe_count = safe_count.saturating_add(1);
            if self.rng.bounded(safe_count) == 0 {
                destination = Some(candidate);
            }
        }

        let fall_damage = resolve_damage(
            DamagePacket::new(
                i32::try_from(mount_level)
                    .unwrap_or(i32::MAX)
                    .saturating_add(3),
                DamageType::Physical,
            ),
            ResistanceLevel::Normal,
        );
        let application =
            plan_damage_application(&self.player, fall_damage, FatalityPolicy::BelowZero);
        commit_damage_application(&mut self.player, &application);
        changed.insert(origin);

        let Some(destination) = destination else {
            events.push(DomainEvent::RidingCollided {
                target_kind_id: target_kind_id.clone(),
                damage: fall_damage,
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: target_kind_id,
                    method_id: Some("rfb.riding.collision".to_owned()),
                    damage: fall_damage,
                });
            }
            return false;
        };

        self.riding_actor_id = None;
        if !application.fatal {
            events.extend(self.relocate_player(destination, changed));
        }
        events.push(DomainEvent::RidingFell {
            target_kind_id: target_kind_id.clone(),
            damage: fall_damage,
        });
        if application.fatal {
            events.push(DomainEvent::PlayerDied {
                source_kind_id: target_kind_id,
                method_id: Some("rfb.riding.fall".to_owned()),
                damage: fall_damage,
            });
        }
        true
    }

    pub(super) fn force_dismount_if_mount_unrideable(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.riding_actor_id.as_deref() == Some(self.entities[index].id.as_str())
            && !self
                .actor_runtime_definition(&self.entities[index])
                .is_some_and(|definition| definition.rideable)
        {
            self.resolve_riding_fall(0, true, events, changed);
        }
    }

    pub(super) fn player_riding_proficiency(&self) -> RidingProficiencyDto {
        let current = self.progress.riding_proficiency;
        let maximum = self
            .riding_proficiency_bounds()
            .map_or(0, |(_, maximum)| maximum);
        RidingProficiencyDto {
            rank: riding_proficiency_rank(current),
            current,
            maximum,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riding_uses_its_own_original_rank_thresholds() {
        assert_eq!(
            riding_proficiency_rank(1_999),
            ProficiencyRankDto::Unskilled
        );
        assert_eq!(riding_proficiency_rank(2_000), ProficiencyRankDto::Beginner);
        assert_eq!(riding_proficiency_rank(4_000), ProficiencyRankDto::Skilled);
        assert_eq!(riding_proficiency_rank(6_000), ProficiencyRankDto::Expert);
        assert_eq!(riding_proficiency_rank(8_000), ProficiencyRankDto::Master);
        assert_eq!(riding_attempt_range(0, 1), 20);
        assert_eq!(riding_attempt_range(6_000, 1), 140);
    }
}

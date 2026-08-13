use rfb_protocol::{ProficiencyRankDto, RidingProficiencyDto};

use super::*;

const RIDING_EXP_BEGINNER: u16 = 2_000;
const RIDING_EXP_SKILLED: u16 = 4_000;
const RIDING_EXP_EXPERT: u16 = 6_000;
const RIDING_EXP_MASTER: u16 = 8_000;

pub(super) fn riding_attempt_range(current: u16, level: u16) -> u16 {
    current / 50 + level / 2 + 20
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

    fn riding_mount_level(&self) -> Option<u32> {
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

    /// Applies the original proficiency gain performed immediately before a
    /// non-forced fall check. The fall resolver itself is a later riding slice.
    #[allow(dead_code)]
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

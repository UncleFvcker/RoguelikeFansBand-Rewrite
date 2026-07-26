// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use rfb_protocol::{DamageTypeDto, ResistanceDto, ResistanceLevelDto, ResistanceSaveDto};

/// Stable damage categories shared by combat, effects, content, and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DamageType {
    Physical,
    Acid,
    Electricity,
    Fire,
    Cold,
    Poison,
    Light,
    Dark,
    Confusion,
    Nether,
    Nexus,
    Sound,
    Shards,
    Chaos,
    Disenchant,
    Time,
    Mana,
    Gravity,
    Inertia,
    Plasma,
    Force,
    Nuke,
    Disintegrate,
    Storm,
    HolyFire,
    HellFire,
    Ice,
    Water,
    Psi,
}

/// A compact first-pass resistance scale.
///
/// The elemental percentages preserve the important low-resistance relationship
/// from RFB while deliberately omitting the legacy random percentage jitter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResistanceLevel {
    Vulnerable,
    #[default]
    Normal,
    Resistant,
    Strong,
    Immune,
}

impl ResistanceLevel {
    #[must_use]
    pub const fn reduction_percent(self) -> i32 {
        match self {
            Self::Vulnerable => -50,
            Self::Normal => 0,
            Self::Resistant => 50,
            Self::Strong => 65,
            Self::Immune => 100,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResistanceProfile {
    levels: BTreeMap<DamageType, ResistanceLevel>,
}

impl ResistanceProfile {
    #[must_use]
    pub fn level(&self, damage_type: DamageType) -> ResistanceLevel {
        self.levels.get(&damage_type).copied().unwrap_or_default()
    }

    pub fn set(&mut self, damage_type: DamageType, level: ResistanceLevel) {
        if level == ResistanceLevel::Normal {
            self.levels.remove(&damage_type);
        } else {
            self.levels.insert(damage_type, level);
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (DamageType, ResistanceLevel)> + '_ {
        self.levels.iter().map(|(kind, level)| (*kind, *level))
    }
}

impl From<DamageType> for DamageTypeDto {
    fn from(value: DamageType) -> Self {
        match value {
            DamageType::Physical => Self::Physical,
            DamageType::Acid => Self::Acid,
            DamageType::Electricity => Self::Electricity,
            DamageType::Fire => Self::Fire,
            DamageType::Cold => Self::Cold,
            DamageType::Poison => Self::Poison,
            DamageType::Light => Self::Light,
            DamageType::Dark => Self::Dark,
            DamageType::Confusion => Self::Confusion,
            DamageType::Nether => Self::Nether,
            DamageType::Nexus => Self::Nexus,
            DamageType::Sound => Self::Sound,
            DamageType::Shards => Self::Shards,
            DamageType::Chaos => Self::Chaos,
            DamageType::Disenchant => Self::Disenchant,
            DamageType::Time => Self::Time,
            DamageType::Mana => Self::Mana,
            DamageType::Gravity => Self::Gravity,
            DamageType::Inertia => Self::Inertia,
            DamageType::Plasma => Self::Plasma,
            DamageType::Force => Self::Force,
            DamageType::Nuke => Self::Nuke,
            DamageType::Disintegrate => Self::Disintegrate,
            DamageType::Storm => Self::Storm,
            DamageType::HolyFire => Self::HolyFire,
            DamageType::HellFire => Self::HellFire,
            DamageType::Ice => Self::Ice,
            DamageType::Water => Self::Water,
            DamageType::Psi => Self::Psi,
        }
    }
}

impl From<DamageTypeDto> for DamageType {
    fn from(value: DamageTypeDto) -> Self {
        match value {
            DamageTypeDto::Physical => Self::Physical,
            DamageTypeDto::Acid => Self::Acid,
            DamageTypeDto::Electricity => Self::Electricity,
            DamageTypeDto::Fire => Self::Fire,
            DamageTypeDto::Cold => Self::Cold,
            DamageTypeDto::Poison => Self::Poison,
            DamageTypeDto::Light => Self::Light,
            DamageTypeDto::Dark => Self::Dark,
            DamageTypeDto::Confusion => Self::Confusion,
            DamageTypeDto::Nether => Self::Nether,
            DamageTypeDto::Nexus => Self::Nexus,
            DamageTypeDto::Sound => Self::Sound,
            DamageTypeDto::Shards => Self::Shards,
            DamageTypeDto::Chaos => Self::Chaos,
            DamageTypeDto::Disenchant => Self::Disenchant,
            DamageTypeDto::Time => Self::Time,
            DamageTypeDto::Mana => Self::Mana,
            DamageTypeDto::Gravity => Self::Gravity,
            DamageTypeDto::Inertia => Self::Inertia,
            DamageTypeDto::Plasma => Self::Plasma,
            DamageTypeDto::Force => Self::Force,
            DamageTypeDto::Nuke => Self::Nuke,
            DamageTypeDto::Disintegrate => Self::Disintegrate,
            DamageTypeDto::Storm => Self::Storm,
            DamageTypeDto::HolyFire => Self::HolyFire,
            DamageTypeDto::HellFire => Self::HellFire,
            DamageTypeDto::Ice => Self::Ice,
            DamageTypeDto::Water => Self::Water,
            DamageTypeDto::Psi => Self::Psi,
        }
    }
}

impl From<rfb_content::ActorDamageType> for DamageType {
    fn from(value: rfb_content::ActorDamageType) -> Self {
        match value {
            rfb_content::ActorDamageType::Physical => Self::Physical,
            rfb_content::ActorDamageType::Acid => Self::Acid,
            rfb_content::ActorDamageType::Electricity => Self::Electricity,
            rfb_content::ActorDamageType::Fire => Self::Fire,
            rfb_content::ActorDamageType::Cold => Self::Cold,
            rfb_content::ActorDamageType::Poison => Self::Poison,
            rfb_content::ActorDamageType::Light => Self::Light,
            rfb_content::ActorDamageType::Dark => Self::Dark,
            rfb_content::ActorDamageType::Confusion => Self::Confusion,
            rfb_content::ActorDamageType::Nether => Self::Nether,
            rfb_content::ActorDamageType::Nexus => Self::Nexus,
            rfb_content::ActorDamageType::Sound => Self::Sound,
            rfb_content::ActorDamageType::Shards => Self::Shards,
            rfb_content::ActorDamageType::Chaos => Self::Chaos,
            rfb_content::ActorDamageType::Disenchant => Self::Disenchant,
            rfb_content::ActorDamageType::Time => Self::Time,
            rfb_content::ActorDamageType::Mana => Self::Mana,
            rfb_content::ActorDamageType::Gravity => Self::Gravity,
            rfb_content::ActorDamageType::Inertia => Self::Inertia,
            rfb_content::ActorDamageType::Plasma => Self::Plasma,
            rfb_content::ActorDamageType::Force => Self::Force,
            rfb_content::ActorDamageType::Nuke => Self::Nuke,
            rfb_content::ActorDamageType::Disintegrate => Self::Disintegrate,
            rfb_content::ActorDamageType::Storm => Self::Storm,
            rfb_content::ActorDamageType::HolyFire => Self::HolyFire,
            rfb_content::ActorDamageType::HellFire => Self::HellFire,
            rfb_content::ActorDamageType::Ice => Self::Ice,
            rfb_content::ActorDamageType::Water => Self::Water,
            rfb_content::ActorDamageType::Psi => Self::Psi,
        }
    }
}

impl From<rfb_content::ActorResistanceLevel> for ResistanceLevel {
    fn from(value: rfb_content::ActorResistanceLevel) -> Self {
        match value {
            rfb_content::ActorResistanceLevel::Vulnerable => Self::Vulnerable,
            rfb_content::ActorResistanceLevel::Resistant => Self::Resistant,
            rfb_content::ActorResistanceLevel::Strong => Self::Strong,
            rfb_content::ActorResistanceLevel::Immune => Self::Immune,
        }
    }
}

/// Stamps the content-declared resistance tiers onto a runtime profile;
/// omitted types stay at the normal default.
pub(crate) fn definition_resistance_profile(
    definition: &rfb_content::ActorDefinition,
) -> ResistanceProfile {
    let mut profile = ResistanceProfile::default();
    for (damage_type, level) in &definition.resistances {
        profile.set(
            DamageType::from(*damage_type),
            ResistanceLevel::from(*level),
        );
    }
    profile
}

impl From<ResistanceLevel> for ResistanceLevelDto {
    fn from(value: ResistanceLevel) -> Self {
        match value {
            ResistanceLevel::Vulnerable => Self::Vulnerable,
            ResistanceLevel::Normal => Self::Normal,
            ResistanceLevel::Resistant => Self::Resistant,
            ResistanceLevel::Strong => Self::Strong,
            ResistanceLevel::Immune => Self::Immune,
        }
    }
}

impl From<ResistanceLevelDto> for ResistanceLevel {
    fn from(value: ResistanceLevelDto) -> Self {
        match value {
            ResistanceLevelDto::Vulnerable => Self::Vulnerable,
            ResistanceLevelDto::Normal => Self::Normal,
            ResistanceLevelDto::Resistant => Self::Resistant,
            ResistanceLevelDto::Strong => Self::Strong,
            ResistanceLevelDto::Immune => Self::Immune,
        }
    }
}

impl ResistanceProfile {
    #[must_use]
    pub fn to_dtos(&self) -> Vec<ResistanceDto> {
        self.iter()
            .map(|(damage_type, level)| ResistanceDto {
                damage_type: damage_type.into(),
                level: level.into(),
            })
            .collect()
    }

    #[must_use]
    pub fn to_save_dtos(&self) -> Vec<ResistanceSaveDto> {
        self.iter()
            .map(|(damage_type, level)| ResistanceSaveDto {
                damage_type: damage_type.into(),
                level: level.into(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_levels_are_implicit_and_non_normal_levels_are_stable() {
        let mut profile = ResistanceProfile::default();
        assert_eq!(profile.level(DamageType::Fire), ResistanceLevel::Normal);
        assert!(profile.is_empty());

        profile.set(DamageType::Fire, ResistanceLevel::Resistant);
        profile.set(DamageType::Cold, ResistanceLevel::Vulnerable);
        assert_eq!(profile.level(DamageType::Fire), ResistanceLevel::Resistant);
        assert_eq!(profile.level(DamageType::Cold), ResistanceLevel::Vulnerable);

        profile.set(DamageType::Fire, ResistanceLevel::Normal);
        assert_eq!(profile.level(DamageType::Fire), ResistanceLevel::Normal);
    }
}
